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

### Requirement: Nickel array contracts express uniqueness and bounds
r[molten.nickel_array_invariants.shared_array_helpers] Repository-owned Nickel contract modules SHOULD use shared helper contracts for array uniqueness, non-empty arrays, maximum lengths, required members, and unique BLAKE3 ref lists when those invariants are part of the reviewed domain.

#### Scenario: Duplicate reviewed ref fails export
- GIVEN a Nickel fixture whose field is declared as a unique evidence-ref array
- WHEN the fixture repeats the same BLAKE3 ref
- THEN Nickel export fails before generated JSON can be refreshed.

### Requirement: Array helper diagnostics identify the invariant
r[molten.nickel_array_invariants.helper_diagnostics] Repository-owned Nickel contracts SHOULD apply named helper predicates or targeted fixtures so duplicate, missing-member, non-empty, and bound failures identify the intended array invariant under test.

#### Scenario: Duplicate descriptor fixture identifies uniqueness
- GIVEN a negative fixture with duplicate plugin descriptor identities
- WHEN Nickel export evaluates the contract
- THEN the failure is associated with the descriptor uniqueness invariant
- AND generated evidence is not refreshed.

### Requirement: Production, peer, and multinode arrays reject ambiguity
r[molten.nickel_array_invariants.production_peer_multinode] Production profile, peer profile, and multinode scenario contracts MUST reject duplicate or contradictory array values where duplicates would make adapter membership, peer identity, artifact kinds, receipt refs, variance refs, or caveats ambiguous.

#### Scenario: Duplicate peer ref denies
- GIVEN a peer profile export with two profiles using the same peer ref
- WHEN Nickel evaluates the fixture
- THEN export fails with a duplicate-identity invariant.

### Requirement: Plugin contract arrays reject duplicate reviewed identities
r[molten.nickel_array_invariants.plugin_arrays] Plugin extension contracts and plugin capability grants MUST reject duplicate lifecycle callbacks, duplicate hostcall descriptor identities, duplicate required refs, and oversized evidence arrays where those fields are reviewed as sets.

#### Scenario: Duplicate lifecycle callback denies
- GIVEN a plugin extension contract fixture with the same lifecycle callback listed twice
- WHEN Nickel evaluates the fixture
- THEN export fails before the plugin contract can be converted to generated evidence.

### Requirement: Cairn policy arrays reject duplicate reviewed ids
r[molten.nickel_array_invariants.policy_arrays] Cairn policy contracts SHOULD use shared array helpers where schema ids, marker ids, marker tokens, replay ids, surface ids, receipt schema commands, or other reviewed policy tokens must be unique.

#### Scenario: Duplicate marker token fails export
- GIVEN a Cairn policy fixture with duplicate task marker tokens
- WHEN Nickel export evaluates the policy contract
- THEN export fails before generated policy JSON can be refreshed.

### Requirement: Array invariant failures have negative fixtures
r[molten.nickel_array_invariants.negative_arrays] Every newly tightened Nickel array invariant SHOULD have a negative fixture that demonstrates the intended duplicate, oversize, missing-member, or contradictory-list failure.

#### Scenario: Oversized array fixture fails
- GIVEN a contract field with a configured maximum array length
- WHEN a negative fixture exceeds that length
- THEN the fixture fails export and identifies the array invariant under test.

### Requirement: Nickel array tightening remains authoring-time only
r[molten.nickel_array_invariants.runtime_boundary] Nickel array invariant contracts MUST remain authoring-time fixture validation and MUST NOT replace runtime Preserves parsing, authority gates, policy gates, resource gates, provenance gates, retention gates, or execution gates.

#### Scenario: Valid export still requires runtime admission
- GIVEN a Nickel fixture exports successfully after array invariant validation
- WHEN runtime admission consumes the generated evidence
- THEN runtime still requires the subsystem's canonical receipt and semantic gates.

### Requirement: Shared bounded helpers use checked arithmetic
r[molten.shared_bounded_sinks.checked_counts] Repository-owned bounded collection helpers MUST calculate counts with checked arithmetic before mutating a collection and MUST fail closed when the next count would overflow or exceed the configured limit.

#### Scenario: One-past-limit push does not mutate
- GIVEN a bounded vector with item count equal to its configured limit
- WHEN a caller attempts to push one more item through the shared helper
- THEN the helper returns an error
- AND the vector contents remain unchanged.

### Requirement: Diagnostic sinks share bounded behavior
r[molten.shared_bounded_sinks.diagnostic_sink] New diagnostic accumulation code SHOULD use the shared bounded diagnostic sink behavior unless a subsystem documents a stricter local invariant.

#### Scenario: Diagnostic overflow denies consistently
- GIVEN a diagnostic sink at its configured maximum
- WHEN a subsystem attempts to add another diagnostic
- THEN the sink fails closed with deterministic diagnostics
- AND the subsystem does not silently drop or append the diagnostic.

### Requirement: Equivalent bounded helpers migrate to shared utilities
r[molten.shared_bounded_sinks.migration] Duplicated bounded push, extend, count, and diagnostic helpers SHOULD migrate to shared utilities when behavior is equivalent and local invariants do not require a stricter subsystem helper.

#### Scenario: Equivalent helper calls shared core
- GIVEN a subsystem helper that previously checked count limits before pushing into a vector
- WHEN the behavior is equivalent to the shared bounded helper
- THEN the subsystem delegates to the shared core
- AND preserves fail-closed no-mutation behavior.

### Requirement: Bounded helper migrations preserve evidence shape
r[molten.shared_bounded_sinks.hash_stability] Refactoring duplicated bounded helpers into shared utilities MUST preserve canonical receipt values when the only change is helper mechanics.

#### Scenario: Migrated receipt hash remains stable
- GIVEN a representative receipt fixture built before helper migration
- WHEN the same semantic input is built after migration
- THEN the canonical receipt ref is unchanged or the change records an explicit evidence migration note.

### Requirement: Bound-denial behavior is negatively covered
r[molten.shared_bounded_sinks.negative_bounds] Shared bounded helpers MUST include negative tests for one-past-limit, arithmetic overflow, extend overflow, and no-mutation-on-error cases.

#### Scenario: Extend overflow leaves destination unchanged
- GIVEN a destination collection and an incoming slice whose combined count exceeds the maximum
- WHEN bounded extend runs
- THEN it denies before appending any incoming item.
