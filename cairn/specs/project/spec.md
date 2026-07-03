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
