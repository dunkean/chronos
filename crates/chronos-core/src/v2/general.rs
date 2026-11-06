//! General canon (§7.1, §14.1): salient collectives and events on decade blocks,
//! shock-free population baseline, contacts. A region reads nothing from another
//! one, and every draw carries a fixed semantic key.
use super::atlas::{partition_domains, ContactEnd, DemographicDomain, HistoryAtlas};
use super::canonical::canonical_value_bytes;
use super::catalog::{HistoryCatalogue, SettingProfile};
use super::ids::random_semantics_v2::uniform_below;
use super::ids::{decade_block, Owner, SemanticAddress};
use super::model::{
    Agency, EntityNature, ErrorCode, EventPhase, HistoryError, HistoryEvent, ObjectRef, Predicate,
    Provenance, ProvenanceStatus, SocialEntity, SocialRelation, TimeSpan,
};
use super::population::{
    AncestryProfile, CatalogueFingerprints, ExternalProfile, KinshipProfile, PopulationLimits,
    PopulationProvider, Regulation, SocialClass,
};
use super::region::{ExposureModel, ImpactContract, ImpactMode, ImpactTarget, TargetSelector};
use super::validate::{causal_ranks, validate_composition};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Process parameters of §7.1, all explicit in the manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessParameters {
    pub growth_ppm: i64,
    /// Per adjacent political pair and free decade.
    pub war_bps: u16,
    pub war_min_years: u32,
    pub war_max_years: u32,
    pub war_site_exposure_bps: u16,
    /// Per contested site, drawn at peace.
    pub control_change_bps: u16,
    /// Per region and decade.
    pub epidemic_bps: u16,
    pub epidemic_cooldown_years: u32,
    pub epidemic_min_years: u32,
    pub epidemic_max_years: u32,
    pub epidemic_site_exposure_bps: u16,
    /// Annual port rate: base plus at most the spread, per contact and block.
    pub port_base_rate_bps: u16,
    pub port_rate_spread_bps: u16,
    /// Per contact and decade.
    pub diplomatic_contact_bps: u16,
    /// Per ruined site.
    pub displacement_bps: u16,
    pub displacement_share_bps: u16,
    pub expected_household_size: u64,
    /// Per jurisdiction, legal act kind and decade.
    pub legal_event_bps: u16,
}
/// Inline profiles and bounds copied into every plan (§24.1, §26.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationDefaults {
    pub ancestries: Vec<AncestryProfile>,
    pub social_classes: Vec<SocialClass>,
    pub external: Vec<ExternalProfile>,
    pub kinship: KinshipProfile,
    pub ancestry_weights: BTreeMap<String, u64>,
    pub baseline_tolerance_bps: u16,
    pub limits: PopulationLimits,
    pub regulation: Regulation,
}
/// Resolved manifest configuration (§2.1). `canon_id` and `society_ids`
/// (template to `demography_society_id`) arrive already computed by the host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldConfig {
    pub canon_id: String,
    pub world_seed: u64,
    pub timeline: TimeSpan,
    pub setting_profile: SettingProfile,
    pub provider: PopulationProvider,
    pub catalogue_fingerprints: CatalogueFingerprints,
    pub society_ids: BTreeMap<String, String>,
    pub processes: ProcessParameters,
    pub population: PopulationDefaults,
}
/// Author constraints delivered in S2: explicit baseline series per site (§26.5),
/// `year -> population`, over a whole region and on the marker date grid.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorConstraints {
    pub site_baselines: BTreeMap<String, BTreeMap<i32, u64>>,
}

/// Collective governing a child cell of its region, jurisdiction of its sites.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Polity {
    pub entity: SocialEntity,
    pub cell_id: String,
    pub region_id: String,
    pub history_family_id: String,
    pub seat_site_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ControlEpisode {
    pub polity_id: String,
    pub valid: TimeSpan,
    pub cause_event_id: Option<String>,
}
/// Shock-free baseline of a region: per culture and year from the world's start,
/// and the total on the date grid. Post-crisis forecasts never enter it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RegionBaseline {
    pub by_culture: BTreeMap<String, Vec<u64>>,
    pub grid: BTreeMap<i32, u64>,
}
/// Expected flow of a contact over a block, shared by both sides (§12.2).
/// `forward` runs from the first end to the second.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct FlowEpisode {
    pub valid: TimeSpan,
    pub forward_rate_bps: u16,
    pub backward_rate_bps: u16,
}
#[derive(Clone, Debug, Serialize)]
pub struct GeneralCanon {
    pub canon_id: String,
    pub world_seed: u64,
    pub timeline: TimeSpan,
    pub domains: Vec<DemographicDomain>,
    pub polities: Vec<Polity>,
    pub baselines: BTreeMap<String, RegionBaseline>,
    pub site_control: BTreeMap<String, Vec<ControlEpisode>>,
    pub contact_flows: BTreeMap<String, Vec<FlowEpisode>>,
    pub events: Vec<HistoryEvent>,
    pub contracts: Vec<ImpactContract>,
    pub relations: Vec<SocialRelation>,
    /// Frozen inputs, outside the canonical projection: the manifest identifies them.
    #[serde(skip)]
    pub config: WorldConfig,
    #[serde(skip)]
    pub atlas: HistoryAtlas,
    #[serde(skip)]
    pub catalogue: HistoryCatalogue,
    #[serde(skip)]
    pub constraints: AuthorConstraints,
}
pub(crate) fn fact_provenance(atlas: &HistoryAtlas, rule_id: &str) -> Provenance {
    Provenance {
        // A fact drawn from a virtual atlas stays synthetic (§4.3).
        status: match atlas.provenance.status {
            ProvenanceStatus::Synthetic => ProvenanceStatus::Synthetic,
            _ => ProvenanceStatus::Derived,
        },
        source_ref: atlas.atlas_id.clone(),
        rule_id: rule_id.into(),
    }
}
impl GeneralCanon {
    pub(crate) fn provenance(&self, rule_id: &str) -> Provenance {
        fact_provenance(&self.atlas, rule_id)
    }
    /// Canonical bytes: `build_canon` has already sorted the lists.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, HistoryError> {
        canonical_value_bytes(self)
    }
    /// Collective controlling a site at this date.
    pub fn controller_at(&self, site_id: &str, year: i32) -> Option<&Polity> {
        let id = &self
            .site_control
            .get(site_id)?
            .iter()
            .find(|e| e.valid.contains(year))?
            .polity_id;
        self.polities.iter().find(|p| &p.entity.id == id)
    }
    pub fn domain_of_site(&self, site_id: &str) -> Option<&DemographicDomain> {
        self.domains
            .iter()
            .find(|d| d.site_ids.iter().any(|s| s == site_id))
    }
}

/// First year, decade boundaries and last year (§26.5). These dates depend on no
/// crisis.
pub fn marker_dates(timeline: TimeSpan) -> BTreeSet<i32> {
    let last = timeline.end_year - 1;
    let mut dates = BTreeSet::from([timeline.start_year, last]);
    dates.extend((timeline.start_year..timeline.end_year).filter(|y| y.rem_euclid(10) == 0));
    dates
}
/// Integer allocation by largest remainders, ties broken by ID. `weights` follows
/// ID order, and so does the result.
pub(crate) fn largest_remainder(total: u64, weights: &[(&str, u128)]) -> Vec<u64> {
    let sum: u128 = weights.iter().map(|(_, w)| w).sum();
    if sum == 0 {
        return vec![0; weights.len()];
    }
    let scaled: Vec<_> = weights.iter().map(|(_, w)| u128::from(total) * w).collect();
    let mut shares: Vec<_> = scaled.iter().map(|n| (n / sum) as u64).collect();
    let mut order: Vec<_> = (0..weights.len()).collect();
    order.sort_by_key(|i| (std::cmp::Reverse(scaled[*i] % sum), weights[*i].0));
    let left = total - shares.iter().sum::<u64>();
    for i in order.into_iter().take(left as usize) {
        shares[i] += 1;
    }
    shares
}
/// One `Pbase` step (§7.1) with a signed remainder. A positive rate is bounded by
/// capacity, a negative rate is kept; at zero population the remainder resets.
pub fn baseline_step(population: u64, remainder: &mut i128, growth_ppm: i64, capacity: u64) -> u64 {
    let (p, cap, growth) = (
        i128::from(population),
        i128::from(capacity),
        i128::from(growth_ppm),
    );
    let effective = if growth > 0 {
        growth * (cap - p).max(0) / cap.max(1)
    } else {
        growth
    };
    let n = *remainder + p * effective;
    *remainder = n.rem_euclid(1_000_000);
    let next = (p + n.div_euclid(1_000_000)).max(0);
    if next == 0 {
        *remainder = 0;
    }
    next as u64
}
pub(crate) fn gate(seed: u64, parts: &[&str], bps: u16) -> Result<bool, HistoryError> {
    Ok(uniform_below(seed, parts, 0, 10_000)? < u64::from(bps))
}
/// Decade blocks covering the period; a partial block keeps its full key.
pub(crate) fn blocks(timeline: TimeSpan) -> impl Iterator<Item = i32> {
    (decade_block(timeline.start_year)..timeline.end_year).step_by(10)
}
/// Year inside a block, under its fixed subkey. Dropped outside the period.
pub(crate) fn date_in_block(
    seed: u64,
    parts: &[&str],
    block: i32,
    timeline: TimeSpan,
) -> Result<Option<i32>, HistoryError> {
    let year = block + uniform_below(seed, parts, 0, 10)? as i32;
    Ok(timeline.contains(year).then_some(year))
}
fn uniform_span(seed: u64, parts: &[&str], min: u32, max: u32) -> Result<u32, HistoryError> {
    Ok(min + uniform_below(seed, parts, 0, u64::from(max - min) + 1)? as u32)
}

/// Numbers events per owner, kind and block. Each region and site has its own
/// counters, so a neighbour never shifts them.
pub(crate) struct EventLedger {
    next: BTreeMap<String, u32>,
    pub(crate) events: Vec<HistoryEvent>,
}
pub(crate) struct NewEvent<'a> {
    pub owner: &'a Owner,
    pub kind: &'a str,
    pub year: i32,
    pub end_year: Option<i32>,
    pub phase: EventPhase,
    pub causes: Vec<String>,
    pub participants: Vec<ObjectRef>,
    pub provenance: Provenance,
}
impl EventLedger {
    pub(crate) fn new() -> Self {
        Self {
            next: BTreeMap::new(),
            events: Vec::new(),
        }
    }
    pub(crate) fn add(&mut self, e: NewEvent) -> Result<SemanticAddress, HistoryError> {
        let block = decade_block(e.year);
        let key = SemanticAddress::event(e.owner, e.kind, block, 0)?.encode();
        let ordinal = self.next.entry(key).or_insert(0);
        let address = SemanticAddress::event(e.owner, e.kind, block, *ordinal)?;
        *ordinal += 1;
        self.events.push(HistoryEvent {
            id: address.encode(),
            owner_scope: e.owner.scope().clone(),
            kind: e.kind.into(),
            year: e.year,
            end_year: e.end_year,
            phase: e.phase,
            causes: e.causes,
            participants: e.participants,
            provenance: e.provenance,
        });
        Ok(address)
    }
}

fn invalid(key: &str) -> HistoryError {
    HistoryError::new(ErrorCode::InvalidCanonicalValue, key)
}
/// Profile of each site. All variants of non-zero weight must resolve the same
/// profile: a mix is an S8 capability (§26.3).
fn site_societies(
    config: &WorldConfig,
    atlas: &HistoryAtlas,
    catalogue: &HistoryCatalogue,
) -> Result<BTreeMap<String, String>, HistoryError> {
    let mut societies = BTreeMap::new();
    for site in &atlas.settlements {
        let mut found = BTreeSet::new();
        for episode in &site.occupation_episodes {
            for (culture, weight) in &episode.culture_weights {
                let template = &catalogue.binding(culture)?.demography_society_template_id;
                let society = config
                    .society_ids
                    .get(template)
                    .ok_or_else(|| HistoryError {
                        owner: Some(template.clone()),
                        ..invalid("canon.society_id_not_provided")
                    })?;
                if *weight > 0 {
                    found.insert(society.clone());
                }
            }
        }
        if found.len() != 1 {
            return Err(HistoryError {
                owner: Some(site.site_id.clone()),
                at_year: Some(atlas.timeline.start_year),
                ..HistoryError::new(
                    ErrorCode::UnsupportedCapability,
                    "canon.mixed_societies_on_site",
                )
            });
        }
        societies.insert(
            site.site_id.clone(),
            found.pop_first().expect("one profile"),
        );
    }
    Ok(societies)
}
/// Principal variant of a site: largest opening weight, then smallest ID.
pub(crate) fn principal_culture(atlas: &HistoryAtlas, site_id: &str) -> Option<String> {
    let site = atlas.settlements.iter().find(|s| s.site_id == site_id)?;
    let episode = site.episode_at(atlas.timeline.start_year)?;
    episode
        .culture_weights
        .iter()
        .max_by_key(|(id, w)| (**w, std::cmp::Reverse(id.as_str())))
        .map(|(id, _)| id.clone())
}
/// One jurisdiction per child cell of the region holding sites, or the region
/// itself for sites placed directly on it.
fn territory(atlas: &HistoryAtlas, region: &str, site_cell: &str) -> String {
    atlas
        .cells
        .iter()
        .find(|c| {
            c.parent_id.as_deref() == Some(region) && atlas.cell_is_within(site_cell, &c.cell_id)
        })
        .map_or_else(|| region.to_owned(), |c| c.cell_id.clone())
}

struct WarRun {
    event_id: String,
    pair: (String, String),
    end: i32,
}
/// Annual run of a region: a war reads the control state of its own date. Every
/// draw depends on its keys only.
struct RegionRun<'a> {
    config: &'a WorldConfig,
    atlas: &'a HistoryAtlas,
    region: &'a str,
    owner: Owner,
    polities: BTreeMap<String, Polity>,
    control: BTreeMap<String, Vec<ControlEpisode>>,
    ledger: EventLedger,
    contracts: Vec<ImpactContract>,
    relations: Vec<SocialRelation>,
}
impl RegionRun<'_> {
    fn relate(
        &mut self,
        predicate: Predicate,
        subject: ObjectRef,
        object: ObjectRef,
        valid: TimeSpan,
        cause_event_id: Option<String>,
    ) -> Result<(), HistoryError> {
        let id = SemanticAddress::relation(
            &self.owner,
            predicate,
            &subject,
            &object,
            valid.start_year,
            0,
        )?;
        self.relations.push(SocialRelation {
            id: id.encode(),
            subject,
            predicate,
            object,
            valid,
            role_id: None,
            weight_bps: None,
            time_cost_bps: None,
            cause_event_id,
            provenance: fact_provenance(self.atlas, "general/relation/v1"),
            rule_id: "general/relation/v1".into(),
        });
        Ok(())
    }
    fn polity_ref(&self, cell: &str) -> ObjectRef {
        ObjectRef::Entity(self.polities[cell].entity.id.clone())
    }
    fn sites_of(&self, polity_id: &str, year: i32) -> Vec<String> {
        self.control
            .iter()
            .filter(|(_, episodes)| {
                episodes
                    .iter()
                    .any(|e| e.valid.contains(year) && e.polity_id == polity_id)
            })
            .map(|(site, _)| site.clone())
            .collect()
    }
    fn impact(
        &mut self,
        event: &SemanticAddress,
        valid: TimeSpan,
        selector: TargetSelector,
        probability_bps: u16,
        profiles: [&str; 4],
    ) -> Result<(), HistoryError> {
        let [severity_profile, timing_profile, channel, cause] = profiles.map(String::from);
        self.contracts.push(ImpactContract {
            id: SemanticAddress::impact(event, "sites")?.encode(),
            parent_event_id: event.encode(),
            owner_scope: self.owner.scope().clone(),
            valid,
            target: ImpactTarget::Sites,
            selector,
            eligibility_time: valid.start_year,
            mode: ImpactMode::Probability { probability_bps },
            exposure_model: ExposureModel::IndependentTargetV1,
            severity_profile,
            timing_profile,
            channel,
            cause,
        });
        Ok(())
    }
    fn start_war(&mut self, pair: &(String, String), year: i32) -> Result<WarRun, HistoryError> {
        let p = &self.config.processes;
        let timeline = self.config.timeline;
        let block = decade_block(year).to_string();
        let keys = [
            "general/war/v1",
            self.region,
            &pair.0,
            &pair.1,
            &block,
            "duration",
        ];
        let duration = uniform_span(
            self.config.world_seed,
            &keys,
            p.war_min_years,
            p.war_max_years,
        )?;
        let end = year + duration as i32;
        let valid = TimeSpan {
            start_year: year,
            end_year: end.min(timeline.end_year),
        };
        let parties = vec![self.polity_ref(&pair.0), self.polity_ref(&pair.1)];
        let event = self.ledger.add(NewEvent {
            owner: &self.owner,
            kind: "war",
            year,
            end_year: Some(valid.end_year),
            phase: EventPhase::ContextAndShocks,
            causes: vec![],
            participants: parties.clone(),
            provenance: fact_provenance(self.atlas, "general/war/v1"),
        })?;
        let mut sites = BTreeSet::new();
        for cell in [&pair.0, &pair.1] {
            sites.extend(self.sites_of(&self.polities[cell].entity.id, year));
        }
        let exposure = p.war_site_exposure_bps;
        self.impact(
            &event,
            valid,
            TargetSelector::Ids(sites),
            exposure,
            ["war.ruin", "war.arrival.v1", "destruction", "war"],
        )?;
        let [first, second] = <[ObjectRef; 2]>::try_from(parties).expect("two parties");
        self.relate(
            Predicate::InConflictWith,
            first,
            second,
            valid,
            Some(event.encode()),
        )?;
        Ok(WarRun {
            event_id: event.encode(),
            pair: pair.clone(),
            end,
        })
    }
    /// Treaty at peace, then one draw per contested site: a `frontier` site of one
    /// party may pass to the other, with the treaty as cause (§7.1).
    fn end_war(&mut self, war: &WarRun, year: i32) -> Result<(), HistoryError> {
        let parties = vec![self.polity_ref(&war.pair.0), self.polity_ref(&war.pair.1)];
        let treaty = self.ledger.add(NewEvent {
            owner: &self.owner,
            kind: "treaty",
            year,
            end_year: None,
            phase: EventPhase::ContextAndShocks,
            causes: vec![war.event_id.clone()],
            participants: parties,
            provenance: fact_provenance(self.atlas, "general/treaty/v1"),
        })?;
        let (first, second) = (
            self.polities[&war.pair.0].entity.id.clone(),
            self.polities[&war.pair.1].entity.id.clone(),
        );
        let held: Vec<_> = self
            .sites_of(&first, year)
            .into_iter()
            .map(|site| (site, &second))
            .chain(
                self.sites_of(&second, year)
                    .into_iter()
                    .map(|site| (site, &first)),
            )
            .collect();
        for (site_id, winner) in held {
            let site = self
                .atlas
                .settlements
                .iter()
                .find(|s| s.site_id == site_id)
                .expect("controlled site");
            let keys = [
                "general/control/v1",
                war.event_id.as_str(),
                site_id.as_str(),
            ];
            if site.site_tags.contains("frontier")
                && gate(
                    self.config.world_seed,
                    &keys,
                    self.config.processes.control_change_bps,
                )?
            {
                self.transfer(&site_id, winner, year, &treaty.encode());
            }
        }
        Ok(())
    }
    fn transfer(&mut self, site: &str, polity_id: &str, year: i32, cause: &str) {
        let end_year = self.config.timeline.end_year;
        let episodes = self.control.get_mut(site).expect("controlled site");
        let last = episodes.last_mut().expect("open episode");
        if last.valid.start_year == year {
            // Two treaties in one year: only the last control remains.
            episodes.pop();
        } else {
            last.valid.end_year = year;
        }
        if let Some(previous) = episodes.last_mut().filter(|e| e.polity_id == polity_id) {
            previous.valid.end_year = end_year;
            return;
        }
        episodes.push(ControlEpisode {
            polity_id: polity_id.into(),
            valid: TimeSpan {
                start_year: year,
                end_year,
            },
            cause_event_id: Some(cause.into()),
        });
    }
    fn start_epidemic(&mut self, year: i32) -> Result<i32, HistoryError> {
        let p = &self.config.processes;
        let block = decade_block(year).to_string();
        let keys = ["general/epidemic/v1", self.region, &block, "duration"];
        let duration = uniform_span(
            self.config.world_seed,
            &keys,
            p.epidemic_min_years,
            p.epidemic_max_years,
        )?;
        let end = (year + duration as i32).min(self.config.timeline.end_year);
        let event = self.ledger.add(NewEvent {
            owner: &self.owner,
            kind: "epidemic",
            year,
            end_year: Some(end),
            phase: EventPhase::ContextAndShocks,
            causes: vec![],
            participants: vec![],
            provenance: fact_provenance(self.atlas, "general/epidemic/v1"),
        })?;
        let exposure = p.epidemic_site_exposure_bps;
        self.impact(
            &event,
            TimeSpan {
                start_year: year,
                end_year: end,
            },
            TargetSelector::All,
            exposure,
            [
                "plague.tiered.v1",
                "plague.frame.v1",
                "contagion",
                "epidemic",
            ],
        )?;
        Ok(end)
    }
    fn run(&mut self, pairs: &[(String, String)]) -> Result<(), HistoryError> {
        let timeline = self.config.timeline;
        let seed = self.config.world_seed;
        let p = self.config.processes.clone();
        let mut pending_wars: BTreeMap<i32, Vec<(String, String)>> = BTreeMap::new();
        let mut pending_epidemic = None;
        let mut active: Vec<WarRun> = Vec::new();
        let mut epidemic_free_from = timeline.start_year;
        for year in timeline.start_year..timeline.end_year {
            if year == timeline.start_year || year.rem_euclid(10) == 0 {
                let block = decade_block(year);
                let text = block.to_string();
                for pair in pairs {
                    // An active war bars the pair for this block.
                    if active.iter().any(|w| &w.pair == pair) {
                        continue;
                    }
                    let keys =
                        |leaf| ["general/war/v1", self.region, &pair.0, &pair.1, &text, leaf];
                    if gate(seed, &keys("gate"), p.war_bps)? {
                        if let Some(date) = date_in_block(seed, &keys("date"), block, timeline)? {
                            pending_wars.entry(date).or_default().push(pair.clone());
                        }
                    }
                }
                let keys = |leaf| ["general/epidemic/v1", self.region, &text, leaf];
                if gate(seed, &keys("gate"), p.epidemic_bps)? {
                    pending_epidemic = date_in_block(seed, &keys("date"), block, timeline)?
                        .filter(|date| *date >= epidemic_free_from);
                }
            }
            let (ended, running): (Vec<_>, Vec<_>) = active.drain(..).partition(|w| w.end == year);
            active = running;
            for war in &ended {
                self.end_war(war, year)?;
            }
            for pair in pending_wars.remove(&year).unwrap_or_default() {
                active.push(self.start_war(&pair, year)?);
            }
            if pending_epidemic == Some(year) {
                pending_epidemic = None;
                let end = self.start_epidemic(year)?;
                epidemic_free_from = end + p.epidemic_cooldown_years as i32;
            }
        }
        for (site, episodes) in self.control.clone() {
            for episode in episodes {
                let polity = ObjectRef::Entity(episode.polity_id);
                let site =
                    ObjectRef::Site(SemanticAddress::site(self.owner.world(), &site)?.encode());
                self.relate(
                    Predicate::Controls,
                    polity,
                    site,
                    episode.valid,
                    episode.cause_event_id,
                )?;
            }
        }
        Ok(())
    }
}
/// Shock-free baseline of a region, per culture. When explicit, the sum of the
/// author series defines the grid total (§26.5).
fn region_baseline(
    config: &WorldConfig,
    atlas: &HistoryAtlas,
    constraints: &AuthorConstraints,
    region: &str,
) -> Result<RegionBaseline, HistoryError> {
    let timeline = config.timeline;
    let dates = marker_dates(timeline);
    let sites: Vec<_> = atlas
        .settlements
        .iter()
        .filter(|s| atlas.region_of(s) == Some(region))
        .collect();
    let explicit = sites
        .iter()
        .filter(|s| constraints.site_baselines.contains_key(&s.site_id))
        .count();
    if explicit != 0 {
        let mut grid = BTreeMap::new();
        for site in &sites {
            let series = constraints.site_baselines.get(&site.site_id);
            if series.is_none_or(|s| !s.keys().copied().eq(dates.iter().copied())) {
                return Err(invalid("canon.baseline_series_partial_or_off_grid"));
            }
            for (year, population) in series.into_iter().flatten() {
                *grid.entry(*year).or_insert(0) += *population;
            }
        }
        return Ok(RegionBaseline {
            by_culture: BTreeMap::new(),
            grid,
        });
    }
    // Split of a site value between its cultures at a date.
    let shares = |site: &super::atlas::SettlementAnchor, total: u64, year: i32| {
        let episode = site.episode_at(year).expect("covering episodes");
        let weights: Vec<_> = episode
            .culture_weights
            .iter()
            .map(|(id, w)| (id.as_str(), u128::from(*w)))
            .collect();
        let split = largest_remainder(total, &weights);
        weights
            .iter()
            .map(|(id, _)| (*id).to_owned())
            .zip(split)
            .collect::<Vec<_>>()
    };
    let mut initial: BTreeMap<String, u64> = BTreeMap::new();
    for site in &sites {
        for (culture, n) in shares(site, site.initial_population, timeline.start_year) {
            *initial.entry(culture).or_insert(0) += n;
        }
    }
    let mut by_culture = BTreeMap::new();
    for (culture, start) in initial {
        let mut series = vec![start];
        let mut remainder = 0;
        for year in timeline.start_year..timeline.end_year - 1 {
            let capacity: u64 = sites
                .iter()
                .flat_map(|s| {
                    let cap = s.episode_at(year).expect("covering episodes").capacity;
                    shares(s, cap, year)
                })
                .filter(|(id, _)| id == &culture)
                .map(|(_, n)| n)
                .sum();
            let last = *series.last().expect("non-empty series");
            series.push(baseline_step(
                last,
                &mut remainder,
                config.processes.growth_ppm,
                capacity,
            ));
        }
        by_culture.insert(culture, series);
    }
    let grid = dates
        .iter()
        .map(|year| {
            let i = (year - timeline.start_year) as usize;
            (*year, by_culture.values().map(|s: &Vec<u64>| s[i]).sum())
        })
        .collect();
    Ok(RegionBaseline { by_culture, grid })
}
fn check_inputs(
    config: &WorldConfig,
    atlas: &HistoryAtlas,
    catalogue: &HistoryCatalogue,
    constraints: &AuthorConstraints,
) -> Result<(), HistoryError> {
    atlas.validate()?;
    let p = &config.processes;
    if config.timeline != atlas.timeline {
        return Err(invalid("canon.manifest_atlas_timeline_mismatch"));
    }
    if p.war_min_years == 0
        || p.war_min_years > p.war_max_years
        || p.epidemic_min_years == 0
        || p.epidemic_min_years > p.epidemic_max_years
        || p.expected_household_size == 0
    {
        return Err(invalid("canon.process_parameters_inconsistent"));
    }
    let tags: BTreeSet<_> = catalogue
        .site_tag_definitions
        .iter()
        .map(|t| t.id.as_str())
        .collect();
    if atlas
        .settlements
        .iter()
        .any(|s| s.site_tags.iter().any(|t| !tags.contains(t.as_str())))
    {
        return Err(invalid("canon.site_tag_not_in_catalogue"));
    }
    if constraints
        .site_baselines
        .keys()
        .any(|id| !atlas.settlements.iter().any(|s| &s.site_id == id))
    {
        return Err(invalid("canon.baseline_series_unknown_site"));
    }
    Ok(())
}
/// Collectives of a region, one per territory.
fn bootstrap_polities(
    config: &WorldConfig,
    atlas: &HistoryAtlas,
    catalogue: &HistoryCatalogue,
    region: &str,
) -> Result<BTreeMap<String, Polity>, HistoryError> {
    let timeline = config.timeline;
    let mut territories: BTreeMap<String, Vec<&super::atlas::SettlementAnchor>> = BTreeMap::new();
    for site in atlas
        .settlements
        .iter()
        .filter(|s| atlas.region_of(s) == Some(region))
    {
        territories
            .entry(territory(atlas, region, &site.cell_id))
            .or_default()
            .push(site);
    }
    let owner = Owner::cell(&config.canon_id, region);
    let mut polities = BTreeMap::new();
    for (cell, sites) in territories {
        // Seat: capital, then largest initial population, then ID.
        let seat = sites
            .iter()
            .min_by_key(|s| {
                (
                    !s.site_tags.contains("capital"),
                    std::cmp::Reverse(s.initial_population),
                    s.site_id.as_str(),
                )
            })
            .expect("non-empty territory");
        let culture = principal_culture(atlas, &seat.site_id).expect("atlas site");
        let block = decade_block(timeline.start_year);
        let address = SemanticAddress::entity(&owner, "polity", block, &cell)?;
        let entity = SocialEntity {
            id: address.encode(),
            semantic_address: address,
            kind: "polity".into(),
            archetype_id: "polity".into(),
            nature: EntityNature::OrganizedGroup,
            agency: Agency::Governed,
            owner_scope: owner.scope().clone(),
            existence: timeline,
            culture_refs: vec![culture.clone()],
            institution_ref: None,
            provenance: fact_provenance(atlas, "general/polity/v1"),
            name_tree: None,
        };
        entity.validate()?;
        polities.insert(
            cell.clone(),
            Polity {
                entity,
                cell_id: cell,
                region_id: region.into(),
                history_family_id: catalogue.binding(&culture)?.history_family_id.clone(),
                seat_site_id: seat.site_id.clone(),
            },
        );
    }
    Ok(polities)
}
/// Sorted pairs of jurisdictions whose cells are neighbours.
fn adjacent_pairs(
    atlas: &HistoryAtlas,
    polities: &BTreeMap<String, Polity>,
) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for cell in atlas
        .cells
        .iter()
        .filter(|c| polities.contains_key(&c.cell_id))
    {
        for neighbor in cell.neighbors.iter().filter(|n| polities.contains_key(*n)) {
            if cell.cell_id < *neighbor {
                pairs.push((cell.cell_id.clone(), neighbor.clone()));
            }
        }
    }
    pairs.sort();
    pairs
}
/// Port rates per block and diplomatic contacts (§7.1, §12.2): drawn once per
/// contact, never per side.
fn run_contacts(
    config: &WorldConfig,
    atlas: &HistoryAtlas,
    ledger: &mut EventLedger,
) -> Result<BTreeMap<String, Vec<FlowEpisode>>, HistoryError> {
    let seed = config.world_seed;
    let p = &config.processes;
    let mut flows = BTreeMap::new();
    for contact in &atlas.external_contacts {
        let owner = Owner::new(&config.canon_id, contact.owner_scope.clone());
        let mut episodes = Vec::new();
        for block in blocks(contact.valid) {
            let text = block.to_string();
            let id = contact.contact_id.as_str();
            let keys = |leaf| ["general/contact/v1", id, &text, leaf];
            let rate = |leaf| -> Result<u16, HistoryError> {
                let spread = u64::from(p.port_rate_spread_bps) + 1;
                Ok(p.port_base_rate_bps + uniform_below(seed, &keys(leaf), 0, spread)? as u16)
            };
            episodes.push(FlowEpisode {
                valid: TimeSpan {
                    start_year: block.max(contact.valid.start_year),
                    end_year: (block + 10).min(contact.valid.end_year),
                },
                forward_rate_bps: rate("forward")?,
                backward_rate_bps: rate("backward")?,
            });
            if gate(seed, &keys("diplomacy"), p.diplomatic_contact_bps)? {
                if let Some(year) = date_in_block(seed, &keys("date"), block, contact.valid)? {
                    let mut participants = Vec::new();
                    for end in &contact.ends {
                        if let ContactEnd::Site(side) = end {
                            let site = SemanticAddress::site(&config.canon_id, &side.site_id)?;
                            participants.push(ObjectRef::Site(site.encode()));
                        }
                    }
                    ledger.add(NewEvent {
                        owner: &owner,
                        kind: "diplomatic_contact",
                        year,
                        end_year: None,
                        phase: EventPhase::ContextAndShocks,
                        causes: vec![],
                        participants,
                        provenance: fact_provenance(atlas, "general/diplomatic-contact/v1"),
                    })?;
                }
            }
        }
        flows.insert(contact.contact_id.clone(), episodes);
    }
    Ok(flows)
}
/// General canon, a pure function of its inputs: no inhabitant is ever walked.
pub fn build_canon(
    config: &WorldConfig,
    atlas: &HistoryAtlas,
    catalogue: &HistoryCatalogue,
    constraints: &AuthorConstraints,
) -> Result<GeneralCanon, HistoryError> {
    check_inputs(config, atlas, catalogue, constraints)?;
    let societies = site_societies(config, atlas, catalogue)?;
    let domains = partition_domains(atlas, &config.canon_id, &societies)?;
    let mut polities = Vec::new();
    let mut baselines = BTreeMap::new();
    let mut site_control = BTreeMap::new();
    let mut events = Vec::new();
    let mut contracts = Vec::new();
    let mut relations = Vec::new();
    let mut regions = atlas.region_ids.clone();
    regions.sort();
    for region in &regions {
        let region_polities = bootstrap_polities(config, atlas, catalogue, region)?;
        let pairs = adjacent_pairs(atlas, &region_polities);
        let control = atlas
            .settlements
            .iter()
            .filter(|s| atlas.region_of(s) == Some(region.as_str()))
            .map(|s| {
                let cell = territory(atlas, region, &s.cell_id);
                let episode = ControlEpisode {
                    polity_id: region_polities[&cell].entity.id.clone(),
                    valid: config.timeline,
                    cause_event_id: None,
                };
                (s.site_id.clone(), vec![episode])
            })
            .collect();
        let mut run = RegionRun {
            config,
            atlas,
            region,
            owner: Owner::cell(&config.canon_id, region),
            polities: region_polities,
            control,
            ledger: EventLedger::new(),
            contracts: Vec::new(),
            relations: Vec::new(),
        };
        run.run(&pairs)?;
        polities.extend(run.polities.into_values());
        site_control.extend(run.control);
        events.extend(run.ledger.events);
        contracts.extend(run.contracts);
        relations.extend(run.relations);
        baselines.insert(
            region.clone(),
            region_baseline(config, atlas, constraints, region)?,
        );
    }
    let mut ledger = EventLedger::new();
    let contact_flows = run_contacts(config, atlas, &mut ledger)?;
    events.extend(ledger.events);
    events.sort_by(|a, b| a.id.cmp(&b.id));
    contracts.sort_by(|a, b| a.id.cmp(&b.id));
    relations.sort_by(|a, b| a.id.cmp(&b.id));
    polities.sort_by(|a, b| a.entity.id.cmp(&b.entity.id));
    causal_ranks(&events)?;
    validate_composition(&relations)?;
    Ok(GeneralCanon {
        canon_id: config.canon_id.clone(),
        world_seed: config.world_seed,
        timeline: config.timeline,
        domains,
        polities,
        baselines,
        site_control,
        contact_flows,
        events,
        contracts,
        relations,
        config: config.clone(),
        atlas: atlas.clone(),
        catalogue: catalogue.clone(),
        constraints: constraints.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_remainder_moves_small_populations_in_both_directions() {
        let run = |growth, start| {
            let (mut p, mut remainder) = (start, 0);
            for _ in 0..100 {
                p = baseline_step(p, &mut remainder, growth, 1_000);
            }
            p
        };
        // A village of 50 at 3,000 ppm gains less than one person a year.
        assert!(run(3_000, 50) > 50);
        assert!(run(-3_000, 50) < 50);
        assert_eq!(run(-3_000_000, 50), 0);
    }
}
