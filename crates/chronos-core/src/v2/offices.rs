//! Offices, tenures and the dated answer to "who held it" (§10, §25.5, §26.4).
use super::model::{string_enum, ErrorCode, HistoryError, Provenance, TimeSpan};
use super::population::SlotSource;
use super::recruit::Fits;
use super::social::SocialPacket;
use serde::{Deserialize, Serialize};

pub const MAYOR_ROLE: &str = "mayor";
pub const COUNCIL_ROLE: &str = "council";
pub const HEAD_ROLE: &str = "head";
/// A person holds one personal charge at a time on a site.
pub const SINGLE_PERSONAL_CHARGE: &str = "single_personal_charge";

string_enum!(AppointmentRule {
    Hereditary,
    Seniority,
    Appointment,
    WeightedElection,
    CollectiveBody
});
string_enum!(TenureMode {
    Effective,
    Claimed,
    Regent,
    Vacant
});
string_enum!(TenureEnd {
    Death,
    Departure,
    TermEnd,
    Ineligible,
    Dissolution,
    Filled
});
string_enum!(SelectionBinding {
    InitialObservedBinding,
    FoundingSelection,
    Succession,
    KinSuccession,
    NoCandidate
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfficeRequirements {
    pub min_age: u32,
    pub max_age: u32,
    /// A local charge requires residence on this site (§25.5).
    pub residence_site_id: String,
    /// An office of the jurisdiction refuses its outlaws (§27.5); the private charge
    /// of a group does not.
    pub jurisdiction_bound: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfficeDefinition {
    pub office_id: String,
    pub organization_id: String,
    pub site_id: String,
    pub role_key: String,
    pub title_profile: String,
    pub collective: bool,
    pub seats: u32,
    pub appointment_rule: AppointmentRule,
    pub requirements: OfficeRequirements,
    /// `None`: held for life.
    pub term_years: Option<u32>,
    pub concurrent_holding_policy: String,
    pub time_cost_bps: u16,
    pub required: bool,
    pub priority: u8,
    pub valid: TimeSpan,
    /// `max(foundation, world start)` (§26.4): nobody is invented before it.
    pub observed_from: i32,
    pub provenance: Provenance,
}
/// Why this tenure exists, without listing thousands of candidates (§15.4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SelectionProof {
    pub rule: AppointmentRule,
    pub binding: SelectionBinding,
    pub year: i32,
    pub vacancy_ordinal: u32,
    pub draw_ordinal: u32,
    pub candidates: u32,
    pub total_weight: u64,
    pub selected_weight: u64,
    pub draw: u64,
    pub fits: Fits,
    pub alignment_fit: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Tenure {
    pub tenure_id: String,
    pub office_id: String,
    pub tenure_ordinal: u32,
    pub person_id: Option<String>,
    pub person_slot: Option<u32>,
    pub valid: TimeSpan,
    pub mode: TenureMode,
    pub selection_proof: SelectionProof,
    pub cause_event_id: Option<String>,
    /// `None` while the tenure is still open at the end of the horizon.
    pub closed_by: Option<TenureEnd>,
}
string_enum!(OfficeStatus {
    Held,
    Vacant,
    Collective,
    NotObserved,
    NotInExistence
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfficeHolder {
    pub person_id: String,
    pub person_slot: u32,
    pub source: SlotSource,
    pub birth_year: i32,
    pub death_year: Option<i32>,
    pub status_id: String,
    pub activity_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfficeView {
    pub office: OfficeDefinition,
    pub at_year: i32,
    pub status: OfficeStatus,
    pub tenure: Option<Tenure>,
    pub holder: Option<OfficeHolder>,
}
/// Dated reading of a full-horizon packet: the sequence of tenures is already
/// resolved, so the year asked never draws anything. A vacant or collective office
/// says so; no replacement person is invented (§10).
pub fn resolve_office(
    packet: &SocialPacket,
    office_id: &str,
    at_year: i32,
) -> Result<OfficeView, HistoryError> {
    let office = packet
        .offices
        .iter()
        .find(|o| o.office_id == office_id)
        .ok_or_else(|| HistoryError {
            owner: Some(office_id.into()),
            at_year: Some(at_year),
            ..HistoryError::new(ErrorCode::InvalidRecordIdentity, "office.unknown")
        })?
        .clone();
    let tenure = packet
        .tenures
        .iter()
        .find(|t| t.office_id == office_id && t.valid.contains(at_year))
        .cloned();
    let status = if !office.valid.contains(at_year) {
        OfficeStatus::NotInExistence
    } else if office.collective {
        OfficeStatus::Collective
    } else if at_year < office.observed_from || at_year >= packet.horizon_end {
        OfficeStatus::NotObserved
    } else {
        match tenure.as_ref().map(|t| t.mode) {
            Some(TenureMode::Effective | TenureMode::Regent) => OfficeStatus::Held,
            _ => OfficeStatus::Vacant,
        }
    };
    let holder = tenure.as_ref().and_then(|t| t.person_slot).map(|slot| {
        let person = &packet.people[slot as usize];
        let life = &packet.lives[slot as usize];
        OfficeHolder {
            person_id: person.person_id.clone(),
            person_slot: slot,
            source: life.source,
            birth_year: life.birth_year,
            death_year: life.death_year,
            status_id: person.status_id.clone(),
            activity_id: person.activity_id.clone(),
        }
    });
    Ok(OfficeView {
        office,
        at_year,
        status,
        tenure,
        holder,
    })
}
/// The municipal head of a site, the office of `maire(village, year)`.
pub fn mayor_office<'a>(packet: &'a SocialPacket, site_id: &str) -> Option<&'a OfficeDefinition> {
    packet
        .offices
        .iter()
        .find(|o| o.site_id == site_id && o.role_key == MAYOR_ROLE)
}
