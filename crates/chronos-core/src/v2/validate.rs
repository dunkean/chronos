//! Validation temporelle et des dépendances, sans génération implicite.
use super::model::*;
use std::collections::{BTreeMap, BTreeSet};

/// Rangs intraphase construits par Kahn ; les IDs ne départagent que les
/// nœuds disponibles. Une dépendance absente ou future est refusée.
pub fn causal_ranks(events: &[HistoryEvent]) -> Result<BTreeMap<String, u32>, HistoryError> {
    let mut by_id = BTreeMap::new();
    for e in events {
        if !super::atlas::valid_id(&e.id)
            || !(-100_000..=100_000).contains(&e.year)
            || by_id.insert(e.id.as_str(), e).is_some()
        {
            return Err(HistoryError::new(
                ErrorCode::InvalidCausalOrder,
                "événement dupliqué ou invalide",
            ));
        }
    }
    // Détecter aussi un cycle traversant plusieurs dates/phases avant de
    // diagnostiquer une arête future : tout cycle porte le même code public.
    let mut global_degree = BTreeMap::new();
    let mut global_children: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for e in events {
        let mut causes = BTreeSet::new();
        for cause in &e.causes {
            if !by_id.contains_key(cause.as_str()) || !causes.insert(cause.as_str()) {
                return Err(event_error(
                    e,
                    ErrorCode::InvalidCausalOrder,
                    "cause absente ou dupliquée",
                ));
            }
            global_children.entry(cause).or_default().insert(&e.id);
        }
        global_degree.insert(e.id.as_str(), causes.len());
    }
    let mut ready: BTreeSet<_> = global_degree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut visited = 0;
    while let Some(id) = ready.pop_first() {
        visited += 1;
        for child in global_children.get(id).into_iter().flatten() {
            let degree = global_degree.get_mut(child).expect("événement référencé");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(*child);
            }
        }
    }
    if visited != events.len() {
        let id = global_degree
            .iter()
            .find(|(_, d)| **d > 0)
            .expect("cycle non vide")
            .0;
        return Err(event_error(
            by_id[id],
            ErrorCode::CanonicalDependencyCycle,
            "cycle causal",
        ));
    }
    let mut groups: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for e in events {
        groups.entry((e.year, e.phase)).or_default().push(e);
    }
    let mut ranks = BTreeMap::new();
    for ((year, phase), group) in groups {
        let mut indegree = BTreeMap::new();
        let mut children: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for e in &group {
            let mut causes = BTreeSet::new();
            let mut degree = 0;
            for cause in &e.causes {
                if !causes.insert(cause) {
                    return Err(event_error(
                        e,
                        ErrorCode::InvalidCausalOrder,
                        "cause dupliquée",
                    ));
                }
                let c = by_id.get(cause.as_str()).ok_or_else(|| {
                    event_error(e, ErrorCode::InvalidCausalOrder, "cause absente")
                })?;
                if (c.year, c.phase) > (year, phase) {
                    return Err(event_error(
                        e,
                        ErrorCode::InvalidCausalOrder,
                        "cause future",
                    ));
                }
                if (c.year, c.phase) == (year, phase) {
                    degree += 1;
                    children
                        .entry(c.id.as_str())
                        .or_default()
                        .insert(e.id.as_str());
                }
            }
            indegree.insert(e.id.as_str(), degree);
        }
        let mut ready: BTreeSet<_> = indegree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut rank = 0;
        while let Some(id) = ready.pop_first() {
            ranks.insert(id.to_owned(), rank);
            rank += 1;
            for child in children.get(id).into_iter().flatten() {
                let degree = indegree.get_mut(child).expect("nœud du groupe");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(*child);
                }
            }
        }
        if rank as usize != group.len() {
            return Err(event_error(
                group[0],
                ErrorCode::CanonicalDependencyCycle,
                "cycle causal intraphase",
            ));
        }
    }
    Ok(ranks)
}
fn event_error(e: &HistoryEvent, code: ErrorCode, message: &str) -> HistoryError {
    HistoryError {
        owner: Some(e.owner_scope.clone()),
        at_year: Some(e.year),
        ..HistoryError::new(code, message)
    }
}

/// Composition stricte sur l'ensemble des épisodes ; les appartenances et
/// affiliations n'ajoutent aucune arête de composition.
pub fn validate_composition(relations: &[SocialRelation]) -> Result<(), HistoryError> {
    let mut edges: BTreeMap<&ObjectRef, BTreeSet<&ObjectRef>> = BTreeMap::new();
    let mut degree = BTreeMap::new();
    if !super::atlas::unique_ids(relations.iter().map(|r| r.id.as_str())) {
        return Err(HistoryError::new(
            ErrorCode::CompositionCycle,
            "identité de relation invalide",
        ));
    }
    for r in relations {
        r.valid.validate(false)?;
        if r.weight_bps.is_some_and(|v| v > 10_000) || r.time_cost_bps.is_some_and(|v| v > 10_000) {
            return Err(HistoryError::new(
                ErrorCode::InvalidCanonicalValue,
                "poids de relation hors bps",
            ));
        }
        if r.predicate != Predicate::ComponentOf {
            continue;
        }
        degree.entry(&r.subject).or_insert(0usize);
        degree.entry(&r.object).or_insert(0usize);
        if edges.entry(&r.subject).or_default().insert(&r.object) {
            *degree.get_mut(&r.object).expect("extrémité") += 1;
        }
    }
    let mut ready: BTreeSet<_> = degree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut visited = 0;
    while let Some(id) = ready.pop_first() {
        visited += 1;
        for child in edges.get(id).into_iter().flatten() {
            let d = degree.get_mut(child).expect("extrémité");
            *d -= 1;
            if *d == 0 {
                ready.insert(*child);
            }
        }
    }
    if visited != degree.len() {
        return Err(HistoryError::new(
            ErrorCode::CompositionCycle,
            "cycle de composition stricte",
        ));
    }
    Ok(())
}
