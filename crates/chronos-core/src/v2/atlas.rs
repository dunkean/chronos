//! Frozen regional atlas and demographic partition (§3.1), no generated geometry.
use super::canonical::canonical_value_bytes;
use super::ids::{DomainKey, OwnerScope, SemanticAddress};
use super::model::{
    required_nullable, string_enum, ErrorCode, HistoryError, Provenance, TimeSpan,
    MAX_ABS_COORDINATE_MM, MAX_ABS_YEAR,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricBounds {
    pub min_x_mm: i64,
    pub min_y_mm: i64,
    pub max_x_mm: i64,
    pub max_y_mm: i64,
}
impl MetricBounds {
    fn contains(&self, x: i64, y: i64) -> bool {
        self.min_x_mm <= x && x <= self.max_x_mm && self.min_y_mm <= y && y <= self.max_y_mm
    }
    fn within_coordinate_limits(&self) -> bool {
        [self.min_x_mm, self.min_y_mm, self.max_x_mm, self.max_y_mm]
            .iter()
            .all(|v| (-MAX_ABS_COORDINATE_MM..=MAX_ABS_COORDINATE_MM).contains(v))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementSummary {
    pub site_ids: Vec<String>,
    pub population_expected_max: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtlasCell {
    pub cell_id: String,
    pub parent_id: Option<String>,
    pub neighbors: Vec<String>,
    pub area_m2: u64,
    pub bounds: MetricBounds,
    pub coordinate_frame: String,
    pub settlement_summary: SettlementSummary,
}
string_enum!(OccupationState {
    Active,
    Ruined,
    Abandoned
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OccupationEpisode {
    pub valid: TimeSpan,
    pub state: OccupationState,
    pub capacity: u64,
    pub culture_weights: BTreeMap<String, u32>,
    pub activity_weights: BTreeMap<String, u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementAnchor {
    pub site_id: String,
    pub cell_id: String,
    pub coordinate_frame: String,
    pub x_mm: i64,
    pub y_mm: i64,
    pub foundation_year: i32,
    pub initial_population: u64,
    pub population_expected_max: u64,
    /// `availability` tags (§23.1), checked against the catalogue by the canon.
    pub site_tags: BTreeSet<String>,
    pub occupation_episodes: Vec<OccupationEpisode>,
}
impl SettlementAnchor {
    /// Occupation episode covering the year; a validated atlas covers them all.
    pub fn episode_at(&self, year: i32) -> Option<&OccupationEpisode> {
        self.occupation_episodes
            .iter()
            .find(|e| e.valid.contains(year))
    }
}
string_enum!(MobilityMode {
    Land,
    Water,
    Air,
    Mixed
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityEdge {
    pub edge_id: String,
    pub from_site_id: String,
    pub to_site_id: String,
    pub valid: TimeSpan,
    pub travel_days: u32,
    pub bidirectional: bool,
    pub mode: MobilityMode,
}
/// Local side of a contact: the attachment site and what it sees beyond.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactSiteEnd {
    pub site_id: String,
    pub external_context_id: String,
    #[serde(deserialize_with = "required_nullable")]
    pub access_edge_id: Option<String>,
    pub external_profile_id: String,
    pub population_expected: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContactEnd {
    Site(ContactSiteEnd),
    External { context_id: String },
}
/// Contact with a single owner and two endpoints (§2.1, §7.1). Both sides
/// share this ID; each attachment site may open its port on it (§12.2).
/// Endpoints are sorted: site before outside, then by ID.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalContact {
    pub contact_id: String,
    pub owner_scope: OwnerScope,
    pub ends: [ContactEnd; 2],
    pub valid: TimeSpan,
}
impl ExternalContact {
    /// Endpoint anchored at this site, if the contact ends there.
    pub fn site_end(&self, site_id: &str) -> Option<&ContactSiteEnd> {
        self.ends.iter().find_map(|end| match end {
            ContactEnd::Site(side) if side.site_id == site_id => Some(side),
            _ => None,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryAtlas {
    pub atlas_id: String,
    pub source_fingerprint: String,
    /// Atlas status: a virtual fixture declares itself `synthetic`.
    pub provenance: Provenance,
    pub timeline: TimeSpan,
    pub cells: Vec<AtlasCell>,
    pub region_ids: Vec<String>,
    pub settlements: Vec<SettlementAnchor>,
    pub mobility_edges: Vec<MobilityEdge>,
    pub external_contacts: Vec<ExternalContact>,
}
/// Catalogue and atlas IDs: printable ASCII without spaces, like the address
/// fields they feed (§4.1), and at most 4,096 bytes.
pub(crate) fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 4096 && id.bytes().all(|b| (0x21..=0x7e).contains(&b))
}
pub(crate) fn unique_ids<'a>(ids: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    ids.into_iter().all(|s| valid_id(s) && seen.insert(s))
}
pub(crate) fn valid_weights(weights: &BTreeMap<String, u32>) -> bool {
    !weights.is_empty()
        && weights
            .iter()
            .all(|(id, w)| valid_id(id) && *w <= 1_000_000)
        && weights.values().any(|w| *w > 0)
}
/// Contiguous ordered episodes, covering exactly the given interval.
pub(crate) fn covers(spans: impl IntoIterator<Item = TimeSpan>, period: TimeSpan) -> bool {
    let mut cursor = period.start_year;
    for s in spans {
        if s.validate(false).is_err() || s.start_year != cursor || s.end_year > period.end_year {
            return false;
        }
        cursor = s.end_year;
    }
    cursor == period.end_year
}
impl HistoryAtlas {
    /// Canonical bytes (§4.2): lists sorted by ID, occupation episodes in their
    /// own order. A contact's endpoints are already sorted by `validate`.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, HistoryError> {
        let mut a = self.clone();
        for cell in &mut a.cells {
            cell.neighbors.sort();
            cell.settlement_summary.site_ids.sort();
        }
        a.cells.sort_by(|x, y| x.cell_id.cmp(&y.cell_id));
        a.region_ids.sort();
        a.settlements.sort_by(|x, y| x.site_id.cmp(&y.site_id));
        a.mobility_edges.sort_by(|x, y| x.edge_id.cmp(&y.edge_id));
        a.external_contacts
            .sort_by(|x, y| x.contact_id.cmp(&y.contact_id));
        canonical_value_bytes(&a)
    }
    /// Every error carries the atlas as owner and its opening date, unless the
    /// constraint has already designated something more precise (§13).
    pub fn validate(&self) -> Result<(), HistoryError> {
        self.validate_inner().map_err(|mut e| {
            e.owner.get_or_insert_with(|| self.atlas_id.clone());
            e.at_year.get_or_insert(self.timeline.start_year);
            e
        })
    }
    fn validate_inner(&self) -> Result<(), HistoryError> {
        let bad = |s: &str| HistoryError::new(ErrorCode::InvalidAtlas, s);
        self.timeline.validate(false)?;
        if !valid_id(&self.atlas_id)
            || !valid_id(&self.source_fingerprint)
            || self.cells.is_empty()
            || !unique_ids(self.cells.iter().map(|c| c.cell_id.as_str()))
            || !unique_ids(self.region_ids.iter().map(String::as_str))
            || !unique_ids(self.settlements.iter().map(|s| s.site_id.as_str()))
            || !unique_ids(self.mobility_edges.iter().map(|e| e.edge_id.as_str()))
            || !unique_ids(self.external_contacts.iter().map(|c| c.contact_id.as_str()))
        {
            return Err(bad("atlas.bad_identity"));
        }
        let cells: BTreeMap<_, _> = self.cells.iter().map(|c| (c.cell_id.as_str(), c)).collect();
        let sites: BTreeMap<_, _> = self
            .settlements
            .iter()
            .map(|s| (s.site_id.as_str(), s))
            .collect();
        if self.region_ids.is_empty()
            || self
                .region_ids
                .iter()
                .any(|r| !cells.contains_key(r.as_str()))
        {
            return Err(bad("atlas.region_missing"));
        }
        for c in &self.cells {
            if c.area_m2 == 0
                || !valid_id(&c.coordinate_frame)
                || c.bounds.min_x_mm > c.bounds.max_x_mm
                || c.bounds.min_y_mm > c.bounds.max_y_mm
                || !c.bounds.within_coordinate_limits()
                || !unique_ids(c.neighbors.iter().map(String::as_str))
                || !unique_ids(c.settlement_summary.site_ids.iter().map(String::as_str))
            {
                return Err(bad("atlas.cell_invalid"));
            }
            let mut seen = BTreeSet::from([c.cell_id.as_str()]);
            let mut current = c;
            while let Some(p) = &current.parent_id {
                if !seen.insert(p) {
                    return Err(bad("atlas.cell_cycle"));
                }
                let parent = cells
                    .get(p.as_str())
                    .ok_or_else(|| bad("atlas.parent_missing"))?;
                if current.coordinate_frame != parent.coordinate_frame
                    || !parent
                        .bounds
                        .contains(current.bounds.min_x_mm, current.bounds.min_y_mm)
                    || !parent
                        .bounds
                        .contains(current.bounds.max_x_mm, current.bounds.max_y_mm)
                {
                    return Err(bad("atlas.child_out_of_bounds"));
                }
                current = parent;
            }
            for n in &c.neighbors {
                if n == &c.cell_id
                    || !cells
                        .get(n.as_str())
                        .is_some_and(|other| other.neighbors.contains(&c.cell_id))
                {
                    return Err(bad("atlas.neighbours_invalid"));
                }
            }
            let expected: BTreeSet<_> = self
                .settlements
                .iter()
                .filter(|s| self.cell_is_within(&s.cell_id, &c.cell_id))
                .map(|s| s.site_id.as_str())
                .collect();
            if expected
                != c.settlement_summary
                    .site_ids
                    .iter()
                    .map(String::as_str)
                    .collect()
            {
                return Err(bad("atlas.site_catalogue_incomplete"));
            }
            let total = expected
                .iter()
                .try_fold(0u64, |sum, id| {
                    sum.checked_add(sites[id].population_expected_max)
                })
                .ok_or_else(|| bad("atlas.population_excessive"))?;
            if total != c.settlement_summary.population_expected_max {
                return Err(bad("atlas.population_summary_inconsistent"));
            }
        }
        for s in &self.settlements {
            let c = cells
                .get(s.cell_id.as_str())
                .ok_or_else(|| bad("atlas.site_cell_missing"))?;
            if c.coordinate_frame != s.coordinate_frame
                || !c.bounds.contains(s.x_mm, s.y_mm)
                || s.foundation_year < -MAX_ABS_YEAR
                || s.foundation_year > self.timeline.start_year
                || !unique_ids(s.site_tags.iter().map(String::as_str))
                || s.initial_population > s.population_expected_max
                || !covers(s.occupation_episodes.iter().map(|e| e.valid), self.timeline)
                || s.occupation_episodes.iter().any(|e| {
                    e.capacity == 0
                        || e.state == OccupationState::Abandoned
                        || !valid_weights(&e.culture_weights)
                        || !valid_weights(&e.activity_weights)
                })
            {
                return Err(bad("atlas.anchor_invalid"));
            }
            if self
                .region_ids
                .iter()
                .filter(|r| self.cell_is_within(&s.cell_id, r))
                .count()
                != 1
            {
                return Err(bad("atlas.site_region_not_unique"));
            }
        }
        for e in &self.mobility_edges {
            validate_mobility_edge(e, self.timeline, &sites.keys().copied().collect())?;
        }
        for c in &self.external_contacts {
            c.valid.validate(false)?;
            let [first, second] = &c.ends;
            if !self.timeline.encloses(c.valid)
                // Strict sort: also rejects the same endpoint twice.
                || first >= second
                || !matches!(first, ContactEnd::Site(_))
                || !self.contact_end_is_valid(c, first, second, &sites)
                || !self.contact_end_is_valid(c, second, first, &sites)
            {
                return Err(bad("atlas.contact_invalid"));
            }
        }
        Ok(())
    }
    fn contact_end_is_valid(
        &self,
        contact: &ExternalContact,
        end: &ContactEnd,
        other: &ContactEnd,
        sites: &BTreeMap<&str, &SettlementAnchor>,
    ) -> bool {
        let side = match end {
            ContactEnd::External { context_id } => return valid_id(context_id),
            ContactEnd::Site(side) => side,
        };
        let context_matches = match other {
            ContactEnd::External { context_id } => context_id == &side.external_context_id,
            ContactEnd::Site(other) => other.site_id != side.site_id,
        };
        context_matches
            && sites.get(side.site_id.as_str()).is_some_and(|s| {
                match &contact.owner_scope {
                    OwnerScope::World => true,
                    OwnerScope::Cell { cell_id } => self.cell_is_within(&s.cell_id, cell_id),
                    // A contact links several places: neither a site nor a domain, which the
                    // atlas does not know, can be its single owner.
                    OwnerScope::Site { .. } | OwnerScope::Domain { .. } => false,
                }
            })
            && valid_id(&side.external_context_id)
            && valid_id(&side.external_profile_id)
            && side.population_expected <= 1_000_000_000_000
            && side.access_edge_id.as_ref().is_none_or(|id| {
                self.mobility_edges.iter().any(|e| {
                    &e.edge_id == id
                        && (e.from_site_id == side.site_id || e.to_site_id == side.site_id)
                        && e.valid.encloses(contact.valid)
                })
            })
    }
    /// Unique region of a site, on a validated atlas.
    pub fn region_of(&self, site: &SettlementAnchor) -> Option<&str> {
        self.region_ids
            .iter()
            .find(|r| self.cell_is_within(&site.cell_id, r))
            .map(String::as_str)
    }
    /// Shortest trips in days from a site, over routes valid at this date and in
    /// their direction (§26.1). An absent site is unreachable.
    pub fn travel_days_from(&self, origin: &str, year: i32) -> BTreeMap<String, u64> {
        let mut best = BTreeMap::from([(origin.to_owned(), 0)]);
        let mut frontier = BTreeSet::from([(0u64, origin.to_owned())]);
        while let Some((days, site)) = frontier.pop_first() {
            for edge in self
                .mobility_edges
                .iter()
                .filter(|e| e.valid.contains(year))
            {
                let next = if edge.from_site_id == site {
                    &edge.to_site_id
                } else if edge.bidirectional && edge.to_site_id == site {
                    &edge.from_site_id
                } else {
                    continue;
                };
                let total = days + u64::from(edge.travel_days);
                if best.get(next).is_none_or(|known| total < *known) {
                    best.insert(next.clone(), total);
                    frontier.insert((total, next.clone()));
                }
            }
        }
        best
    }
    /// Bounded search, even on an unvalidated atlas.
    pub fn cell_is_within(&self, cell: &str, ancestor: &str) -> bool {
        let mut current = cell;
        for _ in 0..=self.cells.len() {
            if current == ancestor {
                return true;
            }
            match self
                .cells
                .iter()
                .find(|c| c.cell_id == current)
                .and_then(|c| c.parent_id.as_deref())
            {
                Some(parent) => current = parent,
                None => return false,
            }
        }
        false
    }
}
/// Caps of a demonstration domain (§3.1, §17).
pub const MAX_DOMAIN_SITES: usize = 32;
pub const MAX_DOMAIN_POPULATION: u64 = 20_000;
/// Whole disjoint sites of a domain, fixed at bootstrap.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DemographicDomain {
    pub domain_id: String,
    pub region_id: String,
    pub demography_society_id: String,
    pub site_ids: Vec<String>,
    pub population_expected_max: u64,
}
/// Next-fit partition of §3.1: per region and social profile, sites sorted by
/// cell then ID. A closed domain is never reopened. Receives each site's resolved
/// profile; political borders play no part.
pub fn partition_domains(
    atlas: &HistoryAtlas,
    world: &str,
    site_society: &BTreeMap<String, String>,
) -> Result<Vec<DemographicDomain>, HistoryError> {
    let mut groups: BTreeMap<(&str, &str), Vec<&SettlementAnchor>> = BTreeMap::new();
    for site in &atlas.settlements {
        let located = |key: &str| HistoryError {
            owner: Some(site.site_id.clone()),
            at_year: Some(atlas.timeline.start_year),
            ..HistoryError::new(ErrorCode::InvalidAtlas, key)
        };
        let region = atlas
            .region_of(site)
            .ok_or_else(|| located("domain.site_without_region"))?;
        let society = site_society
            .get(&site.site_id)
            .ok_or_else(|| located("domain.site_without_society"))?;
        if site.population_expected_max > MAX_DOMAIN_POPULATION {
            return Err(HistoryError {
                code: ErrorCode::BudgetExceeded,
                ..located("domain.site_over_population_cap")
            });
        }
        groups.entry((region, society)).or_default().push(site);
    }
    let mut domains = Vec::new();
    for ((region, society), mut sites) in groups {
        sites.sort_by(|a, b| (&a.cell_id, &a.site_id).cmp(&(&b.cell_id, &b.site_id)));
        let mut ordinal = 0;
        let mut current: Vec<&SettlementAnchor> = Vec::new();
        let mut population = 0;
        let mut close = |sites: &[&SettlementAnchor], population| -> Result<(), HistoryError> {
            let key = DomainKey::new(world, region, society, ordinal)?;
            ordinal += 1;
            domains.push(DemographicDomain {
                domain_id: SemanticAddress::domain(&key)?.encode(),
                region_id: region.into(),
                demography_society_id: society.into(),
                site_ids: sites.iter().map(|s| s.site_id.clone()).collect(),
                population_expected_max: population,
            });
            Ok(())
        };
        for site in sites {
            if current.len() == MAX_DOMAIN_SITES
                || population + site.population_expected_max > MAX_DOMAIN_POPULATION
            {
                close(&current, population)?;
                current.clear();
                population = 0;
            }
            current.push(site);
            population += site.population_expected_max;
        }
        close(&current, population)?;
    }
    Ok(domains)
}
pub(crate) fn validate_mobility_edge(
    e: &MobilityEdge,
    timeline: TimeSpan,
    sites: &BTreeSet<&str>,
) -> Result<(), HistoryError> {
    e.valid.validate(false)?;
    if !valid_id(&e.edge_id)
        || !sites.contains(e.from_site_id.as_str())
        || !sites.contains(e.to_site_id.as_str())
        || e.from_site_id == e.to_site_id
        || !timeline.encloses(e.valid)
        || !(1..=10_000).contains(&e.travel_days)
    {
        Err(HistoryError::new(
            ErrorCode::InvalidAtlas,
            "atlas.mobility_link_invalid",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::model::ProvenanceStatus;
    fn partition(populations: &[u64]) -> Result<Vec<DemographicDomain>, HistoryError> {
        let id = |i: usize| format!("site/{i}");
        let settlements: Vec<_> = populations
            .iter()
            .enumerate()
            .map(|(i, p)| SettlementAnchor {
                site_id: id(i),
                cell_id: "region".into(),
                coordinate_frame: "m".into(),
                x_mm: 0,
                y_mm: 0,
                foundation_year: 1200,
                initial_population: *p / 2,
                population_expected_max: *p,
                site_tags: BTreeSet::new(),
                occupation_episodes: vec![],
            })
            .collect();
        let atlas = HistoryAtlas {
            atlas_id: "a".into(),
            source_fingerprint: "f".into(),
            provenance: Provenance {
                status: ProvenanceStatus::Synthetic,
                source_ref: "test".into(),
                rule_id: "test".into(),
            },
            timeline: TimeSpan {
                start_year: 1300,
                end_year: 1400,
            },
            cells: vec![AtlasCell {
                cell_id: "region".into(),
                parent_id: None,
                neighbors: vec![],
                area_m2: 1,
                bounds: MetricBounds {
                    min_x_mm: 0,
                    min_y_mm: 0,
                    max_x_mm: 0,
                    max_y_mm: 0,
                },
                coordinate_frame: "m".into(),
                settlement_summary: SettlementSummary {
                    site_ids: vec![],
                    population_expected_max: 0,
                },
            }],
            region_ids: vec!["region".into()],
            settlements,
            mobility_edges: vec![],
            external_contacts: vec![],
        };
        let society = (0..populations.len())
            .map(|i| (id(i), "society/a".to_owned()))
            .collect();
        partition_domains(&atlas, "w", &society)
    }
    #[test]
    fn next_fit_gives_15000_then_20000_without_reopening() {
        let domains = partition(&[15_000, 15_000, 5_000]).unwrap();
        let sizes: Vec<_> = domains.iter().map(|d| d.population_expected_max).collect();
        assert_eq!(sizes, [15_000, 20_000]);
        assert_eq!(domains[1].site_ids, ["site/1", "site/2"]);
    }
    #[test]
    fn next_fit_closes_a_domain_at_32_sites() {
        let domains = partition(&[10; 33]).unwrap();
        assert_eq!(domains[0].site_ids.len(), MAX_DOMAIN_SITES);
        assert_eq!(domains[1].site_ids.len(), 1);
    }
    #[test]
    fn site_beyond_20000_is_refused() {
        assert!(partition(&[20_000]).is_ok());
        assert_eq!(
            partition(&[20_001]).unwrap_err().code,
            ErrorCode::BudgetExceeded
        );
    }
}
