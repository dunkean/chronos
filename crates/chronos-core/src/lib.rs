//! Chronos core: deterministic small-world chronology generator.
//!
//! No I/O, no clock, no shared RNG: `generate_world` is a pure function of the
//! configuration and the (optional) bootstrap fixture, so it also compiles for
//! `wasm32-unknown-unknown`.

pub mod generate;
pub mod keys;
pub mod model;
pub mod names;
pub mod social;
pub mod validate;
pub mod world;

use std::collections::{BTreeMap, BTreeSet};

pub use model::*;
pub use validate::validate_world;

use keys::{fingerprint, fnv1a};
use social::{Method, ProfileKind};

/// Builds a complete world. `fixture` is the parsed `fixtures/stack.json`, or
/// the reason it is unavailable; it is only used when `map_mode == "stack"`.
pub fn generate_world(cfg: &Config, fixture: Result<&StackFixture, &str>) -> Result<World, String> {
    cfg.validate()?;
    let mut warnings = Vec::new();
    let mut sources = Vec::new();
    let mut stack = None;
    let mut imported_geo = None;
    if cfg.map_mode == "stack" {
        let attempt = match fixture {
            Ok(fx) if fx.schema_version != SCHEMA_VERSION => Err(format!(
                "schema_version {} non pris en charge (attendu {SCHEMA_VERSION})",
                fx.schema_version
            )),
            Ok(fx) => world::stack_geography(cfg, fx).map(|g| (fx, g)),
            Err(e) => Err(e.to_string()),
        };
        match attempt {
            Ok((fx, g)) => {
                stack = Some(fx);
                imported_geo = Some(g);
            }
            Err(problem) => {
                warnings.push(format!(
                    "Carte importée indisponible ({problem}) : repli sur le monde virtuel synthétique ; aucune donnée n'est présentée comme importée."
                ));
                sources.push(Source {
                    id: "stack-fixture".into(),
                    label: "Fixture d'amorçage fixtures/stack.json".into(),
                    status: "unavailable".into(),
                    detail: problem,
                    fingerprint: String::new(),
                });
            }
        }
    }
    let mut geo = imported_geo.unwrap_or_else(|| world::virtual_geography(cfg));

    if let Some(fx) = stack {
        let bytes = serde_json::to_vec(fx).map_err(|e| e.to_string())?;
        sources.push(Source {
            id: "stack-fixture".into(),
            label: "Fixture d'amorçage fixtures/stack.json".into(),
            status: "imported".into(),
            detail: format!(
                "schema {} : {} sites, {} routes, {} personnes, {} modèles d'institutions ; provenance détaillée dans les sources suivantes.",
                fx.schema_version,
                fx.sites.len(),
                fx.routes.len(),
                fx.people.len(),
                fx.blueprints.len()
            ),
            fingerprint: fingerprint(&bytes),
        });
        warnings.extend(fx.warnings.iter().map(|w| format!("Fixture : {w}")));
        let mut seen: BTreeSet<String> = sources.iter().map(|s| s.id.clone()).collect();
        for s in &fx.sources {
            if seen.insert(s.id.clone()) {
                sources.push(s.clone());
            }
        }
    }
    sources.append(&mut geo.sources);
    warnings.append(&mut geo.warnings);

    // The applied configuration records what was really used.
    let mut applied = cfg.clone();
    if stack.is_some() {
        applied.site_count = geo.sites.len();
    }
    let cfg = &applied;

    // Names: fixture pools (Namebench) when present, built-in synthetic lists
    // otherwise. Each site uses the pool of its own culture when it names one.
    let mut used_cultures: BTreeSet<&str> = BTreeSet::from([cfg.culture.as_str()]);
    used_cultures.extend(
        geo.sites
            .iter()
            .map(|s| names::resolve_culture(&s.culture_id, &cfg.culture)),
    );
    let mut pools = names::Pools {
        by_culture: Default::default(),
        default: cfg.culture.clone(),
    };
    let mut builtin_used = Vec::new();
    for culture in used_cultures {
        let builtin = names::builtin_pool(culture, cfg.seed);
        let mut pool = stack
            .map(|fx| fx.names.get(culture).clone())
            .unwrap_or_default();
        let mut missing = Vec::new();
        for (list, fallback, label) in [
            (&mut pool.forenames, builtin.forenames, "prénoms"),
            (&mut pool.surnames, builtin.surnames, "noms de famille"),
            (&mut pool.places, builtin.places, "toponymes"),
        ] {
            if list.is_empty() {
                *list = fallback;
                missing.push(label);
            }
        }
        if !missing.is_empty() {
            builtin_used.push(format!("{culture} ({})", missing.join(", ")));
        }
        pools.by_culture.insert(culture.to_string(), pool);
    }
    if !builtin_used.is_empty() {
        if stack.is_some() {
            warnings.push(format!(
                "Pools de noms incomplets dans la fixture : listes intégrées pour {}.",
                builtin_used.join(" ; ")
            ));
        }
        sources.push(Source {
            id: "chronos-builtin-names".into(),
            label: "Listes de noms intégrées".into(),
            status: "synthetic".into(),
            detail: format!(
                "Listes courtes écrites pour Chronos, pas un export Namebench : {}.",
                builtin_used.join(" ; ")
            ),
            fingerprint: fingerprint(
                format!("builtin-names-v1/{}", builtin_used.join(";")).as_bytes(),
            ),
        });
    }
    if stack.is_some()
        && geo
            .sites
            .iter()
            .any(|s| names::resolve_culture(&s.culture_id, &cfg.culture) != cfg.culture)
    {
        warnings.push(format!(
            "Noms : les sites importés portent leur propre culture (pool correspondant) ; culture = {} ne sert qu'aux sites sans culture connue.",
            cfg.culture
        ));
    }
    let mut by_culture: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for s in geo.sites.iter().filter(|s| s.name.is_empty()) {
        by_culture
            .entry(names::resolve_culture(&s.culture_id, &cfg.culture).to_string())
            .or_default()
            .push(s.id.clone());
    }
    let mut assigned: BTreeMap<String, String> = BTreeMap::new();
    for (culture, ids) in by_culture {
        let generated = names::place_names(pools.get(&culture), &culture, cfg.seed, &ids);
        assigned.extend(ids.into_iter().zip(generated));
    }
    for site in geo.sites.iter_mut().filter(|s| s.name.is_empty()) {
        site.name = assigned.remove(&site.id).unwrap_or_default();
    }

    // Institution profiles (Pecking Order blueprints when present).
    let blueprints = stack.map(|fx| fx.blueprints.as_slice()).unwrap_or(&[]);
    let profiles = social::select_profiles(blueprints, &cfg.culture);
    let builtins: Vec<&str> = profiles
        .values()
        .filter(|p| p.builtin)
        .map(|p| p.kind.org_kind())
        .collect();
    if !builtins.is_empty() {
        if stack.is_some() {
            warnings.push(format!("Profils institutionnels absents de la fixture, remplacés par des profils génériques Chronos : {}.", builtins.join(", ")));
        }
        sources.push(Source {
            id: "chronos-builtin-blueprints".into(),
            label: "Profils institutionnels génériques".into(),
            status: "synthetic".into(),
            detail: format!(
                "Conçus pour Chronos, pas issus de Pecking Order : {}.",
                builtins.join(", ")
            ),
            fingerprint: fingerprint(builtins.join(",").as_bytes()),
        });
    }

    // People (Lineage Lab sample when present).
    let records = stack.map(|fx| fx.people.as_slice()).unwrap_or(&[]);
    if stack.is_some() && records.is_empty() {
        warnings.push(
            "Aucune personne Lineage Lab dans la fixture : titulaires et héros virtuels (repli)."
                .into(),
        );
    }
    let coverage_end = stack.and_then(|fx| fx.genealogy_end());
    let coverage_start = stack.and_then(|fx| fx.genealogy_start());
    let mut import = social::import_people(
        cfg,
        &pools,
        records,
        &geo.sites,
        (coverage_start, coverage_end),
    );
    warnings.append(&mut import.warnings);
    let imported_count = import.people.list.len();

    let graph = world::Graph::new(&geo.sites, &geo.routes);
    let mut society = social::build_society(cfg, &geo.sites, &graph, &profiles);
    for (i, site) in geo.sites.iter_mut().enumerate() {
        site.polity_id = society.polities[society.site_polity[i]].id.clone();
    }
    let methods: Vec<Method> = society
        .org_profile
        .iter()
        .map(|k: &ProfileKind| profiles[k].method)
        .collect();
    let (history, people) = generate::run_history(
        cfg,
        &geo.sites,
        &graph,
        &mut society,
        methods,
        import.people,
    );
    warnings.extend(history.warnings);

    if imported_count > 0 {
        sources.push(Source {
            id: "chronos-lineage-mapping".into(),
            label: "Résidence des personnes importées".into(),
            status: "derived".into(),
            detail: format!(
                "Indice d'établissement Lineage Lab → site de même rang dans la fixture (adaptation déterministe, pas une correspondance géographique d'origine). Dates de vie et filiations inchangées ; noms attribués par clé stable sur l'ID externe. Couverture de l'archive : {}–{} ({}) ; sans décès enregistré, une personne n'est plus suivie après cette fin.",
                stack.and_then(|fx| fx.genealogy_start()).map_or("?".to_string(), |y| y.to_string()),
                people.import_end.map_or("?".to_string(), |y| y.to_string()),
                if coverage_end.is_some() { "fournie par la fixture" } else { "estimée au dernier fait daté" }
            ),
            fingerprint: fingerprint(b"site_index-modulo-v1"),
        });
    }
    let virtual_count = people.list.len() - imported_count;
    if virtual_count > 0 {
        sources.push(Source {
            id: "chronos-virtual-people".into(),
            label: "Personnes virtuelles".into(),
            status: "synthetic".into(),
            detail: format!(
                "{virtual_count} personne(s) créée(s) par Chronos quand aucune personne importée n'était éligible (titulaires, enfants de titulaires virtuels, héros de repli)."
            ),
            fingerprint: fingerprint(ALGORITHM_VERSION.as_bytes()),
        });
    }
    let mut detail = "Boucle annuelle agrégée générée intégralement à la racine (pas de raffinement paresseux). Taux = intensités de worldbuilding, pas des lois historiques. Pertes = floor(P × num / den) sur la population au début de l'année.".to_string();
    for note in &history.notes {
        detail.push(' ');
        detail.push_str(note);
    }
    sources.push(Source {
        id: "chronos-history".into(),
        label: "Chronologie Chronos".into(),
        status: "derived".into(),
        detail,
        fingerprint: fingerprint(ALGORITHM_VERSION.as_bytes()),
    });

    let genealogy_end_year = people.import_end.filter(|_| imported_count > 0);
    let genealogy_start_year = people.import_start.filter(|_| imported_count > 0);
    let mut world = World {
        schema_version: SCHEMA_VERSION,
        canon_id: String::new(),
        config: cfg.clone(),
        generation_ms: 0.0,
        map: geo.map,
        sources,
        warnings,
        sites: geo.sites,
        routes: geo.routes,
        polities: society.polities,
        organizations: society.organizations,
        people: people.list,
        events: history.events,
        snapshots: history.snapshots,
        genealogy_start_year,
        genealogy_end_year,
    };
    world.canon_id = canon_id(&world.config, &world.sources);
    Ok(world)
}

/// Canon identity: algorithm version, configuration and source fingerprints.
pub fn canon_id(cfg: &Config, sources: &[Source]) -> String {
    let mut text = format!(
        "{ALGORITHM_VERSION}\n{}\n",
        serde_json::to_string(cfg).unwrap_or_default()
    );
    for s in sources {
        text.push_str(&format!("{}|{}|{}\n", s.id, s.status, s.fingerprint));
    }
    format!("canon-{:016x}", fnv1a(text.as_bytes()))
}

/// Hash of the canonical content (everything except `generation_ms`).
pub fn replay_hash(world: &World) -> String {
    let mut w = world.clone();
    w.generation_ms = 0.0;
    fingerprint(&serde_json::to_vec(&w).unwrap_or_default())
}
