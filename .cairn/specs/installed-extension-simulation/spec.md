# Installed Extension Simulation Specification

## Purpose

Defines the `installed-extension-simulation` capability.

## Requirements

### Requirement: Admission is an explicit separate profile

r[molten.fabric_simulation.installed_extension_admission] Installed-extension worlds MUST use a separate bounded admission input and an explicit admitted extension-profile registry. Node and workload bindings MUST select exact nominal immutable profiles; missing, foreign and duplicate selections MUST deny before scheduling. Product names and archive presence MUST NOT grant admission.

#### Scenario: Exact installed world
- GIVEN a known immutable profile and matching core, node and contiguous workload bindings
- WHEN installed admission runs
- THEN the shared scheduler MAY accept the admitted world without unrelated reference-service rows.

#### Scenario: Foreign or missing profile
- GIVEN a missing, foreign, duplicate or incorrectly linked profile
- WHEN installed admission runs
- THEN admission MUST deny before any handler or protected effect.

### Requirement: Invariant handlers are exact

r[molten.fabric_simulation.installed_extension_invariant_bindings] Every selected profile MUST declare finite nonempty required invariant/handler identity pairs. Every required pair MUST have exactly one matching observed binding. Wrong handler, unknown invariant, missing invariant or duplicate binding MUST deny. Admission MUST NOT accept an unchecked callback or arbitrary string exemption.

### Requirement: Same core remains explicit

r[molten.fabric_simulation.installed_extension_same_core] Both existing same-core identities on each installed node MUST equal its admitted profile's complete core identity. The concrete shell MUST execute that maintained core and evaluate its bound invariants against actual states, not a substitute model.

### Requirement: Installed execution remains bounded

r[molten.fabric_simulation.installed_extension_bounded_execution] Existing world/node/port/fault/claim checks MUST apply. Installed profile node and workload ceilings MUST be finite and enforced, and workload sequence and generation linkage MUST be exact. Missing facts, excessive growth, unknown nodes and stale generations MUST deny.

### Requirement: Reference semantics are preserved

r[molten.fabric_simulation.installed_extension_reference_preservation] Reference-world admission MUST retain its original workload and three semantic-oracle requirements and its canonical projection meaning. Installed profiles MUST NOT bypass those requirements when entering a reference runner, nor fabricate reference oracle rows for a different product.

### Requirement: Acceptance includes actual consumers and mutants

r[molten.fabric_simulation.installed_extension_validation] Acceptance MUST exercise the actual installed consumer core, deterministic replay and bounded mechanism faults, and MUST pair positive admission with missing/wrong/duplicate invariant, profile/core substitution and over-bound mutants. Concrete live/multiprocess evidence remains separately required for live consistency claims. Configuration and authored tests alone MUST NOT establish execution, confidentiality or production eligibility.
