## Context

`aspen-core` is currently both Aspen's contract crate and a grab-bag of runtime helpers. Its `Cargo.toml` pulls in `tokio`, `tokio-util`, `iroh`, `chrono`, `serde_json`, `anyhow`, and other `std`-oriented dependencies. Its public surface re-exports pure types and verified helpers beside `std`-bound pieces such as:

- `app_registry` using `std::sync::RwLock`
- watch-registry implementations using `std` atomics and locks
- `transport` traits and helpers coupled to `iroh`, `tokio_util::sync::CancellationToken`, and `anyhow`
- `simulation` persistence using filesystem, env, and process APIs
- "pure" logic such as `circuit_breaker` and `constants` convenience values that still depend on `std::time`

That makes `aspen-core` expensive for downstream crates that only need types/traits, and it prevents Aspen from pointing to one concrete alloc-safe foundation as its functional core. This change establishes that boundary without forcing the entire workspace to become `no_std` at once.

## Goals / Non-Goals

**Goals**

- Make `aspen-core` build in an alloc-backed `no_std` mode.
- Keep foundational contracts, deterministic helpers, and verified logic available from that alloc-only surface.
- Force runtime shells to become explicit through a non-default `std` feature and companion shell boundaries where needed.
- Replace `std`-only pure APIs with explicit primitive/value inputs.
- Add compile and consumer acceptance rails so the boundary stays enforced.

**Non-Goals**

- Converting every Aspen crate to `no_std`.
- Replacing Aspen's runtime stack or removing async traits from the workspace.
- Guaranteeing that every current `aspen_core::*` export remains at the crate root forever after this first cut.
- Solving all possible shell extraction follow-ups in this one change.

## Decisions

### 1. `std` becomes an explicit, non-default shell opt-in

**Choice:** `aspen-core` will expose an alloc-first baseline and require an explicit `std` opt-in for shell APIs. Feature topology for the first cut is:

- `default = []`
- `std` = enables shell dependencies and root exports that require `std`
- `layer` = `std` + `dep:aspen-layer`
- `global-discovery` = `std` + `dep:iroh-blobs`
- `sql` remains available for alloc-safe SQL types/validation; any `Arc<T>` convenience impls stay under `std`

**Rationale:** the spec says shell access must be explicit. Keeping `std` default-enabled would preserve ambiguity exactly where this change is trying to create a hard boundary.

**Alternative:** keep `std` default-enabled for one migration cycle. Rejected because it weakens the new contract and makes compile-fail verification of the alloc-only surface much less meaningful.

### 2. The boundary is enforced by an explicit module/export map

**Choice:** use this target classification for `crates/aspen-core/src`:

| Target surface | Modules / exports |
| --- | --- |
| alloc-only core | `circuit_breaker` (after primitive-time rewrite), `cluster`, `constants` scalar/numeric exports, `crypto`, `error`, `hlc`, `kv`, alloc-safe `prelude`, `protocol`, `spec`, `storage`, `traits`, `types`, `vault`, `verified`, plus alloc-safe optional `sql` |
| `std` shell | `app_registry`, `context`, `simulation`, `transport`, `utils`, `test_support`, duration convenience exports in `constants`, `Arc<T>` convenience impls, runtime-only prelude additions |
| `std` shell + optional feature | `layer`, `global-discovery` integrations, any Iroh-backed context helpers |

Root export routing for the first cut is explicit and exhaustive at the current `src/lib.rs` surface:

### Alloc-only root exports (remain visible without `std`)

- `circuit_breaker` has module-path-only alloc-safe surface (`aspen_core::circuit_breaker::*`) and no root re-exports to gate
- `cluster::{AddLearnerRequest, ChangeMembershipRequest, ClusterNode, ClusterState, InitRequest, TrustConfig}`
- `constants::{api::*, ci::*, coordination::*, directory::*, raft::*}` except the duration convenience re-exports listed in the `std` section below
- `crypto::Signature`
- `error::{ControlPlaneError, KeyValueStoreError}`
- `hlc::{HLC, HlcTimestamp, HlcId, NTP64, SerializableTimestamp, create_hlc, new_timestamp, to_unix_ms, update_from_timestamp}`
- `kv::{BatchCondition, BatchOperation, CompareOp, CompareTarget, DeleteRequest, DeleteResult, KeyValueWithRevision, ReadConsistency, ReadRequest, ReadResult, ScanRequest, ScanResult, TxnCompare, TxnOp, TxnOpResult, WriteCommand, WriteOp, WriteRequest, WriteResult, validate_write_command}`
- `prelude` has module-path-only alloc-safe surface (`aspen_core::prelude::*`) with runtime-only additions gated behind `std`
- `protocol::{Alarm, Envelope, ProtocolCtx, TestCtx}`
- `sql::{SqlColumnInfo, SqlConsistency, SqlQueryError, SqlQueryExecutor, SqlQueryRequest, SqlQueryResult, SqlValue, effective_sql_limit, effective_sql_timeout_ms, validate_sql_query, validate_sql_request}` behind `feature = "sql"`
- `storage::{KvEntry, SM_KV_TABLE}`
- `traits::{ClusterController, CoordinationBackend, KeyValueStore}`
- `types::{ClusterMetrics, NodeAddress, NodeId, NodeState, SnapshotLogId}`
- `vault::{SYSTEM_PREFIX, VaultError, is_system_key, validate_client_key}`
- `verified::{build_scan_metadata, decode_continuation_token, encode_continuation_token, execute_scan, filter_scan_entries, normalize_scan_limit, paginate_entries}`
- `spec` has module-path-only surface (`aspen_core::spec::*`) and no root re-exports to gate

### `std`-gated root exports (same public paths, but only with `feature = "std"`)

- `app_registry::{AppManifest, AppRegistry, SharedAppRegistry, shared_registry}`
- `context::{AspenDocsTicket, DocsEntry, DocsStatus, DocsSyncProvider, EndpointProvider, InMemoryWatchRegistry, KeyOrigin, NetworkFactory, PeerConnectionState, PeerImporter, PeerInfo, PeerManager, ServiceExecutor, ShardTopology, StateMachineProvider, SubscriptionFilter, SyncStatus, WatchInfo, WatchRegistry}`
- `transport::{BlobAnnouncedCallback, BlobAnnouncedInfo, DiscoveredPeer, DiscoveryHandle, IrohTransportExt, MembershipAddressUpdater, NetworkTransport, PeerDiscoveredCallback, PeerDiscovery, StaleTopologyInfo, TopologyStaleCallback}`
- `simulation::{SimulationArtifact, SimulationArtifactBuilder, SimulationStatus}`
- `utils::ensure_disk_space_available`
- duration convenience re-exports: `GOSSIP_SUBSCRIBE_TIMEOUT`, `IROH_CONNECT_TIMEOUT`, `IROH_READ_TIMEOUT`, `IROH_STREAM_OPEN_TIMEOUT`, `MEMBERSHIP_OPERATION_TIMEOUT`, `READ_INDEX_TIMEOUT`, `MEMBERSHIP_COOLDOWN`
- `test_support` is crate-private (`pub(crate)`) and remains test-only shell infrastructure; there is no public root path for it
- runtime-only prelude additions follow the same rule: no alloc-only prelude item becomes shell-only without `#[cfg(feature = "std")]`

### `global-discovery` root exports (require both `std` and `feature = "global-discovery"`)

- `context::{ContentDiscovery, ContentNodeAddr, ContentProviderInfo}` on their existing root paths `aspen_core::{ContentDiscovery, ContentNodeAddr, ContentProviderInfo}`

### `layer` root exports (require both `std` and `feature = "layer"`)

- `layer::{AllocationError, DirectoryError, DirectoryLayer, DirectorySubspace, Element, HighContentionAllocator, Subspace, SubspaceError, Tuple, TupleError}` on their existing root paths

Compile-fail fixtures and `docs/no-std-core.md` are keyed to these exact paths. No root-path renames happen in this first cut.

**Rationale:** a no-std plan is not reviewable unless every public module and root export has a declared destination.

**Alternative:** classify modules opportunistically during implementation or mix gating with ad hoc path moves. Rejected because tasks, docs, and compile-fail fixtures would stay ambiguous.

### 3. Direct dependencies are classified and feature-constrained before code moves

**Choice:** use this dependency map as the target contract:

| Direct dependency | Target surface | Cargo feature policy |
| --- | --- | --- |
| `aspen-constants` | alloc-only core | workspace/path dependency; disable defaults if present and keep only alloc-safe exports |
| `aspen-hlc` | alloc-only core | workspace/path dependency; disable defaults if present and keep alloc-safe mode only |
| `aspen-cluster-types` | alloc-only core | workspace/path dependency; disable defaults if present |
| `aspen-kv-types` | alloc-only core | workspace/path dependency; disable defaults if present |
| `aspen-traits` | alloc-only core | workspace/path dependency; disable defaults if present |
| `aspen-storage-types` | alloc-only core | workspace/path dependency; disable defaults if present |
| `async-trait` | alloc-only core | proc-macro only; no runtime feature surface required |
| `serde` | alloc-only core | `default-features = false`; enable only derive/alloc-safe features needed by core types |
| `bincode` | alloc-only core | `default-features = false`; enable only alloc/serde-safe features needed by codecs |
| `base64` | alloc-only core | `default-features = false` |
| `hex` | alloc-only core | `default-features = false` |
| `snafu` | alloc-only core | `default-features = false`; enable only alloc-safe error support |
| `thiserror` | alloc-only core | proc-macro only; no runtime feature surface required |
| `aspen-layer`, `aspen-time`, `aspen-disk`, `anyhow`, `n0-future`, `tokio-util`, `tokio`, `iroh`, `iroh-base`, `iroh-blobs`, `rand`, `tracing`, `chrono`, `serde_json` | `std` shell | reachable only through `std`, `layer`, or `global-discovery` paths |

If any alloc-only dependency cannot satisfy this feature policy, it must be reconfigured or moved out of the alloc-only surface before the change is considered complete.

**Rationale:** the dependency boundary is as important as the module boundary, and package-name allowlists are not enough unless feature settings are explicit.

**Alternative:** start moving code first and clean up dependencies later. Rejected because the alloc-only build could appear green while still resolving `std` features through allowed package names.

### 4. Pure logic must stop depending on runtime-owned handles

**Choice:** convert alloc-only pure APIs that currently rely on runtime-owned handles or ambient process state into explicit primitive/value inputs. This includes time, randomness, configuration, environment-derived state, and any process-global or hidden runtime context. Examples include circuit-breaker timing, duration convenience layers, and any alloc-safe helper that currently infers behavior from ambient state instead of explicit inputs.

**Verification:** the implementation must pair targeted regression tests with a source audit proving alloc-only modules do not read ambient environment variables, process IDs, global singletons, hidden runtime context, or implicit randomness sources. If an alloc-only API needs randomness or configuration, the value must arrive as an explicit parameter or typed value object.

**Rationale:** a functional core is not truly portable if its algorithms still require `Instant`, filesystem paths, cancellation tokens, environment reads, process-global state, or other runtime-owned handles.

**Alternative:** leave these APIs `std`-only and treat them as part of the shell. Rejected for modules like `circuit_breaker`, where the behavior itself is pure and belongs in the functional core.

### 5. `layer` and `global-discovery` stay shell-side in this first cut

**Choice:** keep both `layer` and `global-discovery` behind explicit `std`-gated paths for this change instead of forcing them into the alloc-only surface.

**Rationale:** both are already optional and currently tied to runtime facilities. Keeping them shell-side lets the change focus on the foundational contract boundary first.

**Alternative:** force either feature into the alloc-only surface immediately. Rejected because it would enlarge scope and delay the more important alloc-only baseline.

### 6. Verification matrix is concrete and deterministic

**Choice:** the change will be considered complete only if it can produce all of these rails:

| Rail | Purpose |
| --- | --- |
| `cargo tree -p aspen-core -e features > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/core-default-features.txt` | captures default feature resolution for the feature-claims checker |
| `cargo check -p aspen-core > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-default.txt` | proves the crate itself builds with the empty default feature set |
| `cargo check -p aspen-core --no-default-features > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-no-default.txt` | proves the explicit alloc-only crate build works |
| `cargo check -p aspen-core --no-default-features --features sql > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-sql.txt` | proves the documented alloc-safe `sql` surface builds without `std` |
| `cargo check -p aspen-core --features std > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-std.txt` | proves the base shell opt-in path builds |
| `cargo check -p aspen-core --features std,sql > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-std-sql.txt` | proves `sql` stays usable when `std` is enabled for shell conveniences |
| `cargo check -p aspen-core --features layer > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-layer.txt` | proves `layer` remains a `std`-gated shell path |
| `cargo check -p aspen-core --features global-discovery > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-global-discovery.txt` | proves `global-discovery` remains a `std`-gated shell path |
| `cp crates/aspen-core-no-std-smoke/Cargo.toml openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/smoke-manifest.txt` plus `cp crates/aspen-core-no-std-smoke/src/lib.rs openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/smoke-source.txt` plus `cargo check -p aspen-core-no-std-smoke > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-smoke.txt` | captures smoke-consumer manifest, alloc-backed `#![no_std]` source, and build output so the feature-claims checker and review record can prove a real no-std downstream with a bare default dependency that imports alloc-only APIs |
| `cargo tree -p aspen-core --no-default-features -e normal --depth 1` asserted to match exactly the alloc-only direct prerequisite set from Decision 3 | proves the direct prerequisite contract |
| `scripts/check-aspen-core-no-std-boundary.py` consuming `cargo metadata` plus `cargo tree -p aspen-core --no-default-features -e normal` | proves every transitive dependency is reachable only through approved alloc-only prerequisites, uses the required alloc-safe feature settings, and appears in the approved alloc-safe transitive allowlist |
| the same boundary audit asserting exclusion of `aspen-layer`, `aspen-time`, `aspen-disk`, `anyhow`, `n0-future`, `tokio`, `tokio-util`, `iroh`, `iroh-base`, `iroh-blobs`, `rand`, `tracing`, `chrono`, and `serde_json` | proves known shell/runtime crates do not leak into the alloc-only graph |
| saved inventory, export-map, and source-audit artifacts tied to the module/export map in Decision 2 | proves the alloc-only surface has no back-edge to `std` shell exports or modules, and that `Arc<T>` convenience impls plus runtime-only prelude additions remain `std`-gated |
| `cargo check -p aspen-cluster > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-cluster.txt` plus `cargo tree -p aspen-cluster -e features -i aspen-core > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/cluster-core-features.txt` after `crates/aspen-cluster/Cargo.toml` explicitly opts into `aspen-core/std` | captures representative transport-shell consumer outputs for the feature-claims checker |
| `cargo check -p aspen-cli > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-cli.txt` plus `cargo tree -p aspen-cli -e features -i aspen-core > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/cli-core-features.txt` after `crates/aspen-cli/Cargo.toml` explicitly opts into `aspen-core/layer` (and therefore `std`) | captures representative layer-shell consumer outputs for the feature-claims checker |
| `python scripts/check-aspen-core-feature-claims.py --default-features openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/core-default-features.txt --smoke-manifest openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/smoke-manifest.txt --smoke-source openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/smoke-source.txt --cluster-features openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/cluster-core-features.txt --cli-features openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/cli-core-features.txt --output openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/feature-claims.json` | proves `default = []`, the smoke consumer is an alloc-backed `#![no_std]` bare dependency without feature overrides, `aspen-cluster -> aspen-core/std`, and `aspen-cli -> aspen-core/layer` |
| `cargo test -p aspen-core --test ui > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/compile-ui.txt` with `trybuild` fixtures under `crates/aspen-core/tests/ui/` | proves gated APIs stay unavailable under the intended missing-feature conditions |
| `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/ui-fixtures.txt` plus stderr snapshots for those UI tests | proves the exact fixture paths, manifest feature settings, commands, and failure reason for each gate. The fixture index must include at minimum: alloc-only fixtures with `default-features = false` for `AppRegistry`, `NetworkTransport`, and `SimulationArtifact`; `std`-enabled but `global-discovery`-disabled fixture(s) for `ContentDiscovery`; and `std`-enabled but `layer`-disabled fixture(s) for `DirectoryLayer` |
| targeted regression tests for converted pure entrypoints | proves refactored functional-core logic keeps both positive and negative coverage |
| `docs/no-std-core.md` reviewed alongside these rails | proves the explicit opt-in contract is documented durably |

The feature-claims command contract for this change is:

- `python scripts/check-aspen-core-feature-claims.py --default-features openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/core-default-features.txt --smoke-manifest openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/smoke-manifest.txt --smoke-source openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/smoke-source.txt --cluster-features openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/cluster-core-features.txt --cli-features openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/cli-core-features.txt --output openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/feature-claims.json`

That checker must:

1. fail unless the default feature set is empty
2. fail unless `crates/aspen-core-no-std-smoke/Cargo.toml` uses a bare `aspen-core` dependency with no feature overrides and `crates/aspen-core-no-std-smoke/src/lib.rs` proves the crate is alloc-backed `#![no_std]`
3. fail unless `cluster-core-features.txt` shows `aspen-core/std`
4. fail unless `cli-core-features.txt` shows `aspen-core/layer`
5. emit a machine-readable summary at `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/feature-claims.json`

The dependency-audit command contract for this change is:

- `cargo tree -p aspen-core --no-default-features -e normal --depth 1 > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-direct.txt`
- `cargo tree -p aspen-core --no-default-features -e normal > openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-full.txt`
- `python scripts/check-aspen-core-no-std-boundary.py --manifest-path crates/aspen-core/Cargo.toml --allowlist scripts/aspen-core-no-std-transitives.txt --output openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive.json --diff-output openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-allowlist-diff.txt`

The acyclic-boundary/source-audit command contract for this change is:

- `python scripts/check-aspen-core-no-std-surface.py --crate-dir crates/aspen-core/src --output-dir openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence`
- Before boundary edits, run that checker once against the pre-refactor tree and save `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/surface-inventory.md` as `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/baseline-surface-inventory.md`; later runs continue to overwrite `surface-inventory.md`, `export-map.md`, and `source-audit.txt` for the current tree.

That checker must emit:

- `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/surface-inventory.md`
- `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/export-map.md`
- `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/source-audit.txt`

and it must validate that alloc-only modules and exports do not import or depend on shell-only modules within `crates/aspen-core/src`, that shell-only impl blocks (for example `Arc<T>` convenience impls) plus runtime-only prelude additions are guarded by `#[cfg(feature = "std")]`, and that alloc-only modules do not read `std::env`, `std::process`, global singletons, hidden runtime context, implicit randomness sources such as `rand::rng` / `thread_rng`, perform I/O, or contain runtime-bound async bodies / `.await` paths. Alloc-only contract traits may still declare async signatures when they remain pure interface contracts and do not pull runtime behavior into the alloc-only implementation surface.

The transitive dependency audit is not ad hoc. `scripts/check-aspen-core-no-std-boundary.py` is the planned deterministic checker for this change. It must:

1. resolve alloc-only metadata for `crates/aspen-core/Cargo.toml`
2. assert the direct normal dependencies exactly match the allowlist from Decision 3
3. assert every transitive normal dependency is reachable through those allowlisted direct prerequisites
4. assert every resolved package uses the required alloc-safe feature policy where the policy is constrained in Decision 3
5. compare the resolved transitive package set against an explicit allowlist file (planned path: `scripts/aspen-core-no-std-transitives.txt`) whose entries are the approved alloc-safe transitives for this change
6. fail on any shell-only dependency in the denylist above
7. emit a machine-readable summary in `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-transitive.json` plus an allowlist diff in `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/deps-allowlist-diff.txt`

Each entry admitted to `scripts/aspen-core-no-std-transitives.txt` must be justified by a saved alloc-safety review note under `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/` using the pattern `evidence/deps-transitive-review-<crate>.md`, so future verification can fail closed on any newly introduced transitive package.

Each `deps-transitive-review-<crate>.md` note must include a small fixed schema:

- `Crate:` name and version
- `Introduced by:` direct or transitive parent path from the alloc-only graph
- `Resolved features:` the feature set seen in the alloc-only build
- `Filesystem:` yes/no with short justification
- `Process/global state:` yes/no with short justification
- `Thread/async-runtime:` yes/no with short justification
- `Network:` yes/no with short justification
- `Decision:` allow or reject

`scripts/check-aspen-core-no-std-boundary.py` must fail if an allowlisted crate lacks a matching review note or if the note is missing any required schema field.

Saved OpenSpec evidence must capture the exact commands, checker outputs, fixture paths, stderr assertions, and outputs for these rails. Compile-fail command/fixture indexing is saved in `openspec/changes/archive/2026-04-21-no-std-aspen-core/evidence/ui-fixtures.txt`.

**Rationale:** this boundary is easy to erode accidentally. Positive tests alone are not enough.

**Alternative:** rely on code review and occasional manual cargo checks. Rejected because the workspace is too large and the boundary is too cross-cutting.

### 7. `verification.md` is the authoritative claim-to-artifact index

**Choice:** `openspec/changes/archive/2026-04-21-no-std-aspen-core/verification.md` will be maintained as the review index for this change, not just as a late-stage dump.

It must contain:

- a task-coverage section mapping each checked `tasks.md` item to one or more evidence files
- a scenario-coverage section mapping each normative scenario from `specs/core/spec.md` and `specs/architecture-modularity/spec.md` to one or more evidence files
- an evidence naming convention that distinguishes compile slices, dependency audits, export/source audits, compile-fail stderr snapshots, and regression tests

The planned evidence naming scheme is:

- `evidence/core-default-features.txt`
- `evidence/compile-<slice>.txt` (for example `compile-default.txt`, `compile-no-default.txt`, `compile-sql.txt`, `compile-std.txt`, `compile-std-sql.txt`, `compile-layer.txt`, `compile-global-discovery.txt`, `compile-smoke.txt`, `compile-cluster.txt`, `compile-cli.txt`, `compile-ui.txt`)
- `evidence/smoke-manifest.txt`
- `evidence/smoke-source.txt`
- `evidence/cluster-core-features.txt`
- `evidence/cli-core-features.txt`
- `evidence/feature-claims.json`
- `evidence/deps-direct.txt`
- `evidence/deps-full.txt`
- `evidence/deps-transitive.json`
- `evidence/deps-allowlist-diff.txt`
- `evidence/deps-transitive-review-<crate>.md`
- `evidence/baseline-surface-inventory.md`
- `evidence/surface-inventory.md`
- `evidence/export-map.md`
- `evidence/source-audit.txt`
- `evidence/ui-fixtures.txt`
- `evidence/ui-<fixture>.stderr`
- `evidence/regression-<topic>.txt`

`verification.md` must instantiate that naming scheme as a concrete claim-to-artifact plan before task completion. At minimum it must map the current normative scenarios to evidence files like this:

| Spec scenario | Required evidence files in `verification.md` |
| --- | --- |
| `specs/core/spec.md` → `Bare dependency uses alloc-only default` | `evidence/core-default-features.txt`, `evidence/feature-claims.json` |
| `specs/core/spec.md` → `Alloc-only build succeeds` | `evidence/compile-no-default.txt`, `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json` |
| `specs/core/spec.md` → `Bare-default downstream consumer remains supported` | `evidence/smoke-manifest.txt`, `evidence/smoke-source.txt`, `evidence/compile-smoke.txt`, `evidence/feature-claims.json` |
| `specs/core/spec.md` → `Alloc-only build rejects shell imports` | `evidence/compile-ui.txt`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr` |
| `specs/core/spec.md` → `Compile-fail verification is reviewable` | `evidence/compile-ui.txt`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr` |
| `specs/core/spec.md` → `Std-dependent helpers require explicit opt-in` | `evidence/compile-std.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt`, `evidence/export-map.md` |
| `specs/core/spec.md` → `Std-gated shell APIs keep current public paths` | `evidence/export-map.md`, `evidence/ui-fixtures.txt`, `evidence/ui-<fixture>.stderr` |
| `specs/core/spec.md` → `Module-family boundary matches documented inventory` | `evidence/baseline-surface-inventory.md`, `evidence/surface-inventory.md`, `evidence/export-map.md` |
| `specs/core/spec.md` → `Representative std consumers remain supported` | `evidence/compile-cluster.txt`, `evidence/cluster-core-features.txt`, `evidence/compile-cli.txt`, `evidence/cli-core-features.txt` |
| `specs/core/spec.md` → `Compile-slice verification is reviewable` | `evidence/compile-default.txt`, `evidence/compile-no-default.txt`, `evidence/compile-sql.txt`, `evidence/compile-std.txt`, `evidence/compile-std-sql.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt`, `evidence/compile-smoke.txt`, `evidence/compile-cluster.txt`, `evidence/compile-cli.txt` |
| `specs/core/spec.md` → `Verified function purity` | `evidence/source-audit.txt`, `evidence/regression-<topic>.txt` |
| `specs/core/spec.md` → `Shell APIs do not leak into alloc-only core` | `evidence/export-map.md`, `evidence/source-audit.txt` |
| `specs/core/spec.md` → `Pure time-dependent logic uses no-std-safe inputs` | `evidence/source-audit.txt`, `evidence/regression-<topic>.txt` |
| `specs/core/spec.md` → `Pure logic accepts explicit randomness and configuration inputs` | `evidence/source-audit.txt`, `evidence/regression-<topic>.txt` |
| `specs/core/spec.md` → `Refactored pure logic keeps regression coverage` | `evidence/regression-<topic>.txt` |
| `specs/architecture-modularity/spec.md` → `Runtime shells depend outward on core` | `evidence/surface-inventory.md`, `evidence/export-map.md`, `evidence/source-audit.txt` |
| `specs/architecture-modularity/spec.md` → `Acyclic boundary proof is reviewable` | `evidence/surface-inventory.md`, `evidence/export-map.md`, `evidence/source-audit.txt` |
| `specs/architecture-modularity/spec.md` → `Pure consumers avoid runtime shells` | `evidence/compile-no-default.txt`, `evidence/compile-smoke.txt`, `evidence/feature-claims.json` |
| `specs/architecture-modularity/spec.md` → `Alloc-only core excludes runtime shells` | `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json`, `evidence/deps-allowlist-diff.txt`, `evidence/deps-transitive-review-<crate>.md` |
| `specs/architecture-modularity/spec.md` → `Std compatibility is an explicit opt-in` | `evidence/core-default-features.txt`, `evidence/compile-std.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt`, `evidence/compile-sql.txt`, `evidence/compile-std-sql.txt` |
| `specs/architecture-modularity/spec.md` → `Dependency boundary is checked deterministically` | `evidence/deps-direct.txt`, `evidence/deps-full.txt`, `evidence/deps-transitive.json`, `evidence/deps-allowlist-diff.txt`, `evidence/deps-transitive-review-<crate>.md` |
| `specs/architecture-modularity/spec.md` → `Feature-topology verification is reviewable` | `evidence/core-default-features.txt`, `evidence/compile-default.txt`, `evidence/compile-no-default.txt`, `evidence/compile-sql.txt`, `evidence/compile-std.txt`, `evidence/compile-std-sql.txt`, `evidence/compile-layer.txt`, `evidence/compile-global-discovery.txt` |

`verification.md` may add more supporting evidence, but it must not omit any row above when tasks are checked.

**Rationale:** both delta specs require claims to be reviewable via durable artifacts. `verification.md` is how later reviewers find the right evidence without re-deriving the mapping from chat history.

**Alternative:** treat `verification.md` as optional until the end. Rejected because traceability gaps are exactly what the stage gates have been flagging.

## Architecture

### Core surface target

The alloc-only `aspen-core` surface contains:

- foundational request/response types, IDs, state structs, and trait contracts
- deterministic helper logic and `verified/` modules
- numeric/resource-bound constants and no-std-safe conversion helpers
- sans-IO protocol structures that only need `core`/`alloc`
- optional alloc-safe `sql` types and validation helpers

### Shell surface target

The explicit `std` shell surface contains:

- transport/Iroh-specific traits, wrappers, and callback handles
- in-memory registry implementations and other lock/atomic-backed conveniences
- filesystem/process/env-backed simulation artifact persistence
- runtime-only blanket impls, prelude additions, and test support helpers
- `layer` and `global-discovery` paths that stay coupled to runtime facilities in the first cut

### Dependency direction

Shell layers may depend on alloc-only core contracts. The alloc-only core may not depend back on shell crates, shell features, or runtime-only third-party dependencies.

## Risks / Trade-offs

**Wide downstream surface area** -> many Aspen crates import `aspen-core` today, and some implicitly rely on shell conveniences.
**Mitigation:** update representative consumers early and require explicit feature opt-ins before broad rollout.

**Dependency-feature churn** -> some third-party crates currently used in `aspen-core` may need different feature flags or shell relocation before alloc-only builds work cleanly.
**Mitigation:** classify dependencies before code motion and fail the change if the alloc-only dependency graph still resolves forbidden crates or unexpected features.

**Compatibility pain at the crate root** -> downstream imports may break when root re-exports become gated.
**Mitigation:** document the migration path in `docs/no-std-core.md` and keep compile-fail coverage on the exact existing public paths so the boundary change is obvious and reviewable.

**Follow-up shell extraction still likely** -> some `std` features may remain inside `aspen-core` for one transition cycle.
**Mitigation:** keep the spec focused on explicit boundaries now; crate extraction can follow once the alloc-only contract is stable.

## Migration Plan

1. Inventory modules, root re-exports, and dependencies against the boundary map above.
2. Add non-default `std` feature scaffolding, `aspen-core-no-std-smoke`, and the verification rails listed above.
3. Convert pure-but-std-bound APIs to explicit primitive/value inputs.
4. Gate shell exports on their exact existing public paths, then update `aspen-cluster` and `aspen-cli` to opt into them explicitly.
5. Write `docs/no-std-core.md` and save durable positive/negative verification artifacts before closing tasks.

Rollback for an implementation slice is straightforward: revert the last shell-gating step and restore the prior explicit feature wiring while keeping the saved inventory/evidence.
