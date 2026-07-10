## ADDED Requirements

### Requirement: Three-node VM quorum shard exercises majority and restart
r[molten.testing.multinode.three_node_vm_quorum_execution] Molten SHOULD provide an executable bounded three-node VM shard that exercises voter membership, majority quorum behavior, restart/rejoin, and duplicate semantic commit suppression with canonical topology, node-summary, quorum, and reconciliation evidence.

#### Scenario: Majority quorum evidence passes with reconciliation
- GIVEN a three-node VM topology with declared voter roles, membership refs, quorum refs, and required workflow receipts
- WHEN a majority workflow completes and node summaries are reconciled
- THEN the shard emits passing evidence that binds topology membership, quorum refs, per-node summaries, duplicate-suppression refs, restart or rejoin refs, reconciliation refs, and child workflow refs
- AND the pass claim is scoped to the declared VM topology.

#### Scenario: Restarting voter rejoins without duplicate commit
- GIVEN a queued operation and a voter node that restarts during the workflow
- WHEN the node rejoins and the workflow is reconciled
- THEN evidence shows idempotent recovery or duplicate suppression
- AND no second semantic commit is accepted for the same operation id.

### Requirement: Three-node VM negatives reject role and quorum confusion
r[molten.testing.multinode.three_node_vm_membership_negatives] Molten MUST reject three-node VM pass claims that treat subscriber, observer, transport-only, partitioned-minority, missing-quorum, or log-only evidence as admitted voter membership, quorum, authority, or policy evidence.

#### Scenario: Subscriber evidence cannot satisfy voter membership
- GIVEN a three-node VM scenario where a subscriber or observer receipt is supplied as voter membership evidence
- WHEN membership, quorum, or reconciliation validation runs
- THEN the gate denies before pass evidence is accepted
- AND diagnostics name the role mismatch.

#### Scenario: Partitioned minority cannot satisfy quorum
- GIVEN a three-node topology with only minority-side receipts after a partition
- WHEN quorum validation evaluates the evidence
- THEN validation denies the quorum claim
- AND diagnostic logs cannot substitute for missing majority receipts.
