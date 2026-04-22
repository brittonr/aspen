# `aspen-core` alloc-only guide

`aspen-core` is now alloc-only.

Goal: keep contracts, pure helpers, request/response types, and verified logic in a crate with no std-enabling feature flags. Runtime helpers now live in separate shell crate: `aspen-core-shell`.

## Package matrix

| Dependency shape | Meaning | What stays available |
| --- | --- | --- |
| `aspen-core` | Bare/default dependency. `default = []`. | Alloc-only root exports and alloc-safe module families. |
| `aspen-core` with `default-features = false` | Explicit alloc-only dependency. | Same surface as bare/default dependency. |
| `aspen-core --features sql` | Alloc-only plus alloc-safe SQL request/trait surface. | `sql::{SqlColumnInfo, SqlConsistency, SqlQueryError, SqlQueryExecutor, SqlQueryRequest, SqlQueryResult, SqlValue, effective_sql_limit, effective_sql_timeout_ms, validate_sql_query, validate_sql_request}` |
| `aspen-core-shell` | Std shell crate. | Runtime helpers, transport traits, watch/context APIs, simulation artifacts, Redb storage helpers, disk/time helpers, and all alloc-core re-exports. |
| `aspen-core-shell --features layer` | Layer shell bundle. | `DirectoryLayer`, `Tuple`, `Subspace`, index helpers, and related errors/types. |
| `aspen-core-shell --features global-discovery` | Global discovery shell bundle. | `ContentDiscovery`, `ContentNodeAddr`, `ContentProviderInfo`. |
| `aspen-core-shell --features sql,layer` | Shell crate plus alloc-safe SQL plus layer helpers. | Combined alloc SQL surface and shell/layer helpers. |

## Alloc-only surface

Bare/default `aspen-core` keeps pure contract surface available:

- root export groups: `cluster`, `constants`, `crypto`, `error`, `hlc`, `kv`, `protocol`, alloc-safe `storage`, `traits`, `types`, `vault`
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
  - `aspen_core::prelude::*`
  - `aspen_core::spec::*`
  - `aspen_core::verified::scan::*`

Representative downstream proof lives in `crates/aspen-core-no-std-smoke/`, which is a real `#![no_std]` crate using bare `aspen-core` dependency with no feature overrides.

## Shell-only paths

These paths moved out of `aspen-core` into `aspen-core-shell`:

- `app_registry::{AppManifest, AppRegistry, SharedAppRegistry, shared_registry}`
- `context::{AspenDocsTicket, DocsEntry, DocsStatus, DocsSyncProvider, EndpointProvider, InMemoryWatchRegistry, KeyOrigin, NetworkFactory, PeerConnectionState, PeerImporter, PeerInfo, PeerManager, ServiceExecutor, ShardTopology, StateMachineProvider, SubscriptionFilter, SyncStatus, WatchInfo, WatchRegistry}`
- `transport::{BlobAnnouncedCallback, BlobAnnouncedInfo, DiscoveredPeer, DiscoveryHandle, IrohTransportExt, MembershipAddressUpdater, NetworkTransport, PeerDiscoveredCallback, PeerDiscovery, StaleTopologyInfo, TopologyStaleCallback}`
- `simulation::{SimulationArtifact, SimulationArtifactBuilder, SimulationStatus}`
- `storage::SM_KV_TABLE` (shell-facing Redb table definition; alloc-only `aspen-core::storage` keeps `KvEntry` only)
- `utils::{check_disk_space, current_time_ms, current_time_secs, ensure_disk_space_available, DiskSpace, DISK_USAGE_THRESHOLD_PERCENT}`
- duration convenience constants:
  - `GOSSIP_SUBSCRIBE_TIMEOUT`
  - `IROH_CONNECT_TIMEOUT`
  - `IROH_READ_TIMEOUT`
  - `IROH_STREAM_OPEN_TIMEOUT`
  - `MEMBERSHIP_OPERATION_TIMEOUT`
  - `READ_INDEX_TIMEOUT`
  - `MEMBERSHIP_COOLDOWN`
- `layer::*` shell bundle behind `aspen-core-shell/layer`
- `ContentDiscovery`, `ContentNodeAddr`, `ContentProviderInfo` behind `aspen-core-shell/global-discovery`

## Migration notes

### If you only need Aspen contracts and pure helpers

Use alloc core directly:

```toml
[dependencies]
aspen-core = { path = "../aspen-core", default-features = false }
```

Right choice for:

- pure data types
- protocol envelopes
- key/value request/response types
- verified helpers
- alloc-only consumers
- no-std smoke crates and portability checks

### If you need runtime helpers

Depend on shell crate explicitly:

```toml
[dependencies]
aspen-core-shell = { path = "../aspen-core-shell" }
```

Examples:

- `AppRegistry`
- `NetworkTransport`
- `SimulationArtifact`
- `storage::SM_KV_TABLE`
- watch/context runtime helpers
- duration convenience constants
- disk/time helpers

### If you want to keep existing `aspen_core::*` import paths in a shell consumer

Alias shell package under existing dependency key:

```toml
[dependencies]
aspen-core = { package = "aspen-core-shell", path = "../aspen-core-shell", features = ["layer"] }
```

That is current workspace pattern for std consumers.

That alias path also preserves `aspen_core::storage::SM_KV_TABLE` for shell users while alloc-only consumers keep `aspen_core::storage::KvEntry` without Redb baggage.

### If you import layer APIs

Use shell crate with `layer`:

```toml
[dependencies]
aspen-core-shell = { path = "../aspen-core-shell", features = ["layer"] }
```

### If you import content discovery APIs

Use shell crate with `global-discovery`:

```toml
[dependencies]
aspen-core-shell = { path = "../aspen-core-shell", features = ["global-discovery"] }
```

## Local verification rails

Useful local checks for this split:

- `cargo check -p aspen-core --no-default-features`
- `cargo check -p aspen-core-no-std-smoke`
- `cargo check -p aspen-core-shell`
- `cargo test -p aspen-core --test ui`
- `cargo check -p aspen-cluster`
- `cargo check -p aspen-cli`

Use those rails when reviewing regressions instead of inferring boundary rules from memory.
