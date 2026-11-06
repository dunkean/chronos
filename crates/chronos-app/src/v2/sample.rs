//! `chronos-app v2 sample` (§15.4): the same world over a list of seeds, exported as
//! aggregates only (counts per class, vacancies, splits, per-year balances) with the
//! versions that produced them. Diagnostics, never a test of invariants.
use super::lineage_worker::WorkerSettings;
use super::orchestrator::{Orchestrator, WorldInputs};
use super::store::Store;
use chronos_core::v2::social::summarize;
use chronos_core::v2::ALGORITHM_VERSION;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `1..100`, `7`, or `1,4,9`. Bounds are inclusive; at most 1,000 seeds.
pub fn parse_seed_range(text: &str) -> Result<Vec<u64>, String> {
    let number = |s: &str| {
        s.trim()
            .parse::<u64>()
            .map_err(|_| format!("invalid seed {s:?}"))
    };
    let seeds: Vec<u64> = if let Some((a, b)) = text.split_once("..") {
        let (a, b) = (number(a)?, number(b)?);
        if a > b {
            return Err(format!("empty seed range {text}"));
        }
        if b - a >= 1000 {
            return Err("at most 1000 seeds".into());
        }
        (a..=b).collect()
    } else {
        text.split(',').map(number).collect::<Result<_, _>>()?
    };
    if seeds.is_empty() || seeds.len() > 1000 {
        return Err("between 1 and 1000 seeds".into());
    }
    Ok(seeds)
}

pub struct SampleOptions {
    pub world: PathBuf,
    pub atlas: PathBuf,
    pub catalogue: PathBuf,
    pub seeds: Vec<u64>,
    pub years: Vec<i32>,
    /// Domain indexes; empty means every domain.
    pub domains: Vec<usize>,
    pub out: PathBuf,
    pub settings: WorkerSettings,
}

fn add(into: &mut BTreeMap<String, u64>, key: String, value: u64) {
    *into.entry(key).or_default() += value;
}
fn distribution(values: &[u64]) -> Value {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    let sum: u64 = sorted.iter().sum();
    json!({
        "n": n,
        "min": sorted[0],
        "median": sorted[n / 2],
        "max": sorted[n - 1],
        "mean": sum as f64 / n as f64,
    })
}

/// Runs every seed and writes `seed-<n>.json` and `summary.json` under `options.out`.
pub fn run_sample(options: &SampleOptions) -> Result<Value, String> {
    std::fs::create_dir_all(&options.out).map_err(|e| format!("cannot create out: {e}"))?;
    let mut per_seed: Vec<(u64, BTreeMap<String, u64>)> = Vec::new();
    let mut versions = Value::Null;
    for seed in &options.seeds {
        let mut inputs = WorldInputs::load(&options.world, &options.atlas, &options.catalogue)
            .map_err(|e| e.to_string())?;
        inputs.config.world_seed = *seed;
        let store = Store::new(options.out.join(".store"));
        let orchestrator = Orchestrator::open(inputs, options.settings.clone(), store)
            .map_err(|e| e.to_string())?;
        let all = orchestrator.domain_ids();
        let picked: Vec<String> = if options.domains.is_empty() {
            all.clone()
        } else {
            options
                .domains
                .iter()
                .map(|i| all.get(*i).cloned().ok_or_else(|| format!("no domain {i}")))
                .collect::<Result<_, _>>()?
        };
        let mut metrics: BTreeMap<String, u64> = BTreeMap::new();
        let mut domain_reports = Vec::new();
        for domain in &picked {
            let population = orchestrator.population(domain).map_err(|e| e.to_string())?;
            let packet = orchestrator
                .social_packet(domain)
                .map_err(|e| e.to_string())?;
            let summary =
                summarize(&packet, &population, &options.years, &[]).map_err(|e| e.to_string())?;
            add(&mut metrics, "slots".into(), summary.slots);
            add(&mut metrics, "offices".into(), summary.offices);
            add(&mut metrics, "tenures".into(), summary.tenures);
            add(
                &mut metrics,
                "vacant_office_years".into(),
                summary.vacant_office_years,
            );
            add(&mut metrics, "splits".into(), summary.splits);
            add(&mut metrics, "preemptions".into(), summary.preemptions);
            add(&mut metrics, "links".into(), summary.links);
            add(&mut metrics, "conflicts".into(), summary.conflicts);
            for (class, n) in &summary.group_slots_by_class {
                add(&mut metrics, format!("group_slots.{class}"), *n);
            }
            for (class, by) in &summary.group_slots_by_conduct {
                for (conduct, n) in by {
                    let name = serde_json::to_value(conduct)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default();
                    add(&mut metrics, format!("group_conduct.{class}.{name}"), *n);
                }
            }
            for year in &summary.years {
                let y = year.year;
                add(
                    &mut metrics,
                    format!("year_{y}.population"),
                    year.population,
                );
                add(
                    &mut metrics,
                    format!("year_{y}.dormant_groups"),
                    year.dormant_groups,
                );
                add(
                    &mut metrics,
                    format!("year_{y}.vacant_offices"),
                    year.vacant_offices,
                );
                add(
                    &mut metrics,
                    format!("year_{y}.memberships"),
                    year.memberships,
                );
                add(
                    &mut metrics,
                    format!("year_{y}.persons_with_affiliation"),
                    year.persons_with_affiliation,
                );
                add(
                    &mut metrics,
                    format!("year_{y}.households"),
                    year.households,
                );
                for (conduct, n) in &year.persons_by_conduct {
                    let name = serde_json::to_value(conduct)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default();
                    add(&mut metrics, format!("year_{y}.persons.{name}"), *n);
                }
            }
            domain_reports.push(json!({
                "domain_id": domain,
                "balance_closed": summary.balance_closed,
                "tenure_closures": summary.tenure_closures,
                "years": summary.years,
            }));
        }
        versions = json!({
            "algorithm_version": ALGORITHM_VERSION,
            "protocol": super::lineage_worker::PROTOCOL,
            "runtime_fingerprint": orchestrator.runtime_fingerprint(),
            "catalogue_version": orchestrator.canon().catalogue.catalogue_version,
        });
        let report = json!({
            "seed": seed,
            "canon_id": orchestrator.canon().canon_id,
            "versions": versions,
            "metrics": metrics,
            "domains": domain_reports,
        });
        write_json(&options.out.join(format!("seed-{seed}.json")), &report)?;
        per_seed.push((*seed, metrics));
    }
    let names: std::collections::BTreeSet<&String> =
        per_seed.iter().flat_map(|(_, m)| m.keys()).collect();
    let mut distributions = Map::new();
    for name in names {
        let values: Vec<u64> = per_seed
            .iter()
            .map(|(_, m)| m.get(name).copied().unwrap_or(0))
            .collect();
        distributions.insert(name.clone(), distribution(&values));
    }
    let summary = json!({
        "format": "chronos-sample-1",
        "seeds": options.seeds,
        "years": options.years,
        "versions": versions,
        "distributions": distributions,
    });
    write_json(&options.out.join("summary.json"), &summary)?;
    Ok(summary)
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::parse_seed_range;

    #[test]
    fn seed_ranges_are_inclusive_and_bounded() {
        assert_eq!(parse_seed_range("1..3").unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_seed_range("7").unwrap(), vec![7]);
        assert_eq!(parse_seed_range("2,5").unwrap(), vec![2, 5]);
        assert!(parse_seed_range("3..1").is_err());
        assert!(parse_seed_range("1..5000").is_err());
        assert!(parse_seed_range("x").is_err());
    }
}
