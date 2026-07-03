## ADDED Requirements

### Requirement: Upgrade drains require terminal protocol evidence
r[molten.upgrade_drain_state_proof.terminal_protocol_gate] Molten MUST prove that protocol-drain tasks complete only when a passing protocol-session gate binds the affected old protocol ref and at least one terminal session-state ref.

#### Scenario: Empty terminal states deny drain
- GIVEN an upgrade drain task with a protocol-session gate receipt that lists no terminal final states
- WHEN drain completion is evaluated
- THEN the upgrade receipt decision is `deny`
- AND cutover side effects are not emitted.

### Requirement: Upgrade drain protocol refs are exact
r[molten.upgrade_drain_state_proof.protocol_ref_binding] Molten MUST prove that protocol-drain evidence matches the task `from_ref` or explicit affected/compatibility refs and denies stale or wrong-protocol gates before mutation.

#### Scenario: Wrong protocol gate denies cutover
- GIVEN an upgrade task from protocol ref `old`
- WHEN the supplied lifecycle gate receipt binds protocol ref `other`
- THEN the drain decision is `deny`
- AND diagnostics identify the wrong protocol ref.

### Requirement: Upgrade denial preserves pre-cutover state
r[molten.upgrade_drain_state_proof.no_mutation_on_deny] Molten MUST prove that missing, denied, stale, malformed, or wrong-protocol drain evidence leaves registry, routing, compatibility, and artifact refs unchanged.

#### Scenario: Stale compatibility ref leaves routing unchanged
- GIVEN a drain task with stale compatibility evidence
- WHEN upgrade cutover admission runs
- THEN the decision is `deny`
- AND before/after routing or registry refs are identical.
