# Consensus Delta

## ADDED Requirements

### Requirement: Protocol-aware storage recovery
r[molten.consensus.storage_fault_recovery] Molten MUST reconcile missing or faulty Raft state with protocol facts before it truncates, replaces, or admits that state.

#### Scenario: Quorum establishes an uncommitted entry
- GIVEN a local faulty entry and bounded peer responses for the same term and index
- WHEN a quorum establishes that the entry was not committed
- THEN the pure recovery core can plan its removal or replacement

#### Scenario: Commitment remains unknown
- GIVEN incomplete or conflicting peer responses
- WHEN no quorum establishes committed or uncommitted status
- THEN the recovery core returns a wait decision and preserves the local evidence

#### Scenario: Committed entry is missing
- GIVEN peer evidence that identifies one committed entry
- WHEN recovery plans a repair
- THEN it never plans committed-entry truncation

### Requirement: Storage recovery progress is participant-scoped
r[molten.consensus.storage_fault_recovery_progress] Molten MUST evaluate recovery progress for each affected participant. The result MUST bind the required item, local sufficiency facts, admitted peer set, disruptive faults, observation completeness, and a finite virtual progress horizon. The result MUST be pass, fail, not-evaluated, or incomplete. Molten MUST NOT request remote repair when admitted local durable state is sufficient. It MUST NOT report global unavailability unless complete observations show that every permitted source lacks the exact required item.

#### Scenario: Local durable state is sufficient
- GIVEN an affected participant has admitted local durable state that contains every required item
- WHEN the pure recovery core plans the next action
- THEN it does not request remote repair.

#### Scenario: An admitted peer has the missing committed item
- GIVEN a stable recovery window and one admitted peer with the exact required committed item
- WHEN the affected participant does not repair that item within the declared virtual horizon
- THEN its recovery progress result fails.

#### Scenario: Every permitted source lacks the item
- GIVEN complete observations from every permitted source show that each source lacks the exact required item
- WHEN recovery progress is evaluated
- THEN the result can report bounded global unavailability.

#### Scenario: Peer observation is incomplete
- GIVEN at least one required peer observation or final-drain fact is missing
- WHEN recovery progress is evaluated
- THEN it returns incomplete and does not report pass, failure, or global absence.

### Requirement: Repair admission before voting
r[molten.consensus.repair_admission] A node with repaired, corrupt, or uncertain persistent consensus state MUST NOT vote, lead, or serve durable operations until recovery admission succeeds.

#### Scenario: Local database repair completes
- GIVEN Redb reports a local repair
- WHEN the node restarts
- THEN Molten keeps voting readiness disabled pending protocol recovery

#### Scenario: Recovery admission succeeds
- GIVEN storage reopened, protocol reconciliation completed, and current membership facts match
- WHEN the pure admission core evaluates the observations
- THEN it can admit the node for its permitted role

### Requirement: Storage-fault claim boundary
r[molten.consensus.storage_fault_claim_boundary] Consensus receipts MUST bind exact storage and campaign cohorts and MUST state that local repair, checksums, and passing campaigns do not prove arbitrary cluster safety.

#### Scenario: Receipt omits a required cohort identity
- GIVEN a storage-fault campaign receipt without its candidate, kernel, filesystem, device, or fault-profile identity
- WHEN Molten validates the receipt
- THEN validation fails closed

### Requirement: Consensus storage-fault validation
r[molten.consensus.storage_fault_validation] Molten MUST include positive and negative persistent-node campaigns for corruption, lost stable state, restart, lagging replicas, unavailable peers, and protocol-aware repair.

#### Scenario: Quorum-intersection node loses state
- GIVEN one faulty quorum-intersection node, one unavailable correct node, and one lagging node
- WHEN the persistent-Raft campaign attempts recovery and election
- THEN the oracle rejects unsafe voting, committed-entry loss, or unsupported truncation
