//! Person and place labels from the frozen pools (§23.3). Every draw here has its own
//! key and feeds labels only: no fact reads a name, and a name is never an identity.
//!
//! Each culture carries an inheritance rule in the pools file (`inheritance`): the family
//! part of a name is patrilineal, matrilineal, a patronymic built from the father's
//! forename, or absent (given name only). The rule reads the table's parent links and
//! semantic keys, never the query order.
use super::attributes::{local_key, PersonAttributes};
use super::general::{GeneralCanon, Polity};
use super::ids::random_semantics_v2::uniform_below;
use super::model::{HistoryError, NameTree, ProvenanceStatus};
use super::naming::{self, NameContext, RenderOptions};
use super::population::CandidateTable;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

pub const POOLS_BYTES: &[u8] = include_bytes!("../../../../fixtures/naming/person-pools.v1.json");
pub const DISPLAY_NAME_RULE: &str = "display-names/v1";
pub const LANGUAGES: [&str; 2] = ["en", "fr"];
const MAX_PARENT_WALK: usize = 64;
const FEMALE: u8 = 1;

/// How a culture gives a person a family part (data: `inheritance_rules` in the pools file).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InheritanceRule {
    #[default]
    Patrilineal,
    Matrilineal,
    Patronymic,
    GivenOnly,
}
/// Patronymic templates: `{f}` is the father's forename. `stem_rules` are
/// `[ending, replacement, male suffix, female suffix]`; the first matching ending wins.
#[derive(Clone, Debug, Deserialize)]
pub struct PatronymicForm {
    pub male: String,
    pub female: String,
    #[serde(default)]
    pub stem_rules: Vec<(String, String, String, String)>,
}
impl Default for PatronymicForm {
    fn default() -> Self {
        Self {
            male: "{f}".into(),
            female: "{f}".into(),
            stem_rules: Vec::new(),
        }
    }
}
impl PatronymicForm {
    fn build(&self, father: &str, female: bool) -> String {
        for (end, replacement, male, fem) in &self.stem_rules {
            if let Some(stem) = father.strip_suffix(end.as_str()) {
                let suffix = if female { fem } else { male };
                return format!("{stem}{replacement}{suffix}");
            }
        }
        let template = if female { &self.female } else { &self.male };
        template.replace("{f}", father)
    }
}
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Inheritance {
    #[serde(default)]
    pub rule: InheritanceRule,
    /// `family_first` prints the family part before the forename.
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub form: Option<PatronymicForm>,
}

#[derive(Debug, Deserialize)]
pub struct CulturePool {
    pub forenames_female: Vec<String>,
    pub forenames_male: Vec<String>,
    pub surnames: Vec<String>,
    pub places: Vec<String>,
    pub provenance: BTreeMap<String, String>,
    #[serde(default)]
    pub inheritance: Inheritance,
}
#[derive(Debug, Deserialize)]
pub struct PersonPools {
    pub content_sha256: String,
    pub cultures: BTreeMap<String, CulturePool>,
}
pub fn person_pools() -> &'static PersonPools {
    static POOLS: OnceLock<PersonPools> = OnceLock::new();
    POOLS.get_or_init(|| serde_json::from_slice(POOLS_BYTES).expect("frozen person pools"))
}
/// Fingerprint of the pools, recorded in the world manifest.
pub fn pools_fingerprint() -> &'static str {
    &person_pools().content_sha256
}
impl CulturePool {
    fn status(&self, list: &str) -> ProvenanceStatus {
        match self.provenance.get(list).map(String::as_str) {
            Some("imported") => ProvenanceStatus::Imported,
            _ => ProvenanceStatus::Derived,
        }
    }
}
fn pick<'a>(list: &'a [String], parts: &[&str], seed: u64) -> Result<&'a str, HistoryError> {
    let index = uniform_below(seed, parts, 0, list.len().max(1) as u64)? as usize;
    Ok(list.get(index).map_or("", String::as_str))
}
fn surname_of(culture: &str, key: &str, seed: u64) -> Result<Option<String>, HistoryError> {
    let Some(pool) = person_pools().cultures.get(culture) else {
        return Ok(None);
    };
    let parts = [DISPLAY_NAME_RULE, "surname", culture, key];
    Ok(Some(pick(&pool.surnames, &parts, seed)?.to_owned()))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersonName {
    pub forename: String,
    /// Family part as printed: a line surname, a patronymic, or empty (given name only).
    pub surname: String,
    /// Name of the person's house, drawn from the line's pool: labels families and groups,
    /// whatever the culture prints on the person.
    pub house: String,
    pub family_first: bool,
    pub culture_id: String,
    pub status: ProvenanceStatus,
}
impl PersonName {
    pub fn text(&self) -> String {
        let (first, second) = if self.family_first {
            (&self.surname, &self.forename)
        } else {
            (&self.forename, &self.surname)
        };
        match (first.is_empty(), second.is_empty()) {
            (_, true) => first.clone(),
            (true, false) => second.clone(),
            _ => format!("{first} {second}"),
        }
    }
}
/// Oldest known ancestor by the father (else the mother), or by the mother first. The
/// table stops at its genealogical limit, so a surname runs as far as the family links do.
fn line_root(table: &CandidateTable, slot: usize, maternal: bool) -> usize {
    let mut current = slot;
    for _ in 0..MAX_PARENT_WALK {
        let next = if maternal {
            table.mother_slot[current].or(table.father_slot[current])
        } else {
            table.father_slot[current].or(table.mother_slot[current])
        };
        match next {
            Some(parent) => current = parent as usize,
            None => break,
        }
    }
    current
}
fn forename_list(pool: &CulturePool, female: bool) -> (&[String], &'static str) {
    if female {
        (&pool.forenames_female, "forenames_female")
    } else {
        (&pool.forenames_male, "forenames_male")
    }
}
/// The forename of a slot, from its culture's pool and its own origin key.
fn forename_at(
    world_seed: u64,
    domain_id: &str,
    table: &CandidateTable,
    people: &[PersonAttributes],
    slot: usize,
) -> Result<Option<String>, HistoryError> {
    let Some(pool) = person_pools().cultures.get(&people[slot].culture_id) else {
        return Ok(None);
    };
    let (list, _) = forename_list(pool, table.sex[slot] == FEMALE);
    let origin = local_key(&table.origin_id[slot], domain_id);
    let parts = [DISPLAY_NAME_RULE, "forename", origin.as_str()];
    Ok(Some(pick(list, &parts, world_seed)?.to_owned()))
}
/// The father's forename for a patronymic: the real father's when the table has him, else
/// a keyed draw from the male forenames, shared by the siblings of one family key.
fn father_forename(
    world_seed: u64,
    domain_id: &str,
    table: &CandidateTable,
    people: &[PersonAttributes],
    pool: &CulturePool,
    slot: usize,
) -> Result<String, HistoryError> {
    if let Some(father) = table.father_slot[slot] {
        if let Some(name) = forename_at(world_seed, domain_id, table, people, father as usize)? {
            return Ok(name);
        }
    }
    let family = local_key(&table.family_key[slot], domain_id);
    let parts = [DISPLAY_NAME_RULE, "patronym-father", family.as_str()];
    Ok(pick(&pool.forenames_male, &parts, world_seed)?.to_owned())
}
/// One name per slot; `None` when the culture has no pool. The forename follows the
/// person key; the family part follows the culture's inheritance rule and the family key
/// of the line's root, in the root's culture.
pub fn person_names(
    world_seed: u64,
    domain_id: &str,
    table: &CandidateTable,
    people: &[PersonAttributes],
) -> Result<Vec<Option<PersonName>>, HistoryError> {
    let mut houses: BTreeMap<usize, Option<String>> = BTreeMap::new();
    let mut house_of = |root: usize| -> Result<Option<String>, HistoryError> {
        if let Some(known) = houses.get(&root) {
            return Ok(known.clone());
        }
        let key = local_key(&table.family_key[root], domain_id);
        let drawn = surname_of(&people[root].culture_id, &key, world_seed)?;
        houses.insert(root, drawn.clone());
        Ok(drawn)
    };
    let mut names = Vec::with_capacity(table.len());
    for (slot, person) in people.iter().enumerate() {
        let Some(pool) = person_pools().cultures.get(&person.culture_id) else {
            names.push(None);
            continue;
        };
        let female = table.sex[slot] == FEMALE;
        let rule = pool.inheritance.rule;
        let forename = forename_at(world_seed, domain_id, table, people, slot)?.unwrap_or_default();
        let (surname, house, family_status) = match rule {
            InheritanceRule::Patrilineal | InheritanceRule::Matrilineal => {
                let root = line_root(table, slot, rule == InheritanceRule::Matrilineal);
                let Some(line) = house_of(root)? else {
                    names.push(None);
                    continue;
                };
                (line.clone(), line, pool.status("surnames"))
            }
            InheritanceRule::Patronymic => {
                let father = father_forename(world_seed, domain_id, table, people, pool, slot)?;
                let form = pool.inheritance.form.clone().unwrap_or_default();
                let house = house_of(line_root(table, slot, false))?.unwrap_or_default();
                (
                    form.build(&father, female),
                    house,
                    ProvenanceStatus::Imported,
                )
            }
            InheritanceRule::GivenOnly => {
                let house = house_of(line_root(table, slot, false))?.unwrap_or_default();
                (String::new(), house, ProvenanceStatus::Imported)
            }
        };
        let (_, list_name) = forename_list(pool, female);
        let imported = pool.status(list_name) == ProvenanceStatus::Imported
            && family_status == ProvenanceStatus::Imported;
        names.push(Some(PersonName {
            forename,
            surname,
            house,
            family_first: pool.inheritance.order.as_deref() == Some("family_first"),
            culture_id: person.culture_id.clone(),
            status: if imported {
                ProvenanceStatus::Imported
            } else {
                ProvenanceStatus::Derived
            },
        }));
    }
    Ok(names)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlaceLabel {
    pub text: String,
    pub culture_id: String,
    pub status: ProvenanceStatus,
}
/// Real name of each site, region and polity, by ID. Sites and regions take distinct
/// pool entries in ID order; a polity carries the name of its seat.
pub fn place_labels(canon: &GeneralCanon) -> Result<BTreeMap<String, PlaceLabel>, HistoryError> {
    let start = canon.timeline.start_year;
    let mut used = BTreeSet::new();
    let mut labels = BTreeMap::new();
    let mut label = |id: &str, culture: &str| -> Result<Option<PlaceLabel>, HistoryError> {
        let Some(pool) = person_pools().cultures.get(culture) else {
            return Ok(None);
        };
        let size = pool.places.len().max(1) as u64;
        let parts = [DISPLAY_NAME_RULE, "place", id];
        let first = uniform_below(canon.world_seed, &parts, 0, size)? as usize;
        let free = (0..pool.places.len())
            .map(|i| &pool.places[(first + i) % pool.places.len()])
            .find(|name| !used.contains(*name));
        Ok(free.map(|text| {
            used.insert(text.clone());
            PlaceLabel {
                text: text.clone(),
                culture_id: culture.to_owned(),
                status: pool.status("places"),
            }
        }))
    };
    let mut sites: Vec<_> = canon.atlas.settlements.iter().collect();
    sites.sort_by(|a, b| a.site_id.cmp(&b.site_id));
    let principal = |site: &super::atlas::SettlementAnchor| {
        let culture = site.episode_at(start).and_then(|e| {
            e.culture_weights
                .iter()
                .max_by_key(|(id, w)| (**w, std::cmp::Reverse(id.as_str())))
        });
        culture.map_or(String::new(), |(id, _)| id.clone())
    };
    for site in &sites {
        if let Some(found) = label(&site.site_id, &principal(site))? {
            labels.insert(site.site_id.clone(), found);
        }
    }
    for region in canon.baselines.keys() {
        let first = sites
            .iter()
            .find(|s| canon.atlas.region_of(s) == Some(region.as_str()));
        if let Some(found) = label(region, &first.map_or(String::new(), |s| principal(s)))? {
            labels.insert(region.clone(), found);
        }
    }
    for polity in &canon.polities {
        if let Some(seat) = labels.get(&polity.seat_site_id).cloned() {
            labels.insert(polity.entity.id.clone(), seat);
        }
    }
    Ok(labels)
}
/// Kind of the Namebench pattern that names a polity ("Realm of {seat}" and the like).
pub const POLITY_NAME_KIND: &str = "organization.community.nation";
/// Name of a polity in each language, from the pack's nation patterns with the seat as
/// the place; `None` when the culture or the pack cannot name it, so that the caller
/// keeps the declared fallback. The draw is keyed by the polity's identity.
pub fn polity_name(
    canon: &GeneralCanon,
    labels: &BTreeMap<String, PlaceLabel>,
    polity: &Polity,
) -> Result<Option<BTreeMap<String, String>>, HistoryError> {
    let Some(label) = labels.get(&polity.entity.id) else {
        return Ok(None);
    };
    let context = NameContext {
        refs: BTreeMap::from([
            ("polity".into(), polity.entity.id.clone()),
            ("site".into(), polity.seat_site_id.clone()),
            ("region".into(), polity.region_id.clone()),
        ]),
        activity: None,
        tone: Some("ordinary".into()),
    };
    let Some(tree) = naming::expand_keyed(
        naming::frozen_pack(),
        POLITY_NAME_KIND,
        &label.culture_id,
        &context,
        canon.world_seed,
        &polity.entity.id,
    )?
    else {
        return Ok(None);
    };
    render_tree(&tree, labels, &BTreeMap::new(), canon.world_seed, "")
}
/// Text of a name tree in each language. A `ref` part is a real label: a place, or a
/// surname of the pool of the tree's culture under its own key. `None` when it cannot
/// render, so that the caller shows the declared fallback.
pub fn render_tree(
    tree: &NameTree,
    labels: &BTreeMap<String, PlaceLabel>,
    surnames: &BTreeMap<String, String>,
    world_seed: u64,
    domain_id: &str,
) -> Result<Option<BTreeMap<String, String>>, HistoryError> {
    let value = tree.value();
    let culture = value["culture"].as_str().unwrap_or_default();
    let mut refs = BTreeMap::new();
    for part in value["parts"]
        .as_object()
        .into_iter()
        .flat_map(|p| p.values())
    {
        if part["type"] != "ref" {
            continue;
        }
        let id = part["id"].as_str().unwrap_or_default();
        let text = match (labels.get(id), surnames.get(id)) {
            (Some(label), _) => Some(label.text.clone()),
            (None, Some(surname)) => Some(surname.clone()),
            _ => surname_of(culture, &local_key(id, domain_id), world_seed)?,
        };
        if let Some(text) = text {
            refs.insert(id.to_owned(), text);
        }
    }
    let pack = naming::frozen_pack();
    let mut texts = BTreeMap::new();
    for language in LANGUAGES {
        let options = RenderOptions {
            target: language.into(),
            ..RenderOptions::default()
        };
        match naming::render(pack, tree, &refs, &options) {
            Ok(result) => texts.insert(language.to_owned(), result.text),
            Err(_) => return Ok(None),
        };
    }
    Ok(Some(texts))
}
/// The same text for every viewer: a proper name is not translated.
pub fn same_text(text: &str) -> Value {
    LANGUAGES
        .iter()
        .map(|l| ((*l).to_owned(), json!(text)))
        .collect()
}
/// Where a label comes from, shown next to it (§4.3).
pub fn name_provenance(status: ProvenanceStatus, rule: &str) -> Value {
    json!({
        "status": status,
        "source": format!("person-pools:{}", &pools_fingerprint()[..12]),
        "rule_id": rule,
    })
}
