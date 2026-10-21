"""Check fixtures/stack.json against the Chronos bootstrap contract (standard library only).

Usage: python tools/validate_fixture.py [path]
"""
import json
import math
import sys
from pathlib import Path

errors = []


def need(cond, message):
    if not cond:
        errors.append(message)


def typed(obj, fields, where):
    for name, kinds in fields.items():
        need(name in obj, f"{where}: missing {name}")
        if name in obj:
            need(isinstance(obj[name], kinds) and not (isinstance(obj[name], bool) and bool not in (kinds if isinstance(kinds, tuple) else (kinds,))),
                 f"{where}.{name}: {type(obj[name]).__name__}")


def main():
    path = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent / "fixtures" / "stack.json")
    s = json.loads(path.read_text(encoding="utf-8"))
    num, opt_str, opt_int = (int, float), (str, type(None)), (int, type(None))
    typed(s, {"schema_version": int, "seed": int, "map": dict, "sites": list, "routes": list, "names": dict,
              "blueprints": list, "people": list, "sources": list}, "stack")
    need(s.get("schema_version") == 1, "schema_version must be 1")
    m = s["map"]
    typed(m, {"width": int, "height": int, "elevation_m": list, "bounds": list, "radius_km": num, "source": str}, "map")
    need(len(m["elevation_m"]) == m["width"] * m["height"], "map.elevation_m length")
    need(all(isinstance(v, num) and math.isfinite(v) for v in m["elevation_m"]), "map.elevation_m finite numbers")
    b = m["bounds"]
    need(len(b) == 4 and b[0] < b[2] and b[1] < b[3], "map.bounds order")
    ids = set()
    for i, site in enumerate(s["sites"]):
        w = f"sites[{i}]"
        typed(site, {"id": str, "x": num, "y": num, "region_id": str, "culture_id": str, "kind": str, "population": int, "resources": dict}, w)
        need(site["id"] not in ids, f"{w}: duplicate id")
        ids.add(site["id"])
        need(0 <= site["x"] <= 1 and 0 <= site["y"] <= 1, f"{w}: x/y outside 0..1")
        need(site["population"] > 0, f"{w}: population")
        typed(site["resources"], {k: num for k in ("food", "water", "timber", "iron", "stone", "salt")}, f"{w}.resources")
        need(site["culture_id"] in s["names"], f"{w}: culture_id without name pool")
    need([x["id"] for x in s["sites"]] == sorted(ids), "sites must be sorted by id (people.site_index relies on it)")
    route_ids = set()
    for i, r in enumerate(s["routes"]):
        w = f"routes[{i}]"
        typed(r, {"id": str, "from": str, "to": str, "distance_km": num, "travel_days": num, "path": list, "kind": str, "source": str}, w)
        need(r["id"] not in route_ids, f"{w}: duplicate id")
        route_ids.add(r["id"])
        need(r["from"] in ids and r["to"] in ids and r["from"] != r["to"], f"{w}: endpoints")
        need(r["distance_km"] > 0 and r["travel_days"] > 0, f"{w}: distance/time")
        need(len(r["path"]) >= 2 and all(len(p) == 2 and 0 <= p[0] <= 1 and 0 <= p[1] <= 1 for p in r["path"]), f"{w}: path")
    for culture in ("french", "british", "fantasy"):
        pool = s["names"].get(culture, {})
        for key in ("forenames", "surnames", "places"):
            values = pool.get(key)
            need(isinstance(values, list) and values and all(isinstance(v, str) and v for v in values), f"names.{culture}.{key}")
            if isinstance(values, list):
                need(values == sorted(set(values)), f"names.{culture}.{key} not sorted/unique")
    for i, bp in enumerate(s["blueprints"]):
        typed(bp, {"id": str, "label": str, "kind": str, "office_title": str, "succession": str, "source": str, "role_id": str, "max_holders": opt_int}, f"blueprints[{i}]")
    need(len(s["blueprints"]) >= 4, "at least four blueprints")
    people = {p["id"]: p for p in s["people"]}
    need(len(people) == len(s["people"]), "duplicate person id")
    for i, p in enumerate(s["people"]):
        w = f"people[{i}]"
        typed(p, {"id": str, "birth_year": int, "death_year": opt_int, "father_id": opt_str, "mother_id": opt_str,
                  "site_index": int, "sex": str, "external_id": str}, w)
        need(0 <= p["site_index"] < len(s["sites"]), f"{w}: site_index")
        need(p["death_year"] is None or p["death_year"] >= p["birth_year"], f"{w}: death before birth")
        need(p["sex"] in ("M", "F"), f"{w}: sex")
        for parent, sex in (("father_id", "M"), ("mother_id", "F")):
            q = people.get(p[parent]) if p[parent] else None
            if p[parent]:
                need(q is not None, f"{w}: {parent} not exported")
            if q:
                need(q["sex"] == sex and q["birth_year"] < p["birth_year"], f"{w}: {parent} sex/age")
                need(parent == "father_id" or q["death_year"] is None or q["death_year"] >= p["birth_year"], f"{w}: mother dead before birth")
    for i, src in enumerate(s["sources"]):
        typed(src, {"id": str, "label": str, "status": str, "detail": str, "fingerprint": str}, f"sources[{i}]")
        need(src["status"] in ("imported", "derived", "synthetic", "unavailable"), f"sources[{i}].status")
        need(":\\" not in src["detail"] and "/Users/" not in src["detail"], f"sources[{i}]: absolute path in detail")
    if errors:
        print("\n".join(errors[:50]))
        raise SystemExit(f"{len(errors)} contract violation(s)")
    adults = sum(1 for p in s["people"] if p["birth_year"] <= 1382 and (p["death_year"] is None or p["death_year"] > 1398))
    print(f"{path.name}: valid - {len(s['sites'])} sites, {len(s['routes'])} routes, {len(s['blueprints'])} blueprints, "
          f"{len(s['people'])} people ({adults} adults alive in 1398), {len(s['sources'])} sources")


if __name__ == "__main__":
    main()
