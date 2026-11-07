//! Compact provider file `population-result-json-2`: the same facts as
//! `PopulationResult`, in columns. Every long or repeated text is an index into
//! `strings`, where `DOMAIN_TOKEN` stands for the domain address. Decoding is total:
//! a bad index or a misaligned table is a shape error, and every fact then goes
//! through `validate_population` like any other.
use super::model::{ErrorCode, HistoryError, TimeSpan};
use super::population::{
    AnnualCensus, BirthAllocation, CandidateTable, CohortResidenceCount, CoreAttributes,
    DeathAllocation, GuardianAssignment, HouseholdEpisode, OriginCohort, ParentageRecord,
    PopulationResult, ProviderManifest, ResidenceTransfer, SlotResidence, UnionAllocation,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

/// Stands for the domain address inside `strings`.
const DOMAIN_TOKEN: char = '\u{1}';

fn shape(detail: impl ToString) -> HistoryError {
    HistoryError::new(
        ErrorCode::InvalidPopulationResult,
        "population.shape_invalid",
    )
    .with_param("detail", detail.to_string())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCandidates {
    origin_id: Vec<u32>,
    origin_cohort_id: Vec<u32>,
    sex_ordinal: Vec<u32>,
    source: Vec<u32>,
    ancestry_id: Vec<u32>,
    sex: Vec<u8>,
    birth_year: Vec<i32>,
    death_year: Vec<Option<i32>>,
    death_site_id: Vec<Option<u32>>,
    coverage_start_year: Vec<i32>,
    coverage_end_year: Vec<i32>,
    departure_state: Vec<u32>,
    departure_year: Vec<Option<i32>>,
    mother_slot: Vec<Option<u32>>,
    father_slot: Vec<Option<u32>>,
    family_key: Vec<u32>,
    reproduction_flags: Vec<u32>,
    core_attributes: Vec<Option<CoreAttributes>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCohorts {
    origin_cohort_id: Vec<u32>,
    origin_type: Vec<u32>,
    ancestry_id: Vec<u32>,
    birth_year: Vec<i32>,
    sealed: Vec<bool>,
    sex_counts: Vec<[u32; 2]>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireResidences {
    person_slot: Vec<u32>,
    site_id: Vec<u32>,
    start_year: Vec<i32>,
    end_year: Vec<i32>,
    start_phase: Vec<u8>,
    end_phase: Vec<u8>,
    end_reason: Vec<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireHouseholds {
    household_id: Vec<u32>,
    kind: Vec<u32>,
    formation_reason: Vec<u32>,
    site_id: Vec<u32>,
    member_slots: Vec<Vec<u32>>,
    start_year: Vec<i32>,
    end_year: Vec<i32>,
    start_phase: Vec<u8>,
    end_phase: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireUnions {
    union_id: Vec<u32>,
    family_key: Vec<u32>,
    partner_slots: Vec<Vec<u32>>,
    site_at_formation: Vec<u32>,
    source: Vec<u32>,
    start_year: Vec<i32>,
    end_year: Vec<i32>,
    start_phase: Vec<u8>,
    end_phase: Vec<u8>,
    end_reason: Vec<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBirths {
    child_slot: Vec<u32>,
    child_origin_cohort_id: Vec<u32>,
    child_sex_ordinal: Vec<u32>,
    family_key: Vec<u32>,
    mother_slot: Vec<Option<u32>>,
    father_slot: Vec<Option<u32>>,
    parental_union_id: Vec<Option<u32>>,
    site_id: Vec<u32>,
    year: Vec<i32>,
    phase: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireDeaths {
    person_slot: Vec<u32>,
    cause: Vec<u32>,
    site_id: Vec<u32>,
    year: Vec<i32>,
    phase: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireTransfers {
    transfer_id: Vec<u32>,
    household_id: Vec<u32>,
    from_site: Vec<Option<u32>>,
    to_site: Vec<Option<u32>>,
    origin_slots: Vec<Vec<u32>>,
    reason: Vec<u32>,
    source: Vec<u32>,
    operation_id: Vec<Option<u32>>,
    port_id: Vec<Option<u32>>,
    year: Vec<i32>,
    phase: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireGuardians {
    child_slot: Vec<u32>,
    guardian_slot: Vec<Option<u32>>,
    source: Vec<u32>,
    year: Vec<i32>,
    phase: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireParentage {
    child_slot: Vec<u32>,
    mother_slot: Vec<Option<u32>>,
    father_slot: Vec<Option<u32>>,
    precision: Vec<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCounts {
    origin_cohort_id: Vec<u32>,
    site_id: Vec<u32>,
    sex: Vec<u8>,
    source: Vec<u32>,
    year: Vec<i32>,
    count: Vec<u64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireResult {
    format: String,
    result_type: String,
    complete: bool,
    coverage: String,
    stage: String,
    plan_id: String,
    plan_fingerprint: String,
    result_fingerprint: String,
    timeline: TimeSpan,
    provider_manifest: ProviderManifest,
    domain_id: String,
    strings: Vec<String>,
    candidate_table: WireCandidates,
    origin_cohorts: WireCohorts,
    residence_index: WireResidences,
    residence_offsets: Vec<u32>,
    household_episodes: WireHouseholds,
    union_allocations: WireUnions,
    birth_allocations: WireBirths,
    death_allocations: WireDeaths,
    residence_transfers: WireTransfers,
    guardian_assignments: WireGuardians,
    parentage: WireParentage,
    children_index: Vec<Vec<u32>>,
    annual_censuses: Vec<AnnualCensus>,
    cohort_residence_counts: WireCounts,
    diagnostics: Value,
    validation_proof: Value,
}

/// `strings`, with the domain address restored.
struct Texts(Vec<String>);
impl Texts {
    fn text(&self, index: u32) -> Result<String, HistoryError> {
        self.0
            .get(index as usize)
            .cloned()
            .ok_or_else(|| shape("string index"))
    }
    fn optional(&self, index: Option<u32>) -> Result<Option<String>, HistoryError> {
        index.map(|i| self.text(i)).transpose()
    }
    fn texts(&self, column: &[u32]) -> Result<Vec<String>, HistoryError> {
        column.iter().map(|i| self.text(*i)).collect()
    }
    /// A closed word of the contract, read once per distinct index.
    fn words<T: DeserializeOwned + Clone>(&self, column: &[u32]) -> Result<Vec<T>, HistoryError> {
        let mut known: Vec<Option<T>> = vec![None; self.0.len()];
        column
            .iter()
            .map(|index| {
                let slot = known
                    .get_mut(*index as usize)
                    .ok_or_else(|| shape("string index"))?;
                if slot.is_none() {
                    let word = Value::String(self.0[*index as usize].clone());
                    *slot = Some(serde_json::from_value(word).map_err(shape)?);
                }
                Ok(slot.clone().expect("word just read"))
            })
            .collect()
    }
}
fn aligned(lengths: &[usize]) -> Result<usize, HistoryError> {
    let n = lengths.first().copied().unwrap_or(0);
    if lengths.iter().all(|len| *len == n) {
        Ok(n)
    } else {
        Err(shape("columns differ in length"))
    }
}

/// Reads a compact provider file into the table form the audit and the social layer use.
pub(super) fn decode(bytes: &[u8]) -> Result<PopulationResult, HistoryError> {
    let wire: WireResult = serde_json::from_slice(bytes).map_err(shape)?;
    let token = DOMAIN_TOKEN.to_string();
    let texts = Texts(
        wire.strings
            .into_iter()
            .map(|s| {
                if s.contains(DOMAIN_TOKEN) {
                    s.replace(&token, &wire.domain_id)
                } else {
                    s
                }
            })
            .collect(),
    );
    let c = wire.candidate_table;
    aligned(&[
        c.origin_id.len(),
        c.origin_cohort_id.len(),
        c.source.len(),
        c.ancestry_id.len(),
        c.death_site_id.len(),
        c.departure_state.len(),
        c.family_key.len(),
    ])?;
    let candidate_table = CandidateTable {
        origin_id: texts.texts(&c.origin_id)?,
        origin_cohort_id: texts.texts(&c.origin_cohort_id)?,
        sex_ordinal: c.sex_ordinal,
        source: texts.words(&c.source)?,
        ancestry_id: texts.texts(&c.ancestry_id)?,
        sex: c.sex,
        birth_year: c.birth_year,
        death_year: c.death_year,
        death_site_id: c
            .death_site_id
            .iter()
            .map(|i| texts.optional(*i))
            .collect::<Result<_, _>>()?,
        coverage_start_year: c.coverage_start_year,
        coverage_end_year: c.coverage_end_year,
        departure_state: texts.words(&c.departure_state)?,
        departure_year: c.departure_year,
        mother_slot: c.mother_slot,
        father_slot: c.father_slot,
        family_key: texts.texts(&c.family_key)?,
        reproduction_flags: c.reproduction_flags,
        core_attributes: c.core_attributes,
    };

    let t = wire.origin_cohorts;
    let n = aligned(&[
        t.origin_cohort_id.len(),
        t.origin_type.len(),
        t.ancestry_id.len(),
        t.birth_year.len(),
        t.sealed.len(),
        t.sex_counts.len(),
    ])?;
    let kinds = texts.words(&t.origin_type)?;
    let mut origin_cohorts = Vec::with_capacity(n);
    for (i, origin_type) in kinds.into_iter().enumerate() {
        origin_cohorts.push(OriginCohort {
            origin_cohort_id: texts.text(t.origin_cohort_id[i])?,
            origin_type,
            ancestry_id: texts.text(t.ancestry_id[i])?,
            birth_year: t.birth_year[i],
            sealed: t.sealed[i],
            sex_counts: t.sex_counts[i],
        });
    }

    let t = wire.residence_index;
    let n = aligned(&[
        t.person_slot.len(),
        t.site_id.len(),
        t.start_year.len(),
        t.end_year.len(),
        t.start_phase.len(),
        t.end_phase.len(),
        t.end_reason.len(),
    ])?;
    let reasons = texts.words(&t.end_reason)?;
    let mut residence_index = Vec::with_capacity(n);
    for (i, end_reason) in reasons.into_iter().enumerate() {
        residence_index.push(SlotResidence {
            person_slot: t.person_slot[i],
            site_id: texts.text(t.site_id[i])?,
            valid: TimeSpan {
                start_year: t.start_year[i],
                end_year: t.end_year[i],
            },
            start_phase: t.start_phase[i],
            end_phase: t.end_phase[i],
            end_reason,
        });
    }

    let mut t = wire.household_episodes;
    let n = aligned(&[
        t.household_id.len(),
        t.kind.len(),
        t.formation_reason.len(),
        t.site_id.len(),
        t.member_slots.len(),
        t.start_year.len(),
        t.end_year.len(),
        t.start_phase.len(),
        t.end_phase.len(),
    ])?;
    let kinds = texts.words(&t.kind)?;
    let mut household_episodes = Vec::with_capacity(n);
    for (i, kind) in kinds.into_iter().enumerate() {
        household_episodes.push(HouseholdEpisode {
            household_id: texts.text(t.household_id[i])?,
            kind,
            formation_reason: texts.text(t.formation_reason[i])?,
            site_id: texts.text(t.site_id[i])?,
            member_slots: std::mem::take(&mut t.member_slots[i]),
            valid: TimeSpan {
                start_year: t.start_year[i],
                end_year: t.end_year[i],
            },
            start_phase: t.start_phase[i],
            end_phase: t.end_phase[i],
        });
    }

    let mut t = wire.union_allocations;
    let n = aligned(&[
        t.union_id.len(),
        t.family_key.len(),
        t.partner_slots.len(),
        t.site_at_formation.len(),
        t.source.len(),
        t.start_year.len(),
        t.end_year.len(),
        t.start_phase.len(),
        t.end_phase.len(),
        t.end_reason.len(),
    ])?;
    let sources = texts.words(&t.source)?;
    let mut union_allocations = Vec::with_capacity(n);
    for (i, source) in sources.into_iter().enumerate() {
        union_allocations.push(UnionAllocation {
            union_id: texts.text(t.union_id[i])?,
            family_key: texts.text(t.family_key[i])?,
            partner_slots: std::mem::take(&mut t.partner_slots[i]),
            site_at_formation: texts.text(t.site_at_formation[i])?,
            source,
            valid: TimeSpan {
                start_year: t.start_year[i],
                end_year: t.end_year[i],
            },
            start_phase: t.start_phase[i],
            end_phase: t.end_phase[i],
            end_reason: texts.text(t.end_reason[i])?,
        });
    }

    let t = wire.birth_allocations;
    let n = aligned(&[
        t.child_slot.len(),
        t.child_origin_cohort_id.len(),
        t.child_sex_ordinal.len(),
        t.family_key.len(),
        t.mother_slot.len(),
        t.father_slot.len(),
        t.parental_union_id.len(),
        t.site_id.len(),
        t.year.len(),
        t.phase.len(),
    ])?;
    let birth_allocations = (0..n)
        .map(|i| {
            Ok(BirthAllocation {
                child_slot: t.child_slot[i],
                child_origin_cohort_id: texts.text(t.child_origin_cohort_id[i])?,
                child_sex_ordinal: t.child_sex_ordinal[i],
                family_key: texts.text(t.family_key[i])?,
                mother_slot: t.mother_slot[i],
                father_slot: t.father_slot[i],
                parental_union_id: texts.optional(t.parental_union_id[i])?,
                site_id: texts.text(t.site_id[i])?,
                year: t.year[i],
                phase: t.phase[i],
            })
        })
        .collect::<Result<_, HistoryError>>()?;

    let t = wire.death_allocations;
    let n = aligned(&[
        t.person_slot.len(),
        t.cause.len(),
        t.site_id.len(),
        t.year.len(),
        t.phase.len(),
    ])?;
    let death_allocations = (0..n)
        .map(|i| {
            Ok(DeathAllocation {
                person_slot: t.person_slot[i],
                cause: texts.text(t.cause[i])?,
                site_id: texts.text(t.site_id[i])?,
                year: t.year[i],
                phase: t.phase[i],
            })
        })
        .collect::<Result<_, HistoryError>>()?;

    let mut t = wire.residence_transfers;
    let n = aligned(&[
        t.transfer_id.len(),
        t.household_id.len(),
        t.from_site.len(),
        t.to_site.len(),
        t.origin_slots.len(),
        t.reason.len(),
        t.source.len(),
        t.operation_id.len(),
        t.port_id.len(),
        t.year.len(),
        t.phase.len(),
    ])?;
    let sources = texts.words(&t.source)?;
    let mut residence_transfers = Vec::with_capacity(n);
    for (i, source) in sources.into_iter().enumerate() {
        residence_transfers.push(ResidenceTransfer {
            transfer_id: texts.text(t.transfer_id[i])?,
            household_id: texts.text(t.household_id[i])?,
            from_site: texts.optional(t.from_site[i])?,
            to_site: texts.optional(t.to_site[i])?,
            origin_slots: std::mem::take(&mut t.origin_slots[i]),
            reason: texts.text(t.reason[i])?,
            source,
            operation_id: texts.optional(t.operation_id[i])?,
            port_id: texts.optional(t.port_id[i])?,
            year: t.year[i],
            phase: t.phase[i],
        });
    }

    let t = wire.guardian_assignments;
    let n = aligned(&[
        t.child_slot.len(),
        t.guardian_slot.len(),
        t.source.len(),
        t.year.len(),
        t.phase.len(),
    ])?;
    let guardian_assignments = (0..n)
        .map(|i| {
            Ok(GuardianAssignment {
                child_slot: t.child_slot[i],
                guardian_slot: t.guardian_slot[i],
                source: texts.text(t.source[i])?,
                year: t.year[i],
                phase: t.phase[i],
            })
        })
        .collect::<Result<_, HistoryError>>()?;

    let t = wire.parentage;
    let n = aligned(&[
        t.child_slot.len(),
        t.mother_slot.len(),
        t.father_slot.len(),
        t.precision.len(),
    ])?;
    let precisions = texts.words(&t.precision)?;
    let mut parentage = Vec::with_capacity(n);
    for (i, precision) in precisions.into_iter().enumerate() {
        parentage.push(ParentageRecord {
            child_slot: t.child_slot[i],
            mother_slot: t.mother_slot[i],
            father_slot: t.father_slot[i],
            precision,
        });
    }

    let t = wire.cohort_residence_counts;
    let n = aligned(&[
        t.origin_cohort_id.len(),
        t.site_id.len(),
        t.sex.len(),
        t.source.len(),
        t.year.len(),
        t.count.len(),
    ])?;
    let sources = texts.words(&t.source)?;
    let mut cohort_residence_counts: Vec<CohortResidenceCount> = Vec::with_capacity(n);
    for (i, source) in sources.into_iter().enumerate() {
        cohort_residence_counts.push(CohortResidenceCount {
            origin_cohort_id: texts.text(t.origin_cohort_id[i])?,
            site_id: texts.text(t.site_id[i])?,
            sex: t.sex[i],
            source,
            year: t.year[i],
            count: t.count[i],
        });
    }

    Ok(PopulationResult {
        format: wire.format,
        result_type: wire.result_type,
        complete: wire.complete,
        coverage: wire.coverage,
        stage: wire.stage,
        plan_id: wire.plan_id,
        plan_fingerprint: wire.plan_fingerprint,
        result_fingerprint: wire.result_fingerprint,
        timeline: wire.timeline,
        provider_manifest: wire.provider_manifest,
        candidate_table,
        origin_cohorts,
        residence_index,
        residence_offsets: wire.residence_offsets,
        household_episodes,
        union_allocations,
        birth_allocations,
        death_allocations,
        residence_transfers,
        guardian_assignments,
        parentage,
        children_index: wire.children_index,
        annual_censuses: wire.annual_censuses,
        cohort_residence_counts,
        diagnostics: wire.diagnostics,
        validation_proof: wire.validation_proof,
    })
}
