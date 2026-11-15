# Aspen system atlas

A bounded map of the current Aspen runtime, storage, and networking boundaries. It is explanatory and does not replace the governing documents or lifecycle evidence.

<!-- Generated from docs/atlas.ncl by scripts/generate-atlas.py. Do not edit by hand. -->

## Chapters

### Turn and dataspace runtime

Canonical envelopes, pure transitions, effect plans, and replay-visible observations.

Components:
- `runtime` — Runtime kernel: Pure transitions and typed effect plans with explicit replay state. (`src/runtime/`)
- `dataspaces` — Dataspaces: Assertions, retractions, observe patterns, and service dependency facts. (`src/dataspace/`)
- `effects` — Effect boundary: Capability-rooted adapters perform effects; authority and uncertainty stay explicit. (`src/effects/`)

Messages:
- `runtime` → `effects`: typed effect request and terminal observation
- `dataspaces` → `runtime`: assertion or retraction turn plan

Questions:
- **open** Which runtime transitions are replay-visible for the selected profile? ([ref](world-replay-capsules.md))
- **open** How does the selected adapter classify an effect that may have happened? ([ref](fabric-port-ownership.md))

### Storage and world state

Prolly maps, Redb durability, content identity, and world commit or restore boundaries.

Components:
- `prolly` — Prolly map: History-independent ordered state with a bounded incremental edit path and rebuild fallback. (`crates/molten-core/src/prolly_map/`)
- `redb` — Redb stores: Capability-rooted durable logs, snapshots, indexes, and partial-state records. (`src/chunk_store/`)
- `content-store` — Content store adapters: Canonical manifests and verified chunks across local, Redb, Iroh Blobs, and simulation profiles. (`src/content_store_adapter/`)
- `world` — World lifecycle: Commit, branch heads, diff/merge, promotion, and logical restore boundaries. (`src/world_commit/`)

Messages:
- `runtime` → `content-store`: content operation and canonical manifest ref
- `prolly` → `redb`: immutable block publication and generation check
- `world` → `prolly`: active-head and reservation publication

Questions:
- **open** What work does an incremental Prolly edit bound actually avoid? ([ref](prolly-semantic-state-map.md))
- **blocked** Which logical restore profile has an admitted runtime adapter today? ([ref](world-execution-snapshots.md))

### Networking and admission

Iroh transport sessions, peer identity, bounded frames, and the current loopback-only profile.

Components:
- `transport` — Fabric transport: Bounded Iroh sessions, registered protocols, peer evidence, and cleanup. (`src/fabric_transport/`)
- `admission` — Admission: Peer, membership, principal, trust, capability, resource, and policy observations. (`crates/molten-core/src/fabric_transport/`)

Messages:
- `transport` → `admission`: authenticated session and bounded frame observation
- `admission` → `effects`: admitted operation scope, never ambient authority

Questions:
- **answered** Is a per-source rate limit applied before inbound admission? ([ref](inbound-admission-rate-limit-review.md))
- **blocked** Which cross-host transport profile has actual two-host evidence? ([ref](fabric-transport-session-runtime.md))

## Components

| Component | Role | Source | Chapter |
|---|---|---|---|
| `runtime` | Runtime kernel: Pure transitions and typed effect plans with explicit replay state. | `src/runtime/` | Turn and dataspace runtime |
| `dataspaces` | Dataspaces: Assertions, retractions, observe patterns, and service dependency facts. | `src/dataspace/` | Turn and dataspace runtime |
| `effects` | Effect boundary: Capability-rooted adapters perform effects; authority and uncertainty stay explicit. | `src/effects/` | Turn and dataspace runtime |
| `prolly` | Prolly map: History-independent ordered state with a bounded incremental edit path and rebuild fallback. | `crates/molten-core/src/prolly_map/` | Storage and world state |
| `redb` | Redb stores: Capability-rooted durable logs, snapshots, indexes, and partial-state records. | `src/chunk_store/` | Storage and world state |
| `content-store` | Content store adapters: Canonical manifests and verified chunks across local, Redb, Iroh Blobs, and simulation profiles. | `src/content_store_adapter/` | Storage and world state |
| `world` | World lifecycle: Commit, branch heads, diff/merge, promotion, and logical restore boundaries. | `src/world_commit/` | Storage and world state |
| `transport` | Fabric transport: Bounded Iroh sessions, registered protocols, peer evidence, and cleanup. | `src/fabric_transport/` | Networking and admission |
| `admission` | Admission: Peer, membership, principal, trust, capability, resource, and policy observations. | `crates/molten-core/src/fabric_transport/` | Networking and admission |

## Edges

| From | To | Message | Chapter |
|---|---|---|---|
| `runtime` | `effects` | typed effect request and terminal observation | Turn and dataspace runtime |
| `dataspaces` | `runtime` | assertion or retraction turn plan | Turn and dataspace runtime |
| `runtime` | `content-store` | content operation and canonical manifest ref | Storage and world state |
| `prolly` | `redb` | immutable block publication and generation check | Storage and world state |
| `world` | `prolly` | active-head and reservation publication | Storage and world state |
| `transport` | `admission` | authenticated session and bounded frame observation | Networking and admission |
| `admission` | `effects` | admitted operation scope, never ambient authority | Networking and admission |

## Open questions

| Status | Question | Reference |
|---|---|---|
| open | Which runtime transitions are replay-visible for the selected profile? | [`world-replay-capsules.md`](world-replay-capsules.md) |
| open | How does the selected adapter classify an effect that may have happened? | [`fabric-port-ownership.md`](fabric-port-ownership.md) |
| open | What work does an incremental Prolly edit bound actually avoid? | [`prolly-semantic-state-map.md`](prolly-semantic-state-map.md) |
| blocked | Which logical restore profile has an admitted runtime adapter today? | [`world-execution-snapshots.md`](world-execution-snapshots.md) |
| answered | Is a per-source rate limit applied before inbound admission? | [`inbound-admission-rate-limit-review.md`](inbound-admission-rate-limit-review.md) |
| blocked | Which cross-host transport profile has actual two-host evidence? | [`fabric-transport-session-runtime.md`](fabric-transport-session-runtime.md) |
