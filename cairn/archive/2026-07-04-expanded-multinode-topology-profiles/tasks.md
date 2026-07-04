# Tasks: expanded-multinode-topology-profiles

## Phase 1: Topology profile model

- [x] [parallel] r[molten.testing.multinode.topology_profile_matrix] Define a pure topology profile model for pairwise transport, control quorum, restart/rejoin, subscriber peer, and wrong-topology fixtures.
- [x] [parallel] r[molten.testing.multinode.role_membership_negatives] Define role and membership validation diagnostics for undeclared node, undeclared link, subscriber-as-voter, transport-as-authority, and missing quorum evidence.

## Phase 2: Matrix wiring and tests

- [x] [serial] r[molten.testing.multinode.topology_profile_matrix] Bind topology profile ids into distributed metadata, run receipts, and gate diagnostics.
- [x] [serial] r[molten.testing.multinode.role_membership_negatives] Add positive fixtures for every topology profile family and negative fixtures for role and membership confusion.

## Phase 3: Readback

- [x] [parallel] r[molten.testing.multinode.topology_profile_matrix] Update distributed testing documentation with the topology-profile matrix and evidence scopes.
- [x] [serial] r[molten.testing.multinode.role_membership_negatives] Run focused topology-profile tests and `cairn validate --root .`, or record the blocker and next best check.
