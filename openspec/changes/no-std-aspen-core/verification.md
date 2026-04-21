# Verification Evidence

This file is the claim-to-artifact index for `no-std-aspen-core`.
Durable evidence lives under `openspec/changes/no-std-aspen-core/evidence/`.
The checked items below cover traceability scaffolding, evidence-plan setup, the frozen pre-refactor surface baseline, the first alloc-only core scaffolding slice, the follow-on feature-topology plus direct-dependency cleanup slice, and the first deterministic boundary-checker tooling slice.

## Implementation Evidence

- Changed file: `docs/no-std-core.md`
- Changed file: `openspec/changes/no-std-aspen-core/tasks.md`
- Changed file: `openspec/changes/no-std-aspen-core/verification.md`
- Changed file: `openspec/changes/no-std-aspen-core/evidence/docs-no-std-core-review.md`

## Evidence Naming Convention

- compile slices: `evidence/compile-<slice>.txt`
- default feature proof: `evidence/core-default-features.txt`
- smoke consumer proof: `evidence/smoke-manifest.txt`, `evidence/smoke-source.txt`, `evidence/compile-smoke.txt`
- representative consumer feature proofs: `evidence/cluster-core-features.txt`, `evidence/cli-core-features.txt`, `evidence/feature-claims.json`
- dependency audits: `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json`, `evidence/deps-allowlist-diff.txt`, `evidence/deps-transitive-review-<crate>.md`
- purity disposition record: `evidence/purity-disposition.md`
- boundary inventories and source audits: `evidence/baseline-surface-inventory.md`, `evidence/surface-inventory.md`, `evidence/export-map.md`, `evidence/source-audit.txt`
- compile-fail artifacts: `evidence/compile-ui.txt`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr`
- regression artifacts: `evidence/regression-<topic>.txt`
- docs review note: `evidence/docs-no-std-core-review.md`
- typed verification-plan stubs: `evidence/verification-compile-slices-plan.md`, `evidence/verification-boundary-plan.md`, `evidence/verification-regression-plan.md`

## Task Coverage

- [x] I1 Add requirement/scenario ID lines and seed typed implementation/verification coverage before scenario-to-evidence mapping begins [covers=core.no-std-core-baseline,core.functional-core-imperative-shell,architecture.modularity.acyclic-no-std-core-boundary,architecture.modularity.feature-bundles-are-explicit-and-bounded]
  - Evidence: `openspec/changes/no-std-aspen-core/specs/core/spec.md`, `openspec/changes/no-std-aspen-core/specs/architecture-modularity/spec.md`, `openspec/changes/no-std-aspen-core/tasks.md`
- [x] I2 Establish verification-plan artifacts, evidence contracts, and baseline harness staging for no-std boundary work [covers=core.no-std-core-baseline,architecture.modularity.acyclic-no-std-core-boundary,architecture.modularity.feature-bundles-are-explicit-and-bounded]
  - Evidence: `openspec/changes/no-std-aspen-core/verification.md`, `openspec/changes/no-std-aspen-core/evidence/verification-compile-slices-plan.md`, `openspec/changes/no-std-aspen-core/evidence/verification-boundary-plan.md`, `openspec/changes/no-std-aspen-core/evidence/verification-regression-plan.md`, `scripts/check-aspen-core-no-std-surface.py`, `openspec/changes/no-std-aspen-core/evidence/baseline-surface-run.txt`, `openspec/changes/no-std-aspen-core/evidence/baseline-surface-inventory.md`
- [x] 1.0 Add explicit requirement/scenario ID lines to `openspec/changes/no-std-aspen-core/specs/core/spec.md` and `openspec/changes/no-std-aspen-core/specs/architecture-modularity/spec.md`, then align the typed `I*` / `V*` task coverage tags with those IDs.
  - Evidence: `openspec/changes/no-std-aspen-core/specs/core/spec.md`, `openspec/changes/no-std-aspen-core/specs/architecture-modularity/spec.md`, `openspec/changes/no-std-aspen-core/tasks.md`
- [x] 1.1 Create `openspec/changes/no-std-aspen-core/verification.md` with an explicit evidence naming-convention section plus task-coverage and scenario-coverage sections that map each checked `tasks.md` item and every normative scenario from `specs/core/spec.md` and `specs/architecture-modularity/spec.md` to evidence files, and keep it updated as evidence lands rather than only at the end.
  - Evidence: `openspec/changes/no-std-aspen-core/verification.md`
- [x] 1.1a Create `openspec/changes/no-std-aspen-core/evidence/verification-compile-slices-plan.md` as the durable plan artifact referenced by `V1`.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/verification-compile-slices-plan.md`
- [x] 1.1b Create `openspec/changes/no-std-aspen-core/evidence/verification-boundary-plan.md` as the durable plan artifact referenced by `V2`.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/verification-boundary-plan.md`
- [x] 1.1c Create `openspec/changes/no-std-aspen-core/evidence/verification-regression-plan.md` as the durable plan artifact referenced by `V3`.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/verification-regression-plan.md`
- [x] 1.2 Record the compile-slice artifact contract in `openspec/changes/no-std-aspen-core/verification.md`, covering `core-default-features.txt`, `compile-default.txt`, `compile-no-default.txt`, `compile-sql.txt`, `compile-std.txt`, `compile-std-sql.txt`, `compile-layer.txt`, `compile-global-discovery.txt`, `compile-smoke.txt`, `compile-cluster.txt`, `compile-cli.txt`, and `compile-ui.txt`.
  - Evidence: `openspec/changes/no-std-aspen-core/verification.md`, `openspec/changes/no-std-aspen-core/evidence/verification-compile-slices-plan.md`
- [x] 1.3 Record the dependency-boundary harnesses and artifact contract in `openspec/changes/no-std-aspen-core/verification.md` for `scripts/check-aspen-core-no-std-boundary.py`, `deps-direct.txt`, `deps-full.txt`, `deps-transitive.json`, `deps-allowlist-diff.txt`, and `deps-transitive-review-<crate>.md`.
  - Evidence: `openspec/changes/no-std-aspen-core/verification.md`, `openspec/changes/no-std-aspen-core/evidence/verification-boundary-plan.md`
- [x] 1.3a Record the purity-disposition artifact contract in `openspec/changes/no-std-aspen-core/verification.md` and `openspec/changes/no-std-aspen-core/evidence/verification-boundary-plan.md` for `openspec/changes/no-std-aspen-core/evidence/purity-disposition.md`.
  - Evidence: `openspec/changes/no-std-aspen-core/verification.md`, `openspec/changes/no-std-aspen-core/evidence/verification-boundary-plan.md`
- [x] 1.4 Stage an initial `scripts/check-aspen-core-no-std-surface.py` capable of producing `surface-inventory.md` before boundary edits, run it once against the pre-refactor tree, and freeze that output as `openspec/changes/no-std-aspen-core/evidence/baseline-surface-inventory.md`.
  - Evidence: `scripts/check-aspen-core-no-std-surface.py`, `openspec/changes/no-std-aspen-core/evidence/baseline-surface-run.txt`, `openspec/changes/no-std-aspen-core/evidence/surface-inventory.md`, `openspec/changes/no-std-aspen-core/evidence/baseline-surface-inventory.md`
- [x] 1.5 Record the surface-audit and UI artifact contract in `openspec/changes/no-std-aspen-core/verification.md` for `scripts/check-aspen-core-no-std-surface.py`, `baseline-surface-inventory.md`, `surface-inventory.md`, `export-map.md`, `source-audit.txt`, `ui-fixtures.txt`, and `ui-<fixture>.stderr`.
  - Evidence: `openspec/changes/no-std-aspen-core/verification.md`, `openspec/changes/no-std-aspen-core/evidence/verification-boundary-plan.md`, `scripts/check-aspen-core-no-std-surface.py`
- [x] 2.1 Add the downstream smoke consumer crate `crates/aspen-core-no-std-smoke/` as a real alloc-backed `#![no_std]` consumer, make it import/use representative alloc-only `aspen-core` types or traits through the bare/default dependency path, and keep its dependency on `aspen-core` free of feature overrides.
  - Evidence: `crates/aspen-core-no-std-smoke/Cargo.toml`, `crates/aspen-core-no-std-smoke/src/lib.rs`, `openspec/changes/no-std-aspen-core/evidence/smoke-manifest.txt`, `openspec/changes/no-std-aspen-core/evidence/smoke-source.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-smoke.txt`
- [x] 2.2 Add alloc/no-std crate scaffolding for `aspen-core` (`no_std` entry, `alloc`, feature map, cfg gates) so the crate can expose the documented alloc-only surface.
  - Evidence: `crates/aspen-core/Cargo.toml`, `crates/aspen-core/src/lib.rs`, `crates/aspen-core/src/crypto.rs`, `crates/aspen-core/src/protocol.rs`, `crates/aspen-core/src/spec/verus_shim.rs`, `crates/aspen-core/src/sql.rs`, `crates/aspen-core/src/verified/scan.rs`, `openspec/changes/no-std-aspen-core/evidence/compile-default.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-no-default.txt`
- [x] 2.3 Wire and verify the exact Cargo feature topology: `default = []`, `std`, alloc-safe `sql`, `std + sql`, `layer = std + dep:aspen-layer`, and `global-discovery = std + dep:iroh-blobs`, while ensuring bare/default `aspen-core` matches the alloc-only surface.
  - Evidence: `crates/aspen-core/Cargo.toml`, `openspec/changes/no-std-aspen-core/evidence/core-default-features.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-default.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-no-default.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-sql.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-std.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-std-sql.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-layer.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-global-discovery.txt`
- [x] 2.4 Audit `crates/aspen-core/Cargo.toml` so alloc-only builds keep only the direct prerequisites from the design with alloc-safe feature settings (`default-features = false` where applicable), while `std`-only dependencies move behind explicit shell opt-ins.
  - Evidence: `crates/aspen-core/Cargo.toml`, `crates/aspen-cluster-types/Cargo.toml`, `crates/aspen-cluster-types/src/lib.rs`, `crates/aspen-hlc/Cargo.toml`, `crates/aspen-hlc/src/lib.rs`, `crates/aspen-kv-types/Cargo.toml`, `crates/aspen-storage-types/Cargo.toml`, `crates/aspen-traits/Cargo.toml`, `openspec/changes/no-std-aspen-core/evidence/deps-direct.txt`, `openspec/changes/no-std-aspen-core/evidence/core-default-features.txt`
- [x] 2.5 Author and maintain `scripts/aspen-core-no-std-transitives.txt` as the approved alloc-only transitive allowlist consumed by the boundary checker.
  - Evidence: `scripts/aspen-core-no-std-transitives.txt`, `openspec/changes/no-std-aspen-core/evidence/deps-allowlist-diff.txt`
- [x] 2.7 Implement `scripts/check-aspen-core-no-std-boundary.py` with the design's dependency-boundary checklist:
  - Evidence: `scripts/check-aspen-core-no-std-boundary.py`, `openspec/changes/no-std-aspen-core/evidence/deps-transitive.json`, `openspec/changes/no-std-aspen-core/evidence/deps-allowlist-diff.txt`
- [x] 2.9 Implement `scripts/check-aspen-core-feature-claims.py` so default-feature resolution, the declared `aspen-core` feature topology, smoke-consumer manifest shape plus `#![no_std]` source proof, and representative consumer feature proofs become deterministic pass/fail checks; the checker must parse the repo-local `crates/aspen-core/Cargo.toml` manifest and fail unless `default = []`, `layer` includes both `std` and `dep:aspen-layer`, `global-discovery` includes both `std` and `dep:iroh-blobs`, and `sql` remains alloc-safe without requiring `std`; it must also fail unless the smoke manifest uses a bare `aspen-core` dependency, the smoke source proves alloc-backed `#![no_std]`, `cluster-core-features.txt` shows `aspen-core/std`, `cli-core-features.txt` shows `aspen-core/layer`, and it must emit `openspec/changes/no-std-aspen-core/evidence/feature-claims.json`.
  - Evidence: `scripts/check-aspen-core-feature-claims.py`, `openspec/changes/no-std-aspen-core/evidence/feature-claims.json`
- [x] 2.10 Create and maintain `openspec/changes/no-std-aspen-core/evidence/deps-transitive-review-<crate>.md` for every allowlisted transitive crate, using the schema defined in `design.md`.
  - Evidence: `scripts/aspen-core-no-std-transitives.txt`, `openspec/changes/no-std-aspen-core/evidence/deps-transitive.json`
- [x] 3.1 Inventory alloc-only APIs for ambient randomness, configuration, environment access, process-global state, I/O, async operations, system calls, hidden runtime context, or implicit randomness sources, and record the per-category disposition in `openspec/changes/no-std-aspen-core/evidence/purity-disposition.md` before refactors begin.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/purity-disposition.md`
- [x] 3.1d Save proof when one of the listed purity categories is absent from the alloc-only surface.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/purity-disposition.md`
- [x] 3.2 Convert pure-but-std-bound APIs such as circuit-breaker timing and duration convenience layers to explicit primitive or no-std-safe time inputs, and keep the documented duration convenience root exports (`GOSSIP_SUBSCRIBE_TIMEOUT`, `IROH_CONNECT_TIMEOUT`, `IROH_READ_TIMEOUT`, `IROH_STREAM_OPEN_TIMEOUT`, `MEMBERSHIP_OPERATION_TIMEOUT`, `READ_INDEX_TIMEOUT`, `MEMBERSHIP_COOLDOWN`) on their current root paths behind `#[cfg(feature = "std")]`.
  - Evidence: `crates/aspen-core/src/circuit_breaker.rs`, `crates/aspen-core/src/constants/mod.rs`, `crates/aspen-core/src/lib.rs`, `openspec/changes/no-std-aspen-core/evidence/compile-default.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-no-default.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-std.txt`
- [x] 3.3 Keep the alloc-safe `sql` surface available without `std` and gate `Arc<T>`-style `sql` convenience impls behind the `std` shell path.
  - Evidence: `crates/aspen-core/src/sql.rs`, `openspec/changes/no-std-aspen-core/evidence/compile-sql.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-std-sql.txt`
- [x] 3.4 Gate `app_registry` and its current root exports on the `std` shell path while preserving the existing public paths under `#[cfg(feature = "std")]`.
  - Evidence: `crates/aspen-core/src/lib.rs`, `openspec/changes/no-std-aspen-core/evidence/compile-std.txt`
- [x] 3.5 Gate `context` / watch-registry implementations and current root exports so `aspen_core::{ContentDiscovery, ContentNodeAddr, ContentProviderInfo}` and any additional `global-discovery` / Iroh-backed context helpers stay on their existing public paths behind both `feature = "std"` and `feature = "global-discovery"`.
  - Evidence: `crates/aspen-core/src/lib.rs`, `openspec/changes/no-std-aspen-core/evidence/compile-std.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-global-discovery.txt`
- [x] 3.6 Gate `simulation`, `utils`, and their current root exports on the `std` shell path while preserving the existing public paths under `#[cfg(feature = "std")]`.
  - Evidence: `crates/aspen-core/src/lib.rs`, `openspec/changes/no-std-aspen-core/evidence/compile-std.txt`
- [x] 3.7 Gate `transport`, runtime convenience impls, and their current root exports on the `std` shell path while preserving the existing public paths under `#[cfg(feature = "std")]`.
  - Evidence: `crates/aspen-core/src/lib.rs`, `openspec/changes/no-std-aspen-core/evidence/compile-std.txt`
- [x] 4.1 Update `crates/aspen-cluster` to opt into `aspen-core/std` explicitly.
  - Evidence: `crates/aspen-cluster/Cargo.toml`, `openspec/changes/no-std-aspen-core/evidence/cluster-core-features.txt`
- [x] 4.3 Write `docs/no-std-core.md` documenting the alloc-only build, bare/default feature behavior, alloc-safe `sql`, `std` opt-in path, `layer`, and `global-discovery`, including migration notes for existing public paths that become `std`-gated.
  - Evidence: `docs/no-std-core.md`
- [x] 5.1 Run `cargo tree -p aspen-core -e features > openspec/changes/no-std-aspen-core/evidence/core-default-features.txt` and assert the default feature set is empty.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/core-default-features.txt`
- [x] 5.2 Run `cargo check -p aspen-core > openspec/changes/no-std-aspen-core/evidence/compile-default.txt` and assert the crate builds with the empty default feature set.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/compile-default.txt`
- [x] 5.3 Run `cargo check -p aspen-core --no-default-features > openspec/changes/no-std-aspen-core/evidence/compile-no-default.txt` and assert the explicit alloc-only crate build succeeds.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/compile-no-default.txt`
- [x] 5.4 Run `cargo check -p aspen-core --no-default-features --features sql > openspec/changes/no-std-aspen-core/evidence/compile-sql.txt` and assert the alloc-safe `sql` surface builds without `std`.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/compile-sql.txt`
- [x] 5.5 Run `cargo check -p aspen-core --features std > openspec/changes/no-std-aspen-core/evidence/compile-std.txt` and `cargo check -p aspen-core --features std,sql > openspec/changes/no-std-aspen-core/evidence/compile-std-sql.txt`, and assert both shell-enabled slices build.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/compile-std.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-std-sql.txt`
- [x] 5.6 Run `cargo check -p aspen-core --features layer > openspec/changes/no-std-aspen-core/evidence/compile-layer.txt` and `cargo check -p aspen-core --features global-discovery > openspec/changes/no-std-aspen-core/evidence/compile-global-discovery.txt`, and assert both optional shell feature slices build.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/compile-layer.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-global-discovery.txt`
- [x] 5.7 Save `crates/aspen-core-no-std-smoke/Cargo.toml` as `openspec/changes/no-std-aspen-core/evidence/smoke-manifest.txt`, save `crates/aspen-core-no-std-smoke/src/lib.rs` as `openspec/changes/no-std-aspen-core/evidence/smoke-source.txt`, run `cargo check -p aspen-core-no-std-smoke > openspec/changes/no-std-aspen-core/evidence/compile-smoke.txt`, and assert the smoke consumer is a real alloc-backed `#![no_std]` downstream crate that keeps a bare `aspen-core` dependency with no feature overrides while importing alloc-only APIs.
  - Evidence: `crates/aspen-core-no-std-smoke/Cargo.toml`, `crates/aspen-core-no-std-smoke/src/lib.rs`, `openspec/changes/no-std-aspen-core/evidence/smoke-manifest.txt`, `openspec/changes/no-std-aspen-core/evidence/smoke-source.txt`, `openspec/changes/no-std-aspen-core/evidence/compile-smoke.txt`
- [x] 5.11 Run `cargo tree -p aspen-core --no-default-features -e normal --depth 1 > openspec/changes/no-std-aspen-core/evidence/deps-direct.txt` and assert the direct prerequisite set matches the design exactly.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/deps-direct.txt`
- [x] 5.12 Run `cargo tree -p aspen-core --no-default-features -e normal > openspec/changes/no-std-aspen-core/evidence/deps-full.txt` and inspect the full alloc-only graph used by the boundary checker.
  - Evidence: `openspec/changes/no-std-aspen-core/evidence/deps-full.txt`
- [x] 5.13 Run `python scripts/check-aspen-core-no-std-boundary.py --manifest-path crates/aspen-core/Cargo.toml --allowlist scripts/aspen-core-no-std-transitives.txt --output openspec/changes/no-std-aspen-core/evidence/deps-transitive.json --diff-output openspec/changes/no-std-aspen-core/evidence/deps-allowlist-diff.txt`, and assert the checker closes the full dependency-boundary checklist:
  - Evidence: `scripts/check-aspen-core-no-std-boundary.py`, `scripts/aspen-core-no-std-transitives.txt`, `openspec/changes/no-std-aspen-core/evidence/deps-transitive.json`, `openspec/changes/no-std-aspen-core/evidence/deps-allowlist-diff.txt`
- [x] 5.17 Save `openspec/changes/no-std-aspen-core/evidence/docs-no-std-core-review.md` as the review note for `docs/no-std-core.md`, showing it matches the feature matrix and current public-path contract.
  - Evidence: `docs/no-std-core.md`, `openspec/changes/no-std-aspen-core/evidence/docs-no-std-core-review.md`

## Scenario Coverage

| Scenario ID | Planned evidence |
| --- | --- |
| `core.no-std-core-baseline.bare-dependency-uses-alloc-only-default` | `evidence/core-default-features.txt`, `evidence/feature-claims.json` |
| `core.no-std-core-baseline.alloc-only-build-succeeds` | `evidence/compile-no-default.txt`, `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json` |
| `core.no-std-core-baseline.bare-default-downstream-consumer-remains-supported` | `evidence/smoke-manifest.txt`, `evidence/smoke-source.txt`, `evidence/compile-smoke.txt`, `evidence/feature-claims.json` |
| `core.no-std-core-baseline.alloc-only-build-rejects-shell-imports` | `evidence/compile-ui.txt`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr` |
| `core.no-std-core-baseline.compile-fail-verification-is-reviewable` | `evidence/compile-ui.txt`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr` |
| `core.no-std-core-baseline.std-dependent-helpers-require-explicit-opt-in` | `evidence/compile-std.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt`, `evidence/export-map.md` |
| `core.no-std-core-baseline.std-gated-shell-apis-keep-current-public-paths` | `evidence/export-map.md`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr` |
| `core.no-std-core-baseline.module-family-boundary-matches-documented-inventory` | `evidence/baseline-surface-inventory.md`, `evidence/surface-inventory.md`, `evidence/export-map.md` |
| `core.no-std-core-baseline.representative-std-consumers-remain-supported` | `evidence/compile-cluster.txt`, `evidence/cluster-core-features.txt`, `evidence/compile-cli.txt`, `evidence/cli-core-features.txt` |
| `core.no-std-core-baseline.compile-slice-verification-is-reviewable` | `evidence/compile-default.txt`, `evidence/compile-no-default.txt`, `evidence/compile-sql.txt`, `evidence/compile-std.txt`, `evidence/compile-std-sql.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt`, `evidence/compile-smoke.txt`, `evidence/compile-cluster.txt`, `evidence/compile-cli.txt` |
| `core.functional-core-imperative-shell.verified-function-purity` | `evidence/source-audit.txt`, `evidence/regression-<topic>.txt` |
| `core.functional-core-imperative-shell.shell-apis-do-not-leak-into-alloc-only-core` | `evidence/export-map.md`, `evidence/source-audit.txt` |
| `core.functional-core-imperative-shell.pure-time-dependent-logic-uses-no-std-safe-inputs` | `evidence/source-audit.txt`, `evidence/regression-<topic>.txt` |
| `core.functional-core-imperative-shell.pure-logic-accepts-explicit-randomness-and-configuration-inputs` | `evidence/source-audit.txt`, `evidence/regression-<topic>.txt` |
| `core.functional-core-imperative-shell.refactored-pure-logic-keeps-regression-coverage` | `evidence/regression-<topic>.txt` |
| `architecture.modularity.acyclic-no-std-core-boundary.runtime-shells-depend-outward-on-core` | `evidence/surface-inventory.md`, `evidence/export-map.md`, `evidence/source-audit.txt` |
| `architecture.modularity.acyclic-no-std-core-boundary.acyclic-boundary-proof-is-reviewable` | `evidence/surface-inventory.md`, `evidence/export-map.md`, `evidence/source-audit.txt` |
| `architecture.modularity.acyclic-no-std-core-boundary.pure-consumers-avoid-runtime-shells` | `evidence/compile-no-default.txt`, `evidence/compile-smoke.txt`, `evidence/feature-claims.json` |
| `architecture.modularity.feature-bundles-are-explicit-and-bounded.alloc-only-core-excludes-runtime-shells` | `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json`, `evidence/deps-allowlist-diff.txt`, `evidence/deps-transitive-review-<crate>.md` |
| `architecture.modularity.feature-bundles-are-explicit-and-bounded.std-compatibility-is-an-explicit-opt-in` | `evidence/core-default-features.txt`, `evidence/compile-std.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt`, `evidence/compile-sql.txt`, `evidence/compile-std-sql.txt` |
| `architecture.modularity.feature-bundles-are-explicit-and-bounded.dependency-boundary-is-checked-deterministically` | `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json`, `evidence/deps-allowlist-diff.txt`, `evidence/deps-transitive-review-<crate>.md` |
| `architecture.modularity.feature-bundles-are-explicit-and-bounded.feature-topology-verification-is-reviewable` | `evidence/core-default-features.txt`, `evidence/compile-default.txt`, `evidence/compile-no-default.txt`, `evidence/compile-sql.txt`, `evidence/compile-std.txt`, `evidence/compile-std-sql.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt` |

## Review Scope Snapshot

No implementation diff artifact yet.
Current implementation slices add alloc-backed no-std scaffolding in `aspen-core`, a real smoke consumer, explicit shell gating for current root exports, alloc-safe manifest settings across direct prerequisite crates, a feature-gated alloc-safe `NodeAddress` bridge, a first purity-disposition inventory, deterministic boundary/feature-claims checker scripts, transitive review notes, and refreshed compile-slice plus dependency evidence. A vendored `uhlc` patch now removes the alloc-only `rand` leak, so the boundary checker passes end to end.

## Verification Commands

### `python scripts/check-aspen-core-no-std-surface.py --crate-dir crates/aspen-core/src --output-dir openspec/changes/no-std-aspen-core/evidence`

- Status: pass
- Artifact: `openspec/changes/no-std-aspen-core/evidence/baseline-surface-run.txt`

### Compile-slice commands

- `cargo tree -p aspen-core -e features > openspec/changes/no-std-aspen-core/evidence/core-default-features.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/core-default-features.txt`
- `cargo check -p aspen-core > openspec/changes/no-std-aspen-core/evidence/compile-default.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/compile-default.txt`
- `cargo check -p aspen-core --no-default-features > openspec/changes/no-std-aspen-core/evidence/compile-no-default.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/compile-no-default.txt`
- `cargo check -p aspen-core --no-default-features --features sql > openspec/changes/no-std-aspen-core/evidence/compile-sql.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/compile-sql.txt`
- `cargo check -p aspen-core --features std > openspec/changes/no-std-aspen-core/evidence/compile-std.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/compile-std.txt`
- `cargo check -p aspen-core --features std,sql > openspec/changes/no-std-aspen-core/evidence/compile-std-sql.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/compile-std-sql.txt`
- `cargo check -p aspen-core --features layer > openspec/changes/no-std-aspen-core/evidence/compile-layer.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/compile-layer.txt`
- `cargo check -p aspen-core --features global-discovery > openspec/changes/no-std-aspen-core/evidence/compile-global-discovery.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/compile-global-discovery.txt`
- `cargo check -p aspen-core-no-std-smoke > openspec/changes/no-std-aspen-core/evidence/compile-smoke.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/compile-smoke.txt`
- `cargo tree -p aspen-core --no-default-features -e normal --depth 1 > openspec/changes/no-std-aspen-core/evidence/deps-direct.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/deps-direct.txt`
- `cargo tree -p aspen-cluster -e features -i aspen-core > openspec/changes/no-std-aspen-core/evidence/cluster-core-features.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/cluster-core-features.txt`
- `cargo tree -p aspen-cli -e features -i aspen-core > openspec/changes/no-std-aspen-core/evidence/cli-core-features.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/cli-core-features.txt`
- `cargo tree -p aspen-core --no-default-features -e normal > openspec/changes/no-std-aspen-core/evidence/deps-full.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/deps-full.txt`
- `python scripts/check-aspen-core-no-std-boundary.py --manifest-path crates/aspen-core/Cargo.toml --allowlist scripts/aspen-core-no-std-transitives.txt --output openspec/changes/no-std-aspen-core/evidence/deps-transitive.json --diff-output openspec/changes/no-std-aspen-core/evidence/deps-allowlist-diff.txt`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/deps-transitive.json`, `openspec/changes/no-std-aspen-core/evidence/deps-allowlist-diff.txt`
- `python scripts/check-aspen-core-feature-claims.py --default-features openspec/changes/no-std-aspen-core/evidence/core-default-features.txt --smoke-manifest openspec/changes/no-std-aspen-core/evidence/smoke-manifest.txt --smoke-source openspec/changes/no-std-aspen-core/evidence/smoke-source.txt --cluster-features openspec/changes/no-std-aspen-core/evidence/cluster-core-features.txt --cli-features openspec/changes/no-std-aspen-core/evidence/cli-core-features.txt --output openspec/changes/no-std-aspen-core/evidence/feature-claims.json`
  - Status: pass
  - Artifact: `openspec/changes/no-std-aspen-core/evidence/feature-claims.json`
