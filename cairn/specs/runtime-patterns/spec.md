# Runtime Patterns Specification

## Purpose

Defines Molten's law-oriented runtime-pattern discipline: Haskell-inspired prior art boundaries, pure core/effectful shell separation, explicit effect handler profiles, transactional turns, typed authority/ref distinctions, property-law fixtures, adapter conformance, typed protocol gates, deterministic parsers, structured redaction/diff traversal, and bounded resource behavior.

## Requirements

### Requirement: Haskell reference boundary
r[molten.haskell_patterns.reference_boundary] Molten MUST treat Haskell, GHC, Cabal, Stack, Hackage, QuickCheck, Hedgehog, STM, mtl, lens/optics, parser combinators, typeclasses, laziness, and related ecosystem ideas as non-normative prior art only. Molten MUST describe the adopted Rust, Preserves, policy, evidence, effect, and runtime contracts rather than claiming Haskell language, runtime, package, API, or library compatibility.

#### Scenario: Documentation cites Haskell without compatibility claim
- GIVEN Molten documentation or design material cites a Haskell pattern
- WHEN it describes the adopted idea
- THEN it states the Molten-specific Preserves, Rust, policy, evidence, effect, and runtime boundaries
- AND it does not claim Haskell compatibility.

### Requirement: Pure core and effectful shell
r[molten.haskell_patterns.pure_core_effect_shell] Molten MUST keep semantic runtime decisions as deterministic transformations over explicit state, events, policy or effect decisions, handler-profile identity, and canonical Preserves inputs. Filesystem, network, process, clock, random, database, Steel, Wasm, and adapter effects MUST enter through admitted effect requests, responses, or receipts outside the pure transition boundary.

#### Scenario: Core transition has no ambient effects
- GIVEN a core transition fixture with explicit state, event, policy decision, and effect response inputs
- WHEN the transition is evaluated
- THEN the result is derived only from those inputs
- AND ambient filesystem, network, clock, random, process, database, Steel, Wasm, and thread-scheduling state do not decide the semantic transition.

### Requirement: Capability-style effect handlers
r[molten.haskell_patterns.effect_capability_handlers] Molten MUST model executable effect needs as explicit effect manifests and bind them through handler profiles. Effect requests and binding receipts MUST identify effect id, execution id, handler profile, capabilities, policy refs, resource/authority refs where applicable, replay sequence metadata where applicable, and canonical input/request refs.

#### Scenario: Same manifest binds to replay profile
- GIVEN an artifact declaring storage, trace, dataspace, or other admitted effects
- WHEN it runs under a replay handler profile
- THEN each request is checked against the manifest and recorded sequence evidence
- AND live external side effects are denied unless represented by admitted replay evidence.

### Requirement: Runtime-pattern schemas are versioned
r[molten.haskell_patterns.schema_versioning] Runtime-pattern evidence that crosses a runtime, storage, policy, or evidence boundary MUST use versioned canonical Preserves schemas or typed Rust wrappers with canonical ref validation.

#### Scenario: Handler profile has a versioned record
- GIVEN a handler profile or effect request crosses an evidence boundary
- WHEN Molten renders it
- THEN the value uses a versioned Preserves record and canonical refs
- AND unsupported or malformed refs fail closed before admission.

### Requirement: STM-style transactional turns
r[molten.haskell_patterns.transactional_turns] Molten actor turns MUST stage runtime-visible state deltas, assertions, retractions, messages, effect intents, resource records, and trace/evidence records before commit. Commit MUST be admitted atomically by the relevant predicate or policy checks, and denial MUST roll back staged runtime-visible state.

#### Scenario: Denied effect rolls back staged turn
- GIVEN an actor turn that stages a state update, assertion, message, and effect intent
- WHEN policy or predicate admission denies the effect before commit
- THEN none of the staged state, assertion, message, effect, or committed trace/evidence records become visible as committed runtime state.

### Requirement: Invisible pre-commit adapter effects are forbidden
r[molten.haskell_patterns.effect_reserve_commit_abort] Molten MUST NOT allow adapters to perform invisible pre-commit side effects during staged turns. Adapters that require preparation before turn commit MUST either deny before side effect or use an admitted reserve/commit/abort receipt extension; this completed slice records the fail-closed boundary and leaves a general reserve/commit/abort adapter API as a future extension.

#### Scenario: Prepared write aborts visibly or never occurs
- GIVEN an adapter needs to prepare a write during turn admission
- WHEN the turn later fails before commit
- THEN Molten either has no external mutation to expose
- OR emits canonical abort evidence from an admitted future reserve/commit/abort extension.

### Requirement: Newtype and phantom-authority discipline
r[molten.haskell_patterns.newtype_authority] Molten MUST use distinct Rust types, enums, or canonical Preserves field/schema distinctions for semantically different ids, refs, capabilities, secrets, evidence, profiles, and staged/committed state markers so values with the same low-level representation are not treated as interchangeable authority.

#### Scenario: Receipt ref cannot stand in for capability ref
- GIVEN a runtime request expects a capability ref
- WHEN a caller supplies a receipt ref with the same textual or hash-like representation shape
- THEN the request is rejected by type, schema, or admission validation before policy treats it as authority.

### Requirement: Type distinctions do not grant authority
r[molten.haskell_patterns.no_type_authority_confusion] Molten MUST treat typed ids and refs as identity and schema discipline only. Authority still requires explicit policy, capability, resource, live-ref, or receipt evidence, and tests or validators SHOULD cover accidental id/ref/profile interchange where implemented.

#### Scenario: Typed wrapper lacks authority evidence
- GIVEN a caller holds a well-formed typed content or receipt ref
- WHEN it attempts an effect requiring a capability
- THEN Molten denies unless separate capability and policy evidence is admitted.

### Requirement: Property laws cover implemented invariants
r[molten.haskell_patterns.property_laws] Molten SHOULD use Hegel or deterministic generated tests for implemented runtime laws such as replay identity, snapshot stability, transaction rollback, adapter behavior, scheduler ordering, Preserves canonical roundtrip, redaction stability, authority monotonicity, revocation cleanup, and resource bounds.

#### Scenario: Generated invariant has deterministic evidence
- GIVEN a bounded generated property test for an implemented invariant
- WHEN the test evaluates a generated case
- THEN the fixture inputs, canonical refs, decisions, and diagnostics are deterministic enough to reproduce the failing or passing case.

### Requirement: Shrunk replay fixtures are canonical when persisted
r[molten.haskell_patterns.shrink_replay_fixtures] Molten MUST store any persisted generated input, replay seed, shrink path, or final shrunk counterexample that crosses an evidence boundary as canonical Preserves fixture data. Automatic export of every shrunk counterexample remains a future harness extension unless a suite explicitly implements it.

#### Scenario: Persisted counterexample is rerunnable
- GIVEN a property suite persists a shrunk counterexample
- WHEN the counterexample is imported into a replay or repro bundle
- THEN the seed, shrink path, final input, expected law, traces, and receipt refs are canonical Preserves data and can be rerun without the generator.

### Requirement: Adapter law conformance
r[molten.haskell_patterns.adapter_law_conformance] Evidence-bearing adapters SHOULD publish and pass law/conformance suites for their implemented effect boundary, including applicable laws for storage, blobs/chunks, policy, network or Iroh, Wasm hostcalls, Steel orchestration, scheduler, replay handlers, and denial-before-side-effect behavior.

#### Scenario: Blob adapter proves content integrity law
- GIVEN a blob or chunk-store adapter under conformance test
- WHEN it accepts a content ref and payload
- THEN the adapter verifies canonical content identity before delivery
- AND emits trace or receipt evidence for accepted and rejected payloads.

### Requirement: Golden artifacts are canonical evidence
r[molten.haskell_patterns.golden_canonical_traces] Molten SHOULD use versioned canonical Preserves trace, receipt, snapshot, fixture, and state-hash artifacts as normative golden evidence where golden outputs are required. Text output is only a rendered view, and golden updates SHOULD bind old/new refs and reviewer, policy, or migration evidence.

#### Scenario: Golden trace update is reviewed
- GIVEN a change to a golden canonical trace artifact
- WHEN the update is admitted
- THEN the update records old and new refs, reason class, compatibility notes, and review or policy evidence.

### Requirement: Typed protocol and state-machine gates
r[molten.haskell_patterns.typed_protocol_state] Molten MUST represent protocol and workflow state machines with typed DTOs, canonical Preserves values, and pure or Trellis-backed transition gates. Illegal transitions MUST be rejected before side effects with replayable diagnostics.

#### Scenario: Illegal transition is denied before effect
- GIVEN a protocol endpoint in state `awaiting-worker`
- WHEN a transition valid only for state `ready` is requested
- THEN the transition is denied before adapter side effects
- AND the diagnostic identifies expected and actual canonical states.

### Requirement: Parser-combinator-style deterministic DSLs
r[molten.haskell_patterns.parser_combinator_dsls] Molten SHOULD build deterministic parser components for Preserves pattern subsets, transcript stanzas, policy fixtures, oracle predicates, redaction selectors, and canonical diff filters. Parser outputs crossing runtime or evidence boundaries MUST have deterministic ASTs, error spans, and canonical DTO or Preserves representations.

#### Scenario: Oracle parser has canonical output
- GIVEN a textual oracle predicate in a transcript or harness suite
- WHEN the harness parses it
- THEN the parser returns a deterministic AST with stable error diagnostics
- AND the output has a canonical representation for hashing and report evidence.

### Requirement: Optic-inspired redaction and diffs
r[molten.haskell_patterns.optic_redaction_diff] Molten MUST use structured traversal over Preserves values and runtime DTOs for redaction, canonical diffs, evidence filtering, report rendering, and selective snapshot comparison where those operations affect evidence. String-only mutation MUST NOT silently delete confidential data or authority-bearing refs.

#### Scenario: Redaction preserves evidence shape
- GIVEN a report containing a confidential capability-bearing field
- WHEN a user without reveal authority exports the report
- THEN structured traversal replaces the field with a canonical redaction marker
- AND records redaction evidence while preserving safe surrounding canonical structure.

### Requirement: Strictness and resource guards
r[molten.haskell_patterns.strictness_resource_guards] Molten MUST make deferred work, queued messages, traces, property generation, transcript rendering, report export, content materialization, and snapshot materialization subject to explicit resource budgets, bounds, deterministic cancellation, or deterministic failure points where implemented.

#### Scenario: Deferred work exceeds deterministic budget
- GIVEN a harness run with a declared budget for deferred actions and trace output
- WHEN actor execution accumulates more deferred work or trace bytes than the budget allows
- THEN the runtime fails deterministically with resource diagnostics and canonical evidence rather than growing unbounded state.

### Requirement: Pure transition tests
r[molten.haskell_patterns.pure_transition_tests] Molten SHOULD include tests proving representative core transitions run from explicit inputs and receipts rather than ambient filesystem, network, process, clock, random, Steel, Wasm, database, or scheduler state.

#### Scenario: Transition test uses explicit input state
- GIVEN a runtime transition unit test with explicit state and event inputs
- WHEN the transition is evaluated twice
- THEN both runs produce the same canonical refs and decisions without ambient observations.

### Requirement: Handler profile tests
r[molten.haskell_patterns.handler_profile_tests] Molten SHOULD test that the same effect manifest can bind to local, record, replay, chaos, profiling, pure, and production profiles where those profiles are implemented, and that policy/evidence differences remain explicit.

#### Scenario: Profile changes evidence not authority
- GIVEN the same effect manifest is bound under local and production profiles
- WHEN requests are admitted
- THEN each receipt names the profile and supporting evidence
- AND no profile grants ambient authority without policy/capability/resource refs.

### Requirement: Transaction rollback tests
r[molten.haskell_patterns.transaction_rollback_tests] Molten SHOULD test that failed, denied, or predicate-rejected turns roll back staged state, assertions, messages, effect intents, resource records, and traces for implemented turn surfaces.

#### Scenario: Rollback leaves state unchanged
- GIVEN a pending turn with staged actions
- WHEN the turn is denied before commit
- THEN the before and after snapshots match
- AND rollback evidence explains why no staged action became visible.

### Requirement: Redaction and diff tests
r[molten.haskell_patterns.redaction_diff_tests] Molten SHOULD test that redaction and evidence filtering preserve safe canonical structure, emit redaction evidence, hide plaintext or protected refs, and produce deterministic first-divergence or diff diagnostics where implemented.

#### Scenario: Redaction test hides plaintext
- GIVEN a value containing confidential text and public surrounding structure
- WHEN the redacted export is generated
- THEN plaintext is absent, a redaction marker is present, and the public canonical structure remains parseable.

### Requirement: Production soak network diagnostics observability
r[molten.prod_soak.network_diagnostics_observability] Molten SHOULD bind network diagnostics, connectivity probes, watcher snapshots, metrics snapshots, relay latency, direct/relay path status, and resource-pressure refs into production-soak receipts.

#### Scenario: Soak reports relay latency degradation
- GIVEN a production-soak run observes relay latency above the configured diagnostic threshold
- WHEN the final soak receipt is emitted
- THEN it records degraded network diagnostics with relay-latency refs and resource-pressure refs
- AND it does not claim broad production transport correctness from the degraded run.

#### Scenario: Network diagnostics remain separate from side-effect gates
- GIVEN a soak run has passing network diagnostics
- WHEN node-control, job execution, retention, provenance, or coordination side effects are evaluated
- THEN those side effects still require their normal authority, policy, resource, provenance, source-gate, retention, and operation receipts independently of the diagnostics pass.

### Requirement: Hegel proof-law catalog
r[molten.haskell_patterns.hegel_proof_properties.catalog] Molten SHOULD maintain an explicit Hegel RS proof-law catalog for core evidence invariants that are better checked by generated inputs than by single examples.

#### Scenario: Proof law is traceable
- GIVEN a Hegel RS property suite for a core evidence invariant
- WHEN traceability scans proof coverage
- THEN the suite is linked to the requirement id for that invariant.

### Requirement: Canonical ref stability property
r[molten.haskell_patterns.hegel_proof_properties.canonical_ref_stability] Core evidence code SHOULD have Hegel RS properties showing identical semantic inputs produce identical canonical refs and semantic binding changes produce changed refs or denial.

#### Scenario: Same generated input has same ref
- GIVEN Hegel RS generates a bounded canonical receipt input
- WHEN the receipt is rendered twice
- THEN both renderings produce the same canonical ref.

### Requirement: Traceability decision law
r[molten.haskell_patterns.hegel_proof_properties.traceability_decision_law] Traceability core SHOULD have Hegel RS properties proving the manifest decision is pass exactly when required positive coverage, required negative coverage, and stale-reference constraints are satisfied under policy.

#### Scenario: Missing generated negative coverage denies
- GIVEN Hegel RS generates a changed evidence-bearing requirement with positive coverage and no negative coverage
- WHEN the manifest is built
- THEN the manifest decision is deny with a missing-negative diagnostic.

### Requirement: Deny monotonicity property
r[molten.haskell_patterns.hegel_proof_properties.deny_monotonicity] Evidence validation SHOULD have Hegel RS properties proving that adding stale, malformed, mismatched, or tampered evidence cannot turn a denied proof into pass and cannot silently satisfy an unrelated requirement.

#### Scenario: Stale generated receipt cannot satisfy coverage
- GIVEN a generated coverage set and a stale receipt for a deleted requirement
- WHEN validation runs
- THEN validation reports stale evidence or excludes it by explicit policy without creating pass coverage.

### Requirement: Diagnostic evidence non-pass property
r[molten.haskell_patterns.hegel_proof_properties.diagnostic_nonpass_law] Gate validation SHOULD have Hegel RS properties proving diagnostic-only evidence cannot satisfy pass gates.

#### Scenario: Generated diagnostic receipt denied as pass
- GIVEN Hegel RS generates a diagnostic-only receipt shape
- WHEN a pass-evidence gate validates it
- THEN the gate denies pass evidence.

### Requirement: Replay compares canonical refs property
r[molten.haskell_patterns.hegel_proof_properties.replay_ref_law] Replay and drift checks SHOULD have Hegel RS properties proving semantic comparisons use canonical refs and declared variance, not rendered logs.

#### Scenario: Rendered log drift is not semantic drift
- GIVEN generated artifacts with the same canonical refs and different rendered diagnostic text allowed by variance
- WHEN drift comparison runs
- THEN the semantic decision follows canonical refs and declared variance.

### Requirement: Shrunk proof counterexamples are canonical
r[molten.haskell_patterns.hegel_proof_properties.shrink_fixture_receipts] Persisted Hegel RS counterexamples that cross proof, replay, or release boundaries MUST be represented as canonical fixture data with seed, shrink path, input, expected law, and relevant receipt refs.

#### Scenario: Persisted counterexample reruns
- GIVEN a shrunk Hegel RS counterexample saved for review
- WHEN it is imported into a repro or proof fixture
- THEN the fixture can rerun the exact generated input without ambient generator state.

### Requirement: Hegel property coverage appears in manifests
r[molten.haskell_patterns.hegel_proof_properties.coverage_manifest] Hegel RS proof-law suites SHOULD appear in traceability coverage manifests as positive or negative evidence for their linked requirements.

#### Scenario: Property suite covers invariant
- GIVEN a Hegel RS suite proving a canonical ref invariant
- WHEN traceability coverage is generated
- THEN the suite's verification receipt appears as evidence for the invariant requirement.

### Requirement: Hegel proof-law documentation
r[molten.haskell_patterns.hegel_proof_properties.docs] Contributor documentation SHOULD explain how to write bounded Hegel RS proof properties, name generator bounds, persist counterexamples, and link property evidence to requirements.

#### Scenario: Contributor adds generated proof law
- GIVEN a contributor adds a core evidence invariant
- WHEN they follow the documentation
- THEN they add Hegel RS positive and negative generated coverage with traceability refs.

### Requirement: External pilot network and resource bounds are canonical
r[molten.external_live_pilot_soak.network_resource_bounds] External pilot soak receipts SHOULD bind canonical network diagnostics, connectivity observations, resource envelope checks, replayability caveats, and degradation thresholds before a pilot decision can pass.

#### Scenario: Resource envelope breach denies pilot
- GIVEN a multi-host pilot run whose queue depth, delivery latency, memory, storage, or network resource evidence exceeds the configured pilot threshold
- WHEN the pilot decision validates the run
- THEN the decision denies or records the run as degraded outside pass scope
- AND logs alone cannot override the canonical resource evidence.

### Requirement: Retention readback remains non-destructive in pilot soak
r[molten.external_live_pilot_soak.retention_readback_boundary] External pilot soak workflows SHOULD bind retention readback, clearance review, or GC-plan evidence for cleanup-sensitive artifacts while denying destructive retention execution unless normal retention plan/apply/execute gates are separately present.

#### Scenario: Retention review does not authorize deletion
- GIVEN pilot soak evidence includes a retention candidate readback bundle
- WHEN a destructive cleanup operation is requested
- THEN the readback bundle alone is insufficient
- AND normal retention admission, apply, execute, tombstone, and audit evidence remains required.

### Requirement: External pilot validation covers positive and negative paths
r[molten.external_live_pilot_soak.validation] External pilot soak readiness SHOULD include deterministic local tests or fixtures for the decision law, negative boundary denials, and release-readback caveats before relying on operator-managed live evidence.

#### Scenario: Negative pilot fixture denies
- GIVEN a generated or fixture pilot evidence set with one required child receipt missing or stale
- WHEN the pilot decision validator runs
- THEN it emits a deny decision with an actionable missing-evidence diagnostic.

### Requirement: Syndicate traces are canonicalized as Molten evidence
r[molten.syndicate_dataspace.trace_evidence] Molten SHOULD convert adopted Syndicate trace observations into canonical Molten trace, turn, or parity evidence records. The records MUST bind the triggering event ref, committed action refs, actor or facet owner refs, route refs, and replayability status. Incomplete or live-only traces MUST remain diagnostic-only.

#### Scenario: Complete trace becomes replayable evidence
- GIVEN a Syndicate reference-harness run records the triggering event, committed assertions, retractions, messages, and owner refs
- WHEN Molten canonicalizes the trace
- THEN the trace evidence binds those refs with replayability status `recorded`
- AND replay can compare the trace by canonical refs.

#### Scenario: Incomplete trace is diagnostic-only
- GIVEN a Syndicate trace observation lacks a required committed action ref or owner ref
- WHEN Molten emits trace evidence
- THEN the evidence is marked diagnostic-only
- AND it cannot satisfy pass gates that require replayable turn evidence.
