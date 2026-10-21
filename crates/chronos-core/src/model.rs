//! Shared JSON contract: configuration, exported world and bootstrap fixture.
//!
//! JSON contract shared with the web client (`web/`) and the data tools
//! (`tools/`): snake_case keys, string IDs. Public description: README.md
//! ("API HTTP"), docs/methods/chronology.md and docs/data-sources.md. Do not
//! rename fields without updating both consumers.

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;
/// Version of the generation rules. Part of every canon identity.
pub const ALGORITHM_VERSION: &str = "chronos-history-1";
/// Largest integer a JavaScript number represents exactly.
pub const MAX_JS_SAFE_INTEGER: u64 = (1 << 53) - 1;
/// Bound on `years * sites` so snapshots stay small enough for a browser.
pub const MAX_SITE_YEARS: u64 = 150_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub seed: u64,
    pub start_year: i32,
    pub years: u32,
    pub site_count: usize,
    pub population: u64,
    pub world_radius_km: f64,
    pub map_mode: String,
    pub culture: String,
    pub war_rate: f64,
    pub epidemic_rate: f64,
    pub trade_intensity: f64,
    pub migration_rate: f64,
    pub growth_rate: f64,
    pub resource_abundance: f64,
    pub travel_km_per_day: f64,
    pub institution_mode: String,
    /// `pivot`: demo scenario (1354–1360 epidemic, war and hero around 1398)
    /// when the window covers it; `free`: only rate-driven events.
    pub scenario: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            seed: 42,
            start_year: 1300,
            years: 200,
            site_count: 50,
            population: 120_000,
            world_radius_km: 6371.0,
            map_mode: "stack".into(),
            culture: "french".into(),
            war_rate: 0.35,
            epidemic_rate: 0.30,
            trade_intensity: 0.60,
            migration_rate: 0.25,
            growth_rate: 0.003,
            resource_abundance: 1.0,
            travel_km_per_day: 25.0,
            institution_mode: "mixed".into(),
            scenario: "pivot".into(),
        }
    }
}

impl Config {
    /// Exclusive end year of the chronicle.
    pub fn end_year(&self) -> i32 {
        self.start_year + self.years as i32
    }

    /// Checks bounds; the error message is meant to be shown to the user.
    pub fn validate(&self) -> Result<(), String> {
        fn range(name: &str, v: f64, lo: f64, hi: f64) -> Result<(), String> {
            if !v.is_finite() || v < lo || v > hi {
                return Err(format!(
                    "{name} doit être un nombre entre {lo} et {hi} (reçu {v})"
                ));
            }
            Ok(())
        }
        if self.seed > MAX_JS_SAFE_INTEGER {
            return Err(format!(
                "seed doit être ≤ {MAX_JS_SAFE_INTEGER} (entier sûr en JavaScript)"
            ));
        }
        if !(-5000..=5000).contains(&self.start_year) {
            return Err("start_year doit être compris entre -5000 et 5000".into());
        }
        if !(1..=2000).contains(&self.years) {
            return Err("years doit être compris entre 1 et 2000".into());
        }
        if !(8..=300).contains(&self.site_count) {
            return Err("site_count doit être compris entre 8 et 300".into());
        }
        if self.years as u64 * self.site_count as u64 > MAX_SITE_YEARS {
            return Err(format!(
                "years × site_count ne doit pas dépasser {MAX_SITE_YEARS} (instantanés annuels) : réduisez la durée ou le nombre de sites"
            ));
        }
        if !(1_000..=50_000_000).contains(&self.population) {
            return Err("population doit être comprise entre 1 000 et 50 000 000".into());
        }
        range("world_radius_km", self.world_radius_km, 100.0, 100_000.0)?;
        range("war_rate", self.war_rate, 0.0, 1.0)?;
        range("epidemic_rate", self.epidemic_rate, 0.0, 1.0)?;
        range("trade_intensity", self.trade_intensity, 0.0, 1.0)?;
        range("migration_rate", self.migration_rate, 0.0, 1.0)?;
        range("growth_rate", self.growth_rate, -0.02, 0.03)?;
        range("resource_abundance", self.resource_abundance, 0.2, 5.0)?;
        range("travel_km_per_day", self.travel_km_per_day, 1.0, 500.0)?;
        if !["stack", "virtual"].contains(&self.map_mode.as_str()) {
            return Err("map_mode doit valoir \"stack\" ou \"virtual\"".into());
        }
        if !["french", "british", "fantasy"].contains(&self.culture.as_str()) {
            return Err("culture doit valoir \"french\", \"british\" ou \"fantasy\"".into());
        }
        if !["mixed", "monarchy", "council"].contains(&self.institution_mode.as_str()) {
            return Err(
                "institution_mode doit valoir \"mixed\", \"monarchy\" ou \"council\"".into(),
            );
        }
        if !["pivot", "free"].contains(&self.scenario.as_str()) {
            return Err("scenario doit valoir \"pivot\" ou \"free\"".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub schema_version: u32,
    pub canon_id: String,
    pub config: Config,
    /// Wall-clock generation time; not canonical, excluded from replay hashes.
    pub generation_ms: f64,
    pub map: MapData,
    pub sources: Vec<Source>,
    pub warnings: Vec<String>,
    pub sites: Vec<Site>,
    pub routes: Vec<Route>,
    pub polities: Vec<Polity>,
    pub organizations: Vec<Organization>,
    pub people: Vec<Person>,
    pub events: Vec<Event>,
    pub snapshots: Vec<Snapshot>,
    /// Optional (additive): first year covered by the imported genealogy.
    /// Imported people take no role in events before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genealogy_start_year: Option<i32>,
    /// Optional (additive): last year covered by the imported genealogy. An
    /// imported person with `death_year: null` is unknown after this year.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genealogy_end_year: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapData {
    pub width: usize,
    pub height: usize,
    /// Row-major, north row first.
    pub elevation_m: Vec<f64>,
    /// `[lon_min_deg, lat_min_deg, lon_max_deg, lat_max_deg]`.
    pub bounds: [f64; 4],
    pub radius_km: f64,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub label: String,
    /// `imported`, `derived`, `synthetic` or `unavailable`.
    pub status: String,
    pub detail: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Resources {
    pub food: f64,
    pub water: f64,
    pub timber: f64,
    pub iron: f64,
    pub stone: f64,
    pub salt: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub region_id: String,
    pub culture_id: String,
    pub kind: String,
    pub initial_population: u64,
    pub polity_id: String,
    pub resources: Resources,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub id: String,
    pub from: String,
    pub to: String,
    pub distance_km: f64,
    pub travel_days: f64,
    pub path: Vec<Vec<f64>>,
    pub kind: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Polity {
    pub id: String,
    pub name: String,
    pub color: String,
    pub capital_id: String,
    pub blueprint_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub polity_id: String,
    pub site_ids: Vec<String>,
    pub blueprint_id: String,
    pub office_title: String,
    pub succession: String,
    /// Holder at `start_year`; later holders are given by succession events.
    /// Always null for a collective office.
    pub holder_id: Option<String>,
    /// The office is held by a collective body (e.g. an assembly), not by one
    /// person: there is no personal holder and no succession.
    #[serde(default)]
    pub collective_office: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub birth_year: i32,
    pub death_year: Option<i32>,
    pub site_id: String,
    pub father_id: Option<String>,
    pub mother_id: Option<String>,
    pub role: String,
    /// `lineage-lab` or `virtual`.
    pub source: String,
    pub external_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub year: i32,
    /// Inclusive last year.
    pub end_year: i32,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub site_ids: Vec<String>,
    pub organization_ids: Vec<String>,
    pub person_ids: Vec<String>,
    /// IDs of earlier events.
    pub causes: Vec<String>,
    pub effects: Vec<Effect>,
    pub importance: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    pub site_id: String,
    pub population_delta: i64,
    pub food_delta: f64,
    pub wealth_delta: f64,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// State at the start of this year.
    pub year: i32,
    pub sites: Vec<SiteState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SiteState {
    pub site_id: String,
    pub population: u64,
    pub food: f64,
    pub wealth: f64,
    pub stability: f64,
    pub polity_id: String,
}

// ---------------------------------------------------------------------------
// Bootstrap fixture (`fixtures/stack.json`), prepared by the data tools.

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StackFixture {
    pub schema_version: u32,
    #[serde(default)]
    pub seed: u64,
    pub map: MapData,
    #[serde(default)]
    pub sites: Vec<BootstrapSite>,
    #[serde(default)]
    pub routes: Vec<Route>,
    #[serde(default)]
    pub names: NamePools,
    #[serde(default)]
    pub blueprints: Vec<Blueprint>,
    #[serde(default)]
    pub people: Vec<BootstrapPerson>,
    #[serde(default)]
    pub sources: Vec<Source>,
    /// Optional coverage of the genealogical archive (inclusive years). An
    /// imported person without a recorded death is only known until the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genealogy_start_year: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genealogy_end_year: Option<i32>,
    /// Alternative form of the coverage: `[first_year, last_year]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub people_window: Option<[i32; 2]>,
    /// Diagnostics written by the data-preparation tools.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl StackFixture {
    /// Last year covered by the genealogical archive, if the fixture states it.
    pub fn genealogy_end(&self) -> Option<i32> {
        self.genealogy_end_year.or(self.people_window.map(|w| w[1]))
    }

    pub fn genealogy_start(&self) -> Option<i32> {
        self.genealogy_start_year
            .or(self.people_window.map(|w| w[0]))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BootstrapSite {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub region_id: String,
    #[serde(default)]
    pub culture_id: String,
    #[serde(default)]
    pub kind: String,
    pub population: u64,
    #[serde(default)]
    pub resources: Resources,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NamePools {
    pub french: NamePool,
    pub british: NamePool,
    pub fantasy: NamePool,
}

impl NamePools {
    pub fn get(&self, culture: &str) -> &NamePool {
        match culture {
            "british" => &self.british,
            "fantasy" => &self.fantasy,
            _ => &self.french,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NamePool {
    pub forenames: Vec<String>,
    pub surnames: Vec<String>,
    pub places: Vec<String>,
    /// Optional gendered subsets of `forenames`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub forenames_female: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub forenames_male: Vec<String>,
}

impl NamePool {
    /// Forenames for a recorded sex ("F"/"M"), falling back to the full list.
    pub fn forenames_for(&self, sex: &str) -> &[String] {
        let list = match sex {
            "F" | "f" => &self.forenames_female,
            "M" | "m" => &self.forenames_male,
            _ => &self.forenames,
        };
        if list.is_empty() {
            &self.forenames
        } else {
            list
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Blueprint {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub office_title: String,
    pub succession: String,
    pub source: String,
    #[serde(default)]
    pub role_id: String,
    #[serde(default)]
    pub max_holders: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub office_title_en: Option<String>,
    /// Explicit flag; otherwise derived from the selected role in `roles`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collective_office: Option<bool>,
    /// Minimal view of the source roles (other role fields are ignored).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<BlueprintRole>,
}

impl Blueprint {
    /// Whether the selected office (`role_id`) is a collective body.
    pub fn is_collective(&self) -> bool {
        self.collective_office.unwrap_or_else(|| {
            self.roles
                .iter()
                .any(|r| r.id == self.role_id && r.collective)
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlueprintRole {
    pub id: String,
    #[serde(default)]
    pub collective: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BootstrapPerson {
    pub id: String,
    pub birth_year: i32,
    #[serde(default)]
    pub death_year: Option<i32>,
    #[serde(default)]
    pub father_id: Option<String>,
    #[serde(default)]
    pub mother_id: Option<String>,
    pub site_index: usize,
    #[serde(default)]
    pub sex: String,
    pub external_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
