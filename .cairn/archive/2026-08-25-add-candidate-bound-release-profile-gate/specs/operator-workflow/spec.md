# Operator Workflow Delta

## ADDED Requirements

### Requirement: Release profile validation has an executable gate

r[molten.prod_release_profile.executable_gate] Molten MUST expose release profile validation through an operator CLI and a Nix check. The command MUST emit the canonical validation value and MUST exit unsuccessfully for a deny decision.

#### Scenario: Valid profile emits pass evidence

- GIVEN complete candidate-bound release profile inputs
- WHEN the operator runs the release profile gate
- THEN the command emits a canonical pass value and exits successfully.

#### Scenario: Invalid profile preserves deny diagnostics

- GIVEN missing or placeholder release profile inputs
- WHEN the operator runs the release profile gate with an output path
- THEN the command writes the canonical deny value and exits unsuccessfully.

### Requirement: Gate fixtures do not claim candidate release readiness

r[molten.prod_release_profile.fixture_non_claim] Release profile command fixtures MUST state that they prove validator wiring only and MUST NOT be treated as candidate release evidence.

#### Scenario: Nix conformance fixture passes

- GIVEN the Nix check uses deterministic non-placeholder fixture references
- WHEN the check passes
- THEN its output remains conformance evidence and does not establish release eligibility.
