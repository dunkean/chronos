//! Atlas régional figé : aucune partition démographique ni géométrie générée.
use super::canonical::canonical_value_bytes;
use super::ids::OwnerScope;
use super::model::{
    required_nullable, string_enum, ErrorCode, HistoryError, TimeSpan, MAX_ABS_COORDINATE_MM,
    MAX_ABS_YEAR,
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
    pub occupation_episodes: Vec<OccupationEpisode>,
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
/// Côté local d'un contact : le site d'attache et ce qu'il voit au-delà.
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
/// Contact à propriétaire unique et à deux extrémités (§2.1, §7.1). Les deux
/// côtés partagent cet ID ; chaque site d'attache peut y ouvrir son port (§12.2).
/// Les extrémités sont triées : site avant extérieur, puis par ID.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalContact {
    pub contact_id: String,
    pub owner_scope: OwnerScope,
    pub ends: [ContactEnd; 2],
    pub valid: TimeSpan,
}
impl ExternalContact {
    /// Extrémité ancrée à ce site, si le contact y aboutit.
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
    pub timeline: TimeSpan,
    pub cells: Vec<AtlasCell>,
    pub region_ids: Vec<String>,
    pub settlements: Vec<SettlementAnchor>,
    pub mobility_edges: Vec<MobilityEdge>,
    pub external_contacts: Vec<ExternalContact>,
}
/// IDs de catalogue et d'atlas : ASCII imprimable sans espace, comme les champs
/// d'adresse qu'ils alimentent (§4.1), et au plus 4 096 octets.
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
/// Épisodes ordonnés contigus, couvrant exactement l'intervalle fourni.
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
    /// Octets canoniques (§4.2) : listes triées par ID, épisodes d'occupation
    /// dans leur ordre. Les extrémités d'un contact sont déjà triées par `validate`.
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
    /// Toute erreur porte l'atlas comme propriétaire et sa date d'ouverture,
    /// sauf si la contrainte a déjà désigné plus précis (§13).
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
            return Err(bad("identité d'atlas vide, dupliquée ou trop longue"));
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
            return Err(bad("région absente"));
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
                return Err(bad("cellule, surface, bornes ou catalogue invalides"));
            }
            let mut seen = BTreeSet::from([c.cell_id.as_str()]);
            let mut current = c;
            while let Some(p) = &current.parent_id {
                if !seen.insert(p) {
                    return Err(bad("cycle de cellules"));
                }
                let parent = cells.get(p.as_str()).ok_or_else(|| bad("parent absent"))?;
                if current.coordinate_frame != parent.coordinate_frame
                    || !parent
                        .bounds
                        .contains(current.bounds.min_x_mm, current.bounds.min_y_mm)
                    || !parent
                        .bounds
                        .contains(current.bounds.max_x_mm, current.bounds.max_y_mm)
                {
                    return Err(bad("enfant hors du référentiel ou des bornes du parent"));
                }
                current = parent;
            }
            for n in &c.neighbors {
                if n == &c.cell_id
                    || !cells
                        .get(n.as_str())
                        .is_some_and(|other| other.neighbors.contains(&c.cell_id))
                {
                    return Err(bad("voisinage absent ou non réciproque"));
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
                return Err(bad("catalogue de sites incomplet"));
            }
            let total = expected
                .iter()
                .try_fold(0u64, |sum, id| {
                    sum.checked_add(sites[id].population_expected_max)
                })
                .ok_or_else(|| bad("population agrégée excessive"))?;
            if total != c.settlement_summary.population_expected_max {
                return Err(bad("résumé de population incohérent"));
            }
        }
        for s in &self.settlements {
            let c = cells
                .get(s.cell_id.as_str())
                .ok_or_else(|| bad("cellule de site absente"))?;
            if c.coordinate_frame != s.coordinate_frame
                || !c.bounds.contains(s.x_mm, s.y_mm)
                || s.foundation_year < -MAX_ABS_YEAR
                || s.foundation_year > self.timeline.start_year
                || s.initial_population > s.population_expected_max
                || !covers(s.occupation_episodes.iter().map(|e| e.valid), self.timeline)
                || s.occupation_episodes.iter().any(|e| {
                    e.capacity == 0
                        || e.state == OccupationState::Abandoned
                        || !valid_weights(&e.culture_weights)
                        || !valid_weights(&e.activity_weights)
                })
            {
                return Err(bad(
                    "ancrage, fondation ou épisodes invalides ; sites dynamiques non livrés",
                ));
            }
            if self
                .region_ids
                .iter()
                .filter(|r| self.cell_is_within(&s.cell_id, r))
                .count()
                != 1
            {
                return Err(bad("site sans région unique"));
            }
        }
        for e in &self.mobility_edges {
            validate_mobility_edge(e, self.timeline, &sites.keys().copied().collect())?;
        }
        for c in &self.external_contacts {
            c.valid.validate(false)?;
            let [first, second] = &c.ends;
            if !self.timeline.encloses(c.valid)
                // Tri strict : refuse aussi deux fois la même extrémité.
                || first >= second
                || !matches!(first, ContactEnd::Site(_))
                || !self.contact_end_is_valid(c, first, second, &sites)
                || !self.contact_end_is_valid(c, second, first, &sites)
            {
                return Err(bad("contact extérieur ou voie d'accès invalide"));
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
                    // Un contact relie plusieurs lieux : ni un site ni un domaine,
                    // que l'atlas ignore, ne peuvent en être le propriétaire unique.
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
    /// Recherche bornée même sur un atlas non validé.
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
            "liaison de mobilité invalide",
        ))
    } else {
        Ok(())
    }
}
