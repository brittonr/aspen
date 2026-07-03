# Design: peer subscriber and read-only roles

## Scope

This change defines subscriber/read-only peer roles as first-class attenuated capabilities. It builds on peer profiles, peer sessions, eventual propagation surfaces, and handoff bundles, but it can be implemented incrementally as a distinct role and receipt layer.

## Role model

A subscriber role is a scoped read capability. It may allow a peer to observe one or more of:

- remote dataspace topic projections,
- Iroh docs namespace projections,
- federation inventory or catalog hints,
- node/service status summaries,
- evidence/readback streams,
- anti-entropy status for a named resource class.

Each role records scope, allowed projection kind, egress filter, redaction profile, maximum delivery/resource envelope, replay requirement, expiry, revocation refs, and policy/resource refs. A read-only role never implies write, publish, assert, retract, control, retention, execution, import, authority delegation, or Raft membership.

## Subscription grant and projection receipts

`peer-subscription-grant-v1` binds peer/session refs, scope, projection kind, egress policy, redaction profile, resource limits, expiry, revocation state, and evidence refs.

`peer-subscription-projection-receipt-v1` records what was delivered or denied: source ref, projection ref, subscriber peer/session refs, filter/redaction decisions, replayability, resource consumption, and diagnostics. Projection receipts are evidence-only and must not replace read authority for future deliveries.

## Egress and confidentiality

Read-only still leaks data. Every subscriber delivery must pass egress policy and redaction before bytes or canonical values leave the node. Private refs, secrets, retention-sensitive state, revoked content, and unauthorized capability-bearing payloads must deny or be redacted according to the configured projection contract.

## Write-upgrade denial

A subscriber may send acknowledgements or pull queries only when the grant explicitly allows that request class. Any attempt to use a subscriber grant for publish/assert/retract, node-control mutation, job execution, sync import, retention clearance, authority delegation, or destructive operation denies before side effects.

## Consensus boundary

Read-only peer is not a Raft learner, non-voter, or voter. If a future feature exposes control-plane read replicas or linearizable read proxying, it must require a separate read-index/read-capability path and, for Raft learners, the stronger membership admission path.

## Functional core

The core validates subscription grants, computes projection decisions, applies redaction/filter declarations over in-memory values, and returns deterministic receipts. The shell owns transport, state-root reads/writes, live delivery, and operator rendering.

## Non-goals

- No anonymous public subscription by default.
- No implicit relay or republish permission.
- No write upgrade from a read-only grant.
- No Raft learner or membership role from a subscriber session.
