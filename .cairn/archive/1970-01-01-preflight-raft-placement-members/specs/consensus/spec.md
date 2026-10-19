# Bounded admitted Raft placement membership

## ADDED Requirements

### Requirement: Public placement reports preflight actual admitted members

r[molten.consensus.placement_admitted_member_preflight] The public `consensus_placement_report` producer MUST reject more than the existing `MAX_RAFT_MEMBERS = 32` actual admitted members before formatting a per-unrecognized-member diagnostic or cloning members into the report. It MUST retain the existing result/error shape, successful and invalid-ref validation behavior at exactly 32, and duplicate-member behavior for admitted input. The candidate pool remains subject to its existing generic `MAX_RAFT_REFS = 4,096` validation; this change MUST NOT impose the membership cap on candidates.

#### Scenario: Exact admitted member bound remains eligible

- GIVEN 32 canonical admitted members matching 32 placement candidates, sufficient distinct fault domains, and valid policy evidence
- WHEN the public producer evaluates placement
- THEN the decision passes and the report contains those 32 admitted members without extra diagnostics.

#### Scenario: Admitted invalid refs retain validation precedence

- GIVEN exactly 32 admitted members and either a malformed candidate ref or a malformed admitted ref
- WHEN the public producer evaluates placement
- THEN the existing corresponding canonical-ref error is returned, not a count error.

#### Scenario: Admitted duplicate membership remains unchanged

- GIVEN exactly 32 admitted entries including a duplicate candidate ref and otherwise safe placement evidence
- WHEN the public producer evaluates placement
- THEN it retains the existing duplicate-entry report behavior; the new check is cardinality, not uniqueness validation.

#### Scenario: Thirty-third admitted member is denied before expansion

- GIVEN 33 valid admitted refs including one ref not in the candidate pool
- WHEN the public producer evaluates placement
- THEN it returns `consensus placement admitted members count 33 exceeds maximum 32` before formatting unmatched-member diagnostics or cloning report members.

#### Scenario: Thirty-third candidate is not a thirty-third admitted member

- GIVEN 33 valid candidate refs but only 32 actual admitted refs
- WHEN the public producer evaluates placement
- THEN the existing generic candidate-ref validation and successful admitted-member decision remain eligible.
