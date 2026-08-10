## Existing requirement restated for traceability repair

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
