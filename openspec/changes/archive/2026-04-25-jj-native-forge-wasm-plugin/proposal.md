## Why

Aspen Forge currently reaches Jujutsu users through Git compatibility paths such as `git-remote-aspen`. That path forces Jujutsu state through Git SHA-1 objects and Git ref semantics, which drops native JJ concepts like stable change IDs, bookmark tracking, and conflict-preserving object forms. A JJ-native Forge plugin would let Aspen speak Jujutsu directly over iroh QUIC while keeping Forge’s BLAKE3-addressed object store and WASM plugin isolation model.

## What Changes

- Add a JJ-native Forge WASM plugin that owns JJ-specific repository operations, object decoding, bookmark updates, and change-id indexes.
- Add an Aspen-specific JJ wire protocol carried by a standalone `jj-remote-aspen` helper over iroh QUIC so clone, fetch, push, bookmark sync, and change-id resolution do not depend on `git-remote-aspen` or SHA-1 translation.
- Store JJ objects and indexes in Forge using BLAKE3-addressed blobs plus Raft-backed metadata under a JJ namespace.
- Let Forge repositories advertise whether they support Git, JJ, or both so Git and JJ clients can coexist without ref collisions.
- Extend plugin/runtime routing so a WASM plugin can claim JJ protocol operations and the corresponding QUIC protocol endpoint with bounded streaming behavior.
- Keep Git support intact; this change is additive and does not replace Git bridge workflows.

## Non-Goals

- Replacing `git-remote-aspen`, Git bridge, or Git-backed Forge repositories.
- Modeling JJ working-copy state or the full JJ operation log on the server in the first cut.
- Automatic migration of all existing Git repos into JJ-enabled repos.
- Adding raw TCP or HTTP endpoints for JJ traffic.
- Solving cross-platform packaging/distribution details beyond defining the standalone `jj-remote-aspen` helper contract.

## Verification

- Integration tests for JJ-native clone, incremental fetch, incremental push, bookmark movement, and change-id resolution.
- Negative tests proving Git access to JJ-only repos and JJ access to Git-only repos fail with capability errors and do not fall back silently.
- Plugin/runtime tests covering protocol-identifier registration, collision rejection, protocol-session routing, and session resource-limit enforcement.
- Authorization tests proving JJ-native reads respect repo read access and JJ-native writes/bookmark updates are rejected without repo write permission.
- Rejection tests proving malformed or inconsistent JJ payloads fail before publish and do not leak partial JJ-visible state.
- Repository-deletion tests proving JJ bookmark namespaces, change-id indexes, and discovery metadata are tombstoned or removed while shared blobs stay on the normal retention/GC path.
- Transport-version tests proving compatible JJ transport versions proceed and incompatible versions are rejected before JJ object exchange begins.
- Dual-backend tests proving Git refs and JJ namespaces remain isolated in the same repo record.

## Capabilities

### New Capabilities

- `jj-native-forge`: First-class Jujutsu repository storage, change-id indexing, bookmark updates, and object sync inside Aspen Forge.
- `jj-remote-aspen`: Native Aspen transport for JJ clients to clone, fetch, push, and synchronize bookmarks over iroh QUIC.

### Modified Capabilities

- `forge`: Forge repositories can expose JJ-native access alongside Git access, with backend metadata and namespaced refs/bookmarks.
- `plugins`: WASM plugins can advertise and serve bounded streaming repo protocols, not only unary request handlers.

## Impact

- **New plugin artifact**: JJ-native Forge WASM plugin crate/artifact, likely loaded beside the existing Forge plugin.
- **Client/API surface**: new JJ-focused request/response types plus a standalone `jj-remote-aspen` helper that speaks the Aspen-specific JJ wire protocol.
- **Forge storage**: JJ object schemas, change-id index keys, bookmark namespace, repo backend metadata.
- **Plugin/runtime plumbing**: protocol advertisement, ALPN routing, bounded stream handling, plugin permissions/config for JJ state.
- **Compatibility**: Git bridge remains supported; mixed Git/JJ repos require explicit namespace separation and compatibility tests.
