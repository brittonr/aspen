## ADDED Requirements

### Requirement: Haskell reference boundary
r[molten.haskell_patterns.reference_boundary] The system MUST treat Haskell, GHC, QuickCheck, Hedgehog, STM, mtl, lens/optics, parser combinators, typeclasses, and related ecosystem ideas as non-normative prior art only, and MUST NOT claim Haskell language, runtime, package, API, or library compatibility.

#### Scenario: Documentation cites Haskell without compatibility claim
r[molten.haskell_patterns.reference_boundary.no_compat]
- GIVEN Molten documentation or design material that cites a Haskell pattern
- WHEN it describes the adopted idea
- THEN it states the Molten-specific Preserves, Rust, policy, evidence, effect, and runtime boundaries rather than claiming Haskell compatibility

### Requirement: Pure core and effectful shell
r[molten.haskell_patterns.pure_core_effect_shell] Core runtime semantics MUST be represented as deterministic transitions over explicit state, events, policy/effect decisions, profile identity, and canonical inputs, while filesystem, network, process, clock, random, database, Steel, Wasm, and adapter effects occur only through admitted effect handlers outside the pure core.

#### Scenario: Core transition has no ambient effects
r[molten.haskell_patterns.pure_core_effect_shell.no_ambient]
- GIVEN a core transition fixture with explicit state, event, policy decision, and effect response inputs
- WHEN the transition is evaluated
- THEN the result is derived only from those inputs and does not read ambient filesystem, network, clock, random, process, database, Steel, Wasm, or thread-scheduling state

### Requirement: Capability-style effect handlers
r[molten.haskell_patterns.effect_capability_handlers] The system MUST model executable effect needs as explicit manifests and bind them through handler profiles, analogous to capability dictionaries, so each effect request identifies the effect id, execution id, handler profile, capabilities, policy refs, replay sequence metadata, and canonical input hash.

#### Scenario: Same effect manifest binds to replay profile
r[molten.haskell_patterns.effect_capability_handlers.replay]
- GIVEN an artifact declaring clock, random, storage, trace, and dataspace effects
- WHEN it runs under a replay handler profile
- THEN each effect request is checked against the manifest and recorded sequence, live external side effects are denied, and recorded responses are injected with canonical trace and receipt evidence

### Requirement: Property laws and shrinking
r[molten.haskell_patterns.property_laws] The system MUST use Hegel property-law suites for replay identity, snapshot stability, transaction rollback, adapter behavior, scheduler total order, Preserves canonical roundtrip, pattern binding order, redaction stability, authority monotonicity, revocation cleanup, and resource bounds, and MUST store generated inputs, seeds, shrink paths, and final shrunk counterexamples as canonical Preserves fixtures.

#### Scenario: Shrunk law failure becomes deterministic fixture
r[molten.haskell_patterns.property_laws.shrunk_fixture]
- GIVEN a generated property test that fails and shrinks
- WHEN the harness records the failure
- THEN the generation seed, shrink path, final input, expected law, traces, receipts, and replay identity are stored as Preserves artifacts that can be rerun without the generator

### Requirement: STM-style transactional turns
r[molten.haskell_patterns.transactional_turns] Actor turns MUST behave as atomic transactions over runtime-visible state: actor state deltas, dataspace assertions/retractions, messages, effect intents, resource consumption, and trace/evidence records are staged, validated, admitted, and committed as a unit or rolled back as a unit.

#### Scenario: Denied effect rolls back staged turn
r[molten.haskell_patterns.transactional_turns.denied_rollback]
- GIVEN an actor turn that stages a state update, assertion, message, and effect intent
- WHEN policy denies the effect intent before commit
- THEN none of the staged state, assertion, message, effect, or committed trace/evidence records become visible as committed runtime state

### Requirement: Effect reserve/commit/abort records
r[molten.haskell_patterns.effect_reserve_commit_abort] Adapters that require preparation before turn commit MUST represent preparation, commit, and abort as canonical reserve/commit/abort effect records so replay can prove no invisible partial side effect occurred.

#### Scenario: Reserved storage write aborts visibly
r[molten.haskell_patterns.effect_reserve_commit_abort.abort]
- GIVEN a storage adapter that reserves a write during turn admission
- WHEN the turn later fails before commit
- THEN the adapter records an abort effect and the harness can replay that no committed storage mutation became visible

### Requirement: Adapter law conformance
r[molten.haskell_patterns.adapter_law_conformance] Evidence-bearing adapters MUST publish and pass law/conformance suites for their effect boundary, including laws for storage, blobs/chunks, policy, network/Iroh, Wasm hostcalls, Steel orchestration, scheduler, and replay handlers where applicable.

#### Scenario: Blob adapter proves content integrity law
r[molten.haskell_patterns.adapter_law_conformance.blob_integrity]
- GIVEN a blob or chunk-store adapter under conformance test
- WHEN it accepts a content ref and payload
- THEN the adapter verifies canonical content identity before delivery and emits trace/receipt evidence for accepted and rejected payloads

### Requirement: Newtype and phantom-authority discipline
r[molten.haskell_patterns.newtype_authority] The system MUST use distinct Rust and Preserves representations for semantically different ids, refs, capabilities, secrets, evidence, profiles, and staged/committed state markers so values with the same low-level representation cannot be accidentally interchanged across runtime, policy, storage, or evidence boundaries.

#### Scenario: Receipt ref cannot stand in for capability ref
r[molten.haskell_patterns.newtype_authority.no_ref_confusion]
- GIVEN a runtime request that expects a capability ref
- WHEN a caller supplies a receipt ref with the same textual or hash-like representation shape
- THEN the request is rejected by type or schema validation before policy admission treats it as authority

### Requirement: Typed protocol and state-machine gates
r[molten.haskell_patterns.typed_protocol_state] Protocol and workflow state machines MUST be represented with typed DTOs and pure or Trellis-backed transition gates, and illegal transitions MUST be rejected before side effects with replayable diagnostics.

#### Scenario: Illegal transition is denied before effect
r[molten.haskell_patterns.typed_protocol_state.illegal_transition]
- GIVEN a protocol endpoint in state `awaiting-worker`
- WHEN a transition valid only for state `ready` is requested
- THEN the transition is denied before adapter side effects and the diagnostic identifies expected and actual canonical states

### Requirement: Golden canonical traces
r[molten.haskell_patterns.golden_canonical_traces] Golden tests MUST use versioned canonical Preserves trace, receipt, snapshot, and state-hash artifacts as the normative expected output, while text output is only a rendered view, and golden updates MUST require review or policy receipts.

#### Scenario: Golden trace update is reviewed
r[molten.haskell_patterns.golden_canonical_traces.reviewed_update]
- GIVEN a change to a golden canonical trace artifact
- WHEN the update is admitted
- THEN the system records old and new refs, review or policy authority, reason class, compatibility notes, and receipt evidence

### Requirement: Parser-combinator-style deterministic DSLs
r[molten.haskell_patterns.parser_combinator_dsls] The system SHOULD use composable deterministic parsers for Preserves pattern subsets, transcript stanzas, policy fixtures, oracle predicates, redaction selectors, and canonical diff filters, and parser outputs that cross runtime or evidence boundaries MUST have canonical DTOs or Preserves representations.

#### Scenario: Oracle parser has canonical output
r[molten.haskell_patterns.parser_combinator_dsls.oracle_canonical]
- GIVEN a textual oracle predicate in a transcript or harness suite
- WHEN the harness parses it
- THEN the parser returns a deterministic AST with stable error spans and a canonical representation used for hashing and report evidence

### Requirement: Optic-inspired redaction and diffs
r[molten.haskell_patterns.optic_redaction_diff] The system MUST provide structured traversal for Preserves values and runtime DTOs to support redaction, canonical diffs, evidence filtering, report rendering, and selective snapshot comparison without string-search mutation or silent data deletion.

#### Scenario: Redaction preserves evidence shape
r[molten.haskell_patterns.optic_redaction_diff.redaction_marker]
- GIVEN a report containing a confidential capability-bearing field
- WHEN a user without reveal authority exports the report
- THEN structured traversal replaces the field with a canonical redaction marker, records redaction evidence, and preserves safe surrounding canonical structure

### Requirement: Strictness and resource guards
r[molten.haskell_patterns.strictness_resource_guards] The system MUST make deferred work, lazy artifact fetches, queued messages, traces, property generation, transcript rendering, report export, content materialization, and snapshot materialization subject to explicit resource budgets and deterministic yield, cancellation, or failure points.

#### Scenario: Deferred work exceeds deterministic budget
r[molten.haskell_patterns.strictness_resource_guards.deferred_budget]
- GIVEN a harness run with a declared budget for deferred actions and trace output
- WHEN actor execution accumulates more deferred work or trace bytes than the budget allows
- THEN the runtime fails deterministically with resource diagnostics and canonical evidence rather than growing unbounded state
