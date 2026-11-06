//! Résolution d'exposition sur le catalogue canonique complet du propriétaire.
use super::atlas::{unique_ids, HistoryAtlas};
use super::ids::random_semantics_v2::{draw, uniform_below};
use super::model::{string_enum, ErrorCode, HistoryError, TimeSpan};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EXACT_CANDIDATES: usize = 100_000;
string_enum!(ImpactTarget {
    Sites,
    People,
    Cohorts,
    Organizations,
    Assets
});
string_enum!(RoundingRule { Floor, Ceil });
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImpactMode {
    Probability {
        probability_bps: u16,
    },
    ExactCount {
        count: u64,
    },
    ExactFraction {
        numerator: u64,
        denominator: u64,
        rounding: RoundingRule,
    },
    FixedTarget {
        target_ref: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "ids",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TargetSelector {
    All,
    Ids(BTreeSet<String>),
}
string_enum!(ExposureModel {
    IndependentTargetV1
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactContract {
    pub id: String,
    pub parent_event_id: String,
    pub owner_scope: String,
    pub valid: TimeSpan,
    pub target: ImpactTarget,
    pub selector: TargetSelector,
    pub eligibility_time: i32,
    #[serde(flatten)]
    pub mode: ImpactMode,
    pub exposure_model: ExposureModel,
    pub severity_profile: String,
    pub timing_profile: String,
    pub channel: String,
    pub cause: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactCandidate {
    pub id: String,
    pub existence: TimeSpan,
}
/// Catalogue déclaré complet par l'autorité du propriétaire. Ce type ne peut
/// être obtenu par une méthode de pagination ou de projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompleteCandidateCatalogue {
    owner_scope: String,
    target: ImpactTarget,
    candidates: BTreeMap<String, TimeSpan>,
}
impl CompleteCandidateCatalogue {
    pub fn from_owner_catalogue(
        owner_scope: String,
        target: ImpactTarget,
        candidates: Vec<ImpactCandidate>,
    ) -> Result<Self, HistoryError> {
        if !super::atlas::valid_id(&owner_scope)
            || !unique_ids(candidates.iter().map(|c| c.id.as_str()))
        {
            return Err(HistoryError::new(
                ErrorCode::ExactImpactInfeasible,
                "catalogue dupliqué ou invalide",
            ));
        }
        for c in &candidates {
            c.existence.validate(false)?;
        }
        Ok(Self {
            owner_scope,
            target,
            candidates: candidates
                .into_iter()
                .map(|c| (c.id, c.existence))
                .collect(),
        })
    }
    /// Pour les sites, la complétude vient de l'atlas validé, pas de la fenêtre.
    pub fn from_atlas_sites(atlas: &HistoryAtlas, owner: &str) -> Result<Self, HistoryError> {
        atlas.validate()?;
        if !atlas.cells.iter().any(|c| c.cell_id == owner) {
            return Err(HistoryError::new(
                ErrorCode::InvalidAtlas,
                "propriétaire absent",
            ));
        }
        let candidates = atlas
            .settlements
            .iter()
            .filter(|s| atlas.cell_is_within(&s.cell_id, owner))
            .map(|s| ImpactCandidate {
                id: s.site_id.clone(),
                existence: atlas.timeline,
            })
            .collect();
        Self::from_owner_catalogue(owner.into(), ImpactTarget::Sites, candidates)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetImpact {
    pub target_id: String,
    pub event_id: String,
    /// Mot aléatoire d'intensité, indépendant de l'exposition ; le profil
    /// métier qui l'interprétera appartient à S2.
    pub intensity_draw: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImpactResolution {
    pub contract_id: String,
    pub targets: Vec<TargetImpact>,
}
impl ImpactResolution {
    pub fn for_target(&self, id: &str) -> Option<&TargetImpact> {
        self.targets.iter().find(|t| t.target_id == id)
    }
    /// Projection seule : aucun tirage ni nouvelle résolution de quota.
    pub fn project<'a>(&'a self, ids: &BTreeSet<String>) -> Vec<&'a TargetImpact> {
        self.targets
            .iter()
            .filter(|t| ids.contains(&t.target_id))
            .collect()
    }
}
pub fn resolve_impact(
    seed: u64,
    contract: &ImpactContract,
    catalogue: &CompleteCandidateCatalogue,
) -> Result<ImpactResolution, HistoryError> {
    let fail = |code, s: &str| HistoryError {
        owner: Some(contract.owner_scope.clone()),
        at_year: Some(contract.eligibility_time),
        ..HistoryError::new(code, s)
    };
    contract.valid.validate(false)?;
    if !contract.valid.contains(contract.eligibility_time)
        || contract.owner_scope != catalogue.owner_scope
        || contract.target != catalogue.target
        || [
            &contract.id,
            &contract.parent_event_id,
            &contract.owner_scope,
            &contract.severity_profile,
            &contract.timing_profile,
            &contract.channel,
            &contract.cause,
        ]
        .iter()
        .any(|id| !super::atlas::valid_id(id))
    {
        return Err(fail(
            ErrorCode::ExactImpactInfeasible,
            "contrat ou catalogue incompatible",
        ));
    }
    if !matches!(contract.mode, ImpactMode::Probability { .. })
        && matches!(
            contract.target,
            ImpactTarget::People | ImpactTarget::Cohorts
        )
    {
        return Err(fail(
            ErrorCode::UnsupportedCapability,
            "exact démographique réservé à S6",
        ));
    }
    if !matches!(contract.mode, ImpactMode::Probability { .. })
        && catalogue.candidates.len() > MAX_EXACT_CANDIDATES
    {
        return Err(fail(
            ErrorCode::BudgetExceeded,
            "catalogue exact au-delà de 100 000 candidats",
        ));
    }
    if let TargetSelector::Ids(ids) = &contract.selector {
        if ids.iter().any(|id| !catalogue.candidates.contains_key(id)) {
            return Err(fail(
                ErrorCode::ExactImpactInfeasible,
                "sélecteur hors catalogue",
            ));
        }
    }
    let candidates: Vec<_> = catalogue
        .candidates
        .iter()
        .filter(|(id, span)| {
            span.contains(contract.eligibility_time)
                && match &contract.selector {
                    TargetSelector::All => true,
                    TargetSelector::Ids(ids) => ids.contains(*id),
                }
        })
        .map(|(id, _)| id)
        .collect();
    let mut selected = Vec::new();
    if let ImpactMode::Probability { probability_bps } = contract.mode {
        if probability_bps > 10_000 {
            return Err(fail(
                ErrorCode::ExactImpactInfeasible,
                "probabilité hors bps",
            ));
        }
        for id in candidates {
            if uniform_below(seed, &[&contract.id, id, "exposure"], 0, 10_000)?
                < u64::from(probability_bps)
            {
                selected.push(id);
            }
        }
    } else {
        let k = match &contract.mode {
            ImpactMode::ExactCount { count } => u128::from(*count),
            ImpactMode::ExactFraction {
                numerator,
                denominator,
                rounding,
            } => {
                if *denominator == 0 || numerator > denominator {
                    return Err(fail(ErrorCode::ExactImpactInfeasible, "fraction invalide"));
                }
                let product = (candidates.len() as u128) * u128::from(*numerator);
                match rounding {
                    RoundingRule::Floor => product / u128::from(*denominator),
                    RoundingRule::Ceil => product.div_ceil(u128::from(*denominator)),
                }
            }
            ImpactMode::FixedTarget { target_ref } => {
                if !candidates.contains(&target_ref) {
                    return Err(fail(
                        ErrorCode::ExactImpactInfeasible,
                        "cible fixe inéligible",
                    ));
                }
                selected.push(target_ref);
                0
            }
            ImpactMode::Probability { .. } => unreachable!(),
        };
        if k > candidates.len() as u128 {
            return Err(fail(
                ErrorCode::ExactImpactInfeasible,
                "K dépasse le catalogue éligible",
            ));
        }
        if !matches!(contract.mode, ImpactMode::FixedTarget { .. }) {
            let mut ranked: Vec<_> = candidates
                .into_iter()
                .map(|id| (draw(seed, &[&contract.id, id, "priority"], 0, 0), id))
                .collect();
            ranked.sort();
            selected.extend(ranked.into_iter().take(k as usize).map(|(_, id)| id));
        }
    }
    selected.sort();
    Ok(ImpactResolution {
        contract_id: contract.id.clone(),
        targets: selected
            .into_iter()
            .map(|id| TargetImpact {
                target_id: id.clone(),
                event_id: contract.parent_event_id.clone(),
                intensity_draw: draw(seed, &[&contract.id, id, "intensity"], 0, 0),
            })
            .collect(),
    })
}
