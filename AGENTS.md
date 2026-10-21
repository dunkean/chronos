# Repository Guidelines

## Project Structure & Module Organization

This repository is currently a design workspace for a deterministic, multiscale procedural history generator. Its only existing project document is `conception_generateur_histoire_multiechelle_v3.md`, a French architecture proposal. No source tree, tests, assets, or build manifests exist yet.

Read sections 14 (Rust architecture), 15 (determinism), 17 (validation), and 18 (implementation phases) before implementing features. The proposed `crates/history-*` layout describes responsibility boundaries, not existing packages. Begin with a small core/world/refine/API implementation; extract social, institutional, storage, and integration modules when justified. Rust is the intended implementation language; WASM bindings are optional.

## Build, Test, and Development Commands

There are currently no executable build, test, or development commands. Do not describe proposed APIs or pseudocode as runnable software.

Once a Cargo workspace exists, run these commands from its root:

- `cargo build --workspace`: compile workspace crates.
- `cargo test --workspace`: run unit and integration tests.
- `cargo fmt --all -- --check`: check Rust formatting.
- `cargo clippy --workspace --all-targets -- -D warnings`: check lint compliance.

Document additional setup and launch commands alongside the implementation that introduces them.

## Coding Style & Naming Conventions

Preserve the architecture document's French prose, numbered sections, anchors, and reference links. Clearly distinguish proposals, implemented behavior, and measured results.

For new Rust code, use rustfmt defaults, four-space indentation, `snake_case` functions/modules, `PascalCase` types, and `SCREAMING_SNAKE_CASE` constants. Keep canonical facts, social constraints, and narrative accounts separate.

Derive random decisions from stable semantic keys and explicit versions. Avoid shared mutable RNG state, `DefaultHasher`, and reliance on `HashMap` iteration order for canonical results.

## Testing Guidelines

No testing framework or coverage threshold is configured. When Rust code is added, use unit tests near implementations and integration tests in each crate's `tests/` directory. Name tests after observable invariants, such as `spatial_query_order_preserves_event_ids`.

Prioritize section 17's invariants: query-order independence, parent/child consistency, exact budgets, unique identity binding, temporal validity, and cache/replay equivalence. Record seeds, versions, and parameters in reproducible regressions. Compare canonical projections using stable serialization.

## Commit & Pull Request Guidelines

No Git metadata or history is available here, so no existing commit convention can be inferred. Use concise, imperative subjects, for example `Define canonical event identity contract`.

Pull requests should explain the problem, affected design sections, validation performed, and unresolved limitations. Link relevant issues when available. Include seeds and replay steps for generation changes; substantiate performance claims with measurements.
