## ADDED Requirements

### Requirement: Three-node VM topology covers quorum and restart/rejoin
r[molten.testing.multinode.three_node_quorum_topology] Molten SHOULD include a bounded three-node VM topology profile that exercises voter membership, majority/minority quorum behavior, restart/rejoin, and duplicate semantic commit suppression with canonical VM evidence.

#### Scenario: Majority evidence passes with matching node summaries
- GIVEN a three-node VM topology with declared voter roles, membership refs, quorum refs, and required workflow receipts
- WHEN a majority workflow completes
- THEN the VM evidence binds topology membership, quorum, per-node summaries, reconciliation refs, and child workflow refs
- AND the reconciliation gate passes only for matching semantic commit evidence.

#### Scenario: Restarting member rejoins without duplicate commit
- GIVEN a three-node VM topology where one member restarts after a queued operation
- WHEN the member rejoins and the workflow is reconciled
- THEN the evidence shows idempotent recovery or duplicate suppression
- AND no second semantic commit is accepted for the same operation id.

### Requirement: Three-node VM negatives reject role and quorum confusion
r[molten.testing.multinode.three_node_membership_negatives] Molten MUST reject three-node VM pass claims that treat subscriber, observer, transport-only, partitioned-minority, or missing-quorum evidence as admitted voter membership or authority evidence.

#### Scenario: Subscriber cannot satisfy voter membership
- GIVEN a three-node VM scenario where a subscriber or observer receipt is supplied as voter membership evidence
- WHEN membership or reconciliation validation runs
- THEN the gate denies before pass evidence is accepted
- AND diagnostics name the role mismatch.

#### Scenario: Partitioned minority cannot satisfy quorum
- GIVEN a three-node VM topology with only minority-side receipts after a partition
- WHEN quorum validation evaluates the evidence
- THEN validation denies the quorum claim
- AND diagnostic logs cannot substitute for missing majority receipts.
