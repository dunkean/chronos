//! Chronos map fixture preparation (prep-only; Chronos never runs this at launch).
//!
//! Neural Earth / Orogen initial graph fields (exported on CPU by
//! tools/orogen/export_orogen.mjs) -> one bounded region on an explicitly chosen small
//! planet -> an equirectangular overview raster for Chronos, and four adjacent
//! 100 km x 100 km Magna Urbis tiles (azimuthal equidistant plane) on which the real
//! `magna_urbis_core::settlements::Terrain::new(Raster)`, `place` and `roads` run.
//! Tiles meet through gateway road sites placed on their shared borders, so every
//! route geometry comes from Magna Urbis; no coarse corridor is invented.
//!
//! Usage: chronos-export-map <orogen export dir> <output json>
mod orogen;

use magna_urbis_core::settlements::{
    Config as PlaceConfig, Point, Raster, SettlementSpec, Terrain,
    roads::{RoadConfig, RoadSite},
};
use orogen::{Aeqd, Graph, Sample};
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::path::PathBuf;

const VERSION: &str = "chronos-export-map-v1";
/// Explicit small planet: Neural Earth's `world_diameter_km` (10..100000 accepted).
/// The Orogen graph lives on the unit sphere; only this scale is chosen here.
const DIAMETER_KM: f64 = 1600.0;
const TILE_M: f64 = 100_000.0;
/// Magna Urbis raster cells per tile side (390.6 m cells).
const TILE_N: usize = 256;
/// Chronos overview raster.
const MAP_W: usize = 128;
const MAP_H: usize = 128;
/// Overview margin around the 200 km detailed square, metres.
const MARGIN_M: f64 = 14_000.0;
const TILES: [(&str, usize, usize); 4] = [("nw", 0, 0), ("ne", 1, 0), ("sw", 0, 1), ("se", 1, 1)];
/// Main settlement population per tile (Magna Urbis `counts` mode input, not a census).
const MAIN_POP: [u32; 4] = [11000, 7500, 6000, 9000];
const TOWNS: usize = 3;
const VILLAGES: usize = 9;
const GATEWAY_POP: u32 = 20;
const KM_PER_DAY: f64 = 25.0;
const SPANNER_STRETCH: f64 = 1.15;
/// Region selection rejects land above this height: the smooth coarse source would
/// otherwise let gentle 3-6 km plateaus look buildable to Magna Urbis.
const MAX_LAND_M: f64 = 2600.0;
const LAT_MIN: i32 = 15;
const LAT_MAX: i32 = 50;
/// Magna Urbis placement preference, -1 lowlands .. +1 high ground.
const ELEVATION_PREF: f64 = -0.4;

fn round(v: f64, digits: i32) -> f64 {
    let s = 10f64.powi(digits);
    let r = (v * s).round() / s;
    if r == 0.0 { 0.0 } else { r }
}
fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

struct Region {
    proj: Aeqd,
    score: f64,
    land: f64,
    tile_land: [f64; 4],
    temperature: f64,
    precipitation: f64,
    relief_std: f64,
}

/// Plane coordinates of a tile's north-west corner (x east, y north).
fn tile_origin(col: usize, row: usize) -> (f64, f64) {
    (-TILE_M + col as f64 * TILE_M, TILE_M - row as f64 * TILE_M)
}

fn evaluate(graph: &Graph, proj: Aeqd) -> Option<Region> {
    const K: usize = 8;
    let mut tile_land = [0.0; 4];
    let (mut t, mut p, mut h, mut h2, mut land, mut top) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0f64);
    for (ti, &(_, col, row)) in TILES.iter().enumerate() {
        let (x0, y0) = tile_origin(col, row);
        for j in 0..K {
            for i in 0..K {
                let x = x0 + (i as f64 + 0.5) * TILE_M / K as f64;
                let y = y0 - (j as f64 + 0.5) * TILE_M / K as f64;
                let (lon, lat) = proj.inverse(x, y);
                let s = graph.sample(lon, lat);
                if s.elevation_m > 0.0 {
                    tile_land[ti] += 1.0 / (K * K) as f64;
                    land += 1.0;
                    t += s.temperature_c;
                    p += s.precipitation_mm;
                    h += s.elevation_m;
                    h2 += s.elevation_m * s.elevation_m;
                    top = top.max(s.elevation_m);
                }
            }
        }
    }
    if tile_land.iter().any(|&f| f < 0.6) || top > MAX_LAND_M {
        return None;
    }
    let (t, p, h) = (t / land, p / land, h / land);
    let relief_std = (h2 / land - h * h).max(0.0).sqrt();
    let land_fraction = land / (4 * K * K) as f64;
    if !(0.72..=0.97).contains(&land_fraction)
        || !(6.0..=18.0).contains(&t)
        || !(450.0..=2000.0).contains(&p)
    {
        return None;
    }
    let score = -4.0 * (land_fraction - 0.88).abs()
        - (t - 12.0).abs() / 5.0
        - (p / 900.0).ln().abs()
        - (relief_std - 350.0).abs() / 350.0;
    Some(Region {
        proj,
        score,
        land: land_fraction,
        tile_land,
        temperature: t,
        precipitation: p,
        relief_std,
    })
}

/// Deterministic scan: 1° grid, mid latitudes, away from the antimeridian; first best wins.
fn choose_region(graph: &Graph, radius_m: f64) -> Result<(Region, usize), String> {
    let mut best: Option<Region> = None;
    let mut candidates = 0;
    for lat in (-LAT_MAX..=LAT_MAX).filter(|l: &i32| l.abs() >= LAT_MIN) {
        for lon in -160..=160 {
            let proj = Aeqd {
                lon0: (lon as f64).to_radians(),
                lat0: (lat as f64).to_radians(),
                radius_m,
            };
            if let Some(r) = evaluate(graph, proj) {
                candidates += 1;
                if std::env::var_os("CHRONOS_LIST_REGIONS").is_some() {
                    eprintln!(
                        "candidate lon {lon} lat {lat}: score {:.3} land {:.2} T {:.1} P {:.0} std {:.0}",
                        r.score, r.land, r.temperature, r.precipitation, r.relief_std
                    );
                }
                if best.as_ref().is_none_or(|b| r.score > b.score) {
                    best = Some(r);
                }
            }
        }
    }
    best.map(|b| (b, candidates))
        .ok_or_else(|| "no region satisfies the selection rule".into())
}

#[derive(Serialize)]
struct Resources {
    food: f64,
    water: f64,
    timber: f64,
    iron: f64,
    stone: f64,
    salt: f64,
}

#[derive(Serialize)]
struct Site {
    id: String,
    x: f64,
    y: f64,
    region_id: String,
    culture_id: String,
    kind: String,
    population: u64,
    resources: Resources,
    lon_deg: f64,
    lat_deg: f64,
    elevation_m: f64,
    temperature_c: f64,
    precipitation_mm: f64,
    magna_urbis: serde_json::Value,
}

#[derive(Serialize)]
struct Route {
    id: String,
    from: String,
    to: String,
    distance_km: f64,
    travel_days: f64,
    path: Vec<[f64; 2]>,
    kind: String,
    source: String,
    tiles: Vec<String>,
    gateways: Vec<String>,
}

/// Initial potentials in documented arbitrary units (see docs/data-sources.md).
fn resources(pop: u32, s: &Sample, slope: f64, buildable: f64, water_distance: f64) -> Resources {
    let (t, p) = (s.temperature_c, s.precipitation_mm);
    let climate = clamp01(1.0 - (t - 14.0).abs() / 14.0) * clamp01((p - 200.0) / 600.0);
    let arable = clamp01(buildable) * (1.0 - clamp01(slope / 0.15));
    let suit = climate * (0.4 + 0.6 * arable);
    let pop = pop as f64;
    Resources {
        // Annual capacities in person-years of food / fresh water.
        food: (pop * (0.7 + 0.8 * suit) + 300.0 * suit).round(),
        water: (pop * (0.6 + 1.4 * clamp01((p - 150.0) / 850.0)) + 200.0 * clamp01(p / 1000.0))
            .round(),
        // Potential indices 0..100 (no surveyed deposits).
        timber: (100.0 * clamp01((p - 300.0) / 900.0) * clamp01((t + 2.0) / 10.0)).round(),
        iron: (100.0 * clamp01(0.8 * s.tectonic + s.elevation_m / 4000.0)).round(),
        stone: (100.0 * clamp01(0.15 + slope / 0.06 + s.elevation_m / 2500.0)).round(),
        salt: (100.0 * clamp01(1.0 - water_distance / 8000.0) * if t > 8.0 { 1.0 } else { 0.6 })
            .round(),
    }
}

fn simplify(points: &[(f64, f64)], tolerance: f64) -> Vec<(f64, f64)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut stack = vec![(0, points.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let (pa, pb) = (points[a], points[b]);
        let (dx, dy) = (pb.0 - pa.0, pb.1 - pa.1);
        let len = dx.hypot(dy);
        let mut worst = (0.0, a);
        for (i, p) in points.iter().enumerate().take(b).skip(a + 1) {
            let d = if len == 0.0 {
                (p.0 - pa.0).hypot(p.1 - pa.1)
            } else {
                ((p.0 - pa.0) * dy - (p.1 - pa.1) * dx).abs() / len
            };
            if d > worst.0 {
                worst = (d, i);
            }
        }
        if worst.0 > tolerance {
            keep[worst.1] = true;
            stack.push((a, worst.1));
            stack.push((worst.1, b));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(p, _)| *p)
        .collect()
}

fn length(points: &[(f64, f64)]) -> f64 {
    points
        .windows(2)
        .map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1))
        .sum()
}

struct Edge {
    a: usize,
    b: usize,
    /// Global plane polyline from node a to node b.
    path: Vec<(f64, f64)>,
    class: String,
    tile: String,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: chronos-export-map <orogen export dir> <output json>".into());
    }
    let (input, output) = (PathBuf::from(&args[1]), PathBuf::from(&args[2]));
    let graph = Graph::load(&input)?;
    let radius_m = DIAMETER_KM * 500.0;
    let (region, candidates) = choose_region(&graph, radius_m)?;
    let proj = region.proj;
    eprintln!(
        "region lon {:.0} lat {:.0}: land {:.2} T {:.1} P {:.0} relief {:.0} ({candidates} candidates)",
        proj.lon0.to_degrees(),
        proj.lat0.to_degrees(),
        region.land,
        region.temperature,
        region.precipitation,
        region.relief_std
    );
    let mut warnings: Vec<String> = vec![];

    // Overview bounds enclosing the detailed square plus margin, made metric-square at lat0.
    let half = TILE_M + MARGIN_M;
    let (mut lon_lo, mut lon_hi, mut lat_lo, mut lat_hi) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for k in 0..=400 {
        let s = -half + 2.0 * half * k as f64 / 400.0;
        for (x, y) in [(s, -half), (s, half), (-half, s), (half, s)] {
            let (lon, lat) = proj.inverse(x, y);
            let lon = proj.lon0 + orogen::wrap(lon - proj.lon0);
            lon_lo = lon_lo.min(lon);
            lon_hi = lon_hi.max(lon);
            lat_lo = lat_lo.min(lat);
            lat_hi = lat_hi.max(lat);
        }
    }
    let lat_span_m = (lat_hi - lat_lo) * radius_m;
    let lon_span_m = (lon_hi - lon_lo) * radius_m * proj.lat0.cos();
    if lat_span_m > lon_span_m {
        let extra = (lat_span_m / (radius_m * proj.lat0.cos()) - (lon_hi - lon_lo)) / 2.0;
        lon_lo -= extra;
        lon_hi += extra;
    } else {
        let extra = (lon_span_m / radius_m - (lat_hi - lat_lo)) / 2.0;
        lat_lo -= extra;
        lat_hi += extra;
    }
    let to_map = |x: f64, y: f64| {
        let (lon, lat) = proj.inverse(x, y);
        let lon = proj.lon0 + orogen::wrap(lon - proj.lon0);
        [
            round((lon - lon_lo) / (lon_hi - lon_lo), 5),
            round((lat_hi - lat) / (lat_hi - lat_lo), 5),
        ]
    };

    let mut elevation = Vec::with_capacity(MAP_W * MAP_H);
    let mut temperature = Vec::with_capacity(MAP_W * MAP_H);
    let mut precipitation = Vec::with_capacity(MAP_W * MAP_H);
    let mut fallback_pixels = 0;
    for j in 0..MAP_H {
        let lat = lat_hi - (j as f64 + 0.5) * (lat_hi - lat_lo) / MAP_H as f64;
        for i in 0..MAP_W {
            let lon = lon_lo + (i as f64 + 0.5) * (lon_hi - lon_lo) / MAP_W as f64;
            let s = graph.sample(orogen::wrap(lon), lat);
            fallback_pixels += usize::from(s.fallback);
            elevation.push(round(s.elevation_m, 1));
            temperature.push(round(s.temperature_c, 2));
            precipitation.push(round(s.precipitation_mm, 1));
        }
    }
    let vertices_in_map = graph
        .xyz
        .iter()
        .filter(|p| {
            let lat = p[1].clamp(-1.0, 1.0).asin();
            let lon = proj.lon0 + orogen::wrap(p[2].atan2(p[0]) - proj.lon0);
            (lat_lo..=lat_hi).contains(&lat) && (lon_lo..=lon_hi).contains(&lon)
        })
        .count();

    // Magna Urbis tiles.
    let cell = TILE_M / TILE_N as f64;
    let mut terrains = vec![];
    let mut placed = vec![];
    let mut tile_meta = vec![];
    for (ti, &(name, col, row)) in TILES.iter().enumerate() {
        let (x0, y0) = tile_origin(col, row);
        let mut height = vec![0f32; TILE_N * TILE_N];
        let mut fallback = 0;
        for y in 0..TILE_N {
            for x in 0..TILE_N {
                let (lon, lat) =
                    proj.inverse(x0 + (x as f64 + 0.5) * cell, y0 - (y as f64 + 0.5) * cell);
                let s = graph.sample(lon, lat);
                fallback += usize::from(s.fallback);
                height[y * TILE_N + x] = s.elevation_m as f32;
            }
        }
        let water: Vec<u8> = height.iter().map(|&h| u8::from(h < 0.0)).collect();
        let zeros = vec![0f32; TILE_N * TILE_N];
        let rivers = vec![-1i32; TILE_N * TILE_N];
        let terrain = Terrain::new(Raster {
            width: TILE_M,
            n: TILE_N,
            height: &height,
            water: &water,
            water_level: &zeros,
            river_width: &zeros,
            river_ids: &rivers,
        })?;
        let land: Vec<f32> = height.iter().copied().filter(|&h| h >= 0.0).collect();
        let mean = land.iter().map(|&h| h as f64).sum::<f64>() / land.len().max(1) as f64;
        let std = (land.iter().map(|&h| (h as f64 - mean).powi(2)).sum::<f64>()
            / land.len().max(1) as f64)
            .sqrt();
        let relief = if std > 450.0 {
            "mountains"
        } else if std > 120.0 {
            "hills"
        } else {
            "plains"
        };
        let cfg = PlaceConfig {
            mode: "counts".into(),
            main: SettlementSpec {
                key: "main".into(),
                population: MAIN_POP[ti],
                ..Default::default()
            },
            counts: BTreeMap::from([
                ("town".to_string(), TOWNS),
                ("village".to_string(), VILLAGES),
            ]),
            relief: relief.into(),
            elevation: ELEVATION_PREF,
            ..Default::default()
        };
        let seed = format!("{VERSION}:tile:{name}:place");
        let out = terrain.place(&seed, &cfg)?;
        for w in &out.warnings {
            warnings.push(format!("magna-urbis place {name}: {w}"));
        }
        tile_meta.push(json!({
            "id": format!("region-{name}"), "tile": name, "place_seed": seed, "relief": relief,
            "origin_plane_m": [x0, y0], "size_m": TILE_M, "cells": TILE_N, "cell_m": cell,
            "fallback_cells": fallback, "land_fraction": round(land.len() as f64 / height.len() as f64, 4),
            "land_elevation_std_m": round(std, 1), "requested": out.requested, "placed": out.settlements.len(),
            "usable_km2": round(out.usable_km2, 1), "main_population": MAIN_POP[ti],
            "counts": {"town": TOWNS, "village": VILLAGES},
        }));
        terrains.push(terrain);
        placed.push(out.settlements);
    }

    // Gateways on shared borders, near the closest pair of settlements across the border.
    let global = |ti: usize, p: Point| {
        let (x0, y0) = tile_origin(TILES[ti].1, TILES[ti].2);
        (x0 + p.x, y0 - p.y)
    };
    let land_at = |x: f64, y: f64| {
        let (lon, lat) = proj.inverse(x, y);
        graph.sample(lon, lat).elevation_m
    };
    let mut gateways: Vec<(String, usize, usize, (f64, f64))> = vec![];
    for (a, b, vertical) in [(0, 1, true), (2, 3, true), (0, 2, false), (1, 3, false)] {
        let mut pair = (f64::INFINITY, (0.0, 0.0));
        for sa in &placed[a] {
            for sb in &placed[b] {
                let (pa, pb) = (global(a, sa.center), global(b, sb.center));
                let d = (pa.0 - pb.0).hypot(pa.1 - pb.1);
                if d < pair.0 {
                    pair = (d, ((pa.0 + pb.0) / 2.0, (pa.1 + pb.1) / 2.0));
                }
            }
        }
        // Border line: x = 0 (vertical) or y = 0, spanning one tile side.
        let lo = if vertical {
            if TILES[a].2 == 0 { 0.0 } else { -TILE_M }
        } else if TILES[a].1 == 0 {
            -TILE_M
        } else {
            0.0
        };
        let target = if vertical { pair.1.1 } else { pair.1.0 };
        let mut best: Option<(f64, (f64, f64))> = None;
        for k in 5..=95 {
            let s = lo + k as f64 * 1000.0;
            let at = |o: f64| if vertical { (o, s) } else { (s, o) };
            let (h0, h1) = (
                land_at(at(-1000.0).0, at(-1000.0).1),
                land_at(at(1000.0).0, at(1000.0).1),
            );
            if h0 <= 0.0 || h1 <= 0.0 || land_at(at(0.0).0, at(0.0).1) <= 0.0 {
                continue;
            }
            let cost = (s - target).abs() / 1000.0 + 50.0 * ((h1 - h0).abs() / 2000.0) / 0.05;
            if best.is_none_or(|(c, _)| cost < c) {
                best = Some((cost, at(0.0)));
            }
        }
        let key = format!("gw-{}-{}", TILES[a].0, TILES[b].0);
        match best {
            Some((_, p)) => gateways.push((key, a, b, p)),
            None => warnings.push(format!(
                "no land crossing found on the {key} border; tiles stay unlinked there"
            )),
        }
    }

    // Roads per tile: placed settlements plus that tile's gateways; no automatic border exits.
    let mut nodes: BTreeMap<String, usize> = BTreeMap::new();
    let mut node_names: Vec<String> = vec![];
    let mut node_of = |key: String, names: &mut Vec<String>| {
        let n = nodes.len();
        *nodes.entry(key.clone()).or_insert_with(|| {
            names.push(key);
            n
        })
    };
    let mut edges: Vec<Edge> = vec![];
    let mut sites: Vec<Site> = vec![];
    let site_id = |tile: &str, key: &str| format!("site-{tile}-{}", key.replace(':', "-"));
    for (ti, &(name, col, row)) in TILES.iter().enumerate() {
        let (x0, y0) = tile_origin(col, row);
        let mut road_sites: Vec<RoadSite> = placed[ti]
            .iter()
            .map(|s| RoadSite {
                key: s.spec.key.clone(),
                population: s.spec.population,
                culture: s.spec.culture.clone(),
                center: s.center,
                radius: s.radius,
            })
            .collect();
        for (key, a, b, p) in &gateways {
            if *a == ti || *b == ti {
                let local = Point {
                    x: (p.0 - x0).clamp(cell / 2.0, TILE_M - cell / 2.0),
                    y: (y0 - p.1).clamp(cell / 2.0, TILE_M - cell / 2.0),
                };
                road_sites.push(RoadSite {
                    key: key.clone(),
                    population: GATEWAY_POP,
                    culture: "medieval".into(),
                    center: local,
                    radius: 0.0,
                });
            }
        }
        let cfg = RoadConfig {
            exits: 0,
            ..Default::default()
        };
        let seed = format!("{VERSION}:tile:{name}:roads");
        let out = terrains[ti].roads(&seed, &road_sites, &cfg)?;
        for w in &out.warnings {
            warnings.push(format!("magna-urbis roads {name}: {w}"));
        }
        for u in &out.unreachable {
            warnings.push(format!("magna-urbis roads {name}: unreachable {u}"));
        }
        tile_meta[ti]["road_seed"] = json!(seed);
        tile_meta[ti]["road_segments"] = json!(out.segments.len());
        tile_meta[ti]["road_routes"] = json!(out.routes);
        tile_meta[ti]["road_length_by_class_m"] = json!(
            out.length_by_class
                .iter()
                .map(|(k, v)| (k.clone(), round(*v, 1)))
                .collect::<BTreeMap<_, _>>()
        );
        let junction_node: Vec<usize> = out
            .junctions
            .iter()
            .enumerate()
            .map(|(j, junction)| {
                let key = match &junction.site {
                    Some(k) if k.starts_with("gw-") => k.clone(),
                    Some(k) if k != "exit" => format!("site:{}", site_id(name, k)),
                    _ => format!("junction:{name}:{j}"),
                };
                node_of(key, &mut node_names)
            })
            .collect();
        for seg in &out.segments {
            let mut path: Vec<(f64, f64)> = seg.path.iter().map(|&p| global(ti, p)).collect();
            let start = global(ti, out.junctions[seg.a].point);
            if let (Some(first), Some(last)) = (path.first(), path.last())
                && (first.0 - start.0).hypot(first.1 - start.1)
                    > (last.0 - start.0).hypot(last.1 - start.1)
            {
                path.reverse();
            }
            edges.push(Edge {
                a: junction_node[seg.a],
                b: junction_node[seg.b],
                path,
                class: seg.class.clone(),
                tile: name.into(),
            });
        }
        for s in &placed[ti] {
            let (gx, gy) = global(ti, s.center);
            let (lon, lat) = proj.inverse(gx, gy);
            let sample = graph.sample(lon, lat);
            let [x, y] = to_map(gx, gy);
            sites.push(Site {
                id: site_id(name, &s.spec.key),
                x,
                y,
                region_id: format!("region-{name}"),
                // Name-pool culture per half of the region (west / east); see docs.
                culture_id: if col == 0 { "french" } else { "british" }.into(),
                kind: s.cls.clone(),
                population: s.spec.population as u64,
                resources: resources(s.spec.population, &sample, s.scores.slope, s.scores.buildable_fraction, s.scores.water_distance),
                lon_deg: round(lon.to_degrees(), 5),
                lat_deg: round(lat.to_degrees(), 5),
                elevation_m: round(sample.elevation_m, 1),
                temperature_c: round(sample.temperature_c, 2),
                precipitation_mm: round(sample.precipitation_mm, 1),
                magna_urbis: json!({
                    "tile": name, "key": s.spec.key, "class": s.cls, "archetype": s.archetype,
                    "center_m": [round(s.center.x, 1), round(s.center.y, 1)], "radius_m": round(s.radius, 1),
                    "score": round(s.scores.total, 4), "slope": round(s.scores.slope, 4),
                    "buildable_fraction": round(s.scores.buildable_fraction, 4),
                    "water_distance_m": round(s.scores.water_distance, 1),
                }),
            });
        }
    }
    sites.sort_by(|a, b| a.id.cmp(&b.id));

    // Site-to-site routes: shortest paths over the merged network that pass through plain
    // junctions and gateways but never through another settlement.
    let n = node_names.len();
    let mut adjacency = vec![vec![]; n];
    for (e, edge) in edges.iter().enumerate() {
        let w = length(&edge.path);
        adjacency[edge.a].push((edge.b, w, e));
        adjacency[edge.b].push((edge.a, w, e));
    }
    #[derive(PartialEq)]
    struct Item(f64, usize);
    impl Eq for Item {}
    impl PartialOrd for Item {
        fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(o))
        }
    }
    impl Ord for Item {
        fn cmp(&self, o: &Self) -> std::cmp::Ordering {
            o.0.total_cmp(&self.0).then(o.1.cmp(&self.1))
        }
    }
    let is_site = |i: usize| node_names[i].starts_with("site:");
    let mut routes = vec![];
    for source in (0..n).filter(|&i| is_site(i)) {
        let mut dist = vec![f64::INFINITY; n];
        let mut prev: Vec<Option<(usize, usize)>> = vec![None; n];
        let mut heap = BinaryHeap::from([Item(0.0, source)]);
        dist[source] = 0.0;
        while let Some(Item(d, u)) = heap.pop() {
            if d > dist[u] || (u != source && is_site(u)) {
                continue;
            }
            for &(v, w, e) in &adjacency[u] {
                if d + w < dist[v] {
                    dist[v] = d + w;
                    prev[v] = Some((u, e));
                    heap.push(Item(d + w, v));
                }
            }
        }
        for target in (0..n)
            .filter(|&t| is_site(t) && node_names[t] > node_names[source] && dist[t].is_finite())
        {
            let mut chain = vec![];
            let mut at = target;
            while let Some((p, e)) = prev[at] {
                chain.push((p, e));
                at = p;
            }
            chain.reverse();
            let mut path: Vec<(f64, f64)> = vec![];
            let mut class_length: BTreeMap<&str, f64> = BTreeMap::new();
            let mut tiles = BTreeSet::new();
            let mut via = vec![];
            for (from, e) in &chain {
                let edge = &edges[*e];
                let mut part = edge.path.clone();
                if edge.a != *from {
                    part.reverse();
                }
                *class_length.entry(&edge.class).or_default() += length(&part);
                tiles.insert(edge.tile.clone());
                if node_names[*from].starts_with("gw-") {
                    via.push(node_names[*from].clone());
                }
                if path.last() == part.first() {
                    path.extend_from_slice(&part[1..]);
                } else {
                    path.extend(part);
                }
            }
            let metres = length(&path);
            let kind = class_length
                .iter()
                .max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(a.0)))
                .map(|(k, _)| k.to_string())
                .unwrap_or_default();
            let (from, to) = (
                node_names[source][5..].to_string(),
                node_names[target][5..].to_string(),
            );
            let distance_km = round(metres / 1000.0, 3);
            routes.push(Route {
                id: format!("route-{}--{}", &from[5..], &to[5..]),
                from,
                to,
                distance_km,
                travel_days: round(distance_km / KM_PER_DAY, 2),
                path: simplify(&path, 120.0)
                    .into_iter()
                    .map(|(x, y)| to_map(x, y))
                    .collect(),
                kind,
                source: "magna-urbis-roads".into(),
                tiles: tiles.into_iter().collect(),
                gateways: via,
            });
        }
    }
    // Trunk sharing makes many settlements mutually reachable through one junction. Keep a
    // greedy 1.15-spanner (shortest first; a link is kept only when the kept links cannot
    // already connect its ends within 1.15 x its network length): connectivity is preserved.
    let candidates_total = routes.len();
    routes.sort_by(|a, b| {
        a.distance_km
            .total_cmp(&b.distance_km)
            .then(a.id.cmp(&b.id))
    });
    let index: BTreeMap<&str, usize> = sites
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();
    let mut kept_adj: Vec<Vec<(usize, f64)>> = vec![vec![]; sites.len()];
    let mut keep = vec![false; routes.len()];
    for (r, route) in routes.iter().enumerate() {
        let (a, b) = (index[route.from.as_str()], index[route.to.as_str()]);
        let limit = SPANNER_STRETCH * route.distance_km;
        let mut dist = vec![f64::INFINITY; sites.len()];
        let mut heap = BinaryHeap::from([Item(0.0, a)]);
        dist[a] = 0.0;
        while let Some(Item(d, u)) = heap.pop() {
            if d > dist[u] || u == b {
                continue;
            }
            for &(v, w) in &kept_adj[u] {
                if d + w < dist[v] && d + w <= limit {
                    dist[v] = d + w;
                    heap.push(Item(d + w, v));
                }
            }
        }
        if dist[b] > limit {
            keep[r] = true;
            kept_adj[a].push((b, route.distance_km));
            kept_adj[b].push((a, route.distance_km));
        }
    }
    let mut routes: Vec<Route> = routes
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(r, _)| r)
        .collect();
    routes.sort_by(|a, b| a.id.cmp(&b.id));
    let linked: BTreeSet<&str> = routes
        .iter()
        .flat_map(|r| [r.from.as_str(), r.to.as_str()])
        .collect();
    for s in &sites {
        if !linked.contains(s.id.as_str()) {
            warnings.push(format!("{} has no Magna Urbis road", s.id));
        }
    }

    let result = json!({
        "schema": "chronos-map-export-v1",
        "generator": VERSION,
        "magna_urbis_core": env!("CARGO_PKG_VERSION"),
        "params": {
            "planet_diameter_km": DIAMETER_KM, "radius_km": DIAMETER_KM / 2.0,
            "tile_m": TILE_M, "tile_cells": TILE_N, "map": [MAP_W, MAP_H], "margin_m": MARGIN_M,
            "main_population": MAIN_POP, "placement_elevation_preference": ELEVATION_PREF, "towns_per_tile": TOWNS, "villages_per_tile": VILLAGES,
            "gateway_population": GATEWAY_POP, "travel_km_per_day": KM_PER_DAY,
            "route_spanner_stretch": SPANNER_STRETCH, "route_candidates": candidates_total,
            "road_config": RoadConfig { exits: 0, ..Default::default() },
            "selection_rule": "1° scan |lat| 15..50, |lon|<=160; each tile >=60% land; land 72..97%; land BIO1 6..18 °C; BIO12 450..2000 mm; sampled land <= 2600 m; score -4|land-0.88| - |T-12|/5 - |ln(P/900)| - |std-350|/350; first maximum",
        },
        "region": {
            "center_lon_deg": round(proj.lon0.to_degrees(), 6), "center_lat_deg": round(proj.lat0.to_degrees(), 6),
            "projection": "azimuthal equidistant (plane x east, y north) for Magna Urbis tiles; equirectangular overview",
            "selection_candidates": candidates, "score": round(region.score, 4), "land_fraction": round(region.land, 4),
            "tile_land_fraction": region.tile_land.map(|v| round(v, 4)),
            "land_temperature_c": round(region.temperature, 2), "land_precipitation_mm": round(region.precipitation, 1),
            "land_elevation_std_m": round(region.relief_std, 1),
            "tiles": tile_meta,
            "gateways": gateways.iter().map(|(k, a, b, p)| json!({"id": k, "between": [format!("region-{}", TILES[*a].0), format!("region-{}", TILES[*b].0)], "plane_m": [round(p.0, 1), round(p.1, 1)], "xy": to_map(p.0, p.1)})).collect::<Vec<_>>(),
        },
        "map": {
            "width": MAP_W, "height": MAP_H, "elevation_m": elevation,
            "bounds": [round(orogen::wrap(lon_lo).to_degrees(), 6), round(lat_lo.to_degrees(), 6), round(orogen::wrap(lon_hi).to_degrees(), 6), round(lat_hi.to_degrees(), 6)],
            "radius_km": DIAMETER_KM / 2.0,
            "source": "neural-earth-orogen-initial",
            "projection": "equirectangular",
            "temperature_c": temperature, "precipitation_mm": precipitation,
            "graph_vertices": graph.len(), "graph_vertices_in_bounds": vertices_in_map, "fallback_pixels": fallback_pixels,
        },
        "sites": sites,
        "routes": routes,
        "warnings": warnings,
        "settlement_api": "magna_urbis_core::settlements::Terrain::new(Raster) + place(counts) + roads(exits=0)",
    });
    if let Some(dir) = output.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        &output,
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    eprintln!(
        "wrote {}: {} sites, {} routes, {} warnings",
        output.display(),
        result["sites"].as_array().map_or(0, Vec::len),
        result["routes"].as_array().map_or(0, Vec::len),
        result["warnings"].as_array().map_or(0, Vec::len)
    );
    Ok(())
}
