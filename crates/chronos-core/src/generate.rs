//! Root chronology: one deterministic annual loop over the whole small world.
//!
//! Each year: snapshot (state at the start of the year), then successions,
//! wars, epidemics, harvests, trade, migration, reconstruction, unrest,
//! discoveries and finally natural growth (not an event). Every population
//! change carried by an event is applied exactly as recorded in its effects;
//! losses never exceed the people present.

use std::collections::{BTreeMap, BTreeSet};

use crate::keys::{key, range_i, unit};
use crate::model::{Config, Effect, Event, Organization, Site, SiteState, Snapshot};
use crate::names::elide;
use crate::social::{is_village, Method, People, ProfileKind, Society};
use crate::world::{round2, Graph};

pub const PIVOT_EPIDEMIC: (i32, i32) = (1354, 1360);
pub const PIVOT_CITY_YEAR: i32 = 1356;
pub const PIVOT_HERO_YEAR: i32 = 1398;
pub const PIVOT_DESTROYED_VILLAGES: usize = 3;

pub struct History {
    pub events: Vec<Event>,
    pub snapshots: Vec<Snapshot>,
    pub warnings: Vec<String>,
    pub notes: Vec<String>,
}

struct SiteSim {
    pop: u64,
    food: f64,
    wealth: f64,
    stability: f64,
    polity: usize,
    cap: f64,
    destroyed: Option<(i32, String)>,
    /// Recent events that hurt this site: (year, event id).
    hurt: Vec<(i32, String)>,
    battle: Option<(i32, String)>,
    mined: bool,
}

struct War {
    id: String,
    attacker: usize,
    defender: usize,
    end: i32,
    pivot: bool,
    /// (year, site, destroy, hero)
    plan: Vec<(i32, usize, bool, bool)>,
}

struct Wave {
    id: String,
    hits: Vec<(i32, usize, u64, u64)>,
}

#[derive(Default)]
struct Ev {
    end: Option<i32>,
    kind: &'static str,
    title: String,
    description: String,
    sites: Vec<usize>,
    orgs: Vec<usize>,
    persons: Vec<usize>,
    causes: Vec<String>,
    effects: Vec<Effect>,
    importance: f64,
}

struct Sim<'a> {
    cfg: &'a Config,
    seed: u64,
    sites: &'a [Site],
    graph: &'a Graph,
    site_index: BTreeMap<String, usize>,
    orgs: Vec<Organization>,
    org_profile: Vec<ProfileKind>,
    org_method: Vec<Method>,
    org_polity: Vec<usize>,
    government: Vec<usize>,
    league: Option<usize>,
    polity_names: Vec<String>,
    polity_ids: Vec<String>,
    capitals: Vec<usize>,
    people: People,
    state: Vec<SiteSim>,
    start_pop: Vec<u64>,
    holders: Vec<Option<usize>>,
    holding: BTreeSet<usize>,
    roles: BTreeMap<usize, String>,
    wars: Vec<War>,
    last_peace: BTreeMap<(usize, usize), i32>,
    waves: Vec<Wave>,
    last_reform: BTreeMap<usize, i32>,
    unrest_ids: Vec<(i32, usize, String)>,
    famine_ids: BTreeMap<usize, String>,
    events: Vec<Event>,
    year_seq: BTreeMap<i32, u32>,
    snapshots: Vec<Snapshot>,
    warnings: Vec<String>,
    notes: Vec<String>,
    pivot_epidemic: bool,
    pivot_war: Option<(i32, i32, i32)>,
}

pub fn run_history(
    cfg: &Config,
    sites: &[Site],
    graph: &Graph,
    society: &mut Society,
    org_method: Vec<Method>,
    people: People,
) -> (History, People) {
    let caps = capacities(cfg, sites);
    let state = sites
        .iter()
        .enumerate()
        .map(|(i, s)| SiteSim {
            pop: s.initial_population,
            food: caps[i] * 0.3,
            wealth: round2(s.initial_population as f64 / 100.0),
            stability: 0.7,
            polity: society.site_polity[i],
            cap: caps[i],
            destroyed: None,
            hurt: Vec::new(),
            battle: None,
            mined: false,
        })
        .collect();
    let polity_index: BTreeMap<&str, usize> = society
        .polities
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.as_str(), i))
        .collect();
    let site_index: BTreeMap<String, usize> = sites
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.clone(), i))
        .collect();
    let mut sim = Sim {
        cfg,
        seed: cfg.seed,
        sites,
        graph,
        org_polity: society
            .organizations
            .iter()
            .map(|o| polity_index[o.polity_id.as_str()])
            .collect(),
        capitals: society
            .polities
            .iter()
            .map(|p| site_index[&p.capital_id])
            .collect(),
        site_index,
        orgs: society.organizations.clone(),
        org_profile: society.org_profile.clone(),
        org_method,
        government: society.government.clone(),
        league: society.league,
        polity_names: society.polities.iter().map(|p| p.name.clone()).collect(),
        polity_ids: society.polities.iter().map(|p| p.id.clone()).collect(),
        people,
        state,
        start_pop: Vec::new(),
        holders: vec![None; society.organizations.len()],
        holding: BTreeSet::new(),
        roles: BTreeMap::new(),
        wars: Vec::new(),
        last_peace: BTreeMap::new(),
        waves: Vec::new(),
        last_reform: BTreeMap::new(),
        unrest_ids: Vec::new(),
        famine_ids: BTreeMap::new(),
        events: Vec::new(),
        year_seq: BTreeMap::new(),
        snapshots: Vec::new(),
        warnings: Vec::new(),
        notes: Vec::new(),
        pivot_epidemic: false,
        pivot_war: None,
    };
    sim.run();
    for (o, h) in society.organizations.iter_mut().zip(&sim.orgs) {
        o.holder_id = h.holder_id.clone();
    }
    for (&i, role) in &sim.roles {
        sim.people.list[i].role = role.clone();
    }
    for i in 0..sim.people.list.len() {
        if sim.people.list[i].role.is_empty() {
            sim.people.list[i].role = if sim.people.imported(i) {
                "résident".into()
            } else {
                "personnage virtuel".into()
            };
        }
    }
    let history = History {
        events: sim.events,
        snapshots: sim.snapshots,
        warnings: sim.warnings,
        notes: sim.notes,
    };
    (history, sim.people)
}

/// Annual food capacity in person-years: initial population scaled by the
/// site's food/water potential rank and by `resource_abundance`.
pub fn capacities(cfg: &Config, sites: &[Site]) -> Vec<f64> {
    let potential: Vec<f64> = sites
        .iter()
        .map(|s| {
            s.resources.food.max(0.0).min(s.resources.water.max(0.0))
                / (s.initial_population.max(1) as f64).sqrt()
        })
        .collect();
    let mut order: Vec<usize> = (0..sites.len()).collect();
    order.sort_by(|&a, &b| potential[a].total_cmp(&potential[b]).then(a.cmp(&b)));
    let mut q = vec![0.0; sites.len()];
    let denom = (sites.len().max(2) - 1) as f64;
    for (rank, &i) in order.iter().enumerate() {
        q[i] = rank as f64 / denom;
    }
    sites
        .iter()
        .enumerate()
        .map(|(i, s)| {
            (s.initial_population.max(50) as f64) * (1.15 + 0.5 * q[i]) * cfg.resource_abundance
        })
        .collect()
}

fn loss_fraction(base: u64, num: u64, den: u64) -> u64 {
    // floor(P × num / den) in integers.
    ((base as u128 * num as u128) / den as u128) as u64
}

impl Sim<'_> {
    fn s(&self, i: usize) -> &str {
        &self.sites[i].name
    }

    fn pname(&self, i: usize) -> String {
        self.people.list[i].name.clone()
    }

    fn record(&mut self, year: i32, ev: Ev) -> String {
        let seq = self.year_seq.entry(year).or_insert(0);
        *seq += 1;
        let id = format!("ev-{year}-{:04}", *seq);
        let mut site_ids = Vec::new();
        for s in ev.sites {
            let sid = self.sites[s].id.clone();
            if !site_ids.contains(&sid) {
                site_ids.push(sid);
            }
        }
        let mut person_ids = Vec::new();
        for p in ev.persons {
            let pid = self.people.list[p].id.clone();
            if !person_ids.contains(&pid) {
                person_ids.push(pid);
            }
        }
        self.events.push(Event {
            id: id.clone(),
            year,
            end_year: ev.end.unwrap_or(year),
            kind: ev.kind.into(),
            title: elide(&ev.title),
            description: elide(&ev.description),
            site_ids,
            organization_ids: ev.orgs.iter().map(|&o| self.orgs[o].id.clone()).collect(),
            person_ids,
            causes: ev.causes,
            effects: ev.effects,
            importance: ev.importance,
        });
        id
    }

    /// Removes up to `amount` people from a site; returns the exact applied effect.
    fn lose(&mut self, s: usize, amount: u64, summary: String) -> Effect {
        let applied = amount.min(self.state[s].pop);
        self.state[s].pop -= applied;
        Effect {
            site_id: self.sites[s].id.clone(),
            population_delta: -(applied as i64),
            food_delta: 0.0,
            wealth_delta: 0.0,
            summary,
        }
    }

    fn gain(&mut self, s: usize, amount: u64, summary: String) -> Effect {
        self.state[s].pop += amount;
        Effect {
            site_id: self.sites[s].id.clone(),
            population_delta: amount as i64,
            food_delta: 0.0,
            wealth_delta: 0.0,
            summary,
        }
    }

    fn adjust_wealth(&mut self, s: usize, delta: f64) -> f64 {
        let before = self.state[s].wealth;
        self.state[s].wealth = round2((before + delta).max(0.0));
        round2(self.state[s].wealth - before)
    }

    fn adjust_food(&mut self, s: usize, delta: f64) -> f64 {
        let before = self.state[s].food;
        self.state[s].food = (before + delta).max(0.0);
        round2(self.state[s].food - before)
    }

    fn hurt(&mut self, s: usize, year: i32, id: &str, stability: f64) {
        let st = &mut self.state[s];
        st.stability = (st.stability - stability).clamp(0.0, 1.0);
        st.hurt.push((year, id.to_string()));
        if st.hurt.len() > 6 {
            st.hurt.remove(0);
        }
    }

    fn site_of_person(&self, p: usize) -> usize {
        self.site_index
            .get(&self.people.list[p].site_id)
            .copied()
            .unwrap_or(0)
    }

    fn ruler(&self, polity: usize, year: i32) -> Option<usize> {
        self.holders[self.government[polity]].filter(|&h| self.people.alive(h, year))
    }

    fn run(&mut self) {
        let (start, end) = (self.cfg.start_year, self.cfg.end_year());
        self.choose_initial_holders(start);
        self.plan_pivot();
        if let Some(l) = self.league {
            let sites: Vec<usize> = self.orgs[l]
                .site_ids
                .iter()
                .map(|id| self.site_index[id])
                .collect();
            let names: Vec<&str> = sites.iter().map(|&s| self.s(s)).collect();
            let description = format!(
                "Les marchands de {} se lient par une charte commune ; la ligue traverse les frontières politiques. Ses membres restent comptés dans la population de leur ville.",
                names.join(", ")
            );
            let persons = self.holders[l].into_iter().collect();
            self.record(
                start,
                Ev {
                    kind: "foundation",
                    title: format!("Charte de la {}", self.orgs[l].name),
                    description,
                    sites,
                    orgs: vec![l],
                    persons,
                    importance: 0.5,
                    ..Ev::default()
                },
            );
        }
        for y in start..end {
            self.snapshot(y);
            self.start_pop = self.state.iter().map(|s| s.pop).collect();
            self.famine_ids.clear();
            self.successions(y);
            self.wars(y);
            self.epidemics(y);
            self.harvest(y);
            self.trade(y);
            self.migration(y);
            self.reconstruction(y);
            self.unrest(y);
            self.discovery(y);
            self.growth();
        }
        self.snapshot(end);
        if self.pivot_war.is_some()
            && !self
                .events
                .iter()
                .any(|e| e.kind == "hero" && e.importance >= 0.9)
        {
            self.warnings.push("Démo pivot : la bataille du héros n'a pas pu avoir lieu (site inaccessible ou déjà perdu).".into());
        }
    }

    fn snapshot(&mut self, year: i32) {
        let sites = self
            .state
            .iter()
            .enumerate()
            .map(|(i, st)| SiteState {
                site_id: self.sites[i].id.clone(),
                population: st.pop,
                food: round2(st.food),
                wealth: round2(st.wealth),
                stability: (st.stability * 1000.0).round() / 1000.0,
                polity_id: self.polity_ids[st.polity].clone(),
            })
            .collect();
        self.snapshots.push(Snapshot { year, sites });
    }

    // -- institutions ------------------------------------------------------

    fn choose_initial_holders(&mut self, year: i32) {
        for o in 0..self.orgs.len() {
            // A collective body (assembly) has no personal holder nor succession.
            if self.orgs[o].collective_office {
                continue;
            }
            if self.org_method[o] == Method::Unsupported {
                self.warnings.push(format!(
                    "{} : règle de succession « {} » non interprétée, office laissé vacant.",
                    self.orgs[o].name, self.orgs[o].succession
                ));
                continue;
            }
            let h = self.choose_holder(o, year, true);
            self.install(o, h);
            self.orgs[o].holder_id = Some(self.people.list[h].id.clone());
        }
    }

    fn install(&mut self, o: usize, h: usize) {
        if let Some(old) = self.holders[o] {
            self.holding.remove(&old);
        }
        self.holders[o] = Some(h);
        self.holding.insert(h);
        let role = format!("{} — {}", self.orgs[o].office_title, self.orgs[o].name);
        self.roles.entry(h).or_insert(role);
        if self.org_method[o] == Method::Hereditary {
            self.people.add_virtual_children(h);
        }
    }

    /// Picks an eligible living adult (imported people first, residents of the
    /// organization's sites first); creates a labelled virtual person otherwise.
    fn choose_holder(&mut self, o: usize, year: i32, initial: bool) -> usize {
        let org_sites: BTreeSet<usize> = self.orgs[o]
            .site_ids
            .iter()
            .map(|id| self.site_index[id])
            .collect();
        let polity = self.org_polity[o];
        let method = self.org_method[o];
        let mut best: Option<((usize, i64, u64), usize)> = None;
        for i in 0..self.people.list.len() {
            if !self.people.imported(i)
                || self.holding.contains(&i)
                || !self.people.active_adult(i, year)
            {
                continue;
            }
            let site = self.site_of_person(i);
            let tier = if org_sites.contains(&site) {
                0
            } else if self.state[site].polity == polity {
                2
            } else {
                continue;
            };
            let age = year - self.people.list[i].birth_year;
            let tier = tier
                + usize::from(
                    initial
                        && method == Method::Hereditary
                        && (self.people.children[i].is_empty() || age > 55),
                );
            let order = match method {
                Method::Seniority => (tier, -(age as i64), 0),
                _ => (
                    tier,
                    0,
                    key(
                        self.seed,
                        &[
                            "holder",
                            &self.orgs[o].id,
                            &year.to_string(),
                            &self.people.list[i].id,
                        ],
                    ),
                ),
            };
            if best.as_ref().is_none_or(|(b, _)| order < *b) {
                best = Some((order, i));
            }
        }
        if let Some((_, i)) = best {
            return i;
        }
        let seat = self.orgs[o].site_ids[0].clone();
        let age = range_i(
            self.seed,
            &["virtual-holder-age", &self.orgs[o].id, &year.to_string()],
            22,
            45,
        ) as i32;
        self.people.add_virtual(&seat, year - age, year, None, None)
    }

    fn successions(&mut self, y: i32) {
        for o in 0..self.orgs.len() {
            let Some(h) = self.holders[o] else { continue };
            let died = self.people.list[h].death_year == Some(y);
            let record_ends = !died && !self.people.alive(h, y);
            if !died && !record_ends {
                continue;
            }
            let method = self.org_method[o];
            let mut how = method.label().to_string();
            let mut next = None;
            if method == Method::Hereditary {
                let alive_child = self.people.children[h].iter().copied().find(|&c| {
                    self.people.alive(c, y)
                        && !self.holding.contains(&c)
                        && self.people.list[c].death_year != Some(y)
                });
                if let Some(c) = alive_child {
                    let minor = y - self.people.list[c].birth_year < crate::social::ADULT_AGE;
                    how = format!(
                        "succession héréditaire : aîné(e) des enfants vivants{}",
                        if minor { " (mineur, régence)" } else { "" }
                    );
                    next = Some(c);
                } else {
                    how = "succession héréditaire : aucun enfant vivant, nouvelle lignée".into();
                }
            }
            let next = next.unwrap_or_else(|| self.choose_holder(o, y, false));
            let old_name = self.pname(h);
            let cause_text = if died {
                if self.people.imported(h) {
                    format!("{old_name} meurt en {y} (décès enregistré par Lineage Lab)")
                } else {
                    format!("{old_name} meurt en {y}")
                }
            } else {
                format!(
                    "La trace de {old_name} s'arrête avec l'échantillon Lineage Lab (fin {}) ; Chronos considère l'office comme vacant",
                    self.people.import_end.unwrap_or(y - 1)
                )
            };
            self.install(o, next);
            let gov = self.org_profile[o] == ProfileKind::Monarchy
                || self.org_profile[o] == ProfileKind::Council;
            let seat = self.site_index[&self.orgs[o].site_ids[0]];
            let next_name = self.pname(next);
            let title = format!(
                "{next_name} devient {} ({})",
                self.orgs[o].office_title, self.orgs[o].name
            );
            let source_note = if self.people.imported(next) {
                ""
            } else {
                " Successeur virtuel (repli, aucun éligible importé)."
            };
            self.record(
                y,
                Ev {
                    kind: "succession",
                    title,
                    description: format!("{cause_text}. Règle appliquée : {how}.{source_note}"),
                    sites: vec![seat],
                    orgs: vec![o],
                    persons: vec![next, h],
                    importance: if gov { 0.7 } else { 0.35 },
                    ..Ev::default()
                },
            );
        }
    }

    // -- wars ----------------------------------------------------------------

    fn plan_pivot(&mut self) {
        if self.cfg.scenario != "pivot" {
            return;
        }
        let (start, end) = (self.cfg.start_year, self.cfg.end_year());
        self.pivot_epidemic = start <= PIVOT_EPIDEMIC.0 && PIVOT_EPIDEMIC.1 < end;
        if self.capitals.len() >= 2 && start + 5 <= PIVOT_HERO_YEAR && PIVOT_HERO_YEAR <= end - 2 {
            let eligible = |sim: &Self, y: i32| {
                (0..sim.people.list.len()).any(|i| {
                    sim.people.imported(i)
                        && sim.people.has_parent_in_record(i)
                        && sim.people.active_adult(i, y)
                })
            };
            let mut hero_year = PIVOT_HERO_YEAR;
            let found = (0..=30)
                .flat_map(|d| [PIVOT_HERO_YEAR - d, PIVOT_HERO_YEAR + d])
                .find(|&y| y >= start + 5 && y <= end - 2 && eligible(self, y));
            match found {
                Some(y) => hero_year = y,
                None => self.warnings.push(format!(
                    "Démo pivot : aucun membre adulte d'une famille Lineage Lab autour de {PIVOT_HERO_YEAR} ; le héros sera virtuel."
                )),
            }
            if hero_year != PIVOT_HERO_YEAR {
                self.warnings.push(format!(
                    "Démo pivot : aucun héros Lineage Lab éligible en {PIVOT_HERO_YEAR} ; année faisable la plus proche retenue : {hero_year}."
                ));
            }
            let war_start = (hero_year - 4).max(start + 1);
            let war_end = (hero_year + 1).min(end - 1);
            self.pivot_war = Some((war_start, war_end, hero_year));
            self.notes.push(format!(
                "Scénario pivot : guerre {war_start}–{war_end} entre {} et {}, {PIVOT_DESTROYED_VILLAGES} villages détruits (top-K sur tous les villages candidats), héros en {hero_year}.",
                self.polity_names[1], self.polity_names[0]
            ));
        }
        if self.pivot_epidemic {
            self.notes.push(format!(
                "Scénario pivot : épidémie continentale {}–{} ; la ville la plus peuplée perd exactement floor(P{PIVOT_CITY_YEAR} × 3 / 10) habitants en {PIVOT_CITY_YEAR}.",
                PIVOT_EPIDEMIC.0, PIVOT_EPIDEMIC.1
            ));
        }
    }

    fn at_war(&self, a: usize, b: usize) -> bool {
        self.wars
            .iter()
            .any(|w| (w.attacker == a && w.defender == b) || (w.attacker == b && w.defender == a))
    }

    fn wars(&mut self, y: i32) {
        // New wars first, so that battles of their first year take place.
        if let Some((ws, _, _)) = self.pivot_war {
            if y == ws {
                self.start_war(y, 1, 0, true);
            }
        }
        let polities = self.capitals.len();
        let mut adjacent = BTreeSet::new();
        for (a, list) in self.graph.adj.iter().enumerate() {
            for &(b, _) in list {
                let (pa, pb) = (self.state[a].polity, self.state[b].polity);
                if pa != pb {
                    adjacent.insert((pa.min(pb), pa.max(pb)));
                }
            }
        }
        for a in 0..polities {
            for b in a + 1..polities {
                if !adjacent.contains(&(a, b)) || self.at_war(a, b) {
                    continue;
                }
                if self.last_peace.get(&(a, b)).is_some_and(|&p| y < p + 8) {
                    continue;
                }
                if let Some((ws, we, _)) = self.pivot_war {
                    if (a, b) == (0, 1) && y >= ws - 8 && y <= we {
                        continue;
                    }
                }
                let envy = self
                    .richest_front(b, a)
                    .map_or(0.0, |s| self.strategic(s) / 100.0);
                let p = self.cfg.war_rate * 0.05 * (0.5 + envy);
                if unit(
                    self.seed,
                    &["war", &y.to_string(), &a.to_string(), &b.to_string()],
                ) >= p
                {
                    continue;
                }
                // The polity with less food per head attacks.
                let fpc = |p: usize| {
                    let (f, n) = self
                        .state
                        .iter()
                        .filter(|s| s.polity == p)
                        .fold((0.0, 0.0), |(f, n), s| (f + s.food, n + s.pop as f64));
                    f / n.max(1.0)
                };
                let (att, def) = if fpc(a) <= fpc(b) { (a, b) } else { (b, a) };
                self.start_war(y, att, def, false);
            }
        }
        // Battles planned for this year.
        for w in 0..self.wars.len() {
            let plan: Vec<_> = self.wars[w]
                .plan
                .iter()
                .copied()
                .filter(|p| p.0 == y)
                .collect();
            for (_, site, destroy, hero) in plan {
                self.battle(y, w, site, destroy, hero);
            }
        }
        // Wars ending this year.
        let mut w = 0;
        while w < self.wars.len() {
            if self.wars[w].end == y {
                let war = self.wars.remove(w);
                self.treaty(y, war);
            } else {
                w += 1;
            }
        }
    }

    fn strategic(&self, s: usize) -> f64 {
        let r = &self.sites[s].resources;
        r.iron.max(r.salt).clamp(0.0, 100.0)
    }

    fn richest_front(&self, defender: usize, attacker: usize) -> Option<usize> {
        let (days, _) = self.graph.dijkstra(&[self.capitals[attacker]]);
        let mut front: Vec<usize> = (0..self.sites.len())
            .filter(|&s| {
                self.state[s].polity == defender
                    && days[s].is_finite()
                    && self.state[s].destroyed.is_none()
            })
            .collect();
        front.sort_by(|&a, &b| days[a].total_cmp(&days[b]).then(a.cmp(&b)));
        front.truncate(4);
        front.into_iter().max_by(|&a, &b| {
            self.strategic(a)
                .total_cmp(&self.strategic(b))
                .then(b.cmp(&a))
        })
    }

    fn start_war(&mut self, y: i32, attacker: usize, defender: usize, pivot: bool) {
        let end_limit = self.cfg.end_year() - 1;
        let end = if pivot {
            self.pivot_war.map(|p| p.1).unwrap_or(y).min(end_limit)
        } else {
            (y + range_i(
                self.seed,
                &["war-length", &y.to_string(), &attacker.to_string()],
                0,
                4,
            ) as i32)
                .min(end_limit)
        };
        let (days, _) = self.graph.dijkstra(&[self.capitals[attacker]]);
        let mut front: Vec<usize> = (0..self.sites.len())
            .filter(|&s| {
                self.state[s].polity == defender
                    && self.state[s].destroyed.is_none()
                    && days[s].is_finite()
            })
            .collect();
        if front.is_empty() {
            // No land access to the defender: no battles are invented.
            front = Vec::new();
        }
        front.sort_by(|&a, &b| days[a].total_cmp(&days[b]).then(a.cmp(&b)));
        // An army cannot fight before it can arrive.
        let arrival = |s: usize| y + (days[s] / 365.0).floor() as i32;
        let mut plan: Vec<(i32, usize, bool, bool)> = Vec::new();
        let target = self.richest_front(defender, attacker);
        if pivot {
            let hero_year = self.pivot_war.map(|p| p.2).unwrap_or(y);
            // Exact quota: top-K over the complete index of candidate villages.
            let mut villages: Vec<(i64, u64, usize)> = front
                .iter()
                .copied()
                .filter(|&s| is_village(&self.sites[s]) && !self.capitals.contains(&s))
                .map(|s| {
                    (
                        days[s] as i64,
                        key(self.seed, &["pivot-destroy", &self.sites[s].id]),
                        s,
                    )
                })
                .collect();
            villages.sort();
            let mut year = y;
            let mut destroyed = 0;
            for &(_, _, s) in &villages {
                if destroyed == PIVOT_DESTROYED_VILLAGES {
                    break;
                }
                let at = year.max(arrival(s));
                if at > end {
                    continue;
                }
                plan.push((at, s, true, false));
                destroyed += 1;
                year = (at + 1).min(hero_year - 1).max(y);
            }
            if destroyed < PIVOT_DESTROYED_VILLAGES {
                self.warnings.push(format!(
                    "Démo pivot : seulement {destroyed} village(s) atteignable(s) pour la guerre (quota {PIVOT_DESTROYED_VILLAGES})."
                ));
            }
            let hero_site = front
                .iter()
                .copied()
                .find(|s| !plan.iter().any(|p| p.1 == *s) && arrival(*s) <= hero_year);
            if let Some(s) = hero_site.filter(|_| hero_year <= end) {
                plan.push((hero_year, s, false, true));
            }
            for yr in y..=end {
                if !plan.iter().any(|p| p.0 == yr) {
                    if let Some(&s) = front
                        .iter()
                        .find(|&&s| arrival(s) <= yr && !plan.iter().any(|p| p.1 == s && p.2))
                    {
                        plan.push((yr, s, false, false));
                    }
                }
            }
        } else {
            let mut destroyed_one = false;
            for yr in y..=end {
                let pick = range_i(
                    self.seed,
                    &[
                        "battle-site",
                        &y.to_string(),
                        &yr.to_string(),
                        &attacker.to_string(),
                    ],
                    0,
                    2,
                ) as usize;
                let candidates: Vec<usize> = front
                    .iter()
                    .copied()
                    .filter(|&s| arrival(s) <= yr)
                    .take(3)
                    .collect();
                let site = if yr == y {
                    target.filter(|&t| arrival(t) <= yr)
                } else {
                    None
                }
                .or_else(|| {
                    candidates
                        .get(pick.min(candidates.len().saturating_sub(1)))
                        .copied()
                });
                if let Some(s) = site {
                    let destroy = !destroyed_one
                        && is_village(&self.sites[s])
                        && unit(
                            self.seed,
                            &["war-destroy", &y.to_string(), &self.sites[s].id],
                        ) < 0.25 * self.cfg.war_rate;
                    destroyed_one |= destroy;
                    plan.push((yr, s, destroy, false));
                }
            }
        }
        plan.sort();
        let (an, dn) = (
            self.polity_names[attacker].clone(),
            self.polity_names[defender].clone(),
        );
        let motive = match target {
            Some(t) if self.strategic(t) >= 50.0 => {
                format!(
                    " {an} convoite {} et ses ressources (fer ou sel, potentiel {:.0}/100).",
                    self.s(t),
                    self.strategic(t)
                )
            }
            _ => String::new(),
        };
        let pivot_note = if pivot {
            " (scénario pivot de la démo)"
        } else {
            ""
        };
        let persons: Vec<usize> = [self.ruler(attacker, y), self.ruler(defender, y)]
            .into_iter()
            .flatten()
            .filter(|&p| self.people.active_adult(p, y) && self.people.alive(p, end))
            .collect();
        let id = self.record(
            y,
            Ev {
                end: Some(end),
                kind: "war",
                title: format!("Guerre entre {an} et {dn}"),
                description: format!(
                    "{an} attaque {dn}{pivot_note}.{motive} Les batailles sont datées après le temps de marche depuis la capitale de l'agresseur."
                ),
                sites: plan.iter().map(|p| p.1).collect(),
                orgs: vec![self.government[attacker], self.government[defender]],
                persons,
                importance: if pivot { 0.95 } else { 0.85 },
                ..Ev::default()
            },
        );
        self.wars.push(War {
            id,
            attacker,
            defender,
            end,
            pivot,
            plan,
        });
    }

    fn battle(&mut self, y: i32, w: usize, s: usize, destroy: bool, hero: bool) {
        let (war_id, attacker, defender, pivot) = (
            self.wars[w].id.clone(),
            self.wars[w].attacker,
            self.wars[w].defender,
            self.wars[w].pivot,
        );
        if self.state[s].polity != defender || self.state[s].destroyed.is_some() {
            return;
        }
        let (days, _) = self.graph.dijkstra(&[self.capitals[attacker]]);
        let march = format!(
            "L'armée de {} arrive de {} après {} jours de marche.",
            self.polity_names[attacker],
            self.s(self.capitals[attacker]),
            days[s]
        );
        let base = self.start_pop[s];
        let mut effects = Vec::new();
        let (kind_title, description);
        if destroy {
            let killed = loss_fraction(base, 25, 100);
            let site_name = self.s(s).to_string();
            let refuge = self.refuge(s, defender);
            let killed = killed.min(self.state[s].pop);
            let fled = self.state[s].pop - killed;
            let mut e = self.lose(s, killed + fled, String::new());
            e.food_delta = self.adjust_food(s, -self.state[s].food);
            e.wealth_delta = self.adjust_wealth(s, -self.state[s].wealth);
            if let Some(r) = refuge {
                e.summary = format!("{killed} morts (floor(P × 25/100), P = {base}), {fled} réfugiés partent ; village détruit");
                effects.push(e);
                let refuge_name = self.s(r).to_string();
                effects.push(self.gain(r, fled, format!("{fled} réfugiés venus de {site_name}")));
                description = format!("{march} {site_name} est pillé et incendié ; les survivants se réfugient à {refuge_name}.");
            } else {
                e.summary = format!("{killed} morts, {fled} survivants dispersés hors des sites cartographiés ; village détruit");
                effects.push(e);
                description = format!("{march} {site_name} est pillé et incendié ; aucun refuge accessible par la route.");
            }
            kind_title = format!("Sac de {site_name}");
        } else {
            let pct = range_i(
                self.seed,
                &["battle-loss", &y.to_string(), &self.sites[s].id],
                2,
                6,
            ) as u64;
            let loss = loss_fraction(base, pct, 100);
            let mut e = self.lose(
                s,
                loss,
                format!("{loss} morts (floor(P × {pct}/100), P = {base})"),
            );
            e.food_delta = self.adjust_food(s, -0.2 * self.state[s].food);
            e.wealth_delta = self.adjust_wealth(s, -0.15 * self.state[s].wealth);
            effects.push(e);
            kind_title = format!("Bataille de {}", self.s(s));
            description = march;
        }
        let id = self.record(
            y,
            Ev {
                kind: "battle",
                title: kind_title,
                description,
                sites: effects
                    .iter()
                    .filter_map(|e| self.site_index.get(&e.site_id).copied())
                    .collect(),
                orgs: vec![self.government[attacker], self.government[defender]],
                causes: vec![war_id],
                effects,
                importance: if destroy { 0.7 } else { 0.5 },
                ..Ev::default()
            },
        );
        self.hurt(s, y, &id, if destroy { 1.0 } else { 0.15 });
        self.state[s].battle = Some((y, id.clone()));
        if destroy {
            self.state[s].destroyed = Some((y, id.clone()));
            self.state[s].stability = 0.1;
        }
        if hero {
            self.pivot_hero(y, s, defender, &id);
        } else if !pivot && unit(self.seed, &["hero", &id]) < 0.08 {
            self.local_hero(y, s, defender, &id);
        }
    }

    fn refuge(&self, s: usize, polity: usize) -> Option<usize> {
        let (days, _) = self.graph.dijkstra(&[s]);
        (0..self.sites.len())
            .filter(|&r| {
                r != s
                    && days[r].is_finite()
                    && self.state[r].destroyed.is_none()
                    && self.state[r].polity == polity
            })
            .min_by(|&a, &b| days[a].total_cmp(&days[b]).then(a.cmp(&b)))
    }

    fn hero_candidate(&self, y: i32, polity: usize, salt: &str) -> Option<usize> {
        (0..self.people.list.len())
            .filter(|&i| {
                self.people.imported(i)
                    && self.people.has_parent_in_record(i)
                    && self.people.active_adult(i, y)
                    && !self.holding.contains(&i)
            })
            .min_by_key(|&i| {
                let other = usize::from(self.state[self.site_of_person(i)].polity != polity);
                (
                    other,
                    key(self.seed, &["hero-pick", salt, &self.people.list[i].id]),
                )
            })
    }

    fn pivot_hero(&mut self, y: i32, s: usize, defender: usize, battle_id: &str) {
        let hero = match self.hero_candidate(y, defender, "pivot") {
            Some(h) => h,
            None => {
                self.warnings.push(format!(
                    "Démo pivot : aucun héros Lineage Lab éligible en {y}, héros virtuel étiqueté."
                ));
                let site = self.sites[s].id.clone();
                self.people.add_virtual(&site, y - 24, y, None, None)
            }
        };
        let p = &self.people.list[hero];
        let origin = if self.people.imported(hero) {
            format!("issu(e) d'une famille Lineage Lab (né(e) en {}, vivant(e) et adulte en {y} selon l'archive)", p.birth_year)
        } else {
            "personnage virtuel de repli".to_string()
        };
        let sovereign = self.ruler(defender, y);
        let hero_name = p.name.clone();
        let (title, description, persons) = match sovereign {
            Some(r) => (
                format!("{hero_name} sauve {}", self.pname(r)),
                format!(
                    "Pendant la bataille de {}, {hero_name}, {origin}, protège {} ({}, {}) d'une capture certaine (scénario pivot de la démo).",
                    self.s(s),
                    self.pname(r),
                    self.orgs[self.government[defender]].office_title,
                    self.orgs[self.government[defender]].name
                ),
                vec![hero, r],
            ),
            None => (
                format!("{hero_name} défend {}", self.s(s)),
                format!(
                    "Pendant la bataille de {}, {hero_name}, {origin}, tient la position ; {} (scénario pivot de la démo).",
                    self.s(s),
                    if self.orgs[self.government[defender]].collective_office {
                        "le pouvoir souverain est exercé collectivement"
                    } else {
                        "l'office souverain est vacant"
                    }
                ),
                vec![hero],
            ),
        };
        self.roles.entry(hero).or_insert_with(|| "héros".into());
        self.record(
            y,
            Ev {
                kind: "hero",
                title,
                description,
                sites: vec![s],
                orgs: vec![self.government[defender]],
                persons,
                causes: vec![battle_id.to_string()],
                importance: 0.95,
                ..Ev::default()
            },
        );
    }

    fn local_hero(&mut self, y: i32, s: usize, defender: usize, battle_id: &str) {
        let Some(hero) = self.hero_candidate(y, defender, battle_id) else {
            return;
        };
        self.roles
            .entry(hero)
            .or_insert_with(|| "héros local".into());
        let name = self.pname(hero);
        self.record(
            y,
            Ev {
                kind: "hero",
                title: format!("{name} se distingue à {}", self.s(s)),
                description: format!(
                    "{name} rallie les défenseurs de {} pendant la bataille.",
                    self.s(s)
                ),
                sites: vec![s],
                persons: vec![hero],
                causes: vec![battle_id.to_string()],
                importance: 0.45,
                ..Ev::default()
            },
        );
    }

    fn treaty(&mut self, y: i32, war: War) {
        let (a, d) = (war.attacker, war.defender);
        self.last_peace.insert((a.min(d), a.max(d)), y);
        let wins = unit(self.seed, &["war-outcome", &war.id]) < 0.55;
        let mut transferred = Vec::new();
        if wins {
            for &(_, s, _, _) in &war.plan {
                if transferred.len() < 2
                    && !transferred.contains(&s)
                    && self.state[s].polity == d
                    && self.state[s].destroyed.is_none()
                    && !self.capitals.contains(&s)
                {
                    transferred.push(s);
                }
            }
        }
        for &s in &transferred {
            self.state[s].polity = a;
        }
        let (an, dn) = (self.polity_names[a].clone(), self.polity_names[d].clone());
        let description = if transferred.is_empty() {
            format!("La paix rétablit les frontières antérieures entre {an} et {dn}.")
        } else {
            let names: Vec<&str> = transferred.iter().map(|&s| self.s(s)).collect();
            format!("{dn} cède {} à {an}.", names.join(", "))
        };
        let sites = if transferred.is_empty() {
            vec![self.capitals[d]]
        } else {
            transferred
        };
        self.record(
            y,
            Ev {
                kind: "treaty",
                title: format!("Paix entre {an} et {dn}"),
                description,
                sites,
                orgs: vec![self.government[a], self.government[d]],
                causes: vec![war.id],
                importance: 0.6,
                ..Ev::default()
            },
        );
    }

    // -- epidemics -----------------------------------------------------------

    fn epidemics(&mut self, y: i32) {
        let active = !self.waves.is_empty();
        let in_pivot =
            self.pivot_epidemic && (PIVOT_EPIDEMIC.0 - 4..=PIVOT_EPIDEMIC.1 + 2).contains(&y);
        if self.pivot_epidemic && y == PIVOT_EPIDEMIC.0 {
            self.start_wave(y, true);
        } else if !active
            && !in_pivot
            && unit(self.seed, &["epidemic", &y.to_string()]) < self.cfg.epidemic_rate * 0.035
        {
            self.start_wave(y, false);
        }
        for w in 0..self.waves.len() {
            let hits: Vec<_> = self.waves[w]
                .hits
                .iter()
                .copied()
                .filter(|h| h.0 == y)
                .collect();
            for (_, s, num, den) in hits {
                let wave_id = self.waves[w].id.clone();
                self.epidemic_hit(y, s, num, den, &wave_id);
            }
        }
        self.waves.retain(|w| w.hits.iter().any(|h| h.0 > y));
    }

    fn start_wave(&mut self, y: i32, pivot: bool) {
        let end_limit = self.cfg.end_year() - 1;
        let mut by_pop: Vec<usize> = (0..self.sites.len())
            .filter(|&s| self.state[s].destroyed.is_none())
            .collect();
        by_pop.sort_by(|&a, &b| self.state[b].pop.cmp(&self.state[a].pop).then(a.cmp(&b)));
        if by_pop.is_empty() {
            return;
        }
        let city = by_pop[0];
        let pool = &by_pop[..by_pop.len().min(6)];
        let origin = pool
            [(key(self.seed, &["epidemic-origin", &y.to_string()]) % pool.len() as u64) as usize];
        let end = if pivot {
            PIVOT_EPIDEMIC.1
        } else {
            (y + range_i(self.seed, &["epidemic-length", &y.to_string()], 0, 3) as i32)
                .min(end_limit)
        };
        let (days, _) = self.graph.dijkstra(&[origin]);
        let mut hits = Vec::new();
        for (s, &d) in days.iter().enumerate() {
            let sid = &self.sites[s].id;
            if pivot && s == city {
                hits.push((PIVOT_CITY_YEAR, s, 3, 10));
                continue;
            }
            if !d.is_finite() {
                continue;
            }
            let at = y + (d / 40.0).floor() as i32;
            let attack = if pivot { 0.65 } else { 0.5 };
            if at <= end && unit(self.seed, &["epidemic-hit", &y.to_string(), sid]) < attack {
                let num = range_i(
                    self.seed,
                    &["epidemic-severity", &y.to_string(), sid],
                    4,
                    if pivot { 20 } else { 14 },
                ) as u64;
                hits.push((at, s, num, 100));
            }
        }
        hits.sort();
        let names = if pivot {
            "Grande épidémie".to_string()
        } else {
            format!("Épidémie partie de {}", self.s(origin))
        };
        let description = if pivot {
            format!(
                "Une épidémie continentale (scénario pivot) atteint la région depuis {} et progresse le long des routes (≈ 40 jours de trajet par an). {} perd exactement floor(P{PIVOT_CITY_YEAR} × 3 / 10) habitants en {PIVOT_CITY_YEAR}.",
                self.s(origin),
                self.s(city)
            )
        } else {
            format!(
                "Une épidémie part de {} et suit les routes (≈ 40 jours de trajet par an).",
                self.s(origin)
            )
        };
        let sites: Vec<usize> = std::iter::once(origin)
            .chain(hits.iter().map(|h| h.1))
            .collect();
        let id = self.record(
            y,
            Ev {
                end: Some(end),
                kind: "epidemic",
                title: names,
                description,
                sites,
                importance: if pivot { 0.9 } else { 0.65 },
                ..Ev::default()
            },
        );
        self.waves.push(Wave { id, hits });
    }

    fn victims(&self, y: i32, sites: &[usize]) -> Vec<usize> {
        (0..self.people.list.len())
            .filter(|&i| {
                self.people.imported(i)
                    && self.people.alive(i, y)
                    && self.people.list[i].death_year == Some(y)
                    && sites.contains(&self.site_of_person(i))
            })
            .collect()
    }

    fn epidemic_hit(&mut self, y: i32, s: usize, num: u64, den: u64, wave_id: &str) {
        if self.state[s].destroyed.is_some() || self.state[s].pop == 0 {
            return;
        }
        let base = self.start_pop[s];
        let loss = loss_fraction(base, num, den);
        let effect = self.lose(
            s,
            loss,
            format!("{loss} morts = floor({base} × {num}/{den}), population au début de {y}"),
        );
        let victims = self.victims(y, &[s]);
        let mut description = format!(
            "L'épidémie tue {} habitants de {} (règle : floor(P × {num}/{den}) sur la population au début de l'année).",
            -effect.population_delta,
            self.s(s)
        );
        if !victims.is_empty() {
            let names: Vec<String> = victims.iter().map(|&v| self.pname(v)).collect();
            description.push_str(&format!(
                " Parmi les morts figure {} (décès enregistré par Lineage Lab cette année-là).",
                names.join(", ")
            ));
        }
        let id = self.record(
            y,
            Ev {
                kind: "epidemic",
                title: format!("L'épidémie frappe {}", self.s(s)),
                description,
                sites: vec![s],
                persons: victims,
                causes: vec![wave_id.to_string()],
                effects: vec![effect],
                importance: 0.35 + 0.4 * (num as f64 / den as f64),
                ..Ev::default()
            },
        );
        self.hurt(s, y, &id, 0.1);
    }

    // -- economy -------------------------------------------------------------

    fn harvest(&mut self, y: i32) {
        let mut regions: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (s, site) in self.sites.iter().enumerate() {
            regions.entry(site.region_id.clone()).or_default().push(s);
        }
        for (region, members) in regions {
            let ys = y.to_string();
            let mut factor = 0.8 + 0.38 * unit(self.seed, &["harvest", &region, &ys]);
            let bad = unit(self.seed, &["bad-harvest", &region, &ys]) < 0.07;
            if bad {
                factor *= 0.65;
            }
            let mut effects = Vec::new();
            let mut causes = Vec::new();
            let mut hit_sites = Vec::new();
            for s in members {
                if self.state[s].destroyed.is_some() {
                    continue;
                }
                let mut f = factor;
                let war_damage = self.state[s]
                    .battle
                    .clone()
                    .filter(|(by, _)| *by == y || *by == y - 1);
                if war_damage.is_some() {
                    f *= 0.7;
                }
                let st = &mut self.state[s];
                let stock = st.food + st.cap * f - st.pop as f64;
                if stock >= 0.0 {
                    st.food = stock.min(st.cap * 0.6);
                    continue;
                }
                st.food = 0.0;
                let deaths = ((-stock) * 0.12).min(st.pop as f64 * 0.10).floor() as u64;
                if deaths < 5.max(st.pop / 300) {
                    continue;
                }
                if let Some((_, id)) = war_damage {
                    causes.push(id);
                }
                effects.push(self.lose(
                    s,
                    deaths,
                    format!(
                        "{deaths} morts de faim (12 % du déficit, au plus 10 % de la population)"
                    ),
                ));
                hit_sites.push(s);
            }
            if effects.is_empty() {
                continue;
            }
            let victims = self.victims(y, &hit_sites);
            let title = if hit_sites.len() == 1 {
                format!("Disette à {}", self.s(hit_sites[0]))
            } else {
                format!("Disette dans {} ({} lieux)", region, hit_sites.len())
            };
            let mut description = format!(
                "Récolte au facteur {:.2}{} : les réserves ne couvrent plus la consommation.",
                factor,
                if bad {
                    " (mauvaise année régionale)"
                } else {
                    ""
                }
            );
            if !causes.is_empty() {
                description
                    .push_str(" Les champs ravagés par les combats produisent 30 % de moins.");
            }
            if !victims.is_empty() {
                let names: Vec<String> = victims.iter().map(|&v| self.pname(v)).collect();
                description.push_str(&format!(
                    " Décès enregistrés par Lineage Lab cette année : {}.",
                    names.join(", ")
                ));
            }
            causes.sort();
            causes.dedup();
            let id = self.record(
                y,
                Ev {
                    kind: "famine",
                    title,
                    description,
                    sites: hit_sites.clone(),
                    persons: victims,
                    causes,
                    effects,
                    importance: 0.5,
                    ..Ev::default()
                },
            );
            for s in hit_sites {
                self.hurt(s, y, &id, 0.12);
                self.famine_ids.insert(s, id.clone());
            }
        }
    }

    fn trade(&mut self, y: i32) {
        for s in 0..self.sites.len() {
            let degree = self.graph.adj[s].len() as f64;
            let st = &mut self.state[s];
            st.wealth = round2(
                st.wealth * 0.98
                    + st.pop as f64 / 1000.0 * (0.4 + 0.25 * self.cfg.trade_intensity * degree),
            );
            st.stability += (0.75 - st.stability) * 0.1;
        }
        let Some(l) = self.league else { return };
        if unit(self.seed, &["fair", &y.to_string()]) >= self.cfg.trade_intensity * 0.3 {
            return;
        }
        let members: Vec<usize> = self.orgs[l]
            .site_ids
            .iter()
            .map(|id| self.site_index[id])
            .filter(|&s| self.state[s].destroyed.is_none())
            .collect();
        if members.is_empty() {
            return;
        }
        let host = members
            [(key(self.seed, &["fair-host", &y.to_string()]) % members.len() as u64) as usize];
        let mut effects = Vec::new();
        for &s in &members {
            let gain = self.state[s].pop as f64 / 1000.0
                * 3.0
                * self.cfg.trade_intensity
                * if s == host { 2.0 } else { 1.0 };
            let applied = self.adjust_wealth(s, gain);
            self.state[s].stability = (self.state[s].stability + 0.02).min(1.0);
            effects.push(Effect {
                site_id: self.sites[s].id.clone(),
                population_delta: 0,
                food_delta: 0.0,
                wealth_delta: applied,
                summary: format!("recettes de foire +{applied:.2}"),
            });
        }
        let persons = self.holders[l]
            .filter(|&h| self.people.active_adult(h, y))
            .into_iter()
            .collect();
        self.record(
            y,
            Ev {
                kind: "trade",
                title: format!("Foire de {}", self.s(host)),
                description: format!(
                    "La {} tient sa foire à {} ; les villes membres en tirent des recettes (unité de richesse conventionnelle).",
                    self.orgs[l].name,
                    self.s(host)
                ),
                sites: std::iter::once(host).chain(members.iter().copied()).collect(),
                orgs: vec![l],
                persons,
                effects,
                importance: 0.25,
                ..Ev::default()
            },
        );
    }

    fn migration(&mut self, y: i32) {
        let mut pressured: Vec<(u64, usize)> = (0..self.sites.len())
            .filter(|&s| {
                let st = &self.state[s];
                st.destroyed.is_none()
                    && (st.pop as f64 > st.cap * 1.02 || self.famine_ids.contains_key(&s))
            })
            .map(|s| {
                (
                    ((self.state[s].pop as f64 / self.state[s].cap) * 1000.0) as u64,
                    s,
                )
            })
            .collect();
        pressured.sort_by(|a, b| b.cmp(a));
        let mut done = 0;
        for (_, s) in pressured {
            if done == 2 {
                break;
            }
            if unit(self.seed, &["migration", &y.to_string(), &self.sites[s].id])
                >= self.cfg.migration_rate * 0.6
            {
                continue;
            }
            let dest = self.graph.adj[s]
                .iter()
                .map(|&(d, _)| d)
                .filter(|&d| {
                    self.state[d].destroyed.is_none()
                        && (self.state[d].pop as f64) < self.state[d].cap * 0.9
                })
                .min_by(|&a, &b| {
                    let ra = self.state[a].pop as f64 / self.state[a].cap;
                    let rb = self.state[b].pop as f64 / self.state[b].cap;
                    ra.total_cmp(&rb).then(a.cmp(&b))
                });
            let Some(d) = dest else { continue };
            let pct = (2.0 + 6.0 * self.cfg.migration_rate).floor() as u64;
            let migrants = loss_fraction(self.state[s].pop, pct, 100);
            if migrants < 10 {
                continue;
            }
            let (from, to) = (self.s(s).to_string(), self.s(d).to_string());
            let out = self.lose(s, migrants, format!("{migrants} départs vers {to}"));
            let arrived = (-out.population_delta) as u64;
            let inn = self.gain(d, arrived, format!("{arrived} arrivées depuis {from}"));
            let causes: Vec<String> = self.famine_ids.get(&s).cloned().into_iter().collect();
            let why = if causes.is_empty() {
                "la terre ne suffit plus à nourrir tout le monde"
            } else {
                "la disette pousse au départ"
            };
            self.record(
                y,
                Ev {
                    kind: "migration",
                    title: format!("Migration de {from} vers {to}"),
                    description: format!("{arrived} habitants quittent {from} pour {to} : {why}. Transfert compté une seule fois."),
                    sites: vec![s, d],
                    causes,
                    effects: vec![out, inn],
                    importance: 0.3,
                    ..Ev::default()
                },
            );
            done += 1;
        }
    }

    fn reconstruction(&mut self, y: i32) {
        for s in 0..self.sites.len() {
            let Some((since, destroy_id)) = self.state[s].destroyed.clone() else {
                continue;
            };
            let delay = range_i(
                self.seed,
                &["rebuild-delay", &self.sites[s].id, &since.to_string()],
                6,
                18,
            ) as i32;
            if y < since + delay {
                continue;
            }
            let polity = self.state[s].polity;
            let (days, _) = self.graph.dijkstra(&[s]);
            let source = (0..self.sites.len())
                .filter(|&r| {
                    r != s
                        && days[r].is_finite()
                        && self.state[r].destroyed.is_none()
                        && self.state[r].pop >= 300
                })
                .min_by(|&a, &b| {
                    let pa = usize::from(self.state[a].polity != polity);
                    let pb = usize::from(self.state[b].polity != polity);
                    pa.cmp(&pb)
                        .then(days[a].total_cmp(&days[b]))
                        .then(a.cmp(&b))
                });
            let Some(r) = source else { continue };
            let settlers = loss_fraction(self.state[r].pop, 4, 100)
                .min((self.sites[s].initial_population / 3).max(50));
            if settlers < 20 {
                continue;
            }
            let (from, to) = (self.s(r).to_string(), self.s(s).to_string());
            let out = self.lose(r, settlers, format!("{settlers} colons partent pour {to}"));
            let mut inn = self.gain(s, settlers, format!("{settlers} colons réinstallés"));
            inn.food_delta = self.adjust_food(s, self.state[s].cap * 0.1);
            self.state[s].destroyed = None;
            self.state[s].stability = 0.5;
            self.record(
                y,
                Ev {
                    kind: "reconstruction",
                    title: format!("Reconstruction de {to}"),
                    description: format!("{} ans après sa destruction, {to} est repeuplé par {settlers} colons venus de {from}.", y - since),
                    sites: vec![s, r],
                    causes: vec![destroy_id],
                    effects: vec![out, inn],
                    importance: 0.45,
                    ..Ev::default()
                },
            );
        }
    }

    fn unrest(&mut self, y: i32) {
        for s in 0..self.sites.len() {
            let st = &self.state[s];
            if st.destroyed.is_some() || st.stability >= 0.3 || st.pop < 50 {
                continue;
            }
            if unit(self.seed, &["unrest", &y.to_string(), &self.sites[s].id]) >= 0.3 {
                continue;
            }
            let causes: Vec<String> = st
                .hurt
                .iter()
                .rev()
                .filter(|(hy, _)| y - hy <= 3)
                .take(3)
                .map(|(_, id)| id.clone())
                .collect();
            let base = self.start_pop[s];
            let loss = loss_fraction(base, 1, 100);
            let mut e = self.lose(s, loss, format!("{loss} morts lors des troubles"));
            e.wealth_delta = self.adjust_wealth(s, -0.1 * self.state[s].wealth);
            let id = self.record(
                y,
                Ev {
                    kind: "local",
                    title: format!("Révolte à {}", self.s(s)),
                    description: format!(
                        "La population de {} se soulève après des années difficiles (stabilité {:.2}).",
                        self.s(s),
                        self.state[s].stability
                    ),
                    sites: vec![s],
                    causes,
                    effects: vec![e],
                    importance: 0.4,
                    ..Ev::default()
                },
            );
            self.state[s].stability = (self.state[s].stability + 0.2).min(1.0);
            let polity = self.state[s].polity;
            self.unrest_ids.push((y, polity, id));
        }
        self.unrest_ids.retain(|(uy, _, _)| y - uy <= 5);
        for p in 0..self.capitals.len() {
            let members: Vec<usize> = (0..self.sites.len())
                .filter(|&s| self.state[s].polity == p)
                .collect();
            let mean = members
                .iter()
                .map(|&s| self.state[s].stability)
                .sum::<f64>()
                / members.len().max(1) as f64;
            let causes: Vec<String> = self
                .unrest_ids
                .iter()
                .filter(|u| u.1 == p)
                .map(|u| u.2.clone())
                .rev()
                .take(3)
                .collect();
            if causes.is_empty()
                || mean >= 0.55
                || self.last_reform.get(&p).is_some_and(|&r| y - r < 15)
            {
                continue;
            }
            if unit(self.seed, &["reform", &y.to_string(), &p.to_string()]) >= 0.35 {
                continue;
            }
            self.last_reform.insert(p, y);
            for &s in &members {
                self.state[s].stability = (self.state[s].stability + 0.12).min(1.0);
            }
            let g = self.government[p];
            let holder = self.holders[g].filter(|&h| self.people.active_adult(h, y));
            let who = holder.map_or_else(
                || "Le pouvoir".to_string(),
                |h| format!("{} {}", self.orgs[g].office_title, self.pname(h)),
            );
            self.record(
                y,
                Ev {
                    kind: "reform",
                    title: format!("Réformes dans {}", self.polity_names[p]),
                    description: format!("{who} concède des allègements fiscaux après les révoltes ; la stabilité remonte de 0,12."),
                    sites: vec![self.capitals[p]],
                    orgs: vec![g],
                    persons: holder.into_iter().collect(),
                    causes,
                    importance: 0.5,
                    ..Ev::default()
                },
            );
        }
    }

    fn discovery(&mut self, y: i32) {
        for s in 0..self.sites.len() {
            let r = &self.sites[s].resources;
            let (what, value) = if r.iron >= r.salt {
                ("fer", r.iron)
            } else {
                ("sel", r.salt)
            };
            if self.state[s].mined || self.state[s].destroyed.is_some() || value < 70.0 {
                continue;
            }
            if unit(self.seed, &["discovery", &y.to_string(), &self.sites[s].id])
                >= 0.004 * self.cfg.resource_abundance
            {
                continue;
            }
            self.state[s].mined = true;
            let gain = self.state[s].pop as f64 / 1000.0 * 10.0;
            let applied = self.adjust_wealth(s, gain);
            self.record(
                y,
                Ev {
                    kind: "discovery",
                    title: format!("Exploitation du {what} à {}", self.s(s)),
                    description: format!(
                        "Les habitants de {} ouvrent une exploitation de {what} (potentiel {value:.0}/100, indice non arpenté).",
                        self.s(s)
                    ),
                    sites: vec![s],
                    effects: vec![Effect {
                        site_id: self.sites[s].id.clone(),
                        population_delta: 0,
                        food_delta: 0.0,
                        wealth_delta: applied,
                        summary: format!("richesse +{applied:.2}"),
                    }],
                    importance: 0.3,
                    ..Ev::default()
                },
            );
        }
    }

    /// Natural demographic change (not an event): logistic towards capacity.
    fn growth(&mut self) {
        let g = self.cfg.growth_rate;
        for st in &mut self.state {
            if st.destroyed.is_some() || st.pop == 0 {
                continue;
            }
            let p = st.pop as f64;
            let delta = if p < st.cap {
                p * g * (1.0 + 2.0 * (1.0 - p / st.cap))
            } else {
                -(p - st.cap) * 0.05 + p * g.min(0.0)
            };
            let delta = delta.round() as i64;
            st.pop = if delta < 0 {
                st.pop.saturating_sub(delta.unsigned_abs())
            } else {
                st.pop + delta as u64
            };
        }
    }
}
