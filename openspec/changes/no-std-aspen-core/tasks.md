## 0. Traceability scaffolding

- [x] I1 Add requirement/scenario ID lines and seed typed implementation/verification coverage before scenario-to-evidence mapping begins [covers=core.no-std-core-baseline,core.functional-core-imperative-shell,architecture.modularity.acyclic-no-std-core-boundary,architecture.modularity.feature-bundles-are-explicit-and-bounded]
- [x] I2 Establish verification-plan artifacts, evidence contracts, and baseline harness staging for no-std boundary work [covers=core.no-std-core-baseline,architecture.modularity.acyclic-no-std-core-boundary,architecture.modularity.feature-bundles-are-explicit-and-bounded]
- [x] I3 Umbrella tracker for tasks `2.1`-`3.11`: implement alloc-only crate scaffolding, dependency cleanup, shell gating, and purity conversion for `aspen-core` [covers=core.no-std-core-baseline,core.functional-core-imperative-shell,architecture.modularity.acyclic-no-std-core-boundary,architecture.modularity.feature-bundles-are-explicit-and-bounded]
- [x] I4 Umbrella tracker for tasks `4.1`-`4.3`: update representative consumers and docs for the explicit `std` opt-in contract [covers=core.no-std-core-baseline,architecture.modularity.feature-bundles-are-explicit-and-bounded]
- [ ] V1 Verify feature topology and compile-slice evidence stays complete and reviewable [covers=core.no-std-core-baseline,architecture.modularity.feature-bundles-are-explicit-and-bounded] [evidence=openspec/changes/no-std-aspen-core/evidence/verification-compile-slices-plan.md]
- [x] V2 Verify dependency-boundary and surface/source-audit evidence stays complete and reviewable [covers=core.no-std-core-baseline,core.functional-core-imperative-shell,architecture.modularity.acyclic-no-std-core-boundary] [evidence=openspec/changes/no-std-aspen-core/evidence/verification-boundary-plan.md]
- [ ] V3 Verify UI gating, regression, and docs evidence stays complete and reviewable [covers=core.no-std-core-baseline,core.functional-core-imperative-shell,architecture.modularity.feature-bundles-are-explicit-and-bounded] [evidence=openspec/changes/no-std-aspen-core/evidence/verification-regression-plan.md]

Traceability rule: umbrella tasks `I3`, `I4`, `V1`, `V2`, and `V3` stay unchecked until every descendant task in their range is complete, but checked child tasks that explicitly say they inherit umbrella coverage already satisfy incremental scenario linkage while implementation proceeds.

## 1. Baseline and verification harnesses

- [x] 1.0 Add explicit requirement/scenario ID lines to `openspec/changes/no-std-aspen-core/specs/core/spec.md` and `openspec/changes/no-std-aspen-core/specs/architecture-modularity/spec.md`, then align the typed `I*` / `V*` task coverage tags with those IDs.
- [x] 1.1 Create `openspec/changes/no-std-aspen-core/verification.md` with an explicit evidence naming-convention section plus task-coverage and scenario-coverage sections that map each checked `tasks.md` item and every normative scenario from `specs/core/spec.md` and `specs/architecture-modularity/spec.md` to evidence files, and keep it updated as evidence lands rather than only at the end.
- [x] 1.1a Create `openspec/changes/no-std-aspen-core/evidence/verification-compile-slices-plan.md` as the durable plan artifact referenced by `V1`.
- [x] 1.1b Create `openspec/changes/no-std-aspen-core/evidence/verification-boundary-plan.md` as the durable plan artifact referenced by `V2`.
- [x] 1.1c Create `openspec/changes/no-std-aspen-core/evidence/verification-regression-plan.md` as the durable plan artifact referenced by `V3`.
- [x] 1.2 Record the compile-slice artifact contract in `openspec/changes/no-std-aspen-core/verification.md`, covering `core-default-features.txt`, `compile-default.txt`, `compile-no-default.txt`, `compile-sql.txt`, `compile-std.txt`, `compile-std-sql.txt`, `compile-layer.txt`, `compile-global-discovery.txt`, `compile-smoke.txt`, `compile-cluster.txt`, `compile-cli.txt`, and `compile-ui.txt`.
- [x] 1.3 Record the dependency-boundary harnesses and artifact contract in `openspec/changes/no-std-aspen-core/verification.md` for `scripts/check-aspen-core-no-std-boundary.py`, `deps-direct.txt`, `deps-full.txt`, `deps-transitive.json`, `deps-allowlist-diff.txt`, and `deps-transitive-review-<crate>.md`.
- [x] 1.3a Record the purity-disposition artifact contract in `openspec/changes/no-std-aspen-core/verification.md` and `openspec/changes/no-std-aspen-core/evidence/verification-boundary-plan.md` for `openspec/changes/no-std-aspen-core/evidence/purity-disposition.md`.
- [x] 1.4 Stage an initial `scripts/check-aspen-core-no-std-surface.py` capable of producing `surface-inventory.md` before boundary edits, run it once against the pre-refactor tree, and freeze that output as `openspec/changes/no-std-aspen-core/evidence/baseline-surface-inventory.md`.
- [x] 1.5 Record the surface-audit and UI artifact contract in `openspec/changes/no-std-aspen-core/verification.md` for `scripts/check-aspen-core-no-std-surface.py`, `baseline-surface-inventory.md`, `surface-inventory.md`, `export-map.md`, `source-audit.txt`, `ui-fixtures.txt`, and `ui-<fixture>.stderr`.

## 2. Crate scaffolding and dependency cleanup

Traceability: tasks `2.1`-`2.10` decompose umbrella task `I3` and inherit `I3` coverage unless a later subtask narrows the evidence contract further.

- [x] 2.1 Add the downstream smoke consumer crate `crates/aspen-core-no-std-smoke/` as a real alloc-backed `#![no_std]` consumer, make it import/use representative alloc-only `aspen-core` types or traits through the bare/default dependency path, and keep its dependency on `aspen-core` free of feature overrides.
- [x] 2.2 Add alloc/no-std crate scaffolding for `aspen-core` (`no_std` entry, `alloc`, feature map, cfg gates) so the crate can expose the documented alloc-only surface.
- [x] 2.3 Wire and verify the exact Cargo feature topology: `default = []`, `std`, alloc-safe `sql`, `std + sql`, `layer = std + dep:aspen-layer`, and `global-discovery = std + dep:iroh-blobs`, while ensuring bare/default `aspen-core` matches the alloc-only surface.
- [x] 2.4 Audit `crates/aspen-core/Cargo.toml` so alloc-only builds keep only the direct prerequisites from the design with alloc-safe feature settings (`default-features = false` where applicable), while `std`-only dependencies move behind explicit shell opt-ins.
- [x] 2.5 Author and maintain `scripts/aspen-core-no-std-transitives.txt` as the approved alloc-only transitive allowlist consumed by the boundary checker.
- [x] 2.6 Preserve the documented alloc-only root export groups (`cluster`, `constants` scalar/numeric exports, `crypto`, `error`, `hlc`, `kv`, `protocol`, optional alloc-safe `sql`, `storage`, `traits`, `types`, `vault`, and `verified::{build_scan_metadata, decode_continuation_token, encode_continuation_token, execute_scan, filter_scan_entries, normalize_scan_limit, paginate_entries}`) plus the alloc-only module-path families `circuit_breaker`, alloc-safe `prelude`, `spec`, and the exact `verified` module surface: `verified::scan::{build_scan_metadata, decode_continuation_token, encode_continuation_token, execute_scan, filter_scan_entries, normalize_scan_limit, paginate_entries}` together with `verified::{SerializableTimestamp, create_hlc, to_unix_ms, validate_write_command, NodeId, NodeState}`, while gating root module exports/re-exports so alloc-only consumers only see the no-std-safe surface by default.
- [x] 2.7 Implement `scripts/check-aspen-core-no-std-boundary.py` with the design's dependency-boundary checklist:
  - direct-prerequisite equality check
  - transitive allowlist membership check
  - alloc-safe feature-setting check
  - denylist absence check
  - machine-readable JSON output
  - allowlist diff output
  - per-crate review-note schema validation
- [x] 2.8 Extend the staged `scripts/check-aspen-core-no-std-surface.py` so current-tree runs:
  - emit `surface-inventory.md`
  - emit `export-map.md`
  - emit `source-audit.txt`
  - enforce the documented root-path inventory
  - enforce the documented alloc-only module-path families for `circuit_breaker`, alloc-safe `prelude`, `spec`, `verified::scan::{build_scan_metadata, decode_continuation_token, encode_continuation_token, execute_scan, filter_scan_entries, normalize_scan_limit, paginate_entries}`, and `verified::{SerializableTimestamp, create_hlc, to_unix_ms, validate_write_command, NodeId, NodeState}`
  - enforce the documented shell-gating rules
  - enforce the export-map checks required by the design
- [x] 2.9 Implement `scripts/check-aspen-core-feature-claims.py` so default-feature resolution, the declared `aspen-core` feature topology, smoke-consumer manifest shape plus `#![no_std]` source proof, and representative consumer feature proofs become deterministic pass/fail checks; the checker must parse the repo-local `crates/aspen-core/Cargo.toml` manifest and fail unless `default = []`, `layer` includes both `std` and `dep:aspen-layer`, `global-discovery` includes both `std` and `dep:iroh-blobs`, and `sql` remains alloc-safe without requiring `std`; it must also fail unless the smoke manifest uses a bare `aspen-core` dependency, the smoke source proves alloc-backed `#![no_std]`, `cluster-core-features.txt` shows `aspen-core/std`, `cli-core-features.txt` shows `aspen-core/layer`, and it must emit `openspec/changes/no-std-aspen-core/evidence/feature-claims.json`.
- [x] 2.10 Create and maintain `openspec/changes/no-std-aspen-core/evidence/deps-transitive-review-<crate>.md` for every allowlisted transitive crate, using the schema defined in `design.md`.

## 3. Functional-core surface conversion

Traceability: tasks `3.1`-`3.11` continue decomposing umbrella task `I3` and inherit `I3` coverage unless a later subtask narrows the evidence contract further.

- [x] 3.1 Inventory alloc-only APIs for ambient randomness, configuration, environment access, process-global state, I/O, async operations, system calls, hidden runtime context, or implicit randomness sources, and record the per-category disposition in `openspec/changes/no-std-aspen-core/evidence/purity-disposition.md` before refactors begin.
- [x] 3.1a Audit alloc-only APIs that still depend on randomness or configuration and narrow them to concrete targets with explicit value inputs (for this change: re-check `protocol`, alloc-safe `sql`, and `verified::scan` helpers), saving follow-up regression proof under `openspec/changes/no-std-aspen-core/evidence/regression-<topic>.txt` when a target changes.
- [x] 3.1b Audit alloc-only APIs for ambient environment or process-global state and either remove each concrete hit or record its absence in `openspec/changes/no-std-aspen-core/evidence/purity-disposition.md` plus later `source-audit.txt`.
- [x] 3.1c Audit alloc-only production modules for hidden runtime context, I/O, runtime-bound async bodies, or system calls and move each concrete hit behind the `std` shell boundary, with resulting proof saved under `openspec/changes/no-std-aspen-core/evidence/purity-disposition.md`, `source-audit.txt`, or `regression-<topic>.txt`.
- [x] 3.1d Save proof when one of the listed purity categories is absent from the alloc-only surface.
- [x] 3.2 Convert pure-but-std-bound APIs such as circuit-breaker timing and duration convenience layers to explicit primitive or no-std-safe time inputs, and keep the documented duration convenience root exports (`GOSSIP_SUBSCRIBE_TIMEOUT`, `IROH_CONNECT_TIMEOUT`, `IROH_READ_TIMEOUT`, `IROH_STREAM_OPEN_TIMEOUT`, `MEMBERSHIP_OPERATION_TIMEOUT`, `READ_INDEX_TIMEOUT`, `MEMBERSHIP_COOLDOWN`) on their current root paths behind `#[cfg(feature = "std")]`.
- [x] 3.3 Keep the alloc-safe `sql` surface available without `std` and gate `Arc<T>`-style `sql` convenience impls behind the `std` shell path.
- [x] 3.4 Gate `app_registry` and its current root exports on the `std` shell path while preserving the existing public paths under `#[cfg(feature = "std")]`.
- [x] 3.5 Gate `context` / watch-registry implementations and current root exports so `aspen_core::{ContentDiscovery, ContentNodeAddr, ContentProviderInfo}` and any additional `global-discovery` / Iroh-backed context helpers stay on their existing public paths behind both `feature = "std"` and `feature = "global-discovery"`.
- [x] 3.6 Gate `simulation`, `utils`, and their current root exports on the `std` shell path while preserving the existing public paths under `#[cfg(feature = "std")]`.
- [x] 3.7 Gate `transport`, runtime convenience impls, and their current root exports on the `std` shell path while preserving the existing public paths under `#[cfg(feature = "std")]`.
- [x] 3.8 Keep `test_support` crate-private and test-only with no public `aspen_core::test_support` root path, and gate runtime-only prelude additions on the `std` shell path while preserving the documented surface behavior.
- [x] 3.9 Keep `layer` on its documented existing `aspen_core::*` public paths (`AllocationError`, `DirectoryError`, `DirectoryLayer`, `DirectorySubspace`, `Element`, `HighContentionAllocator`, `Subspace`, `SubspaceError`, `Tuple`, `TupleError`) behind both `feature = "std"` and `feature = "layer"` for this change.
- [x] 3.10 Create/update `crates/aspen-core/tests/ui/` `trybuild` fixtures so the fixture set includes: alloc-only fixtures with `default-features = false` for `AppRegistry`, `NetworkTransport`, and `SimulationArtifact`; `std`-enabled but `global-discovery`-disabled fixture(s) for `ContentDiscovery`; and `std`-enabled but `layer`-disabled fixture(s) for `DirectoryLayer`, all on their current `aspen_core::*` paths.
- [x] 3.11 Extend `scripts/check-aspen-core-no-std-surface.py` (or companion rule data) with the alloc-only purity/source-audit checklist so alloc-only modules are mechanically checked for:
  - ambient environment reads
  - process-global state access
  - hidden runtime context access
  - implicit randomness sources
  - I/O
  - runtime-bound async bodies
  - system calls
  while still allowing pure alloc-only contract traits to declare async signatures.

## 4. Downstream migration and docs

Traceability: tasks `4.1`-`4.3` decompose umbrella task `I4` and inherit `I4` coverage.

- [x] 4.1 Update `crates/aspen-cluster` to opt into `aspen-core/std` explicitly.
- [x] 4.2 Update `crates/aspen-cli` to opt into `aspen-core/layer` explicitly (and therefore `std`).
- [x] 4.3 Write `docs/no-std-core.md` documenting the alloc-only build, bare/default feature behavior, alloc-safe `sql`, `std` opt-in path, `layer`, and `global-discovery`, including migration notes for existing public paths that become `std`-gated.

## 5. Verification

Traceability: tasks `5.1`-`5.10` decompose `V1`, tasks `5.11`-`5.15` decompose `V2`, and tasks `5.16`-`5.20` decompose `V3`; each verification task inherits the matching umbrella coverage.

- [x] 5.1 Run `cargo tree -p aspen-core -e features > openspec/changes/no-std-aspen-core/evidence/core-default-features.txt` and assert the default feature set is empty.
- [x] 5.2 Run `cargo check -p aspen-core > openspec/changes/no-std-aspen-core/evidence/compile-default.txt` and assert the crate builds with the empty default feature set.
- [x] 5.3 Run `cargo check -p aspen-core --no-default-features > openspec/changes/no-std-aspen-core/evidence/compile-no-default.txt` and assert the explicit alloc-only crate build succeeds.
- [x] 5.4 Run `cargo check -p aspen-core --no-default-features --features sql > openspec/changes/no-std-aspen-core/evidence/compile-sql.txt` and assert the alloc-safe `sql` surface builds without `std`.
- [x] 5.5 Run `cargo check -p aspen-core --features std > openspec/changes/no-std-aspen-core/evidence/compile-std.txt` and `cargo check -p aspen-core --features std,sql > openspec/changes/no-std-aspen-core/evidence/compile-std-sql.txt`, and assert both shell-enabled slices build.
- [x] 5.6 Run `cargo check -p aspen-core --features layer > openspec/changes/no-std-aspen-core/evidence/compile-layer.txt` and `cargo check -p aspen-core --features global-discovery > openspec/changes/no-std-aspen-core/evidence/compile-global-discovery.txt`, and assert both optional shell feature slices build.
- [x] 5.7 Save `crates/aspen-core-no-std-smoke/Cargo.toml` as `openspec/changes/no-std-aspen-core/evidence/smoke-manifest.txt`, save `crates/aspen-core-no-std-smoke/src/lib.rs` as `openspec/changes/no-std-aspen-core/evidence/smoke-source.txt`, run `cargo check -p aspen-core-no-std-smoke > openspec/changes/no-std-aspen-core/evidence/compile-smoke.txt`, and assert the smoke consumer is a real alloc-backed `#![no_std]` downstream crate that keeps a bare `aspen-core` dependency with no feature overrides while importing alloc-only APIs.
- [ ] 5.8 Run `cargo check -p aspen-cluster > openspec/changes/no-std-aspen-core/evidence/compile-cluster.txt` and `cargo tree -p aspen-cluster -e features -i aspen-core > openspec/changes/no-std-aspen-core/evidence/cluster-core-features.txt`, and assert `aspen-cluster` resolves `aspen-core/std`.
- [ ] 5.9 Run `cargo check -p aspen-cli > openspec/changes/no-std-aspen-core/evidence/compile-cli.txt` and `cargo tree -p aspen-cli -e features -i aspen-core > openspec/changes/no-std-aspen-core/evidence/cli-core-features.txt`, and assert `aspen-cli` resolves `aspen-core/layer` (and therefore `std`).
- [x] 5.10 Run `python scripts/check-aspen-core-feature-claims.py --default-features openspec/changes/no-std-aspen-core/evidence/core-default-features.txt --smoke-manifest openspec/changes/no-std-aspen-core/evidence/smoke-manifest.txt --smoke-source openspec/changes/no-std-aspen-core/evidence/smoke-source.txt --cluster-features openspec/changes/no-std-aspen-core/evidence/cluster-core-features.txt --cli-features openspec/changes/no-std-aspen-core/evidence/cli-core-features.txt --output openspec/changes/no-std-aspen-core/evidence/feature-claims.json`, and assert the checker proves `default = []`, `layer` includes `std` plus `dep:aspen-layer`, `global-discovery` includes `std` plus `dep:iroh-blobs`, `sql` stays alloc-safe without requiring `std`, the smoke consumer is an alloc-backed `#![no_std]` crate that uses a bare `aspen-core` dependency, `aspen-cluster` resolves `aspen-core/std`, and `aspen-cli` resolves `aspen-core/layer`.
- [x] 5.11 Run `cargo tree -p aspen-core --no-default-features -e normal --depth 1 > openspec/changes/no-std-aspen-core/evidence/deps-direct.txt` and assert the direct prerequisite set matches the design exactly.
- [x] 5.12 Run `cargo tree -p aspen-core --no-default-features -e normal > openspec/changes/no-std-aspen-core/evidence/deps-full.txt` and inspect the full alloc-only graph used by the boundary checker.
- [x] 5.13 Run `python scripts/check-aspen-core-no-std-boundary.py --manifest-path crates/aspen-core/Cargo.toml --allowlist scripts/aspen-core-no-std-transitives.txt --output openspec/changes/no-std-aspen-core/evidence/deps-transitive.json --diff-output openspec/changes/no-std-aspen-core/evidence/deps-allowlist-diff.txt`, and assert the checker closes the full dependency-boundary checklist:
  - direct-prerequisite equality
  - transitive allowlist membership
  - constrained feature settings
  - denylist absence
  - presence of per-crate `deps-transitive-review-<crate>.md` notes
- [x] 5.14 Run `python scripts/check-aspen-core-no-std-surface.py --crate-dir crates/aspen-core/src --output-dir openspec/changes/no-std-aspen-core/evidence`, and assert `surface-inventory.md`, `export-map.md`, and the exact root-path inventory from the design/export map match observed alloc-only, `std`, `layer`, and `global-discovery` exports, that the alloc-only module-path families for `aspen_core::circuit_breaker::*`, alloc-safe `aspen_core::prelude::*`, `aspen_core::spec::*`, `aspen_core::verified::scan::{build_scan_metadata, decode_continuation_token, encode_continuation_token, execute_scan, filter_scan_entries, normalize_scan_limit, paginate_entries}`, and `aspen_core::verified::{SerializableTimestamp, create_hlc, to_unix_ms, validate_write_command, NodeId, NodeState}` remain present as documented, and that there is no public `aspen_core::test_support` path.
- [x] 5.15 Use the same source-audit outputs plus `openspec/changes/no-std-aspen-core/evidence/purity-disposition.md` to assert alloc-only exports do not leak back to shell-only modules or crates, `Arc<T>` convenience impls plus runtime-only prelude additions stay `std`-gated, and alloc-only/`verified` modules do not perform I/O, runtime-bound async bodies, system calls, ambient environment reads, process-global state access, hidden runtime context access, or implicit randomness reads, while pure alloc-only contract traits may still declare async signatures.
- [x] 5.16 Run `cargo test -p aspen-core --test ui > openspec/changes/no-std-aspen-core/evidence/compile-ui.txt`, save `openspec/changes/no-std-aspen-core/evidence/ui-fixtures.txt`, capture `openspec/changes/no-std-aspen-core/evidence/ui-<fixture>.stderr`, and assert `ui-fixtures.txt` records the exact fixture path, manifest feature settings, and command for each case while each stderr snapshot fails for the intended missing gate (`std`, `global-discovery`, or `layer`) rather than an unrelated error.
- [x] 5.17 Save `openspec/changes/no-std-aspen-core/evidence/docs-no-std-core-review.md` as the review note for `docs/no-std-core.md`, showing it matches the feature matrix and current public-path contract.
- [x] 5.18 Run targeted regression tests for each refactored pure entrypoint, covering both positive and negative cases, and save outputs under `openspec/changes/no-std-aspen-core/evidence/regression-<topic>.txt`.
- [ ] 5.19 Update `openspec/changes/no-std-aspen-core/verification.md` as each artifact lands, maintaining per-task and per-scenario claim-to-artifact mapping.
- [ ] 5.20 Finalize `openspec/changes/no-std-aspen-core/verification.md`, index `core-default-features.txt`, every compile slice, `smoke-manifest.txt`, `smoke-source.txt`, `cluster-core-features.txt`, `cli-core-features.txt`, `feature-claims.json`, `deps-direct.txt`, `deps-full.txt`, `deps-transitive.json`, `deps-allowlist-diff.txt`, `baseline-surface-inventory.md`, `surface-inventory.md`, `export-map.md`, `source-audit.txt`, `openspec/changes/no-std-aspen-core/evidence/purity-disposition.md`, `ui-fixtures.txt`, `openspec/changes/no-std-aspen-core/evidence/docs-no-std-core-review.md`, per-crate transitive review notes, compile-fail stderr assertions, and regression outputs under `openspec/changes/no-std-aspen-core/evidence/`.
- [x] 5.21 After tasks `2.6`, `2.8`, `3.8`-`3.11`, `4.2`, and `4.3` land, rerun or explicitly revalidate the already-saved compile/dependency rails (`5.1`-`5.7`, `5.11`-`5.13`) so `verification.md` ends with a fresh claim-to-artifact index rather than stale pre-surface-check outputs.
