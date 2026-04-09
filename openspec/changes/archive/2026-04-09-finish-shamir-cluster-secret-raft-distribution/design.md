## Context

`crates/aspen-trust` already provides Shamir splitting, reconstruction, digests, key derivation, and follow-on primitives like reconfiguration scaffolding. The remaining gap is in `crates/aspen-raft/src/node/trust.rs`: `initialize_trust()` generates shares and writes only the leader's share directly into local redb storage. That bypasses the Raft application path and leaves follower share persistence unspecified.

The existing `cluster-secret-lifecycle` main spec already expects share distribution through consensus. This change closes the gap without reworking the cryptography layer.

## Goals / Non-Goals

**Goals:**

- Make trust initialization durable through a committed Raft application request.
- Ensure each node persists only its own share during state-machine apply.
- Keep the implementation behind the existing `trust` feature gate.
- Add tests that fail if trust init falls back to leader-local storage in multi-node setups.

**Non-Goals:**

- Membership-change rotation, encrypted secret chains, or expungement.
- Share collection over Iroh.
- Changes to the Shamir math or HKDF derivation APIs.

## Decisions

### Add a dedicated `AppRequest::TrustInitialize` payload

A committed trust-init request is the smallest change that fits Aspen's current Raft plumbing. It carries the epoch, a per-node share assignment map encoded as serialized share bytes, and the epoch digests.

Alternative considered: piggyback trust data on membership metadata or snapshots. Rejected because cluster init already has a normal application-request path, and snapshots would make the first durable write harder to reason about.

### Apply only the local node's share in the state machine

The state machine must not persist every node's share on every node. `SharedRedbStorage` will track the local Raft node ID and use it during trust-init apply to select the assigned share for this node before writing `trust_shares`.

Alternative considered: store all shares and rely on access control later. Rejected because it defeats the trust boundary; any node with all shares can reconstruct the secret alone.

### Make `initialize_trust()` async and submit through `client_write`

The cluster controller already runs in async context. Converting the trust init hook to async lets the leader submit the trust-init request through the same committed path as other Raft-backed state changes.

Alternative considered: append raw log entries directly through storage helpers. Rejected because `client_write()` already handles leadership checks and keeps the request in the normal consensus pipeline.

## Risks / Trade-offs

- **State-machine local-node awareness** -> `SharedRedbStorage` needs local node identity for apply-time selection. Mitigation: store an optional parsed node ID from the constructor input and fail clearly if trust init runs without it.
- **Wire-format expansion** -> adding a new `AppRequest` variant touches serialization tests. Mitigation: keep the payload simple and add targeted request/dispatch tests.
- **Init path regression** -> changing cluster init from sync local writes to async consensus could surface leadership timing issues. Mitigation: keep the trust-init request small and add a regression test for the leader path.

## Migration Plan

- New clusters use the committed trust-init path immediately.
- Existing clusters that already initialized trust keep their current stored shares; this change only affects cluster initialization.
- If trust init fails after Raft initialization, the operation returns an error and the operator can restart the cluster with a clean data dir as today.

## Open Questions

- None for this phase.
