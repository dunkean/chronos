"""Verifie le catalogue social v2 (lot S1) et sa grille de couverture (bibliotheque standard seulement).

Usage : python tools/check_catalogue_v2.py [racine_du_depot]

L'outil ne lit que le depot de Chronos. La liste des systemes Pecking Order est celle qui est figee dans
archetype-coverage.v1.json (epinglee sur un commit) : le depot vivant de Pecking Order n'est jamais ouvert.
Code de sortie 0 si tout passe, 1 sinon.
"""
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent
DOC = ROOT / "docs" / "design" / "chronos-social-v2"
CAT_PATH = DOC / "culture-bindings.v1.json"
COV_PATH = DOC / "archetype-coverage.v1.json"
MD_PATH = DOC / "archetype-coverage.md"
PLAN_PATH = DOC / "village-population-plan.example.json"

SETTINGS = ["historical", "low_fantasy", "high_fantasy"]
RANK = {s: i for i, s in enumerate(SETTINGS)}
CLASS_RULES = {  # classe -> (slot_basis, duree moyenne par defaut)
    "activity_unit": ("expected_workforce", 20),
    "association": ("expected_active_stock", 30),
    "community": ("expected_active_stock", 50),
    "friend_circle": ("expected_active_stock", 5),
    "institution": ("site_occupation_anchor", None),
    "lineage_view": ("core_lineages", None),
    "household_view": ("provider_households", None),
}
VIEW_CLASSES = {"lineage_view", "household_view"}
CELL_CATEGORIES = {"subsistence_extraction", "craft_trade", "power", "justice_order", "military", "religion",
                   "knowledge_care", "culture_sociability", "crime_margins", "fantasy", "added"}
ARCH_FIELDS = ["id", "class", "slot_basis", "default_mean_lifetime_years", "membership_policy",
               "default_time_cost_bps", "activity_ids", "preemptible"]
ALIGN_FIELDS = {"morality_bps", "lawfulness_bps", "morality_spread_bps", "lawfulness_spread_bps",
                "conduct_tags", "default_visibility", "parameters_status"}
S1_ENGINE_KEYS = {"alignment", "setting_profile"}
BASE_SECTIONS = ["source_inventory", "families", "bindings", "governance_templates", "kinship_templates",
                 "demography_society_templates", "institution_coverage", "required_activity_coverage", "resolution_order"]
# seuils du README 27.2 et 27.4
SPEC_THRESHOLDS = {
    "lawless": {"morality_max_bps": -6000, "lawfulness_max_bps": -3000},
    "crooked": {"morality_max_bps": -2500},
    "ordinary": {"morality_below_bps": 2500},
    "upright": {"morality_below_bps": 7000},
}
SPEC_STATUS = {"conformity_proscribed_at_or_below_bps": -5000, "conformity_tolerated_at_or_below_bps": -1500}

errors = []
groups = []  # (nom, nombre d'erreurs)
_start = [0]


def need(cond, message):
    if not cond:
        errors.append(message)
    return bool(cond)


def end_group(name):
    groups.append((name, len(errors) - _start[0]))
    _start[0] = len(errors)


def canon(o):
    return json.dumps(o, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def sha(s):
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def load(path):
    raw = path.read_bytes()
    need(not raw.startswith(b"\xef\xbb\xbf"), f"{path.name}: BOM interdit")
    try:
        return json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as e:
        errors.append(f"{path.name}: illisible ({e})")
        return None


def conduct_class(m, l):
    """Fonction pure du README 27.2 ; premiere regle verifiee."""
    if m <= -6000 and l <= -3000:
        return "lawless"
    if m <= -2500:
        return "crooked"
    if m < 2500:
        return "ordinary"
    if m < 7000:
        return "upright"
    return "saintly"


def is_int(v):
    return isinstance(v, int) and not isinstance(v, bool)


def main():
    cat = load(CAT_PATH)
    cov = load(COV_PATH)
    if cat is None or cov is None:
        print("ECHEC : fichiers JSON illisibles")
        print("\n".join(errors))
        return 1
    end_group("lecture des fichiers")

    defs = cat["archetype_definitions"]
    ids = [a["id"] for a in defs]
    idset = set(ids)
    need(len(ids) == len(idset), "archetype_definitions: identifiants en double")
    by_id = {a["id"]: a for a in defs}

    # ------------------------------------------------ 1. systemes Pecking Order
    pin = cov["source_pin"]
    need(re.fullmatch(r"[0-9a-f]{40}", pin.get("commit", "")) is not None, "source_pin.commit: hash de 40 caracteres attendu")
    need(pin.get("commit", "").startswith("7dc27d0"), "source_pin.commit: la grille doit rester epinglee sur 7dc27d0")
    need(len(pin.get("files", [])) >= 3, "source_pin.files: au moins les trois fichiers de systemes")
    for f in pin.get("files", []):
        need(re.fullmatch(r"[0-9a-f]{64}", f.get("sha256", "")) is not None, f"source_pin.files {f.get('path')}: sha256 invalide")
    frozen = cov["frozen_systems"]
    fids = [s["id"] for s in frozen]
    need(len(fids) == len(set(fids)), "frozen_systems: identifiants en double")
    need(len(fids) == pin.get("system_count") == 242, f"frozen_systems: {len(fids)} systemes, 242 attendus")
    need(sha("\n".join(sorted(fids))) == pin.get("frozen_ids_sha256"), "frozen_systems: empreinte de la liste figee differente")
    fam_counts = {}
    for s in frozen:
        fam_counts[s["family"]] = fam_counts.get(s["family"], 0) + 1
    need(dict(sorted(fam_counts.items())) == pin.get("family_counts"), "frozen_systems: effectifs par categorie differents")
    need(len(fam_counts) == 10, "frozen_systems: 10 categories attendues")
    decs = cov["system_decisions"]
    dids = [d["system_id"] for d in decs]
    need(len(dids) == len(set(dids)), "system_decisions: systeme decide deux fois")
    for sid in sorted(set(fids) - set(dids)):
        errors.append(f"systeme Pecking Order sans decision : {sid}")
    for sid in sorted(set(dids) - set(fids)):
        errors.append(f"decision pour un systeme inconnu de la liste figee : {sid}")
    codes = cov.get("exclusion_reason_codes", {})
    n_map = n_exc = 0
    for d in decs:
        if d["decision"] == "map":
            n_map += 1
            need(d.get("archetype_id") in idset, f"decision {d['system_id']}: archetype absent {d.get('archetype_id')}")
        elif d["decision"] == "exclude":
            n_exc += 1
            need(d.get("reason_code") in codes, f"decision {d['system_id']}: code de raison inconnu {d.get('reason_code')}")
            need(bool(str(d.get("reason", "")).strip()), f"decision {d['system_id']}: raison d'exclusion vide")
        else:
            errors.append(f"decision {d['system_id']}: valeur inattendue {d['decision']}")
    end_group(f"systemes Pecking Order ({len(fids)} figes, {n_map} relies, {n_exc} exclus)")

    # ------------------------------------------------ 2. grille fonctionnelle
    cells = cov["grid_cells"]
    cids = [c["id"] for c in cells]
    need(len(cids) == len(set(cids)), "grid_cells: identifiants de case en double")
    md = MD_PATH.read_text(encoding="utf-8") if MD_PATH.exists() else ""
    need(bool(md), "archetype-coverage.md absent")
    n_cov = n_out = 0
    for c in cells:
        need(c.get("category") in CELL_CATEGORIES, f"case {c['id']}: categorie inconnue")
        need(c.get("origin") in ("brief", "added"), f"case {c['id']}: origine inconnue")
        if c.get("origin") == "added":
            need(bool(str(c.get("justification", "")).strip()), f"case ajoutee {c['id']}: justification manquante")
        if c.get("status") == "covered":
            n_cov += 1
            need(bool(c.get("archetype_ids")), f"case {c['id']}: aucune archetype pour une case couverte")
            for a in c.get("archetype_ids", []):
                need(a in idset, f"case {c['id']}: archetype absent {a}")
        elif c.get("status") == "out_of_scope":
            n_out += 1
            need(bool(str(c.get("reason", "")).strip()), f"case {c['id']}: raison de hors perimetre manquante")
        else:
            errors.append(f"case {c['id']}: case sans decision (statut {c.get('status')})")
        need(c["id"] in md, f"archetype-coverage.md: case {c['id']} absente de la synthese")
    n_brief = sum(1 for c in cells if c.get("origin") == "brief")
    need(n_brief == cov.get("brief_cell_count") == 86, f"grille: {n_brief} cases du cahier des charges, 86 attendues")
    need(pin["commit"] in md, "archetype-coverage.md: commit epingle absent")
    for a in cov["new_archetypes"]:
        need(a in idset, f"new_archetypes: {a} absent du catalogue")
        need(bool(cov["new_archetypes"][a].get("cells")), f"archetype ajoute {a}: aucune case ne le justifie")
    end_group(f"grille ({len(cells)} cases : {n_cov} couvertes, {n_out} hors perimetre)")

    # ------------------------------------------------ 3. references du catalogue
    gov = {g["id"] for g in cat["governance_templates"]}
    kin = {k["id"] for k in cat["kinship_templates"]}
    ovl = {o["id"]: o for o in cat["overlay_definitions"]}
    fam = {f["id"]: f for f in cat["families"]}
    soc = {t["id"] for t in cat["demography_society_templates"]}
    need(len(cat["bindings"]) == 79, f"bindings: {len(cat['bindings'])}, 79 attendus")
    need(len(fam) == 21, f"familles: {len(fam)}, 21 attendues")
    for f in cat["families"]:
        need(f["governance_template"] in gov, f"famille {f['id']}: gouvernance absente {f['governance_template']}")
        need(f["kinship_template"] in kin, f"famille {f['id']}: parente absente {f['kinship_template']}")
        for g in f["common_groups"]:
            need(g in idset, f"famille {f['id']}: archetype absent {g}")
    for b in cat["bindings"]:
        need(b["history_family_id"] in fam, f"raccord {b['mu_culture_id']}: famille absente")
        need(b["demography_society_template_id"] in soc, f"raccord {b['mu_culture_id']}: template social absent")
        for o in b["overlays"]:
            need(o in ovl, f"raccord {b['mu_culture_id']}: overlay absent {o}")
    for o in cat["overlay_definitions"]:
        for a in o["archetype_ids"]:
            need(a in idset, f"overlay {o['id']}: archetype absent {a}")
    base = cat["base_archetype_ids"]
    need(len(base) == 12 and len(set(base)) == 12, f"base_archetype_ids: {len(base)} identifiants, 12 distincts attendus")
    for a in base:
        need(a in idset, f"base_archetype_ids: archetype absent {a}")
    for a in ("base_criminal_network", "base_charitable_order"):
        need(a in base, f"base_archetype_ids: {a} manquant")
    end_group("references (archetypes, gouvernance, parente, overlays, raccords)")

    # ------------------------------------------------ 4. format des archetypes
    activities = cat["required_activity_coverage"]
    kinds = {k["kind"]: k for k in cat["name_kind_catalogue"]["kinds"]}
    for a in defs:
        aid = a["id"]
        for fld in ARCH_FIELDS:
            need(fld in a, f"archetype {aid}: champ {fld} manquant")
        need(a.get("setting") in SETTINGS, f"archetype {aid}: setting manquant ou invalide ({a.get('setting')})")
        need(a.get("class") in CLASS_RULES, f"archetype {aid}: classe inconnue {a.get('class')}")
        if a.get("class") not in CLASS_RULES:
            continue
        slot, life = CLASS_RULES[a["class"]]
        need(a["slot_basis"] == slot, f"archetype {aid}: slot_basis {a['slot_basis']} attendu {slot}")
        need(a["default_mean_lifetime_years"] == life, f"archetype {aid}: duree {a['default_mean_lifetime_years']} attendue {life}")
        pol, cost = a["membership_policy"], a["default_time_cost_bps"]
        need(is_int(cost) and 0 <= cost <= 10000, f"archetype {aid}: default_time_cost_bps hors 0..10000")
        if a["class"] in VIEW_CLASSES:
            need(pol == "derived_from_provider" and cost == 0, f"archetype {aid}: une vue derive du fournisseur sans cout")
        elif a["class"] == "activity_unit":
            need(pol == "time_budgeted" and cost == 10000, f"archetype {aid}: unite d'activite a plein temps attendue")
        else:
            need(pol in ("overlapping", "time_budgeted"), f"archetype {aid}: politique inconnue {pol}")
            need((pol == "overlapping") == (cost == 0), f"archetype {aid}: cout de temps incoherent avec {pol}")
        if pol == "time_budgeted":
            need(a["preemptible"] is True or a["class"] == "activity_unit", f"archetype {aid}: un temps partiel doit etre preemptible")
        acts = a["activity_ids"]
        if a["class"] == "activity_unit":
            need(bool(acts) and all(x in activities for x in acts), f"archetype {aid}: activity_ids invalides {acts}")
        else:
            need(acts == [], f"archetype {aid}: activity_ids reserve aux unites d'activite")
        np = a.get("name_profile")
        need(isinstance(np, str) and np.startswith("organization."), f"archetype {aid}: name_profile doit etre un kind Namebench")
        need(np in kinds, f"archetype {aid}: kind absent du catalogue de kinds ({np})")
    for k, e in kinds.items():
        need(e["origin"] in ("namebench_starting", "proposed_to_namebench"), f"kind {k}: origine inconnue")
        actual = sorted(a["id"] for a in defs if a.get("name_profile") == k)
        need(e["archetype_ids"] == actual, f"kind {k}: liste d'archetypes differente de l'usage reel")
        need(bool(actual), f"kind {k}: kind liste mais inutilise")
    end_group("format des archetypes et kinds de noms")

    # ------------------------------------------------ 5. alignement
    ea = cat["engine_defaults"].get("alignment")
    tags = [t["id"] for t in cat.get("conduct_tag_definitions", [])]
    tagset = set(tags)
    need(len(tags) == len(tagset) and len(tags) > 0, "conduct_tag_definitions: catalogue vide ou identifiants en double")
    if need(ea is not None, "engine_defaults.alignment manquant"):
        need(ea["axis_bounds_bps"] == {"min": -10000, "max": 10000}, "alignment: bornes des axes differentes de la spec")
        need(ea["conduct_class_thresholds"].get("lawless") == SPEC_THRESHOLDS["lawless"]
             and ea["conduct_class_thresholds"].get("crooked") == SPEC_THRESHOLDS["crooked"]
             and ea["conduct_class_thresholds"].get("ordinary") == SPEC_THRESHOLDS["ordinary"]
             and ea["conduct_class_thresholds"].get("upright") == SPEC_THRESHOLDS["upright"],
             "alignment: seuils de classe differents du README 27.2")
        lst = ea["legal_status_thresholds"]
        need(all(lst.get(k) == v for k, v in SPEC_STATUS.items()), "alignment: seuils de statut differents du README 27.4")
        need(is_int(ea["person_axis_draws"]["draws_per_axis"]) and ea["person_axis_draws"]["draws_per_axis"] == 3, "alignment: trois tirages par axe attendus")
        need(is_int(ea["person_axis_draws"]["draw_bound_bps"]) and 0 < ea["person_axis_draws"]["draw_bound_bps"] <= 10000, "alignment: borne de tirage hors bornes")
        vb = ea["conformity"]["value_weight_bounds"]
        need(vb == {"min": -1000, "max": 1000}, "alignment: bornes des poids de valeur differentes de la spec")
        vis = ea["visibility_values"]
        need(set(vis) == {"open", "covert"}, "alignment: visibilites attendues open/covert")
    else:
        vis = ["open", "covert"]
        vb = {"min": -1000, "max": 1000}
    aa = cat.get("archetype_alignment", {})
    for aid in sorted(idset - set(aa)):
        errors.append(f"archetype sans alignement : {aid}")
    for aid in sorted(set(aa) - idset):
        errors.append(f"alignement pour un archetype inconnu : {aid}")
    for aid, v in aa.items():
        need(set(v) == ALIGN_FIELDS, f"alignement {aid}: champs {sorted(set(v) ^ ALIGN_FIELDS)} en trop ou manquants")
        for k in ("morality_bps", "lawfulness_bps"):
            need(is_int(v.get(k)) and -10000 <= v[k] <= 10000, f"alignement {aid}: {k} hors -10000..10000")
        for k in ("morality_spread_bps", "lawfulness_spread_bps"):
            need(is_int(v.get(k)) and 0 <= v[k] <= 10000, f"alignement {aid}: {k} hors 0..10000")
        need(isinstance(v.get("conduct_tags"), list) and len(set(v["conduct_tags"])) == len(v["conduct_tags"]), f"alignement {aid}: tags en double")
        for t in v.get("conduct_tags", []):
            need(t in tagset, f"alignement {aid}: tag hors catalogue fermé {t}")
        need(v.get("default_visibility") in vis, f"alignement {aid}: visibilite invalide {v.get('default_visibility')}")
    vp = cat.get("family_value_profiles", {})
    for fid in sorted(set(fam) - set(vp)):
        errors.append(f"famille sans value_profile : {fid}")
    for fid in sorted(set(vp) - set(fam)):
        errors.append(f"value_profile pour une famille inconnue : {fid}")
    for fid, p in vp.items():
        w = p.get("weights", {})
        for t, x in w.items():
            need(t in tagset, f"value_profile {fid}: tag hors catalogue fermé {t}")
            need(is_int(x) and vb["min"] <= x <= vb["max"], f"value_profile {fid}: poids {t}={x} hors bornes")
        need(set(w) == tagset, f"value_profile {fid}: tags manquants {sorted(tagset - set(w))}")
    end_group("alignement (axes, tags, visibilite, 21 value_profile)")

    # ------------------------------------------------ 6. attentes d'alignement du cahier des charges
    exp = cov["alignment_expectations"]
    oe = exp["ordinary_economic"]
    for aid in oe["archetype_ids"]:
        v = aa.get(aid)
        if not need(v is not None, f"economie ordinaire {aid}: alignement manquant"):
            continue
        need(abs(v["morality_bps"]) <= oe["max_abs_base_morality_bps"], f"economie ordinaire {aid}: base de moralite trop eloignee de zero")
        need(v["morality_spread_bps"] >= oe["min_morality_spread_bps"], f"economie ordinaire {aid}: ecart trop etroit")
        need(v["morality_bps"] - v["morality_spread_bps"] <= -2500, f"economie ordinaire {aid}: 'crooked' inatteignable")
        need(v["morality_bps"] + v["morality_spread_bps"] >= 2500, f"economie ordinaire {aid}: 'upright' inatteignable")
        need(conduct_class(v["morality_bps"], v["lawfulness_bps"]) == "ordinary", f"economie ordinaire {aid}: la base doit etre 'ordinary'")
    for aid, allowed in exp["base_class"].items():
        v = aa.get(aid)
        if v is not None:
            c = conduct_class(v["morality_bps"], v["lawfulness_bps"])
            need(c in allowed, f"archetype {aid}: classe de base {c}, attendu parmi {allowed}")
    # aucune base deduite d'une espece ou d'une ascendance (README 6.2, 27.1)
    for k in ("nonhuman_enclave",):
        v = aa.get(k)
        if v is not None:
            need(abs(v["morality_bps"]) <= 500 and abs(v["lawfulness_bps"]) <= 500, f"{k}: base d'alignement non neutre")
    v = aa.get("monstrous_horde")
    if v is not None:
        need(any(t in v["conduct_tags"] for t in ("raiding", "violence")), "monstrous_horde: doit etre definie par une conduite")
        need(v["morality_bps"] + v["morality_spread_bps"] >= -2500 + 0, "monstrous_horde: les hordes ordinaires doivent rester atteignables")
    for aid, v in aa.items():
        need(not any("ancestry" in k or "species" in k for k in v), f"alignement {aid}: champ d'ascendance interdit")
    for fid, p in vp.items():
        need(not any("ancestry" in k or "species" in k for k in p), f"value_profile {fid}: champ d'ascendance interdit")
    end_group("attentes du cahier des charges (economie ordinaire, crime, ordres hospitaliers, neutralite d'ascendance)")

    # ------------------------------------------------ 7. reglage de fantastique
    sp = cat["engine_defaults"].get("setting_profile", {})
    need(sp.get("values") == SETTINGS and sp.get("rank") == RANK, "engine_defaults.setting_profile: ordre des reglages incorrect")
    need("setting_profile" in cat["parameter_definitions"], "parameter_definitions.setting_profile manquant")
    in_overlay = {}
    for o in cat["overlay_definitions"]:
        for a in o["archetype_ids"]:
            in_overlay.setdefault(a, []).append(o["id"])
    for world in SETTINGS:
        active = {a["id"] for a in defs if RANK.get(a.get("setting"), 99) <= RANK[world]}
        via_overlay = {a for o in ovl.values() for a in o["archetype_ids"] if a in active}
        if world == "historical":
            need(all(by_id[a].get("setting") == "historical" for a in active | via_overlay), "monde historique: archetype fantastique actif")
            need(all(by_id.get(a, {}).get("setting") == "historical" for a in base), "monde historique: archetype de base fantastique ou absent")
    for a in defs:
        if a.get("setting") in SETTINGS and a["setting"] != "historical":
            need(a["id"] not in base, f"{a['id']}: un archetype fantastique ne peut pas etre de base")
            need(a["id"] in in_overlay, f"{a['id']}: archetype fantastique sans overlay d'activation")
            for f in cat["families"]:
                need(a["id"] not in f["common_groups"], f"{a['id']}: archetype fantastique dans common_groups de {f['id']}")
    end_group("reglage de fantastique (setting_profile, overlays)")

    # ------------------------------------------------ 8. activites positives (README 25.1)
    def compatible(act, pool):
        return any(by_id.get(x, {}).get("class") == "activity_unit" and act in by_id[x].get("activity_ids", [])
                   and by_id[x].get("setting") == "historical" for x in pool)

    for act in activities:
        need(compatible(act, base), f"activite {act}: aucun type de base compatible")
        for f in cat["families"]:
            pool = set(base) | set(f["common_groups"])
            need(compatible(act, pool), f"activite {act}: aucun type compatible pour la famille {f['id']}")
    if PLAN_PATH.exists():
        plan = load(PLAN_PATH)

        def walk(o):
            if isinstance(o, dict):
                if "activity_weights" in o:
                    yield o["activity_weights"]
                for x in o.values():
                    yield from walk(x)
            elif isinstance(o, list):
                for x in o:
                    yield from walk(x)
        for weights in walk(plan):
            for act, wt in weights.items():
                if wt > 0:
                    need(act in activities, f"plan d'exemple: activite {act} inconnue")
                    need(compatible(act, base), f"plan d'exemple: activite positive {act} sans type compatible")
    end_group("activites positives et types compatibles")

    # ------------------------------------------------ 9. anciens identifiants et sections inchangees
    lb = cov["legacy_baseline"]
    legacy = lb["archetype_definitions"]
    need(len(legacy) == lb["counts"]["archetypes"] == 53, "baseline: 53 anciens archetypes attendus")
    need(ids[:len(legacy)] == [a["id"] for a in legacy], "anciens archetypes: ordre ou identifiants modifies")
    for old in legacy:
        cur = by_id.get(old["id"])
        if not need(cur is not None, f"ancien identifiant disparu : {old['id']}"):
            continue
        for k, v in old.items():
            need(cur.get(k) == v, f"ancien archetype {old['id']}: champ {k} modifie ({v!r} -> {cur.get(k)!r})")
    need(base[:len(lb["base_archetype_ids"])] == lb["base_archetype_ids"], "base_archetype_ids: anciens identifiants modifies")
    cur_ovl = {o["id"]: o for o in cat["overlay_definitions"]}
    for o in lb["overlay_definitions"]:
        need(cur_ovl.get(o["id"]) == o, f"ancien overlay modifie ou disparu : {o['id']}")
    for s in BASE_SECTIONS:
        need(sha(canon(cat[s])) == lb["sections_sha256"][s], f"section {s} modifiee (79 raccords, 21 familles et inventaire doivent rester intacts)")
    ed_old = {k: v for k, v in cat["engine_defaults"].items() if k not in S1_ENGINE_KEYS}
    need(sha(canon(ed_old)) == lb["engine_defaults_without_s1_keys_sha256"], "engine_defaults: valeurs anterieures modifiees")
    end_group("anciens identifiants et sections inchangees (53 archetypes, 79 raccords, 21 familles)")

    # ------------------------------------------------ rapport
    print("Verification du catalogue social v2 (S1)")
    print(f"  catalogue : {CAT_PATH.relative_to(ROOT)}")
    print(f"  grille    : {COV_PATH.relative_to(ROOT)} (Pecking Order {pin['commit'][:7]})")
    print()
    for name, n in groups:
        print(f"  [{'OK' if n == 0 else 'ECHEC'}] {name}" + (f" : {n} erreur(s)" if n else ""))
    by_cls, by_set = {}, {}
    for a in defs:
        by_cls[a["class"]] = by_cls.get(a["class"], 0) + 1
        by_set[a["setting"]] = by_set.get(a["setting"], 0) + 1
    print()
    print(f"  archetypes : {len(defs)}  classes {dict(sorted(by_cls.items()))}")
    print(f"  settings   : {dict((s, by_set.get(s, 0)) for s in SETTINGS)}")
    reach = {}
    for aid, v in aa.items():
        reach[conduct_class(v["morality_bps"], v["lawfulness_bps"])] = reach.get(conduct_class(v["morality_bps"], v["lawfulness_bps"]), 0) + 1
    order = ["lawless", "crooked", "ordinary", "upright", "saintly"]
    print(f"  classe de conduite de la base : {dict((k, reach.get(k, 0)) for k in order)}")
    if errors:
        print()
        print(f"ECHEC : {len(errors)} erreur(s)")
        for e in errors[:200]:
            print("  - " + e)
        return 1
    print()
    print("SUCCES : aucune erreur")
    return 0


if __name__ == "__main__":
    try:
        code = main()
    except (KeyError, TypeError, IndexError, AttributeError, ValueError) as exc:
        print(f"ECHEC : structure inattendue ({type(exc).__name__}: {exc})")
        for e in errors[:50]:
            print("  - " + e)
        code = 1
    sys.exit(code)
