//! Résolution d'exposition sur le catalogue canonique complet du propriétaire.
use super::atlas::HistoryAtlas;
use super::ids::random_semantics_v2::{draw, uniform_below};
use super::ids::{AddressKind, Owner, OwnerScope, SemanticAddress};
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
    pub owner_scope: OwnerScope,
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
impl ImpactContract {
    /// `id` décode en adresse d'impact, fille de `parent_event_id` et portée par
    /// `owner_scope`. Rend cette adresse, parente des effets.
    pub fn validate(&self) -> Result<SemanticAddress, HistoryError> {
        let fail = |message: &str| HistoryError {
            owner: Some(self.owner_scope.to_string()),
            at_year: Some(self.eligibility_time),
            ..HistoryError::new(ErrorCode::InvalidImpactContract, message)
        };
        let impact = SemanticAddress::decode(&self.id)
            .ok()
            .filter(|a| a.kind() == AddressKind::Impact)
            .ok_or_else(|| fail("id non adressé comme impact"))?;
        if impact.parent_event().map(|e| e.encode()).as_ref() != Some(&self.parent_event_id) {
            return Err(fail("impact étranger à son événement parent"));
        }
        if impact.owner().as_ref().map(|o| o.scope()) != Some(&self.owner_scope) {
            return Err(fail("owner_scope différent de l'adresse"));
        }
        Ok(impact)
    }
}
/// Catalogue complet d'un propriétaire. Il n'a ni champ public ni `Deserialize`
/// et ne se construit que depuis une source d'autorité du noyau, qui énumère
/// elle-même les candidats : aucune liste n'est reçue de l'appelant. S0 n'a que
/// l'atlas ; S2 ajoutera d'autres sources de la même façon.
///
/// ```compile_fail
/// use chronos_core::v2::region::CompleteCandidateCatalogue;
/// let forged: CompleteCandidateCatalogue = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompleteCandidateCatalogue {
    owner: Owner,
    target: ImpactTarget,
    candidates: BTreeMap<String, TimeSpan>,
}
impl CompleteCandidateCatalogue {
    /// Sites du propriétaire dans l'atlas validé. Un domaine n'est pas connu de
    /// l'atlas : sa partition appartient à S2.
    pub fn from_atlas(atlas: &HistoryAtlas, owner: &Owner) -> Result<Self, HistoryError> {
        atlas.validate()?;
        let fail = |code, message: &str| HistoryError {
            owner: Some(owner.scope().to_string()),
            at_year: Some(atlas.timeline.start_year),
            ..HistoryError::new(code, message)
        };
        let within = |cell_id: &str, site_id: &str| match owner.scope() {
            OwnerScope::Cell { cell_id: root } => atlas.cell_is_within(cell_id, root),
            OwnerScope::Site { site_id: only } => site_id == only,
            _ => true,
        };
        let known = match owner.scope() {
            OwnerScope::World => true,
            OwnerScope::Cell { cell_id } => atlas.cells.iter().any(|c| &c.cell_id == cell_id),
            OwnerScope::Site { site_id } => atlas.settlements.iter().any(|s| &s.site_id == site_id),
            OwnerScope::Domain { .. } => {
                return Err(fail(
                    ErrorCode::UnsupportedCapability,
                    "catalogue d'un domaine : partition S2",
                ));
            }
        };
        if !known {
            return Err(fail(ErrorCode::InvalidAtlas, "propriétaire absent"));
        }
        Ok(Self {
            owner: owner.clone(),
            target: ImpactTarget::Sites,
            candidates: atlas
                .settlements
                .iter()
                .filter(|s| within(&s.cell_id, &s.site_id))
                .map(|s| (s.site_id.clone(), atlas.timeline))
                .collect(),
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetImpact {
    pub target_id: String,
    pub event_id: String,
    /// Effet propre à cette cible, fils de `event_id`.
    pub effect_id: String,
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
/// Résout l'exposition sur tout le catalogue. En mode exact, K se calcule sur
/// les candidats éligibles à `eligibility_time` et retenus par le sélecteur,
/// jamais sur le catalogue brut : une fraction porte sur ce qui peut être touché.
pub fn resolve_impact(
    seed: u64,
    contract: &ImpactContract,
    catalogue: &CompleteCandidateCatalogue,
) -> Result<ImpactResolution, HistoryError> {
    let located = |e: HistoryError| HistoryError {
        owner: Some(contract.owner_scope.to_string()),
        at_year: Some(contract.eligibility_time),
        ..e
    };
    let fail = |code, s: &str| located(HistoryError::new(code, s));
    contract.valid.validate(false).map_err(located)?;
    let impact = contract.validate()?;
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
    // Contrat et catalogue partagent canon et propriétaire ; les effets en
    // héritent par leur adresse.
    if !contract.valid.contains(contract.eligibility_time)
        || impact.owner().as_ref() != Some(&catalogue.owner)
        || contract.target != catalogue.target
        || [
            &contract.severity_profile,
            &contract.timing_profile,
            &contract.channel,
            &contract.cause,
        ]
        .iter()
        .any(|id| !super::atlas::valid_id(id))
    {
        return Err(fail(
            ErrorCode::InvalidImpactContract,
            "contrat ou catalogue incompatible",
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
                ErrorCode::InvalidImpactContract,
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
                ErrorCode::InvalidImpactContract,
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
                    return Err(fail(ErrorCode::InvalidImpactContract, "fraction invalide"));
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
    let targets = selected
        .into_iter()
        .map(|id| {
            Ok(TargetImpact {
                target_id: id.clone(),
                event_id: contract.parent_event_id.clone(),
                effect_id: SemanticAddress::site(catalogue.owner.world(), id)
                    .and_then(|target| SemanticAddress::effect(&impact, &target))
                    .map_err(located)?
                    .encode(),
                intensity_draw: draw(seed, &[&contract.id, id, "intensity"], 0, 0),
            })
        })
        .collect::<Result<_, HistoryError>>()?;
    Ok(ImpactResolution {
        contract_id: contract.id.clone(),
        targets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::ids::Owner;
    const PERIOD: TimeSpan = TimeSpan {
        start_year: 1300,
        end_year: 1400,
    };
    fn owner() -> Owner {
        Owner::cell("w", "region")
    }
    // Construction directe, réservée à ce module : le budget se teste sans
    // bâtir un atlas de 100 000 sites.
    fn catalogue(n: usize) -> CompleteCandidateCatalogue {
        CompleteCandidateCatalogue {
            owner: owner(),
            target: ImpactTarget::Sites,
            candidates: (0..n)
                .map(|i| (format!("candidate/{i:06}"), PERIOD))
                .collect(),
        }
    }
    fn contract(target: ImpactTarget, mode: ImpactMode) -> ImpactContract {
        let event = SemanticAddress::event(&owner(), "war", 1350, 0).unwrap();
        ImpactContract {
            id: SemanticAddress::impact(&event, "impact-0")
                .unwrap()
                .encode(),
            parent_event_id: event.encode(),
            owner_scope: owner().scope().clone(),
            valid: PERIOD,
            target,
            selector: TargetSelector::All,
            eligibility_time: 1354,
            mode,
            exposure_model: ExposureModel::IndependentTargetV1,
            severity_profile: "war.medium".into(),
            timing_profile: "annual-1".into(),
            channel: "destruction".into(),
            cause: "war".into(),
        }
    }
    #[test]
    fn exact_catalogue_boundary_accepts_100_000_and_refuses_100_001() {
        let c = contract(ImpactTarget::Sites, ImpactMode::ExactCount { count: 3 });
        let accepted = catalogue(MAX_EXACT_CANDIDATES);
        assert_eq!(resolve_impact(42, &c, &accepted).unwrap().targets.len(), 3);
        let refused = catalogue(MAX_EXACT_CANDIDATES + 1);
        assert_eq!(
            resolve_impact(42, &c, &refused).unwrap_err().code,
            ErrorCode::BudgetExceeded
        );
    }
    #[test]
    fn probability_mode_ignores_the_exact_budget() {
        let c = contract(
            ImpactTarget::Sites,
            ImpactMode::Probability { probability_bps: 1 },
        );
        assert!(resolve_impact(42, &c, &catalogue(MAX_EXACT_CANDIDATES + 1)).is_ok());
    }
    #[test]
    fn exact_demographic_impacts_are_rejected_until_s6() {
        for target in [ImpactTarget::People, ImpactTarget::Cohorts] {
            let c = contract(target, ImpactMode::ExactCount { count: 0 });
            assert_eq!(
                resolve_impact(42, &c, &catalogue(0)).unwrap_err().code,
                ErrorCode::UnsupportedCapability
            );
        }
    }
    #[test]
    fn exact_count_is_taken_from_eligible_candidates_only() {
        // Un seul des deux candidats existe à la date : K=2 est infaisable et
        // une fraction d'un demi se calcule sur un seul éligible.
        let mut full = catalogue(2);
        *full.candidates.values_mut().last().unwrap() = TimeSpan {
            start_year: 1360,
            end_year: 1400,
        };
        let c = contract(ImpactTarget::Sites, ImpactMode::ExactCount { count: 2 });
        assert_eq!(
            resolve_impact(42, &c, &full).unwrap_err().code,
            ErrorCode::ExactImpactInfeasible
        );
        let half = |rounding| ImpactMode::ExactFraction {
            numerator: 1,
            denominator: 2,
            rounding,
        };
        for (rounding, k) in [(RoundingRule::Floor, 0), (RoundingRule::Ceil, 1)] {
            let c = contract(ImpactTarget::Sites, half(rounding));
            assert_eq!(resolve_impact(42, &c, &full).unwrap().targets.len(), k);
        }
    }
    #[test]
    fn invalid_contracts_never_report_an_exact_infeasibility() {
        let full = catalogue(3);
        let bad_probability = contract(
            ImpactTarget::Sites,
            ImpactMode::Probability {
                probability_bps: 10_001,
            },
        );
        let wrong_target = contract(
            ImpactTarget::Assets,
            ImpactMode::Probability { probability_bps: 1 },
        );
        let mut unaddressed_parent = contract(
            ImpactTarget::Sites,
            ImpactMode::Probability { probability_bps: 1 },
        );
        unaddressed_parent.parent_event_id = "event/demo".into();
        let mut foreign_selector = contract(
            ImpactTarget::Sites,
            ImpactMode::Probability { probability_bps: 1 },
        );
        foreign_selector.selector = TargetSelector::Ids(BTreeSet::from(["absent".into()]));
        let zero_fraction = contract(
            ImpactTarget::Sites,
            ImpactMode::ExactFraction {
                numerator: 1,
                denominator: 0,
                rounding: RoundingRule::Floor,
            },
        );
        for c in [
            bad_probability,
            wrong_target,
            unaddressed_parent,
            foreign_selector,
            zero_fraction,
        ] {
            let e = resolve_impact(42, &c, &full).unwrap_err();
            assert_eq!(e.code, ErrorCode::InvalidImpactContract);
            assert_eq!(e.owner.as_deref(), Some("region"));
            assert_eq!(e.at_year, Some(1354));
        }
    }
}
