# Tasks: three-node-vm-quorum-topology

## Phase 1: Topology and gate model

- [x] [parallel] r[molten.testing.multinode.three_node_quorum_topology] Define a pure three-node topology profile for voter, restarting-member, subscriber or observer roles, allowed links, quorum refs, required receipts, and caveats.
- [x] [parallel] r[molten.testing.multinode.three_node_membership_negatives] Extend membership and reconciliation validation fixtures for subscriber-as-voter, transport-only-as-authority, partitioned-minority quorum, and missing quorum refs.

## Phase 2: VM scenario

- [x] [serial] r[molten.testing.multinode.three_node_quorum_topology] Add a bounded three-node VM shard or profile that starts the declared nodes, runs a majority workflow, restarts one member, and records rejoin evidence.
- [x] [serial] r[molten.testing.multinode.three_node_membership_negatives] Add VM or pure negative fixtures for role confusion, wrong topology, missing majority receipt, duplicate semantic commit, and log-only quorum success.

## Phase 3: Evidence and docs

- [x] [parallel] r[molten.testing.multinode.three_node_quorum_topology] Bind three-node topology profile, membership, quorum, node summaries, reconciliation gate, and child refs into VM manifests.
- [x] [parallel] r[molten.testing.multinode.three_node_membership_negatives] Document the evidence scope and explain why two-node transport evidence cannot satisfy three-node quorum claims.
- [x] [serial] r[molten.testing.multinode.three_node_quorum_topology] Run focused topology/reconciliation tests plus the smallest three-node VM shard, or record host-support blockers and next best checks.
