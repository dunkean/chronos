# Social v2 method: S0 core, S2 batch, S3 host and S4 social layer

English is the source of this document. The complete French translation follows
the English text, under "Méthode sociale v2".

## Implemented state

L5 naming is implemented separately in [Organization naming v2](naming-v2.md),
including its complete French translation and frozen Namebench provenance.

The public module `chronos_core::v2` delivers the data contracts of batch S0.
It is independent of `chronos-history-1`: the only addition to v1 code is
`pub mod v2;`. `chronos-social-2` names the v2 family without announcing a
complete social generator. No provider, Python process, disk access, clock or
new dependency is added to the core.

### Addresses and indexed decisions

`ids::SemanticAddress` carries the type and the fields of the address. `encode`
writes `chr2\0`, then, for each field, its little-endian u32 length and its
UTF-8, in unpadded base64url, behind the `type.` prefix (`event.Y2hy…`). The dot
is absent from the base64url alphabet. `decode` also checks the canonical
encoding, the lengths, the type, the normalized ASCII and the decimal integers.
An ID longer than 4,096 characters is rejected.

**Each object has a single address form.** `from_fields`, the only entry point
including for `decode` and serde, fixes the number and role of the fields per
type:

| Type | Fields |
|---|---|
| `domain`, `plan` | world, region, social profile, ordinal |
| `person` | flat domain, provider algorithm, local ID |
| `household` | flat domain, `household`, origin, creation year |
| `site`, `activity` | world, catalogue ID |
| `entity` | owner, type, `origin_block`, slot |
| `asset` | owner, `asset`, type, `origin_block`, slot |
| `event` | owner, type, `origin_block`, ordinal |
| `impact` | fields of the parent event, contract slot |
| `effect` | fields of the contract, target type, target fields without the world |
| `group` | owner, `group_slot`, archetype, `origin_block`, ordinal |
| `transfer` | owner, `transfer`, origin, destination, year, ordinal |
| `tenure` | owner, `office`, `origin_block`, slot, ordinal |
| `relation` | owner, `relation`, predicate, endpoint types, three counted segments, year, ordinal |
| `standing` | owner, `standing`, endpoint types, three counted segments, year, ordinal |

`DomainKey` is the structured domain. `OwnerScope` is the scope of an owner, in
four variants: `world`, `cell`, `site`, `domain`. `Owner` adds the world to it
and enters the address flat, preceded by its discriminant, with no string
joining. A cell whose name imitated a domain therefore remains a different
address. Facts carry `owner_scope: OwnerScope`, without the world: the atlas
also carries this type, and it is an input of the canon, hence earlier than any
`canon_id`. `SemanticAddress::owner` reads back the owner of an address. A
tenure takes the address of its office, which must be an entity of type
`office`.

Literal fields (`household`, `transfer`, `relation`, `standing`, `asset`,
`group_slot`, `office`) are checked at construction as at decoding. `relation`
rejects a predicate outside the closed enumeration and a non-maximal common
prefix. `origin_block` is the start year of the block: it must be aligned, and
`ids::decade_block` computes it with `div_euclid`, including before year zero
(`-1` gives `-10`). Years and blocks are bounded to ±100,000.

A field never contains an encoded ID. A substring is rejected only if it
decodes to a real address: `chr2\0` payload, complete fields and the structure
of a known type, at any position, with or without a type prefix, followed or not
by other characters. The mere presence of the base64url signature `Y2hyMg` is
not enough. An address whose structure is valid but which itself contains an ID
is still recognized, which catches double nesting. Migration, a change of name
and a change of weight do not take part in a new address.

An `ObjectRef` has a single canonical form, which `ObjectRef::address` checks: a
person is a `PersonSlot`, an entity of type `office` an `Office`, any other
entity, group or household an `Entity`. The relation constructor accepts any
addressable `ObjectRef`, sites, assets and activities included, and `link`
reads back owner, predicate, endpoints and start.

`impact` addresses a contract, child of its event. `effect` addresses the
exposure of a target to that contract, child of the contract and unique per
target; it has no annual dimension. The episodes of the timing profile (§7.2)
are its sub-slots, of type `episode`, added in S2.

`ids::random_semantics_v2` is the facade of the draws. `domain_seed` calls the
v1 mixer exactly, with the three parts of §4.1. The other draws prefix
`random_semantics_v2`, the number of parts, their base64url encodings, the
sub-counter and the rejection attempt, all explicit. The `keys.rs` mixer stays
untouched. `uniform_below` is uniform on `[0, upper)` by rejecting the prefix of
size `2^64 mod upper`, with no shared state. The tests pin these derivations
with literal values obtained from a Python port independent of `keys.rs`:
changing a key makes a test fail.

### Canonical bytes

`canonical::canonical_bytes(&serde_json::Value)` produces compact UTF-8 JSON,
without BOM, with keys sorted by UTF-8 bytes, decimal integers and minimal JSON
escapes. Every floating-point number is rejected, `1.0` included. The API
receives an already built JSON value: it does not convert Rust floating-point
objects, which avoids implicitly turning NaN into `null`.

The function preserves the order of lists. The caller must supply the contract
projection: lists of facts sorted by ID, episodes and registers in their
explicit order, descriptive fields and durations excluded. It guesses neither
the identity columns nor the fields to exclude in an arbitrary JSON. S0
computes and authenticates no SHA-256: `plan_canonical_bytes` provides the
bytes, the host computes the fingerprint. The `example` fingerprints of the
fixture remain shape-only demonstration values.

§4.2 holds for the whole core. `plan_canonical_bytes`,
`HistoryAtlas::canonical_bytes` and `canonical_list_bytes` apply the canonical
order without requiring it to be the one received. `canonical_list_bytes`
handles any list of `CanonicalRecord`: entities, states, relations, events,
people and legal standings. It sorts by ID, or by entity then date for states,
and sorts the internal lists of IDs. Episodes and registers keep their order,
which is a fact.

The vectors [canonical-json.v1.json](../design/chronos-social-v2/vectors/canonical-json.v1.json)
contain input, expected UTF-8 text and hexadecimal bytes. They cover Unicode,
control characters, key order, preserved list order and the two integer bounds.
`integer_bounds` documents these bounds: from `-9223372036854775808` to
`18446744073709551615`, inclusive. `rejections` lists the texts to reject:
floats, exponents, `-0`, out-of-bounds integers, `NaN`, `Infinity`, lone
surrogate and BOM. Rust rejects them at text decoding or at canonicalization.
Python's `json` module accepts several of these texts by default: the future
worker will have to reject them explicitly. These vectors do not yet validate a
worker. A duplicated object key in a JSON text is not detected: decoding keeps
the last value before `canonical_bytes` receives it.

### Atlas, social model and time

`atlas::HistoryAtlas::validate` checks references and uniqueness, acyclic
hierarchy, bounds and frames, reciprocal neighbours, complete site catalogues
per subtree and sums of maximum expected populations. Future foundations and
imposed abandonment are rejected. Occupations cover the horizon exactly, with
no gap or overlap. Routes carry whole days, a validity and a direction; an
empty list stays an empty list. Atlas IDs are printable ASCII without spaces,
and coordinates are bounded to ±9e15 mm. Every error carries the atlas as owner
and its opening date.

`ExternalContact` has a single owner, the world or a cell enclosing its sites,
and two sorted endpoints. An endpoint is an attachment site, with the context,
profile and population it sees beyond and its possible access route, or a
purely outside context. At least one site is required. Facing an outside
context, the site declares that same context. Two sites may share a contact:
each then opens its port under the same `contact_id`.

As the atlas JSON is not frozen by a schema in the study, S0 uses integer
`area_m2` surfaces and metric `*_mm` bounds, with no geographic conversion nor
interpretation of display LODs. A cell's summaries include its descendants. The
demographic partition belongs to batch S2.

`model` separates provenance, precision and coverage of counts, entities,
states, relations and people. The predicates of §5.1 are a closed enumeration.
Event phases are serialized with their numbers from §7.3. `TimeSpan` describes
`[start_year, end_year)`; only lives may have an empty interval.
`HistoryEvent.end_year` gives the exclusive end of a frame event and is `null`
for a dated event.

`CountMeasure.value`, `PersonRecord.death_year`, `HistoryEvent.end_year` and the
causes of `LegalStanding` and `AlignmentEpisode` require explicit `null`: an
absent field is rejected, because `null` means "unknown" or "none".

`PersonRecord.observed_span` gives the years observed by the provider; the name
avoids confusion with the `coverage` axis of §4.3. `is_alive_and_covered`
excludes the year of death and any date outside that interval, even if the death
is unknown. `parent_refs` targets a row (`slot`) or a typed outside reference
(`coverage_limit`, `external_arrival`). `PersonRecord::validate` bounds the
dates, rejects a death before birth, validates `observed_span`, and limits
parents to two, since only two-parent reproduction is delivered.

**Record identity.** `validate` proves that a fact is the one its ID addresses.
Error: `InvalidRecordIdentity`.

| Fact | Check |
|---|---|
| `SocialEntity` | `id` equals `semantic_address`; owner equals `owner_scope`; entity: `kind` in the address; group slot: `archetype_id` in the address; household: `kind = household`; person: `kind = person` and `nature = person`, the only nature admitted for this address |
| `HistoryEvent` | event address of this type and this owner |
| `SocialRelation` | relation address with the same predicate, endpoints and start |
| `LegalStanding` | standing address with the same subject, jurisdiction and start |
| `PersonRecord` | person address whose local ID is `provider_ref`; table parents addressed |
| `ImpactContract` | impact address, child of `parent_event_id` and carried by `owner_scope` |

`causal_ranks`, `validate_composition`, `validate_legal_standings` and
`resolve_impact` call these checks. A group slot does not constrain `kind`,
which its address does not carry.

`SocialEntity.name_tree` stores the tree produced by the frozen Namebench grammar.
The canonical tree is rendered to text only at display time. S0 types it as `NameTree`, an opaque JSON value that passes the
canonical check at construction as at reading: a float is rejected there. The
field is absent as long as no tree exists, `null` is not accepted in its place,
and S0 contains no generation logic.

`validate::causal_ranks` rejects missing references, duplicates, future causes,
invalid frame-event dates and cycles. Every causal cycle receives
`CanonicalDependencyCycle`, including one that crosses phases or years. Ranks
are built by topological sort within each phase; the ID only breaks ties among
nodes that are already available. A cause in the same year but an earlier phase
is accepted. `validate_composition` rejects strict `component_of` cycles over
the set of episodes, while allowing overlapping affiliations.
`CompositionCycle` designates a real cycle; a duplicated relation ID receives
`InvalidCanonicalValue`. These checks do not yet amount to a validation of
provider genealogy.

### Alignment and legality

`AlignmentAxes` carries `morality_bps` and `lawfulness_bps`, bounded to ±10,000
at construction as at reading. `conduct_class` is a pure function of the axes
and of `ConductThresholds`: the core fixes no threshold, their values come from
`engine_defaults.alignment` in S1. The class is never stored.
`entity_conduct_class` returns `None` for a category or a population.

`EntityState` carries `alignment`, `conduct_tags` and `visibility`. A conduct
tag is an ID, not a Rust enumeration: the catalogue evolves as data (§5.2), and
`EntityState::validate` and `PersonRecord::validate` receive this catalogue as a
parameter. `PersonRecord.alignment_episodes` is an ordered list: every episode
after the first cites the event that changes the axes.

`LegalStanding` is relative to a jurisdiction and dated. Its subject is an
entity, an activity or a person slot; its jurisdiction is an entity.
`legal_status_at` returns `licit` when no record applies, and another
jurisdiction has no effect. `validate_legal_standings` receives `observed_start`
and the end of the world, groups records by subject/jurisdiction pair and
rejects overlaps and exits from the period. Only a segment starting at
`observed_start` may have no cause; any other start cites one, including a late
first segment. A segment that ends before the end of the world without an
immediate successor, or before a gap, cites its end event in
`end_cause_event_id`. All these causes are of type `edict`, `charter`,
`heresy_trial` or `banishment`, and no later than the date they explain. Since
S2, the first record of a chain may cite the `foundation` of its subject, and
the last one its `dissolution`.

### Impacts

`region::resolve_impact` consumes a `CompleteCandidateCatalogue`. This type has
no public field and no `Deserialize` and is built only from an authoritative
core source: `from_atlas(&atlas, &owner)`, which itself enumerates the sites of
the validated atlas, for the world, a cell or a site. No list is received from
the caller. A domain is rejected with `UnsupportedCapability`: its partition
belongs to S2, which will add other sources in the same way. A projection never
produces a catalogue and never recomputes a quota.

The contract, its parent event, the catalogue and the effects share canon and
owner: a contract from another world or another scope than the catalogue is
rejected with `InvalidImpactContract`.

In probabilistic mode, each eligible target draws a single exposure in bps,
with no annual dimension. The random intensity word has its own key. It is
delivered as `intensity_draw`; the interpretation of the severity profile and
the local episodes belong to S2. Each `TargetImpact` carries the ID of the
parent event and an `effect_id` specific to its target. These identities are
derived from the contract and the target: S0 invents no fact at query time. The
draw keys carry the contract ID, hence its address.

In exact mode, **K is computed over the eligible candidates** at
`eligibility_time` and retained by the selector, never over the raw catalogue: a
declared count must fit within them, a fraction applies to their number.
Rounding is mandatory: only `floor` and `ceil` are supported, with no implicit
rounding. Ranking uses priority then ID, over the complete set. The budget of
100,000 candidates, however, is measured on the whole catalogue, before
filtering: 100,000 is accepted, 100,001 rejected. An impossible K fails. Exact
modes, including fixed target, over people and cohorts are rejected with
`UnsupportedCapability` until S6. `ExactImpactInfeasible` is used only for
non-probabilistic modes that cannot reach their target; a malformed contract
receives `InvalidImpactContract`.

### Population plans

`population::PopulationPlan` follows schema 1, with closed types,
`deny_unknown_fields`, required nullable fields and optional effects that may be
omitted but not null. Future variants stay readable. `validate_plan(plan,
references)` checks the schema embedded at compile time and read once, the
bounds, units, dates, references, positive weights, contiguous social ranks,
domain seed, episode coverages and prototype budgets. `PlanReferences` supplies
the canon, the world horizon, `world_seed`, events, offices, contacts, cultural
bindings, activities, pack fingerprints and expected atlas maxima over the whole
horizon. These references are prepared by the authority of the canon and the
catalogues, never by the query window.

The plan is bound to its canon: `canon_id`, `world_seed` and `timeline` equal
those of the references. `domain_id` is a domain address of this world and this
social profile, `plan_id` the plan address of the same domain. `domain_sites`
carries the expected partition: the sites of the plan equal exactly those of its
domain, and the same sites under another ordinal are rejected. The example plan
carries real addresses, and its domain seed follows from them.

`culture_variants` resolves each `mu_culture_id` to its family and social
profile. The main variant, all variants of non-zero weight in all episodes and
all sites of the domain must resolve to the plan's profile; otherwise
`UnsupportedCapability`, until S8. A zero-weight variant may have another
profile but must exist in the binding. A missing reference or a wrong family
remains a plan error, not a capability error.

A kinship profile must take up the `dependent_age` of the social profile, must
not allow a parent who is still dependent, and must use a household model
admitted for the residence mode. A single model is delivered today, compatible
with all three modes: this last check therefore only has an effect when a
second model arrives. A port takes up the context, profile and population of the
contact endpoint anchored at its site.

`plan_canonical_bytes` excludes `plan_id` and `plan_fingerprint`. Sorted by ID
are: capabilities, profiles, sites, role demands, fixed people, constraints,
ports, routes and their lists of IDs; markers are sorted by site then year,
shocks by `shock_id`. Occupation episodes, rate episodes and
protected periods keep their order.

`historical_program` is an explicitly ordered list by `(year, phase, op_id)`,
the application and normalization key that `application_key` returns. The phase
follows from the operation type: 10 for `protect_person`, at its start date; 30
for `force_death`; 40 for `move_households` and `admit_people`.

The validator accepts ordinary S0–S5 data as contracts to prepare; it executes
no S3 capability. It rejects unknown capabilities, S6–S8 fields, fixed people,
exact demographic constraints, personal operations, required roles, required
core, named bridges, custom reproduction and a fully procedural provider. The
free policy requires the four attempts declared in §25.4, without executing
them.

The recognized parameters are those of the normative example, all explicit.
`_bps`, `_ppm`, `_mm` and ages are checked without floats. `residence` accepts
`patrilocal`, `matrilocal` and `either`, as Lineage Lab's `config.py` does; any
other value is a plan error. The effect conversions of §26.4 remain to be done
in the S3 adapter, separately from the profile parameter names.

## S2: general canon, regions and domains

English is the source for this section; the French translation is in the second half of this document.

### Implemented

**Inputs.** `general::build_canon(config, atlas, catalogue, constraints)` reads the
S1 catalogue through `catalog::HistoryCatalogue`. `WorldConfig` carries `canon_id`
and `society_ids` (template to `demography_society_id`): the host computes both, so
the core has no SHA-256. `SettlementAnchor.site_tags` and `HistoryAtlas.provenance`
are new required fields. Facts drawn from a synthetic atlas stay `synthetic`.
`fixtures/social-v2/region-demo.atlas.json` and `region-demo.world.json` are virtual:
one region, four cells, 20 sites, four cultures, two social profiles, 24 routes and
four contacts over 1300-1400. Seed 11 was picked from a sweep of seeds 1-60 at the
default rates, because it shows every event kind; it is not a calibration.

**Domains.** `atlas::partition_domains` is the next-fit of §3.1. A site above 20,000
is refused with `BudgetExceeded`. A site mixing social profiles is refused with
`UnsupportedCapability`.

**General processes.** Each region runs its own annual loop, with decade-block draws
under fixed subkeys; a partial block keeps its full key and drops dates outside the
period. Polities are derived: one per child cell of the region holding sites, and
adjacent cells give the war pairs. War and epidemic are frame events with one
`ImpactContract` each. At peace a `treaty` draws each `frontier` site of both parties
and may move its control. The baseline follows §7.1 per region and culture, with a
signed remainder, and never reads an event.

**Region.** `region::plan_region(canon, region_id)` resolves each contract on the
complete catalogue and compiles exposures into `LocalShock` values with the §7.5
profiles. The local date is drawn under its own key; a military arrival cannot
precede `start + ceil(travel_days / 365)`. Each shock has an address of the new kind
`episode`, a sub-slot of its effect. Site markers follow §26.5, or explicit author
series when `AuthorConstraints.site_baselines` covers the region.
`population_expected_after_impacts` is a separate forecast and feeds no marker.
Each domain receives its ports, sorted by travel time then ID and capped by
`max_halo_contacts`; the rest stays a summary. The two sides of a contact share one
drawn flow, one side's exit being the other's entry.

**Birth events.**

| Event | Definition | Group born |
|---|---|---|
| `diplomatic_contact` | One draw per contact and decade, owned by the contact's owner, phase 10, with its attached sites as participants | `embassy` on an attached site, if `availability` allows |
| `displacement` | One draw per ruined site, owned by that site, phase 40, caused by the war frame; it feeds a `move_households` to the nearest site of the domain, else to a port | `refugee_community` on a destination site, if `availability` allows |
| `heresy_trial` | Legal act of a jurisdiction (§27.3) | `inquisition` on the group's site, if `availability` allows |

**Legality.** Initial status follows §27.4 under the controlling polity. S2 creates
own-stock groups and event-born groups, with bootstrap ages and per-block flows.
Per polity, kind and decade, a jurisdiction may issue `edict` (tolerated to
proscribed), `charter` (tolerated to licit), `heresy_trial` or `banishment`. A control
change emits one `charter`, caused by the treaty, which closes the old chains and opens
the new ones. `validate_legal_standings` also admits two life events: `foundation`
for the first standing of a chain and `dissolution` for the last one, each at its
exact date and with the subject as participant.

**Plan.** `population::plan_population(canon, region_plan, domain_id)` returns a plan
that passes `validate_plan`. It holds one shock per local episode: `shock_id` is the
`episode` address, distinct from `source_event_id`, which several shocks share.
`validate_plan` refuses a second exposure of a site to one event, whatever its dates.
`plan_fingerprint` holds `PLAN_FINGERPRINT_PENDING` until
the host hashes `plan_canonical_bytes`.

**Diagnostics.** Every `HistoryError` puts a stable English code in `constraint`
(`domain.site_over_population_cap`, `plan.unknown_domain`...) and never a
sentence. `owner`, `at_year` and the optional `params` map (for example `path`
for `plan.schema_shape`) are its parameters. Each code has a message under the
`diagnostics.` prefix in `web/i18n/en.json` and `web/i18n/fr.json`, and
`tools/check_i18n.mjs` fails if a code raised by `src/v2` has no message or a
message is never raised. The error kinds of §13 are unchanged.

### Limits

Not delivered: anchored institutions, offices and `role_demands` (S4), activity units
and shared stocks (S4), interregional contagion, wealth and cohesion states, a process
for `harvest.bad`, and candidate catalogues for a domain owner. A world-owned contact
numbers its events across regions, so adding one shifts those ordinals.

### Validation

`tests/v2_s2.rs` proves byte-identical replay, `world_extent_does_not_reseed_domains`,
`generated_baseline_nodes_are_independent_of_shocks`,
`spared_village_keeps_baseline_target` at plan level and `plan_region` call order. No
performance is measured, and no provider has run these plans.

## S3: worker host, validation and cache

### Implemented

`chronos-app` runs the Lineage Lab worker as a child process, JSONL on stdin/stdout, from
an argument list. Each job uses a fresh process and a private job root. `prepare` sends the
plan in canonical list order, then `materialize` returns `result_ref`. The host checks, in
order: the path stays in the job directory, size, SHA-256, canonical bytes, then
`result_fingerprint`. Any failure is a typed error, leaves no package and selects no other
provider.

`validate_population(plan, result)` is pure and reads tables only: references, coverage,
chronology on `(year, phase)`, one unbroken residence chain per slot, one household per
resident, reciprocal kinship, sparse counts, and balances per site-year and per domain.
`validation_proof` from the worker is not trusted. `VerifiedPopulation` has private fields
and no deserializer, so a warm read validates again.

`canon_id` is now host-derived: `canon-` plus the first 32 hex digits of the SHA-256 of the
normalized manifest (resolved configuration, constraints, atlas and catalogue byte
fingerprints, runtime manifest and `runtime_fingerprint`). The `canon_id` of a world file
is ignored. A file with `synthetic-runtime-not-pinned` is pinned to the runtime found;
another pinned value fails with `ProviderVersionMismatch`. A changed provider source
therefore gives another canon, with other domain and plan IDs.

Packages are immutable under `worlds/<canon>/`, published by one rename. File names are the
SHA-256 of the ID, as §16.1 allows; the real IDs are compared on every read. The population
cache key covers canon, owner, plan and region fingerprints, versions, protocol, format and
detail level. A damaged package is an error and is never recomputed silently. One domain is
resolved once; concurrent requests share its job, and a failed job can be replayed.

### Limits

`fixed_people`, `exact_constraints` and role demands are not checked: S3 plans leave them
empty. Pack fingerprints cover file bytes, so line endings change the canon. The number of
concurrent workers is not capped. There is no archive-only mode: opening a world needs the
runtime. Directory entries are not flushed after a rename.

### Validation and measures

`crates/chronos-core/tests/v2_s3.rs` holds `crisis_is_applied_once` and the tampered balance
test. `crates/chronos-app/tests/v2_s3.rs` runs the real worker when `CHRONOS_PYTHON` and
`CHRONOS_LINEAGE_SRC` are set, and otherwise skips with a message.

Measured on 2026-11-06, release build, i9-12900K, Windows 11, demo fixture with world seed
11, full century, on a frozen copy matching provider commit `67f7842` (runtime `6ef2eee98aad`),
one sample per case, on a machine shared with other work:

| Domain | Slots | Archive | First run, empty JIT cache | Worker run, JIT cached | Warm read | Host peak |
|---|---:|---:|---:|---:|---:|---:|
| 0 | 19,266 | 146.8 MiB | 28.5 s | not measured | 2.8 s | 899 MiB |
| 1 | 24,137 | 187.5 MiB | 34.1 s | 28.5 s | 3.3 s | 1,066 MiB |

The worker's own memory was not measured here.

## S4: social layer

### Implemented

`social::build_social_packet(region_plan, verified_population)` is pure and builds one packet
per domain over the whole horizon. `RegionPlan.social` carries the frozen canon inputs it reads
(catalogue, atlas, polities, control): the field is outside the canonical projection, so the
two arguments of §14.1 hold and region fingerprints do not move. `build_social_prefix` stops
at a year and gives the same facts as the full packet read before it (§9.5).
`offices::resolve_office(packet, office_id, at_year)` only reads: the year asked draws nothing.

**Attributes** (`attributes.rs`). Status, activity, workforce draw, culture, family latent,
alignment axes and initial skills, under `social/attributes/v1` and `alignment/v1`, keyed by
domain seed, family key and person key. The §9.4 selector picks the family uniform or the
individual one, never their mean. Status uses the §8.4 residues. Core attributes are kept.
`ancestry_id` is read by no draw.

**Group slots** (`social.rs`). S2 groups are adopted as they are. S4 adds, under
`social/group/v1` and per site: one anchor per occupation (`site_occupation_anchor`), one
secondary anchor per threshold, activity units sharing the expected workers, and shared
association, community and friend-circle stocks, with bootstrap ages and block flows as in S2.
Friend circles use the catalogue's duration and size tables. Households and lineages stay views:
`household_of`, `family_of`, `site_views`, and `person_record` for the §5 `PersonRecord`.

**Year.** Phase 50 closes commitments (death, departure, dissolution, capacity, barred seats) and
every tenure that ended. Phase 55 runs cohesion and splits. Phase 60 assigns offices, those a split
just created included, then jobs, other time-budgeted groups and overlapping affiliations, and last
circles. Each draw is keyed by `(request_id, year, vacancy_ordinal, draw_ordinal)`. Weights
are those of §23.2 with the multiplicative `alignment_fit`. Network uses the published state Y−1.
Circles create `trusts` links, one per member, never a clique.

**Relations.** `Membership.opened_year` keeps the start of the full fact, so a clipped membership
gives the same relation ID in every window; only `valid` follows the window. `cause_event_id` says
what opened a relation (a split, for a transferred member) and `end_cause_event_id` what closed it.

**Offices.** A site anchor has a head (`mayor`) and a collective body; a secondary anchor and an
association have a head. A term ends, a holder dies, leaves or loses eligibility, and the mandate
closes at that boundary; the replacement or the vacancy opens at the same date. The first
observed holder carries `initial_observed_binding` and no event. A full-time job is preempted
and traced by `employment_preemption`.

**Legality.** S4 groups get standing chains like S2 groups, and cite `foundation`, `dissolution`
and the control `charter`. `in_conflict_with` is derived from open proscribed standings. A
proscribed person or a known member of an open proscribed group is barred from offices of that
jurisdiction, and from its bodies. Conversely, an open proscribed group never recruits a holder of
an office or seat of the jurisdiction that proscribes it. The private head of a group is not an
office of the jurisdiction.

**Names (L5).** `name_tree` is expanded once from the frozen Namebench pack, exact
`name_profile`, source culture and founding tone. `GroupRecord.name` remains a legacy
descriptor; `name_fallback` is explicitly `synthetic` only without a pack or an
expandable kind. Household/core-lineage name trees are in `person_group_names`.
See [the implemented naming method](naming-v2.md).

**Display names.** `display_names` labels people, sites, regions and polities from
`fixtures/naming/person-pools.v1.json`, under `display-names/v1`, a stream no fact reads. A forename
follows sex and person key; a surname follows the family key of the oldest known ancestor by the
father, else the mother, and that person's culture. A polity carries its seat's name. `render_tree`
gives each organization's name in `en` and `fr` from its `name_tree`, and these labels fill its
`ref` parts. Every label shows its provenance; the pools hold present-day data. Their fingerprint
enters the world manifest.

**Association heads.** The head of an association is a member. The first holder is drawn from the
residents and becomes its first member; afterwards only current members qualify, so an association
without an eligible member stays vacant and takes the -400 bps cohesion penalty. The head of a
time-budgeted association adds no cost to that membership, and a split leaves the head in place.

**Influence.** `influence::influence_at` reads a packet: strength from realized members,
attenuation by travel days, persistence after the group's end, capped per channel.

### Choices not fixed by the study

- Rule of the head: `collective_body` templates elect their head (`weighted_election`); an
  `appointment` or `seniority` template gives its own rule. `lordship` is hereditary, read from
  its archetype ID because the catalogue declares no succession field. Secondary anchors appoint.
- `hereditary` draws among eligible kin of the last holder, then among all candidates.
- A person holds one personal charge at a time. Associations and communities recruit within
  the `recruitment` travel budget, others on their site; reach is read at the world's start.
- `status_fit` of a group is the closeness to a stable focus rank drawn per slot; for an office it
  is the rank itself. Friend circles carry no standing and no life events.

### Limits

An edict only reaches the archetypes S2 holds; S4 groups change status at control changes only.
`stock_multiplier` of overlays is not applied. Person standings are read, never generated. A
packet is recomputed, not stored: `Orchestrator::social_packet` keeps it in memory. Tenures have
no `claimed` or `regent` case yet. Status and activity are fixed for life. Weights are untuned.

### Validation and measures

`src/v2/social_tests.rs` holds the pure proofs on a hand-built table, and
`crates/chronos-app/tests/v2_s4.rs` runs both demo domains on the real worker, skipping with a
message without `CHRONOS_PYTHON` and `CHRONOS_LINEAGE_SRC`. `chronos-app v2 social` prints groups
by class and conduct class, vacancies and mayors.

Measured on 2026-11-06 on the demo fixture, world seed 11, fertility peak 4000, one run: no vacancy,
because the smallest pool any selection saw has 35 candidates, and one split, a friend circle in
1392. With two sites reduced to 8 initial inhabitants, the real worker gives 21 vacant office-years.
A vacancy needs a site without an eligible adult; these figures say nothing about other seeds.

## S5: API v2, web views on real data, export and sample

Implemented on 2026-11-06 (lot `social-v2/s5-final`). Proposed in the study and implemented here: §14.2,
§14.3, §15.2, §15.4, §16.2, §18.4, §27.6. Measured: the walk of `docs/reviews/social-v2-acceptance.md`.

### Implemented

- **`chronos_core::v2::query`** (pure, WASM-compatible). `project_view(canon, materialized, query)` returns a
  `HistoryView`: entities, offices, relations, legal standings, events, tenures, expected and realised measures,
  balances, the window of facts applied (`period`) and the whole `timeline`, `coverage` with the number of
  domains materialized, `provenance`, and the limits. A domain whose packet is missing is an error
  (`query.packet_not_materialized`), never a guess. IDs are `entity.id` and person addresses; the
  packet-local group index and the person slot never leave the module. Entities carry `precision`, `coverage`,
  `provenance`, and a declared fallback name: organizations `org_name.fallback` with `name_kind.<kind>`, people
  `person.fallback` (tag, birth year, activity), all flagged `name_synthetic`. Selection proofs are counts and
  weights, not candidate lists (§15.4). `entity_sheet` and `explain_fact` serve the dated sheet and the
  explanation of an event, tenure, standing, relation, membership or kin link.
- **Selection of what a view holds.** Without `scope.ids`, the seeds are the offices and organizations
  (institutions, associations, communities) of the site, then the holders of its mayoralty over the window
  and of the other charges at the date asked. Breadth-first expansion (depth 0, 1 or 2 by `detail`) in a fixed
  order adds kin, partners, children, affiliations, offices and jurisdictions. With `scope.ids` the entities
  asked are the seeds and a group also opens up to 16 members. A truncated list is never complete.
- **Pages.** One merged list of nodes and relations (a relation follows its last endpoint). A page stops when
  `limits.people` or `limits.relations` is reached; `cursor` is `c1.<position>.<hash of the query>`, so it is
  stable and refused for another query. `limits.complete` is true only when no page follows and no event was cut.
  `max_nodes` 300 and `max_edges` 1000 are the drawing caps of the web graph.
- **Overlay** (`HistoricalOverlay`, §15.2): atlas frame, sites with their atlas positions, routes valid in the
  year, channels `recruitment`, `economy`, `culture`, `religion`, `military`, `memory` (from `influence_at`) and
  `legality` (share of groups tolerated or proscribed by site), external contacts, and for a selected group the
  sites where its members live, its seat and the other chapters of its archetype. A contact keeps its attached
  site and names its outside context; no position is given to the outside.
- **`chronos-app/src/v2/server.rs`**: `POST /api/v2/worlds`, `POST /worlds/{canon}/query`, `GET /jobs/{id}`,
  `GET /worlds/{canon}/entities/{id}?at_year=`, `POST /worlds/{canon}/graph`, `GET /worlds/{canon}/explain/{fact_id}`,
  `GET /worlds/{canon}/export[?materialized=1]`, `POST /import` and `GET /worlds/{canon}`; `POST /branch`
  answers `501 unsupported_capability` (S6). The routes sit on the existing std server, bound to `127.0.0.1`,
  ID segments are percent-decoded once. A world is a preset name under `fixtures/social-v2/` and a seed; the
  canon ID comes from the pinned runtime. A request whose packets are not in memory answers `202` with a job
  (`job_id`, `dependencies`, non-canonical `progress`); identical requests share the job; a ready request
  answers inline. Limits cap the response, never what the job materializes. A view is built from the domains
  the request names only, so its fingerprint does not depend on what else is in memory.
- **`exchange.rs`**: export `chronos-world-export-1`, canonical bytes. `--canon-only` holds the config, the exact
  bytes of the packs, their SHA-256 and the canon fingerprint. `--with-materialized` adds, sorted by domain, the
  provider archive of each domain already computed, with its fingerprints and the fingerprint of the packet built
  from it. Import rebuilds the canon (another pinned runtime gives another canon and is refused), then adopts
  each archive after the same structural validation as a fresh result and the fingerprint check, and requires
  the rebuilt packet to match. The provider is not rerun: the report says `provider_replayed: false`.
- **CLI**: `v2 export`, `v2 import`, `v2 sample --config FILE --seeds 1..N --out DIR` (aggregates per seed and
  distributions across seeds with the versions, §15.4), `serve --store DIR`.
- **Web** (`web/`): `data-source.js` talks to `/api/v2`, follows jobs with a progress line, and keeps the fixture
  behind `?fixture=1`, the only case where the banner shows. The page has a site selector, reloads the view when
  the year leaves the window or moves more than 12 years, merges the neighbourhood of the entity in focus by ID,
  loads further pages on an explicit click, exports, and has a new map panel (`social-map.js`). Names that are
  fallbacks show a dashed outline in the graph and a `synthetic` badge. The sheet explains tenures, events and
  relations through `/explain` and shows the dated record through `/entities`. Conduct classes in the balances
  are realised counts only: no target is modelled.

### Choices not fixed by the study

1. The window of a view is the 51 years around the date; `period` is that window and `timeline` the canon.
2. `legality` is a map channel computed from standings, not an influence profile of the catalogue.
3. A group's "settlements" are the sites where its members live at the date, plus its seat and the sites of
   other groups of the same archetype.
4. Population forecasts come from `population_expected_after_impacts`, the shock-free baseline is a second
   measure; neither is a band, and no uncertainty is drawn.
5. Export of archives embeds the provider file as text; an archive is about 190 MB for the demo domain.

### Limits

- A first request on a cold store runs the worker (about 36 s for domain 0 in the S4 report) plus the validation
  of 190 MB; the page shows the stage and the elapsed time.
- Import requires the same pinned runtime; without it the canon differs and the export is refused. Strict replay
  of an archive by the provider is not offered.
- Only the domain of the request is materialized; the other domain shows "packet not computed".
- Person and organization names are declared fallbacks until the name grammar exists.
- `sample` runs the provider once per seed and domain, so a hundred seeds take hours; tests use one seed.
- The acceptance walk is played by hand with the Playwright tools, not by a test.

### Validation and measures

`crates/chronos-app/tests/v2_s5.rs` on the real worker: stable and shared jobs, `501` on `/branch`, IDs and
fallback names, pagination (pages disjoint, stable, adding up to the unpaged view), mayor sheet, graph and
explanation, the overlay without a position for the outside, deterministic export, refusal of a damaged export,
import of a canon only and of a computed domain with 0 worker runs, and the sampler. Acceptance walk:
`docs/reviews/social-v2-acceptance.md`.

## Proposed, not implemented

S6–S8 keep their scope in the study: exact personal constraints, branches, bridges
and optimizations. The function signatures proposed in §14.1 are not executable
stubs in S0. Catalogue sources other than the atlas and rendering of name trees belong to
later batches.

## Validation and measures

`tests/v2_s0.rs` contains the pure fixture (seed 42, horizon 1300–1400,
`chronos-social-2`) and the contract regressions. The S0 exit proof compares,
without a provider, direct access by target and the paged projection of an
independent resolution: a frame event, two child battles over two cells, an
epidemic, distinct `effect_id`s per target and the `intensity_draw`s.
Normalization tests permute every sorted list of the plan and of the atlas, with
at least two elements. Their power to fail was only partly verified: a campaign
removing behaviours was interrupted, and most normalization sorts did not go
through it. Probability tests check the per-target decisions, never a quota on
their sum. The example plan is deserialized, validated with its fixture
references, then compared canonically. No performance measurement or
demographic execution is claimed.

---

# Méthode sociale v2 — socle S0 et lot S2 (traduction française)

Traduction complète du texte anglais qui précède ; l'anglais fait foi.

## État implémenté

Le module public `chronos_core::v2` livre les contrats de données du lot S0.
Il est indépendant de `chronos-history-1` : le seul ajout au code v1 est
`pub mod v2;`. `chronos-social-2` désigne la famille v2, sans annoncer un
générateur social complet. Aucun fournisseur, processus Python, accès disque,
horloge ou nouvelle dépendance n'est ajouté au noyau.

### Adresses et décisions indexées

`ids::SemanticAddress` porte le type et les champs de l'adresse. `encode` écrit
`chr2\0`, puis, pour chaque champ, sa longueur u32 little-endian et son UTF-8,
en base64url sans bourrage, derrière le préfixe `type.` (`event.Y2hy…`). Le
point est absent de l'alphabet base64url. `decode` vérifie aussi l'encodage
canonique, les longueurs, le type, l'ASCII normalisé et les entiers décimaux.
Un ID dépassant 4 096 caractères est refusé.

**Chaque objet a une seule forme d'adresse.** `from_fields`, seule porte
d'entrée y compris pour `decode` et serde, fixe par type le nombre et le rôle
des champs :

| Type | Champs |
|---|---|
| `domain`, `plan` | monde, région, profil social, ordinal |
| `person` | domaine à plat, algorithme fournisseur, ID local |
| `household` | domaine à plat, `household`, origine, année de création |
| `site`, `activity` | monde, ID de catalogue |
| `entity` | propriétaire, type, `origin_block`, slot |
| `asset` | propriétaire, `asset`, type, `origin_block`, slot |
| `event` | propriétaire, type, `origin_block`, ordinal |
| `impact` | champs de l'événement parent, slot du contrat |
| `effect` | champs du contrat, type de la cible, champs de la cible sans le monde |
| `group` | propriétaire, `group_slot`, archétype, `origin_block`, ordinal |
| `transfer` | propriétaire, `transfer`, origine, destination, année, ordinal |
| `tenure` | propriétaire, `office`, `origin_block`, slot, ordinal |
| `relation` | propriétaire, `relation`, prédicat, types des extrémités, trois segments comptés, année, ordinal |
| `standing` | propriétaire, `standing`, types des extrémités, trois segments comptés, année, ordinal |

`DomainKey` est le domaine structuré. `OwnerScope` est la portée d'un propriétaire,
en quatre variantes : `world`, `cell`, `site`, `domain`. `Owner` lui ajoute le
monde et entre dans l'adresse à plat, précédé de son discriminant, sans jointure
de chaînes. Une cellule dont le nom imiterait un domaine reste donc une autre
adresse. Les faits portent `owner_scope: OwnerScope`, sans le monde : l'atlas
porte aussi ce type, et il est une entrée du canon, donc antérieur à tout
`canon_id`. `SemanticAddress::owner` relit le propriétaire d'une adresse. Un
mandat prend l'adresse de son office, qui doit être une entité de type `office`.

Les champs littéraux (`household`, `transfer`, `relation`, `standing`, `asset`,
`group_slot`, `office`) sont vérifiés à la construction comme au décodage. `relation` refuse un prédicat
hors de l'énumération fermée et un préfixe commun non maximal. `origin_block`
est l'année de début du bloc : il doit être aligné, et `ids::decade_block` le
calcule par `div_euclid`, y compris avant l'an zéro (`-1` donne `-10`). Années
et blocs sont bornés à ±100 000.

Un champ ne contient jamais d'ID encodé. Une sous-chaîne n'est refusée que si
elle décode en vraie adresse : payload `chr2\0`, champs complets et structure d'un
type connu, à toute position, avec ou sans préfixe de type, suivie ou non d'autres
caractères. La seule présence de la signature base64url `Y2hyMg` ne suffit pas.
Une adresse dont la structure est valide mais qui contient elle-même un ID reste
reconnue, ce qui attrape une double imbrication. La migration, un changement de
nom et une variation de poids ne participent pas à une nouvelle adresse.

Un `ObjectRef` a une seule forme canonique, qu'`ObjectRef::address` vérifie : une
personne est un `PersonSlot`, une entité de type `office` un `Office`, une autre
entité, un groupe ou un ménage une `Entity`. Le constructeur de relation accepte
tout `ObjectRef` adressable, sites, actifs et activités compris, et `link` relit
propriétaire, prédicat, extrémités et début.

`impact` adresse un contrat, fils de son événement. `effect` adresse l'exposition
d'une cible à ce contrat, fille du contrat et unique par cible ; il n'a pas de
dimension annuelle. Les épisodes du profil temporel (§7.2) sont ses sous-slots,
de type `episode`, ajoutés en S2.

`ids::random_semantics_v2` est la façade des tirages. `domain_seed` appelle
exactement le mélangeur v1 avec les trois parties du §4.1. Les autres tirages
préfixent `random_semantics_v2`, le nombre de parties, leurs encodages
base64url, le sous-compteur et l'essai de rejet, tous explicites. Le mélangeur
`keys.rs` reste intact. `uniform_below` est uniforme sur `[0, upper)` par
rejet du préfixe de taille `2^64 mod upper`, sans état partagé. Les tests
épinglent ces dérivations par des valeurs littérales, obtenues d'un port Python
indépendant de `keys.rs` : changer une clé fait échouer un test.

### Octets canoniques

`canonical::canonical_bytes(&serde_json::Value)` produit du JSON UTF-8 compact,
sans BOM, avec clés triées par octets UTF-8, entiers décimaux et échappements
JSON minimaux. Tout nombre flottant est refusé, y compris `1.0`. L'API reçoit
une valeur JSON déjà construite : elle ne convertit pas des objets Rust
flottants, ce qui évite de transformer implicitement NaN en `null`.

La fonction préserve l'ordre des listes. L'appelant doit fournir la projection
contractuelle : listes de faits triées par ID, épisodes et registres dans leur
ordre explicite, champs descriptifs et durées exclus. Elle ne devine ni les
colonnes d'identité ni les champs à exclure dans un JSON arbitraire. S0 ne
calcule et n'authentifie aucun SHA-256 : `plan_canonical_bytes` fournit les
octets, l'hôte calcule l'empreinte. Les empreintes `example` de la fixture
restent des valeurs de démonstration de forme.

Le §4.2 vaut pour tout le socle. `plan_canonical_bytes`, `HistoryAtlas::canonical_bytes`
et `canonical_list_bytes` appliquent l'ordre canonique sans exiger qu'il soit celui
reçu. `canonical_list_bytes` traite toute liste de `CanonicalRecord` : entités,
états, relations, événements, personnes et statuts légaux. Elle trie par ID, ou par
entité puis date pour les états, et trie les listes internes d'IDs. Épisodes et
registres gardent leur ordre, qui est un fait.

Les vecteurs [canonical-json.v1.json](../design/chronos-social-v2/vectors/canonical-json.v1.json)
contiennent entrée, texte UTF-8 attendu et octets hexadécimaux. Ils couvrent
Unicode, contrôles, ordre des clés, ordre conservé des listes et les deux bornes
entières. `integer_bounds` documente ces bornes : de `-9223372036854775808` à
`18446744073709551615`, incluses. `rejections` liste les textes à refuser :
flottants, exposants, `-0`, entiers hors bornes, `NaN`, `Infinity`, surrogate
isolée et BOM. Rust les refuse au décodage du texte ou à la canonisation. Le
module `json` de Python accepte plusieurs de ces textes par défaut : le futur
worker devra les refuser explicitement. Ces vecteurs ne valident pas encore un
worker. Une clé d'objet dupliquée dans un texte JSON n'est pas détectée : le
décodage garde la dernière valeur avant que `canonical_bytes` ne la reçoive.

### Atlas, modèle social et temps

`atlas::HistoryAtlas::validate` vérifie références et unicité, hiérarchie
acyclique, bornes et référentiels, voisins réciproques, catalogues complets
de sites par sous-arbre et sommes des populations attendues maximales.
Fondations futures et abandon imposé sont refusés. Les occupations couvrent
exactement l'horizon, sans lacune ni chevauchement. Les routes portent des
jours entiers, une validité et un sens ; une liste vide reste une liste vide.
Les IDs d'atlas sont en ASCII imprimable sans espace, les coordonnées bornées à
±9e15 mm. Toute erreur porte l'atlas comme propriétaire et sa date d'ouverture.

`ExternalContact` a un propriétaire unique, le monde ou une cellule qui enclôt ses
sites, et deux extrémités triées. Une extrémité est un site d'attache, avec le
contexte, le profil et la population qu'il voit au-delà et sa voie d'accès
éventuelle, ou un contexte purement extérieur. Il faut au moins un site. Face à un
contexte extérieur, le site déclare ce même contexte. Deux sites peuvent partager
un contact : chacun ouvre alors son port sous le même `contact_id`.

Le JSON d'atlas n'étant pas figé par un schéma dans l'étude, S0 emploie des
surfaces `area_m2` entières et des bornes métriques `*_mm`, sans conversion
géographique ni interprétation des LOD d'affichage. Les résumés d'une cellule
incluent ses descendants. La partition démographique relève du lot S2.

`model` sépare provenance, précision et couverture des comptes, entités,
états, relations et personnes. Les prédicats du §5.1 sont une énumération
fermée. Les phases d'événement sont sérialisées avec leurs numéros du §7.3.
`TimeSpan` décrit `[start_year, end_year)` ; seules les vies peuvent avoir
un intervalle vide. `HistoryEvent.end_year` donne la fin exclusive d'un
événement-cadre et vaut `null` pour un événement daté.

`CountMeasure.value`, `PersonRecord.death_year`, `HistoryEvent.end_year` et les
causes de `LegalStanding` et `AlignmentEpisode` exigent la présence explicite
de `null` : un champ absent est refusé, car `null` dit « inconnu » ou « aucun ».

`PersonRecord.observed_span` donne les années observées par le fournisseur ; le
nom évite la confusion avec l'axe `coverage` du §4.3. `is_alive_and_covered` exclut
l'année du décès et toute date hors de cet intervalle, même si le décès est inconnu.
`parent_refs` vise une ligne (`slot`) ou une référence extérieure typée
(`coverage_limit`, `external_arrival`). `PersonRecord::validate` borne les dates,
refuse un décès avant la naissance, valide `observed_span`, et limite les parents
à deux, seule la reproduction biparentale étant livrée.

**Identité des enregistrements.** `validate` prouve qu'un fait est bien celui que
son ID adresse. Erreur : `InvalidRecordIdentity`.

| Fait | Contrôle |
|---|---|
| `SocialEntity` | `id` égale `semantic_address` ; propriétaire égal à `owner_scope` ; entité : `kind` dans l'adresse ; slot de groupe : `archetype_id` dans l'adresse ; ménage : `kind = household` ; personne : `kind = person` et `nature = person`, seule nature admise pour cette adresse |
| `HistoryEvent` | adresse d'événement de ce type et de ce propriétaire |
| `SocialRelation` | adresse de relation de mêmes prédicat, extrémités et début |
| `LegalStanding` | adresse de statut de mêmes sujet, juridiction et début |
| `PersonRecord` | adresse de personne dont l'ID local est `provider_ref` ; parents de table adressés |
| `ImpactContract` | adresse d'impact, fille de `parent_event_id` et portée par `owner_scope` |

`causal_ranks`, `validate_composition`, `validate_legal_standings` et `resolve_impact`
appellent ces contrôles. Un slot de groupe ne contraint pas `kind`, que son adresse
ne porte pas.

`SocialEntity.name_tree` stocke l'arbre produit par la grammaire Namebench figée.
L'arbre canonique n'est rendu en texte qu'à l'affichage. S0 le type comme `NameTree`, valeur JSON opaque qui passe le
contrôle canonique à la construction comme à la lecture : un flottant y est
refusé. Le champ est absent tant qu'aucun arbre n'existe, `null` n'est pas
accepté à sa place, et S0 ne contient aucune logique de génération.

`validate::causal_ranks` refuse les références absentes, doublons, causes
futures, dates d'événement-cadre invalides et cycles. Tout cycle causal reçoit
`CanonicalDependencyCycle`, y compris s'il traverse des phases ou des années.
Les rangs sont construits par tri topologique dans chaque phase ; l'ID départage
seulement les nœuds déjà disponibles. Une cause de même année en phase
antérieure est acceptée. `validate_composition` refuse les cycles stricts
`component_of` sur l'ensemble des épisodes, tout en autorisant les affiliations
superposées. `CompositionCycle` désigne un vrai cycle ; un ID de relation
dupliqué reçoit `InvalidCanonicalValue`. Ces contrôles ne constituent pas encore
une validation de généalogie fournisseur.

### Alignement et légalité

`AlignmentAxes` porte `morality_bps` et `lawfulness_bps`, bornés à ±10 000 à la
construction comme à la lecture. `conduct_class` est une fonction pure des axes
et de `ConductThresholds` : le noyau ne fige aucun seuil, leurs valeurs viennent
d'`engine_defaults.alignment` en S1. La classe n'est jamais stockée.
`entity_conduct_class` rend `None` pour une catégorie ou une population.

`EntityState` porte `alignment`, `conduct_tags` et `visibility`. Un tag de conduite
est un ID, pas une énumération Rust : le catalogue évolue en donnée (§5.2), et
`EntityState::validate` et `PersonRecord::validate` reçoivent ce catalogue en
paramètre. `PersonRecord.alignment_episodes` est une liste ordonnée : tout épisode
après le premier cite l'événement qui change les axes.

`LegalStanding` est relatif à une juridiction et daté. Son sujet est une entité,
une activité ou un slot de personne ; sa juridiction est une entité.
`legal_status_at` rend `licit` sans fiche applicable, et une autre juridiction
n'a aucun effet. `validate_legal_standings` reçoit `observed_start` et la fin du
monde, regroupe les fiches par couple sujet/juridiction et refuse chevauchements
et sorties de période. Seul un segment commençant à `observed_start` peut être sans
cause ; tout autre début en cite une, y compris un premier segment tardif. Un
segment qui finit avant la fin du monde sans successeur immédiat, ou avant un
trou, cite son événement de fin dans `end_cause_event_id`. Toutes ces causes sont
de type `edict`, `charter`, `heresy_trial` ou `banishment`, sans être postérieures
à la date qu'elles expliquent. Depuis S2, la première fiche d'une chaîne peut citer la
`foundation` de son sujet, et la dernière sa `dissolution`.

### Impacts

`region::resolve_impact` consomme un `CompleteCandidateCatalogue`. Ce type n'a ni
champ public ni `Deserialize` et ne se construit que depuis une source d'autorité
du noyau : `from_atlas(&atlas, &owner)`, qui énumère elle-même les sites de l'atlas
validé, pour le monde, une cellule ou un site. Aucune liste n'est reçue de
l'appelant. Un domaine est refusé avec `UnsupportedCapability` : sa partition
appartient à S2, qui ajoutera d'autres sources de la même façon. Une projection ne
produit jamais un catalogue et ne recalcule jamais de quota.

Le contrat, son événement parent, le catalogue et les effets partagent canon et
propriétaire : un contrat d'un autre monde ou d'une autre portée que le catalogue
est refusé avec `InvalidImpactContract`.

En mode probabiliste, chaque cible éligible tire une seule exposition en bps,
sans dimension annuelle. Le mot aléatoire d'intensité a sa propre clé. Il est
livré comme `intensity_draw` ; l'interprétation du profil de gravité et les
épisodes locaux relèvent de S2. Chaque `TargetImpact` porte l'ID de l'événement
parent et un `effect_id` propre à sa cible. Ces identités sont dérivées du
contrat et de la cible : S0 n'invente pas de fait à la consultation. Les clés de
tirage portent l'ID du contrat, donc son adresse.

En mode exact, **K se calcule sur les candidats éligibles** à `eligibility_time`
et retenus par le sélecteur, jamais sur le catalogue brut : un compte déclaré
doit y tenir, une fraction porte sur leur nombre. L'arrondi est obligatoire :
seuls `floor` et `ceil` sont pris en charge, sans arrondi implicite. Le classement
utilise priorité puis ID, sur l'ensemble complet. Le budget de 100 000 candidats
se mesure en revanche sur le catalogue entier, avant filtrage : 100 000 est
accepté, 100 001 refusé. Un K impossible échoue. Les modes exacts, y compris cible
fixe, sur personnes et cohortes sont refusés avec `UnsupportedCapability` jusqu'à
S6. `ExactImpactInfeasible` ne sert qu'aux modes non probabilistes qui ne peuvent
atteindre leur cible ; un contrat mal formé reçoit `InvalidImpactContract`.

### Plans de population

`population::PopulationPlan` reprend le schéma 1, avec types fermés,
`deny_unknown_fields`, champs nullable obligatoires et effets optionnels
omettables mais non nuls. Les variantes futures restent lisibles.
`validate_plan(plan, references)` vérifie le schéma embarqué à la compilation et
lu une seule fois, les bornes, unités, dates, références, poids positifs, rangs
sociaux contigus, graine de domaine, couvertures d'épisodes et budgets du prototype.
`PlanReferences` fournit le canon, l'horizon du monde, `world_seed`, les
événements, offices, contacts, raccords culturels, activités, empreintes des
packs et maxima attendus de l'atlas sur tout l'horizon. Ces références sont
préparées par l'autorité du canon et des catalogues, jamais par la fenêtre de
consultation.

Le plan est lié à son canon : `canon_id`, `world_seed` et `timeline` égalent ceux
des références. `domain_id` est une adresse de domaine de ce monde et de ce profil
social, `plan_id` l'adresse de plan du même domaine. `domain_sites` porte la
partition attendue : les sites du plan égalent exactement ceux de son domaine, et
les mêmes sites sous un autre ordinal sont refusés. L'exemple de plan porte de
vraies adresses, et sa graine de domaine en découle.

`culture_variants` résout chaque `mu_culture_id` vers sa famille et son profil
social. La variante principale, toutes les variantes de poids non nul de tous les
épisodes et tous les sites du domaine doivent résoudre le profil du plan ; sinon
`UnsupportedCapability`, jusqu'à S8. Une variante de poids nul peut avoir un autre
profil mais doit exister dans le raccord. Une référence absente ou une famille
erronée reste une erreur de plan, pas de capacité.

Un profil de parenté doit reprendre le `dependent_age` du profil social, ne pas
permettre un parent encore dépendant et utiliser un modèle de ménage admis pour
le mode de résidence. Un seul modèle est livré aujourd'hui, compatible avec les
trois modes : ce dernier contrôle n'a donc d'effet qu'à l'arrivée d'un second
modèle. Un port reprend le contexte, le profil et la population de l'extrémité du
contact ancrée à son site.

`plan_canonical_bytes` exclut `plan_id` et `plan_fingerprint`. Sont triés par ID :
capacités, profils, sites, demandes de rôles, personnes fixées, contraintes, ports,
routes et leurs listes d'IDs ; les marqueurs le sont par site puis année, les chocs
par `shock_id`. Les épisodes d'occupation, épisodes de taux et
périodes protégées gardent leur ordre.

`historical_program` est une liste explicitement ordonnée par `(année, phase, op_id)`,
clé d'application et de normalisation que rend `application_key`. La phase découle
du type d'opération : 10 pour `protect_person`, à sa date de début ; 30 pour
`force_death` ; 40 pour `move_households` et `admit_people`.

Le validateur accepte les données ordinaires S0–S5 comme contrats à préparer ;
il n'exécute aucune capacité S3. Il refuse capacités inconnues, champs S6–S8,
personnes fixées, contraintes démographiques exactes, opérations personnelles,
rôles requis, noyau requis, ponts nominatifs, reproduction personnalisée et
fournisseur entièrement procédural. La politique libre requiert les quatre
tentatives déclarées du §25.4, sans les exécuter.

Les paramètres reconnus sont ceux de l'exemple normatif, tous explicites.
Les `_bps`, `_ppm`, `_mm` et âges sont contrôlés sans flottants. `residence`
accepte `patrilocal`, `matrilocal` et `either`, comme `config.py` de Lineage Lab ;
toute autre valeur est une erreur de plan. Les conversions d'effets du §26.4
restent à effectuer dans l'adaptateur S3, séparément des noms de paramètres de
profils.

## S2 : canon général, régions et domaines

**Entrées.** `general::build_canon(config, atlas, catalogue, constraints)` lit le
catalogue S1 par `catalog::HistoryCatalogue`. `WorldConfig` porte `canon_id` et
`society_ids` (template vers `demography_society_id`) : l'hôte calcule les deux, et le
noyau n'a donc pas de SHA-256. `SettlementAnchor.site_tags` et
`HistoryAtlas.provenance` sont de nouveaux champs obligatoires. Les faits tirés d'un
atlas synthétique restent `synthetic`. `fixtures/social-v2/region-demo.atlas.json` et
`region-demo.world.json` sont virtuels : une région, quatre cellules, 20 sites, quatre
cultures, deux profils sociaux, 24 routes et quatre contacts sur 1300-1400. La graine
11 vient d'un balayage des graines 1 à 60 aux taux par défaut, car elle montre tous les
types d'événements ; ce n'est pas une calibration.

**Domaines.** `atlas::partition_domains` est le next-fit du §3.1. Un site au-delà de
20 000 est refusé avec `BudgetExceeded`, un site mêlant des profils sociaux avec
`UnsupportedCapability`.

**Processus généraux.** Chaque région suit sa propre boucle annuelle, avec des tirages
par bloc décennal sous sous-clés fixes ; un bloc partiel garde sa clé complète et
écarte les dates hors période. Les collectifs politiques sont dérivés : un par cellule
fille de la région contenant des sites, et les cellules voisines donnent les paires de
guerre. Guerre et épidémie sont des événements-cadres, avec un `ImpactContract`
chacun. À la paix, un `treaty` tire chaque site `frontier` des deux parties et peut en
déplacer le contrôle. La base suit le §7.1 par région et culture, avec un reste signé,
et ne lit aucun événement.

**Région.** `region::plan_region(canon, region_id)` résout chaque contrat sur le
catalogue complet et compile les expositions en `LocalShock` avec les profils du §7.5.
La date locale a sa propre clé ; une arrivée militaire ne précède jamais
`début + ceil(travel_days / 365)`. Chaque choc a une adresse du nouveau type `episode`,
sous-slot de son effet. Les marqueurs de site suivent le §26.5, ou les séries d'auteur
quand `AuthorConstraints.site_baselines` couvre la région.
`population_expected_after_impacts` est une prévision séparée, qui n'alimente aucun
marqueur. Chaque domaine reçoit ses ports, triés par temps de trajet puis ID et bornés
par `max_halo_contacts` ; le reste devient un résumé. Les deux côtés d'un contact
partagent un seul flux tiré : la sortie de l'un est l'entrée de l'autre.

**Événements de naissance.**

| Événement | Définition | Groupe né |
|---|---|---|
| `diplomatic_contact` | Un tirage par contact et décennie, porté par le propriétaire du contact, phase 10, avec ses sites d'attache pour participants | `embassy` sur un site d'attache, si `availability` le permet |
| `displacement` | Un tirage par site ruiné, porté par ce site, phase 40, causé par le cadre de guerre ; il alimente un `move_households` vers le site le plus proche du domaine, sinon vers un port | `refugee_community` sur un site de destination, si `availability` le permet |
| `heresy_trial` | Acte légal d'une juridiction (§27.3) | `inquisition` sur le site du groupe, si `availability` le permet |

**Légalité.** Le statut initial suit le §27.4 sous le collectif qui contrôle le site.
S2 crée les groupes à stock propre et les groupes nés d'un événement, avec âges de
bootstrap et flux par bloc. Par collectif, type et décennie, une juridiction peut
émettre `edict` (toléré vers proscrit), `charter` (toléré vers licite), `heresy_trial`
ou `banishment`. Un changement de contrôle émet un `charter` causé par le traité, qui
ferme les anciennes chaînes et ouvre les nouvelles. `validate_legal_standings` admet
aussi deux événements de vie : `foundation` pour la première fiche d'une chaîne et
`dissolution` pour la dernière, à leur date exacte et avec le sujet pour participant.

**Plan.** `population::plan_population(canon, region_plan, domain_id)` rend un plan qui
passe `validate_plan`. Il porte un choc par épisode local : `shock_id` est l'adresse
`episode`, distincte de `source_event_id`, que plusieurs chocs partagent. `validate_plan`
refuse une seconde exposition d'un site à un même événement, quelles que soient ses dates.
`plan_fingerprint` vaut `PLAN_FINGERPRINT_PENDING` jusqu'à ce que
l'hôte hache `plan_canonical_bytes`.

**Diagnostics.** Toute `HistoryError` porte un code anglais stable dans `constraint`
(`domain.site_over_population_cap`, `plan.unknown_domain`...) et jamais une phrase.
`owner`, `at_year` et la table facultative `params` (par exemple `path` pour
`plan.schema_shape`) en sont les paramètres. Chaque code a un message sous le préfixe
`diagnostics.` dans `web/i18n/en.json` et `web/i18n/fr.json`, et `tools/check_i18n.mjs`
échoue si un code levé par `src/v2` n'a pas de message ou si un message n'est jamais levé.
Les genres d'erreur du §13 sont inchangés.

**Limites.** Non livrés : institutions ancrées, offices et `role_demands` (S4), unités
d'activité et stocks partagés (S4), contagion interrégionale, états de richesse et de
cohésion, processus pour `harvest.bad`, catalogues de candidats d'un propriétaire
domaine. Un contact détenu par le monde numérote ses événements entre régions : en
ajouter un décale ces ordinaux.

**Validation.** `tests/v2_s2.rs` prouve le rejeu à l'octet,
`world_extent_does_not_reseed_domains`,
`generated_baseline_nodes_are_independent_of_shocks`,
`spared_village_keeps_baseline_target` au niveau du plan et l'ordre d'appel de
`plan_region`. Aucune performance n'est mesurée, et aucun fournisseur n'a exécuté ces
plans.

## S3 : hôte du worker, validation et cache

### Implémenté

`chronos-app` lance le worker Lineage Lab comme processus enfant, en JSONL sur stdin/stdout,
depuis une liste d'arguments. Chaque job a un processus neuf et une racine de job privée.
`prepare` envoie le plan dans l'ordre canonique de ses listes, puis `materialize` renvoie
`result_ref`. L'hôte vérifie dans l'ordre : chemin dans le répertoire du job, taille, SHA-256,
octets canoniques, puis `result_fingerprint`. Tout échec est une erreur typée, ne laisse aucun
paquet et ne choisit aucun autre fournisseur.

`validate_population(plan, result)` est pure et ne lit que les tables : références, couverture,
chronologie sur `(année, phase)`, une seule chaîne de résidences continue par slot, un ménage
par résident, parenté réciproque, comptes clairsemés et bilans par site-année et par domaine.
Le `validation_proof` du worker n'est pas pris pour acquis. `VerifiedPopulation` a des champs
privés et aucun désérialiseur : une lecture à chaud revalide donc le résultat.

`canon_id` est désormais dérivé par l'hôte : `canon-` suivi des 32 premiers chiffres hexadécimaux
du SHA-256 du manifeste normalisé (configuration résolue, contraintes, empreintes d'octets de
l'atlas et du catalogue, manifeste du runtime et `runtime_fingerprint`). Le `canon_id` d'un
fichier de monde est ignoré. Un fichier portant `synthetic-runtime-not-pinned` est épinglé sur le
runtime trouvé ; une autre valeur épinglée échoue avec `ProviderVersionMismatch`. Un code
fournisseur modifié donne donc un autre canon, avec d'autres IDs de domaines et de plans.

Les paquets sont immuables sous `worlds/<canon>/` et publiés par un seul renommage. Les noms de
fichiers sont le SHA-256 de l'ID, comme le permet le §16.1 ; les vrais IDs sont comparés à chaque
lecture. La clé de cache de population couvre canon, propriétaire, empreintes du plan et de la
région, versions, protocole, format et niveau de détail. Un paquet endommagé est une erreur et
n'est jamais recalculé en silence. Un domaine est résolu une fois ; les requêtes concurrentes
partagent son job, et un job échoué peut être rejoué.

### Limites

`fixed_people`, `exact_constraints` et les demandes de rôles ne sont pas vérifiés : les plans S3
les laissent vides. Les empreintes de packs portent sur les octets des fichiers : les fins de
ligne changent donc le canon. Le nombre de workers concurrents n'est pas plafonné. Il n'y a pas de
mode archive seule : ouvrir un monde exige le runtime. Les entrées de répertoire ne sont pas
synchronisées après un renommage.

### Validation et mesures

`crates/chronos-core/tests/v2_s3.rs` contient `crisis_is_applied_once` et le test de bilan falsifié.
`crates/chronos-app/tests/v2_s3.rs` exécute le vrai worker si `CHRONOS_PYTHON` et
`CHRONOS_LINEAGE_SRC` sont définis, et sinon s'ignore avec un message.

Mesuré le 2026-11-06, build release, i9-12900K, Windows 11, fixture de démonstration avec la seed
de monde 11, siècle complet, sur une copie figée conforme au commit fournisseur `67f7842` (runtime
`6ef2eee98aad`), un échantillon par cas, sur une machine partagée avec d'autres travaux :

| Domaine | Slots | Archive | Premier lancement, cache JIT vide | Worker, JIT en cache | Lecture à chaud | Pic de l'hôte |
|---|---:|---:|---:|---:|---:|---:|
| 0 | 19 266 | 146,8 Mio | 28,5 s | non mesuré | 2,8 s | 899 Mio |
| 1 | 24 137 | 187,5 Mio | 34,1 s | 28,5 s | 3,3 s | 1 066 Mio |

La mémoire du worker lui-même n'a pas été mesurée ici.

## S4 : couche sociale

### Implémenté

`social::build_social_packet(region_plan, verified_population)` est pure et construit un paquet
par domaine sur tout l'horizon. `RegionPlan.social` porte les entrées figées du canon qu'elle lit
(catalogue, atlas, collectifs, contrôle) : ce champ est hors de la projection canonique, donc les
deux arguments du §14.1 tiennent et les empreintes de région ne bougent pas. `build_social_prefix`
s'arrête à une année et donne les mêmes faits que le paquet complet lu avant cette date (§9.5).
`offices::resolve_office(packet, office_id, at_year)` ne fait que lire : l'année demandée ne tire rien.

**Attributs** (`attributes.rs`). Statut, activité, tirage de participation, culture, latent familial,
axes d'alignement et compétences initiales, sous `social/attributes/v1` et `alignment/v1`, avec la
graine de domaine, la clé de famille et la clé de personne. Le sélecteur du §9.4 choisit l'uniforme
familial ou l'individuel, jamais leur moyenne. Le statut suit les résidus du §8.4. Les attributs du
noyau sont conservés. Aucun tirage ne lit `ancestry_id`.

**Slots de groupes** (`social.rs`). Les groupes de S2 sont repris tels quels. S4 ajoute, sous
`social/group/v1` et par site : une ancre par occupation (`site_occupation_anchor`), une ancre
secondaire par seuil, les unités d'activité qui se partagent les travailleurs attendus, et les stocks
partagés d'associations, communautés et cercles d'amis, avec âges de bootstrap et flux par bloc comme
en S2. Les cercles utilisent les tables de durée et de taille du catalogue. Foyers et lignages restent
des vues : `household_of`, `family_of`, `site_views`, et `person_record` pour le `PersonRecord` du §5.

**Année.** La phase 50 ferme les engagements (décès, départ, dissolution, capacité, sièges interdits)
et tous les mandats terminés. La phase 55 traite cohésion et scissions. La phase 60 attribue les
offices, y compris ceux qu'une scission vient de créer, puis emplois, autres groupes à budget de
temps et affiliations superposables, et enfin les cercles. Chaque tirage a pour clé `(request_id, année, vacancy_ordinal, draw_ordinal)`.
Les poids sont ceux du §23.2, avec `alignment_fit` multiplicatif. Le réseau lit l'état publié Y−1.
Les cercles créent des liens `trusts`, un par membre, jamais une clique.

**Relations.** `Membership.opened_year` garde le début du fait complet : une appartenance tronquée
donne le même ID de relation dans toute fenêtre, et seul `valid` suit la fenêtre. `cause_event_id` dit
ce qui a ouvert une relation (une scission, pour un membre transféré), `end_cause_event_id` ce qui l'a
fermée.

**Offices.** Une ancre de site a un chef (`mayor`) et un corps collectif ; une ancre secondaire et une
association ont un chef. Une échéance, un décès, un départ ou une perte d'éligibilité ferment le mandat
à cette frontière ; le remplacement ou la vacance s'ouvre à la même date. Le premier titulaire observé
porte `initial_observed_binding` et aucun événement. Un emploi à plein temps est préempté et tracé par
`employment_preemption`.

**Légalité.** Les groupes de S4 ont des chaînes de statuts comme ceux de S2 et citent `foundation`,
`dissolution` et la `charter` de contrôle. `in_conflict_with` se déduit des statuts proscrits de groupes
ouverts. Une personne proscrite ou un membre connu d'un groupe ouvert proscrit est écarté des offices de
cette juridiction et de ses corps. À l'inverse, un groupe ouvert proscrit ne recrute jamais le titulaire
d'un office ou d'un siège de la juridiction qui le proscrit. Le chef privé d'un groupe n'est pas un office
de la juridiction.

**Noms (L5).** `name_tree` est développé une fois depuis le pack Namebench figé,
le `name_profile` exact, la culture source et le ton de fondation. `GroupRecord.name`
conserve le descripteur historique ; `name_fallback` est explicitement `synthetic`
uniquement sans pack ou sans kind développable. Les arbres des vues de ménages/lignées
simulées sont dans `person_group_names`. Voir [la méthode implémentée](naming-v2.md).

**Noms affichés.** `display_names` nomme personnes, sites, régions et collectifs depuis
`fixtures/naming/person-pools.v1.json`, sous `display-names/v1`, un flux qu'aucun fait ne lit. Le prénom
suit le sexe et la clé de personne ; le nom de famille suit la clé de famille de l'ancêtre connu le plus
ancien par le père, sinon la mère, et sa culture. Un collectif porte le nom de son siège. `render_tree`
donne le nom de chaque organisation en `en` et `fr` depuis son `name_tree`, et ces libellés remplissent
ses parties `ref`. Chaque libellé montre sa provenance ; les pools sont des données actuelles. Leur
empreinte entre dans le manifeste du monde.

**Chefs d'association.** Le chef d'une association en est membre. Le premier titulaire est tiré parmi les
résidents et devient son premier membre ; ensuite seuls les membres courants sont candidats : une
association sans membre éligible reste vacante et subit la pénalité de cohésion de -400 bps. Le chef d'une
association à budget de temps n'ajoute aucun coût à cette appartenance, et une scission le laisse en place.

**Influence.** `influence::influence_at` lit un paquet : force tirée des membres réalisés, atténuation
par jours de trajet, persistance après la fin du groupe, plafond par canal.

### Choix non fixés par l'étude

- Règle du chef : les templates `collective_body` élisent leur chef (`weighted_election`) ; un template
  `appointment` ou `seniority` donne sa règle. `lordship` est héréditaire, lu sur l'ID d'archétype car le
  catalogue ne déclare aucun champ de succession. Les ancres secondaires nomment.
- `hereditary` tire parmi la parenté éligible du dernier titulaire, puis parmi tous les candidats.
- Une personne tient une seule charge personnelle à la fois. Associations et communautés recrutent dans
  le budget de trajet `recruitment`, les autres sur leur site ; la portée est lue au début du monde.
- Le `status_fit` d'un groupe est la proximité d'un rang stable tiré par slot ; pour un office, c'est le
  rang lui-même. Les cercles d'amis n'ont ni statut légal ni événements de vie.

### Limites

Un édit n'atteint que les archétypes détenus par S2 ; les groupes de S4 ne changent de statut qu'aux
changements de contrôle. Le `stock_multiplier` des overlays n'est pas appliqué. Les statuts de personnes
sont lus, jamais générés. Un paquet est recalculé, pas stocké : `Orchestrator::social_packet` le garde en
mémoire. Les mandats n'ont pas encore de cas `claimed` ni `regent`. Statut et activité sont fixés pour la
vie. Les poids ne sont pas réglés.

### Validation et mesures

`src/v2/social_tests.rs` contient les preuves pures sur une table écrite à la main, et
`crates/chronos-app/tests/v2_s4.rs` exécute les deux domaines de démonstration sur le vrai worker, et
s'ignore avec un message sans `CHRONOS_PYTHON` et `CHRONOS_LINEAGE_SRC`. `chronos-app v2 social` affiche
les groupes par classe et classe de conduite, les vacances et les maires.

Mesuré le 2026-11-06 sur la fixture de démonstration, seed de monde 11, pic de fécondité 4000, un seul
lancement : aucune vacance, car le plus petit vivier vu par une sélection compte 35 candidats, et une
scission, celle d'un cercle d'amis en 1392. Avec deux sites réduits à 8 habitants initiaux, le vrai worker
donne 21 années-offices vacantes. Une vacance exige un site sans adulte éligible ; ces chiffres ne disent
rien des autres seeds.

## S5 : API v2, vues web sur données réelles, export et échantillonnage

Implémenté le 06/11/2026 (lot `social-v2/s5-final`). Proposé dans l'étude et implémenté ici : §14.2, §14.3, §15.2,
§15.4, §16.2, §18.4, §27.6. Mesuré : le parcours de `docs/reviews/social-v2-acceptance.md`.

### Implémenté

- **`chronos_core::v2::query`** (pur, compatible WASM). `project_view(canon, materialized, query)` rend une
  `HistoryView` : entités, charges, relations, statuts légaux, événements, mandats, mesures attendues et
  réalisées, bilans, la fenêtre de faits appliquée (`period`) et toute la `timeline`, la `coverage` avec le nombre
  de domaines matérialisés, la `provenance` et les limites. Un domaine sans paquet est une erreur
  (`query.packet_not_materialized`), jamais une supposition. Les IDs sont `entity.id` et les adresses de
  personnes ; l'indice de groupe local au paquet et le slot de personne ne sortent jamais du module. Les entités
  portent `precision`, `coverage`, `provenance` et un nom de repli déclaré : `org_name.fallback` avec
  `name_kind.<kind>` pour les organisations, `person.fallback` (étiquette, année de naissance, activité) pour les
  personnes, tous marqués `name_synthetic`. Les preuves de sélection sont des comptes et des poids, pas des listes
  de candidats (§15.4). `entity_sheet` et `explain_fact` servent la fiche datée et l'explication d'un événement,
  d'un mandat, d'un statut, d'une relation, d'une appartenance ou d'un lien de parenté.
- **Choix de ce qu'une vue contient.** Sans `scope.ids`, les graines sont les charges et organisations
  (institutions, associations, communautés) du site, puis les titulaires de sa mairie sur la fenêtre et des autres
  charges à la date demandée. Une expansion en largeur (profondeur 0, 1 ou 2 selon `detail`), dans un ordre fixe,
  ajoute parents, partenaires, enfants, appartenances, charges et juridictions. Avec `scope.ids`, les entités
  demandées sont les graines et un groupe ouvre jusqu'à 16 membres. Une liste tronquée n'est jamais complète.
- **Pages.** Une seule liste fusionnée de nœuds et de relations (une relation suit son dernier point d'attache).
  Une page s'arrête quand `limits.people` ou `limits.relations` est atteint ; `cursor` vaut
  `c1.<position>.<empreinte de la requête>`, donc stable et refusé pour une autre requête. `limits.complete` n'est
  vrai que si aucune page ne suit et qu'aucun événement n'a été coupé. `max_nodes` 300 et `max_edges` 1000 sont les plafonds de dessin du
  graphe web.
- **Superposition** (`HistoricalOverlay`, §15.2) : repère de l'atlas, sites à leurs positions d'atlas, routes
  valides dans l'année, canaux `recruitment`, `economy`, `culture`, `religion`, `military`, `memory` (via
  `influence_at`) et `legality` (part de groupes tolérés ou proscrits par site), contacts extérieurs, et pour un
  groupe choisi les sites où vivent ses membres, son siège et les autres implantations de son archétype. Un contact
  garde son site d'attache et nomme son contexte extérieur ; aucune position n'est donnée à l'extérieur.
- **`chronos-app/src/v2/server.rs`** : `POST /api/v2/worlds`, `POST /worlds/{canon}/query`, `GET /jobs/{id}`,
  `GET /worlds/{canon}/entities/{id}?at_year=`, `POST /worlds/{canon}/graph`, `GET /worlds/{canon}/explain/{fact_id}`,
  `GET /worlds/{canon}/export[?materialized=1]`, `POST /import` et `GET /worlds/{canon}` ; `POST /branch` répond
  `501 unsupported_capability` (S6). Les routes reposent sur le serveur std existant, lié à `127.0.0.1`, les
  segments d'ID sont décodés une seule fois. Un monde est un nom de préréglage sous `fixtures/social-v2/` et une
  graine ; l'ID de canon vient du runtime épinglé. Une requête dont les paquets ne sont pas en mémoire répond `202`
  avec un job (`job_id`, `dependencies`, `progress` non canonique) ; les requêtes identiques partagent le job ; une
  requête prête répond en ligne. Les limites plafonnent la réponse, jamais ce que le job matérialise. Une vue est
  construite depuis les seuls domaines que la requête nomme : son empreinte ne dépend pas de ce qui est en mémoire.
- **`exchange.rs`** : export `chronos-world-export-1`, octets canoniques. `--canon-only` contient la config, les
  octets exacts des packs, leur SHA-256 et l'empreinte du canon. `--with-materialized` ajoute, triée par domaine,
  l'archive du fournisseur de chaque domaine déjà calculé, avec ses empreintes et celle du paquet construit
  depuis elle. L'import reconstruit le canon (un autre runtime épinglé donne un autre canon et est refusé), puis
  adopte chaque archive après la même validation structurelle qu'un résultat frais et le contrôle d'empreinte, et
  exige que le paquet reconstruit corresponde. Le fournisseur n'est pas relancé : le rapport dit
  `provider_replayed: false`.
- **CLI** : `v2 export`, `v2 import`, `v2 sample --config FICHIER --seeds 1..N --out DOSSIER` (agrégats par graine
  et distributions entre graines avec les versions, §15.4), `serve --store DOSSIER`.
- **Web** (`web/`) : `data-source.js` parle à `/api/v2`, suit les jobs avec une ligne de progression et garde la
  fixture derrière `?fixture=1`, seul cas où la bannière s'affiche. La page a un sélecteur de site, recharge la vue
  quand l'année sort de la fenêtre ou s'éloigne de plus de 12 ans, fusionne par ID le voisinage de l'entité
  en focus, charge les pages suivantes sur clic explicite, exporte, et porte un nouveau panneau carte
  (`social-map.js`). Les noms de repli ont un contour en tirets dans le graphe et un badge `synthetic`. La fiche
  explique mandats, événements et relations par `/explain` et montre la fiche datée par `/entities`. Les classes de
  conduite des bilans sont des effectifs réalisés seulement : aucune cible n'est modélisée.

### Choix non fixés par l'étude

1. La fenêtre d'une vue couvre 51 ans autour de la date ; `period` est cette fenêtre et `timeline` le canon.
2. `legality` est un canal de carte calculé depuis les statuts, pas un profil d'influence du catalogue.
3. Les « implantations » d'un groupe sont les sites où vivent ses membres à la date, plus son siège et les sites
   des autres groupes du même archétype.
4. Les prévisions de population viennent de `population_expected_after_impacts` ; la base sans choc est une
   seconde mesure ; ni l'une ni l'autre n'est une bande, et aucune incertitude n'est tracée.
5. L'export des archives embarque le fichier du fournisseur sous forme de texte ; une archive pèse environ 190 Mo
   pour le domaine de démonstration.

### Limites

- Une première requête sur un stock froid lance le worker (environ 36 s pour le domaine 0 dans le rapport S4)
  puis valide 190 Mo ; la page montre l'étape et le temps écoulé.
- L'import exige le même runtime épinglé ; sans lui le canon diffère et l'export est refusé. Le rejeu strict d'une
  archive par le fournisseur n'est pas proposé.
- Seul le domaine de la requête est matérialisé ; l'autre domaine affiche « paquet non calculé ».
- Les noms de personnes et d'organisations sont des replis déclarés tant que la grammaire de noms n'existe pas.
- `sample` lance le fournisseur une fois par graine et par domaine : cent graines prennent des heures ; les tests en
  utilisent une.
- Le parcours de recette est joué à la main avec les outils Playwright, pas par un test.

### Validation et mesures

`crates/chronos-app/tests/v2_s5.rs` sur le vrai worker : jobs stables et partagés, `501` sur `/branch`, IDs et noms
de repli, pagination (pages disjointes, stables, dont la somme égale la vue non paginée), fiche du maire, graphe et
explication, superposition sans position pour l'extérieur, export déterministe, refus d'un export abîmé, import d'un
canon seul et d'un domaine calculé avec 0 exécution du worker, et l'échantillonneur. Parcours de recette :
`docs/reviews/social-v2-acceptance.md`.

## Proposé, non implémenté

S6–S8 conservent leur périmètre dans l'étude : contraintes personnelles
exactes, branches, ponts et optimisations. Les signatures de
fonctions proposées au §14.1 ne sont pas des stubs exécutables dans S0. Sources de
catalogue autres que l'atlas et rendu des arbres de nom appartiennent aux lots suivants.

## Validation et mesures

`tests/v2_s0.rs` contient la fixture pure (seed 42, horizon 1300–1400,
`chronos-social-2`) et les régressions contractuelles. La preuve de sortie S0
compare, sans fournisseur, l'accès direct par cible et la projection paginée d'une
résolution indépendante : un événement-cadre, deux batailles filles sur deux
cellules, une épidémie, des `effect_id` distincts par cible et les `intensity_draw`.
Les tests de normalisation permutent chaque liste triée du plan et de l'atlas, avec
au moins deux éléments. Leur pouvoir d'échec n'a été vérifié qu'en partie : une
campagne de retrait des comportements a été interrompue, et la plupart des tris
de normalisation n'y sont pas passés. Les tests de probabilité vérifient les décisions
par cible, jamais un quota sur leur somme. L'exemple de plan est désérialisé,
validé avec ses références de fixture, puis comparé en canonique. Aucune mesure de
performance ni exécution démographique n'est revendiquée.
