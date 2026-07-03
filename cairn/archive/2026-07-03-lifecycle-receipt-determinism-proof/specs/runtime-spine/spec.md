## ADDED Requirements

### Requirement: Lifecycle receipts are deterministic for identical inputs
r[molten.lifecycle_state_machine_proof.receipt_determinism] Molten MUST produce stable lifecycle transition refs, receipt refs, decisions, diagnostics, and canonical receipt values when the same lifecycle transition input is evaluated more than once.

#### Scenario: Repeated receipt generation is stable
- GIVEN a lifecycle transition input with canonical refs and a fixed logical step
- WHEN Molten constructs the transition record and receipt twice
- THEN both runs produce the same transition ref, receipt ref, decision, diagnostics, and canonical value.

### Requirement: Lifecycle receipts bind transition evidence
r[molten.lifecycle_state_machine_proof.receipt_evidence_binding] Molten MUST validate lifecycle receipt evidence by binding the receipt ref to the canonical receipt value, the transition ref to the canonical transition value, and the decision to the deterministic diagnostics for that transition.

#### Scenario: Tampered receipt is rejected
- GIVEN a lifecycle transition receipt whose decision, transition ref, diagnostics, or checks have been modified after receipt creation
- WHEN Molten validates the lifecycle receipt as proof evidence
- THEN validation denies the receipt
- AND diagnostics identify the binding that failed.
