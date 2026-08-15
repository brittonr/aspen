# Evidence Gates

## Purpose

Adds stack adapter boundaries for Molten evidence-gated runtime integration.

## Requirements

### Requirement: Stack evidence envelope
r[molten.evidence.stack_adapters.envelope] Molten MUST define a canonical stack evidence envelope for upstream Basalt, UCAN, Trellis, Octet, Valence, Cairn, and Mantle inputs used by runtime admission or release evidence decisions.

#### Scenario: Complete stack envelope passes
r[molten.evidence.stack_adapters.envelope.positive]
- GIVEN an envelope member names a supported role, schema, BLAKE3 identity, producer identity, verification role, and required non-claim boundary
- WHEN the envelope contract validates the member
- THEN validation MUST pass and preserve the member role for runtime admission.

#### Scenario: Missing or stale envelope member fails
r[molten.evidence.stack_adapters.envelope.negative]
- GIVEN an envelope member omits a required role, declares an unsupported schema, uses a stale digest, or weakens non-claims
- WHEN the envelope contract validates the member
- THEN validation MUST fail with deterministic diagnostics.

### Requirement: Stack adapter ports
r[molten.evidence.stack_adapters.ports] Molten MUST route upstream-specific parsing, filesystem reads, command execution, and receipt verification through shell adapter ports before pure runtime cores consume stack facts.

#### Scenario: Pure core receives parsed facts only
r[molten.evidence.stack_adapters.ports.core]
- GIVEN a runtime admission decision depends on Basalt, UCAN, Trellis, Octet, Valence, Cairn, or Mantle evidence
- WHEN the pure runtime core evaluates the decision
- THEN it MUST receive parsed facts, refs, and verification roles rather than performing upstream I/O or command execution.

#### Scenario: Adapter shell owns upstream I/O
r[molten.evidence.stack_adapters.ports.shell]
- GIVEN an upstream evidence file, command, or repository-specific DTO must be consumed
- WHEN Molten imports that evidence
- THEN an adapter shell MUST verify or parse the upstream artifact and emit a Molten stack evidence envelope member.

### Requirement: Dependency-boundary diagnostic
r[molten.evidence.stack_adapters.dependency_boundary] Molten SHOULD provide a diagnostic that identifies stack-owned crates used outside approved adapter modules.

#### Scenario: Approved adapter use is accepted
r[molten.evidence.stack_adapters.dependency_boundary.approved]
- GIVEN stack-owned crates are referenced only from approved adapter modules
- WHEN the dependency-boundary diagnostic runs
- THEN it SHOULD report the usage as accepted.

#### Scenario: Direct internal dependency leak is reported
r[molten.evidence.stack_adapters.dependency_boundary.leak]
- GIVEN a runtime core module directly imports an upstream stack crate that is not approved for that module
- WHEN the dependency-boundary diagnostic runs
- THEN it SHOULD report the module, crate, and required adapter boundary.

### Requirement: Stack evidence non-claims
r[molten.evidence.stack_adapters.non_claims] Molten stack evidence envelopes MUST state that upstream evidence refs do not grant authority, prove runtime correctness, or prove upstream verifier soundness by themselves.

#### Scenario: Overbroad claim is rejected
r[molten.evidence.stack_adapters.non_claims.reject_overclaim]
- GIVEN an envelope member claims universal runtime correctness, upstream verifier soundness, or release eligibility without the required supporting role
- WHEN the envelope contract validates the member
- THEN validation MUST fail closed with an overclaim diagnostic.

### Requirement: Validation evidence
r[molten.evidence.stack_adapters.validation] The change MUST include positive and negative envelope fixtures plus focused runtime or contract checks before archive.

#### Scenario: Fixture matrix covers stack envelope behavior
r[molten.evidence.stack_adapters.validation.fixtures]
- GIVEN complete, missing-role, stale-ref, unsupported-schema, and overclaim fixtures
- WHEN focused validation runs
- THEN complete fixtures MUST pass, negative fixtures MUST fail closed, and the receipt MUST bind fixture and policy identities.
