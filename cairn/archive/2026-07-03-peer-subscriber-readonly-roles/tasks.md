# Tasks: peer-subscriber-readonly-roles

## Phase 1: Role and grant model

- [x] [serial] r[molten.peer_subscriber.role_model] Define subscriber/read-only peer roles as scoped, attenuated read capabilities with projection kind, egress policy, redaction profile, resource limits, expiry, revocation, and evidence refs.
- [x] [serial] r[molten.peer_subscriber.subscription_grant] Define canonical `peer-subscription-grant-v1` and `peer-subscription-projection-receipt-v1` records.
- [x] [parallel] r[molten.peer_subscriber.read_requires_authority] Enforce that read-only subscription still requires explicit read authority, policy, resource, and egress admission.

## Phase 2: Projection and denial boundaries

- [x] [serial] r[molten.peer_subscriber.egress_policy] Implement pure projection/egress validation with redaction, deny-sensitive content handling, resource bounds, replayability metadata, and diagnostics.
- [x] [serial] r[molten.peer_subscriber.no_write_upgrade] Deny attempts to use subscriber/read-only grants for publish, assert, retract, node-control mutation, job execution, sync import, retention clearance, authority delegation, or destructive operation.
- [x] [parallel] r[molten.peer_subscriber.no_relay_republish] Deny relay, republish, cache-export, or transitive subscription unless the grant explicitly includes that attenuated scope.

## Phase 3: Surface and consensus integration

- [x] [parallel] r[molten.peer_subscriber.surface_projection] Bind subscriber projections into eventual propagation surfaces without allowing subscriber receipts to claim consensus or authority.
- [x] [parallel] r[molten.peer_subscriber.federation_readonly] Ensure federation inventory/catalog subscriber roles are hint/readback only and cannot import artifacts without receiver-side verification and admission.
- [x] [parallel] r[molten.peer_subscriber.raft_boundary] Document and enforce that subscriber/read-only peers are not Raft voters, non-voters, or learners without separate membership admission and read-index/read-capability evidence.

## Phase 4: Tests and validation

- [x] [serial] r[molten.peer_subscriber.positive_negative_tests] Add positive subscriber projection tests and negative tests for missing read authority, egress denial, stale grant, write-upgrade attempt, unauthorized republish, read-only sync import, and Raft learner confusion.
- [x] [serial] r[molten.peer_subscriber.validation] Run focused subscriber/read-only tests, peer/session tests, eventual surface/federation tests, consensus boundary tests, formatting, and Cairn validation before archiving.
