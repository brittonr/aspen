## Why

Aspen's accepted fabric transport contract defines adapter-neutral protocol, session, stream, framing, flow-control, cancellation, identity, failure, and evidence semantics. The live `IrohTransportAdapter` currently exercises those semantics only through `live_loopback_frame`, which creates both endpoints inside one process, exchanges one bounded echo, and returns. It cannot keep an admitted listener alive, export a bounded endpoint descriptor, connect a distinct process, or route protocol frames between supervised service instances.

That gap blocks `fabric-consistency-service-runtime` from producing honest multi-process Raft evidence and transitively blocks whole-system simulation, coordination delivery, DAG synchronization, content replication, federation, and later consensus acceleration. Same-process loopback, ambient sockets, or fabricated endpoint receipts cannot discharge the blocker.

## What Changes

- Add canonical capability-bound endpoint descriptors and listener/session lifecycle values for cross-process transport without exposing Iroh handles, leaking raw locators through default evidence, or treating endpoint possession as authority.
- Add pure admission and transition logic for atomic readiness and endpoint publication, endpoint consumption, protocol/profile compatibility and revocation, service-generation fencing, peer binding, framing, resource limits, cancellation, drain, cleanup, and terminal outcomes.
- Add a thin long-lived Iroh listener/client shell that binds only admitted endpoint capabilities, routes bounded frames through the existing canonical transport contract, and never falls back to ambient sockets.
- Add distinct-process conformance fixtures using the receipt-first cluster harness, including parent-observed child separation, positive exchange, and negative readiness, revocation, protocol, profile, generation, identity, framing, deadline, disconnect, cancellation, and cleanup cases.
- Emit bounded endpoint, listener, session, failure, drain, and cleanup evidence sufficient to distinguish cross-process live transport from same-process diagnostic loopback while preserving delivery and authority non-claims.

## Impact

- **Files**: `crates/molten-core/src/fabric_transport/`, `src/fabric_transport/`, transport profile input and deterministic exports when configuration is required, cluster-harness fixtures, operator readback, `docs/fabric-transport-session-runtime.md`, and `cairn/specs/fabric-transport/spec.md`.
- **Testing**: pure endpoint/listener/session admission, shared adapter conformance, separate-process Iroh exchange, bounded framing and flow control, wrong-protocol/profile/generation/peer denial, timeout and uncertain delivery, cancellation, drain, cleanup, handle-leak guards, and no-ambient-fallback checks.
- **Safety**: endpoint descriptors and successful transport establish connectivity observations only. They do not grant membership, protocol authority, application capability, durability, retry safety, consensus, protocol correctness, or release readiness.
