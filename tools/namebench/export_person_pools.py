"""Export frozen person and place name pools (per Magna Urbis culture) from Namebench.

Reads the sibling Namebench database strictly read-only (SQLite URI mode=ro&immutable=1; Namebench's
own NameDB.conn would switch the file to WAL mode, so it is never opened through that path).
Selection is deterministic: SHA-256 keyed samples of distinct names, never ORDER BY RANDOM() and never
the unseeded `random`. Fantasy pools come from Namebench's FantasyGenerator driven by
`random.seed(<fixed string>)`, then reduced by the same keyed sample.

Period credibility. Namebench's real-name tables are present-day data (GeoNames, Forebears and a
phone-book corpus): they carry no date. The pools therefore use, in this order of preference, and
declare per culture and per pool which one applies (field `period`, summarised in `period_summary`):
  period-list         Namebench's historical-era generators (Normans, Franks, Vikings, Romans, ...):
                      name arrays read directly, no random draw.
  traditional-list    Namebench's ethnic generators (traditional names of one people, undated).
  ranked-core+lexicon the most frequent present-day names of the culture's countries that also occur in
                      a historical-era list (the "lexicon"): a frequency-and-attestation proxy.
  ranked-core         the most frequent present-day names only (rank cut, clean spelling).
  modern-unranked     present-day names with no frequency rank, cleaned: no period signal at all.
  fantasy             Namebench fantasy generators: no real-world period.
Surnames get no lexicon test: the most frequent surnames of a country are mostly centuries old.

Each culture also carries `inheritance`, the rule that gives a person a family part (read by
`display_names.rs`): patrilineal, matrilineal, patronymic (form per culture) or given_only.

Usage (Namebench virtual environment, run from the repository root):
  ../namebench/.venv/Scripts/python.exe tools/namebench/export_person_pools.py ../namebench \
      fixtures/naming/person-pools.v1.json
"""
import hashlib
import itertools
import json
import random
import re
import sqlite3
import subprocess
import sys
import unicodedata
from pathlib import Path

VERSION = "chronos-person-pools-v2"
SOURCE_COMMIT = "2820b50"
TARGETS = {"forenames_male": 300, "forenames_female": 300, "surnames": 300, "places": 200}
POOL_ROLE = {"forenames_male": "m", "forenames_female": "f", "surnames": "s"}
FANTASY_DRAWS = 4000
THIN = 50          # a pool below this is reported as thin
FORE_CUT = 700     # forename rank cut for the ranked-core proxy (rank within one country)
SURN_CUT = 3000    # surname rank cut
MIN_RANKED = 80    # fewer ranked names than this: top up with unranked ones
LATIN_MIN = 150   # at least this many Latin-script names: the pool keeps only Latin script
MIN_LEXICON = 60   # fewer lexicon-attested names than this: drop the lexicon test

# ---------------------------------------------------------------------------------------------
# Namebench historical and ethnic lists: which array plays which role. Default: nm1 male forenames,
# nm2 female forenames, nm3 surnames (checked by sampling). A role is a list of array sequences; the
# names of a sequence are concatenated with `sep` (e.g. Edo-period given names are two morphemes).
# `era` is written to the file header. Kind: "period" (dated generator) or "traditional".
DEF = {"m": [["nm1"]], "f": [["nm2"]], "s": [["nm3"]], "sep": "", "kind": "traditional", "era": "undated"}


def _r(kind, era, **over):
    d = dict(DEF)
    d.update(kind=kind, era=era, **over)
    return d


NONE_S = {"s": []}
ROLES = {
    # --- dated generators ---------------------------------------------------------------------
    "real.normans": _r("period", "Normans, c. 1000-1300"),
    "real.frankishs": _r("period", "Franks, c. 500-900", s=[]),
    "real.old_high_germans": _r("period", "Old High German, c. 700-1100", s=[]),
    "real.anglo_saxons": _r("period", "Anglo-Saxons, c. 450-1066", s=[["nm5"]]),
    "real.gothics": _r("period", "Goths, c. 250-700", s=[]),
    "real.suebis": _r("period", "Suebi / Visigothic Iberia, c. 400-700", s=[]),
    "real.ancient_greeks": _r("period", "Ancient and Byzantine Greek", s=[]),
    "real.romans": _r("period", "Roman Republic and Empire (praenomen/cognomen, gentilicia)",
                      m=[["nm3"], ["nm1"]], f=[["nm4"], ["nm5"]], s=[["nm2"]]),
    "real.vikings": _r("period", "Viking Age Norse", f=[["nm3"]], s=[["nm2"]]),
    "real.shakespeareans": _r("period", "Elizabethan England (Shakespeare)"),
    "real.edo_japaneses": _r("period", "Edo-period Japan", m=[["nm1", "nm1"]], f=[["nm2", "nm3"]], s=[["nm4"]]),
    "real.sumerians": _r("period", "Sumer, Babylon and Assyria", s=[["nm1"]]),
    "real.babylonians": _r("period", "Sumer, Babylon and Assyria", s=[["nm1"]]),
    "real.assyrians": _r("period", "Sumer, Babylon and Assyria", s=[["nm1"]]),
    "real.egyptians": _r("period", "Ancient Egypt", s=[["nm1"]]),
    "real.mayans": _r("period", "Classic and Postclassic Maya", s=[["nm1"]]),
    "real.aztecs": _r("period", "Nahua / Aztec", m=[["nm1"], ["nm5", "nm6"]], f=[["nm4"], ["nm2", "nm3"]], s=[["nm4"]]),
    "real.biblicals": _r("period", "Biblical names, used by medieval Christians, Jews and Muslims", s=[]),
    # --- traditional lists --------------------------------------------------------------------
    "real.italians": _r("traditional", "Italian saints and surnames"),
    "real.catalans": _r("traditional", "Catalan"),
    "real.celtic_bretons": _r("traditional", "Breton"),
    "real.irishs": _r("traditional", "Irish (Gaelic forms)", f=[["nm3"]], s=[["nm2"], ["nm4"]]),
    "real.russians": _r("traditional", "Russian", f=[["nm4"]], s=[["nm2"]]),
    "real.slavics": _r("traditional", "Slavic"),
    "real.greeks": _r("traditional", "Greek"),
    "real.turkishs": _r("traditional", "Turkish"),
    "real.persians": _r("traditional", "Persian"),
    "real.muslims": _r("traditional", "Arabic and Islamic"),
    "real.moroccans": _r("traditional", "Moroccan"),
    "real.amazighs": _r("traditional", "Amazigh (Berber)"),
    "real.georgians": _r("traditional", "Georgian"),
    "real.kazakhs": _r("traditional", "Kazakh"),
    "real.nepaleses": _r("traditional", "Nepali"),
    "real.khmers": _r("traditional", "Khmer"),
    "real.hindus": _r("traditional", "Hindu (Sanskritic)"),
    "real.telugus": _r("traditional", "Telugu (Dravidian)"),
    "real.punjabis": _r("traditional", "Punjabi"),
    "real.koreans": _r("traditional", "Korean"),
    "real.shonas": _r("traditional", "Shona"),
    "real.zulus": _r("traditional", "Zulu", s=[]),
    "real.hausas": _r("traditional", "Hausa", s=[]),
    "real.swahilis": _r("traditional", "Swahili", s=[]),
    "real.yorubas": _r("traditional", "Yoruba"),
    "real.hawaiians": _r("traditional", "Hawaiian"),
    "real.tibetans": _r("traditional", "Tibetan (names are mostly unisex)", m=[["nm1"]], f=[["nm1"]], s=[["nm2"]]),
    "real.inuits": _r("traditional", "Inuit (one flat list)", m=[["nm1"]], f=[["nm1"]], s=[["nm1"]]),
    "real.aboriginals": _r("traditional", "Aboriginal Australian", s=[["nm1"], ["nm2"]]),
    "real.vietnameses": _r("traditional", "Vietnamese", m=[["nm2", "nm3"]], f=[["nm4", "nm5"]], s=[["nm1"]], sep=" "),
    "real.burmese_myanmars": _r("traditional", "Burmese syllables", m=[["nm1", "nm1"]], f=[["nm2", "nm2"]],
                               s=[["nm1", "nm2"]], sep=" "),
}
LEXICON_STYLES = ["real.normans", "real.frankishs", "real.old_high_germans", "real.anglo_saxons", "real.gothics",
                  "real.suebis", "real.ancient_greeks", "real.romans", "real.biblicals", "real.italians",
                  "real.catalans", "real.celtic_bretons", "real.vikings", "real.shakespeareans", "real.irishs"]

# Native American list entries end with a "(tribe)" tag: used to split the one list per culture.
TRIBES = {
    "pueblo": ["Hopi", "Zuni", "Laguna"],
    "plains": ["Sioux", "Dakota", "Cheyenne", "Kiowa", "Pawnee", "Arapaho", "Blackfoot", "Black Foot", "Crow",
               "Comanche", "Omaha", "Osage", "Ponca", "Oto"],
    "woodland": ["Iroquois", "Seneca", "Cherokee", "Algonquin", "Algonquian", "Abnaki", "Potawatomi", "Chippewa",
                 "Ojibwa", "Ojibway"],
    "southeast": ["Cherokee", "Choctaw", "Seminole", "Timucua", "Osage", "Sauk"],
    "northwest": ["Nez Perce", "Chinook", "Coos", "Yakima", "Cayuse", "Shoshone", "Paiute", "Miwok", "Moquelumnan"],
}

# Place-name filters for GeoNames pools whose region was colonised.
COLONIAL = re.compile(r"^(San|Santa|Santo|Santiago|Nueva|Nuevo|Colonia|Ejido|Rancho|El|La|Los|Las|Del|Villa|Ciudad)\b")
PLACE_FILTERS = {
    "nocol": lambda n: not COLONIAL.search(n),
    "maori": lambda n: len(n) >= 4 and bool(MAORI.match(n.lower().replace(" ", ""))),
    "nahuatl": lambda n: " " not in n and not COLONIAL.search(n) and re.search(r"(tl|tlan|tlán|tepec|apan|pan|hua|huac|tla|lco|yan)$", n.lower()),
}
# Maori phonotactics: open syllables of (h k m n p r t w ng wh) + vowel(s), at least four letters.
MAORI = re.compile(r"^(?:(?:ng|wh|[hkmnprtw])?[aeiouāēīōū]{1,2})+$")
NAME_FILTERS = {
    "maori": lambda n: len(n) >= 4 and bool(MAORI.match(n.lower().replace(" ", ""))),
}

# ---------------------------------------------------------------------------------------------
# Inheritance rules, in data (read by display_names.rs).
PAT = {"rule": "patrilineal"}
MAT = {"rule": "matrilineal"}
GIV = {"rule": "given_only"}
FAM1 = dict(PAT, order="family_first")


def patro(male, female, rules=None, note=""):
    form = {"male": male, "female": female}
    if rules:
        form["stem_rules"] = rules
    out = {"rule": "patronymic", "form": form}
    if note:
        out["note"] = note
    return out


IBN = patro("ibn {f}", "bint {f}")
BIN = patro("bin {f}", "binti {f}")
OGLU = patro("{f} oğlu", "{f} kızı")
SSON = patro("{f}sson", "{f}sdottir")
MAC = patro("mac {f}", "ingen {f}")
RUS = patro("{f}ovich", "{f}ovna", [["iy", "ie", "vich", "vna"], ["ey", "eye", "vich", "vna"], ["y", "ie", "vich", "vna"],
                                    ["ya", "i", "ch", "chna"], ["a", "i", "ch", "chna"]])

# mu_culture_id -> dict(names atoms, places atoms, surnames atoms (optional override), inheritance, fit, note).
# Name atoms (one string each; a pool is the keyed sample of the union of its atoms):
#   L:<style>                    historical or ethnic list, roles from ROLES
#   T:<style>:<culture tribes>   Native American list restricted to the tribes of TRIBES[...]
#   R:zone:<ref>|cc:<ISO,..>[|lex][|uni][|maori]   ranked-core present-day names
#   G:<fantasy style>            seeded Namebench fantasy generator
# Place atoms: P:zone:<ref>|cc:<ISO,..>[|nocol|nahuatl] (GeoNames), G:<style>, LP:<style> (list names).
M = {}


def c(cid, names, places, inh, fit="exact", pfit=None, note="", sur=None, script=None):
    M[cid] = dict(names=names, places=places, sur=sur, inh=inh, fit=fit, pfit=pfit or fit, note=note, script=script)


c("al-andalus", ["L:real.muslims", "L:real.moroccans", "R:zone:style.arabic_maghreb"], ["P:cc:ES", "P:zone:style.arabic_maghreb"],
  IBN, "nearest", "nearest", "No Andalusi style; Arabic traditional lists and Maghreb frequent names. Places mix Iberian and Maghreb towns.")
c("aztec", ["L:real.aztecs"], ["P:cc:MX|nahuatl"], GIV, "exact", "nearest",
  "Nahua given names; places are Mexican GeoNames places with a Nahuatl ending and no Spanish colonial prefix.")
c("bagan", ["L:real.burmese_myanmars", "R:cc:MM"], ["P:cc:MM"], GIV, "nearest", "nearest",
  "Burmese names are syllable compounds without a surname.")
c("barbarian", ["G:fantasy.barbarians"], ["G:fantasy.barbarians"], GIV, "exact")
c("barbarian-celtic", ["L:real.irishs", "L:real.celtic_bretons"], ["P:zone:overlay.celtic_ancient"], MAC, "exact")
c("barbarian-norse", ["L:real.vikings"], ["P:cc:NO,SE,DK,IS"], SSON, "exact",
  note="Viking generator has no town list; places are Scandinavian and Icelandic GeoNames places.")
c("bastide", ["L:real.normans", "L:real.catalans", "R:zone:style.french|lex"], ["P:zone:style.french"], PAT, "nearest",
  note="No Occitan style; Norman and Catalan lists plus frequent French names attested in historical lists.")
c("budj-bim", ["L:real.aboriginals"], ["LP:real.aboriginals"], GIV, "exact",
  note="Places reuse the Aboriginal name list (no Aboriginal gazetteer in Namebench).")
c("byzantine-greek", ["L:real.ancient_greeks", "L:real.biblicals"], ["P:cc:GR", "G:towns_and_cities.ancient_greek_towns"], PAT,
  "exact", sur=["L:real.greeks", "R:zone:style.greek"])
c("caucasian-highland", ["L:real.georgians", "R:cc:GE"], ["P:cc:GE"], PAT, "exact")
c("celtic-oppidum", ["L:real.irishs", "L:real.celtic_bretons"], ["P:zone:overlay.celtic_ancient"], PAT, "exact")
c("chinese", ["R:cc:CN,TW,SG,HK|cut=300|noup"], ["P:cc:CN,TW"], FAM1, "exact", script="latin",
  note="No dated Chinese list in Namebench: frequent present-day names only, romanised (most Chinese-region forenames in the database are Han characters, so the Latin pool is small).")
c("dravidian-temple", ["L:real.telugus", "L:real.hindus"], ["P:cc:IN,LK"], patro("{f}", "{f}"), "exact")
c("drow-enclave", ["G:pathfinder.drows"], ["G:towns_and_cities.elven_citys"], MAT, "exact", "nearest",
  "Places use the elven city generator.")
c("duergar-hold", ["G:pathfinder.dwarfs"], ["G:towns_and_cities.dwarven_citys"], PAT, "nearest",
  note="Dwarf styles stand in for duergar.")
c("dwarven", ["G:fantasy.dwarfs"], ["G:towns_and_cities.dwarven_citys"], PAT, "exact")
c("egyptian-planned", ["L:real.egyptians"], ["P:cc:EG"], patro("sa {f}", "sat {f}"), "exact", "nearest",
  "Ancient Egyptian names with filiation (sa = son of, sat = daughter of); places are present-day Egyptian places.")
c("elven", ["G:fantasy.elfs"], ["G:towns_and_cities.elven_citys"], GIV, "exact")
c("ethiopian-highland", ["R:zone:style.amhari_tigrinya|uni"], ["P:zone:style.amhari_tigrinya"], patro("{f}", "{f}"), "exact",
  note="Ethiopian naming: the second name is the father's given name.")
c("european-organic", ["L:real.normans", "L:real.old_high_germans", "L:real.italians", "L:real.catalans",
                       "L:real.shakespeareans", "R:zone:script.latin_west_europe|lex"],
  ["P:zone:script.latin_west_europe"], PAT, "nearest", note="Generic western Europe: medieval lists and attested frequent names.")
c("flemish-cloth-town", ["L:real.frankishs", "L:real.old_high_germans", "R:zone:style.dutch_flemish|lex"],
  ["P:zone:style.dutch_flemish"], PAT, "exact")
c("giant-hold", ["G:fantasy.giants"], ["G:fantasy.giants"], SSON, "exact")
c("gnomish", ["G:fantasy.gnomes"], ["G:fantasy.gnomes"], PAT, "exact")
c("goblin-warren", ["G:fantasy.goblins"], ["G:fantasy.goblins"], GIV, "exact")
c("great-zimbabwe", ["L:real.shonas", "R:cc:ZW"], ["P:cc:ZW"], PAT, "exact", note="Shona traditional names; surnames are totem-clan style.")
c("guarani-village", ["R:cc:PY|uni"], ["P:cc:PY"], GIV, "nearest", note="Paraguay pools; Guarani names are sparse in the database.")
c("hadhrami-tower", ["L:real.muslims", "R:zone:style.arabic_yemen_oman"], ["P:zone:style.arabic_yemen_oman"], BIN, "exact")
c("halfling", ["G:fantasy.hobbits"], ["G:towns_and_cities.fantasy_towns"], PAT, "exact", "nearest",
  "Places use the generic fantasy town generator.")
c("hanseatic", ["L:real.old_high_germans", "L:real.frankishs", "R:zone:style.german_de_at|lex"], ["P:zone:style.german_de_at"], PAT,
  "nearest", note="No Low German style; German.")
c("hausa-kano", ["L:real.hausas", "L:real.muslims"], ["P:zone:style.hausa"], patro("dan {f}", "'yar {f}"), "exact",
  note="Hausa and Islamic traditional lists; frequent Nigerian names are mostly modern and are left out.", sur=["L:real.muslims"])
c("himalayan-terraces", ["L:real.tibetans"], ["P:cc:NP"], GIV, "exact", "nearest",
  "Tibetan names; places are Nepali GeoNames places.")
c("inca", ["R:zone:style.quechua_inca"], ["P:zone:style.quechua_inca|nocol"], GIV, "exact",
  note="Quechua-region frequent names are mostly Spanish; no Inca list in Namebench.")
c("indian-temple", ["L:real.hindus", "R:zone:style.hindi"], ["P:zone:style.hindi"], PAT, "nearest", note="No Sanskrit style; Hindu traditional list and Hindi.")
c("indus-planned", ["L:real.hindus"], ["P:zone:style.punjabi"], GIV, "nearest", note="No Indus-era style; Sanskritic traditional list.")
c("inuit-camp", ["L:real.inuits"], ["P:cc:GL"], GIV, "exact", "nearest",
  "Inuit list is a single flat list shared by all name pools; places are Greenland GeoNames places.")
c("italian-comune", ["L:real.italians", "R:cc:IT|lex"], ["P:cc:IT"], PAT, "exact",
  sur=["R:cc:IT"])
c("japanese-jokamachi", ["L:real.edo_japaneses"], ["P:zone:overlay.japanese_feudal_edo"], FAM1, "exact")
c("javanese-courts", ["L:real.hindus", "R:cc:ID"], ["P:cc:ID"], GIV, "nearest", note="Sanskritic traditional list plus Indonesian names; no Javanese style.")
c("khmer", ["L:real.khmers", "R:cc:KH"], ["P:cc:KH"], FAM1, "exact")
c("korean", ["L:real.koreans", "R:zone:style.korean"], ["P:zone:style.korean"], FAM1, "exact")
c("kraal", ["L:real.zulus"], ["P:zone:style.zulu_xhosa"], PAT, "exact", sur=["R:zone:style.zulu_xhosa"],
  note="Traditional Zulu names; surnames are frequent Zulu and Xhosa clan names.")
c("ksar-oasis", ["L:real.amazighs", "R:zone:style.amazigh_berber"], ["P:zone:style.amazigh_berber"], patro("u {f}", "ult {f}"), "nearest",
  note="Berber oasis settlements; Amazigh style.")
c("levantine-tell", ["L:real.muslims", "L:real.biblicals", "R:zone:style.arabic_levant"], ["P:zone:style.arabic_levant"], IBN, "exact")
c("lizardfolk-swamp", ["G:warhammer.lizardmens"], ["G:warhammer.lizardmens"], GIV, "nearest", note="Lizardmen stand in for lizardfolk.")
c("maori-pa", ["R:cc:NZ|uni|maori"], ["P:cc:NZ|maori"], GIV, "nearest",
  note="New Zealand pools filtered by Maori phonotactics (open syllables of h k m n p r t w ng wh).")
c("maya", ["L:real.mayans"], ["P:cc:GT,BZ|nocol"], PAT, "exact", "nearest", "Maya names; places are Guatemalan and Belizean GeoNames places.")
c("medina", ["L:real.muslims", "R:zone:macro.arabic"], ["P:zone:macro.arabic"], IBN, "exact")
c("merfolk-tidal", ["G:fantasy.mermaids"], ["G:towns_and_cities.underwater_citys"], MAT, "nearest",
  note="Mermaid names mix mythological and ordinary names.")
c("mesopotamian", ["L:real.sumerians", "L:real.babylonians", "L:real.assyrians"], ["P:cc:IQ"], patro("mar {f}", "mart {f}"), "exact", "nearest",
  "Sumerian, Babylonian and Assyrian lists (mar = son of); places are present-day Iraqi places.")
c("mississippian", ["T:real.native_americans:southeast"], ["T:real.native_americans:southeast"], MAT, "nearest",
  note="Southeastern tribes of the Pan-Native-American list; no Mississippian style.")
c("mongol-ordu", ["R:cc:MN|uni"], ["P:cc:MN"], patro("{f}iin", "{f}iin"), "exact",
  note="Mongolian pools (unisex: the database has almost no gendered Mongolian forenames).")
c("myconid-colony", ["G:miscellaneous.fungis"], ["G:miscellaneous.fungis"], GIV, "nearest", note="Mushroom species name parts; no myconid style.")
c("native-iroquoian", ["T:real.native_americans:woodland"], ["T:real.native_americans:woodland"], MAT, "nearest",
  note="Eastern Woodlands tribes of the Pan-Native-American list.")
c("native-plains", ["T:real.native_americans:plains"], ["T:real.native_americans:plains"], PAT, "nearest",
  note="Plains tribes of the Pan-Native-American list.")
c("native-pueblo", ["T:real.native_americans:pueblo"], ["T:real.native_americans:pueblo"], MAT, "nearest",
  note="Hopi, Zuni and Laguna entries of the Pan-Native-American list.")
c("necropolis", ["L:real.egyptians"], ["P:cc:EG"], GIV, "nearest", note="Ancient Egyptian names stand in; no undead style.")
c("newar", ["L:real.nepaleses", "R:cc:NP|uni"], ["P:cc:NP"], PAT, "nearest", note="Nepali lists; Newar names are sparse in the database.")
c("nomad-camp", ["L:real.kazakhs", "R:zone:style.central_asia_turkic"], ["P:zone:style.central_asia_turkic"], patro("{f} uly", "{f} kyzy"),
  "nearest", note="Central Asian Turkic.")
c("norse-ringfort", ["L:real.vikings"], ["P:cc:NO,SE,DK,IS"], SSON, "exact",
  note="Viking generator has no town list; places are Scandinavian and Icelandic GeoNames places.")
c("nubian-fortified", ["L:real.muslims", "R:zone:style.arabic_egypt_sudan"], ["P:zone:style.arabic_egypt_sudan"], IBN, "nearest",
  note="No Nubian style; Egypt and Sudan Arabic.")
c("oasis-caravan", ["L:real.muslims", "R:zone:style.arabic_gulf"], ["P:zone:style.arabic_gulf"], IBN, "nearest", note="Gulf Arabic.")
c("orcish", ["G:pathfinder.orcs"], ["G:towns_and_cities.orcish_citys"], patro("gro-{f}", "gra-{f}"), "exact")
c("ottoman", ["L:real.turkishs", "L:real.persians", "L:real.muslims", "R:zone:style.turkish"], ["P:zone:overlay.ottoman"], OGLU, "exact")
c("persian", ["L:real.persians", "R:zone:style.persian"], ["P:zone:style.persian"], PAT, "exact")
c("pnw-plank-village", ["T:real.native_americans:northwest"], ["T:real.native_americans:northwest"], MAT, "nearest",
  note="Plateau, Coast and Californian tribes of the Pan-Native-American list; very few Northwest Coast entries.")
c("rajput-fort", ["L:real.hindus", "R:zone:style.hindi"], ["P:zone:style.hindi"], PAT, "nearest", note="Hindu traditional list and Hindi.",
  sur=["L:real.hindus", "L:real.punjabis", "R:zone:style.hindi"])
c("roman-core", ["L:real.romans"], ["P:zone:overlay.roman_latin_ancient"], PAT, "exact",
  note="Praenomina and cognomina as forenames, gentilicia as surnames; places from the Roman overlay countries.")
c("russian-kremlin", ["L:real.russians", "R:zone:style.russian"], ["P:zone:style.russian"], RUS, "exact")
c("rynek-town", ["L:real.slavics", "R:zone:style.polish|lex"], ["P:zone:style.polish"], PAT, "exact")
c("sahel", ["L:real.hausas", "L:real.muslims"], ["P:zone:macro.sahel"], PAT, "exact", sur=["R:zone:macro.sahel"],
  note="Hausa and Islamic traditional lists; surnames are frequent Sahelian family names.")
c("siberian-camp", ["L:real.kazakhs", "R:cc:MN|uni"], ["P:cc:MN"], PAT, "nearest",
  note="No Evenki or Yakut style; Kazakh list and Mongolian names stand in.")
c("slavic-gord", ["L:real.slavics", "R:zone:macro.slavic|lex"], ["P:zone:macro.slavic"], PAT, "exact", note="Places from the Slavic macro zone.")
c("stilt-town", ["L:real.muslims", "R:cc:MY"], ["P:cc:MY"], BIN, "nearest", note="Malaysian pools plus Islamic traditional list.")
c("swahili-stone-town", ["L:real.swahilis", "L:real.muslims"], ["P:zone:style.swahili"], BIN, "exact", sur=["L:real.muslims"])
c("tibetan-monastery", ["L:real.tibetans"], ["P:cc:NP"], GIV, "exact", "nearest", "Tibetan names; places are Nepali GeoNames places.")
c("venetian-lagoon", ["L:real.italians", "L:real.romans", "R:cc:IT|lex"], ["P:cc:IT"], PAT, "nearest",
  note="No Venetian style; Italian.", sur=["R:cc:IT"])
c("vietnamese-citadel", ["L:real.vietnameses", "R:zone:style.vietnamese"], ["P:zone:style.vietnamese"], FAM1, "exact")
c("wizard-city", ["G:fantasy.wizards"], ["G:fantasy.wizards"], PAT, "exact")
c("yoruba", ["L:real.yorubas"], ["P:zone:style.yoruba"], PAT, "exact", sur=["L:real.yorubas", "R:zone:style.yoruba"],
  note="Traditional Yoruba names; surnames add frequent Yoruba-region surnames.")

PERIOD_BASES = {
    "period-list": "Namebench historical-era generator lists, read directly (strongest period signal)",
    "traditional-list": "Namebench ethnic generator lists of traditional names (undated)",
    "ranked-core+lexicon": "frequent present-day names that also occur in a historical-era list",
    "ranked-core": "frequent present-day names only (rank cut, clean spelling)",
    "modern-unranked": "present-day names with no frequency rank: no period signal",
    "fantasy": "Namebench fantasy generators: no real-world period",
    "gazetteer": "present-day GeoNames places (places only)",
}
STRENGTH = ["period-list", "traditional-list", "ranked-core+lexicon", "ranked-core", "modern-unranked", "fantasy", "gazetteer"]


def sha256_file(path, chunk=1 << 22):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while block := f.read(chunk):
            h.update(block)
    return h.hexdigest()


def key(text):
    return hashlib.sha256(f"{VERSION}:{text}".encode()).hexdigest()


GLOSS = re.compile(r"\s*\([^)]*\)\s*$")
TAG = re.compile(r"\(([^)]*)\)\s*$")
PARTICLES = {"de", "del", "della", "di", "da", "dal", "van", "von", "der", "den", "ten", "ter", "le", "la", "el", "al", "bin",
             "ben", "ibn", "ap", "ab", "mac", "mc", "o", "ó", "of", "dos", "das", "do", "du", "des", "af", "av", "bint", "abu"}


def fold(text):
    d = unicodedata.normalize("NFD", text.lower())
    return "".join(ch for ch in d if ch.isalpha() and not unicodedata.combining(ch))


def accept(n, kind, short_ok=False):
    """Spelling test shared by every pool: letters, one hyphen or apostrophe per token, no repeats."""
    if not n or any(ch in n for ch in "?�"):
        return False
    if not all(ch.isalpha() or ch in " -'’" or unicodedata.combining(ch) for ch in n):
        return False
    toks = n.split(" ")
    if len(set(toks)) != len(toks) or len(toks) > (2 if kind == "fore" else 4):
        return False
    if len(n) > 24 or len(n) < (2 if short_ok else 3):
        return False
    if n.isupper() and len(n) > 3:
        return False
    for i, tok in enumerate(toks):
        first = next((ch for ch in tok if ch.isalpha()), "")
        if first and first.lower() != first.upper() and first.islower() and not (i < len(toks) - 1 and tok.lower() in PARTICLES):
            return False
    return True


def is_latin(n):
    return all(unicodedata.name(ch, "").startswith("LATIN") for ch in n if ch.isalpha())


VOWEL = re.compile(r"[aeiouyáàâäãåāæéèêëēíìîïīóòôöõøōœúùûüūýÿ]", re.I)


def clean(names, kind="fore", short_ok=False, gloss=True):
    """NFC, strip, drop a trailing '(gloss)', collapse spaces, test spelling, deduplicate, sort by code point."""
    out = set()
    for n in names:
        n = unicodedata.normalize("NFC", n)
        if gloss:
            n = GLOSS.sub("", n)
        n = re.sub(r"\s+", " ", n).strip()
        if accept(n, kind, short_ok) and (not is_latin(n) or VOWEL.search(n)):
            out.add(n)
    return sorted(out)


def keyed(values, label, limit):
    return sorted(sorted(values, key=lambda v: key(f"{label}:{v}"))[:limit])


def cap(name):
    return " ".join(t[:1].upper() + t[1:] if t.islower() else t for t in name.split(" "))


class Namebench:
    def __init__(self, root):
        sys.path.insert(0, str(root))
        from namegen.db import NameDB
        from namegen.zones import OnomasticResolver
        from namegen.strategies.db_lookup import _SHORT_NAME_COUNTRIES
        db_path = root / "data" / "names.db"
        self.conn = sqlite3.connect(f"file:{db_path.as_posix()}?mode=ro&immutable=1", uri=True)
        self.conn.row_factory = sqlite3.Row
        self.db = NameDB(db_path)
        self.db._conn = self.conn  # read-only connection; skips NameDB's WAL pragma
        self.zones = OnomasticResolver()
        self.short = _SHORT_NAME_COUNTRIES
        self.db_path = db_path
        self.gens = {}
        self._arrays = {}
        self._lexicon = None

    def arrays(self, style):
        if style not in self._arrays:
            self._arrays[style] = {n: json.loads(v) for n, v in self.conn.execute(
                "SELECT array_name, values_json FROM fantasy_arrays WHERE style_key=?", (style,))}
        return self._arrays[style]

    def codes(self, spec):
        kind, _, arg = spec.partition(":")
        if kind == "cc":
            return arg.split(",")
        codes = self.zones.resolve(arg)
        if not codes:
            raise SystemExit(f"unknown zone {arg}")
        return sorted(codes)

    def list_names(self, style, role):
        """Names of one role of a historical or ethnic list, read straight from its arrays."""
        spec = ROLES[style]
        arrs = self.arrays(style)
        if not arrs:
            raise SystemExit(f"no arrays for {style}")
        out = []
        for seq in spec[role]:
            for combo in itertools.product(*[arrs[a] for a in seq]):
                parts = [p.strip() for p in combo if p and p.strip()]
                out.append(cap(spec["sep"].join(parts)) if len(seq) > 1 else spec["sep"].join(parts))
        return out

    def lexicon(self):
        if self._lexicon is None:
            self._lexicon = {fold(n) for style in LEXICON_STYLES for role in ("m", "f")
                             for n in self.list_names(style, role)}
        return self._lexicon

    def tribe_names(self, style, tribes, role):
        arrs = self.arrays(style)
        out = []
        for name in arrs["nm1" if role in ("m", "s") else "nm2"]:
            m = TAG.search(name)
            if m and m.group(1).strip() in tribes:
                out.append(name)
        return out

    def fantasy(self, style, name_type, gender, label):
        from namegen.strategies.fantasy import FantasyGenerator
        gen = self.gens.setdefault(style, FantasyGenerator(self.db, style))
        random.seed(f"{VERSION}:{label}")
        return gen.generate(name_type, FANTASY_DRAWS, gender)

    def ranked(self, codes, name_type, gender, flags):
        """Present-day names of the countries: the most frequent first; returns (names, tag)."""
        q = ",".join("?" for _ in codes)
        cut = FORE_CUT if name_type == "forename" else SURN_CUT
        for fl in flags:
            if fl.startswith("cut=") and name_type == "surname":
                cut = int(fl[4:])
        short = bool(set(codes) & self.short)
        kind = "fore" if name_type == "forename" else "sur"
        unisex = "uni" in flags
        gcl, gpar = "", []
        if name_type == "forename" and gender and not unisex:
            gcl, gpar = "AND gender=? ", [gender]
        base = f"SELECT DISTINCT name FROM names WHERE country_iso IN ({q}) AND name_type=? "
        names = clean((r[0] for r in self.conn.execute(
            base + gcl + "AND rank IS NOT NULL AND rank<=?", list(codes) + [name_type] + gpar + [cut])), kind, short)
        tag = "ranked-core"
        if name_type == "forename" and "lex" in flags:
            lex = [n for n in names if fold(n) in self.lexicon()]
            if len(lex) >= MIN_LEXICON:
                names, tag = lex, "ranked-core+lexicon"
        if "maori" in flags:
            names = [n for n in names if NAME_FILTERS["maori"](n)]
        if len(names) < MIN_RANKED and tag == "ranked-core" and "noup" not in flags:
            gcl2, gpar2 = ("AND (gender=? OR gender IS NULL) ", [gender]) if gcl else ("", [])
            more = clean((r[0] for r in self.conn.execute(
                base + gcl2, list(codes) + [name_type] + gpar2)), kind, short)
            if "maori" in flags:
                more = [n for n in more if NAME_FILTERS["maori"](n)]
            have = set(names)
            extra = [n for n in more if n not in have]
            if extra:
                names = names + keyed(extra, f"unranked:{','.join(codes)}:{name_type}:{gender}", 600)
                tag = "modern-unranked"
        return names, tag

    def places(self, codes, label, limit, flt):
        q = ",".join("?" for _ in codes)
        sql = f"SELECT DISTINCT name FROM places WHERE country_iso IN ({q}) AND feature_class='P' "
        rows = clean((r[0] for r in self.conn.execute(
            sql + "AND feature_code='PPL' AND population BETWEEN 1000 AND 30000", codes)), "place")
        if len(rows) < limit:
            rows = clean(rows + [r[0] for r in self.conn.execute(sql, codes)], "place")
        if flt:
            rows = [n for n in rows if PLACE_FILTERS[flt](n)]
        return rows


def atom_values(nb, atom, pool, cid):
    """(names, tag) of one atom for one pool."""
    kind, _, rest = atom.partition(":")
    label = f"{cid}:{pool}"
    if pool == "places":
        if kind == "P":
            spec, _, flt = rest.partition("|")
            return nb.places(nb.codes(spec), label, TARGETS["places"], flt), "gazetteer"
        if kind == "G":
            return clean(nb.fantasy(rest, "place", None, label), "place"), "fantasy"
        if kind == "LP":
            arrs = nb.arrays(rest)
            return clean((n for a in sorted(arrs) for n in arrs[a]), "place"), "traditional-list"
        if kind == "T":
            style, _, culture = rest.partition(":")
            names = [n for role in ("m", "f") for n in nb.tribe_names(style, TRIBES[culture], role)]
            return clean(names, "place"), "traditional-list"
        raise SystemExit(f"bad place atom {atom}")
    role = POOL_ROLE[pool]
    name_type = "surname" if role == "s" else "forename"
    gender = {"m": "M", "f": "F"}.get(role)
    short = False
    if kind == "L":
        names = nb.list_names(rest, role)
        return clean(names, "sur" if role == "s" else "fore", True), ROLES[rest]["kind"] + "-list" if ROLES[rest]["kind"] == "traditional" else "period-list"
    if kind == "T":
        style, _, culture = rest.partition(":")
        names = nb.tribe_names(style, TRIBES[culture], role)
        return clean(names, "sur" if role == "s" else "fore", True), "traditional-list"
    if kind == "R":
        spec, *flags = rest.split("|")
        return nb.ranked(nb.codes(spec), name_type, gender, set(flags))
    if kind == "G":
        names = nb.fantasy(rest, name_type, gender, label)
        return clean(names, "sur" if role == "s" else "fore", True), "fantasy"
    raise SystemExit(f"bad atom {atom}")


def weakest(tags):
    return max(tags, key=STRENGTH.index) if tags else "none"


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

    nb = Namebench(root)
    cultures, thin = {}, []
    for cid in ids:
        spec = M[cid]
        entry = {"fit": spec["fit"], "places_fit": spec["pfit"], "note": spec["note"], "inheritance": spec["inh"],
                 "style": {"names": spec["names"], "surnames": spec["sur"] or spec["names"], "places": spec["places"]}}
        prov, pools, period = {}, {}, {}
        for pool in TARGETS:
            atoms = spec["places"] if pool == "places" else (spec["sur"] if pool == "surnames" and spec["sur"] else spec["names"])
            results = [(atom_values(nb, atom, pool, cid)) for atom in atoms]
            if pool != "places" and any(t != "modern-unranked" for _, t in results):
                results = [r for r in results if r[1] != "modern-unranked"]  # unranked data never dilutes a list
            origin, seen = {}, {}
            for vals, tag in results:
                for v in set(vals):
                    seen[v] = seen.get(v, 0) + 1
                    if v not in origin or STRENGTH.index(tag) < STRENGTH.index(origin[v]):
                        origin[v] = tag
            latin = [v for v in origin if is_latin(v)]
            other = [v for v in origin if not is_latin(v)]
            # One script per pool: Latin (every list is romanised) unless too few; else the larger script.
            keep = latin if len(latin) >= LATIN_MIN or len(latin) >= len(other) else other
            if spec["script"] == "latin" and pool != "places":
                keep = latin
            origin = {v: origin[v] for v in keep}
            if not origin:
                raise SystemExit(f"{cid}.{pool} is empty")
            # Names found by two or more sources (a list and the frequent names) come first.
            core = [v for v in origin if seen[v] > 1]
            chosen = keyed(core, f"{cid}:{pool}:core", int(TARGETS[pool] * 0.6))
            rest = [v for v in origin if v not in set(chosen)]
            pools[pool] = sorted(set(chosen) | set(keyed(rest, f"{cid}:{pool}", TARGETS[pool] - len(chosen))))
            tags = {origin[v] for v in pools[pool]}
            prov[pool] = "derived" if tags <= {"fantasy", "period-list", "traditional-list"} else "imported"
            period[pool] = {t: sum(1 for v in pools[pool] if origin[v] == t) for t in sorted(tags, key=lambda x: STRENGTH.index(x))}
            if len(pools[pool]) < THIN:
                thin.append((cid, pool, len(pools[pool])))
        share = {}
        for p in ("forenames_male", "forenames_female"):
            for t, n in period[p].items():
                share[t] = share.get(t, 0) + n
        entry["period"] = {"forenames": max(share, key=lambda t: (share[t], -STRENGTH.index(t))), "pools": period}
        entry["provenance"] = prov
        entry.update(pools)
        cultures[cid] = entry

    content = json.dumps(cultures, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    eras = {s: {"kind": r["kind"], "era": r["era"]} for s, r in sorted(ROLES.items())}
    doc = {
        "schema": "chronos-person-pools-2",
        "generator": VERSION,
        "source": {"project": "namebench", "commit": SOURCE_COMMIT, "script": "tools/namebench/export_person_pools.py",
                   "database_bytes": nb.db_path.stat().st_size,
                   "code_sha256": {p: sha256_file(root / p) for p in (
                       "namegen/db.py", "namegen/styles.py", "namegen/strategies/fantasy.py",
                       "namegen/onomastic_zones.json")}},
        "content_sha256": hashlib.sha256(content.encode("utf-8")).hexdigest(),
        "content_sha256_of": "compact JSON of 'cultures' (sort_keys, separators ',' ':', UTF-8)",
        "provenance": {"imported": "values sampled from Namebench's real-name and GeoNames tables (present-day data)",
                       "derived": "values read from Namebench's historical, ethnic or fantasy list generators"},
        "targets": TARGETS,
        "selection": "SHA-256 keyed sample of the union of a pool's sources; real names are the most frequent ones (rank cut "
                     f"{FORE_CUT} for forenames, {SURN_CUT} for surnames), unranked names only when too few ranked ones exist; "
                     "places are GeoNames PPL of 1,000-30,000 inhabitants (widened to any populated place when fewer than the target); "
                     "trailing '(gloss)' removed; NFC; letters, spaces, hyphens and apostrophes only; deduplicated; sorted by code point.",
        "period_bases": PERIOD_BASES,
        "period_note": "Namebench has no dated name frequencies. Each culture declares, per pool, the basis of its names "
                       "(see `period` in each culture: the dominant basis of the forenames, and the count of each basis per pool; `period_summary` lists the dominant basis). Setting: medieval and fantasy, c. 500-1600.",
        "period_lists": eras,
        "inheritance_rules": {
            "patrilineal": "family name drawn from the pool for the oldest known ancestor through the father (else the mother)",
            "matrilineal": "same, walking through the mother first",
            "patronymic": "family part built from the father's forename with `form` (male/female templates, `{f}` = forename; "
                          "`stem_rules` = [ending, replacement, male suffix, female suffix], first match wins); a founder's father "
                          "is a keyed draw from the male forenames; the pool surnames still name houses",
            "given_only": "no family part on the person; the pool surnames still name houses and families",
            "order": "`family_first` prints the family part before the forename",
        },
        "period_summary": {cid: cultures[cid]["period"]["forenames"] for cid in ids},
        "limitations": "Frequencies and places are present-day; period lists are generator lists, not attested censuses; "
                       "some fantasy styles ignore gender; patronymic stems are approximate; see each culture's fit, period and note.",
        "cultures": cultures,
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(doc, ensure_ascii=False, indent=1, sort_keys=True) + "\n", encoding="utf-8", newline="\n")

    print(f"wrote {out}: {out.stat().st_size} bytes, content_sha256 {doc['content_sha256']}")
    for cid in ids:
        e = cultures[cid]
        print(f"{cid:22s} {e['fit']:8s} {e['inheritance']['rule']:11s} {e['period']['forenames']:20s} "
              f"M{len(e['forenames_male']):4d} F{len(e['forenames_female']):4d} S{len(e['surnames']):4d} P{len(e['places']):4d}")
    print("nearest:", ", ".join(x for x in ids if cultures[x]["fit"] == "nearest"))
    print(f"thin pools (<{THIN}):", ", ".join(f"{x}.{p}={n}" for x, p, n in thin) or "none")


if __name__ == "__main__":
    main()
