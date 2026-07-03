## ADDED Requirements

### Requirement: Lifecycle denial diagnostics are deterministic
r[molten.lifecycle_state_machine_proof.denial_diagnostics] Molten MUST emit bounded, deterministic lifecycle transition diagnostics for denied transition predicates, including invalid state edges and action-target mismatches.

#### Scenario: Invalid transition names the denied edge
- GIVEN a lifecycle transition that jumps across required intermediate states
- WHEN Molten evaluates the lifecycle transition receipt
- THEN the receipt decision is `deny`
- AND diagnostics identify the invalid source and target state edge.

#### Scenario: Multiple predicate failures stay stable
- GIVEN a lifecycle transition whose state edge is invalid and whose action does not match the target state
- WHEN Molten evaluates the transition more than once
- THEN the diagnostic strings appear in the same order each time
- AND the denial receipt ref is stable for the same canonical input.

### Requirement: Lifecycle denial receipts bind failed checks
r[molten.lifecycle_state_machine_proof.denial_receipt_binding] Molten MUST bind denial receipts to the canonical lifecycle transition ref, the `deny` decision, deterministic diagnostics, and lifecycle check names whenever transition input validation succeeds.

#### Scenario: Denial receipt remains proof evidence
- GIVEN a syntactically valid lifecycle transition that fails semantic transition checks
- WHEN Molten emits the lifecycle transition receipt
- THEN the receipt binds the transition ref and denial diagnostics
- AND the receipt MUST NOT be accepted as a passing lifecycle transition.
