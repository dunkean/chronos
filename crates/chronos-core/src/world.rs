//! Geography: map raster, sites, routes and the travel-time graph.
//!
//! Two providers fill the same structures: the imported stack fixture
//! (Neural Earth / Magna Urbis outputs prepared by `tools/`) and a fully
//! synthetic virtual world used as an explicit fallback.

use std::collections::BTreeSet;

use crate::keys::{fingerprint, unit};
use crate::model::{Config, MapData, Resources, Route, Site, Source, StackFixture};

pub struct Geography {
    pub map: MapData,
    /// `name` may be empty (named later) and `polity_id` is assigned later.
    pub sites: Vec<Site>,
    pub routes: Vec<Route>,
    pub sources: Vec<Source>,
    pub warnings: Vec<String>,
}

/// Longitude/latitude in degrees of a normalized map position.
pub fn lon_lat(map: &MapData, x: f64, y: f64) -> (f64, f64) {
    let [lon0, lat0, lon1, lat1] = map.bounds;
    (lon0 + x * (lon1 - lon0), lat1 - y * (lat1 - lat0))
}

/// Great-circle distance in km between two normalized map positions.
pub fn surface_km(map: &MapData, radius_km: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    let (lon_a, lat_a) = lon_lat(map, a.0, a.1);
    let (lon_b, lat_b) = lon_lat(map, b.0, b.1);
    let (p1, p2) = (lat_a.to_radians(), lat_b.to_radians());
    let dp = p2 - p1;
    let dl = (lon_b - lon_a).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * radius_km * h.sqrt().min(1.0).asin()
}

pub fn path_km(map: &MapData, radius_km: f64, path: &[Vec<f64>]) -> f64 {
    path.windows(2)
        .filter(|w| w[0].len() >= 2 && w[1].len() >= 2)
        .map(|w| surface_km(map, radius_km, (w[0][0], w[0][1]), (w[1][0], w[1][1])))
        .sum()
}

/// Canonical travel time: whole days, at least one.
pub fn travel_days(distance_km: f64, km_per_day: f64) -> f64 {
    (distance_km / km_per_day).ceil().max(1.0)
}

fn elevation_at(map: &MapData, x: f64, y: f64) -> f64 {
    let cx = ((x * map.width as f64) as usize).min(map.width - 1);
    let cy = ((y * map.height as f64) as usize).min(map.height - 1);
    map.elevation_m[cy * map.width + cx]
}

// ---------------------------------------------------------------------------
// Imported stack fixture

pub fn stack_geography(cfg: &Config, fx: &StackFixture) -> Result<Geography, String> {
    let map = &fx.map;
    if map.width == 0 || map.height == 0 || map.elevation_m.len() != map.width * map.height {
        return Err(format!(
            "carte de la fixture incohérente : {}×{} mais {} altitudes",
            map.width,
            map.height,
            map.elevation_m.len()
        ));
    }
    if map.bounds.iter().any(|v| !v.is_finite())
        || map.bounds[0] >= map.bounds[2]
        || map.bounds[1] >= map.bounds[3]
    {
        return Err("bornes de carte de la fixture invalides".into());
    }
    if fx.sites.len() < 8 {
        return Err("la fixture contient moins de 8 sites".into());
    }
    let mut warnings = Vec::new();
    let mut ids = BTreeSet::new();
    for s in &fx.sites {
        if !ids.insert(s.id.as_str()) {
            return Err(format!("ID de site dupliqué dans la fixture : {}", s.id));
        }
        if !(0.0..=1.0).contains(&s.x) || !(0.0..=1.0).contains(&s.y) {
            return Err(format!(
                "site {} hors de la carte (x, y doivent être dans 0..1)",
                s.id
            ));
        }
    }
    let n = fx.sites.len();
    if n > 300 || cfg.years as u64 * n as u64 > crate::model::MAX_SITE_YEARS {
        return Err(format!(
            "la fixture a {n} sites : trop pour {} ans d'instantanés annuels",
            cfg.years
        ));
    }
    if cfg.site_count != n {
        warnings.push(format!(
            "site_count = {} remplacé par {} : la carte importée fixe ses sites (positions figées) ; la configuration appliquée l'indique.",
            cfg.site_count, n
        ));
    }

    // Relative populations come from the fixture; the total follows the configuration.
    let weights: Vec<f64> = fx
        .sites
        .iter()
        .map(|s| s.population.max(1) as f64)
        .collect();
    let fixture_total: u64 = fx.sites.iter().map(|s| s.population).sum();
    let populations = apportion(cfg.population, &weights);
    if fixture_total != cfg.population {
        warnings.push(format!(
            "Populations : répartition relative importée, total remis à l'échelle de {} à {} habitants.",
            fixture_total, cfg.population
        ));
    }
    let sites: Vec<Site> = fx
        .sites
        .iter()
        .zip(&populations)
        .map(|(s, &pop)| Site {
            id: s.id.clone(),
            name: s.name.clone().unwrap_or_default(),
            x: s.x,
            y: s.y,
            region_id: if s.region_id.is_empty() {
                "region-1".into()
            } else {
                s.region_id.clone()
            },
            culture_id: if s.culture_id.is_empty() {
                cfg.culture.clone()
            } else {
                s.culture_id.clone()
            },
            kind: if s.kind.is_empty() {
                "settlement".into()
            } else {
                s.kind.clone()
            },
            initial_population: pop,
            polity_id: String::new(),
            resources: s.resources.clone(),
        })
        .collect();

    let radius_scale = if map.radius_km > 0.0 {
        cfg.world_radius_km / map.radius_km
    } else {
        1.0
    };
    if map.radius_km > 0.0 && (radius_scale - 1.0).abs() > 1e-9 {
        warnings.push(format!(
            "Rayon configuré {:.0} km ≠ rayon source {:.0} km : distances et temps multipliés par {:.3} ; la géométrie du raster importé est inchangée.",
            cfg.world_radius_km, map.radius_km, radius_scale
        ));
    }
    let mut routes = Vec::new();
    let mut rejected = 0;
    let mut route_ids = BTreeSet::new();
    for r in &fx.routes {
        if r.from == r.to
            || !ids.contains(r.from.as_str())
            || !ids.contains(r.to.as_str())
            || !route_ids.insert(r.id.clone())
        {
            rejected += 1;
            continue;
        }
        let distance_km = if r.distance_km.is_finite() && r.distance_km > 0.0 {
            r.distance_km * radius_scale
        } else {
            path_km(map, cfg.world_radius_km, &r.path)
        };
        if !distance_km.is_finite() || distance_km <= 0.0 {
            rejected += 1;
            continue;
        }
        routes.push(Route {
            id: r.id.clone(),
            from: r.from.clone(),
            to: r.to.clone(),
            distance_km: round2(distance_km),
            travel_days: travel_days(distance_km, cfg.travel_km_per_day),
            path: r.path.clone(),
            kind: r.kind.clone(),
            source: r.source.clone(),
        });
    }
    if rejected > 0 {
        warnings.push(format!("{rejected} route(s) de la fixture rejetée(s) : extrémités inconnues, doublon ou longueur invalide."));
    }
    // The source radius stays physical metadata; only travel distances are scaled.
    let map_out = map.clone();
    let geo = Geography {
        map: map_out,
        sites,
        routes,
        sources: vec![Source {
            id: "chronos-stack-adapter".into(),
            label: "Adaptation Chronos de la fixture".into(),
            status: "derived".into(),
            detail: format!(
                "Sites, routes et carte lus dans fixtures/stack.json ; populations remises à l'échelle (plus grand reste), temps de trajet = ceil(distance / {} km/j). Rayon source {} km.",
                cfg.travel_km_per_day, map.radius_km
            ),
            fingerprint: fingerprint(ALG_TAG.as_bytes()),
        }],
        warnings,
    };
    Ok(finish_connectivity(geo))
}

const ALG_TAG: &str = "chronos-geography-1";

// ---------------------------------------------------------------------------
// Virtual fallback

const VW: usize = 128;
const VH: usize = 96;
const VBOUNDS: [f64; 4] = [-6.0, 40.0, 14.0, 53.0];

fn lattice(seed: u64, octave: usize, ix: i64, iy: i64) -> f64 {
    unit(
        seed,
        &[
            "terrain",
            &octave.to_string(),
            &ix.to_string(),
            &iy.to_string(),
        ],
    )
}

fn value_noise(seed: u64, octave: usize, x: f64, y: f64) -> f64 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (ix, iy) = (x0 as i64, y0 as i64);
    let a = lattice(seed, octave, ix, iy);
    let b = lattice(seed, octave, ix + 1, iy);
    let c = lattice(seed, octave, ix, iy + 1);
    let d = lattice(seed, octave, ix + 1, iy + 1);
    let top = a + (b - a) * sx;
    let bottom = c + (d - c) * sx;
    top + (bottom - top) * sy
}

fn virtual_map(cfg: &Config) -> MapData {
    let mut elevation_m = Vec::with_capacity(VW * VH);
    for cy in 0..VH {
        for cx in 0..VW {
            let (x, y) = ((cx as f64 + 0.5) / VW as f64, (cy as f64 + 0.5) / VH as f64);
            let mut n = 0.0;
            let mut amp = 0.5;
            let mut freq = 4.0;
            for octave in 0..5 {
                n += amp * value_noise(cfg.seed, octave, x * freq, y * freq * 0.75);
                amp *= 0.5;
                freq *= 2.0;
            }
            let (dx, dy) = ((x - 0.5) / 0.5, (y - 0.5) / 0.5);
            let mask = 1.0 - (dx * dx * 0.9 + dy * dy).min(1.5);
            let h = n * 0.75 + mask * 0.55 - 0.62;
            let e = if h >= 0.0 {
                h * 3800.0 + 5.0
            } else {
                h * 3000.0 - 5.0
            };
            elevation_m.push(e.round());
        }
    }
    MapData {
        width: VW,
        height: VH,
        elevation_m,
        bounds: VBOUNDS,
        radius_km: cfg.world_radius_km,
        source: "chronos-virtual: bruit de valeur déterministe (synthétique, aucune donnée Neural Earth)".into(),
    }
}

pub fn virtual_geography(cfg: &Config) -> Geography {
    let map = virtual_map(cfg);
    let mut warnings = Vec::new();
    let land: Vec<usize> = (0..VW * VH).filter(|&i| map.elevation_m[i] > 0.0).collect();
    let is_sea = |cx: i64, cy: i64| -> bool {
        if cx < 0 || cy < 0 || cx >= VW as i64 || cy >= VH as i64 {
            return true;
        }
        map.elevation_m[cy as usize * VW + cx as usize] <= 0.0
    };
    let coastal = |i: usize| -> bool {
        let (cx, cy) = ((i % VW) as i64, (i / VW) as i64);
        (-2..=2).any(|dy| (-2..=2).any(|dx| is_sea(cx + dx, cy + dy)))
    };

    // Rank land cells by suitability (lowland, coast) with a stable jitter.
    let mut candidates: Vec<(f64, usize)> = land
        .iter()
        .map(|&i| {
            let e = map.elevation_m[i];
            let lowland = 1.0 - (e / 1500.0).min(1.0);
            let coast = if coastal(i) { 0.25 } else { 0.0 };
            let jitter = unit(cfg.seed, &["site-cell", &i.to_string()]);
            (lowland * 0.6 + coast + jitter * 0.6, i)
        })
        .collect();
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let n = cfg.site_count;
    let mut spacing = (land.len() as f64 / n as f64).sqrt() * 0.8;
    let mut chosen: Vec<usize> = Vec::new();
    while chosen.len() < n && spacing >= 0.5 {
        for &(_, i) in &candidates {
            if chosen.len() >= n {
                break;
            }
            let (cx, cy) = ((i % VW) as f64, (i / VW) as f64);
            let ok = chosen.iter().all(|&j| {
                let (jx, jy) = ((j % VW) as f64, (j / VW) as f64);
                ((cx - jx).powi(2) + (cy - jy).powi(2)).sqrt() >= spacing
            });
            if ok && !chosen.contains(&i) {
                chosen.push(i);
            }
        }
        spacing *= 0.75;
    }
    if chosen.len() < n {
        warnings.push(format!(
            "Carte virtuelle : seulement {} emplacements terrestres pour {} sites demandés.",
            chosen.len(),
            n
        ));
    }

    let weights: Vec<f64> = (0..chosen.len())
        .map(|r| {
            (0.7 + 0.6 * unit(cfg.seed, &["site-weight", &r.to_string()]))
                / ((r + 1) as f64).powf(0.85)
        })
        .collect();
    let populations = apportion(cfg.population, &weights);
    let region_count = if chosen.len() > 60 { 3 } else { 2 };
    let cities = (chosen.len() / 12).max(1);
    let towns = chosen.len() / 3;
    let sites: Vec<Site> = chosen
        .iter()
        .enumerate()
        .map(|(r, &i)| {
            let id = format!("site-{:03}", r + 1);
            let (cx, cy) = ((i % VW) as f64, (i / VW) as f64);
            let jx = unit(cfg.seed, &["site-jx", &id]) * 0.6 + 0.2;
            let jy = unit(cfg.seed, &["site-jy", &id]) * 0.6 + 0.2;
            let (x, y) = ((cx + jx) / VW as f64, (cy + jy) / VH as f64);
            let e = map.elevation_m[i];
            let u = |what: &str| unit(cfg.seed, &["resource", what, &id]);
            let resources = Resources {
                food: round2(100.0 * (1.0 - (e / 1600.0).min(0.9)) * (0.6 + 0.4 * u("food"))),
                water: round2(40.0 + 60.0 * u("water")),
                timber: round2(25.0 + 75.0 * u("timber")),
                iron: round2((e / 1200.0).min(1.0) * 75.0 + 25.0 * u("iron")),
                stone: round2((e / 900.0).min(1.0) * 65.0 + 35.0 * u("stone")),
                salt: round2(if coastal(i) {
                    50.0 + 50.0 * u("salt")
                } else {
                    12.0 * u("salt")
                }),
            };
            let kind = if r < cities {
                "city"
            } else if r < cities + towns {
                "town"
            } else {
                "village"
            };
            let region = ((x * region_count as f64) as usize).min(region_count - 1) + 1;
            Site {
                id,
                name: String::new(),
                x: round4(x),
                y: round4(y),
                region_id: format!("region-{region}"),
                culture_id: cfg.culture.clone(),
                kind: kind.into(),
                initial_population: populations[r],
                polity_id: String::new(),
                resources,
            }
        })
        .collect();

    // k nearest neighbours over land; links crossing the sea are not invented.
    let mut pairs = BTreeSet::new();
    for (a, sa) in sites.iter().enumerate() {
        let mut near: Vec<(f64, usize)> = sites
            .iter()
            .enumerate()
            .filter(|&(b, _)| b != a)
            .map(|(b, sb)| ((sa.x - sb.x).powi(2) + (sa.y - sb.y).powi(2), b))
            .collect();
        near.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)));
        for &(_, b) in near.iter().take(3) {
            pairs.insert((a.min(b), a.max(b)));
        }
    }
    let mut routes = Vec::new();
    let mut sea_blocked = 0;
    for (a, b) in pairs {
        let (sa, sb) = (&sites[a], &sites[b]);
        let steps = 24;
        let crosses_sea = (1..steps).any(|k| {
            let t = k as f64 / steps as f64;
            elevation_at(&map, sa.x + (sb.x - sa.x) * t, sa.y + (sb.y - sa.y) * t) <= 0.0
        });
        if crosses_sea {
            sea_blocked += 1;
            continue;
        }
        let path = vec![vec![sa.x, sa.y], vec![sb.x, sb.y]];
        let distance_km = path_km(&map, cfg.world_radius_km, &path);
        routes.push(Route {
            id: format!("route-{}-{}", sa.id, sb.id),
            from: sa.id.clone(),
            to: sb.id.clone(),
            distance_km: round2(distance_km),
            travel_days: travel_days(distance_km, cfg.travel_km_per_day),
            path,
            kind: "track".into(),
            source: "chronos-virtual".into(),
        });
    }
    if sea_blocked > 0 {
        warnings.push(format!("Carte virtuelle : {sea_blocked} liaison(s) traversant la mer non créée(s) (pas de route maritime inventée)."));
    }
    let geo = Geography {
        map,
        sites,
        routes,
        sources: vec![Source {
            id: "chronos-virtual-map".into(),
            label: "Carte, sites et routes virtuels".into(),
            status: "synthetic".into(),
            detail: format!(
                "Repli autonome : relief par bruit de valeur seedé, sites sur les meilleures cellules terrestres, 3 plus proches voisins sans traversée maritime, distances orthodromiques (rayon {} km).",
                cfg.world_radius_km
            ),
            fingerprint: fingerprint(format!("{ALG_TAG}/virtual/{}", cfg.seed).as_bytes()),
        }],
        warnings,
    };
    finish_connectivity(geo)
}

fn finish_connectivity(mut geo: Geography) -> Geography {
    let graph = Graph::new(&geo.sites, &geo.routes);
    let (dist, _) = graph.dijkstra(&[0]);
    let unreachable = dist.iter().filter(|d| d.is_infinite()).count();
    if unreachable > 0 {
        geo.warnings.push(format!(
            "{unreachable} site(s) sans liaison terrestre avec le premier site : ils restent isolés (aucune route inventée)."
        ));
    }
    geo
}

/// Splits `total` proportionally to `weights` with the largest-remainder rule
/// (exact total, ties by index).
pub fn apportion(total: u64, weights: &[f64]) -> Vec<u64> {
    let sum: f64 = weights.iter().sum();
    if weights.is_empty() || sum <= 0.0 {
        return vec![0; weights.len()];
    }
    let exact: Vec<f64> = weights.iter().map(|w| total as f64 * w / sum).collect();
    let mut out: Vec<u64> = exact.iter().map(|e| e.floor() as u64).collect();
    let assigned: u64 = out.iter().sum();
    let mut order: Vec<usize> = (0..weights.len()).collect();
    order.sort_by(|&a, &b| {
        (exact[b] - exact[b].floor())
            .total_cmp(&(exact[a] - exact[a].floor()))
            .then(a.cmp(&b))
    });
    let mut rest = total.saturating_sub(assigned);
    for &i in order.iter().cycle() {
        if rest == 0 {
            break;
        }
        out[i] += 1;
        rest -= 1;
    }
    out
}

pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

// ---------------------------------------------------------------------------
// Travel graph

pub struct Graph {
    /// For each site index: (neighbour index, travel days).
    pub adj: Vec<Vec<(usize, f64)>>,
}

impl Graph {
    pub fn new(sites: &[Site], routes: &[Route]) -> Graph {
        let index: std::collections::BTreeMap<&str, usize> = sites
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.as_str(), i))
            .collect();
        let mut adj = vec![Vec::new(); sites.len()];
        for r in routes {
            if let (Some(&a), Some(&b)) = (index.get(r.from.as_str()), index.get(r.to.as_str())) {
                adj[a].push((b, r.travel_days));
                adj[b].push((a, r.travel_days));
            }
        }
        for list in &mut adj {
            list.sort_by(|p, q| p.0.cmp(&q.0).then(p.1.total_cmp(&q.1)));
        }
        Graph { adj }
    }

    /// Multi-source shortest travel days. Returns (days, index of nearest source).
    /// Ties are broken by source order, then site index, so results are stable.
    pub fn dijkstra(&self, sources: &[usize]) -> (Vec<f64>, Vec<Option<usize>>) {
        let n = self.adj.len();
        let mut dist = vec![f64::INFINITY; n];
        let mut owner = vec![None; n];
        let mut done = vec![false; n];
        for (k, &s) in sources.iter().enumerate() {
            if s < n && dist[s].is_infinite() {
                dist[s] = 0.0;
                owner[s] = Some(k);
            }
        }
        // O(n²) selection is plenty for ≤ 300 sites and fully deterministic.
        loop {
            let mut best: Option<usize> = None;
            for i in 0..n {
                if !done[i] && dist[i].is_finite() {
                    best = match best {
                        Some(b)
                            if dist[b] < dist[i]
                                || (dist[b] == dist[i] && owner[b] <= owner[i]) =>
                        {
                            Some(b)
                        }
                        _ => Some(i),
                    };
                }
            }
            let Some(u) = best else { break };
            done[u] = true;
            for &(v, w) in &self.adj[u] {
                if done[v] {
                    continue;
                }
                let d = dist[u] + w;
                if d < dist[v] || (d == dist[v] && owner[u] < owner[v]) {
                    dist[v] = d;
                    owner[v] = owner[u];
                }
            }
        }
        (dist, owner)
    }
}
