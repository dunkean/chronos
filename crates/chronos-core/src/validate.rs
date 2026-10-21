//! Structural validation of an exported world (used on import and in tests).

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{World, SCHEMA_VERSION};

/// Returns non-blocking diagnostics, or an error describing the first
/// blocking inconsistency.
pub fn validate_world(w: &World) -> Result<Vec<String>, String> {
    let mut notes = Vec::new();
    if w.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "schema_version {} non pris en charge (attendu {SCHEMA_VERSION})",
            w.schema_version
        ));
    }
    w.config
        .validate()
        .map_err(|e| format!("configuration invalide : {e}"))?;
    let expected = crate::canon_id(&w.config, &w.sources);
    if w.canon_id != expected {
        return Err(format!(
            "empreinte incompatible : canon_id {} ≠ {expected} (configuration, sources ou version d'algorithme modifiées)",
            w.canon_id
        ));
    }
    let m = &w.map;
    if m.width == 0 || m.height == 0 || m.elevation_m.len() != m.width * m.height {
        return Err("carte : elevation_m doit contenir width × height valeurs".into());
    }

    fn unique<'a>(
        what: &str,
        ids: impl Iterator<Item = &'a str>,
    ) -> Result<BTreeSet<&'a str>, String> {
        let mut set = BTreeSet::new();
        for id in ids {
            if !set.insert(id) {
                return Err(format!("{what} : ID dupliqué {id}"));
            }
        }
        Ok(set)
    }
    let sites = unique("sites", w.sites.iter().map(|s| s.id.as_str()))?;
    let polities = unique("polities", w.polities.iter().map(|p| p.id.as_str()))?;
    let orgs = unique(
        "organizations",
        w.organizations.iter().map(|o| o.id.as_str()),
    )?;
    let people = unique("people", w.people.iter().map(|p| p.id.as_str()))?;
    unique("routes", w.routes.iter().map(|r| r.id.as_str()))?;
    unique("events", w.events.iter().map(|e| e.id.as_str()))?;

    for s in &w.sites {
        if !polities.contains(s.polity_id.as_str()) {
            return Err(format!("site {} : polity inconnue {}", s.id, s.polity_id));
        }
    }
    for r in &w.routes {
        if !sites.contains(r.from.as_str()) || !sites.contains(r.to.as_str()) {
            return Err(format!("route {} : extrémité inconnue", r.id));
        }
        if !(r.travel_days.is_finite() && r.travel_days >= 1.0 && r.travel_days.fract() == 0.0) {
            return Err(format!(
                "route {} : travel_days doit être un nombre entier de jours ≥ 1",
                r.id
            ));
        }
    }
    for p in &w.polities {
        if !sites.contains(p.capital_id.as_str()) {
            return Err(format!("polity {} : capitale inconnue", p.id));
        }
    }
    for o in &w.organizations {
        if !polities.contains(o.polity_id.as_str())
            || o.site_ids.iter().any(|s| !sites.contains(s.as_str()))
        {
            return Err(format!("organisation {} : polity ou site inconnu", o.id));
        }
        if o.collective_office && o.holder_id.is_some() {
            return Err(format!(
                "organisation {} : un office collectif n'a pas de titulaire personnel",
                o.id
            ));
        }
        if o.holder_id
            .as_ref()
            .is_some_and(|h| !people.contains(h.as_str()))
        {
            return Err(format!("organisation {} : titulaire inconnu", o.id));
        }
    }
    let mut external_parents = 0;
    for p in &w.people {
        if !sites.contains(p.site_id.as_str()) {
            return Err(format!("personne {} : site inconnu", p.id));
        }
        if p.death_year.is_some_and(|d| d < p.birth_year) {
            return Err(format!("personne {} : décès avant naissance", p.id));
        }
        external_parents += [&p.father_id, &p.mother_id]
            .into_iter()
            .flatten()
            .filter(|id| !people.contains(id.as_str()))
            .count();
    }
    if external_parents > 0 {
        notes.push(format!(
            "{external_parents} référence(s) parentale(s) externe(s) absente(s) du tableau people."
        ));
    }

    let (start, end) = (w.config.start_year, w.config.end_year());
    let person: BTreeMap<&str, _> = w.people.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut seen_events = BTreeSet::new();
    for e in &w.events {
        if e.year < start || e.end_year < e.year || e.end_year >= end {
            return Err(format!(
                "événement {} : années {}–{} hors de la fenêtre {start}–{}",
                e.id,
                e.year,
                e.end_year,
                end - 1
            ));
        }
        for c in &e.causes {
            if !seen_events.contains(c.as_str()) {
                return Err(format!(
                    "événement {} : la cause {c} n'est pas un événement antérieur",
                    e.id
                ));
            }
        }
        if e.site_ids
            .iter()
            .chain(e.effects.iter().map(|f| &f.site_id))
            .any(|s| !sites.contains(s.as_str()))
        {
            return Err(format!("événement {} : site inconnu", e.id));
        }
        if e.organization_ids
            .iter()
            .any(|o| !orgs.contains(o.as_str()))
        {
            return Err(format!("événement {} : organisation inconnue", e.id));
        }
        for pid in &e.person_ids {
            let Some(p) = person.get(pid.as_str()) else {
                return Err(format!("événement {} : personne inconnue {pid}", e.id));
            };
            if p.birth_year > e.year || p.death_year.is_some_and(|d| d < e.end_year) {
                return Err(format!(
                    "événement {} : {} n'est pas en vie de {} à {}",
                    e.id, p.name, e.year, e.end_year
                ));
            }
        }
        if e.end_year != e.year && !e.effects.is_empty() {
            return Err(format!(
                "événement {} : un événement pluriannuel ne porte pas d'effets directs",
                e.id
            ));
        }
        seen_events.insert(e.id.as_str());
    }

    if w.snapshots.len() != w.config.years as usize + 1 {
        return Err(format!(
            "{} instantanés pour {} ans (attendu années + 1)",
            w.snapshots.len(),
            w.config.years
        ));
    }
    for (k, snap) in w.snapshots.iter().enumerate() {
        if snap.year != start + k as i32 || snap.sites.len() != w.sites.len() {
            return Err(format!(
                "instantané {} incohérent (année ou nombre de sites)",
                snap.year
            ));
        }
        if snap
            .sites
            .iter()
            .zip(&w.sites)
            .any(|(st, s)| st.site_id != s.id || !polities.contains(st.polity_id.as_str()))
        {
            return Err(format!(
                "instantané {} : sites dans un ordre différent ou polity inconnue",
                snap.year
            ));
        }
    }

    // Budgets: losses at a site in a year never exceed the people present
    // (start-of-year population plus arrivals recorded that year).
    let mut ledger: BTreeMap<(i32, &str), (i64, i64)> = BTreeMap::new();
    for e in &w.events {
        for f in &e.effects {
            let entry = ledger.entry((e.year, f.site_id.as_str())).or_default();
            if f.population_delta < 0 {
                entry.0 += -f.population_delta;
            } else {
                entry.1 += f.population_delta;
            }
        }
    }
    let index: BTreeMap<&str, usize> = w
        .sites
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();
    for ((year, site), (lost, gained)) in ledger {
        let pop = w.snapshots[(year - start) as usize].sites[index[site]].population as i64;
        if lost > pop + gained {
            return Err(format!(
                "{site} en {year} : {lost} pertes pour {pop} habitants (+{gained} arrivées)"
            ));
        }
    }
    Ok(notes)
}
