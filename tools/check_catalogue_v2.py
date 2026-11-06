"""Verifie le catalogue social v2 (lot S1) et sa grille de couverture (bibliotheque standard seulement).

Usage : python tools/check_catalogue_v2.py [racine_du_depot]

L'outil ne lit que le depot de Chronos. La liste des systemes Pecking Order est celle qui est figee dans
archetype-coverage.v1.json (epinglee sur un commit) : le depot vivant de Pecking Order n'est jamais ouvert.
Il recalcule exactement (entiers) les distributions morales, les statuts de conformite, les densites par
taille de site et la correlation entre familles. Code de sortie 0 si tout passe, 1 sinon.
"""
import hashlib
import json
import re
import statistics
import sys
from pathlib import Path

DOCREL = Path("docs") / "design" / "chronos-social-v2"
SETTINGS = ["historical", "low_fantasy", "high_fantasy"]
RANK = {s: i for i, s in enumerate(SETTINGS)}
CLASS_SLOTS = {
    "activity_unit": {"expected_workforce"},
    "association": {"expected_active_stock"},
    "community": {"expected_active_stock"},
    "friend_circle": {"expected_active_stock"},
    "institution": {"site_occupation_anchor", "site_threshold_anchor"},
    "lineage_view": {"core_lineages"},
    "household_view": {"provider_households"},
}
LEGACY_CLASS_LIFE = {"activity_unit": 20, "association": 30, "community": 50, "friend_circle": 5,
                     "institution": None, "lineage_view": None, "household_view": None}
VIEW_CLASSES = {"lineage_view", "household_view"}
STOCK_CLASSES = {"association", "community", "friend_circle"}
CELL_CATEGORIES = {"subsistence_extraction", "craft_trade", "power", "justice_order", "military", "religion",
                   "knowledge_care", "culture_sociability", "crime_margins", "fantasy", "added"}
ARCH_FIELDS = ["id", "class", "slot_basis", "default_mean_lifetime_years", "membership_policy",
               "default_time_cost_bps", "activity_ids", "preemptible", "setting", "name_profile", "size",
               "recruitment_filters", "availability"]
AVAIL_KEYS = {"min_expected_adults", "site_tags_any", "weight", "max_per_site", "min_per_site",
              "requires_overlay_any", "birth_events"}
ALIGN_FIELDS = {"morality_bps", "lawfulness_bps", "morality_spread_bps", "lawfulness_spread_bps",
                "defining_tags", "risk_tags", "default_visibility", "jurisdiction_organ", "parameters_status"}
FILTER_KEYS = {"sex", "min_age", "max_age"}
S1_ENGINE_KEYS = {"alignment", "setting_profile", "availability_model"}
BASE_SECTIONS = ["source_inventory", "families", "bindings", "governance_templates", "kinship_templates",
                 "demography_society_templates", "institution_coverage", "required_activity_coverage", "resolution_order"]
CONDUCT_ORDER = ["lawless", "crooked", "ordinary", "upright", "saintly"]
# seuils du README 27.2 (personnes) et 27.4 ; saintly des organisations separe (decision du coordinateur)
SPEC_PERSON = {"lawless": {"morality_max_bps": -6000, "lawfulness_max_bps": -3000},
               "crooked": {"morality_max_bps": -2500},
               "ordinary": {"morality_below_bps": 2500},
               "upright": {"morality_below_bps": 7000}}
SPEC_ORG_UPRIGHT_BELOW = 5500
SPEC_STATUS = {"conformity_proscribed_at_or_below_bps": -5000, "conformity_tolerated_at_or_below_bps": -1500}

errors = []
groups = []
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


def is_text(o):
    """Bilingual text: {"en": ..., "fr": ...}, English being the source."""
    return isinstance(o, dict) and set(o) == {"en", "fr"}


def localized(o, lang="fr"):
    """Replaces every bilingual text by its `lang` string (the frozen baselines hold the French wording)."""
    if is_text(o):
        return o[lang]
    if isinstance(o, dict):
        return {k: localized(v, lang) for k, v in o.items()}
    if isinstance(o, list):
        return [localized(v, lang) for v in o]
    return o


def sha(s):
    return hashlib.sha256(s.encode("utf-8")).hexdigest()


def is_int(v):
    return isinstance(v, int) and not isinstance(v, bool)


def is_num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def load(path):
    raw = path.read_bytes()
    need(not raw.startswith(b"\xef\xbb\xbf"), f"{path.name}: BOM interdit")
    try:
        return json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as e:
        errors.append(f"{path.name}: illisible ({e})")
        return None


# ===================================================================== calculs exacts (importables)
_cache = {}


class Dist:
    """Somme de n tirages entiers uniformes dans [-b, b], decalee d'une base, bornee a [-10000, 10000]."""

    def __init__(self, base, b, n):
        key = (b, n)
        if key not in _cache:
            counts = [1]
            w = 2 * b + 1
            for _ in range(n if b > 0 else 0):
                pref = [0]
                for c in counts:
                    pref.append(pref[-1] + c)
                new_len = len(counts) + w - 1
                out = []
                for k in range(new_len):
                    lo = max(0, k - w + 1)
                    hi = min(len(counts) - 1, k)
                    out.append(pref[hi + 1] - pref[lo])
                counts = out
            cum, s = [], 0
            for c in counts:
                s += c
                cum.append(s)
            _cache[key] = (-b * n if b > 0 else 0, cum, s)
        off, cum, total = _cache[key]
        self.off, self.cum, self.total, self.base = off, cum, total, base

    def cdf(self, x):
        """P(axe <= x), bornage a +-10000 inclus (le bornage ne change pas les seuils utilises)."""
        k = x - self.off - self.base
        if k < 0:
            return 0.0
        if k >= len(self.cum):
            return 1.0
        return self.cum[k] / self.total


def org_probs(m, l, sm, sl, upright_below=SPEC_ORG_UPRIGHT_BELOW, draws=3):
    """Probabilites des cinq classes pour une organisation (tirage par slot), en fractions."""
    dm = Dist(m, sm // 2, draws)
    dl = Dist(l, sl // 2, draws)
    law = dm.cdf(-6000) * dl.cdf(-3000)
    crooked = dm.cdf(-2500) - law
    ordinary = dm.cdf(2499) - dm.cdf(-2500)
    upright = dm.cdf(upright_below - 1) - dm.cdf(2499)
    saintly = 1.0 - dm.cdf(upright_below - 1)
    return {"lawless": law, "crooked": crooked, "ordinary": ordinary, "upright": upright, "saintly": saintly}


def person_probs(m, l, bound=1500, draws=6):
    dm = Dist(m, bound, draws)
    dl = Dist(l, bound, draws)
    law = dm.cdf(-6000) * dl.cdf(-3000)
    return {"lawless": law, "crooked": dm.cdf(-2500) - law, "ordinary": dm.cdf(2499) - dm.cdf(-2500),
            "upright": dm.cdf(6999) - dm.cdf(2499), "saintly": 1.0 - dm.cdf(6999)}


def conduct_class(m, l, nature="person"):
    """Fonction pure du README 27.2 ; premiere regle verifiee. Pour une organisation, saintly des 5500."""
    up = 7000 if nature == "person" else SPEC_ORG_UPRIGHT_BELOW
    if m <= -6000 and l <= -3000:
        return "lawless"
    if m <= -2500:
        return "crooked"
    if m < 2500:
        return "ordinary"
    if m < up:
        return "upright"
    return "saintly"


def conformity(weights_of_defining, scale=10, cap=200, div=2):
    """conformity = 10 x (pire poids constitutif + min(200, floor(meilleure vertu / 2))) ; sans poids negatif,
    10 x meilleure vertu. Division euclidienne (plancher pour un diviseur positif), borne a +-10000."""
    if not weights_of_defining:
        return 0
    neg = [w for w in weights_of_defining if w < 0]
    best = max([w for w in weights_of_defining if w > 0], default=0)
    v = (min(neg) + min(cap, best // div)) if neg else best
    return max(-10000, min(10000, scale * v))


def status_of(c):
    if c <= SPEC_STATUS["conformity_proscribed_at_or_below_bps"]:
        return "proscribed"
    if c <= SPEC_STATUS["conformity_tolerated_at_or_below_bps"]:
        return "tolerated"
    return "licit"


def effective_weights(cat, fid):
    w = dict(cat["family_value_profiles"][fid]["weights"])
    cult = cat["family_value_profiles"][fid]["default_cult_profile_id"]
    w.update(cat["cult_value_profiles"][cult]["weights"])
    return w


def pearson(a, b):
    n = len(a)
    ma, mb = sum(a) / n, sum(b) / n
    cov = sum((x - ma) * (y - mb) for x, y in zip(a, b))
    va = sum((x - ma) ** 2 for x in a)
    vb = sum((y - mb) ** 2 for y in b)
    return cov / (va * vb) ** 0.5 if va and vb else 1.0


def active_overlays(cat, setting, local=()):
    act = set(local)
    for o in cat["overlay_definitions"]:
        mn = o.get("world_activation_min_setting")
        if mn is not None and RANK[setting] >= RANK[mn]:
            act.add(o["id"])
    return act


def expected_counts(cat, fid, adults, tags, setting, activity_mix, local_overlays=(), participation_bps=7500):
    """Effectifs attendus par archetype pour un site (README 23.1 amende). Retourne {id: nombre attendu}.
    Les vues (maisons, lignages) derivent du fournisseur et ne sont pas comptees."""
    ed = cat["engine_defaults"]
    fam = next(f for f in cat["families"] if f["id"] == fid)
    commons = set(fam["common_groups"])
    formal = fam["parameters"]["formal_association_weight"]
    overlays = active_overlays(cat, setting, local_overlays)
    mods = {}
    for o in cat["overlay_definitions"]:
        if o["id"] in overlays:
            for aid, m in o.get("archetype_modifiers", {}).items():
                mods[aid] = m
    tagset = set(tags)
    elig = []
    for a in cat["archetype_definitions"]:
        if a["class"] in VIEW_CLASSES:
            continue
        av = a["availability"]
        if adults < av["min_expected_adults"]:
            continue
        if av["site_tags_any"] and not (tagset & set(av["site_tags_any"])):
            continue
        if RANK[a["setting"]] > RANK[setting]:
            continue
        if av["requires_overlay_any"] and not (overlays & set(av["requires_overlay_any"])):
            continue
        elig.append(a)
    out = {a["id"]: 0.0 for a in cat["archetype_definitions"]}

    def wt(a):
        w = a["availability"]["weight"] * (2 if a["id"] in commons else 1)
        return w * mods.get(a["id"], {}).get("weight_multiplier", 1)

    def cap(a, x):
        mx = a["availability"]["max_per_site"]
        return min(x, mx) if mx is not None else x

    workforce = adults * participation_bps / 10000
    for act, pct in activity_mix.items():
        units = [a for a in elig if a["class"] == "activity_unit" and act in a["activity_ids"]]
        tot = sum(wt(a) for a in units)
        for a in units:
            size = (mods.get(a["id"], {}).get("size") or a["size"])["mean"]
            out[a["id"]] += cap(a, workforce * pct / 100 * wt(a) / tot / size)
    pools = {"association": ed["formal_associations_per_1000_adults"] * adults / 1000 * formal / 50,
             "community": max(1.0, ed["community_groups_per_1000_adults"] * adults / 1000),
             "friend_circle": ed["friend_circles_per_1000_adults"] * adults / 1000}
    for cls, pool in pools.items():
        shared = [a for a in elig if a["class"] == cls and "stock_per_1000_adults" not in a["availability"]]
        tot = sum(wt(a) for a in shared)
        for a in shared:
            out[a["id"]] += cap(a, pool * wt(a) / tot)
    for a in elig:
        av = a["availability"]
        if a["class"] in STOCK_CLASSES and "stock_per_1000_adults" in av:
            x = av["stock_per_1000_adults"] * adults / 1000 * mods.get(a["id"], {}).get("stock_multiplier", 1)
            if av["min_per_site"] and not av["birth_events"]:
                x = max(x, av["min_per_site"])
            out[a["id"]] += cap(a, x)
        elif a["class"] == "institution":
            if a["slot_basis"] == "site_threshold_anchor":
                out[a["id"]] += cap(a, 1.0)
    anchors = [a for a in elig if a["class"] == "institution" and a["slot_basis"] == "site_occupation_anchor"]
    tot = sum(wt(a) for a in anchors)
    for a in anchors:
        out[a["id"]] += wt(a) / tot
    return out


# ======================================================================= verification
def verify(root):
    doc = root / DOCREL
    cat = load(doc / "culture-bindings.v1.json")
    cov = load(doc / "archetype-coverage.v1.json")
    if cat is None or cov is None:
        print("ECHEC : fichiers JSON illisibles")
        print("\n".join(errors))
        return 1, {}
    end_group("lecture des fichiers")
    info = {}

    defs = cat["archetype_definitions"]
    ids = [a["id"] for a in defs]
    idset = set(ids)
    need(len(ids) == len(idset), "archetype_definitions: identifiants en double")
    by_id = {a["id"]: a for a in defs}
    activities = cat["required_activity_coverage"]

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
    md_path = doc / "archetype-coverage.md"
    md = md_path.read_text(encoding="utf-8") if md_path.exists() else ""
    need(bool(md), "archetype-coverage.md absent")
    n_cov = n_out = 0
    for c in cells:
        need(c.get("category") in CELL_CATEGORIES, f"case {c['id']}: categorie inconnue")
        need(c.get("origin") in ("brief", "added"), f"case {c['id']}: origine inconnue")
        if c.get("origin") == "added":
            need(bool(str(c.get("justification", "")).strip()), f"case ajoutee {c['id']}: justification manquante")
        if c.get("status") == "covered":
            n_cov += 1
            need(bool(c.get("archetype_ids")), f"case {c['id']}: aucun archetype pour une case couverte")
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
    info.update(n_cells=len(cells), n_cov=n_cov, n_out=n_out, n_map=n_map, n_exc=n_exc)
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
        for a in o.get("archetype_modifiers", {}):
            need(a in idset, f"overlay {o['id']}: archetype modifie absent {a}")
        if "world_activation_min_setting" in o:
            need(o["world_activation_min_setting"] in SETTINGS, f"overlay {o['id']}: reglage d'activation inconnu")
    base = cat["base_archetype_ids"]
    need(len(base) == 12 and len(set(base)) == 12, f"base_archetype_ids: {len(base)} identifiants, 12 distincts attendus")
    for a in base:
        need(a in idset, f"base_archetype_ids: archetype absent {a}")
    for a in ("base_criminal_network", "base_charitable_order"):
        need(a in base, f"base_archetype_ids: {a} manquant")
    end_group("references (archetypes, gouvernance, parente, overlays, raccords)")

    # ------------------------------------------------ 4. format, taille, filtres, disponibilite
    site_tags = {t["id"] for t in cat["site_tag_definitions"]}
    need(len(site_tags) == len(cat["site_tag_definitions"]) > 0, "site_tag_definitions: vide ou en double")
    events = {e["id"] for e in cat["birth_event_definitions"]}
    legacy_ids = {a["id"] for a in cov["legacy_baseline"]["archetype_definitions"]}
    kcat = cat["name_kind_catalogue"]
    base_kinds = {k["kind"]: k for k in kcat["kinds"]}
    sub_kinds = {k["kind"]: k for k in kcat["subkinds"]}
    for a in defs:
        aid = a["id"]
        for fld in ARCH_FIELDS:
            need(fld in a, f"archetype {aid}: champ {fld} manquant")
        if not all(f in a for f in ARCH_FIELDS):
            continue
        need(a["setting"] in SETTINGS, f"archetype {aid}: setting invalide ({a['setting']})")
        cls = a["class"]
        if not need(cls in CLASS_SLOTS, f"archetype {aid}: classe inconnue {cls}"):
            continue
        need(a["slot_basis"] in CLASS_SLOTS[cls], f"archetype {aid}: slot_basis {a['slot_basis']} incompatible avec {cls}")
        life = a["default_mean_lifetime_years"]
        pol, cost = a["membership_policy"], a["default_time_cost_bps"]
        if aid in legacy_ids:
            need(life == LEGACY_CLASS_LIFE[cls], f"ancien archetype {aid}: duree {life} differente de la classe")
        if a["slot_basis"] in ("expected_workforce", "expected_active_stock"):
            need(is_int(life) and 1 <= life <= 500, f"archetype {aid}: duree propre attendue entre 1 et 500 ans")
        else:
            need(life is None, f"archetype {aid}: pas de duree pour une ancre ou une vue")
        need(is_int(cost) and 0 <= cost <= 10000, f"archetype {aid}: default_time_cost_bps hors 0..10000")
        if cls in VIEW_CLASSES:
            need(pol == "derived_from_provider" and cost == 0, f"archetype {aid}: une vue derive du fournisseur sans cout")
        elif cls == "activity_unit":
            need(pol == "time_budgeted" and cost == 10000, f"archetype {aid}: unite d'activite a plein temps attendue")
        else:
            need(pol in ("overlapping", "time_budgeted"), f"archetype {aid}: politique inconnue {pol}")
            need((pol == "overlapping") == (cost == 0), f"archetype {aid}: cout de temps incoherent avec {pol}")
        need(isinstance(a["preemptible"], bool), f"archetype {aid}: preemptible booleen attendu")
        acts = a["activity_ids"]
        if cls == "activity_unit":
            need(bool(acts) and all(x in activities for x in acts), f"archetype {aid}: activity_ids invalides {acts}")
        else:
            need(acts == [], f"archetype {aid}: activity_ids reserve aux unites d'activite")
        np = a["name_profile"]
        need(isinstance(np, str) and (np in base_kinds or np in sub_kinds), f"archetype {aid}: kind hors catalogue ({np})")
        # taille
        sz = a["size"]
        if cls in VIEW_CLASSES:
            need(sz is None, f"archetype {aid}: une vue n'a pas de taille propre")
        elif need(isinstance(sz, dict) and set(sz) == {"min", "mean", "max"}, f"archetype {aid}: size {{min, mean, max}} attendu"):
            need(all(is_int(sz[k]) and sz[k] >= 1 for k in sz) and sz["min"] <= sz["mean"] <= sz["max"],
                 f"archetype {aid}: taille incoherente {sz}")
        # filtres
        fl = a["recruitment_filters"]
        if need(isinstance(fl, dict) and set(fl) <= FILTER_KEYS, f"archetype {aid}: filtres inconnus {sorted(set(fl) - FILTER_KEYS) if isinstance(fl, dict) else fl}"):
            need(fl.get("sex") in (None, "female", "male"), f"archetype {aid}: filtre de sexe invalide")
            for k in ("min_age", "max_age"):
                need(k not in fl or (is_int(fl[k]) and 0 <= fl[k] <= 120), f"archetype {aid}: {k} hors 0..120")
            need("min_age" not in fl or "max_age" not in fl or fl["min_age"] < fl["max_age"], f"archetype {aid}: min_age >= max_age")
        # disponibilite
        av = a["availability"]
        if not need(isinstance(av, dict) and AVAIL_KEYS <= set(av) <= AVAIL_KEYS | {"stock_per_1000_adults"},
                    f"archetype {aid}: availability a des cles incorrectes"):
            continue
        need(is_int(av["min_expected_adults"]) and av["min_expected_adults"] >= 0, f"archetype {aid}: min_expected_adults invalide")
        need(isinstance(av["site_tags_any"], list) and set(av["site_tags_any"]) <= site_tags,
             f"archetype {aid}: etiquette de site inconnue {sorted(set(av['site_tags_any']) - site_tags)}")
        need(is_int(av["weight"]) and av["weight"] >= 1, f"archetype {aid}: weight >= 1 attendu")
        need(av["max_per_site"] is None or (is_int(av["max_per_site"]) and av["max_per_site"] >= 1), f"archetype {aid}: max_per_site invalide")
        need(is_int(av["min_per_site"]) and av["min_per_site"] >= 0 and
             (av["max_per_site"] is None or av["min_per_site"] <= av["max_per_site"]), f"archetype {aid}: min_per_site invalide")
        need(set(av["requires_overlay_any"]) <= set(ovl), f"archetype {aid}: overlay requis absent")
        need(set(av["birth_events"]) <= events, f"archetype {aid}: evenement de naissance inconnu {sorted(set(av['birth_events']) - events)}")
        if "stock_per_1000_adults" in av:
            s = av["stock_per_1000_adults"]
            need(cls in STOCK_CLASSES, f"archetype {aid}: un stock propre exige une classe a stock")
            need(is_num(s) and s >= 0 and abs(s * 10 - round(s * 10)) < 1e-9, f"archetype {aid}: stock a une decimale attendu ({s})")
            need((s == 0) == bool(av["birth_events"]), f"archetype {aid}: stock nul si et seulement si ne d'un evenement")
        else:
            need(not av["birth_events"], f"archetype {aid}: archetype ne d'un evenement sans stock propre nul")
        if a["slot_basis"] == "site_occupation_anchor":
            need(av["max_per_site"] == 1, f"archetype {aid}: ancre municipale unique (max_per_site = 1)")
        if a["slot_basis"] == "site_threshold_anchor":
            need(av["max_per_site"] == 1 and av["weight"] == 1 and av["min_expected_adults"] > 0,
                 f"archetype {aid}: ancre a seuil (max_per_site = 1, weight = 1, seuil > 0)")
        # activation par overlay : relation bidirectionnelle
        if a["setting"] != "historical":
            need(bool(av["requires_overlay_any"]), f"{aid}: archetype fantastique sans overlay d'activation")
        for oid in av["requires_overlay_any"]:
            if oid in ovl:
                need(aid in ovl[oid]["archetype_ids"], f"{aid}: overlay requis {oid} ne le liste pas")
    for o in cat["overlay_definitions"]:
        for aid in o["archetype_ids"]:
            if aid in by_id and "availability" in by_id[aid]:
                need(o["id"] in by_id[aid]["availability"]["requires_overlay_any"], f"overlay {o['id']} liste {aid} qui ne le requiert pas")
    # kinds
    used = {}
    for a in defs:
        used.setdefault(a.get("name_profile"), []).append(a["id"])
    for k, e in base_kinds.items():
        need(e["origin"] in ("namebench_starting", "proposed_to_namebench"), f"kind {k}: origine inconnue")
        need(e["archetype_ids"] == sorted(used.get(k, [])), f"kind {k}: liste d'archetypes differente de l'usage reel")
    for k, e in sub_kinds.items():
        need(e["parent"] in base_kinds and k.startswith(e["parent"] + "."), f"sous-kind {k}: parent invalide")
        need(e["archetype_ids"] == sorted(used.get(k, [])), f"sous-kind {k}: liste d'archetypes differente de l'usage reel")
        need(bool(used.get(k)), f"sous-kind {k}: inutilise")
    for k, e in base_kinds.items():
        need(bool(used.get(k)) or any(s["parent"] == k and used.get(sk) for sk, s in sub_kinds.items()), f"kind {k}: inutilise")
    need(len(base_kinds) == 27, f"name_kind_catalogue: {len(base_kinds)} kinds de base, 27 attendus")
    info["kinds_distinct"] = sorted(set(used))
    end_group("format des archetypes (classes, durees, tailles, filtres, disponibilite, kinds)")

    # ------------------------------------------------ 5. alignement : structure et bornes
    ea = cat["engine_defaults"].get("alignment")
    tag_defs = {t["id"]: t for t in cat.get("conduct_tag_definitions", [])}
    tagset = set(tag_defs)
    need(len(tag_defs) == len(cat.get("conduct_tag_definitions", [])) > 0, "conduct_tag_definitions: catalogue vide ou identifiants en double")
    for t, d in tag_defs.items():
        need(d.get("role") in ("conduct", "risk"), f"tag {t}: role invalide")
        need(is_int(d.get("weight_min")) and is_int(d.get("weight_max")) and -1000 <= d["weight_min"] <= d["weight_max"] <= 1000,
             f"tag {t}: bornes de poids invalides")
    need({t for t, d in tag_defs.items() if d["role"] == "risk"} == {"corruption", "exploitation"}, "tags de risque: corruption et exploitation exactement")
    vis = ["open", "covert"]
    if need(ea is not None, "engine_defaults.alignment manquant"):
        need(ea["axis_bounds_bps"] == {"min": -10000, "max": 10000}, "alignment: bornes des axes differentes de la spec")
        th = ea["conduct_class_thresholds"]
        pt, ot = th.get("person", {}), th.get("organization", {})
        for k, v in SPEC_PERSON.items():
            need(pt.get(k) == v, f"alignment: seuil personne {k} different du README 27.2")
            need(ot.get(k) == v or (k == "upright" and ot.get(k) == {"morality_below_bps": SPEC_ORG_UPRIGHT_BELOW}),
                 f"alignment: seuil organisation {k} incorrect")
        need(ot.get("upright") == {"morality_below_bps": SPEC_ORG_UPRIGHT_BELOW}, "alignment: saintly des organisations a 5500")
        lst = ea["legal_status_thresholds"]
        need(all(lst.get(k) == v for k, v in SPEC_STATUS.items()), "alignment: seuils de statut differents du README 27.4")
        pd = ea["person_axis_draws"]
        need(pd["family_latent"] == {"draws": 3, "bound_bps": 1500} and pd["individual"] == {"draws": 3, "bound_bps": 1500},
             "alignment: personnes = latent familial 3 x 1500 + ecart individuel 3 x 1500")
        need(ea["organization_axis_draws"]["draws_per_axis"] == 3 and ea["organization_axis_draws"]["per_draw_bound"] == "floor(spread_bps / 2)",
             "alignment: organisations = 3 tirages de floor(ecart/2)")
        need(ea["alignment_fit"] == {"floor_per_mille": 100, "divisor": 20, "form": "multiplicative"}
             or (ea["alignment_fit"].get("floor_per_mille") == 100 and ea["alignment_fit"].get("divisor") == 20
                 and ea["alignment_fit"].get("form") == "multiplicative"), "alignment_fit: facteur multiplicatif, plancher 100, diviseur 20")
        cf = ea["conformity"]
        need(cf["scale"] == 10 and cf["attenuation_cap"] == 200 and cf["attenuation_divisor"] == 2, "conformite: 10 x (pire + min(200, meilleure / 2))")
        need(cf["value_weight_bounds"] == {"min": -1000, "max": 1000}, "alignment: bornes des poids de valeur differentes de la spec")
        vis = ea["visibility_values"]
        need(set(vis) == {"open", "covert"}, "alignment: visibilites attendues open/covert")
    aa = cat.get("archetype_alignment", {})
    for aid in sorted(idset - set(aa)):
        errors.append(f"archetype sans alignement : {aid}")
    for aid in sorted(set(aa) - idset):
        errors.append(f"alignement pour un archetype inconnu : {aid}")
    organs = set()
    for aid, v in aa.items():
        need(set(v) == ALIGN_FIELDS, f"alignement {aid}: champs {sorted(set(v) ^ ALIGN_FIELDS)} en trop ou manquants")
        for k in ("morality_bps", "lawfulness_bps"):
            need(is_int(v.get(k)) and -10000 <= v[k] <= 10000, f"alignement {aid}: {k} hors -10000..10000")
        for k in ("morality_spread_bps", "lawfulness_spread_bps"):
            need(is_int(v.get(k)) and 0 <= v[k] <= 10000, f"alignement {aid}: {k} hors 0..10000")
        for key, role in (("defining_tags", "conduct"), ("risk_tags", "risk")):
            lst = v.get(key, [])
            need(isinstance(lst, list) and len(set(lst)) == len(lst), f"alignement {aid}: {key} en double")
            for t in lst:
                need(t in tagset, f"alignement {aid}: tag hors catalogue fermé {t}")
                need(tag_defs.get(t, {}).get("role") == role, f"alignement {aid}: tag {t} range dans {key} contre son role")
        need(v.get("default_visibility") in vis, f"alignement {aid}: visibilite invalide {v.get('default_visibility')}")
        need(isinstance(v.get("jurisdiction_organ"), bool), f"alignement {aid}: jurisdiction_organ booleen attendu")
        if v.get("jurisdiction_organ"):
            organs.add(aid)
        if aid in by_id and by_id[aid]["class"] in VIEW_CLASSES:
            need(v["morality_spread_bps"] == 0 and v["lawfulness_spread_bps"] == 0, f"alignement {aid}: une vue est une lecture sans tirage (ecarts nuls)")
        if aid in by_id and by_id[aid]["class"] == "institution":
            need(v.get("jurisdiction_organ") is True, f"alignement {aid}: une institution est un organe de la juridiction")
    # profils de valeurs
    vp = cat.get("family_value_profiles", {})
    cults = cat.get("cult_value_profiles", {})
    need(len(cults) >= 4, "cult_value_profiles: au moins quatre profils de culte")
    for fid in sorted(set(fam) - set(vp)):
        errors.append(f"famille sans value_profile : {fid}")
    for fid in sorted(set(vp) - set(fam)):
        errors.append(f"value_profile pour une famille inconnue : {fid}")
    for fid, p in vp.items():
        w = p.get("weights", {})
        for t, x in w.items():
            need(t in tagset, f"value_profile {fid}: tag hors catalogue fermé {t}")
            if t in tag_defs:
                need(is_int(x) and tag_defs[t]["weight_min"] <= x <= tag_defs[t]["weight_max"], f"value_profile {fid}: poids {t}={x} hors bornes du tag")
        need(set(w) == tagset, f"value_profile {fid}: tags manquants {sorted(tagset - set(w))}")
        need(p.get("default_cult_profile_id") in cults, f"value_profile {fid}: profil de culte par defaut absent")
        need(not any("ancestry" in k or "species" in k for k in p), f"value_profile {fid}: champ d'ascendance interdit")
    for cid, c in cults.items():
        for t, x in c.get("weights", {}).items():
            need(t in tagset, f"culte {cid}: tag hors catalogue fermé {t}")
            if t in tag_defs:
                need(is_int(x) and tag_defs[t]["weight_min"] <= x <= tag_defs[t]["weight_max"], f"culte {cid}: poids {t}={x} hors bornes du tag")
        need("heresy" in c.get("weights", {}), f"culte {cid}: heresy sans referent")
    for fid, p in vp.items():
        if fid in fam and set(p.get("weights", {})) == tagset:
            w = p["weights"]
            P = ["charity", "hospitality", "healing", "solidarity", "fair_dealing"]
            Q = ["violence", "extortion", "theft", "fraud", "slavery", "exploitation", "raiding"]
            Oo = ["oathkeeping", "justice", "craft_standards", "fair_dealing"]
            R = ["rebellion", "smuggling", "secrecy", "corruption"]
            m = (sum(w[t] for t in P) // len(P) + sum(w[t] for t in Q) // len(Q)) // 2
            l = (sum(w[t] for t in Oo) // len(Oo) + sum(w[t] for t in R) // len(R)) // 2
            m, l = max(-300, min(300, m)), max(-500, min(500, l))
            ab = p.get("alignment_base", {})
            need(ab == {"morality_bps": m, "lawfulness_bps": l}, f"value_profile {fid}: alignment_base {ab} different de la derivation {(m, l)}")
    end_group("alignement : structure, tags (risque/constitutif), bornes, 21 value_profile, profils de culte")

    # ------------------------------------------------ 6. echelle morale : convolutions exactes et cibles
    tg = cov["moral_targets"]
    pct = lambda x: 100 * x
    ordb = tg["ordinary_economic"]["bands_percent"]
    ord_rows = {}
    for aid in tg["ordinary_economic"]["archetype_ids"]:
        v = aa.get(aid)
        if not need(v is not None, f"economie ordinaire {aid}: alignement manquant"):
            continue
        pr = org_probs(v["morality_bps"], v["lawfulness_bps"], v["morality_spread_bps"], v["lawfulness_spread_bps"])
        ord_rows[aid] = pr
        for cls, (lo, hi) in ordb.items():
            need(lo <= pct(pr[cls]) <= hi, f"economie ordinaire {aid}: {cls} {pct(pr[cls]):.2f} % hors de [{lo}, {hi}]")
        need(pct(pr["lawless"]) < tg["ordinary_economic"]["max_lawless_percent"], f"economie ordinaire {aid}: mafia >= 1 %")
    info["ordinary_rows"] = ord_rows
    org_rows = {}
    for aid, v in aa.items():
        org_rows[aid] = org_probs(v["morality_bps"], v["lawfulness_bps"], v["morality_spread_bps"], v["lawfulness_spread_bps"])
    info["org_rows"] = org_rows
    for aid, spec in tg["organization_minimums"].items():
        pr = org_rows.get(aid)
        if pr is None:
            errors.append(f"cible morale: archetype absent {aid}")
            continue
        for cls, lo in spec.items():
            need(pct(pr[cls]) > lo, f"cible morale {aid}: {cls} {pct(pr[cls]):.2f} % <= {lo} %")
    for aid, allowed in tg["base_class"].items():
        v = aa.get(aid)
        if v is not None:
            c = conduct_class(v["morality_bps"], v["lawfulness_bps"], "organization")
            need(c in allowed, f"archetype {aid}: classe de base {c}, attendu parmi {allowed}")
    for aid in tg["neutral_bases"]:
        v = aa.get(aid)
        if v is not None:
            need(abs(v["morality_bps"]) <= 500 and abs(v["lawfulness_bps"]) <= 500, f"{aid}: base d'alignement non neutre")
    persons = {}
    pb = tg["persons"]
    for fid, p in vp.items():
        ab = p.get("alignment_base")
        if not ab:
            continue
        pr = person_probs(ab["morality_bps"], ab["lawfulness_bps"])
        persons[fid] = pr
        for cls in ("lawless", "saintly"):
            need(pb["extreme_per_person_min"] <= pr[cls] <= pb["extreme_per_person_max"],
                 f"personnes {fid}: {cls} 1/{(1 / pr[cls]) if pr[cls] else float('inf'):.0f} hors de la cible")
    info["person_rows"] = persons
    end_group("echelle morale (convolutions exactes : economie ordinaire, criminels, ordres, personnes)")

    # ------------------------------------------------ 7. conformite
    fams = list(fam)
    eff = {f: effective_weights(cat, f) for f in fams if f in vp and "default_cult_profile_id" in vp[f]}
    cases = [([-900, 400], -7000, "proscribed"), ([-300, 300], -1500, "tolerated"), ([], 0, "licit"), ([700], 7000, "licit"),
             ([-100, -300, 900], -1000, "licit"), ([-1000, 1000], -8000, "proscribed")]
    for ws, expected, st in cases:
        c = conformity(ws)
        need(c == expected and status_of(c) == st, f"conformite {ws}: {c} ({status_of(c)}), attendu {expected} ({st})")
    need(conformity([-7]) == -70 and conformity([-3, 5]) == -10, "conformite: division euclidienne de la vertu")
    cm = {}
    for aid, v in aa.items():
        if v["jurisdiction_organ"]:
            continue
        cm[aid] = {f: status_of(conformity([eff[f][t] for t in v["defining_tags"]])) for f in eff}
    info["conformity"] = cm

    def count(aid, st):
        return sum(1 for s in cm[aid].values() if s == st)
    nf = len(eff)
    need(count("mining_crew", "licit") == nf, "conformite: les mines doivent etre licites partout")
    need(count("thieves_guild", "proscribed") == nf, "conformite: les voleurs doivent etre proscrits partout")
    need(count("assassin_brotherhood", "proscribed") == nf, "conformite: les assassins doivent etre proscrits partout")
    for aid in ("secret_society", "beggar_brotherhood"):
        need(nf - count(aid, "licit") >= tg["conformity"]["non_licit_min_families"], f"conformite: {aid} licite dans trop de familles")
        need(count(aid, "licit") < nf, f"conformite: {aid} ne doit pas etre licite partout")
    # heresie : non licite dans toute culture dont le culte la condamne
    cond = [f for f in eff if cat["cult_value_profiles"][vp[f]["default_cult_profile_id"]]["weights"]["heresy"] <= tg["conformity"]["heresy_condemned_at_or_below"]]
    need(len(cond) >= 4, "conformite: au moins quatre familles dont le culte condamne l'heresie")
    for f in cond:
        need(cm["heterodox_movement"][f] != "licit", f"conformite: l'heresie est licite dans {f}, dont le culte la condamne")
    need(any(cm["heterodox_movement"][f] == "licit" for f in eff), "conformite: l'heresie doit rester licite quelque part")
    for aid in ("slave_trading_ring", "raiding_band", "banking_house", "witch_coven"):
        sts = set(cm[aid].values())
        need(len(sts) >= 2, f"conformite: {aid} a le meme statut dans toutes les familles ({sorted(sts)})")
    need("licit" in set(cm["slave_trading_ring"].values()) and "proscribed" in set(cm["slave_trading_ring"].values()),
         "conformite: la traite doit etre licite dans une famille et proscrite dans une autre")
    need("licit" in set(cm["raiding_band"].values()), "conformite: le pillage doit etre licite dans une famille guerriere")
    need(count("banking_house", "licit") < nf, "conformite: l'usure doit gener la banque dans une famille")
    need(count("executioner_house", "proscribed") == 0 and count("executioner_house", "tolerated") >= tg["conformity"]["executioner_tolerated_min"],
         "conformite: le bourreau est stigmatise (tolere), jamais proscrit")
    need(count("witch_coven", "licit") < nf, "conformite: la sorciere est mal vue quelque part (README 27.4)")
    varying = sum(1 for aid, d in cm.items() if len(set(d.values())) > 1)
    need(varying >= tg["conformity"]["min_archetypes_with_varying_status"], f"conformite: {varying} archetypes seulement ont un statut variable")
    info["conformity_varying"] = (varying, len(cm))
    # correlation entre familles
    vecs = {f: [eff[f][t] for t in sorted(tagset)] for f in eff}
    cs = [pearson(vecs[a], vecs[b]) for i, a in enumerate(sorted(vecs)) for b in sorted(vecs)[i + 1:]]
    med = statistics.median(cs)
    info["correlation"] = (med, min(cs), max(cs), len(cs))
    need(med < tg["values"]["median_correlation_below"], f"valeurs: correlation mediane {med:.3f} >= {tg['values']['median_correlation_below']}")
    for t in tg["values"]["varying_tags"]:
        ws = {eff[f][t] for f in eff}
        need(len(ws) >= tg["values"]["min_distinct_weights"], f"valeurs: le tag {t} ne varie pas assez entre familles")
    end_group("conformite (cas nommes, statuts par famille, correlation entre familles)")

    # ------------------------------------------------ 8. reglage de fantastique
    sp = cat["engine_defaults"].get("setting_profile", {})
    need(sp.get("values") == SETTINGS and sp.get("rank") == RANK, "engine_defaults.setting_profile: ordre des reglages incorrect")
    need("setting_profile" in cat["parameter_definitions"], "parameter_definitions.setting_profile manquant")
    for world in SETTINGS:
        active = {a["id"] for a in defs if RANK.get(a.get("setting"), 99) <= RANK[world]}
        if world == "historical":
            need(all(by_id[a]["setting"] == "historical" for a in active), "monde historique: archetype fantastique actif")
            need(all(by_id.get(a, {}).get("setting") == "historical" for a in base), "monde historique: archetype de base fantastique ou absent")
    for a in defs:
        if a.get("setting") in SETTINGS and a["setting"] != "historical":
            need(a["id"] not in base, f"{a['id']}: un archetype fantastique ne peut pas etre de base")
            for f in cat["families"]:
                need(a["id"] not in f["common_groups"], f"{a['id']}: archetype fantastique dans common_groups de {f['id']}")
    # un monde historique n'attend aucun archetype fantastique, quelle que soit la famille ou l'overlay local
    scen = cov["density_expectations"]["scenarios"]
    all_overlays = list(ovl)
    big = scen[-1]
    for f in fams:
        cnt = expected_counts(cat, f, big["adults"], [t["id"] for t in cat["site_tag_definitions"]], "historical", big["activity_mix_percent"], all_overlays)
        bad = [a for a, x in cnt.items() if x > 0 and by_id[a]["setting"] != "historical"]
        need(not bad, f"monde historique, famille {f}: archetypes fantastiques attendus {bad}")
    # les overlays fantastiques s'activent par setting_profile, sans raccord
    for o in cat["overlay_definitions"]:
        if o["id"] in tg_overlay_ids(cat):
            need(o.get("world_activation_min_setting") in ("low_fantasy", "high_fantasy"), f"overlay {o['id']}: activation par setting_profile manquante")
    end_group("reglage de fantastique (setting_profile, overlays de monde)")

    # ------------------------------------------------ 9. activites positives (README 25.1)
    def compatible(act, pool):
        return any(by_id.get(x, {}).get("class") == "activity_unit" and act in by_id[x].get("activity_ids", [])
                   and by_id[x].get("setting") == "historical" for x in pool)

    for act in activities:
        need(compatible(act, base), f"activite {act}: aucun type de base compatible")
        for f in cat["families"]:
            need(compatible(act, set(base) | set(f["common_groups"])), f"activite {act}: aucun type compatible pour la famille {f['id']}")
    plan_path = doc / "village-population-plan.example.json"
    if plan_path.exists():
        plan = load(plan_path)

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

    # ------------------------------------------------ 10. eligibilite : un chemin de generation par archetype
    all_tags = [t["id"] for t in cat["site_tag_definitions"]]
    top = scen[-1]
    reach = {}
    for f in fams:
        cnt = expected_counts(cat, f, top["adults"] * 20, all_tags, "high_fantasy", top["activity_mix_percent"], all_overlays)
        for a, x in cnt.items():
            reach[a] = reach.get(a, 0) + x
    for a in defs:
        if a["class"] in VIEW_CLASSES:
            continue
        av = a["availability"]
        born = bool(av["birth_events"])
        need(reach.get(a["id"], 0) > 0 or born,
             f"archetype sans chemin de generation : {a['id']} (aucun contexte ne l'attend et aucun evenement ne le fait naitre)")
    info["reach"] = reach
    end_group("eligibilite (chaque archetype a un chemin de generation)")

    # ------------------------------------------------ 11. densites par taille de site
    de = cov["density_expectations"]
    dens = {}
    for sc in scen:
        need(abs(sc["adults"] - sc["population"] * cat["engine_defaults"]["expected_adult_share_bps"] / 10000) < 1e-6,
             f"densite {sc['id']}: adultes != population x part d'adultes")
        need(sum(sc["activity_mix_percent"].values()) == 100, f"densite {sc['id']}: la repartition d'activites ne somme pas a 100")
    for sc in scen:
        for setting in ("historical", "high_fantasy"):
            for f in fams:
                dens[(sc["id"], setting, f)] = expected_counts(cat, f, sc["adults"], sc["plausible_site_tags"], setting,
                                                              sc["activity_mix_percent"], all_overlays if setting == "high_fantasy" else ())
    info["density"] = dens
    ff = de["forbidden"]
    for sid in ff["scenarios"]:
        sc = next(s for s in scen if s["id"] == sid)
        for f in fams:
            # etiquettes maximales : aucune combinaison de contexte ne doit produire ces groupes
            cnt = expected_counts(cat, f, sc["adults"], all_tags, "high_fantasy", sc["activity_mix_percent"], all_overlays)
            for aid in ff["archetype_ids"]:
                need(cnt[aid] == 0, f"densite {sid}/{f}: {aid} attendu {cnt[aid]:.3f}, interdit sous {sc['adults']:.0f} adultes")
    for rq in de["required"]:
        sc = next(s for s in scen if s["id"] == rq["scenario"])
        for f in fams:
            for setting in ("historical", "high_fantasy"):
                cnt = dens[(sc["id"], setting, f)]
                tot = sum(cnt[a] for a in rq["archetype_ids"])
                need(tot >= rq["min_expected"], f"densite {rq['scenario']}/{f}/{setting}: {rq['label']} {tot:.2f} < {rq['min_expected']}")
                if "max_expected" in rq:
                    need(tot <= rq["max_expected"], f"densite {rq['scenario']}/{f}/{setting}: {rq['label']} {tot:.2f} > {rq['max_expected']}")
    for bd in de["upper_bounds"]:
        sc = next(s for s in scen if s["id"] == bd["scenario"])
        classes = set(bd["classes"])
        for f in fams:
            cnt = dens[(sc["id"], "historical", f)]
            tot = sum(x for a, x in cnt.items() if by_id[a]["class"] in classes and not (bd.get("exclude_slot_basis") and by_id[a]["slot_basis"] in bd["exclude_slot_basis"]))
            need(tot <= bd["max_expected"], f"densite {bd['scenario']}/{f}: {bd['label']} {tot:.2f} > {bd['max_expected']}")
            info.setdefault("upper", {})[(bd["scenario"], bd["label"], f)] = tot
    end_group("densites attendues (hameau, village, bourg, ville ; 21 familles ; monde historique et haute fantaisie)")

    # ------------------------------------------------ 12. anciens identifiants et sections inchangees
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
        need(localized(cur_ovl.get(o["id"])) == o, f"ancien overlay modifie ou disparu : {o['id']}")
    for s in BASE_SECTIONS:
        need(sha(canon(localized(cat[s]))) == lb["sections_sha256"][s], f"section {s} modifiee (79 raccords, 21 familles et inventaire doivent rester intacts)")
    ed_old = {k: v for k, v in cat["engine_defaults"].items() if k not in S1_ENGINE_KEYS}
    need(sha(canon(localized(ed_old))) == lb["engine_defaults_without_s1_keys_sha256"], "engine_defaults: valeurs anterieures modifiees")
    end_group("anciens identifiants et sections inchangees (53 archetypes, 79 raccords, 21 familles)")

    # ------------------------------------------------ 13. bilingual texts
    def check_texts(o, path):
        if isinstance(o, dict) and ("en" in o or "fr" in o) and set(o) <= {"en", "fr"}:
            need(is_text(o) and all(isinstance(v, str) and v.strip() for v in o.values()), f"texte bilingue incomplet : {path}")
        elif isinstance(o, dict):
            for k, v in o.items():
                check_texts(v, f"{path}/{k}")
        elif isinstance(o, list):
            for i, v in enumerate(o):
                check_texts(v, f"{path}/{i}")
    check_texts(cat, "")
    for tag in cat["conduct_tag_definitions"]:
        need(is_text(tag.get("label")) and is_text(tag.get("description")), f"conduct_tag {tag['id']}: label et description bilingues requis")
    end_group("textes bilingues {en, fr}")

    info.update(defs=defs, aa=aa, cat=cat, cov=cov, fams=fams)
    return (1 if errors else 0), info


def tg_overlay_ids(cat):
    return {o["id"] for o in cat["overlay_definitions"] if "world_activation_min_setting" in o}


def report(root, code, info):
    cat, cov = info.get("cat"), info.get("cov")
    print("Verification du catalogue social v2 (S1, passe 1)")
    print(f"  catalogue : {DOCREL / 'culture-bindings.v1.json'}")
    print(f"  grille    : {DOCREL / 'archetype-coverage.v1.json'} (Pecking Order {cov['source_pin']['commit'][:7] if cov else '?'})")
    print()
    for name, n in groups:
        print(f"  [{'OK' if n == 0 else 'ECHEC'}] {name}" + (f" : {n} erreur(s)" if n else ""))
    if info.get("defs"):
        defs = info["defs"]
        by_cls, by_set = {}, {}
        for a in defs:
            by_cls[a["class"]] = by_cls.get(a["class"], 0) + 1
            by_set[a["setting"]] = by_set.get(a["setting"], 0) + 1
        print()
        print(f"  archetypes : {len(defs)}  classes {dict(sorted(by_cls.items()))}")
        print(f"  settings   : {dict((s, by_set.get(s, 0)) for s in SETTINGS)}")
        if "correlation" in info:
            med, mn, mx, n = info["correlation"]
            print(f"  correlation entre familles ({n} paires) : mediane {med:.3f}, min {mn:.3f}, max {mx:.3f}")
        if "conformity_varying" in info:
            v, n = info["conformity_varying"]
            print(f"  statut legal variable selon la famille : {v} archetypes sur {n}")
    if errors:
        print()
        print(f"ECHEC : {len(errors)} erreur(s)")
        for e in errors[:200]:
            print("  - " + e)
        return 1
    print()
    print("SUCCES : aucune erreur")
    return 0


def main():
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent
    code, info = verify(root)
    return report(root, code, info)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (KeyError, TypeError, IndexError, AttributeError, ValueError, StopIteration) as exc:
        print(f"ECHEC : structure inattendue ({type(exc).__name__}: {exc})")
        for e in errors[:50]:
            print("  - " + e)
        sys.exit(1)
