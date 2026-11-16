# Aspen system atlas

A bounded map of the current Aspen runtime, storage, and networking boundaries. It is explanatory and does not replace the governing documents or lifecycle evidence.

<!-- Generated from docs/atlas.ncl by scripts/generate-atlas.py. Do not edit by hand. -->

## Chapters

### Turn and dataspace runtime

Canonical envelopes, pure transitions, effect plans, and replay-visible observations.

Components:
- `runtime` — Runtime kernel: Pure transitions and typed effect plans with explicit replay state. (`src/runtime/`)
- `dataspaces` — Dataspaces: Assertions, retractions, observe patterns, and service dependency facts. (`src/runtime/dataspace/`)
- `effects` — Effect boundary: Capability-rooted adapters perform effects; authority and uncertainty stay explicit. (`src/effects/`)

Messages:
- `runtime` → `effects`: typed effect request and terminal observation
  - Payload: `EffectRequest / EffectResponse` {request_ref, effect_id, operation, handler_profile, capability_refs, decision, output_ref} in `src/effects/parts/mod/p000/body.rs`
- `dataspaces` → `runtime`: assertion or retraction turn plan
  - Payload: `PendingTurn` {actions, events, actor, cause} in `src/runtime/turn/mod.rs`

Questions:
- **open** Which runtime transitions are replay-visible for the selected profile? ([ref](world-replay-capsules.md))
- **open** How does the selected adapter classify an effect that may have happened? ([ref](fabric-port-ownership.md))

### Storage and world state

Prolly maps, Redb durability, content identity, and world commit or restore boundaries.

Components:
- `prolly` — Prolly map: History-independent ordered state with a bounded incremental edit path and rebuild fallback. (`crates/molten-core/src/prolly_map/`)
- `redb` — Redb stores: Capability-rooted durable logs, snapshots, indexes, and partial-state records. (`src/chunk/store.rs`)
- `content-store` — Content store adapters: Canonical manifests and verified chunks across local, Redb, Iroh, optional object-storage and simulation profiles. (`src/content_store_adapter/`)
- `world` — World lifecycle: Commit, branch heads, diff/merge, promotion, and logical restore boundaries. (`src/worldcommit/`)
- `node-state` — Node state storage: Capability-rooted node namespaces and typed store roots; world commits persist through its storage namespace. (`src/node_state/`)

Messages:
- `runtime` → `content-store`: content operation and canonical manifest ref
  - Payload: `ContentCommand` {operation_ref, adapter_ref, operation, manifest_ref, range, deadline_tick} in `crates/molten-core/src/content_store_adapter/model.rs`
- `prolly` → `redb`: immutable block publication and generation check
  - Payload: `ProllyPublicationReceipt` {map_id, prior_root_ref, next_root_ref, generation, staged_block_refs, status} in `src/prolly_map/records.rs`
- `world` → `node-state`: content-addressed root objects, canonical commit, and capture receipt
  - Payload: `CaptureReceipt` {decision, commit_ref, persisted_roots, revision_fences, publication} in `src/worldcommit/wire.rs`

Questions:
- **open** What work does an incremental Prolly edit bound actually avoid? ([ref](prolly-semantic-state-map.md))
- **blocked** Which logical restore profile has exact current-admission and restart qualification? ([ref](world-execution-snapshots.md))

### Networking and admission

Iroh session admission and bounded loopback or explicit direct-LAN adapters; actual cross-host qualification remains separate.

Components:
- `transport` — Fabric transport: Bounded Iroh sessions, registered protocols, peer evidence, and cleanup. (`src/fabric_transport/`)
- `admission` — Admission: Peer, membership, principal, trust, capability, resource, and policy observations. (`crates/molten-core/src/fabric_transport/`)

Messages:
- `transport` → `admission`: authenticated session and bounded frame observation
  - Payload: `ReceiveObservation` {peer_identity, peer_context_ref, service_id, alpn, generation, role} in `crates/molten-core/src/fabric_transport/receive.rs`
- `admission` → `effects`: admitted operation scope, never ambient authority
  - Payload: `AdmittedOperation` {request, peer, authority_ref} in `src/fabric_transport/cross_process/iroh/direct.rs`

Questions:
- **answered** Is a per-source rate limit applied before inbound admission? ([ref](inbound-admission-rate-limit-review.md); tracked by [issue #7](https://github.com/brittonr/aspen/issues/7), [Cairn change admit-direct-lan-streams](../.cairn/changes/admit-direct-lan-streams/proposal.md))
- **blocked** Which cross-host transport profile has actual two-host evidence? ([ref](fabric-transport-session-runtime.md))

### Evidence and qualification

Implementation coverage, exact execution artifacts and diagnostic reports remain distinct from authority or release admission.

Components:
- `qualification` — Capability qualification: Read-only, exact-build artifact validation; incomplete or simulation evidence cannot qualify live durability or cross-host behavior. (`tools/capability-report.rs`)

Messages:
- `runtime` → `qualification`: canonical profile-scoped execution observations, never ambient authority
  - Payload: `ObservedReadback` {subject, binding_ref, validation_ref, receipt_refs} in `src/capability_qualification/model.rs`

Questions:
- **open** Which capabilities have actual validated execution evidence for this exact build/profile? Use the diagnostic report, not handwritten status badges. ([ref](capability-qualification-report.md))

## Components

| Component | Role | Source | Chapter |
|---|---|---|---|
| `runtime` | Runtime kernel: Pure transitions and typed effect plans with explicit replay state. | `src/runtime/` | Turn and dataspace runtime |
| `dataspaces` | Dataspaces: Assertions, retractions, observe patterns, and service dependency facts. | `src/runtime/dataspace/` | Turn and dataspace runtime |
| `effects` | Effect boundary: Capability-rooted adapters perform effects; authority and uncertainty stay explicit. | `src/effects/` | Turn and dataspace runtime |
| `prolly` | Prolly map: History-independent ordered state with a bounded incremental edit path and rebuild fallback. | `crates/molten-core/src/prolly_map/` | Storage and world state |
| `redb` | Redb stores: Capability-rooted durable logs, snapshots, indexes, and partial-state records. | `src/chunk/store.rs` | Storage and world state |
| `content-store` | Content store adapters: Canonical manifests and verified chunks across local, Redb, Iroh, optional object-storage and simulation profiles. | `src/content_store_adapter/` | Storage and world state |
| `world` | World lifecycle: Commit, branch heads, diff/merge, promotion, and logical restore boundaries. | `src/worldcommit/` | Storage and world state |
| `node-state` | Node state storage: Capability-rooted node namespaces and typed store roots; world commits persist through its storage namespace. | `src/node_state/` | Storage and world state |
| `transport` | Fabric transport: Bounded Iroh sessions, registered protocols, peer evidence, and cleanup. | `src/fabric_transport/` | Networking and admission |
| `admission` | Admission: Peer, membership, principal, trust, capability, resource, and policy observations. | `crates/molten-core/src/fabric_transport/` | Networking and admission |
| `qualification` | Capability qualification: Read-only, exact-build artifact validation; incomplete or simulation evidence cannot qualify live durability or cross-host behavior. | `tools/capability-report.rs` | Evidence and qualification |

## Edges

| From | To | Message | Payload | Chapter |
|---|---|---|---|---|
| `runtime` | `effects` | typed effect request and terminal observation | `EffectRequest / EffectResponse` {request_ref, effect_id, operation, handler_profile, capability_refs, decision, output_ref} in `src/effects/parts/mod/p000/body.rs` | Turn and dataspace runtime |
| `dataspaces` | `runtime` | assertion or retraction turn plan | `PendingTurn` {actions, events, actor, cause} in `src/runtime/turn/mod.rs` | Turn and dataspace runtime |
| `runtime` | `content-store` | content operation and canonical manifest ref | `ContentCommand` {operation_ref, adapter_ref, operation, manifest_ref, range, deadline_tick} in `crates/molten-core/src/content_store_adapter/model.rs` | Storage and world state |
| `prolly` | `redb` | immutable block publication and generation check | `ProllyPublicationReceipt` {map_id, prior_root_ref, next_root_ref, generation, staged_block_refs, status} in `src/prolly_map/records.rs` | Storage and world state |
| `world` | `node-state` | content-addressed root objects, canonical commit, and capture receipt | `CaptureReceipt` {decision, commit_ref, persisted_roots, revision_fences, publication} in `src/worldcommit/wire.rs` | Storage and world state |
| `transport` | `admission` | authenticated session and bounded frame observation | `ReceiveObservation` {peer_identity, peer_context_ref, service_id, alpn, generation, role} in `crates/molten-core/src/fabric_transport/receive.rs` | Networking and admission |
| `admission` | `effects` | admitted operation scope, never ambient authority | `AdmittedOperation` {request, peer, authority_ref} in `src/fabric_transport/cross_process/iroh/direct.rs` | Networking and admission |
| `runtime` | `qualification` | canonical profile-scoped execution observations, never ambient authority | `ObservedReadback` {subject, binding_ref, validation_ref, receipt_refs} in `src/capability_qualification/model.rs` | Evidence and qualification |

## Open questions

| Status | Question | Reference | Tracking |
|---|---|---|---|
| open | Which runtime transitions are replay-visible for the selected profile? | [`world-replay-capsules.md`](world-replay-capsules.md) | — |
| open | How does the selected adapter classify an effect that may have happened? | [`fabric-port-ownership.md`](fabric-port-ownership.md) | — |
| open | What work does an incremental Prolly edit bound actually avoid? | [`prolly-semantic-state-map.md`](prolly-semantic-state-map.md) | — |
| blocked | Which logical restore profile has exact current-admission and restart qualification? | [`world-execution-snapshots.md`](world-execution-snapshots.md) | — |
| answered | Is a per-source rate limit applied before inbound admission? | [`inbound-admission-rate-limit-review.md`](inbound-admission-rate-limit-review.md) | [issue #7](https://github.com/brittonr/aspen/issues/7), [Cairn change admit-direct-lan-streams](../.cairn/changes/admit-direct-lan-streams/proposal.md) |
| blocked | Which cross-host transport profile has actual two-host evidence? | [`fabric-transport-session-runtime.md`](fabric-transport-session-runtime.md) | — |
| open | Which capabilities have actual validated execution evidence for this exact build/profile? Use the diagnostic report, not handwritten status badges. | [`capability-qualification-report.md`](capability-qualification-report.md) | — |
