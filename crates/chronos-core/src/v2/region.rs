//! Résolution d'exposition sur le catalogue canonique complet du propriétaire.
use super::atlas::{unique_ids, HistoryAtlas};
use super::canonical::canonical_bytes;
use super::ids::random_semantics_v2::{draw, uniform_below};
use super::ids::{AddressKind, SemanticAddress};
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
/// Preuve de complétude publiée par l'autorité du propriétaire, indépendamment
/// de la liste qui voyage avec elle. L'empreinte FNV-1a est un contrôle
/// d'intégrité, jamais une identité (§4.1). Une fenêtre de consultation n'a pas
/// de quoi émettre cette preuve : seuls l'atlas validé et le noyau le peuvent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueProof {
    pub owner_scope: String,
    pub target: ImpactTarget,
    pub candidate_count: u64,
    pub catalogue_digest: String,
}
fn catalogue_error(message: &str) -> HistoryError {
    HistoryError::new(ErrorCode::IncompleteCatalogue, message)
}
fn sorted_candidates(
    candidates: Vec<ImpactCandidate>,
) -> Result<BTreeMap<String, TimeSpan>, HistoryError> {
    if !unique_ids(candidates.iter().map(|c| c.id.as_str())) {
        return Err(catalogue_error("catalogue dupliqué ou invalide"));
    }
    for c in &candidates {
        c.existence.validate(false)?;
    }
    Ok(candidates
        .into_iter()
        .map(|c| (c.id, c.existence))
        .collect())
}
fn catalogue_digest(
    owner_scope: &str,
    target: ImpactTarget,
    candidates: &BTreeMap<String, TimeSpan>,
) -> Result<String, HistoryError> {
    let value = serde_json::json!({
        "owner_scope": owner_scope,
        "target": target,
        "candidates": candidates
            .iter()
            .map(|(id, span)| serde_json::json!([id, span.start_year, span.end_year]))
            .collect::<Vec<_>>(),
    });
    Ok(crate::keys::fingerprint(&canonical_bytes(&value)?))
}
impl CatalogueProof {
    /// Réservée au noyau, seule autorité du catalogue d'un propriétaire.
    pub(crate) fn issue(
        owner_scope: &str,
        target: ImpactTarget,
        candidates: Vec<ImpactCandidate>,
    ) -> Result<Self, HistoryError> {
        let candidates = sorted_candidates(candidates)?;
        Ok(Self {
            owner_scope: owner_scope.into(),
            target,
            candidate_count: candidates.len() as u64,
            catalogue_digest: catalogue_digest(owner_scope, target, &candidates)?,
        })
    }
    /// Pour les sites, la complétude vient de l'atlas validé, pas de la fenêtre.
    pub fn for_atlas_sites(atlas: &HistoryAtlas, owner: &str) -> Result<Self, HistoryError> {
        Self::issue(owner, ImpactTarget::Sites, atlas_sites(atlas, owner)?)
    }
}
fn atlas_sites(atlas: &HistoryAtlas, owner: &str) -> Result<Vec<ImpactCandidate>, HistoryError> {
    atlas.validate()?;
    if !atlas.cells.iter().any(|c| c.cell_id == owner) {
        return Err(HistoryError {
            owner: Some(owner.into()),
            at_year: Some(atlas.timeline.start_year),
            ..HistoryError::new(ErrorCode::InvalidAtlas, "propriétaire absent")
        });
    }
    Ok(atlas
        .settlements
        .iter()
        .filter(|s| atlas.cell_is_within(&s.cell_id, owner))
        .map(|s| ImpactCandidate {
            id: s.site_id.clone(),
            existence: atlas.timeline,
        })
        .collect())
}
/// Catalogue dont la complétude est prouvée. Ce type ne peut être obtenu par
/// une méthode de pagination ou de projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompleteCandidateCatalogue {
    owner_scope: String,
    target: ImpactTarget,
    candidates: BTreeMap<String, TimeSpan>,
}
impl CompleteCandidateCatalogue {
    /// Refuse une liste que la preuve du propriétaire ne couvre pas exactement.
    pub fn from_owner_catalogue(
        proof: &CatalogueProof,
        candidates: Vec<ImpactCandidate>,
    ) -> Result<Self, HistoryError> {
        let located = |mut e: HistoryError| {
            e.owner.get_or_insert_with(|| proof.owner_scope.clone());
            e
        };
        if !super::atlas::valid_id(&proof.owner_scope) {
            return Err(located(catalogue_error("propriétaire invalide")));
        }
        let candidates = sorted_candidates(candidates).map_err(located)?;
        if candidates.len() as u64 != proof.candidate_count
            || catalogue_digest(&proof.owner_scope, proof.target, &candidates).map_err(located)?
                != proof.catalogue_digest
        {
            return Err(located(catalogue_error(
                "liste non couverte par la preuve de complétude",
            )));
        }
        Ok(Self {
            owner_scope: proof.owner_scope.clone(),
            target: proof.target,
            candidates,
        })
    }
    pub fn from_atlas_sites(atlas: &HistoryAtlas, owner: &str) -> Result<Self, HistoryError> {
        let proof = CatalogueProof::for_atlas_sites(atlas, owner)?;
        Self::from_owner_catalogue(&proof, atlas_sites(atlas, owner)?)
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
        owner: Some(contract.owner_scope.clone()),
        at_year: Some(contract.eligibility_time),
        ..e
    };
    let fail = |code, s: &str| located(HistoryError::new(code, s));
    contract.valid.validate(false).map_err(located)?;
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
            ErrorCode::InvalidImpactContract,
            "contrat ou catalogue incompatible",
        ));
    }
    let parent = SemanticAddress::decode(&contract.parent_event_id)
        .ok()
        .filter(|a| a.kind() == AddressKind::Event)
        .ok_or_else(|| {
            fail(
                ErrorCode::InvalidImpactContract,
                "événement parent non adressé",
            )
        })?;
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
                effect_id: SemanticAddress::effect(&parent, &contract.id, id)
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
    fn candidates(n: usize) -> Vec<ImpactCandidate> {
        (0..n)
            .map(|i| ImpactCandidate {
                id: format!("candidate/{i:06}"),
                existence: PERIOD,
            })
            .collect()
    }
    fn catalogue(target: ImpactTarget, n: usize) -> CompleteCandidateCatalogue {
        let list = candidates(n);
        let proof = CatalogueProof::issue("region", target, list.clone()).unwrap();
        CompleteCandidateCatalogue::from_owner_catalogue(&proof, list).unwrap()
    }
    fn contract(target: ImpactTarget, mode: ImpactMode) -> ImpactContract {
        let event = SemanticAddress::event(&Owner::cell("w", "region"), "war", 1350, 0).unwrap();
        ImpactContract {
            id: "impact/test/0".into(),
            parent_event_id: event.encode(),
            owner_scope: "region".into(),
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
        let exact = ImpactMode::ExactCount { count: 3 };
        let accepted = catalogue(ImpactTarget::Sites, MAX_EXACT_CANDIDATES);
        let c = contract(ImpactTarget::Sites, exact.clone());
        assert_eq!(resolve_impact(42, &c, &accepted).unwrap().targets.len(), 3);
        let refused = catalogue(ImpactTarget::Sites, MAX_EXACT_CANDIDATES + 1);
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
        let big = catalogue(ImpactTarget::Sites, MAX_EXACT_CANDIDATES + 1);
        assert!(resolve_impact(42, &c, &big).is_ok());
    }
    #[test]
    fn exact_demographic_impacts_are_rejected_until_s6() {
        for target in [ImpactTarget::People, ImpactTarget::Cohorts] {
            let c = contract(target, ImpactMode::ExactCount { count: 0 });
            assert_eq!(
                resolve_impact(42, &c, &catalogue(target, 0))
                    .unwrap_err()
                    .code,
                ErrorCode::UnsupportedCapability
            );
        }
    }
    #[test]
    fn exact_count_is_taken_from_eligible_candidates_only() {
        // Un seul des deux candidats existe à la date : K=2 est infaisable et
        // une fraction d'un demi se calcule sur un seul éligible.
        let mut list = candidates(2);
        list[1].existence = TimeSpan {
            start_year: 1360,
            end_year: 1400,
        };
        let proof = CatalogueProof::issue("region", ImpactTarget::Sites, list.clone()).unwrap();
        let full = CompleteCandidateCatalogue::from_owner_catalogue(&proof, list).unwrap();
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
        let full = catalogue(ImpactTarget::Sites, 3);
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
    #[test]
    fn organisation_targets_with_encoded_ids_keep_flat_effect_addresses() {
        let owner = Owner::cell("w", "region");
        let group = SemanticAddress::group(&owner, "farm", 1350, 0).unwrap();
        let list = vec![ImpactCandidate {
            id: group.encode(),
            existence: PERIOD,
        }];
        let proof =
            CatalogueProof::issue("region", ImpactTarget::Organizations, list.clone()).unwrap();
        let full = CompleteCandidateCatalogue::from_owner_catalogue(&proof, list).unwrap();
        let c = contract(
            ImpactTarget::Organizations,
            ImpactMode::ExactCount { count: 1 },
        );
        let target = &resolve_impact(42, &c, &full).unwrap().targets[0];
        let effect = SemanticAddress::decode(&target.effect_id).unwrap();
        assert_eq!(effect.kind(), AddressKind::Effect);
        assert!(!effect.fields().contains(&target.target_id));
    }
}
