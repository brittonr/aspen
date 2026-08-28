# Project Specification Delta

## ADDED Requirements

### Requirement: Molten selects strict static message-boundary admission

r[molten.message_boundary.static_admission]

Molten MUST pin a published immutable Octet revision that provides compiler-backed message-boundary architecture enforcement.

Molten MUST declare state owners, core scopes, message types, transitions, effect plans, shell and adapter scopes, runtime-handle provider identities, composition roots, targets, and features in its active Octet architecture policy.

#### Scenario: Compiler-backed boundary passes

- GIVEN the selected Octet revision and complete Molten architecture policy
- WHEN strict Octet admission runs across the declared production scope
- THEN no runtime handle, hidden shared-state path, undeclared ingress, or unresolved required fact MAY remain in a passing result.

#### Scenario: String scan passes but compiler facts find a leak

- GIVEN source-text boundary tests find no forbidden token
- AND compiler facts resolve an alias or vendor type to a live runtime handle in core state
- WHEN strict admission runs
- THEN admission MUST fail on the compiler-backed handle finding.

### Requirement: Message-boundary claims remain scoped

r[molten.message_boundary.claim_boundary]

A passing static or runtime message-boundary result MUST NOT claim exact-once delivery, durable delivery, global ordering, complete determinism, protocol correctness, availability, security, production readiness, or whole-stack compliance.

Static, pure-model, deterministic whole-system, multiprocess live, host-chaos, and VM or hardware evidence MUST retain separate roles and identities.

#### Scenario: Deterministic simulation passes

- GIVEN a complete deterministic message-oriented run passes replay and invariants
- WHEN release evidence is assembled
- THEN the result MAY satisfy the deterministic-simulation prerequisite
- AND it MUST NOT satisfy required live, host-chaos, VM, hardware, or production evidence.

#### Scenario: Octet receipt is labeled runtime proof

- GIVEN a passing static Octet message-boundary receipt
- WHEN evidence validation sees a deterministic-runtime or live-runtime role
- THEN validation MUST reject the role mismatch.

### Requirement: Final validation combines static and runtime boundaries

r[molten.message_boundary.verification]

Molten MUST validate policy, compiler facts, handle containment, transition paths, callback envelopes, effect completions, scheduler closure, same-core identity, live and deterministic parity, evidence, roadmap compatibility, and non-claims with positive and negative fixtures.

#### Scenario: Complete validation passes

- GIVEN all static and runtime message-boundary mechanisms pass under the selected source, targets, features, profiles, and evidence roles
- WHEN final validation runs
- THEN the candidate MUST satisfy this change's bounded acceptance contract.

#### Scenario: One mechanism is missing

- GIVEN static admission passes but scheduler closure or same-core conformance is absent
- WHEN final validation runs
- THEN final validation MUST fail rather than infer runtime conformance from static evidence.
