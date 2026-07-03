## ADDED Requirements

### Requirement: Lifecycle transition relation is finite and explicit
r[molten.lifecycle_state_machine_proof.transition_relation_table] Molten MUST expose the lifecycle transition relation as a bounded, reviewable finite relation so proof tests can enumerate every lifecycle source and target state without relying on adapter behavior.

#### Scenario: Allowed edge appears in the matrix
- GIVEN the lifecycle state enum and the lifecycle transition relation
- WHEN the lifecycle proof matrix enumerates all source and target states
- THEN every permitted lifecycle edge appears in the relation exactly once
- AND no unlisted lifecycle edge produces a passing transition receipt.

### Requirement: Lifecycle action-target matrix is exhaustive
r[molten.lifecycle_state_machine_proof.action_target_matrix] Molten MUST prove lifecycle receipt decisions across every lifecycle state, action, and target-state combination, and a receipt MUST pass only when the state edge is allowed and the action is valid for the target state or is an explicit supervisor decision.

#### Scenario: Mismatched action denies
- GIVEN an allowed lifecycle edge with an action that does not match the target state
- WHEN Molten evaluates the lifecycle transition receipt
- THEN the receipt decision is `deny`
- AND diagnostics identify the action-target mismatch.
