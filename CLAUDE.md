# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Chronos is a local prototype of a deterministic chronology generator for a small world: sites, routes, polities, organizations, named people and dated events with effects, explored in a web workshop with a map, a timeline and an inspector. User-facing docs, UI strings, warnings and source details are in French. Keep that language when editing them, and preserve the numbered sections and anchors of `conception_generateur_histoire_multiechelle_v3.md`.

Before changing the engine, read sections 14 (Rust architecture), 15 (determinism), 17 (validation) and 18 (implementation phases) of the design study. In docs, clearly separate what is proposed, what is implemented and what was measured. Keep canonical facts, social constraints and narrative accounts separate.

## Resuming current work

The prototype above is the J0–J4 baseline. The active work is the redesign around recursive social entities, multi-scale refinement, world resources and Azgaar-derived content. Nothing of it is implemented in the engine yet.

- **Start with `docs/suivi-implementation.md`.** §0 is a resume prompt, §2 a reading order, §3 the ecosystem split, §7.0 the battle order (next steps), §7.1–7.3 task status (P0–P5, R*, A0–A5, AZ*), §8 open decisions and §9 the session log. Update §7 and §9 at the end of every session.
- **Design:** `docs/reviews/vision-entites-sociales.md` (§7 macro design, §7.7 what is kept from Azgaar) and the v3 study (§4A, §4.5, §10, §14–18).
- **Azgaar:** `docs/integration-azgaar.md` is the reference (content packs, integration paths, invariants, lots A0–A5). `docs/reviews/azgaar-fmg-inspiration.md` covers the generation rules (AZ tasks). The upstream clone is `../Fantasy-Map-Generator`, pinned at `67d555b`, code in `src/model/<domain>/`. Reimplement from rules; never add an FMG runtime dependency.
- **Generic model first** (vision §7.8). Do not add typed fields such as "state religion", "culture type" or "government form". Model generic ideas (belief, language, custom, knowledge, style), integer adherence distributions per cohort, dated endorsements (`ENDORSES`, `ADMINISTERS`, `SUPPRESSES`, `PATRONS`) and Pecking Order power structures. The familiar notions are pure, versioned classifiers over that state. Azgaar supplies content and heuristics, not structure. Religion is a strong driver through weights, not a special code path.
- **Ecosystem split** (decided, do not reopen):
  - Neural Earth (`../infinite_map`) is the only multi-scale system, from planet down to cm.
  - Magna Urbis (`../city_generator`) builds cities on Neural Earth 40 km tiles and will be merged into Neural Earth.
  - Chronos is the world-scale time dimension: it reads regional aggregates and owns the historical delta.
- **Resources work** lives outside this repo. It was merged into Neural Earth `main` (`de2fec6`, `docs/RESOURCE_LAYERS.md`) and Magna Urbis `main` (`bb7dcdf`) on 27/10/2026. The worktrees `../neural-earth-resources` and `../magna-urbis-resources` (branch `resources-layers`) remain for follow-up work. Never touch the main checkouts of `../infinite_map` or `../city_generator`.
- **Coding is delegated to Codex CLI** (`codex exec -m gpt-6.1-sol -c model_reasoning_effort=high`), not to Claude subagents. The tracking doc §0 and §9 describe how.

## Commands

Run these from the repository root. The workspace contains only `crates/chronos-core` and `crates/chronos-app`.

```powershell
cargo build --workspace
cargo test --workspace
cargo test -p chronos-core --test invariants pivot_war_destroys_exactly_three_villages_after_travel_time   # single test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p chronos-core --target wasm32-unknown-unknown   # core must stay free of I/O and clock

.\demo.ps1                                                     # release build + browser at http://127.0.0.1:8785/
cargo run --release -p chronos-app -- serve --port 8785
cargo run --release -p chronos-app -- generate [--config config.json] --out world.json
cargo run --release -p chronos-app -- check world.json         # validate + compare to replay
```

The server binds only to `127.0.0.1`. It finds `web/` and `fixtures/` from the current directory, or by walking up from the executable. Override with `--root`, `--fixture` or `--web`.

Rebuilding the fixture is a separate data-preparation step. It needs Node, Python and the sibling repos `../infinite_map`, `../city_generator`, `../namebench`, `../pecking-order` and `../lineage-lab`. Chronos never runs it at runtime.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/prepare.ps1
python tools/assemble_fixture.py --check   # fixture == reassembly of output/
python tools/validate_fixture.py           # fixture contract checks
```

The crates under `tools/` declare their own `[workspace]`, are excluded from the main workspace, and build into `output/target-tools`.

## Architecture

**`chronos-core`** (`serde`/`serde_json` only, no I/O). `lib.rs::generate_world(cfg, fixture)` is a pure function that runs this pipeline:
1. Geography (`world.rs`): `stack_geography` uses the fixture's map, sites and routes. `virtual_geography` is the synthetic fallback. A missing or invalid fixture in `stack` mode falls back to the virtual world, adds a warning and records an `unavailable` source.
2. Names (`names.rs`): each site uses its culture's pool. Namebench pools come from the fixture, and built-in lists fill any gaps and are labelled `synthetic`.
3. Institutions (`social.rs::select_profiles`): Pecking Order blueprints are matched to profile kinds. Generic built-in profiles are the fallback.
4. People (`social.rs::import_people`): Lineage Lab records, then `build_society`, which assigns polities, organizations and office holders.
5. History (`generate.rs::run_history`): one annual loop over the whole world, in a fixed order each year: snapshot (state at the start of the year), successions, wars, epidemics, harvests, trade, migrations, reconstructions, unrest, discoveries, then natural growth (no event).
6. Assembly: the `sources` list with statuses and fingerprints, then `canon_id`.

`model.rs` holds every serialized type (`Config`, `World`, `StackFixture`, …) and the constants `SCHEMA_VERSION`, `ALGORITHM_VERSION` and `MAX_SITE_YEARS`. `validate.rs::validate_world` does structural validation for import and tests.

**`chronos-app`**: `main.rs` contains the CLI (`serve` / `generate` / `check`) and the fixture loading. `server.rs` is a std-only HTTP server for `/api/health`, `/api/defaults`, `/api/world`, `POST /api/generate` and `POST /api/import`, and it serves the static files in `web/`. An import is rejected if its structure is invalid, if its `canon_id` does not match its config and sources, or if its facts differ from a replay.

**`web/`**: plain ES modules with no build step and no external resources. It only reads the `World` returned by the API and never draws randomness or re-implements rules. DOM content is set via `textContent`. Form bounds in `config-form.js` mirror `Config::validate`, so change both together. Export downloads the server's exact JSON text.

**`fixtures/stack.json`** (schema 1) is a frozen bootstrap assembled from five sibling projects, with SHA-256 fingerprints. Its shape is defined in `model.rs::StackFixture`, checked by `tools/validate_fixture.py` and documented in `docs/data-sources.md`. Keep all three in sync.

## Invariants that span files

- **Determinism**: every random decision is `keys::key/unit/range_i(seed, semantic parts)`. Do not use shared RNG state, `HashMap` or `DefaultHasher`; use `BTreeMap`/`BTreeSet`. `generate` must produce byte-identical output for identical inputs. `generation_ms` is non-canonical, so it is excluded from `replay_hash` and written as `0` by the CLI.
- **Canon identity**: `canon_id` = FNV-1a of `ALGORITHM_VERSION` + the *applied* config + every source's `id|status|fingerprint`. Any change to generation semantics changes replays. `docs/methods/chronology.md` documents the algorithm for the current `ALGORITHM_VERSION`; update it alongside behavioral changes.
- **Truthful provenance**: each source has the status `imported`, `derived`, `synthetic` or `unavailable`. A fallback must never be presented as imported data, and the UI displays these statuses.
- **Exact effects**: losses are `floor(P × num / den)` on the population at the start of the year, capped by the people present. Recorded effects equal the applied change. Transfers are one event with two zero-sum effects. Multi-year wars and epidemics are frame events without effects, and the dated battles or strikes cite them in `causes`. Causes always precede the event.
- **Imported people** (Lineage Lab): life dates and parentage are never changed. Chronos never causes their death, and they hold no role outside the genealogy coverage window. Virtual people (`source: "virtual"`) are created only when no imported person is eligible.
- **Collective offices** are exported with `collective_office: true` and `holder_id: null`. This represents an abstract collective body, not a vacancy, so no individual succession is generated. Succession runs only from a closed list of rule identifiers. Any other rule leaves the office vacant and adds a diagnostic.
- **Scenarios**: `pivot` (the default) imposes the 1354–1360 epidemic and the 1398 war/hero events. `free` uses rates only. Tests in `crates/chronos-core/tests/invariants.rs` pin these exact outcomes.
- **Budget**: `years × sites ≤ MAX_SITE_YEARS` (150 000).

## Tests, commits and PRs

- Put unit tests next to the code and integration tests in each crate's `tests/`. Name tests after the invariant they check, e.g. `spatial_query_order_preserves_event_ids`. Prioritize the section 17 invariants: query-order independence, parent/child consistency, exact budgets, unique identity binding, temporal validity, cache/replay equivalence.
- Regression tests record their seed, versions and parameters, and compare canonical projections through stable serialization.
- Commit subjects are short and imperative, e.g. `Define canonical event identity contract`.
- A PR explains the problem, the affected design sections, the validation done and the remaining limits. Generation changes include seeds and replay steps. Performance claims need measurements.

## Context docs

- `docs/suivi-implementation.md`: **read first when resuming** (see "Resuming current work").
- `docs/integration-azgaar.md` and `docs/reviews/azgaar-fmg-inspiration.md`: Azgaar reuse (content, rules, lots A0–A5, AZ tasks).
- `docs/reviews/vision-entites-sociales.md`: vision, gap analysis, feasibility and macro design.
- `docs/methods/chronology.md`: the implemented method and its known limits.
- `docs/data-sources.md`: the fixture's provenance and structure.
- `docs/interface.md`: the UI.
- `Roadmap.md`: milestones J0–J8. The current state is the local slice of J0–J4, and lazy refinement (J5) does not exist yet.
- `docs/reviews/`: integration review, including the reference generation `canon-3fa7f6a56d5ce200` (seed 42, 1300–1500, `pivot`).
- `.agent-work/implementation-contract.md` (gitignored, may be absent): the original JSON/API contract.
