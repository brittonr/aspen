# project Specification

## Purpose
Capture the current Molten repository baseline before runtime architecture work lands.

## Requirements

### Requirement: Rust and Nix scaffold
r[molten.project.scaffold] The repository MUST provide a Rust workspace with a library crate, a binary crate, and a Nix development shell.

#### Scenario: Developer runs the baseline test
r[molten.project.scaffold.test]
- GIVEN a checkout of the repository
- WHEN a developer runs the configured Rust test command
- THEN the scaffold library test passes

### Requirement: Runtime integration dependencies
r[molten.project.dependencies] The repository MUST declare dependencies for Preserves, Syndicate, Iroh blobs/docs/gossip, Nickel, Steel, Wasmtime/WASI/component tooling, Redb, Blake3, Snafu, Serde, Tracing, Clap, Basalt, Cairn, Octet Valence, and Trellis, plus a Hegel dev dependency for property-based testing.

#### Scenario: Developer inspects project dependencies
r[molten.project.dependencies.inspect]
- GIVEN the repository manifest
- WHEN a developer reviews declared dependencies
- THEN the manifest identifies the crates intended for the Molten runtime integration

### Requirement: Proof checklist required fields
r[molten.project.change_proof_checklist.required_fields] Proof-affecting Cairn changes SHOULD include a proof checklist that names the proof claim, out-of-scope claims, trusted assumptions, positive evidence, negative evidence, canonical evidence refs, traceability updates, and regeneration commands.

#### Scenario: Change records proof scope
- GIVEN a Cairn change that alters proof, gate, evidence, traceability, replay, release, or mutation behavior
- WHEN the change is prepared for implementation or archive
- THEN its checklist identifies what is proved and what remains out of scope.

### Requirement: Checklist maps to Cairn tasks
r[molten.project.change_proof_checklist.cairn_tasks] Proof checklist items SHOULD map to Cairn tasks, requirement ids, or evidence notes so incomplete proof work remains visible.

#### Scenario: Negative evidence task is visible
- GIVEN a change that adds a new evidence gate
- WHEN tasks are reviewed
- THEN at least one task names the negative evidence required for that gate or records an explicit exemption.

### Requirement: Checklist exemptions are explicit
r[molten.project.change_proof_checklist.exemptions] Documentation-only, operator-guidance, or non-executable changes MAY use checklist exemptions, but the exemption class and supporting evidence SHOULD be recorded explicitly.

#### Scenario: Documentation-only exemption is recorded
- GIVEN a change that only updates operator guidance
- WHEN the proof checklist is completed
- THEN it records a documentation-only exemption and supporting doc evidence.

### Requirement: Traceability updates accompany evidence requirements
r[molten.project.change_proof_checklist.traceability_update] Changes that add or alter evidence-bearing requirements SHOULD include tasks to update traceability coverage with positive and negative evidence or explicit exemptions.

#### Scenario: New evidence requirement requires coverage task
- GIVEN a change adds an evidence-bearing requirement
- WHEN tasks are reviewed
- THEN the task list includes positive and negative coverage updates or an explicit exemption.

### Requirement: Hegel RS properties for core invariants
r[molten.project.change_proof_checklist.hegel_when_core] Changes that alter pure core invariants SHOULD include Hegel RS property-test tasks when generated inputs can strengthen coverage beyond example fixtures.

#### Scenario: Core decision law gets property task
- GIVEN a change alters a pure core decision law
- WHEN the proof checklist is completed
- THEN it includes a Hegel RS property-test task or explains why generated testing is not applicable.

### Requirement: Release-review evidence commands are recorded
r[molten.project.change_proof_checklist.release_review] Proof-affecting changes SHOULD record the smallest relevant validation commands and canonical evidence refs needed for release review.

#### Scenario: Reviewer can regenerate proof evidence
- GIVEN an implemented proof-affecting change
- WHEN a reviewer reads the checklist
- THEN they can identify the command or gate that regenerates the canonical evidence.

### Requirement: Checklist fixtures or examples
r[molten.project.change_proof_checklist.fixtures] The project SHOULD provide examples or fixtures for complete and incomplete proof checklists so contributors can recognize missing evidence.

#### Scenario: Incomplete checklist example shows denial
- GIVEN an example change with no negative evidence task
- WHEN the example is reviewed
- THEN it demonstrates the missing negative evidence gap.

### Requirement: Checklist documentation
r[molten.project.change_proof_checklist.docs] Contributor documentation SHOULD explain when a proof checklist is required and how to fill out claims, out-of-scope boundaries, trusted assumptions, positive evidence, negative evidence, Hegel RS properties, traceability updates, and regeneration commands.

#### Scenario: Contributor follows checklist docs
- GIVEN a contributor creates a proof-affecting Cairn change
- WHEN they follow the documentation
- THEN the change carries explicit proof scope, evidence coverage, and validation command notes before archive.

### Requirement: Current clippy gate stays warning-free
r[molten.project.clippy_gate.current_warning_free] The repository SHOULD keep `cargo clippy --all-targets -- -D warnings` passing before proof-affecting replay, release, or production-readiness evidence is promoted.

#### Scenario: Clippy gate blocks evidence refresh
- GIVEN a candidate source tree with clippy diagnostics denied by the configured command
- WHEN replay or release evidence is being prepared for promotion
- THEN the quality gate must be fixed or explicitly recorded as blocking before refreshed evidence is treated as current.

### Requirement: Nickel contract modules share common domain helpers
r[molten.project.nickel_contract_prelude.shared_helpers] Repository-owned Nickel contract modules SHOULD import shared pure helper contracts for common domains such as non-empty strings, non-empty arrays, BLAKE3 refs, stable ids, positive integers, exact schema metadata, allowed values, and distinct string collections.

#### Scenario: Contract module uses shared helper
- GIVEN a Nickel contract module needs to validate a BLAKE3 ref or non-empty array
- WHEN the module is reviewed
- THEN it imports the shared helper instead of copying a divergent local predicate unless a local domain-specific exception is documented

#### Scenario: Shared helper tightens consistently
- GIVEN a shared helper predicate is tightened to reject a malformed common value
- WHEN dependent contract modules evaluate their fixtures
- THEN every importer observes the same reviewed behavior

### Requirement: Shared Nickel prelude remains authoring-time only
r[molten.project.nickel_contract_prelude.authoring_boundary] Shared Nickel contract helpers MUST remain part of source-controlled authoring and fixture validation and MUST NOT introduce runtime Nickel evaluation for production startup, plugin admission, policy authority, or receipt verification.

#### Scenario: Runtime consumes checked exports
- GIVEN a contract module imports the shared prelude for fixture validation
- WHEN runtime admission or startup validation runs
- THEN it consumes checked exported JSON or Preserves evidence and does not invoke the Nickel prelude as live authority

### Requirement: Cairn policy contracts resolve internal references
r[molten.project.cairn_policy_integrity.references] Cairn policy Nickel contracts MUST reject generated policy source when artifact dependencies, determinism surfaces, replay groups, replay cases, receipt contracts, or receipt schemas reference ids that are not declared in the same reviewed policy source.

#### Scenario: Policy with declared references exports
- GIVEN a Cairn policy whose artifact `requires` entries, replay surface refs, receipt schema commands, and receipt contracts all point to declared ids or commands
- WHEN the policy is evaluated through Nickel
- THEN the export succeeds and generated policy JSON preserves those reviewed relationships

#### Scenario: Policy with stale reference fails
- GIVEN a Cairn policy whose artifact dependency, replay case, replay group, receipt command, or determinism surface points to an unknown id
- WHEN the policy is evaluated through Nickel
- THEN export fails before stale policy JSON can be generated

### Requirement: Cairn policy ids and markers are unique
r[molten.project.cairn_policy_integrity.uniqueness] Cairn policy Nickel contracts MUST reject duplicate artifact ids, duplicate marker ids, duplicate marker tokens, duplicate replay case ids, duplicate replay group ids, and ambiguous receipt schema command entries.

#### Scenario: Distinct policy ids export
- GIVEN a Cairn policy with distinct artifact, marker, replay, and receipt schema identities
- WHEN the policy is evaluated through Nickel
- THEN export succeeds without introducing ambiguous policy lookup behavior

#### Scenario: Duplicate policy identity fails
- GIVEN a Cairn policy with a repeated marker token, artifact id, replay case id, replay group id, or receipt schema command
- WHEN the policy is evaluated through Nickel
- THEN export fails before ambiguous validation behavior can enter generated policy evidence

### Requirement: Contract validation diagnostics identify fields or invariants
r[molten.project.contract_diagnostics.locality] Repository-owned Nickel contract validation SHOULD report or name the failing field, domain helper, fixture, or cross-field invariant closely enough that reviewers can distinguish malformed input from unrelated import, parse, or tooling failures.

#### Scenario: Field-domain failure is local
- GIVEN a contract fixture with one malformed BLAKE3 ref, invalid enum value, unsafe path, or empty required array
- WHEN fixture validation fails
- THEN the failure output or fixture expectation identifies the intended field-domain invariant

#### Scenario: Cross-field failure is local
- GIVEN a contract fixture with an inverted validity window, duplicate descriptor, stale internal reference, or contradictory resource limit
- WHEN fixture validation fails
- THEN the failure output or fixture expectation identifies the intended cross-field invariant

### Requirement: Diagnostic improvements preserve fail-closed behavior
r[molten.project.contract_diagnostics.no_validation_weakening] Refactoring contracts for clearer diagnostics MUST NOT cause previously rejected malformed fixtures to export successfully or weaken runtime Rust admission of checked-in evidence.

#### Scenario: Diagnostic refactor keeps negative fixtures failing
- GIVEN a contract module is refactored into field-level contracts and named predicates
- WHEN the positive and negative fixture suite runs
- THEN valid fixtures still export and malformed fixtures still fail for the expected invariant classes
