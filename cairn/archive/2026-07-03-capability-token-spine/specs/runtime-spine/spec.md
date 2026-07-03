## ADDED Requirements

### Requirement: Capability admission does not bypass subsystem gates
r[molten.capability_token.subsystem_boundary] Molten MUST preserve subsystem-specific provenance, source-gate, retention, execution, replay, consensus, and resource gates after capability admission passes.

#### Scenario: Capability token cannot replace provenance
- GIVEN a peer has a passing capability admission receipt for a node-control install operation
- WHEN the install payload lacks admitted provenance or source-gate evidence
- THEN install still denies before side effects
- AND diagnostics report the missing subsystem evidence separately from capability admission.
