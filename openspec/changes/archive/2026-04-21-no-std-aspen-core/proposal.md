## Why

`crates/aspen-core` is supposed to be Aspen's foundational dependency, but today it mixes pure contracts and verified helpers with `std`-bound runtime helpers such as Iroh transport types, Tokio cancellation/state, filesystem-backed simulation artifacts, and lock-based in-memory registries. That blocks a true `no_std` consumer path and keeps Aspen's functional-core / imperative-shell boundary blurrier than it should be.

Making `aspen-core` support an alloc-only `no_std` build is the first concrete step toward a real functional core. It gives Aspen one portable, deterministic contract layer that can be reused by verified logic, WASM/guest code, and future constrained targets while forcing runtime shells to become explicit.

## What Changes

- Make `aspen-core` support a documented alloc-backed `no_std` build for its foundational types, traits, constants, and deterministic helper logic.
- Define the public opt-in contract as `default = []`, `std` for shell APIs, `sql` as an alloc-safe optional feature, `std + sql` for shell conveniences plus SQL, `layer = std + dep:aspen-layer`, and `global-discovery = std + dep:iroh-blobs`.
- Move `std`-dependent APIs and implementations behind that explicit shell path, primarily by `std`-gating existing public paths in this first cut instead of doing a parallel API rename.
- Replace `std`-only "pure" APIs in the core surface with explicit primitive inputs or `std`-gated adapters so alloc-only deterministic logic takes explicit time, randomness, and configuration inputs and no longer depends on ambient environment, process-global state, hidden runtime context, or ambient runtime handles.
- Add compile-slice, dependency-boundary, consumer, and compile-fail verification for the alloc-backed `#![no_std]` smoke consumer `aspen-core-no-std-smoke` and the representative std consumers `aspen-cluster` and `aspen-cli`.

## Non-Goals

- Making the entire Aspen workspace `no_std` in one change.
- Replacing Iroh, Tokio, or other runtime technologies used by Aspen shells.
- Completing every possible shell extraction outside `aspen-core` if `std`-gating existing paths is enough for this first cut.
- Reworking distributed semantics such as Raft, KV, or transport behavior beyond what is needed to establish the new boundary.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `core`: Aspen core contracts gain an alloc-only `no_std` baseline and a stricter functional-core / imperative-shell separation.
- `architecture-modularity`: crate and feature boundaries must keep `std` shell dependencies outside the alloc-only core unless explicitly opted into.

## API Boundary Inventory

### Alloc-only baseline

- Module families: `circuit_breaker`, `cluster`, `constants` scalar/numeric exports, `crypto`, `error`, `hlc`, `kv`, alloc-safe `prelude`, `protocol`, `spec`, `storage`, `traits`, `types`, `vault`, `verified`
- Optional alloc-safe feature surface: `sql` value types and validation helpers
- Smoke consumer target: alloc-backed `#![no_std]` crate `crates/aspen-core-no-std-smoke/`

### `std`-gated in place

- Module families: `app_registry`, `context`, `simulation`, `transport`, `utils`, `test_support`, runtime-only prelude additions, duration convenience exports
- Optional `std`-gated path families: `layer` exports on existing `aspen_core::*` paths, `global-discovery` exports on existing `aspen_core::*` paths
- Feature bundles: `layer`, `global-discovery`
- Representative std consumers: `crates/aspen-cluster`, `crates/aspen-cli`

### Verification evidence contract

- Durable evidence for this change lives under `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/`
- Required evidence families include compile slices, compile-fail artifacts (`ui-<fixture>.stderr` plus saved fixture paths/commands), dependency audits (`deps-direct.txt`, `deps-full.txt`, `deps-transitive.json`), baseline inventory capture (`baseline-surface-inventory.md`), and acyclic-boundary artifacts (`surface-inventory.md`, `export-map.md`, `source-audit.txt`)
- `openspec/changes/archive/2026-04-21-no-std-aspen-core/verification.md` is the claim-to-artifact index and MUST map each feature-topology, representative-consumer, compile-fail, dependency-boundary, acyclic-boundary, and regression claim to specific files under `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/`

## Impact

- **Files**: `crates/aspen-core/`, a new alloc-backed `#![no_std]` smoke consumer crate `crates/aspen-core-no-std-smoke/`, likely new or narrowed shell modules/crates for runtime helpers, downstream `Cargo.toml` files that consume `aspen-core`, OpenSpec specs for `core` and `architecture-modularity`, and `docs/no-std-core.md`
- **APIs**: shell APIs keep their current public paths in this first cut but become `#[cfg(feature = "std")]`; pure contracts remain available from the alloc-only surface
- **Dependencies**: the alloc-only build must keep its direct prerequisite set limited to `aspen-constants`, `aspen-hlc`, `aspen-cluster-types`, `aspen-kv-types`, `aspen-traits`, `aspen-storage-types`, `async-trait`, `serde`, `bincode`, `base64`, `hex`, `snafu`, and `thiserror`; those approved alloc-only prerequisites must use alloc-only feature settings with `default-features = false` wherever a default feature set exists and only the features needed by the alloc-only core surface; shell/runtime crates such as `aspen-layer`, `aspen-time`, `aspen-disk`, `anyhow`, `n0-future`, `tokio`, `tokio-util`, `iroh`, `iroh-base`, `iroh-blobs`, `rand`, `tracing`, `chrono`, and `serde_json` must stay out of the alloc-only graph
- **Testing**: the feature-topology matrix is saved as durable compile-slice evidence under `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/`: `cargo check -p aspen-core > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-default.txt`, `cargo check -p aspen-core --no-default-features > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-no-default.txt`, `cargo check -p aspen-core --no-default-features --features sql > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-sql.txt`, `cargo check -p aspen-core --features std > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-std.txt`, `cargo check -p aspen-core --features std,sql > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-std-sql.txt`, `cargo check -p aspen-core --features layer > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-layer.txt`, `cargo check -p aspen-core --features global-discovery > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-global-discovery.txt`, and `cargo check -p aspen-core-no-std-smoke > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-smoke.txt`. The smoke slice is only valid if `smoke-manifest.txt` and `smoke-source.txt` also prove `crates/aspen-core-no-std-smoke` is an alloc-backed `#![no_std]` downstream crate with a bare `aspen-core` dependency. Representative std-consumer verification is exact and paired: `cargo check -p aspen-cluster > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-cluster.txt` plus `cargo tree -p aspen-cluster -e features -i aspen-core > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/cluster-core-features.txt` proves `aspen-cluster` opts into `aspen-core/std`, and `cargo check -p aspen-cli > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-cli.txt` plus `cargo tree -p aspen-cli -e features -i aspen-core > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/cli-core-features.txt` proves `aspen-cli` opts into `aspen-core/layer` (and therefore `std`). Compile-fail coverage runs `cargo test -p aspen-core --test ui > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-ui.txt` using `trybuild` fixtures under `crates/aspen-core/tests/ui/`; per-case fixture paths, manifest feature settings, and commands are saved in `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/ui-fixtures.txt`; and the fixture set MUST cover at least `aspen_core::{AppRegistry, NetworkTransport, SimulationArtifact}` behind `std`, `aspen_core::ContentDiscovery` behind `global-discovery`, and `aspen_core::DirectoryLayer` behind `layer`. Dependency-boundary verification runs `cargo tree -p aspen-core --no-default-features -e normal --depth 1 > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-direct.txt`, `cargo tree -p aspen-core --no-default-features -e normal > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-full.txt`, and `python scripts/check-aspen-core-no-std-boundary.py --manifest-path crates/aspen-core/Cargo.toml --allowlist scripts/aspen-core-no-std-transitives.txt --output openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive.json`. Acyclic-boundary verification runs `python scripts/check-aspen-core-no-std-surface.py --crate-dir crates/aspen-core/src --output-dir openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence`. Regression tests cover refactored pure logic.
