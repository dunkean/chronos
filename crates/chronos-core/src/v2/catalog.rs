//! Typed reading of the S1 culture catalogue (§6, §23, §27): only the sections
//! the canon needs. The catalogue carries no `demography_society_id`: the host
//! computes them (§26.3) and passes them in the world manifest.
use super::model::{string_enum, ErrorCode, HistoryError, LegalStatus, Visibility};
use super::population::{ParameterValue, SocietyProfile, SocietySchema};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};

string_enum!(SettingProfile {
    Historical,
    LowFantasy,
    HighFantasy
});

#[derive(Clone, Debug, Deserialize)]
pub struct CultureBinding {
    pub mu_culture_id: String,
    pub history_family_id: String,
    pub overlays: Vec<String>,
    pub demography_society_template_id: String,
}
#[derive(Clone, Debug, Deserialize)]
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
#[derive(Clone, Debug, Deserialize)]
pub struct Availability {
    pub min_expected_adults: u64,
    pub site_tags_any: BTreeSet<String>,
    pub max_per_site: Option<u32>,
    pub min_per_site: Option<u32>,
    #[serde(default, rename = "stock_per_1000_adults", deserialize_with = "tenths")]
    pub stock_tenths_per_1000_adults: u64,
    pub requires_overlay_any: BTreeSet<String>,
    pub birth_events: BTreeSet<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct ArchetypeDefinition {
    pub id: String,
    pub class: String,
    pub default_mean_lifetime_years: Option<u32>,
    pub setting: SettingProfile,
    pub availability: Availability,
}
#[derive(Clone, Debug, Deserialize)]
pub struct OverlayDefinition {
    pub id: String,
    #[serde(default)]
    pub world_activation_min_setting: Option<SettingProfile>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct CatalogueId {
    pub id: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct CultProfile {
    pub weights: BTreeMap<String, i32>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct FamilyValues {
    pub default_cult_profile_id: String,
    pub weights: BTreeMap<String, i32>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct ArchetypeAlignment {
    pub defining_tags: BTreeSet<String>,
    pub default_visibility: Visibility,
    pub jurisdiction_organ: bool,
}
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct LegalThresholds {
    pub conformity_proscribed_at_or_below_bps: i32,
    pub conformity_tolerated_at_or_below_bps: i32,
}
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct AlignmentDefaults {
    pub legal_status_thresholds: LegalThresholds,
}
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct EngineDefaults {
    pub expected_adult_share_bps: u64,
    pub alignment: AlignmentDefaults,
}
#[derive(Clone, Debug, Deserialize)]
pub struct HistoryCatalogue {
    pub catalogue_version: String,
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
