## ADDED Requirements

### Requirement: Three-node quorum has executable VM shard evidence
r[molten.testing.three_node_quorum_vm.executable_shard] Molten MUST provide executable VM shard evidence for the `three-node-quorum` topology when host support is available, and the shard MUST bind fixture metadata, explicit voter roles, majority quorum receipts, restart or rejoin evidence, reconciliation refs, aggregate refs, and bounded-topology caveats.

#### Scenario: Majority quorum passes in executable topology
- GIVEN a three-node VM topology with explicit voter membership and a checked `vm-three-node-quorum` fixture
- WHEN the shard executes a majority quorum workflow
- THEN it emits canonical child receipts for membership, quorum, reconciliation, and aggregate evidence
- AND the shard states that the claim is bounded to the VM topology.

#### Scenario: Restarting member rejoins without duplicate semantic commit
- GIVEN a three-node VM topology where one voter restarts during the workflow
- WHEN the member rejoins and the workflow reconciles
- THEN recovery and reconciliation receipts bind the restart path
- AND duplicate semantic commit evidence is suppressed or denied before pass evidence is accepted.

### Requirement: Three-node quorum VM negatives fail closed
r[molten.testing.three_node_quorum_vm.negatives] Molten MUST deny three-node quorum VM pass evidence when voter membership is confused with subscriber or observer roles, topology refs are wrong, quorum refs are missing, transport evidence is treated as authority, minority partitions claim success, duplicate semantic commits are accepted, or logs are the only quorum evidence.

#### Scenario: Subscriber cannot satisfy voter membership
- GIVEN a three-node topology containing a subscriber or observer role
- WHEN that role is presented as voter membership evidence
- THEN the membership gate denies pass evidence
- AND diagnostics name the subscriber or observer promotion.

#### Scenario: Log-only quorum is rejected
- GIVEN VM logs that describe quorum success but no canonical quorum, reconciliation, or commit receipts
- WHEN the VM quorum gate evaluates the run
- THEN the decision is deny
- AND diagnostics state that logs are diagnostic-only.
