# Molten Nickel Toolchain Delta

## ADDED Requirements

### Requirement: Molten pins the Nickel 1.17 cohort

r[molten.nickel_toolchain.cohort] Molten MUST pin Nickel CLI `1.17.0`, `nickel-lang 2.2.0`, and `nickel-lang-core 0.18.0` from the reviewed upstream cohort.

#### Scenario: The cohort resolves

- GIVEN Molten builds embedded and command-line evaluator paths
- WHEN dependency resolution completes
- THEN every Nickel surface MUST match the declared cohort

#### Scenario: A floating or mixed cohort resolves

- GIVEN a branch, ambient package, or older embedded crate supplies Nickel
- WHEN cohort admission runs
- THEN admission MUST fail

### Requirement: Product policy remains Molten-owned

r[molten.nickel_toolchain.boundary] Nickel MUST remain an evaluator dependency. Molten MUST retain contract selection, defaults, target decoding, authority, runtime effects, and release decisions.

#### Scenario: A configuration is valid

- GIVEN Nickel accepts a configuration value
- WHEN Molten decodes it
- THEN Molten MUST still apply its own policy and authority checks

#### Scenario: Evaluation succeeds without authority

- GIVEN a value passes Nickel contracts but lacks required Molten authority
- WHEN runtime admission runs
- THEN the operation MUST be denied before effects

### Requirement: Compatibility tests include failure paths

r[molten.nickel_toolchain.compatibility] Molten MUST test valid policy and configuration fixtures plus malformed, import, contract, bound, and redaction failures.

#### Scenario: Valid fixtures run

- GIVEN representative supported fixtures
- WHEN the new cohort evaluates them
- THEN supported decoded outcomes MUST remain stable

#### Scenario: Invalid fixtures run

- GIVEN malformed or disallowed input
- WHEN the new cohort evaluates it
- THEN the input MUST fail with a stable bounded disposition
- AND secret-like data MUST remain redacted

### Requirement: Evidence records bounded provenance

r[molten.nickel_toolchain.evidence] Release evidence MUST record the exact crate versions, CLI version, upstream commit, and compatibility results.

#### Scenario: Evidence is emitted

- GIVEN repository checks pass
- WHEN evidence is generated
- THEN it MUST name the evaluator cohort
- AND it MUST NOT claim policy correctness or runtime correctness

### Requirement: The repository validation rail passes

r[molten.nickel_toolchain.validation] The change MUST pass focused tests, formatting, Clippy, Cairn gates, policy checks, and relevant Nix checks.

#### Scenario: Validation completes

- GIVEN dependencies, adapters, fixtures, and evidence are current
- WHEN validation runs
- THEN every required check MUST pass or report one exact blocker
