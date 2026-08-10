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

### Requirement: Repository config paths are relocatable

r[molten.project.config_portability.relocatable_paths] Repository-owned development, hook, Nix, and validation configuration SHOULD avoid user-specific absolute paths and MUST allow required sibling repository paths to be supplied by reviewed workspace-relative defaults, flake inputs, or explicit environment variables.

#### Scenario: Common workspace checkout works without user-specific paths

- GIVEN Molten is checked out under a normal OnixResearch sibling workspace
- WHEN development hooks, Nix checks, or config validation resolve Cairn and private dependency source paths
- THEN they resolve through reviewed defaults or explicit environment variables
- AND reviewed config does not require a `/home/<user>/...` literal to run.

#### Scenario: User-specific path is rejected by config lint

- GIVEN a repo-owned config file introduces a hard-coded user home path for a required tool or sibling repository
- WHEN the config lint check runs in release-review mode
- THEN the check fails with a diagnostic naming the file and portability rule.

### Requirement: Release toolchains are pinned

r[molten.project.config_portability.toolchain_pin] Rust toolchain configuration used for release, CI, Nix checks, or canonical evidence SHOULD pin an exact toolchain identity and MUST NOT rely on a floating channel unless that channel is explicitly scoped to local exploratory use and excluded from release evidence.

#### Scenario: Pinned release toolchain passes

- GIVEN the release and Nix check toolchain is a dated Rust channel or exact toolchain identity
- WHEN config validation inspects the toolchain source
- THEN validation records the pinned identity as release-review evidence.

#### Scenario: Floating release nightly fails

- GIVEN release-scoped config uses a floating `nightly` Rust channel with no exact date or toolchain identity
- WHEN config validation runs in release-review mode
- THEN validation fails before formatter, Clippy, unit2nix, or test evidence can be treated as reproducible release evidence.

### Requirement: Cargo and Nix private source pins stay aligned

r[molten.project.config_portability.git_source_pin_drift] Molten SHOULD provide a deterministic check that compares private OnixResearch git dependency revisions in `Cargo.lock` with the Nix local-source map used for hermetic unit2nix builds.

#### Scenario: Matching source pins pass

- GIVEN Cargo.lock names private dependency revisions that match the Nix local-source map
- WHEN the source-pin drift check runs
- THEN it passes and reports the dependency names and revisions that were compared.

#### Scenario: Mismatched source pin fails

- GIVEN Cargo.lock names a private dependency revision that differs from the Nix local-source map
- WHEN the source-pin drift check runs
- THEN it fails closed with diagnostics naming the dependency, Cargo revision, and Nix revision.

### Requirement: Config lint is pure-core and shell-owned

r[molten.project.config_portability.config_lint] Config lint decisions SHOULD be computed by a deterministic pure core over explicit file records, while the shell owns filesystem discovery, environment lookup, command execution, and rendered diagnostics.

#### Scenario: Pure config lint accepts explicit inputs

- GIVEN in-memory config records with paths, toolchain channels, source pins, and profile refs
- WHEN the config lint core evaluates them
- THEN it returns pass or denial diagnostics without reading files, executing commands, consulting environment variables, or rendering stdout.

#### Scenario: Shell reports denied config

- GIVEN the shell reads repo config files and the pure core returns a denial
- WHEN the config lint command renders the result
- THEN it names the denied rule and source file while keeping runtime authority, policy, provenance, and source-gate decisions out of scope.

### Requirement: Repeated config values are named

r[molten.project.config_portability.named_config_constants] Long-lived Nix and test configuration SHOULD express VM addresses, attempt bounds, event limits, timeout values, profile names, and evidence-output paths through named constants or small modules when those values are part of reviewed behavior.

#### Scenario: Named config constant is review-visible

- GIVEN a VM address, retry bound, event limit, timeout, or evidence profile changes
- WHEN reviewers inspect the diff
- THEN the changed value is associated with a name describing its role rather than appearing only as an unexplained numeric or string literal.

#### Scenario: Refactor preserves check behavior

- GIVEN a Nix check is refactored to use named constants
- WHEN the check runs with the same semantic values
- THEN canonical evidence outputs and pass/deny behavior remain unchanged or the change records an explicit evidence migration note.

### Requirement: Effective config readback artifacts
r[molten.project.effective_config_readback.artifact] Molten SHOULD emit canonical effective-configuration readback artifacts that record schema metadata, normalized effective values, source traces, profile refs, override refs, default caveats, diagnostics, and checks. Effective-config artifact identity MUST be derived from canonical bytes using BLAKE3.

#### Scenario: Effective config has stable identity
- GIVEN the same checked profile inputs, CLI override inputs, and default policy
- WHEN Molten computes an effective-config readback twice
- THEN both readbacks have the same canonical BLAKE3 ref
- AND rendered text output is not used as the identity source.

#### Scenario: Hidden default is visible
- GIVEN an effective value comes from a local fixture default rather than a reviewed profile
- WHEN the readback artifact is emitted
- THEN the field source records the default origin and caveat
- AND release review can distinguish it from profile-backed configuration.

### Requirement: Config source traces are field-local
r[molten.project.effective_config_readback.source_trace] Each effective-config field SHOULD identify its source class, source ref or command input when available, override status, and caveats closely enough for reviewers to distinguish reviewed profile values from CLI overrides, environment-resolved shell inputs, ledger evidence, and fixture defaults.

#### Scenario: CLI override source is recorded
- GIVEN a profile value is overridden by an admitted CLI value
- WHEN effective-config readback runs
- THEN the field records both the profile source and the CLI override source
- AND diagnostics identify the override rule that admitted the value.

#### Scenario: Conflicting sources deny
- GIVEN two non-mergeable sources provide different values for a field that must be unique
- WHEN effective-config normalization runs
- THEN it denies with diagnostics naming the conflicting source classes and field.

### Requirement: Config validate, explain, diff, and fingerprint share a pure core
r[molten.project.effective_config_readback.cli_core] Molten SHOULD provide config validation/readback CLI commands whose decisions come from a deterministic pure core over explicit input records. The CLI shell MUST own filesystem reads, path resolution, environment lookup, artifact writing, and rendered diagnostics.

#### Scenario: Explain renders canonical readback
- GIVEN a valid effective-config input set
- WHEN an operator runs a config explain command
- THEN the command writes or references the canonical effective-config artifact
- AND the rendered explanation is a diagnostic view over that artifact.

#### Scenario: Diff uses normalized artifacts
- GIVEN two effective-config artifacts with different values or source traces
- WHEN an operator runs config diff
- THEN the diff is computed over normalized artifact fields
- AND diagnostics identify changed values, changed sources, and changed caveats.

### Requirement: Effective config readback is evidence-only
r[molten.project.effective_config_readback.evidence_only] Effective-config readback artifacts MUST NOT grant authority, policy admission, provenance trust, source-gate acceptance, resource rights, retention clearance, transport correctness, execution permission, or release eligibility by themselves.

#### Scenario: Readback cannot authorize mutation
- GIVEN a passing effective-config readback artifact
- WHEN a caller attempts to use it as the only evidence for install, run, delete, retention GC, live send, or policy mutation
- THEN the downstream gate denies unless the normal subsystem-specific receipts and authority are supplied independently.

### Requirement: Molten is a workload-neutral distributed-systems fabric
r[molten.fabric_boundary.fabric_identity] Molten MUST define its core as a workload-neutral distributed-systems fabric that supplies canonical communication, lifecycle, authority, resource, execution, durability, transport, scheduling, supervision, and evidence mechanisms without defining one database, replicated-log, scheduler, actor, or workflow semantic as the product-wide default.

#### Scenario: Database semantics remain extension-owned
- GIVEN a system extension implements transactions, conflict detection, shards, and replicas
- WHEN reviewers inspect the node-core contract
- THEN the fabric exposes only the general ports and lifecycle mechanisms required by that extension
- AND database transaction semantics are not treated as global Molten behavior.

#### Scenario: Non-database service uses the same fabric
- GIVEN a replicated log or distributed scheduler selects admitted fabric ports
- WHEN the service is installed
- THEN it can use the same transport, durability, membership, scheduling, policy, resource, and simulation mechanisms without adopting database semantics.

### Requirement: Fabric mechanisms and extension semantics are separated
r[molten.fabric_boundary.mechanism_semantics_separation] Molten MUST keep mechanism contracts in the fabric and workload semantics in system extensions. Moving extension-specific semantics into the node core MUST require a separate reviewed change that identifies the general invariant, compatibility impact, authority impact, and reference-system evidence.

#### Scenario: Extension-specific offset policy stays outside core
- GIVEN a replicated-log extension defines consumer offsets and retention rules
- WHEN its implementation uses fabric durable-state and scheduling ports
- THEN offset and retention semantics remain owned by the extension
- AND the fabric does not infer those semantics for other services.

#### Scenario: Hidden semantic promotion is rejected
- GIVEN a node-core change makes one extension's transaction, ordering, retry, or retention rule mandatory for unrelated services
- WHEN architecture validation evaluates the change
- THEN validation denies unless a separate fabric requirement justifies the general mechanism
- AND diagnostics identify the leaked extension semantic.

### Requirement: Extension tiers have distinct authority
r[molten.fabric_boundary.extension_tiers] Molten MUST distinguish sandboxed plugins, system extensions, and applications or workloads. System-extension authority for long-lived services, protocol ownership, durable state, timers, membership, placement, or consistency MUST NOT be inferred from ordinary plugin installation or artifact possession.

#### Scenario: System extension receives reviewed service authority
- GIVEN an artifact has a system-extension manifest, passing policy and provenance evidence, explicit port bindings, resource grants, and lifecycle admission
- WHEN the node activates the extension
- THEN only the declared system-extension capabilities become available
- AND ordinary plugin capabilities remain narrower.

#### Scenario: Sandboxed plugin cannot claim system authority
- GIVEN a sandboxed plugin declares a storage or network-shaped operation string
- WHEN it lacks a system-extension profile and matching capability grants
- THEN activation or hostcall admission denies protocol ownership, durable-state ownership, timers, membership, placement, and consistency access.

### Requirement: Fabric capability ports are canonical and fail closed
r[molten.fabric_boundary.port_registry] Molten MUST represent fabric ports with canonical descriptors and registry entries that bind port id, version, operation classes, input and output schema refs, authority requirements, resource requirements, determinism class, replay class, implementation profile, conformance refs, and non-claims. Unknown, duplicate, incompatible, disabled, or silently substituted ports MUST deny before extension activation.

#### Scenario: Compatible port binding passes
- GIVEN a system extension requires a supported transport port version with matching schemas, authority, resources, and conformance evidence
- WHEN activation resolves the port through the registry
- THEN the binding passes and emits a canonical binding receipt naming the selected implementation profile.

#### Scenario: Silent fallback is rejected
- GIVEN an extension requests an unsupported durable-state port version
- WHEN the registry contains another version or adapter with different semantics
- THEN activation denies instead of silently selecting the other port
- AND diagnostics identify the unsupported version and available reviewed profiles.

### Requirement: Fabric evidence is emitted at bounded semantic boundaries
r[molten.fabric_boundary.evidence_granularity] Molten MUST emit canonical evidence at declared trust, lifecycle, commit, checkpoint, failure, and operator-observation boundaries while allowing bounded aggregate evidence for internal hot-path operations. A fabric profile MUST NOT require one heavyweight receipt for every internal page read, packet, scheduler poll, or cache lookup unless a reviewed security or debugging profile explicitly selects that behavior.

#### Scenario: Commit boundary emits aggregate evidence
- GIVEN a system extension processes many internal storage and transport operations for one admitted commit
- WHEN the semantic commit completes
- THEN the extension emits evidence binding the admitted inputs, resulting state or output refs, relevant durable and quorum boundaries, and aggregate diagnostics
- AND internal operations need not each become independent authority receipts.

#### Scenario: Debug profile does not become production default
- GIVEN a diagnostic profile records every internal operation
- WHEN a production profile is selected
- THEN the diagnostic evidence granularity is not enabled implicitly
- AND production resource admission remains bounded by its reviewed profile.

### Requirement: Diverse reference systems demonstrate fabric sufficiency
r[molten.fabric_boundary.reference_system_exit_criteria] Molten SHOULD maintain capability and conformance matrices showing that a transactional key-value service, a replicated log, and a distributed scheduler can be implemented as system extensions without modifying node-core semantics or bypassing authority, resource, durability, transport, scheduling, or simulation ports.

#### Scenario: Three reference systems use common mechanisms
- GIVEN reviewed reference designs for the three service classes
- WHEN their required capabilities are compared
- THEN common needs map to fabric ports
- AND transaction isolation, log-offset behavior, and scheduling policy remain extension-specific.

#### Scenario: Missing general primitive is visible
- GIVEN a reference system requires behavior unavailable through reviewed fabric ports
- WHEN its conformance matrix is evaluated
- THEN the matrix reports the missing primitive
- AND the system cannot claim fabric conformance through direct ambient access.

### Requirement: Fabric non-claims are explicit
r[molten.fabric_boundary.non_claims] Molten MUST state that fabric descriptors, bindings, receipts, simulations, and reference-system matrices do not by themselves prove database correctness, global ordering, global consensus, transport delivery, durable persistence, Byzantine tolerance, protocol compatibility, production readiness, or extension semantic correctness.

#### Scenario: Port binding is not behavioral proof
- GIVEN an extension has passing bindings for transport, durability, membership, and consistency ports
- WHEN a release gate evaluates extension correctness
- THEN the bindings count only as mechanism and admission evidence
- AND separate implementation, conformance, simulation, and operational evidence remains required.

### Requirement: Fabric boundary validation covers positive and negative paths
r[molten.fabric_boundary.final_validation] Molten MUST include positive and negative validation for fabric identity, extension tiers, port registration, profile compatibility, evidence granularity, reference-system matrices, and non-claim enforcement.

#### Scenario: Valid fabric fixture passes
- GIVEN a system-extension fixture uses compatible registered ports and preserves extension-owned semantics
- WHEN focused validation runs
- THEN validation passes with canonical descriptor and binding refs.

#### Scenario: Semantic leakage fixture denies
- GIVEN a fixture grants system authority to an ordinary plugin or treats one reference service's semantics as global fabric behavior
- WHEN focused validation runs
- THEN validation denies with a tier or mechanism-semantics diagnostic.

### Requirement: Molten-owned packages declare AGPL

r[molten.project.license_boundary.package_metadata] Molten-owned Rust packages MUST declare `AGPL-3.0-or-later` in repository-owned package metadata.

#### Scenario: Package metadata is inspected

- GIVEN a distributor inspects Molten and `molten-core` package metadata
- WHEN the selected project license is read
- THEN each Molten-owned package MUST report `AGPL-3.0-or-later`.

### Requirement: License artifacts accompany source

r[molten.project.license_boundary.license_artifacts] Molten MUST ship the complete AGPL-3.0-or-later license text and MUST identify that third-party and vendored material remains governed by its original terms.

#### Scenario: Source archive is distributed

- GIVEN a source archive contains Molten-owned and vendored material
- WHEN a recipient reviews its license artifacts
- THEN the archive MUST include the AGPL text and MUST NOT represent vendored code as relicensed Molten-owned code.

### Requirement: Current documentation states the selected boundary

r[molten.project.license_boundary.documentation] Current Molten documentation MUST state the AGPL boundary and MUST NOT claim that the license selection revokes earlier grants or proves legal compliance in every jurisdiction.

#### Scenario: A reviewer reads the architecture boundary

- GIVEN current architecture documentation describes Molten and historical Aspen material
- WHEN licensing is discussed
- THEN it MUST distinguish project-owned AGPL source from separately licensed third-party material without retaining a contradictory permissive project declaration.

### Requirement: Generated package metadata remains fresh

r[molten.project.license_boundary.generated_metadata] Checked-in generated package metadata MUST agree with the repository-owned Cargo manifests for Molten package license expressions.

#### Scenario: A stale build plan retains the permissive expression

- GIVEN Cargo metadata declares AGPL while a generated Molten package row declares MIT or Apache
- WHEN freshness validation runs
- THEN validation MUST fail until the generated row is refreshed.

### Requirement: License boundary validation is deterministic

r[molten.project.license_boundary.final_validation] The repository MUST validate the selected package expressions, required license artifacts, current documentation, and absence of contradictory project-owned license declarations.

#### Scenario: A project-owned declaration drifts

- GIVEN one current project-owned metadata or documentation surface declares a conflicting project license
- WHEN the focused license audit runs
- THEN the audit MUST fail without treating dependency license expressions as project drift.

### Requirement: Release dependency source rows are typed
r[molten.project.reproducible_dependencies.contract] Molten MUST normalize release dependency declarations from repository-owned manifests, Cargo lock data, and Nix source inputs into typed rows containing package identity, source kind, source coordinate, immutable revision, transport policy, and release disposition.

#### Scenario: Complete source row validates
r[molten.project.reproducible_dependencies.fixtures.positive]
- GIVEN a release dependency has matching package, source, immutable revision, and transport policy across reviewed inputs
- WHEN dependency validation runs
- THEN the row MUST pass and preserve its exact source identity.

#### Scenario: Malformed source row fails
r[molten.project.reproducible_dependencies.fixtures.negative]
- GIVEN a row is missing a source coordinate or immutable revision, uses an unsupported source kind, or has conflicting package identity
- WHEN dependency validation runs
- THEN validation MUST fail with a deterministic source-row diagnostic.

### Requirement: Git dependencies use immutable release pins
r[molten.project.reproducible_dependencies.exact_pins] Every Git dependency in the Molten release closure MUST be bound to an immutable revision in the repository-owned dependency source of truth, and manifest, lockfile, and Nix source identities MUST agree.

#### Scenario: Floating Git source fails release validation
- GIVEN a release dependency is bound only to a branch, tag, moving reference, or unpinned SSH URL
- WHEN release dependency validation runs
- THEN validation MUST fail before the dependency contributes to release evidence.

#### Scenario: Lock drift fails release validation
- GIVEN the manifest or Nix source pin names one immutable revision and the Cargo lockfile resolves another
- WHEN release dependency validation runs
- THEN validation MUST fail with a revision-drift diagnostic naming both identities.

### Requirement: Dependency drift validation has a pure core
r[molten.project.reproducible_dependencies.drift_validation] Molten MUST compare normalized dependency rows in pure deterministic logic while filesystem reads, manifest parsing, lockfile loading, and Nix evaluation remain in shell or adapter code.

#### Scenario: Equivalent row ordering is stable
- GIVEN equivalent dependency rows arrive in different input order
- WHEN drift validation runs
- THEN diagnostics and receipt identity MUST remain deterministic.

### Requirement: Canonical standalone Valence is required
r[molten.project.reproducible_dependencies.canonical_valence] Molten MUST consume `valence-core` semantics from the exact standalone Valence revision accepted by the archived Valence integrity-hardening and Octet cutover receipts.

#### Scenario: Standalone canonical source passes
- GIVEN Aspen's dependency row matches the accepted standalone Valence source and revision
- WHEN dependency validation runs
- THEN the Valence source contribution MUST pass.

#### Scenario: Octet-hosted Valence is rejected
- GIVEN Aspen resolves `valence-core` from Octet's hosted or legacy compatibility package
- WHEN dependency validation runs
- THEN validation MUST fail with a non-canonical-source diagnostic.

### Requirement: Canonical Valence package identity is unique
r[molten.project.reproducible_dependencies.unique_valence_identity] Molten MUST reject a resolved dependency graph containing different source identities under the same canonical `valence-core` package name and version.

#### Scenario: Duplicate semantic providers fail
- GIVEN standalone and Octet-hosted Valence implementations both resolve under canonical package identity
- WHEN package-graph validation runs
- THEN validation MUST fail and identify each source.

### Requirement: Cross-repository cutover evidence is required
r[molten.project.reproducible_dependencies.cross_repo_dependencies] Molten MUST require archived receipts for Valence integrity hardening and Octet standalone cutover before accepting the canonical Valence migration as release evidence.

#### Scenario: Missing upstream receipt blocks migration
- GIVEN either required upstream archive receipt is absent or mismatched
- WHEN Aspen evaluates migration readiness
- THEN the canonical Valence migration MUST remain blocked.

### Requirement: AGPL is an allowed typed distribution profile
r[molten.project.agpl_distribution_profile.contract] Molten MAY use an AGPL distribution profile, and the profile MUST record the selected license identity, notice artifacts, source coordinate, immutable revision, and project-required corresponding-source or source-export evidence.

#### Scenario: Complete AGPL profile passes project policy
- GIVEN a release declares AGPL and supplies the configured notice and source evidence
- WHEN distribution-profile validation runs
- THEN the profile MUST pass without treating AGPL itself as a blocker.

#### Scenario: Missing configured evidence fails
- GIVEN an AGPL distribution profile omits a required notice, source coordinate, immutable revision, or configured source-export artifact
- WHEN distribution-profile validation runs
- THEN validation MUST fail with a deterministic missing-evidence diagnostic.

### Requirement: Distribution boundary is documented
r[molten.project.agpl_distribution_profile.docs] Molten documentation MUST state that AGPL is an accepted project choice and that distribution-profile evidence records project-policy facts rather than providing legal advice or proving compliance in every jurisdiction.

#### Scenario: License choice is not misclassified
- GIVEN a reviewer reads release documentation
- WHEN AGPL distribution is described
- THEN the documentation MUST distinguish accepted license choice from missing project-required release evidence.

### Requirement: Reproducible dependency verification rail
r[molten.project.reproducible_dependencies.final_validation] The change MUST include positive and negative evidence for exact source pins, lock agreement, canonical Valence selection, unique package identity, and the AGPL distribution profile.

#### Scenario: Drift and missing evidence fail closed
- GIVEN floating, drifting, duplicate, non-canonical, or incomplete fixtures
- WHEN focused validation runs
- THEN every invalid fixture MUST fail while complete exact-pin fixtures pass.

### Requirement: Flake-check CI is checked in
r[molten.project.reproducible_dependencies.flake_check_ci] Molten MUST include a checked-in CI workflow for this remediation whose verification command is `nix flake check`.

#### Scenario: CI uses the scoped verification rail
- GIVEN a change is evaluated by checked-in CI
- WHEN the remediation workflow runs
- THEN it MUST execute `nix flake check` without requiring a separate expanded CI command matrix in this change.

### Requirement: Inherited Tracey debt has an exact baseline
r[molten.project.inherited_tracey_debt.baseline] Molten MUST store the sorted inherited uncovered requirement set with typed metadata, an exact count, and a BLAKE3 digest.

#### Scenario: Baseline matches the source tree
- GIVEN the accepted requirements, admitted evidence roots, and checked-in baseline
- WHEN the inherited Tracey debt guard evaluates them
- THEN the actual uncovered set equals the baseline and its typed identity.

### Requirement: Verified marker defects are repaired directly
r[molten.project.inherited_tracey_debt.marker_repair] Molten MUST repair malformed or stale requirement markers only when accepted requirement text and existing source evidence establish the exact identity.

#### Scenario: An inline requirement marker is not silently omitted
- GIVEN an accepted requirement with verified source evidence
- WHEN the marker placement prevents requirement discovery
- THEN the marker moves to the accepted standalone form without changing requirement semantics.

### Requirement: Traceability growth fails closed
r[molten.project.inherited_tracey_debt.growth_denial] Molten MUST deny new uncovered requirements, dangling evidence references, malformed baselines, duplicate entries, unsorted entries, and unreviewed baseline reductions.

#### Scenario: A new uncovered requirement appears
- GIVEN a source tree with one requirement absent from the reviewed baseline and no evidence reference
- WHEN the debt guard evaluates the tree
- THEN it fails and identifies the unexpected requirement.

### Requirement: The debt guard has positive and negative tests
r[molten.project.inherited_tracey_debt.fixtures] Molten MUST test exact baseline admission and MUST test malformed markers, duplicate baselines, unsorted baselines, new gaps, stale gaps, and dangling references.

#### Scenario: Exact baseline passes
- GIVEN a valid requirement set, valid evidence references, and an exact sorted baseline
- WHEN the self-test evaluates the inputs
- THEN admission passes.

#### Scenario: Traceability drift fails
- GIVEN a new gap, stale gap, duplicate baseline, unsorted baseline, malformed marker, or dangling reference
- WHEN the self-test evaluates the inputs
- THEN admission fails with the expected diagnostic class.

### Requirement: Validation evidence is reproducible
r[molten.project.inherited_tracey_debt.validation] Molten MUST record the scanner profile, source revision, baseline identity, focused commands, and final lifecycle receipts.

#### Scenario: A reviewer repeats validation
- GIVEN the archived validation evidence and source revision
- WHEN the reviewer runs the recorded commands
- THEN the guard, metadata, lifecycle, and repository results can be compared with the recorded identities.

### Requirement: The debt baseline does not claim coverage
r[molten.project.inherited_tracey_debt.non_claims] Molten MUST state that the baseline does not exempt uncovered requirements or prove marker truth, behavior, release readiness, or whole-system correctness.

#### Scenario: Baseline admission passes
- GIVEN the actual inherited uncovered set equals the baseline
- WHEN the guard emits a passing result
- THEN the result states that inherited requirements remain uncovered and require direct evidence.

### Requirement: Inherited Tracey debt has a complete classification inventory
r[molten.project.inherited_tracey_classification.inventory] Molten MUST classify every identifier in the reviewed inherited Tracey debt baseline in a deterministic inventory.

#### Scenario: Every baseline entry appears once
- GIVEN the reviewed baseline and accepted project specifications
- WHEN the classifier runs
- THEN every baseline identifier appears exactly once in the classification inventory.

### Requirement: Classification defaults remain conservative
r[molten.project.inherited_tracey_classification.conservative_default] Molten MUST classify a baseline entry as `accepted-implementation-unestablished` unless reviewed evidence establishes implementation, replacement, obsolescence, or invalidity.

#### Scenario: No evidence supports a stronger class
- GIVEN an accepted requirement without a direct source reference or lifecycle disposition
- WHEN the classifier emits its row
- THEN it uses the conservative class and makes no implementation claim.

### Requirement: Duplicate accepted definitions fail classification
r[molten.project.inherited_tracey_classification.duplicate_denial] Molten MUST reject classification when a baseline identifier has no accepted definition or has more than one accepted definition.

#### Scenario: Duplicate identifier is present
- GIVEN two accepted specifications define the same baseline identifier
- WHEN the classifier runs
- THEN it fails with both definition locations and does not write a passing inventory.

### Requirement: Classification output is grouped deterministically
r[molten.project.inherited_tracey_classification.deterministic_grouping] Molten MUST group classification rows by accepted specification path and source area with deterministic ordering.

#### Scenario: Input order changes
- GIVEN the same baseline and definitions in a different discovery order
- WHEN the classifier runs
- THEN the emitted inventory remains byte-identical.

### Requirement: Classification has positive and negative fixtures
r[molten.project.inherited_tracey_classification.fixtures] Molten MUST test valid inventory generation and reject missing definitions, duplicate definitions, malformed baselines, and foreign namespaces.

#### Scenario: Invalid classification input is tested
- GIVEN a duplicate accepted definition or malformed baseline
- WHEN the negative fixture runs
- THEN classification fails for the expected invariant.

### Requirement: Classification preserves non-claims
r[molten.project.inherited_tracey_classification.non_claims] Molten MUST state that classification does not establish implementation, replacement, obsolescence, invalidity, behavioral correctness, or release readiness.

#### Scenario: Operator reads the inventory
- GIVEN a passing classification report
- WHEN the operator reviews its meaning
- THEN the report states its bounded routing purpose and non-claims.

### Requirement: Proven inherited links use direct production evidence
r[molten.project.inherited_tracey_classification.verified_repair] Molten MUST remove an inherited debt entry only when existing production logic or documentation and a relevant test directly support the accepted requirement.

#### Scenario: High-confidence repair is applied
- GIVEN a requirement has matching production behavior and a positive or negative test
- WHEN its direct source markers are added
- THEN the exact baseline shrinks and the classifier reports only the remaining entries.
