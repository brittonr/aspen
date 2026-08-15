# Evidence Gates

## Purpose

Adds a Molten-to-Valence adapter for stack evidence role and identity compatibility.

## Requirements

### Requirement: Valence stack evidence adapter contract
r[molten.evidence.valence_stack_adapter.contract] Molten MUST define a stack evidence adapter contract that maps local stack evidence members to Valence role/schema vocabulary while preserving Molten runtime and release-gate ownership.

#### Scenario: Complete stack envelope maps to Valence vocabulary
r[molten.evidence.valence_stack_adapter.fixtures.positive]
- GIVEN a stack evidence envelope contains Basalt, UCAN, Trellis, Octet, Valence, Cairn, and Mantle members with valid artifact refs, verification roles, supported schemas, and evidence-only non-claims
- WHEN the Molten-to-Valence adapter validates the envelope
- THEN validation MUST pass and report the corresponding Valence role/schema rows.

### Requirement: Invalid stack evidence fails closed
r[molten.evidence.valence_stack_adapter.validation] Molten MUST fail closed when stack evidence role, ref, schema, verification-role, non-claim, or Valence vocabulary compatibility is missing or inconsistent.

#### Scenario: Missing or malformed member fails
r[molten.evidence.valence_stack_adapter.fixtures.negative]
- GIVEN a stack evidence envelope has a missing role, duplicate role, malformed BLAKE3 ref, unsupported schema, missing verification role, missing evidence-only non-claim, overbroad authority claim, or Valence vocabulary mismatch
- WHEN adapter validation runs
- THEN validation MUST fail with deterministic diagnostics naming the invalid member and rule.

### Requirement: Adapter is pure core
r[molten.evidence.valence_stack_adapter.pure_core] The stack evidence adapter MUST be implemented as pure deterministic core logic over in-memory inputs.

#### Scenario: Shell owns side effects
r[molten.evidence.valence_stack_adapter.pure_core.shell]
- GIVEN a future CLI or harness loads Valence role policy or stack evidence artifacts from files
- WHEN validation is invoked
- THEN file reads, process execution, network access, clock access, and output rendering MUST remain outside the adapter core.

### Requirement: Stack adapter remains evidence-only
r[molten.evidence.valence_stack_adapter.docs] A passing stack evidence adapter report MUST NOT grant runtime authority, release authority, transport trust, storage trust, UCAN authority, or permission to bypass subsystem gates.

#### Scenario: Boundary is visible
r[molten.evidence.valence_stack_adapter.final_validation]
- GIVEN a stack evidence adapter report passes
- WHEN an operator reads the supported claim
- THEN the report MUST state that it proves only stack evidence role/schema/ref compatibility and evidence-only non-claim conformance.
