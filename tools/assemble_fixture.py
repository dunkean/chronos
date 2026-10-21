"""Assemble fixtures/stack.json (Chronos bootstrap, schema_version 1) from the prepared exports.

Standard library only. Inputs are the files written by the other tools under output/; sibling
directories are only read to fingerprint the source code that produced them.

Usage: python tools/assemble_fixture.py [--check]
  --check  rebuild in memory and fail if fixtures/stack.json differs.
"""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "output"
RPG = ROOT.parent
FIXTURE = ROOT / "fixtures" / "stack.json"
SEED = 42


def sha_bytes(data):
    return hashlib.sha256(data).hexdigest()


def sha_file(path):
    return sha_bytes(Path(path).read_bytes())


def sha_tree(root, patterns=("**/*",)):
    root = Path(root)
    files = sorted({p for pattern in patterns for p in root.glob(pattern) if p.is_file() and "target" not in p.parts})
    return sha_bytes(json.dumps({p.relative_to(root).as_posix(): sha_file(p) for p in files}, sort_keys=True).encode())


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def fp(digest):
    return "sha256:" + digest


def build():
    orogen = load(OUT / "orogen-seed42" / "manifest.json")
    mapx = load(OUT / "map" / "map-export.json")
    names = load(OUT / "names" / "names.json")
    blue = load(OUT / "blueprints" / "blueprints.json")
    people = load(OUT / "people" / "people.json")
    snapshot = load(OUT / "pecking-order-snapshot" / "SNAPSHOT.json")

    fields_sha = sha_file(OUT / "orogen-seed42" / "climate" / "fields.f32")
    graph_sha = sha_file(OUT / "orogen-seed42" / "climate" / "graph.json")
    mu_root = RPG / "city_generator" / "rust" / "crates"
    mu_sha = sha_tree(mu_root, ("core/Cargo.toml", "core/src/**/*.rs", "random/Cargo.toml", "random/src/**/*.rs"))
    tools_sha = sha_tree(ROOT / "tools", ("**/*.rs", "**/*.py", "**/*.mjs", "**/Cargo.toml", "**/Cargo.lock"))
    region, params, m = mapx["region"], mapx["params"], mapx["map"]

    sites = []
    for s in sorted(mapx["sites"], key=lambda s: s["id"]):
        mu = s["magna_urbis"]
        sites.append({
            "id": s["id"], "x": s["x"], "y": s["y"], "region_id": s["region_id"], "culture_id": s["culture_id"],
            "kind": s["kind"], "population": s["population"], "resources": s["resources"],
            "lon_deg": s["lon_deg"], "lat_deg": s["lat_deg"], "elevation_m": s["elevation_m"],
            "temperature_c": s["temperature_c"], "precipitation_mm": s["precipitation_mm"],
            "magna_urbis": {k: mu[k] for k in ("tile", "key", "class", "archetype", "radius_m", "water_distance_m")},
        })
    site_ids = [s["id"] for s in sites]
    mapping = people["scenario"]["settlements"]
    if [p["name"] for p in mapping] != site_ids or [p["id"] for p in mapping] != list(range(len(site_ids))):
        raise SystemExit("people.json was simulated on a different site list; rerun tools/lineage/export_people.py")

    routes = [{k: r[k] for k in ("id", "from", "to", "distance_km", "travel_days", "path", "kind", "source", "tiles", "gateways")}
              for r in sorted(mapx["routes"], key=lambda r: r["id"])]
    pools = names["pools"]
    blueprints = [{k: b[k] for k in ("id", "label", "label_en", "kind", "office_title", "office_title_en", "succession",
                                     "source", "role_id", "max_holders", "system_id", "theme", "authority", "structure",
                                     "selection_fr", "selection_en", "office_powers", "roles")}
                  for b in blue["blueprints"]]
    persons = [{k: p[k] for k in ("id", "birth_year", "death_year", "father_id", "mother_id", "site_index", "sex",
                                  "external_id", "status", "activity", "founder", "birth_site_index", "residences", "selection")}
               for p in people["people"]]

    window = people["window"]
    tiles = region["tiles"]
    sources = [
        {"id": "neural-earth", "label": "Neural Earth · Orogen (contexte initial)", "status": "imported",
         "detail": (f"Monde Orogen seed {orogen['seed']}, profil « orogen » par défaut ({orogen['config']['detail']} sommets, "
                    f"{orogen['config']['plate_count']} plaques), exécuté sur CPU par native/orogen/pipeline.mjs (relief → érosion → climat), "
                    "sans inférence neuronale fine. Petite planète choisie explicitement : diamètre "
                    f"{params['planet_diameter_km']:.0f} km. Carte {m['width']}×{m['height']} équirectangulaire centrée "
                    f"lon {region['center_lon_deg']:.0f}°, lat {region['center_lat_deg']:.0f}° ; {m['graph_vertices_in_bounds']} sommets source "
                    "dans l'emprise (≈ 9 km d'espacement) : relief et climat grossiers interpolés, ni rivières, ni lacs, ni sols."),
         "fingerprint": fp(fields_sha)},
        {"id": "regional-raster", "label": "Rasters régionaux Magna Urbis", "status": "derived",
         "detail": (f"{len(tiles)} tuiles de {params['tile_m']/1000:.0f} km, {params['tile_cells']}² cellules (projection azimutale équidistante), "
                    "interpolation barycentrique sphérique des champs Orogen. Eau = altitude < 0 (mer) ; river_width = 0, "
                    "river_ids = -1, aucun lac : approximation, les poids rivière/pont de Magna Urbis sont sans effet."),
         "fingerprint": fp(sha_file(OUT / "map" / "map-export.json"))},
        {"id": "magna-urbis", "label": "Magna Urbis · établissements et routes", "status": "imported",
         "detail": (f"magna_urbis_core::settlements : Terrain::new(Raster), place (mode counts : 1 principal, {params['towns_per_tile']} bourgs, "
                    f"{params['villages_per_tile']} villages par tuile), roads (exits = 0). {len(sites)} sites, routes tracées par Magna Urbis ; "
                    "les tuiles se raccordent par des sites-passerelles sur leurs bords communs. "
                    f"{len(routes)} liaisons site à site retenues parmi {params['route_candidates']} (sous-graphe 1,15-spanner des distances réseau)."),
         "fingerprint": fp(mu_sha)},
        {"id": "site-resources", "label": "Potentiels de ressources", "status": "derived",
         "detail": ("Calculés par l'outil Chronos à partir du climat Orogen (BIO1, BIO12), de l'altitude, de l'activité tectonique "
                    "et des scores de site Magna Urbis. food/water : capacités annuelles en personnes-années ; timber, iron, stone, salt : "
                    "indices 0–100. Aucun gisement relevé."),
         "fingerprint": fp(tools_sha)},
        {"id": "namebench", "label": "Namebench · réservoirs de noms", "status": "imported",
         "detail": ("Base mondiale locale en lecture seule. french = style.french (FR), british = style.english_uk (GB) : prénoms par "
                    "rang de popularité actuel, noms de famille par rang, lieux GeoNames PPL de 1 000 à 30 000 habitants (échantillon "
                    "par clé SHA-256). fantasy = tableaux fantasy.medievals, générateurs dungeon_and_dragons.humans et "
                    "towns_and_cities.fantasy_towns (graine fixe). Fréquences contemporaines, pas d'authenticité historique."),
         "fingerprint": fp(names["database"]["sha256"])},
        {"id": "pecking-order", "label": "Pecking Order · modèles d'institutions", "status": "imported",
         "detail": (f"Moteur Rust pecking-order {blue['blueprints'][0]['content_version']} (instantané figé du dépôt en cours de réécriture), "
                    f"thème generic : {', '.join(b['system_id'] for b in blueprints)}. Titres, rôles et pouvoirs de la source ; "
                    "« succession » est une interprétation Chronos de l'autorité (hereditary / vote / appointment)."),
         "fingerprint": fp(blue["kb_digest"])},
        {"id": "lineage-lab", "label": "Lineage Lab · personnes et familles", "status": "imported",
         "detail": (f"Simulation complète en mémoire, préréglage medieval (sans ses événements datés 1060/1100), {people['scenario']['initial_population']} "
                    f"fondateurs, régulation bornée (plafond {people['scenario']['target_population']}), années {window[0]}–{window[1]}, "
                    f"établissements = les {len(sites)} sites Magna Urbis (indice = rang du site). {len(persons)} personnes exportées sur "
                    f"{people['totals']['simulated_people']} simulées : élites, {sum(p['selection'] == 'commoner-1398' for p in persons)} roturiers adultes en 1398 "
                    f"et tous leurs ancêtres. death_year null = vivant en {window[1]}, décès ultérieur inconnu. Noms attribués par Chronos."),
         "fingerprint": fp(people["scenario_sha256"])},
    ]

    stack = {
        "schema_version": 1,
        "seed": SEED,
        "map": {
            "width": m["width"], "height": m["height"], "elevation_m": m["elevation_m"], "bounds": m["bounds"],
            "radius_km": m["radius_km"], "source": m["source"], "projection": m["projection"],
            "temperature_c": m["temperature_c"], "precipitation_mm": m["precipitation_mm"],
            "center_lon_deg": region["center_lon_deg"], "center_lat_deg": region["center_lat_deg"],
            "planet_diameter_km": params["planet_diameter_km"], "source_vertices_in_bounds": m["graph_vertices_in_bounds"],
        },
        "sites": sites,
        "routes": routes,
        "names": {c: pools[c] for c in ("french", "british", "fantasy")},
        "blueprints": blueprints,
        "people": persons,
        "sources": sources,
        "people_window": window,
        "regions": [{"id": t["id"], "tile": t["tile"], "relief": t["relief"], "land_fraction": t["land_fraction"],
                     "origin_plane_m": t["origin_plane_m"], "size_m": t["size_m"]} for t in tiles],
        "gateways": region["gateways"],
        "resource_units": {
            "food": "annual food capacity, person-years", "water": "annual fresh-water capacity, person-years",
            "timber": "potential index 0-100", "iron": "potential index 0-100 (tectonic/relief proxy, no survey)",
            "stone": "potential index 0-100", "salt": "potential index 0-100 (coastal proximity)",
        },
        "warnings": mapx["warnings"] + [
            "Equirectangular overview: east-west scale varies by about 13 % between the top and bottom rows; route distance_km is measured in the regional plane.",
            "Neural Earth relief is interpolated from a coarse graph; Magna Urbis sees no rivers or lakes.",
        ],
        "provenance": {
            "orogen": {"seed": orogen["seed"], "config": orogen["config"], "node": orogen["node_version"],
                       "fields_sha256": fields_sha, "graph_sha256": graph_sha, "stages": orogen["stages"],
                       "sources_sha256": sha_bytes(json.dumps(orogen["sources"], sort_keys=True).encode())},
            "map_export": {"generator": mapx["generator"], "params": params, "selection_candidates": region["selection_candidates"],
                           "tiles": [{k: t[k] for k in ("tile", "place_seed", "road_seed", "requested", "placed", "road_segments", "fallback_cells")} for t in tiles],
                           "magna_urbis_sources_sha256": mu_sha, "sha256": sha_file(OUT / "map" / "map-export.json")},
            "namebench": {"selection": names["selection"], "database_sha256": names["database"]["sha256"],
                          "code_sha256": names["code_sha256"], "sha256": sha_file(OUT / "names" / "names.json")},
            "pecking_order": {"kb_digest": blue["kb_digest"], "kb_digest_rule": blue["kb_digest_rule"], "counts": blue["counts"],
                              "snapshot_tree_sha256": snapshot["tree_sha256"], "succession_mapping": blue["succession_mapping"],
                              "sha256": sha_file(OUT / "blueprints" / "blueprints.json")},
            "lineage_lab": {"scenario_sha256": people["scenario_sha256"], "preset_sha256": people["preset_sha256"],
                            "selection_rule": people["selection_rule"], "totals": people["totals"],
                            "settlement_mapping": people["settlement_mapping"], "sha256": sha_file(OUT / "people" / "people.json")},
            "tools_sha256": tools_sha,
        },
    }
    return json.dumps(stack, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def main():
    data = build()
    if "--check" in sys.argv[1:]:
        if not FIXTURE.exists() or FIXTURE.read_bytes() != data:
            raise SystemExit("fixtures/stack.json differs from a fresh assembly")
        print(f"fixtures/stack.json is current ({sha_bytes(data)})")
        return
    FIXTURE.parent.mkdir(parents=True, exist_ok=True)
    FIXTURE.write_bytes(data)
    (FIXTURE.parent / "stack.json.sha256").write_text(f"{sha_bytes(data)}  stack.json\n", encoding="utf-8")
    print(f"wrote {FIXTURE.relative_to(ROOT)}: {len(data)} bytes, sha256 {sha_bytes(data)}")


if __name__ == "__main__":
    main()
