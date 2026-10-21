"""Simulate a small Lineage Lab population on the Chronos/Magna Urbis sites and export a sample.

Runs Lineage Lab's own engine in memory (`lineage_lab.engine.generate(config, None)`), full
population model, from its bundled medieval preset with explicit settlements = the 52 sites of
output/map/map-export.json (settlement id = index of the site in the fixture's sorted site list).
Nothing is written to the sibling project.

Usage (Lineage Lab virtual environment):
  ../lineage-lab/.venv/Scripts/python.exe tools/lineage/export_people.py ../lineage-lab \
      output/map/map-export.json output/people/people.json
"""
import hashlib
import json
import sys
import time
from pathlib import Path

VERSION = "chronos-lineage-export-v1"
START_YEAR = 1300
YEARS = 120  # census years 1300..1420
FOUNDERS = 2000
# Lineage Lab bounded regulation: the founders are the recovery floor, this the strict ceiling.
# The preset alone declines by about half in 120 years (measured: 3000 -> 1454 without its events).
CEILING = 2800
ELITE_STATUS = 2  # status classes 2 and 3 of the preset's four (0 = most common)
COMMONER_SAMPLE = 120
ADULT_AGE = 16
HERO_YEAR = 1398
ACTIVITIES = {
    "town": {"agriculture": 4, "craft": 3, "trade": 3},
    "city": {"trade": 3, "craft": 5, "agriculture": 2},
    "village": {"agriculture": 9, "craft": 1},
    "hamlet": {"agriculture": 1},
}


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    root, map_path, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2]), Path(sys.argv[3])
    sys.path.insert(0, str(root / "src"))
    import numpy as np
    import yaml
    import lineage_lab
    from lineage_lab.config import Scenario
    from lineage_lab.engine import generate
    from lineage_lab.schema import null_sentinel

    export = json.loads(map_path.read_text(encoding="utf-8"))
    sites = sorted(export["sites"], key=lambda s: s["id"])
    origins = {t["tile"]: t["origin_plane_m"] for t in export["region"]["tiles"]}
    settlements = []
    for index, site in enumerate(sites):
        mu = site["magna_urbis"]
        ox, oy = origins[mu["tile"]]
        settlements.append({
            "id": index, "name": site["id"], "kind": site["kind"],
            # Global azimuthal-equidistant plane, km (x east, y north).
            "x": round((ox + mu["center_m"][0]) / 1000, 3), "y": round((oy - mu["center_m"][1]) / 1000, 3),
            "capacity": int(site["population"]), "initial_weight": 1,
            "activities": ACTIVITIES.get(site["kind"], ACTIVITIES["village"]),
            "metadata": {"chronos_site_id": site["id"], "region_id": site["region_id"]},
        })

    preset_path = root / "src" / "lineage_lab" / "presets" / "medieval.yaml"
    preset_bytes = preset_path.read_bytes()
    scenario = yaml.safe_load(preset_bytes)
    # The preset's war (1060) and epidemic (1100) are dated for its own 1000-1150 window; Chronos
    # owns crises, so they and the preset's 1120 period are removed rather than shifted.
    removed = {"events": scenario.pop("events", []), "periods": scenario.pop("periods", [])}
    scenario.pop("virtual_settlements", None)
    scenario.pop("virtual_spacing", None)
    scenario.pop("generations", None)
    scenario.pop("generation_years", None)
    scenario.update(
        seed=42, start_year=START_YEAR, years=YEARS, initial_population=FOUNDERS,
        target_population=CEILING, target_mode="bounded",
        settlements=settlements, capacity_mode="scale",
        metadata={"description": "Chronos fixture: Lineage Lab medieval preset on the 52 Magna Urbis sites of the Neural Earth region",
                  "chronos_export": VERSION},
    )
    config = Scenario.model_validate(scenario)
    canonical = json.dumps(config.model_dump(mode="json"), sort_keys=True, separators=(",", ":")).encode()
    scenario_sha = sha256_bytes(canonical)

    started = time.perf_counter()
    archive = generate(config, None)
    seconds = time.perf_counter() - started
    data, n = archive.data, archive.n
    end_year = START_YEAR + YEARS
    alive_marker = null_sentinel("death")
    none = {name: null_sentinel(name) for name in ("father", "mother", "death_place")}
    birth, death = data["birth"].astype(np.int64), data["death"].astype(np.int64)
    death_or_end = np.where(death == alive_marker, end_year + 1, death)
    in_window = (birth <= end_year) & (death_or_end >= START_YEAR)
    elite = np.flatnonzero(in_window & (data["status"] >= ELITE_STATUS))
    adult_1398 = np.flatnonzero((birth <= HERO_YEAR - ADULT_AGE) & (death_or_end > HERO_YEAR) & (data["status"] < ELITE_STATUS))
    key = lambda i: hashlib.sha256(f"{VERSION}:{scenario_sha}:{int(i)}".encode()).hexdigest()
    commoners = sorted(adult_1398.tolist(), key=key)[:COMMONER_SAMPLE]
    chosen, stack = set(), [int(i) for i in elite] + commoners
    while stack:  # ancestor closure: every exported parent is exported too
        i = stack.pop()
        if i in chosen:
            continue
        chosen.add(i)
        for field in ("father", "mother"):
            parent = int(data[field][i])
            if parent != none[field]:
                stack.append(parent)

    moves = archive.move_table.data
    moved = {}  # person -> [(year, origin, destination)] in recorded order
    for row in moves[np.argsort(moves["year"], kind="stable")]:
        p = int(row["person"])
        if p in chosen:
            moved.setdefault(p, []).append((int(row["year"]), int(row["origin"]), int(row["destination"])))
    activities = archive.activities
    sampled = set(commoners)
    people = []
    for i in sorted(chosen):
        d = int(death[i])
        father, mother = int(data["father"][i]), int(data["mother"][i])
        rows = moved.get(i, [])
        initial = rows[0][1] if rows else int(data["place"][i])
        activity = int(data["activity"][i])
        people.append({
            "id": f"person-ll-{i}",
            "birth_year": int(birth[i]),
            "death_year": None if d == alive_marker else d,
            "father_id": None if father == none["father"] else f"person-ll-{father}",
            "mother_id": None if mother == none["mother"] else f"person-ll-{mother}",
            # Last recorded residence (death place, or residence at the end of the window).
            "site_index": int(data["place"][i]),
            "sex": "F" if int(data["sex"][i]) == 1 else "M",
            "external_id": f"lineage-lab:{scenario_sha[:12]}:{i}",
            "status": int(data["status"][i]),
            "activity": activities[activity] if 0 <= activity < len(activities) else None,
            "founder": father == none["father"] and mother == none["mother"],
            "birth_site_index": int(data["birth_place"][i]),
            # [from_year, site_index]: residence at max(birth, window start), then each recorded move.
            "residences": [[max(int(birth[i]), START_YEAR), initial]] + [[y, dest] for y, _, dest in rows],
            "selection": "elite" if data["status"][i] >= ELITE_STATUS and in_window[i]
            else "commoner-1398" if i in sampled else "ancestor",
        })

    overview = archive.overview()
    history = overview["history"]
    result = {
        "schema": "chronos-lineage-people-v1", "generator": VERSION,
        "lineage_lab": {"version": getattr(lineage_lab, "__version__", None), "model": "full",
                        "api": "lineage_lab.engine.generate(Scenario, None) -> MemoryArchive"},
        "scenario_sha256": scenario_sha, "preset": "src/lineage_lab/presets/medieval.yaml",
        "preset_sha256": sha256_bytes(preset_bytes), "removed_from_preset": removed,
        "window": [START_YEAR, end_year],
        "selection_rule": (f"status >= {ELITE_STATUS} and alive at some year of {START_YEAR}..{end_year}; plus {COMMONER_SAMPLE} "
                           f"commoners (status < {ELITE_STATUS}) alive and aged >= {ADULT_AGE} in {HERO_YEAR}, SHA-256 keyed sample; "
                           "plus every ancestor of those people (closure to founders)"),
        "totals": {"simulated_people": int(n), "elite_in_window": int(len(elite)), "adult_commoners_1398": int(len(adult_1398)),
                   "exported": len(people), "census_first": history[0], "census_last": history[-1]},
        "status_weights": config.society.status_weights,
        "settlement_mapping": "Lineage Lab settlement id = index of the site in fixture sites (sorted by id); explicit settlements built from Magna Urbis positions (km) and populations (capacity, scaled by Lineage Lab)",
        "scenario": json.loads(canonical),
        "people": people,
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, ensure_ascii=False, indent=1, sort_keys=True), encoding="utf-8")
    print(f"wrote {out}: {len(people)} of {n} simulated people in {seconds:.1f} s; census {history[0]['population']} -> {history[-1]['population']}")


if __name__ == "__main__":
    main()
