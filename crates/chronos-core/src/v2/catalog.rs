//! Typed reading of the S1 culture catalogue (§6, §23, §27): only the sections
//! the canon needs. The catalogue carries no `demography_society_id`: the host
//! computes them (§26.3) and passes them in the world manifest.
use super::model::{
    string_enum, ConductThresholds, ErrorCode, HistoryError, LegalStatus, Visibility,
};
use super::population::{ParameterValue, SocietyProfile, SocietySchema};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};

string_enum!(SettingProfile {
    Historical,
    LowFantasy,
    HighFantasy
});

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CultureBinding {
    pub mu_culture_id: String,
    pub history_family_id: String,
    pub overlays: Vec<String>,
    pub demography_society_template_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SocietyTemplate {
    pub id: String,
    pub schema_id: SocietySchema,
    pub parameters: BTreeMap<String, ParameterValue>,
    pub status_weights: BTreeMap<String, u64>,
    pub core_status_weights: BTreeMap<String, u64>,
}
impl SocietyTemplate {
    /// Resolved profile under the ID computed by the host.
    pub fn resolved(&self, society_id: &str) -> SocietyProfile {
        SocietyProfile {
            id: society_id.into(),
            schema_id: self.schema_id,
            parameters: self.parameters.clone(),
            status_weights: self.status_weights.clone(),
            core_status_weights: self.core_status_weights.clone(),
        }
    }
}
/// One-decimal stock, read as integer tenths from the number's text: no float
/// enters a computation (§23.1).
fn tenths<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let text = serde_json::Number::deserialize(d)?.to_string();
    let (units, decimals) = text.split_once('.').unwrap_or((&text, "0"));
    match (units.parse::<u64>(), decimals.parse::<u64>()) {
        (Ok(units), Ok(tenth)) if decimals.len() == 1 => Ok(units * 10 + tenth),
        _ => Err(serde::de::Error::custom("catalogue.stock_not_in_tenths")),
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Availability {
    pub min_expected_adults: u64,
    pub site_tags_any: BTreeSet<String>,
    pub max_per_site: Option<u32>,
    pub min_per_site: Option<u32>,
    pub weight: u32,
    #[serde(default, rename = "stock_per_1000_adults", deserialize_with = "tenths")]
    pub stock_tenths_per_1000_adults: u64,
    pub requires_overlay_any: BTreeSet<String>,
    pub birth_events: BTreeSet<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SizeBounds {
    pub min: u32,
    pub mean: u32,
    pub max: u32,
}
/// Hard recruitment rules of an archetype, never an ancestry (§23.1).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct RecruitmentFilters {
    pub sex: Option<String>,
    pub min_age: Option<u32>,
    pub max_age: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ArchetypeDefinition {
    pub id: String,
    pub class: String,
    pub slot_basis: String,
    pub default_mean_lifetime_years: Option<u32>,
    pub membership_policy: String,
    pub default_time_cost_bps: u16,
    pub activity_ids: BTreeSet<String>,
    pub preemptible: bool,
    pub setting: SettingProfile,
    pub name_profile: String,
    pub size: Option<SizeBounds>,
    pub recruitment_filters: RecruitmentFilters,
    pub availability: Availability,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ArchetypeModifier {
    pub size: Option<SizeBounds>,
    pub weight_multiplier: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct OverlayDefinition {
    pub id: String,
    #[serde(default)]
    pub world_activation_min_setting: Option<SettingProfile>,
    #[serde(default)]
    pub archetype_modifiers: BTreeMap<String, ArchetypeModifier>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FamilyParameters {
    pub kin_recruitment_bonus: u32,
    pub formal_association_weight: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CultureFamily {
    pub id: String,
    pub common_groups: BTreeSet<String>,
    pub governance_template: String,
    pub parameters: FamilyParameters,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct SeatThreshold {
    pub min_expected_adults: u64,
    pub seats: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct GovernanceTemplate {
    pub id: String,
    pub default_rule: String,
    pub default_personal_term_years: u32,
    pub personal_min_age: u32,
    pub personal_max_age: u32,
    pub collective_seats_thresholds: Vec<SeatThreshold>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CatalogueId {
    pub id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CultProfile {
    pub weights: BTreeMap<String, i32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AxisBase {
    pub morality_bps: i32,
    pub lawfulness_bps: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FamilyValues {
    pub default_cult_profile_id: String,
    pub alignment_base: AxisBase,
    pub weights: BTreeMap<String, i32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ArchetypeAlignment {
    pub morality_bps: i32,
    pub lawfulness_bps: i32,
    pub morality_spread_bps: i32,
    pub lawfulness_spread_bps: i32,
    pub defining_tags: BTreeSet<String>,
    pub default_visibility: Visibility,
    pub jurisdiction_organ: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct LegalThresholds {
    pub conformity_proscribed_at_or_below_bps: i32,
    pub conformity_tolerated_at_or_below_bps: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct LawlessRule {
    pub morality_max_bps: i32,
    pub lawfulness_max_bps: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct MoralityMax {
    pub morality_max_bps: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct MoralityBelow {
    pub morality_below_bps: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct ClassThresholds {
    pub lawless: LawlessRule,
    pub crooked: MoralityMax,
    pub ordinary: MoralityBelow,
    pub upright: MoralityBelow,
}
impl From<ClassThresholds> for ConductThresholds {
    fn from(t: ClassThresholds) -> Self {
        Self {
            lawless_morality_max_bps: t.lawless.morality_max_bps,
            lawless_lawfulness_max_bps: t.lawless.lawfulness_max_bps,
            crooked_morality_max_bps: t.crooked.morality_max_bps,
            ordinary_morality_below_bps: t.ordinary.morality_below_bps,
            upright_morality_below_bps: t.upright.morality_below_bps,
        }
    }
}
/// Separate thresholds of §27.2: organizations reach `saintly` earlier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct ConductClassThresholds {
    pub person: ClassThresholds,
    pub organization: ClassThresholds,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct AlignmentDefaults {
    pub legal_status_thresholds: LegalThresholds,
    pub conduct_class_thresholds: ConductClassThresholds,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct CapacityBand {
    pub min_age: u32,
    pub max_age: u32,
    pub capacity_bps: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct LocalOfficeDefaults {
    pub min_age: u32,
    pub max_age: u32,
    pub term_years: u32,
    pub time_cost_bps: u16,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SplitTrigger {
    pub eligible_classes: BTreeSet<String>,
    pub cohesion_below_bps: i32,
    pub consecutive_years: u32,
    pub min_realized_members: usize,
    pub annual_probability_bps: u16,
    pub cooldown_years: i32,
    pub new_group_member_share_bps: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct CohesionEffects {
    pub exposure_member_share_min_bps: u32,
    pub crisis_delta_bps_per_event_year: i32,
    pub max_crises_per_year: usize,
    pub required_office_vacancy_delta_bps_per_year: i32,
    pub recovery_delta_bps_per_unpenalized_year: i32,
}
/// Reach, persistence and decay of a diffuse channel (§11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct InfluenceProfile {
    pub travel_budget_days: u64,
    pub persistence_years: i32,
    pub decay_bps_per_year: u32,
    pub max_sources: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EngineDefaults {
    pub expected_adult_share_bps: u64,
    pub alignment: AlignmentDefaults,
    pub activity_inheritance_bps: u32,
    pub status_inheritance_bps: u32,
    pub cohesion_mean_bps: i32,
    pub friend_circles_per_1000_adults: u64,
    pub formal_associations_per_1000_adults: u64,
    pub community_groups_per_1000_adults: u64,
    pub local_office: LocalOfficeDefaults,
    pub friend_circle_lifetime_year_weights: BTreeMap<String, u64>,
    pub friend_circle_size_weights: BTreeMap<String, u64>,
    pub adult_workforce_participation_bps: u64,
    pub travel_scale_days: u64,
    pub skill_initial_max: u32,
    pub skill_gain_per_activity_year: u32,
    pub network_bonus_per_prior_link: u32,
    pub network_bonus_max: u32,
    pub work_capacity_bps_by_age: Vec<CapacityBand>,
    pub influence_profiles: BTreeMap<String, InfluenceProfile>,
    pub split_trigger: SplitTrigger,
    pub cohesion_effects: CohesionEffects,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct HistoryCatalogue {
    pub catalogue_version: String,
    pub families: Vec<CultureFamily>,
    pub governance_templates: Vec<GovernanceTemplate>,
    pub bindings: Vec<CultureBinding>,
    pub demography_society_templates: Vec<SocietyTemplate>,
    pub archetype_definitions: Vec<ArchetypeDefinition>,
    pub overlay_definitions: Vec<OverlayDefinition>,
    pub site_tag_definitions: Vec<CatalogueId>,
    pub required_activity_coverage: BTreeSet<String>,
    pub cult_value_profiles: BTreeMap<String, CultProfile>,
    pub family_value_profiles: BTreeMap<String, FamilyValues>,
    pub archetype_alignment: BTreeMap<String, ArchetypeAlignment>,
    pub engine_defaults: EngineDefaults,
}
fn missing(key: &str, id: &str) -> HistoryError {
    HistoryError {
        owner: Some(id.into()),
        ..HistoryError::new(ErrorCode::InvalidCanonicalValue, key)
    }
}
/// `conformity_bps` of §27.4 over the defining tag weights: the worst weight
/// softened by the best virtue, never a mean.
pub fn conformity_bps(weights: &[i32]) -> i32 {
    let worst = weights.iter().copied().filter(|w| *w < 0).min();
    let best = weights
        .iter()
        .copied()
        .filter(|w| *w > 0)
        .max()
        .unwrap_or(0);
    let value = match worst {
        Some(worst) => worst + best.div_euclid(2).min(200),
        None => best,
    };
    (10 * value).clamp(-10_000, 10_000)
}
impl HistoryCatalogue {
    pub fn binding(&self, culture: &str) -> Result<&CultureBinding, HistoryError> {
        self.bindings
            .iter()
            .find(|b| b.mu_culture_id == culture)
            .ok_or_else(|| missing("catalogue.culture_without_binding", culture))
    }
    pub fn family(&self, id: &str) -> Result<&CultureFamily, HistoryError> {
        self.families
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| missing("catalogue.family_missing", id))
    }
    pub fn governance(&self, id: &str) -> Result<&GovernanceTemplate, HistoryError> {
        self.governance_templates
            .iter()
            .find(|g| g.id == id)
            .ok_or_else(|| missing("catalogue.governance_template_missing", id))
    }
    pub fn template(&self, id: &str) -> Result<&SocietyTemplate, HistoryError> {
        self.demography_society_templates
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| missing("catalogue.society_template_missing", id))
    }
    /// Initial status of an archetype in a jurisdiction of this family (§27.4):
    /// the jurisdiction's cult weight wins over the family's, and a jurisdiction
    /// does not proscribe its own organs.
    pub fn initial_status(
        &self,
        archetype: &str,
        family: &str,
    ) -> Result<LegalStatus, HistoryError> {
        let alignment = self
            .archetype_alignment
            .get(archetype)
            .ok_or_else(|| missing("catalogue.archetype_without_alignment", archetype))?;
        if alignment.jurisdiction_organ {
            return Ok(LegalStatus::Licit);
        }
        let values = self
            .family_value_profiles
            .get(family)
            .ok_or_else(|| missing("catalogue.family_without_values", family))?;
        let cult = self
            .cult_value_profiles
            .get(&values.default_cult_profile_id)
            .ok_or_else(|| missing("catalogue.cult_profile_missing", family))?;
        let weights: Vec<_> = alignment
            .defining_tags
            .iter()
            .filter_map(|tag| cult.weights.get(tag).or(values.weights.get(tag)).copied())
            .collect();
        let conformity = conformity_bps(&weights);
        let thresholds = self.engine_defaults.alignment.legal_status_thresholds;
        Ok(
            if conformity <= thresholds.conformity_proscribed_at_or_below_bps {
                LegalStatus::Proscribed
            } else if conformity <= thresholds.conformity_tolerated_at_or_below_bps {
                LegalStatus::Tolerated
            } else {
                LegalStatus::Licit
            },
        )
    }
    /// Overlays active on a site: those of its variants and those the world
    /// setting activates (§23.3).
    pub fn active_overlays<'a>(
        &'a self,
        cultures: impl Iterator<Item = &'a str>,
        setting: SettingProfile,
    ) -> Result<BTreeSet<&'a str>, HistoryError> {
        let mut active: BTreeSet<&str> = self
            .overlay_definitions
            .iter()
            .filter(|o| {
                o.world_activation_min_setting
                    .is_some_and(|min| min <= setting)
            })
            .map(|o| o.id.as_str())
            .collect();
        for culture in cultures {
            active.extend(self.binding(culture)?.overlays.iter().map(String::as_str));
        }
        Ok(active)
    }
}
impl ArchetypeDefinition {
    /// Context eligibility (§23.1), regardless of groups already present.
    pub fn is_available(
        &self,
        adults: u64,
        site_tags: &BTreeSet<String>,
        setting: SettingProfile,
        overlays: &BTreeSet<&str>,
    ) -> bool {
        let a = &self.availability;
        self.setting <= setting
            && adults >= a.min_expected_adults
            && (a.site_tags_any.is_empty() || !a.site_tags_any.is_disjoint(site_tags))
            && (a.requires_overlay_any.is_empty()
                || a.requires_overlay_any
                    .iter()
                    .any(|o| overlays.contains(o.as_str())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conformity_is_not_averaged() {
        assert_eq!(conformity_bps(&[-900, 400]), -7_000);
        assert_eq!(conformity_bps(&[-300, 300]), -1_500);
        assert_eq!(conformity_bps(&[300, 600]), 6_000);
        assert_eq!(conformity_bps(&[]), 0);
    }
}
