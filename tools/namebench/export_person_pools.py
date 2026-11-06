"""Export frozen person and place name pools (per Magna Urbis culture) from Namebench.

Reads the sibling Namebench database strictly read-only (SQLite URI mode=ro&immutable=1; Namebench's
own NameDB.conn would switch the file to WAL mode, so it is never opened through that path).
Real pools are SHA-256 keyed samples of distinct names (never ORDER BY RANDOM(), never the unseeded
`random`). Fantasy / historical-list pools come from Namebench's own FantasyGenerator driven by
`random.seed(<fixed string>)`, then reduced by the same keyed sample.

Usage (Namebench virtual environment, run from the repository root):
  ../namebench/.venv/Scripts/python.exe tools/namebench/export_person_pools.py ../namebench \
      fixtures/naming/person-pools.v1.json
"""
import hashlib
import json
import random
import re
import sqlite3
import subprocess
import sys
import unicodedata
from pathlib import Path

VERSION = "chronos-person-pools-v1"
SOURCE_COMMIT = "2820b50"
TARGETS = {"forenames_male": 300, "forenames_female": 300, "surnames": 300, "places": 200}
FANTASY_DRAWS = 4000
THIN = 50

# mu_culture_id -> (names source, places source, names fit, places fit, note).
# Source syntax: "zone:<namegen ref>" | "cc:<ISO,ISO>" | "fantasy:<fantasy style key>".
# "exact": a Namebench style made for that culture. "nearest": closest credible substitute.
M = {}


def c(cid, names, places, fit, pfit=None, note=""):
    M[cid] = (names, places, fit, pfit or fit, note)


c("al-andalus", "zone:style.arabic_maghreb", "zone:style.arabic_maghreb", "nearest", note="No Andalusi style; Maghreb Arabic.")
c("aztec", "fantasy:real.aztecs", "fantasy:real.aztecs", "exact")
c("bagan", "zone:style.indochina_thai_lao_khmer_burmese", "cc:MM", "nearest", note="Burmese names are sparse; regional Indochina style, Myanmar places.")
c("barbarian", "fantasy:fantasy.barbarians", "fantasy:fantasy.barbarians", "exact")
c("barbarian-celtic", "zone:overlay.celtic_ancient", "zone:overlay.celtic_ancient", "exact")
c("barbarian-norse", "fantasy:real.vikings", "zone:overlay.viking_age", "exact", note="Viking generator has no town list; places from the Viking Age overlay countries.")
c("bastide", "zone:style.french", "zone:style.french", "nearest", note="No Occitan style; French.")
c("budj-bim", "fantasy:real.aboriginals", "fantasy:real.aboriginals", "exact")
c("byzantine-greek", "zone:overlay.byzantine", "zone:overlay.byzantine", "exact")
c("caucasian-highland", "cc:GE", "cc:GE", "exact", note="Georgian pools.")
c("celtic-oppidum", "zone:overlay.celtic_ancient", "zone:overlay.celtic_ancient", "exact")
c("chinese", "zone:style.mandarin", "zone:style.mandarin", "exact")
c("dravidian-temple", "zone:style.tamil", "zone:style.tamil", "exact")
c("drow-enclave", "fantasy:pathfinder.drows", "fantasy:towns_and_cities.elven_citys", "exact", "nearest", "Places use the elven city generator.")
c("duergar-hold", "fantasy:pathfinder.dwarfs", "fantasy:towns_and_cities.dwarven_citys", "nearest", note="Dwarf styles stand in for duergar.")
c("dwarven", "fantasy:fantasy.dwarfs", "fantasy:towns_and_cities.dwarven_citys", "exact")
c("egyptian-planned", "fantasy:real.egyptians", "cc:EG", "exact", "nearest", "Ancient Egyptian names; places are present-day Egyptian GeoNames places.")
c("elven", "fantasy:fantasy.elfs", "fantasy:towns_and_cities.elven_citys", "exact")
c("ethiopian-highland", "zone:style.amhari_tigrinya", "zone:style.amhari_tigrinya", "exact")
c("european-organic", "zone:script.latin_west_europe", "zone:script.latin_west_europe", "nearest", note="Generic western Latin-script Europe.")
c("flemish-cloth-town", "zone:style.dutch_flemish", "zone:style.dutch_flemish", "exact")
c("giant-hold", "fantasy:fantasy.giants", "fantasy:fantasy.giants", "exact")
c("gnomish", "fantasy:fantasy.gnomes", "fantasy:fantasy.gnomes", "exact")
c("goblin-warren", "fantasy:fantasy.goblins", "fantasy:fantasy.goblins", "exact")
c("great-zimbabwe", "cc:ZW", "cc:ZW", "exact", note="Zimbabwean (Shona) pools.")
c("guarani-village", "cc:PY", "cc:PY", "nearest", note="Paraguay pools; Guarani names are sparse in the database.")
c("hadhrami-tower", "zone:style.arabic_yemen_oman", "zone:style.arabic_yemen_oman", "exact")
c("halfling", "fantasy:fantasy.hobbits", "fantasy:towns_and_cities.fantasy_towns", "exact", "nearest", "Places use the generic fantasy town generator.")
c("hanseatic", "zone:style.german_de_at", "zone:style.german_de_at", "nearest", note="No Low German style; German.")
c("hausa-kano", "zone:style.hausa", "zone:style.hausa", "exact")
c("himalayan-terraces", "fantasy:real.tibetans", "cc:NP", "exact", "nearest", "Tibetan names; places are Nepali GeoNames places.")
c("inca", "zone:style.quechua_inca", "zone:style.quechua_inca", "exact")
c("indian-temple", "zone:style.hindi", "zone:style.hindi", "nearest", note="No Sanskrit style; Hindi.")
c("indus-planned", "zone:style.punjabi", "zone:style.punjabi", "nearest", note="No Indus-era style; Punjabi.")
c("inuit-camp", "fantasy:real.inuits", "cc:GL", "exact", "nearest", "Inuit list is a single flat list shared by all name pools; places are Greenland GeoNames places.")
c("italian-comune", "zone:style.italian", "zone:style.italian", "exact")
c("japanese-jokamachi", "zone:overlay.japanese_feudal_edo", "zone:overlay.japanese_feudal_edo", "exact")
c("javanese-courts", "cc:ID", "cc:ID", "nearest", note="Indonesian pools; no Javanese-specific style.")
c("khmer", "cc:KH", "cc:KH", "exact")
c("korean", "zone:style.korean", "zone:style.korean", "exact")
c("kraal", "zone:style.zulu_xhosa", "zone:style.zulu_xhosa", "exact")
c("ksar-oasis", "zone:style.amazigh_berber", "zone:style.amazigh_berber", "nearest", note="Berber oasis settlements; Amazigh style.")
c("levantine-tell", "zone:style.arabic_levant", "zone:style.arabic_levant", "exact")
c("lizardfolk-swamp", "fantasy:warhammer.lizardmens", "fantasy:warhammer.lizardmens", "nearest", note="Lizardmen stand in for lizardfolk.")
c("maori-pa", "cc:NZ", "cc:NZ", "nearest", note="New Zealand pools; Maori names are sparse in the database.")
c("maya", "fantasy:real.mayans", "fantasy:real.mayans", "exact")
c("medina", "zone:macro.arabic", "zone:macro.arabic", "exact")
c("merfolk-tidal", "fantasy:fantasy.mermaids", "fantasy:towns_and_cities.underwater_citys", "nearest", note="Mermaid names mix mythological and ordinary names.")
c("mesopotamian", "cc:IQ", "cc:IQ", "nearest", note="No Akkadian style; Iraqi pools.")
c("mississippian", "fantasy:real.native_americans", "fantasy:real.native_americans", "nearest", note="Pan-Native-American list; no Mississippian style.")
c("mongol-ordu", "cc:MN", "cc:MN", "exact", note="Mongolian pools; gendered forenames are sparse in the database.")
c("myconid-colony", "fantasy:miscellaneous.fungis", "fantasy:miscellaneous.fungis", "nearest", note="Mushroom species name parts; no myconid style.")
c("native-iroquoian", "fantasy:real.native_americans", "fantasy:real.native_americans", "nearest", note="Pan-Native-American list.")
c("native-plains", "fantasy:real.native_americans", "fantasy:real.native_americans", "nearest", note="Pan-Native-American list.")
c("native-pueblo", "fantasy:real.native_americans", "fantasy:real.native_americans", "nearest", note="Pan-Native-American list.")
c("necropolis", "fantasy:real.egyptians", "cc:EG", "nearest", note="Ancient Egyptian names stand in; no undead style.")
c("newar", "cc:NP", "cc:NP", "nearest", note="Nepali pools; Newar names are sparse in the database.")
c("nomad-camp", "zone:style.central_asia_turkic", "zone:style.central_asia_turkic", "nearest", note="Central Asian Turkic.")
c("norse-ringfort", "fantasy:real.vikings", "zone:overlay.viking_age", "exact", note="Viking generator has no town list; places from the Viking Age overlay countries.")
c("nubian-fortified", "zone:style.arabic_egypt_sudan", "zone:style.arabic_egypt_sudan", "nearest", note="No Nubian style; Egypt and Sudan Arabic.")
c("oasis-caravan", "zone:style.arabic_gulf", "zone:style.arabic_gulf", "nearest", note="Gulf Arabic.")
c("orcish", "fantasy:pathfinder.orcs", "fantasy:towns_and_cities.orcish_citys", "exact")
c("ottoman", "zone:overlay.ottoman", "zone:overlay.ottoman", "exact")
c("persian", "zone:style.persian", "zone:style.persian", "exact")
c("pnw-plank-village", "fantasy:real.native_americans", "fantasy:real.native_americans", "nearest", note="Pan-Native-American list.")
c("rajput-fort", "zone:style.hindi", "zone:style.hindi", "nearest", note="Hindi.")
c("roman-core", "zone:overlay.roman_latin_ancient", "zone:overlay.roman_latin_ancient", "exact")
c("russian-kremlin", "zone:style.russian", "zone:style.russian", "exact")
c("rynek-town", "zone:style.polish", "zone:style.polish", "exact")
c("sahel", "zone:macro.sahel", "zone:macro.sahel", "exact")
c("siberian-camp", "zone:style.russian", "zone:style.russian", "nearest", note="No Evenki style; Russian.")
c("slavic-gord", "fantasy:real.slavics", "zone:macro.slavic", "exact", note="Places from the Slavic macro zone.")
c("stilt-town", "cc:MY", "cc:MY", "nearest", note="Malaysian pools.")
c("swahili-stone-town", "zone:style.swahili", "zone:style.swahili", "exact")
c("tibetan-monastery", "fantasy:real.tibetans", "cc:NP", "exact", "nearest", "Tibetan names; places are Nepali GeoNames places.")
c("venetian-lagoon", "zone:style.italian", "zone:style.italian", "nearest", note="No Venetian style; Italian.")
c("vietnamese-citadel", "zone:style.vietnamese", "zone:style.vietnamese", "exact")
c("wizard-city", "fantasy:fantasy.wizards", "fantasy:fantasy.wizards", "exact")
c("yoruba", "zone:style.yoruba", "zone:style.yoruba", "exact")


def sha256_file(path, chunk=1 << 22):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while block := f.read(chunk):
            h.update(block)
    return h.hexdigest()


def key(text):
    return hashlib.sha256(f"{VERSION}:{text}".encode()).hexdigest()


GLOSS = re.compile(r"\s*\([^)]*\)\s*$")


def clean(names):
    """NFC, strip, drop a trailing '(gloss)', collapse spaces, deduplicate, sort by code point."""
    out = set()
    for n in names:
        n = GLOSS.sub("", unicodedata.normalize("NFC", n)).strip()
        n = re.sub(r"\s+", " ", n)
        if n:
            out.add(n)
    return sorted(out)


def keyed(values, label, limit):
    return sorted(sorted(values, key=lambda v: key(f"{label}:{v}"))[:limit])


def resolve_codes(zones, spec):
    kind, _, arg = spec.partition(":")
    if kind == "cc":
        return arg.split(",")
    codes = zones.resolve(arg)
    if not codes:
        raise SystemExit(f"unknown zone {arg}")
    return sorted(codes)


def pick_names(conn, codes, name_type, gender, label, limit, short_ok):
    q = ",".join("?" for _ in codes)
    base = (f"SELECT DISTINCT name FROM names WHERE country_iso IN ({q}) AND name_type=? "
            + ("AND gender=? " if gender else "") + ("" if short_ok else "AND length(name)>=3 "))
    params = list(codes) + [name_type] + ([gender] if gender else [])
    ranked = clean(r[0] for r in conn.execute(base + "AND rank IS NOT NULL", params))
    if len(ranked) >= limit:
        return keyed(ranked, label, limit)
    # Top up with unranked names (still keyed) when too few ranked ones exist.
    have = set(ranked)
    rest = [n for n in clean(r[0] for r in conn.execute(base, params)) if n not in have]
    return clean(ranked + keyed(rest, label, limit - len(ranked)))


def pick_places(conn, codes, label, limit):
    q = ",".join("?" for _ in codes)
    sql = f"SELECT DISTINCT name FROM places WHERE country_iso IN ({q}) AND feature_class='P' "
    rows = clean(r[0] for r in conn.execute(sql + "AND feature_code='PPL' AND population BETWEEN 1000 AND 30000", codes))
    if len(rows) < limit:
        rows = clean(rows + [r[0] for r in conn.execute(sql, codes)])
    return keyed(rows, label, limit)


def pick_fantasy(db, style, name_type, gender, label, limit, gens):
    from namegen.strategies.fantasy import FantasyGenerator
    gen = gens.setdefault(style, FantasyGenerator(db, style))
    random.seed(f"{VERSION}:{label}")
    return keyed(clean(gen.generate(name_type, FANTASY_DRAWS, gender)), label, limit)


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    root, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2])
    repo = Path(__file__).resolve().parents[2]
    bindings = json.loads((repo / "docs/design/chronos-social-v2/culture-bindings.v1.json").read_text(encoding="utf-8"))
    ids = [b["mu_culture_id"] for b in bindings["bindings"]]
    if sorted(ids) != sorted(M) or len(ids) != 79:
        raise SystemExit(f"mapping/bindings mismatch: {sorted(set(ids) ^ set(M))}")
    commit = subprocess.run(["git", "-C", str(root), "rev-parse", "--short=7", "HEAD"],
                            capture_output=True, text=True).stdout.strip()
    if commit != SOURCE_COMMIT:
        raise SystemExit(f"Namebench is at {commit}, expected {SOURCE_COMMIT}")

    sys.path.insert(0, str(root))
    from namegen.db import NameDB
    from namegen.zones import OnomasticResolver
    from namegen.strategies.db_lookup import _SHORT_NAME_COUNTRIES

    db_path = root / "data" / "names.db"
    conn = sqlite3.connect(f"file:{db_path.as_posix()}?mode=ro&immutable=1", uri=True)
    conn.row_factory = sqlite3.Row
    db = NameDB(db_path)
    db._conn = conn  # read-only connection; skips NameDB's WAL pragma
    zones = OnomasticResolver()
    gens = {}

    cultures, thin = {}, []
    for cid in ids:
        names_src, places_src, fit, pfit, note = M[cid]
        entry = {"style": {"names": names_src, "places": places_src}, "fit": fit, "places_fit": pfit, "note": note}
        prov, pools = {}, {}
        for pool, (name_type, gender) in (("forenames_male", ("forename", "M")),
                                          ("forenames_female", ("forename", "F")),
                                          ("surnames", ("surname", None)),
                                          ("places", ("place", None))):
            src = places_src if pool == "places" else names_src
            label = f"{cid}:{pool}"
            kind, _, arg = src.partition(":")
            limit = TARGETS[pool]
            if kind == "fantasy":
                vals = pick_fantasy(db, arg, name_type, gender, label, limit, gens)
                prov[pool] = "derived"
            else:
                codes = resolve_codes(zones, src)
                if pool == "places":
                    vals = pick_places(conn, codes, label, limit)
                else:
                    vals = pick_names(conn, codes, name_type, gender, label, limit,
                                      bool(set(codes) & _SHORT_NAME_COUNTRIES))
                prov[pool] = "imported"
            pools[pool] = vals
            if len(vals) < THIN:
                thin.append((cid, pool, len(vals)))
        entry["provenance"] = prov
        entry.update(pools)
        cultures[cid] = entry

    content = json.dumps(cultures, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    doc = {
        "schema": "chronos-person-pools-1",
        "generator": VERSION,
        "source": {"project": "namebench", "commit": SOURCE_COMMIT, "script": "tools/namebench/export_person_pools.py",
                   "database_bytes": db_path.stat().st_size,
                   "code_sha256": {p: sha256_file(root / p) for p in (
                       "namegen/db.py", "namegen/styles.py", "namegen/strategies/fantasy.py",
                       "namegen/onomastic_zones.json")}},
        "content_sha256": hashlib.sha256(content.encode("utf-8")).hexdigest(),
        "content_sha256_of": "compact JSON of 'cultures' (sort_keys, separators ',' ':', UTF-8)",
        "provenance": {"imported": "values sampled from Namebench's real-name and GeoNames tables (present-day data)",
                       "derived": "values drawn from Namebench's fantasy / historical list generators with a fixed random seed"},
        "targets": TARGETS,
        "selection": "SHA-256 keyed sample of distinct names (ranked names first, unranked top-up); places are GeoNames PPL of 1,000-30,000 inhabitants (widened to any populated place when fewer than the target); trailing '(gloss)' removed; NFC, deduplicated, sorted by code point; no profanity filter.",
        "limitations": "Present-day name frequencies and places, no historical authenticity; some fantasy styles ignore gender; see each culture's fit and note.",
        "cultures": cultures,
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(doc, ensure_ascii=False, indent=1, sort_keys=True) + "\n", encoding="utf-8", newline="\n")

    print(f"wrote {out}: {out.stat().st_size} bytes, content_sha256 {doc['content_sha256']}")
    for cid in ids:
        e = cultures[cid]
        print(f"{cid:22s} {e['fit']:8s} M{len(e['forenames_male']):4d} F{len(e['forenames_female']):4d} "
              f"S{len(e['surnames']):4d} P{len(e['places']):4d}")
    print("nearest:", ", ".join(x for x in ids if cultures[x]["fit"] == "nearest"))
    print(f"thin pools (<{THIN}):", ", ".join(f"{x}.{p}={n}" for x, p, n in thin) or "none")


if __name__ == "__main__":
    main()
