# Tasks: three-node-quorum-platform-coverage

## Phase 1: Pure gate extensions

- [x] [parallel] r[molten.testing.multinode.three_node_vm_quorum_execution] Extend or reuse pure three-node topology/reconciliation inputs to model executable VM node summaries and duplicate-suppression evidence.
- [x] [parallel] r[molten.testing.multinode.three_node_vm_membership_negatives] Add negative pure fixtures for subscriber, observer, transport-only, partitioned-minority, missing-quorum, and log-only claims.

## Phase 2: VM shard

- [x] [serial] r[molten.testing.multinode.three_node_vm_quorum_execution] Add an executable three-node VM shard with explicit voter roles, isolated state roots, and bounded restart/rejoin workflow evidence.
- [x] [serial] r[molten.testing.multinode.three_node_vm_membership_negatives] Bind denial fixtures into the shard output without promoting diagnostic logs to pass evidence.

## Phase 3: Aggregate and validation

- [x] [serial] r[molten.testing.multinode.three_node_vm_quorum_execution] Bind the three-node shard receipt into VM aggregate evidence or document when the shard is explicitly excluded from fast gates.
- [x] [serial] r[molten.testing.multinode.three_node_vm_membership_negatives] Run pure three-node tests plus the executable shard when host VM support is available.
