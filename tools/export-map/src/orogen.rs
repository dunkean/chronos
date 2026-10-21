//! Orogen graph fields on the unit sphere and their projection to rasters.
//!
//! The projection is a CPU port of Neural Earth's maintained atlas rasterization
//! (`infinite_map/backend/terrain_orogen_cuda.py::rasterize`): the 4 nearest graph
//! vertices (as `cKDTree(...).query(k=4)`), their incident triangle fans, spherical
//! barycentric weights `M^-1 q`, the candidate with the largest minimum weight, clamped
//! and normalized; nearest vertex when no triangle contains the query (counted).
//! Elevation is interpolated in Orogen units then converted with the coastal-slope-v1
//! hypsometry (`terrain_orogen._metres`); climate uses `terrain_conditioning.orogen_bioclim`.
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Manifest {
    #[serde(rename = "final")]
    last: Final,
}
#[derive(Deserialize)]
struct Final {
    fields: String,
    layout: Vec<(String, usize)>,
}

/// Continuous fields interpolated by the backend; categorical fields are not used here.
pub const FIELDS: [&str; 6] = [
    "elevation",
    "temperature_summer",
    "temperature_winter",
    "precip_summer",
    "precip_winter",
    "tecActivity",
];

pub struct Graph {
    pub xyz: Vec<[f64; 3]>,
    triangles: Vec<[usize; 3]>,
    /// Incident triangles per vertex, increasing triangle index (stable argsort order).
    fans: Vec<Vec<usize>>,
    /// One array per entry of [`FIELDS`].
    fields: Vec<Vec<f32>>,
    tree: Option<Box<Node>>,
}

/// Physical values at a point.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub elevation_m: f64,
    /// BIO1, annual mean temperature (°C), from the two seasonal proxies.
    pub temperature_c: f64,
    /// BIO12, annual precipitation (mm).
    pub precipitation_mm: f64,
    /// Orogen tectonic activity (0..1), a source diagnostic, not a deposit survey.
    pub tectonic: f64,
    pub fallback: bool,
}

pub fn metres(e: f64) -> f64 {
    if e <= 0.0 {
        e * 10000.0
    } else {
        let t = e.min(1.0);
        6000.0 * t.powi(4) * (5.0 - 4.0 * t) + 1000.0 * t * (1.0 - t).powi(4)
    }
}

impl Graph {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let manifest: Manifest = serde_json::from_slice(
            &std::fs::read(dir.join("manifest.json")).map_err(|e| format!("manifest.json: {e}"))?,
        )
        .map_err(|e| format!("manifest.json: {e}"))?;
        let bytes = std::fs::read(dir.join(&manifest.last.fields)).map_err(|e| e.to_string())?;
        let flat: Vec<f32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect();
        let mut at = 0;
        let mut arrays = std::collections::BTreeMap::new();
        for (name, len) in &manifest.last.layout {
            arrays.insert(
                name.clone(),
                flat.get(at..at + len).ok_or("fields.f32 is truncated")?,
            );
            at += len;
        }
        if at != flat.len() {
            return Err("fields.f32 does not match its layout".into());
        }
        let xyz: Vec<[f64; 3]> = arrays["xyz"]
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| p.map(f64::from))
            .collect();
        let count = xyz.len();
        let triangles: Vec<[usize; 3]> = arrays["triangles"]
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| t.map(|v| v as usize))
            .collect();
        if triangles.iter().flatten().any(|&v| v >= count) {
            return Err("triangle index out of range".into());
        }
        let mut fans = vec![vec![]; count];
        for (i, t) in triangles.iter().enumerate() {
            for &v in t {
                fans[v].push(i);
            }
        }
        let fields = FIELDS
            .iter()
            .map(|name| {
                let values = arrays
                    .get(*name)
                    .ok_or(format!("missing Orogen field {name}"))?;
                if values.len() != count || values.iter().any(|v| !v.is_finite()) {
                    return Err(format!("invalid Orogen field {name}"));
                }
                Ok(values.to_vec())
            })
            .collect::<Result<Vec<_>, String>>()?;
        let tree = Node::build(&mut (0..count).collect::<Vec<_>>(), &xyz, 0);
        Ok(Self {
            xyz,
            triangles,
            fans,
            fields,
            tree,
        })
    }

    pub fn len(&self) -> usize {
        self.xyz.len()
    }

    /// Raw interpolated Orogen values (in [`FIELDS`] order) at a unit vector.
    fn interpolate(&self, q: [f64; 3]) -> ([f64; 6], bool) {
        let mut best = [(f64::INFINITY, 0usize); 4];
        if let Some(tree) = &self.tree {
            tree.nearest(&q, &self.xyz, &mut best);
        }
        let mut chosen: Option<(f64, usize, [f64; 3])> = None;
        for &(_, v) in &best {
            for &t in &self.fans[v] {
                let Some(w) = barycentric(self.triangles[t].map(|i| self.xyz[i]), q) else {
                    continue;
                };
                let score = w[0].min(w[1]).min(w[2]);
                if chosen.is_none_or(|(s, _, _)| score > s) {
                    chosen = Some((score, t, w));
                }
            }
        }
        let mut out = [0.0; 6];
        match chosen {
            Some((score, t, w)) if score >= -1e-4 => {
                let w = w.map(|v| v.max(0.0));
                let total: f64 = w.iter().sum();
                for (f, values) in self.fields.iter().enumerate() {
                    out[f] = (0..3)
                        .map(|k| w[k] / total * values[self.triangles[t][k]] as f64)
                        .sum();
                }
                (out, false)
            }
            _ => {
                for (f, values) in self.fields.iter().enumerate() {
                    out[f] = values[best[0].1] as f64;
                }
                (out, true)
            }
        }
    }

    pub fn sample(&self, lon: f64, lat: f64) -> Sample {
        let q = [lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()];
        let (v, fallback) = self.interpolate(q);
        let (ts, tw) = (v[1] * 90.0 - 45.0, v[2] * 90.0 - 45.0);
        Sample {
            elevation_m: metres(v[0]),
            temperature_c: (ts + tw) * 0.5,
            precipitation_mm: (v[3] + v[4]) * 1000.0,
            tectonic: v[5],
            fallback,
        }
    }
}

/// Coefficients of `q` in the basis of the triangle's vertices (`M^-1 q`, M columns = vertices).
fn barycentric(p: [[f64; 3]; 3], q: [f64; 3]) -> Option<[f64; 3]> {
    let [a, b, c] = p;
    let cross = |u: [f64; 3], v: [f64; 3]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let dot = |u: [f64; 3], v: [f64; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let det = dot(a, cross(b, c));
    if det.abs() < 1e-300 {
        return None;
    }
    // Cramer's rule: rows of the inverse are the cross products of the other columns.
    Some([
        dot(cross(b, c), q) / det,
        dot(cross(c, a), q) / det,
        dot(cross(a, b), q) / det,
    ])
}

/// 3D k-d tree, as in neural-earth/src/atlas.rs.
struct Node {
    index: usize,
    axis: usize,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}
impl Node {
    fn build(ids: &mut [usize], xyz: &[[f64; 3]], depth: usize) -> Option<Box<Self>> {
        if ids.is_empty() {
            return None;
        }
        let axis = depth % 3;
        let mid = ids.len() / 2;
        ids.select_nth_unstable_by(mid, |&a, &b| {
            xyz[a][axis].total_cmp(&xyz[b][axis]).then(a.cmp(&b))
        });
        let (left, rest) = ids.split_at_mut(mid);
        Some(Box::new(Self {
            index: rest[0],
            axis,
            left: Self::build(left, xyz, depth + 1),
            right: Self::build(&mut rest[1..], xyz, depth + 1),
        }))
    }
    fn nearest(&self, q: &[f64; 3], xyz: &[[f64; 3]], best: &mut [(f64, usize); 4]) {
        let p = xyz[self.index];
        let delta = q[self.axis] - p[self.axis];
        let d = (0..3).map(|a| (q[a] - p[a]).powi(2)).sum::<f64>();
        if d < best[3].0 || (d == best[3].0 && self.index < best[3].1) {
            best[3] = (d, self.index);
            best.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        }
        let (near, far) = if delta < 0.0 {
            (&self.left, &self.right)
        } else {
            (&self.right, &self.left)
        };
        if let Some(node) = near {
            node.nearest(q, xyz, best);
        }
        if delta * delta <= best[3].0
            && let Some(node) = far
        {
            node.nearest(q, xyz, best);
        }
    }
}

/// Azimuthal equidistant projection centred on (lon0, lat0), radians; plane in metres,
/// x east, y north. Distances from the centre are exact on the sphere of radius `radius_m`.
#[derive(Clone, Copy, Debug)]
pub struct Aeqd {
    pub lon0: f64,
    pub lat0: f64,
    pub radius_m: f64,
}
impl Aeqd {
    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let rho = x.hypot(y);
        if rho == 0.0 {
            return (self.lon0, self.lat0);
        }
        let c = rho / self.radius_m;
        let (sc, cc) = c.sin_cos();
        let (sl, cl) = self.lat0.sin_cos();
        let lat = (cc * sl + y * sc * cl / rho).clamp(-1.0, 1.0).asin();
        let lon = self.lon0 + (x * sc).atan2(rho * cl * cc - y * sl * sc);
        (wrap(lon), lat)
    }
}

pub fn wrap(lon: f64) -> f64 {
    let pi = std::f64::consts::PI;
    (lon + pi).rem_euclid(2.0 * pi) - pi
}
