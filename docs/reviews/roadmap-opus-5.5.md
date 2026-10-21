# Roadmap review — 2026-10-21

Reviewer: Claude Opus 5.5. Scope: `Roadmap.md` only. AGENTS.md, the design study and sibling projects were read, not modified. No code, git, network or servers.

## Sources checked

- Neural Earth: `native/neural-earth/src/atlas.rs`, `lib.rs` (`set_world`), `wasm/runtime-worker.mjs`.
- Magna Urbis: `settlements/mod.rs` (`Raster`, `Terrain::new`, `validate`), `roads/mod.rs`, `crates/core/Cargo.toml`.
- Pecking Order: `models.py` (`Role`, `Rule`), `docs/integration.fr.md`.
- Namebench: `namegen/db.py` (`ORDER BY RANDOM()`, `all_names_pool`, `all_places_pool`).
- Lineage Lab: `docs/genealogy-hybrid.md`, `config.py` (`Event`, `start_year`, anchors).
- Design study §12 and the section headings.

## Corrections made

1. **Sequencing.** The first integrated demo was at J6, behind lazy refinement and hierarchical cross-allocation (old J4) and conditional genealogy (old J5). It now ships at **J4**: J0 virtual skeleton → J1 Neural Earth + Magna Urbis → J2 Namebench/Pecking Order/Lineage Lab → J3 history loop and pivot demo → J4 workshop. The small world is generated entirely at the root, so query-order independence holds by construction, and exact quotas use `top-K` over the complete candidate index. Refinement (J5), conditional genealogy (J6), constrained present (J7), scale and hosts (J8) and extensions (J9) are deferred, not removed.
2. **Added a NOW/LATER scope table (§6).** WASM verification is now non-blocking until J8. There is no DSL, plugin system or validation platform.
3. **Data authority (§3.1).** Each kind of fact now has one owner. Lineage Lab owns lives and kinship. Chronos owns aggregates, events and office tenures. Pecking Order owns templates, identified by `kb_digest`. Namebench owns name strings. Rules:
   - Chronos crises never kill imported people.
   - A recorded death of an office holder opens a Chronos vacancy.
   - Only a closed list of rule methods is interpreted; any other rule produces a diagnostic.
   - Differences between Lineage Lab totals and Chronos counts are reported, not reconciled.
4. **Units and numbers.**
   - Neural Earth takes a *diameter* (`C = π·d`). The roadmap said "rayon en mètres". It now stores the source value and derives the radius.
   - The atlas is 1024×512 equirectangular, about 39 km per cell at the equator for an Earth-sized planet. The 7,680 m conditioning step only interpolates. Cell areas are weighted by cos(lat), and atlas metre coordinates are not distances away from the equator.
   - Magna Urbis limits are stated: 16–1024 cells, 250–100,000 m, ≤1000 settlements per call, `u32` population ≤5M per specified settlement. Chronos counts are `u64`, and out-of-range values are rejected, not truncated.
   - "Exactement 30 %" now has an integer rounding rule (`floor(P·3/10)`) applied to the population at the start of the block.
   - Lineage Lab event years are inclusive; Chronos ranges are half-open. The conversion and the calendar offset are now explicit.
   - Travel-time rounding is defined.
   - The pivot demo now has a concrete time window: 1300, about 200 years, with Lineage Lab covering roughly 1300–1420.
5. **Coarse map vs regional routing (§3.3).** One Magna Urbis region is about 2.5 atlas cells, and the atlas has no rivers or lakes. The raster mapping is now stated (water from the sign of elevation, `river_width=0`, `river_ids=-1`) and labelled *approximé*. Chronos owns a coarse least-cost corridor graph that gives costs and times but no road geometry. Corridors meet Magna Urbis roads through stable gateway `RoadSite`s instead of the automatic border `exits`. This also resolves the contradiction with old J9 ("no second routing pipeline"): Chronos keeps only its coarse cost graph.
6. **Truthful availability.** Every source shows a status: importée, approximée or repli virtuel. Chronos-made fantasy profiles are labelled as not coming from Pecking Order. Lineage Lab people are named by Namebench, keyed on their external ID, because no personal-name field was found.
7. **Fixed missing commands.** `cargo clippy` is now in the per-slice checks, matching AGENTS.md.

## Outstanding risks

Note ajoutée pendant l’intégration : Pecking Order a ensuite migré son code Python vers une API Rust 2.0. La relecture principale a actualisé Roadmap.md et ses liens vers `src/model.rs`, `src/engine.rs` et `data/systems.json`. Le rapport initial est conservé comme trace de la revue demandée.

- **Neural Earth export path not verified.** The atlas inputs come from the browser's Orogen worker. Whether the Python backend or a headless run can produce `r_xyz` and the fields offline is unconfirmed; this blocks J1's real-map status.
- **Cross-workspace dependency.** `magna-urbis-core` takes `edition`, `license` and `publish` from its own workspace and needs `magna-urbis-random`. A Cargo `path` dependency from Chronos should work, but it has not been compiled.
- **Gateway sites are untested.** Whether Magna Urbis routes cleanly to low-population gateway sites, and whether `exits = 0` is accepted, is unverified.
- **Magna Urbis determinism was not checked.** It is only assumed, so the roadmap adds a same-seed regression test and freezes its outputs.
- **Terrain quality.** With interpolated relief and no rivers, Magna Urbis placement in the demo will look plausible but is weakly grounded. A smaller planet helps.
- **Lineage Lab mapping.** Matching its settlement indices to Chronos sites is manual or rank-based. Its own crises can contradict Chronos events, which will show up as diagnostics.
- **Pecking Order coverage.** It is unknown which catalogue `method` identifiers map to "simple hereditary succession" or "simple vote". The closed method list has to be built from the frozen catalogue.
- **Pecking Order wording.** Its integration doc says computed heirs stay in Lineage Lab, but Lineage Lab has no office succession. The roadmap records this; the sibling doc is unchanged.
- **Namebench pools** are real-world, country-based name pools. Using them for fantasy cultures needs an explicit culture→pool mapping.
