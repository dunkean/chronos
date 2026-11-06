//! Validation temporelle et des dépendances, sans génération implicite.
use super::model::*;
use std::collections::{BTreeMap, BTreeSet};

/// Rangs intraphase construits par Kahn ; les IDs ne départagent que les
/// nœuds disponibles. Une dépendance absente ou future est refusée.
pub fn causal_ranks(events: &[HistoryEvent]) -> Result<BTreeMap<String, u32>, HistoryError> {
    let mut by_id = BTreeMap::new();
    for e in events {
        e.validate()?;
        if !super::atlas::valid_id(&e.id)
            || !(-MAX_ABS_YEAR..=MAX_ABS_YEAR).contains(&e.year)
            || e.end_year
                .is_some_and(|end| end <= e.year || end > MAX_ABS_YEAR)
            || by_id.insert(e.id.as_str(), e).is_some()
        {
            return Err(event_error(
                e,
                ErrorCode::InvalidCausalOrder,
                "événement dupliqué ou dates invalides",
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
        owner: Some(e.owner_scope.to_string()),
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
            ErrorCode::InvalidCanonicalValue,
            "identité de relation invalide ou dupliquée",
        ));
    }
    for r in relations {
        r.valid.validate(false)?;
        r.validate()?;
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

/// Événements capables de changer un statut légal (§27.3).
pub const LEGAL_CHANGE_EVENT_KINDS: [&str; 4] = ["edict", "charter", "heresy_trial", "banishment"];

/// Life bounds of a group: `foundation` may open the first standing of a chain and
/// `dissolution` close the last one, each at its exact date and for a participating
/// subject. Neither is a status change.
pub const SUBJECT_FOUNDATION_EVENT_KIND: &str = "foundation";
pub const SUBJECT_DISSOLUTION_EVENT_KIND: &str = "dissolution";

/// Statut d'un sujet dans une juridiction à une date. L'absence de
/// `LegalStanding` vaut `licit` ; une autre juridiction n'a aucun effet (§27.3).
pub fn legal_status_at(
    standings: &[LegalStanding],
    subject: &ObjectRef,
    jurisdiction: &ObjectRef,
    year: i32,
) -> LegalStatus {
    standings
        .iter()
        .find(|s| {
            &s.subject == subject && &s.jurisdiction == jurisdiction && s.valid.contains(year)
        })
        .map_or(LegalStatus::Licit, |s| s.status)
}

/// Une chaîne par couple sujet/juridiction (§27.3). Seul un segment commençant
/// à `observed_start` peut ne citer aucune cause : tout autre début est un
/// changement, y compris un premier segment tardif. Un segment qui finit avant
/// `world_end` sans successeur immédiat cite son événement de fin. Les causes
/// sont de type `edict`, `charter`, `heresy_trial` ou `banishment`, sans être
/// postérieures à la date qu'elles expliquent.
///
/// Only exceptions: the subject's foundation for its first standing and its
/// dissolution for its last one.
pub fn validate_legal_standings(
    standings: &[LegalStanding],
    events: &[HistoryEvent],
    observed_start: i32,
    world_end: i32,
) -> Result<(), HistoryError> {
    let fail = |s: &LegalStanding, message: &str| HistoryError {
        owner: Some(s.id.clone()),
        at_year: Some(s.valid.start_year),
        ..HistoryError::new(ErrorCode::InvalidLegalStanding, message)
    };
    if !super::atlas::unique_ids(standings.iter().map(|s| s.id.as_str())) {
        return Err(HistoryError::new(
            ErrorCode::InvalidLegalStanding,
            "identité de statut légal invalide ou dupliquée",
        ));
    }
    let events: BTreeMap<_, _> = events.iter().map(|e| (e.id.as_str(), e)).collect();
    // `life_kind`: the only subject life event admitted at this place in the chain.
    let check_cause = |s: &LegalStanding, id: &str, date: i32, life_kind: Option<&str>| {
        let event = events
            .get(id)
            .ok_or_else(|| fail(s, "événement de changement absent"))?;
        let subject_life = life_kind == Some(event.kind.as_str())
            && event.year == date
            && event.participants.contains(&s.subject);
        if !subject_life
            && (!LEGAL_CHANGE_EVENT_KINDS.contains(&event.kind.as_str()) || event.year > date)
        {
            return Err(fail(s, "événement d'un autre type ou postérieur"));
        }
        Ok(())
    };
    let mut chains: BTreeMap<_, Vec<&LegalStanding>> = BTreeMap::new();
    for s in standings {
        s.validate()?;
        s.valid.validate(false).map_err(|e| HistoryError {
            owner: Some(s.id.clone()),
            at_year: Some(s.valid.start_year),
            ..e
        })?;
        if !matches!(
            s.subject,
            ObjectRef::Entity(_) | ObjectRef::Activity(_) | ObjectRef::PersonSlot(_)
        ) || !matches!(s.jurisdiction, ObjectRef::Entity(_))
        {
            return Err(fail(s, "sujet ou juridiction d'un type interdit"));
        }
        if s.valid.start_year < observed_start || s.valid.end_year > world_end {
            return Err(fail(s, "statut hors de la période observée"));
        }
        chains
            .entry((&s.subject, &s.jurisdiction))
            .or_default()
            .push(s);
    }
    for chain in chains.values_mut() {
        chain.sort_by_key(|s| (s.valid.start_year, s.valid.end_year));
        for (i, s) in chain.iter().enumerate() {
            let next_start = chain.get(i + 1).map(|next| next.valid.start_year);
            if next_start.is_some_and(|start| start < s.valid.end_year) {
                return Err(fail(s, "statuts légaux chevauchants"));
            }
            match &s.cause_event_id {
                Some(id) => {
                    let life = (i == 0).then_some(SUBJECT_FOUNDATION_EVENT_KIND);
                    check_cause(s, id, s.valid.start_year, life)?
                }
                None if s.valid.start_year != observed_start => {
                    return Err(fail(s, "changement de statut sans événement"));
                }
                None => {}
            }
            match &s.end_cause_event_id {
                Some(id) => {
                    let life = next_start
                        .is_none()
                        .then_some(SUBJECT_DISSOLUTION_EVENT_KIND);
                    check_cause(s, id, s.valid.end_year, life)?
                }
                None if s.valid.end_year != next_start.unwrap_or(world_end) => {
                    return Err(fail(s, "fin de statut sans événement"));
                }
                None => {}
            }
        }
    }
    Ok(())
}
