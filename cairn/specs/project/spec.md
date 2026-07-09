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


### Requirement: CLI modules stay thin shells
r[molten.modularity.cli_shell.thin_shell] CLI modules SHOULD limit themselves to argument parsing, path resolution, file IO, adapter orchestration, stdout/stderr rendering, process-exit behavior, and conversion between user-facing diagnostics and structured domain results.

#### Scenario: CLI handler delegates decision
- GIVEN a CLI command that performs domain validation, planning, or admission decisions
- WHEN the command is migrated for modularity
- THEN the handler converts parsed arguments into typed input, calls a library command core, and performs only the shell effects required by the structured result

#### Scenario: Domain decision in CLI is flagged
- GIVEN a CLI module contains deterministic domain decision logic that can be evaluated from in-memory inputs
- WHEN reviewers inspect a modularity change touching that module
- THEN the logic is moved to a library core or an explicit staged-migration exemption is recorded

### Requirement: Command cores are typed and testable
r[molten.modularity.cli_shell.typed_core] Extracted command cores MUST be callable without Clap parsing, filesystem state, stdout, stderr, process exits, network services, or live adapter execution.

#### Scenario: Valid command input succeeds in memory
- GIVEN a typed command-core input representing a valid command request
- WHEN a unit test calls the command core
- THEN it returns structured success, planned operations, receipts, or diagnostics without invoking the CLI binary

#### Scenario: Invalid command input fails in memory
- GIVEN malformed paths, missing evidence refs, stale refs, unsupported options, contradictory flags, or denied domain inputs represented in memory
- WHEN a unit test calls the command core
- THEN it returns a structured error or denial without writing files, printing output, or exiting the process

### Requirement: CLI modularity preserves UX contracts
r[molten.modularity.cli_shell.compatible_ux] CLI shell refactors MUST preserve existing command names, flags, canonical artifact outputs, and documented behavior unless a separate UX change owns the compatibility break.

#### Scenario: Existing command still works
- GIVEN a documented CLI command covered by the refactor
- WHEN the command is run with previously valid inputs
- THEN it accepts the same flags and emits equivalent canonical artifacts or documented diagnostics

### Requirement: CLI core extraction carries positive and negative tests
r[molten.modularity.cli_shell.tests] CLI core extraction SHOULD include positive tests for valid command inputs and negative tests for malformed, missing, stale, unsupported, or denied inputs.

#### Scenario: CLI core test matrix covers denial
- GIVEN a command core controls admission, artifact generation, or mutation planning
- WHEN reviewers inspect the tests
- THEN at least one positive path and at least one denial or malformed-input path are covered


### Requirement: Pure core crate boundary
r[molten.modularity.core_crate.pure_foundation] The repository SHOULD provide a dedicated core crate for foundational deterministic types and pure validation that can be tested without adapters, CLI commands, filesystem state, network services, clocks, or process execution.

#### Scenario: Core validator runs in memory
- GIVEN a core validation API for refs, envelopes, bounds, or identity inputs
- WHEN a unit test calls the API with in-memory valid data
- THEN the API returns a structured pass result without reading files, spawning processes, opening network connections, reading clocks, or rendering CLI output

#### Scenario: Core rejects malformed data before adapters
- GIVEN malformed refs, missing required fields, invalid bounds, or unsupported states
- WHEN a core validation API evaluates the input
- THEN it returns a structured error or deny result before any adapter or CLI shell is invoked

### Requirement: Core dependency direction is enforced
r[molten.modularity.core_crate.dependency_direction] The core crate MUST NOT depend on adapter crates, CLI modules, filesystem traversal, process execution, environment reads, wall-clock reads, Iroh, Redb, Wasmtime, Steel execution, or live Nickel evaluation.

#### Scenario: Adapter dependency is blocked
- GIVEN a proposed core crate change imports an adapter or CLI dependency
- WHEN dependency-boundary validation runs
- THEN validation fails or records the violation before release evidence is promoted

#### Scenario: Root crate re-export preserves compatibility
- GIVEN a foundational item moves into the core crate
- WHEN existing callers use the previous root-crate module path during the migration window
- THEN compatibility re-exports continue to compile until a separate public API change removes them

### Requirement: Core extraction carries positive and negative evidence
r[molten.modularity.core_crate.validation] Core extraction changes SHOULD include positive and negative tests or fixtures for each moved invariant, or record an explicit exemption when the moved surface is a re-export only.

#### Scenario: Positive and negative moved invariant tests exist
- GIVEN a moved core invariant is executable
- WHEN reviewers inspect the change evidence
- THEN valid examples and invalid examples are both covered by focused tests or fixtures


### Requirement: Policy authoring, export, runtime consumption, and freshness are layered
r[molten.project.policy_boundary.layered_policy] Repository policy systems SHOULD separate authoring-time contracts, deterministic generated exports, runtime consumption of checked artifacts, and freshness validation.

#### Scenario: Policy layer responsibility is clear
- GIVEN a policy source, generated policy artifact, runtime admission path, or freshness check
- WHEN reviewers inspect the implementation
- THEN the artifact is assigned to authoring, export, runtime consumption, or freshness validation

### Requirement: Runtime does not invoke live policy tooling as authority
r[molten.project.policy_boundary.runtime_no_live_tooling] Runtime admission MUST NOT invoke Nickel evaluation, Cairn policy export, or policy tooling availability as live authority; it MUST consume checked exports, canonical refs, or policy-gate receipts.

#### Scenario: Runtime consumes checked policy
- GIVEN runtime admission requires policy data
- WHEN admission evaluates a request
- THEN it consumes checked policy exports, explicit policy refs, or canonical policy-gate receipts without running Nickel or Cairn policy commands

#### Scenario: Live policy tooling attempt is rejected
- GIVEN runtime code attempts to run Nickel or Cairn policy tooling to decide live authority
- WHEN boundary validation runs
- THEN validation fails or records the violation before release evidence is promoted

### Requirement: Generated policy freshness is validated
r[molten.project.policy_boundary.fresh_generated_policy] Generated policy artifacts SHOULD be validated for freshness against reviewed source contracts and the current expected schema before promotion.

#### Scenario: Fresh generated policy passes
- GIVEN reviewed policy source and generated policy artifacts match the current schema
- WHEN freshness validation runs
- THEN validation passes and records the source and generated artifact identities

#### Scenario: Stale generated policy fails
- GIVEN generated policy JSON is missing required schema fields, has duplicate ids, contains stale refs, or diverges from reviewed source
- WHEN freshness validation runs
- THEN validation fails before runtime or release evidence treats the artifact as current

### Requirement: Policy boundary has positive and negative fixtures
r[molten.project.policy_boundary.tests] Policy boundary changes SHOULD include positive fixtures for valid fresh exports and negative fixtures for stale generated policy, missing schema fields, duplicate ids, bad refs, or runtime live-tooling violations.

#### Scenario: Missing schema field fixture fails
- GIVEN a generated policy fixture omits a required current schema field
- WHEN policy freshness validation runs
- THEN the fixture fails for the expected missing-field invariant


### Requirement: Public modules are classified
r[molten.modularity.public_api.classified_surface] Public root-crate modules and re-exports SHOULD be classified as stable API, compatibility alias, internal implementation, or generated/test support before modularity refactors remove or hide them.

#### Scenario: Public export has intent
- GIVEN a public module, compatibility alias, or re-export in the root crate
- WHEN the API inventory is reviewed
- THEN the export is classified with its intended stability and migration status

#### Scenario: Unclassified public export blocks removal
- GIVEN a public export lacks a stability classification
- WHEN a refactor proposes to remove, rename, or hide it
- THEN the change records a classification first or defers the removal to a compatibility-owned change

### Requirement: Stable API surface is intentional
r[molten.modularity.public_api.intentional_exports] The repository SHOULD expose a small intentional API or prelude for stable consumers and SHOULD avoid making implementation modules public solely for internal convenience.

#### Scenario: Preferred API is discoverable
- GIVEN a consumer needs stable Molten core types or constructors
- WHEN they inspect public documentation or the root API module
- THEN the preferred stable import path is identifiable without relying on compatibility aliases

#### Scenario: Compatibility alias is not preferred
- GIVEN a compatibility alias remains for migration
- WHEN new internal code is added
- THEN it uses the preferred stable or crate-internal path instead of expanding use of the compatibility alias

### Requirement: Implementation visibility is minimized
r[molten.modularity.public_api.visibility] Implementation details SHOULD be private or `pub(crate)` unless they are required for the reviewed public API, canonical artifact parsing, fixture support, or compatibility migration.

#### Scenario: Internal helper is hidden
- GIVEN an implementation helper has no external compatibility requirement
- WHEN modularity cleanup touches its owning module
- THEN the helper becomes private or `pub(crate)` while existing tests and consumers continue to compile

#### Scenario: Required public helper records reason
- GIVEN an implementation-looking helper must remain public
- WHEN reviewers inspect the API inventory
- THEN the reason is recorded as stable API, fixture support, generated boundary, or compatibility migration

### Requirement: API surface changes are validated
r[molten.modularity.public_api.validation] Public API tightening SHOULD include compile checks, tests, or policy checks proving intended public paths still work and accidental implementation exports do not expand.

#### Scenario: Intended public API compiles
- GIVEN the preferred public API surface after cleanup
- WHEN compile or UI checks run
- THEN representative imports and calls for the intended surface succeed

#### Scenario: Accidental public surface is detected
- GIVEN a new implementation module is exported publicly without classification
- WHEN API surface validation runs
- THEN validation fails or records the unclassified export before release evidence is promoted


### Requirement: Local persistence uses explicit store ports
r[molten.modularity.store_ports.explicit_port] Repository-owned domain cores that require local indexes or durable metadata SHOULD express persistence needs through explicit store ports, deterministic plans, or typed query/result records rather than direct Redb access.

#### Scenario: Domain core returns store plan
- GIVEN a domain operation needs to read or update local persistent indexes
- WHEN the pure core evaluates admitted in-memory inputs
- THEN it returns a structured store query or mutation plan without opening Redb or beginning a transaction

#### Scenario: Direct Redb access is contained
- GIVEN a module imports Redb types or opens Redb transactions
- WHEN reviewers inspect the module after migration
- THEN the code is inside an approved store adapter or records a staged-migration exemption

### Requirement: Redb adapter owns database mechanics
r[molten.modularity.store_ports.redb_adapter] Redb table definitions, database open/create, transaction lifetimes, migration checks, and low-level Redb error mapping MUST be owned by the Redb adapter shell, not by pure domain cores.

#### Scenario: Adapter maps Redb result
- GIVEN a Redb read or write operation completes
- WHEN the adapter returns to the domain shell
- THEN the result is expressed as typed store data, canonical diagnostics, or structured adapter error

### Requirement: Admission precedes store writes
r[molten.modularity.store_ports.admission_before_write] Store mutation plans MUST be produced only after domain admission succeeds, and denied requests MUST NOT begin Redb write transactions or mutate local indexes.

#### Scenario: Denied mutation has empty plan
- GIVEN missing authority, stale evidence, malformed refs, resource denial, or unsupported store profile
- WHEN the domain planner evaluates the request
- THEN it returns a deny result with no write transaction or mutation plan

### Requirement: Store port extraction has positive and negative tests
r[molten.modularity.store_ports.tests] Store port refactors SHOULD include positive tests for admitted plans and negative tests for denied, malformed, stale, unavailable, or conflicting inputs.

#### Scenario: Store tests cover denial
- GIVEN a store port boundary is introduced
- WHEN reviewers inspect the tests
- THEN valid admitted inputs and denied inputs are both covered, including proof that denied inputs do not request writes


### Requirement: Root dependencies are classified by layer
r[molten.project.modularity.dependency_classes] Cargo dependencies SHOULD be classified by their intended layer: core, codec, policy-evidence, runtime, adapter, CLI, test, or integration.

#### Scenario: Dependency purpose is reviewable
- GIVEN a dependency in the repository manifest
- WHEN reviewers inspect dependency classification
- THEN the dependency has an intended layer and migration status

### Requirement: Minimal core build excludes adapters
r[molten.project.modularity.minimal_core_build] The project SHOULD provide a minimal core or core-plus-codec build surface that excludes transport, storage, executor, CLI, live policy tooling, and integration dependencies not required for pure validation.

#### Scenario: Minimal core compiles without adapters
- GIVEN the minimal core build target
- WHEN focused validation builds it
- THEN it succeeds without requiring Iroh, Redb, Wasmtime, Steel execution, Nickel CLI/tooling, NixOS VM, dogfood, or live transport dependencies

#### Scenario: Adapter leak is reported
- GIVEN the minimal core surface imports an adapter dependency
- WHEN dependency validation runs
- THEN validation fails or reports the adapter leak before release evidence is promoted

### Requirement: Default build compatibility is preserved
r[molten.project.modularity.default_compatibility] Introducing dependency or feature boundaries MUST preserve existing default developer build and CLI behavior unless a separate compatibility change owns the break.

#### Scenario: Default build still includes integrations
- GIVEN a developer runs the existing default build or CLI test path
- WHEN feature boundaries are introduced
- THEN the default path continues to include required adapters and integration features

### Requirement: Dependency-boundary checks include positive and negative coverage
r[molten.project.modularity.dependency_tests] Dependency-boundary changes SHOULD include positive checks for the intended minimal surface and negative checks or diagnostics for forbidden adapter leakage.

#### Scenario: Dependency checks cover leak
- GIVEN a forbidden adapter import appears in the minimal core surface
- WHEN the dependency check runs
- THEN it reports the offending dependency and owning layer


### Requirement: Source splits use semantic boundaries
r[molten.modularity.semantic_modules.named_boundaries] Rust source modules SHOULD prefer semantically named submodules over ordinal `include!` shards when the code is repository-owned and manually reviewed.

#### Scenario: Named module reveals review boundary
- GIVEN a large repository-owned module selected for modularity cleanup
- WHEN the module is split or reorganized
- THEN each new source file name identifies a domain responsibility such as model, codec, admission, receipts, store, runner, shell, or tests
- AND existing public module paths remain available unless a separate public API change owns the break

#### Scenario: Ordinal shard expansion is blocked
- GIVEN a repository-owned module still using ordinal body shards
- WHEN new manually reviewed behavior is added to that module
- THEN the behavior is placed in a semantic module or the change records an explicit generated-code or staged-migration exemption

### Requirement: Semantic splits preserve functional core boundaries
r[molten.modularity.semantic_modules.functional_core] Semantic module refactors MUST keep deterministic core logic separate from filesystem, process, network, clock, environment, and CLI rendering effects.

#### Scenario: Pure decision moves behind in-memory API
- GIVEN parsing, validation, admission, or receipt-decision logic is moved during a semantic split
- WHEN focused tests exercise the moved logic
- THEN the tests can call the core with in-memory inputs and observe structured outputs without standing up adapters or CLI commands

#### Scenario: IO leakage is rejected
- GIVEN a module marked as a functional core after the split
- WHEN reviewers inspect that module
- THEN filesystem traversal, process execution, network IO, wall-clock reads, environment reads, and direct stdout or stderr rendering are absent or explicitly moved to the shell

### Requirement: Remaining ordinal shards are explicit exemptions
r[molten.modularity.semantic_modules.exemptions] Remaining ordinal shards MAY exist only when they are generated, externally constrained, or staged for later migration, and the owner SHOULD record the exemption near the entry point or in the change evidence.

#### Scenario: Generated shard remains reviewable
- GIVEN a generated or machine-partitioned shard remains after cleanup
- WHEN reviewers inspect the module boundary
- THEN the exemption identifies why semantic naming was not applied and what stable generated input or review artifact owns the content
