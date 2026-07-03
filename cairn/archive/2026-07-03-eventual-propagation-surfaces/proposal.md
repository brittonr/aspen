## Why

Molten uses Iroh gossip, Iroh docs, remote dataspace envelopes, and federated pull-sync for propagation, but those paths should not be confused with consensus. Operators and future implementers need an explicit model for eventual surfaces: what can be propagated, how replicas converge, what merge law applies, what evidence is replayable, and what remains non-authoritative. This avoids calling gossip "eventual consensus" while still giving eventual propagation a crisp contract.

## What Changes

- Define `eventual-surface-manifest-v1` for gossip/docs/federation surfaces with scope, merge law, idempotency key, tombstone/retraction policy, replay requirement, anti-entropy policy, and authority boundaries.
- Require deterministic merge/convergence laws before claiming eventual consistency for a surface.
- Record propagation and anti-entropy evidence as canonical receipts, with delivery logs or snapshots required for deterministic gates.
- Clarify that eventual propagation is not Raft, not linearizable, not authority, and not a global dataspace.

## Impact

- **Files**: runtime/remote/federation specs, remote dataspace and federation receipt models, merge-law validation core, docs, and tests.
- **Testing**: positive merge convergence fixtures and negative tests for missing merge law, conflicting concurrent state without resolution, stale tombstones, unrecorded live timing in deterministic gates, and attempts to use propagation as authority.
