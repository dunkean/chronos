//! Polities, institution profiles, organizations and the people registry.

use std::collections::{BTreeMap, BTreeSet};

use crate::keys::{key, range_i};
use crate::model::{Blueprint, BootstrapPerson, Config, Organization, Person, Polity, Site};
use crate::names::{elide, pick, resolve_culture, Pools};
use crate::world::Graph;

pub const ADULT_AGE: i32 = 16;
const COLORS: &[&str] = &[
    "#8c3b3b", "#3b5f8c", "#5f7f3b", "#8c6f3b", "#5d3b8c", "#3b8c84",
];

// ---------------------------------------------------------------------------
// Institution profiles

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProfileKind {
    Monarchy,
    Council,
    League,
    Village,
    Informal,
}

impl ProfileKind {
    pub const ALL: [ProfileKind; 5] = [
        ProfileKind::Monarchy,
        ProfileKind::Council,
        ProfileKind::League,
        ProfileKind::Village,
        ProfileKind::Informal,
    ];

    pub fn org_kind(self) -> &'static str {
        match self {
            ProfileKind::Monarchy => "monarchy",
            ProfileKind::Council => "council",
            ProfileKind::League => "merchant_league",
            ProfileKind::Village => "village_community",
            ProfileKind::Informal => "informal_collective",
        }
    }

    fn keywords(self) -> &'static [&'static str] {
        match self {
            ProfileKind::Monarchy => &["monarch", "kingdom", "royaume", "crown", "king", "roi"],
            ProfileKind::Council => &[
                "council",
                "conseil",
                "republic",
                "république",
                "senate",
                "commune urbaine",
            ],
            ProfileKind::League => &[
                "league", "ligue", "hanse", "merchant", "marchand", "guild", "trade",
            ],
            ProfileKind::Village => &[
                "village",
                "rural",
                "commune",
                "community",
                "communauté",
                "parish",
            ],
            ProfileKind::Informal => &[
                "informal",
                "informel",
                "collective",
                "collectif",
                "band",
                "fellowship",
                "confr",
            ],
        }
    }
}

/// Closed list of succession methods Chronos interprets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Hereditary,
    Election,
    Appointment,
    Seniority,
    Unsupported,
}

impl Method {
    /// Exact identifiers only (case, spaces and hyphens normalized); any other
    /// text, e.g. a prose rule or "non-hereditary", is not executed.
    pub fn parse(text: &str) -> Method {
        let id: String = text
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| if c == ' ' || c == '-' { '_' } else { c })
            .collect();
        match id.as_str() {
            "hereditary"
            | "hereditary_primogeniture"
            | "primogeniture"
            | "dynastic"
            | "dynastic_selection"
            | "dynastic_succession" => Method::Hereditary,
            "election" | "elected" | "member_vote" | "simple_vote" | "vote" => Method::Election,
            "appointment" | "appointed" | "nomination" => Method::Appointment,
            "seniority" | "eldest_member" => Method::Seniority,
            _ => Method::Unsupported,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Method::Hereditary => "succession héréditaire simple",
            Method::Election => "élection simple",
            Method::Appointment => "nomination par le pouvoir souverain",
            Method::Seniority => "désignation du doyen",
            Method::Unsupported => "règle non interprétée",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub kind: ProfileKind,
    pub blueprint: Blueprint,
    pub method: Method,
    pub builtin: bool,
}

fn builtin_blueprint(kind: ProfileKind, culture: &str) -> Blueprint {
    let en = culture == "british";
    let (label, title, succession) = match kind {
        ProfileKind::Monarchy => (
            "Monarchie",
            if en { "Sovereign" } else { "Souverain" },
            "hereditary",
        ),
        ProfileKind::Council => (
            "Conseil urbain",
            if en { "Lord Mayor" } else { "Premier consul" },
            "election",
        ),
        ProfileKind::League => (
            "Ligue marchande",
            if en {
                "Alderman"
            } else {
                "Prévôt des marchands"
            },
            "election",
        ),
        ProfileKind::Village => (
            "Communauté villageoise",
            if en { "Reeve" } else { "Syndic" },
            "election",
        ),
        ProfileKind::Informal => (
            "Collectif informel",
            if en { "Elder" } else { "Doyen" },
            "seniority",
        ),
    };
    Blueprint {
        id: format!("chronos-builtin-{}", kind.org_kind()),
        label: format!("{label} (profil générique Chronos)"),
        kind: kind.org_kind().into(),
        office_title: title.into(),
        succession: succession.into(),
        source: "chronos-builtin".into(),
        role_id: format!("{}-head", kind.org_kind()),
        max_holders: Some(1),
        office_title_en: None,
        collective_office: None,
        roles: Vec::new(),
    }
}

/// Picks one fixture blueprint per profile (exact `kind` identifier first,
/// then keywords); missing ones become
/// built-in profiles labelled as such.
pub fn select_profiles(blueprints: &[Blueprint], culture: &str) -> BTreeMap<ProfileKind, Profile> {
    let mut out = BTreeMap::new();
    let mut used = BTreeSet::new();
    for kind in ProfileKind::ALL {
        let exact = blueprints
            .iter()
            .find(|b| !used.contains(&b.id) && b.kind.trim().eq_ignore_ascii_case(kind.org_kind()));
        let found = exact.or_else(|| {
            blueprints.iter().find(|b| {
                if used.contains(&b.id) {
                    return false;
                }
                let text = format!("{} {} {}", b.kind, b.label, b.id).to_lowercase();
                kind.keywords().iter().any(|k| text.contains(k))
            })
        });
        let (blueprint, builtin) = match found {
            Some(b) => {
                used.insert(b.id.clone());
                (b.clone(), false)
            }
            None => (builtin_blueprint(kind, culture), true),
        };
        let method = Method::parse(&blueprint.succession);
        out.insert(
            kind,
            Profile {
                kind,
                blueprint,
                method,
                builtin,
            },
        );
    }
    out
}

// ---------------------------------------------------------------------------
// People registry

pub struct People {
    pub list: Vec<Person>,
    pub children: Vec<Vec<usize>>,
    /// Family name used for descendants (imported names are kept untouched).
    pub surname: Vec<String>,
    /// Recorded sex of imported people (empty for virtual people).
    sex: Vec<String>,
    pub index: BTreeMap<String, usize>,
    /// First and last years covered by the imported archive.
    pub import_start: Option<i32>,
    pub import_end: Option<i32>,
    next_virtual: usize,
    seed: u64,
    pools: Pools,
    /// Culture of each site, to name virtual people after their residence.
    site_culture: BTreeMap<String, String>,
    end_year: i32,
}

impl People {
    pub fn imported(&self, i: usize) -> bool {
        self.list[i].source == "lineage-lab"
    }

    /// Alive and observed by the record in year `y`. Imported people are only
    /// observed inside the archive coverage: before its start their roles and
    /// residence are unknown; after its end, without a recorded death, so is
    /// their survival. Their recorded dates are never changed.
    pub fn alive(&self, i: usize, y: i32) -> bool {
        let p = &self.list[i];
        if p.birth_year > y || (self.imported(i) && self.import_start.is_some_and(|s| y < s)) {
            return false;
        }
        match p.death_year {
            Some(d) => y <= d,
            None if self.imported(i) => self.import_end.is_none_or(|end| y <= end),
            None => true,
        }
    }

    /// Alive, adult, and not dying before the event can involve them.
    pub fn active_adult(&self, i: usize, y: i32) -> bool {
        self.alive(i, y)
            && y - self.list[i].birth_year >= ADULT_AGE
            && self.list[i].death_year.is_none_or(|d| y < d)
    }

    pub fn has_parent_in_record(&self, i: usize) -> bool {
        let p = &self.list[i];
        [&p.father_id, &p.mother_id]
            .iter()
            .any(|id| id.as_ref().is_some_and(|id| self.index.contains_key(id)))
    }

    fn virtual_female(&self, id: &str) -> bool {
        key(self.seed, &["virtual-sex", id]).is_multiple_of(2)
    }

    /// Recorded sex for imported people, stable key for virtual ones.
    fn is_female(&self, i: usize) -> bool {
        match self.sex.get(i).map(String::as_str) {
            Some("F") | Some("f") => true,
            Some("M") | Some("m") => false,
            _ => self.virtual_female(&self.list[i].id),
        }
    }

    /// Creates a virtual person (explicit fallback) who is alive in `alive_in`.
    pub fn add_virtual(
        &mut self,
        site_id: &str,
        birth_year: i32,
        alive_in: i32,
        surname: Option<String>,
        parent: Option<usize>,
    ) -> usize {
        self.next_virtual += 1;
        let id = format!("vp-{:04}", self.next_virtual);
        let life = range_i(self.seed, &["virtual-life", &id], 38, 78) as i32;
        let death = (birth_year + life)
            .max(alive_in + 1 + range_i(self.seed, &["virtual-extra-life", &id], 0, 20) as i32);
        let culture = self
            .site_culture
            .get(site_id)
            .map(String::as_str)
            .unwrap_or("");
        let pool = self.pools.get(culture);
        let surname = surname.unwrap_or_else(|| {
            pick(&pool.surnames, self.seed, &["virtual-surname", &id]).to_string()
        });
        let sex = if self.virtual_female(&id) { "F" } else { "M" };
        let forename = pick(
            pool.forenames_for(sex),
            self.seed,
            &["virtual-forename", &id],
        )
        .to_string();
        let (father_id, mother_id) = match parent {
            Some(p) if self.is_female(p) => (None, Some(self.list[p].id.clone())),
            Some(p) => (Some(self.list[p].id.clone()), None),
            None => (None, None),
        };
        let i = self.list.len();
        self.list.push(Person {
            id: id.clone(),
            name: format!("{forename} {surname}"),
            birth_year,
            death_year: if death < self.end_year {
                Some(death)
            } else {
                None
            },
            site_id: site_id.to_string(),
            father_id,
            mother_id,
            role: String::new(),
            source: "virtual".into(),
            external_id: None,
        });
        self.children.push(Vec::new());
        self.surname.push(surname);
        self.sex.push(String::new());
        self.index.insert(id, i);
        if let Some(p) = parent {
            self.children[p].push(i);
        }
        i
    }

    /// Gives a virtual ruler a few virtual children so hereditary rules can apply.
    pub fn add_virtual_children(&mut self, parent: usize) {
        if !self.children[parent].is_empty() || self.imported(parent) {
            return;
        }
        let pid = self.list[parent].id.clone();
        let count = range_i(self.seed, &["virtual-children", &pid], 1, 4);
        let (birth, site, surname) = (
            self.list[parent].birth_year,
            self.list[parent].site_id.clone(),
            self.surname[parent].clone(),
        );
        for k in 0..count {
            let offset = 20
                + 3 * k as i32
                + range_i(
                    self.seed,
                    &["virtual-child-gap", &pid, &k.to_string()],
                    0,
                    2,
                ) as i32;
            self.add_virtual(
                &site,
                birth + offset,
                birth + offset,
                Some(surname.clone()),
                Some(parent),
            );
        }
    }
}

pub struct PeopleImport {
    pub people: People,
    pub warnings: Vec<String>,
}

/// Imports fixture people as-is (dates and parents unchanged) and names them.
/// `coverage_end` is the last year the archive covers, when the fixture says so.
pub fn import_people(
    cfg: &Config,
    pools: &Pools,
    records: &[BootstrapPerson],
    sites: &[Site],
    coverage: (Option<i32>, Option<i32>),
) -> PeopleImport {
    let (coverage_start, coverage_end) = coverage;
    let mut warnings = Vec::new();
    let mut seen = BTreeSet::new();
    let mut kept: Vec<&BootstrapPerson> = Vec::new();
    let mut invalid = 0;
    for r in records {
        if r.id.is_empty()
            || r.id.starts_with("vp-")
            || !seen.insert(r.id.as_str())
            || r.death_year.is_some_and(|d| d < r.birth_year)
        {
            invalid += 1;
            continue;
        }
        kept.push(r);
    }
    if invalid > 0 {
        warnings.push(format!(
            "{invalid} personne(s) importée(s) ignorée(s) : ID vide, réservé ou dupliqué, ou décès avant naissance."
        ));
    }
    let index: BTreeMap<String, usize> = kept
        .iter()
        .enumerate()
        .map(|(i, r)| (r.id.clone(), i))
        .collect();
    let mut children = vec![Vec::new(); kept.len()];
    for (i, r) in kept.iter().enumerate() {
        for parent in [&r.father_id, &r.mother_id].into_iter().flatten() {
            if let Some(&p) = index.get(parent) {
                children[p].push(i);
            }
        }
    }
    for list in &mut children {
        list.sort_by_key(|&c| (kept[c].birth_year, kept[c].id.clone()));
    }
    // Family name keyed on the earliest recorded ancestor (father line first).
    let root = |mut i: usize| -> usize {
        for _ in 0..64 {
            let r = kept[i];
            match [&r.father_id, &r.mother_id]
                .into_iter()
                .flatten()
                .find_map(|p| index.get(p))
            {
                Some(&p) => i = p,
                None => break,
            }
        }
        i
    };
    let site_of = |r: &BootstrapPerson| &sites[r.site_index % sites.len()];
    let mut remapped = 0;
    let mut list = Vec::with_capacity(kept.len());
    let mut surnames = Vec::with_capacity(kept.len());
    for (i, r) in kept.iter().enumerate() {
        let ancestor = kept[root(i)];
        // The whole family is named from the pool of the ancestor's residence.
        let pool = pools.get(&site_of(ancestor).culture_id);
        let surname =
            pick(&pool.surnames, cfg.seed, &["family", &ancestor.external_id]).to_string();
        let name = match &r.name {
            Some(n) if !n.trim().is_empty() => n.clone(),
            _ => format!(
                "{} {surname}",
                pick(
                    pool.forenames_for(&r.sex),
                    cfg.seed,
                    &["forename", &r.external_id]
                )
            ),
        };
        if r.site_index >= sites.len() {
            remapped += 1;
        }
        list.push(Person {
            id: r.id.clone(),
            name,
            birth_year: r.birth_year,
            death_year: r.death_year,
            site_id: site_of(r).id.clone(),
            father_id: r.father_id.clone(),
            mother_id: r.mother_id.clone(),
            role: String::new(),
            source: "lineage-lab".into(),
            external_id: Some(r.external_id.clone()),
        });
        surnames.push(surname);
    }
    if remapped > 0 {
        warnings.push(format!(
            "{remapped} personne(s) importée(s) avec un indice d'établissement ≥ {} : résidence ramenée par modulo (adaptation).",
            sites.len()
        ));
    }
    let last_fact = list
        .iter()
        .map(|p| p.death_year.unwrap_or(p.birth_year))
        .max();
    let import_end = match (coverage_end, last_fact) {
        (Some(end), _) => Some(end),
        (None, Some(last)) => {
            warnings.push(format!(
                "Couverture de l'archive généalogique non fournie : fin estimée au dernier fait daté ({last}) ; au-delà, une personne sans décès enregistré n'est plus considérée."
            ));
            Some(last)
        }
        (None, None) => None,
    };
    if let Some(end) = import_end.filter(|_| !list.is_empty()) {
        let first = list.iter().map(|p| p.birth_year).min().unwrap_or(end);
        if end < cfg.start_year || first >= cfg.end_year() {
            warnings.push(format!(
                "L'échantillon Lineage Lab couvre {first}–{end}, hors de la fenêtre {}–{} : personnes virtuelles utilisées.",
                cfg.start_year,
                cfg.end_year() - 1
            ));
        }
    }
    PeopleImport {
        people: People {
            index: list
                .iter()
                .enumerate()
                .map(|(i, p)| (p.id.clone(), i))
                .collect(),
            list,
            children,
            surname: surnames,
            sex: kept.iter().map(|r| r.sex.clone()).collect(),
            import_start: coverage_start,
            import_end,
            next_virtual: 0,
            seed: cfg.seed,
            pools: pools.clone(),
            site_culture: sites
                .iter()
                .map(|s| (s.id.clone(), s.culture_id.clone()))
                .collect(),
            end_year: cfg.end_year(),
        },
        warnings,
    }
}

// ---------------------------------------------------------------------------
// Polities and organizations

pub struct Society {
    pub polities: Vec<Polity>,
    /// Polity index of each site at `start_year`.
    pub site_polity: Vec<usize>,
    pub organizations: Vec<Organization>,
    /// Profile of each organization.
    pub org_profile: Vec<ProfileKind>,
    /// Index of the government organization of each polity.
    pub government: Vec<usize>,
    pub league: Option<usize>,
}

pub fn is_village(site: &Site) -> bool {
    let k = site.kind.to_lowercase();
    k.contains("village") || k.contains("hamlet") || k.contains("hameau")
}

pub fn build_society(
    cfg: &Config,
    sites: &[Site],
    graph: &Graph,
    profiles: &BTreeMap<ProfileKind, Profile>,
) -> Society {
    let n = sites.len();
    let count = (n / 30).clamp(2, 6).min(n);
    // Capitals: most populous site, then farthest-point sampling weighted by size.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        sites[b]
            .initial_population
            .cmp(&sites[a].initial_population)
            .then(a.cmp(&b))
    });
    let mut capitals = vec![order[0]];
    while capitals.len() < count {
        let best = (0..n)
            .filter(|i| !capitals.contains(i))
            .map(|i| {
                let d = capitals
                    .iter()
                    .map(|&c| {
                        ((sites[i].x - sites[c].x).powi(2) + (sites[i].y - sites[c].y).powi(2))
                            .sqrt()
                    })
                    .fold(f64::INFINITY, f64::min);
                (d * (sites[i].initial_population as f64).sqrt(), i)
            })
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
        match best {
            Some((_, i)) => capitals.push(i),
            None => break,
        }
    }
    let (_, owner) = graph.dijkstra(&capitals);
    let site_polity: Vec<usize> = (0..n)
        .map(|i| {
            owner[i].unwrap_or_else(|| {
                // Isolated site: nearest capital in straight line (no route invented).
                (0..capitals.len())
                    .min_by(|&a, &b| {
                        let da = (sites[i].x - sites[capitals[a]].x).powi(2)
                            + (sites[i].y - sites[capitals[a]].y).powi(2);
                        let db = (sites[i].x - sites[capitals[b]].x).powi(2)
                            + (sites[i].y - sites[capitals[b]].y).powi(2);
                        da.total_cmp(&db)
                    })
                    .unwrap_or(0)
            })
        })
        .collect();

    let gov_kind = |p: usize| match cfg.institution_mode.as_str() {
        "monarchy" => ProfileKind::Monarchy,
        "council" => ProfileKind::Council,
        _ if p < 2 => ProfileKind::Monarchy,
        _ if p.is_multiple_of(2) => ProfileKind::Council,
        _ => ProfileKind::Monarchy,
    };
    // Wording follows the culture of the site an entity is named after.
    let fr_at = |i: usize| resolve_culture(&sites[i].culture_id, &cfg.culture) != "british";
    let mut polities = Vec::new();
    let mut organizations = Vec::new();
    let mut org_profile = Vec::new();
    let mut government = Vec::new();
    for (p, &cap) in capitals.iter().enumerate() {
        let kind = gov_kind(p);
        let profile = &profiles[&kind];
        let fr = fr_at(cap);
        let cap_name = &sites[cap].name;
        let name = match (kind, fr) {
            (ProfileKind::Monarchy, true) => format!("Royaume de {cap_name}"),
            (ProfileKind::Monarchy, false) => format!("Kingdom of {cap_name}"),
            (_, true) => format!("République de {cap_name}"),
            (_, false) => format!("Commonwealth of {cap_name}"),
        };
        let id = format!("pol-{}", p + 1);
        let name = elide(&name);
        polities.push(Polity {
            id: id.clone(),
            name: name.clone(),
            color: COLORS[p % COLORS.len()].into(),
            capital_id: sites[cap].id.clone(),
            blueprint_id: profile.blueprint.id.clone(),
        });
        let mut member_sites: Vec<String> = vec![sites[cap].id.clone()];
        member_sites.extend(
            (0..n)
                .filter(|&i| site_polity[i] == p && i != cap)
                .map(|i| sites[i].id.clone()),
        );
        government.push(organizations.len());
        organizations.push(org(
            format!("org-gov-{}", p + 1),
            if kind == ProfileKind::Monarchy {
                if fr {
                    format!("Couronne de {cap_name}")
                } else {
                    format!("Crown of {cap_name}")
                }
            } else if fr {
                format!("Conseil de {cap_name}")
            } else {
                format!("Council of {cap_name}")
            },
            profile,
            &id,
            member_sites,
            fr,
        ));
        org_profile.push(kind);
    }

    // Village communities: one federation per polity.
    for (p, pol) in polities.iter().enumerate() {
        let villages: Vec<String> = (0..n)
            .filter(|&i| site_polity[i] == p && is_village(&sites[i]))
            .map(|i| sites[i].id.clone())
            .collect();
        if villages.is_empty() {
            continue;
        }
        let fr = fr_at(sites.iter().position(|s| s.id == villages[0]).unwrap_or(0));
        let first = &sites
            .iter()
            .find(|s| s.id == villages[0])
            .map(|s| s.name.clone())
            .unwrap_or_default();
        let name = if fr {
            format!("Communautés rurales autour de {first}")
        } else {
            format!("Village communities around {first}")
        };
        organizations.push(org(
            format!("org-village-{}", p + 1),
            name,
            &profiles[&ProfileKind::Village],
            &pol.id,
            villages,
            fr,
        ));
        org_profile.push(ProfileKind::Village);
    }

    // Merchant league spanning the largest towns of every polity.
    let mut league_sites = Vec::new();
    for p in 0..polities.len() {
        league_sites.extend(
            order
                .iter()
                .copied()
                .filter(|&i| site_polity[i] == p)
                .take(2),
        );
    }
    league_sites.sort_by_key(|&i| order.iter().position(|&o| o == i));
    league_sites.truncate(6);
    let league = if league_sites.len() >= 2 {
        let seat = league_sites[0];
        let name = if fr_at(seat) {
            format!(
                "Ligue de {} et {}",
                sites[league_sites[0]].name, sites[league_sites[1]].name
            )
        } else {
            format!(
                "League of {} and {}",
                sites[league_sites[0]].name, sites[league_sites[1]].name
            )
        };
        organizations.push(org(
            "org-league-1".into(),
            name,
            &profiles[&ProfileKind::League],
            &polities[site_polity[seat]].id,
            league_sites.iter().map(|&i| sites[i].id.clone()).collect(),
            fr_at(seat),
        ));
        org_profile.push(ProfileKind::League);
        Some(organizations.len() - 1)
    } else {
        None
    };

    // One informal collective in a mid-sized site.
    if n >= 4 {
        let i = order[(n / 4).max(1)];
        let name = if fr_at(i) {
            format!("Confrérie de {}", sites[i].name)
        } else {
            format!("Fellowship of {}", sites[i].name)
        };
        organizations.push(org(
            "org-collective-1".into(),
            name,
            &profiles[&ProfileKind::Informal],
            &polities[site_polity[i]].id,
            vec![sites[i].id.clone()],
            fr_at(i),
        ));
        org_profile.push(ProfileKind::Informal);
    }

    Society {
        polities,
        site_polity,
        organizations,
        org_profile,
        government,
        league,
    }
}

fn org(
    id: String,
    name: String,
    profile: &Profile,
    polity_id: &str,
    site_ids: Vec<String>,
    fr: bool,
) -> Organization {
    let title = match (&profile.blueprint.office_title_en, fr) {
        (Some(en), false) if !en.is_empty() => en.clone(),
        _ => profile.blueprint.office_title.clone(),
    };
    Organization {
        id,
        name: elide(&name),
        kind: profile.kind.org_kind().into(),
        polity_id: polity_id.into(),
        site_ids,
        blueprint_id: profile.blueprint.id.clone(),
        office_title: title,
        succession: profile.blueprint.succession.clone(),
        holder_id: None,
        collective_office: profile.blueprint.is_collective(),
    }
}
