# `aspen-core` alloc-only and `std` feature guide

`aspen-core` now defaults to an alloc-only surface.

Goal: let pure consumers depend on Aspen's contract crate without pulling runtime networking, Tokio, or other shell-heavy dependencies into the bare dependency graph.

## Feature matrix

| Dependency shape | Meaning | What stays available |
| --- | --- | --- |
| `aspen-core` | Bare/default dependency. `default = []`. | Alloc-only root exports and alloc-safe module families. |
| `aspen-core` with `default-features = false` | Explicit alloc-only dependency. | Same surface as the bare/default dependency. |
| `aspen-core --features sql` | Alloc-only plus alloc-safe SQL request/trait surface. | `sql::{SqlColumnInfo, SqlConsistency, SqlQueryError, SqlQueryExecutor, SqlQueryRequest, SqlQueryResult, SqlValue, effective_sql_limit, effective_sql_timeout_ms, validate_sql_query, validate_sql_request}` |
| `aspen-core --features std` | Shell opt-in for runtime helpers. | `app_registry`, `transport`, `simulation`, `utils`, `context` root exports, and duration convenience re-exports. |
| `aspen-core --features layer` | Layer shell path. Implies `std`. | `aspen_core::*` layer public paths such as `DirectoryLayer`, `Tuple`, `Subspace`, and related errors/types. |
| `aspen-core --features global-discovery` | Global discovery shell path. Implies `std`. | `ContentDiscovery`, `ContentNodeAddr`, `ContentProviderInfo` on their existing root paths. |
| `aspen-core --features std,sql` | Runtime helpers plus SQL surface. | Alloc-only SQL APIs plus `std`-only convenience impls. |

## Alloc-only surface

Bare/default `aspen-core` keeps the pure contract surface available:

- root export groups: `cluster`, `constants`, `crypto`, `error`, `hlc`, `kv`, `protocol`, `storage`, `traits`, `types`, `vault`
- optional alloc-safe root SQL exports behind `feature = "sql"`
- root verified scan helpers:
  - `build_scan_metadata`
  - `decode_continuation_token`
  - `encode_continuation_token`
  - `execute_scan`
  - `filter_scan_entries`
  - `normalize_scan_limit`
  - `paginate_entries`
- alloc-only module-path families:
  - `aspen_core::circuit_breaker::*`
  - `aspen_core::prelude::*` (alloc-safe subset only)
  - `aspen_core::spec::*`
  - `aspen_core::verified::scan::*`
  - `aspen_core::verified::{SerializableTimestamp, create_hlc, to_unix_ms, validate_write_command, NodeId, NodeState}`

Representative downstream proof lives in `crates/aspen-core-no-std-smoke/`, which is a real `#![no_std]` crate using a bare `aspen-core` dependency with no feature overrides.

## `std`-gated paths

The following stay on their current public paths, but now require `feature = "std"`:

- `app_registry::{AppManifest, AppRegistry, SharedAppRegistry, shared_registry}`
- `context::{AspenDocsTicket, DocsEntry, DocsStatus, DocsSyncProvider, EndpointProvider, InMemoryWatchRegistry, KeyOrigin, NetworkFactory, PeerConnectionState, PeerImporter, PeerInfo, PeerManager, ServiceExecutor, ShardTopology, StateMachineProvider, SubscriptionFilter, SyncStatus, WatchInfo, WatchRegistry}`
- `transport::{BlobAnnouncedCallback, BlobAnnouncedInfo, DiscoveredPeer, DiscoveryHandle, IrohTransportExt, MembershipAddressUpdater, NetworkTransport, PeerDiscoveredCallback, PeerDiscovery, StaleTopologyInfo, TopologyStaleCallback}`
- `simulation::{SimulationArtifact, SimulationArtifactBuilder, SimulationStatus}`
- `utils::ensure_disk_space_available`
- duration convenience re-exports:
  - `GOSSIP_SUBSCRIBE_TIMEOUT`
  - `IROH_CONNECT_TIMEOUT`
  - `IROH_READ_TIMEOUT`
  - `IROH_STREAM_OPEN_TIMEOUT`
  - `MEMBERSHIP_OPERATION_TIMEOUT`
  - `READ_INDEX_TIMEOUT`
  - `MEMBERSHIP_COOLDOWN`

## `layer` and `global-discovery`

These remain explicit shell-side opt-ins in this first cut.

### `layer`

`layer = ["std", "dep:aspen-layer"]`

This keeps the existing layer API on the same `aspen_core::*` paths while ensuring the layer stack is never part of the bare alloc-only graph.

Representative consumer: `crates/aspen-cli/Cargo.toml` depends on `aspen-core` with `features = ["layer"]`.

### `global-discovery`

`global-discovery = ["std", "dep:iroh-blobs"]`

This keeps Iroh-backed content discovery on the same root paths while preventing discovery/networking crates from leaking into the bare dependency graph.

Representative consumer: `crates/aspen-cluster/Cargo.toml` opts into `aspen-core/std`, and higher layers opt into `global-discovery` explicitly when needed.

## Migration notes

### If you only need Aspen contracts and pure helpers

Use a bare dependency or spell it explicitly:

```toml
[dependencies]
aspen-core = { path = "../aspen-core", default-features = false }
```

This is the right choice for:

- pure data types
- protocol envelopes
- key/value request/response types
- verified helpers
- alloc-only consumers
- no-std smoke crates and similar portability checks

### If you import runtime helpers that disappeared from the bare graph

Opt into `std` explicitly:

```toml
[dependencies]
aspen-core = { path = "../aspen-core", features = ["std"] }
```

Examples:

- `AppRegistry`
- `NetworkTransport`
- `SimulationArtifact`
- content/watch/context runtime helpers
- duration convenience root exports

### If you import layer APIs

Opt into `layer` rather than `std` directly:

```toml
[dependencies]
aspen-core = { path = "../aspen-core", features = ["layer"] }
```

This implies `std` and keeps the layer contract obvious at the call site.

### If you import content discovery APIs

Opt into `global-discovery`:

```toml
[dependencies]
aspen-core = { path = "../aspen-core", features = ["global-discovery"] }
```

This also implies `std`.

## Verification rails

The OpenSpec change `no-std-aspen-core` keeps durable evidence under `openspec/changes/no-std-aspen-core/evidence/`.

Key rails:

- `core-default-features.txt`
- `compile-default.txt`
- `compile-no-default.txt`
- `compile-sql.txt`
- `compile-std.txt`
- `compile-std-sql.txt`
- `compile-layer.txt`
- `compile-global-discovery.txt`
- `compile-smoke.txt`
- `feature-claims.json`
- `deps-direct.txt`
- `deps-full.txt`
- `deps-transitive.json`
- `deps-allowlist-diff.txt`

Use those artifacts when reviewing regressions instead of re-deriving the feature contract from memory.
