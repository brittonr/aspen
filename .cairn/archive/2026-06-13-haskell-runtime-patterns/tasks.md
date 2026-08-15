## Phase 1: Reference boundary and core/effect split

- [x] [serial] r[molten.haskell_patterns.reference_boundary] Document Haskell as non-normative prior art and avoid claims of Haskell, GHC, Cabal, Stack, QuickCheck, Hedgehog, STM, mtl, lens, or parser-combinator API compatibility.
- [x] [serial] r[molten.haskell_patterns.pure_core_effect_shell] Structure core runtime transitions as pure deterministic functions over explicit state, events, decisions, profile identity, and Preserves inputs, with effects handled only by adapters.
- [x] [serial] r[molten.haskell_patterns.effect_capability_handlers] Model effect manifests and handler profiles as capability-style bindings for local, record, replay, chaos, profiling, pure, and production execution where implemented.
- [x] [parallel] r[molten.haskell_patterns.schema_versioning] Ensure Haskell-inspired DTOs and effects have canonical Preserves schemas and version refs at runtime/evidence boundaries.

## Phase 2: Transactional turns and typed authority

- [x] [serial] r[molten.haskell_patterns.transactional_turns] Implement STM-style staged actor turns that atomically commit or roll back state deltas, assertions, messages, effect intents, resource records, and trace/evidence records for implemented turn surfaces.
- [x] [serial] r[molten.haskell_patterns.effect_reserve_commit_abort] Document and enforce the fail-closed boundary for adapters that would need pre-commit preparation; a general reserve/commit/abort adapter API remains a future explicit extension.
- [x] [serial] r[molten.haskell_patterns.newtype_authority] Introduce distinct Rust wrappers, enums, and Preserves schema distinctions for actor/session/vat/peer/run/turn ids, artifact/schema/policy/receipt/evidence/effect-log refs, capability/secret/content/snapshot/trace refs, and profile/state markers where implemented.
- [x] [parallel] r[molten.haskell_patterns.no_type_authority_confusion] Add tests or validators preventing accidental authority from ids, refs, secret/capability markers, staged/committed state, and deterministic/replay/record/non-replayable profiles.

## Phase 3: Laws, properties, and conformance

- [x] [serial] r[molten.haskell_patterns.property_laws] Define Hegel/property law coverage for replay identity, snapshot stability, transaction rollback, adapter laws, scheduler total order, Preserves canonical roundtrip, pattern binding order, redaction stability, authority monotonicity, revocation cleanup, and resource bounds where implemented.
- [x] [serial] r[molten.haskell_patterns.shrink_replay_fixtures] Require persisted generated inputs, seeds, shrink paths, and shrunk counterexamples to be canonical Preserves fixtures; automatic export remains future work unless a suite implements it.
- [x] [serial] r[molten.haskell_patterns.adapter_law_conformance] Require evidence-bearing adapters to publish law/conformance suites for storage, blobs/chunks, policy, network/Iroh, Wasm hostcalls, Steel orchestration, scheduler, and replay handlers where applicable.
- [x] [parallel] r[molten.haskell_patterns.golden_canonical_traces] Add or require golden canonical Preserves traces, receipts, snapshots, fixtures, and state-hash artifacts with review evidence for updates where implemented.

## Phase 4: Protocols, parsers, and traversals

- [x] [serial] r[molten.haskell_patterns.typed_protocol_state] Represent protocol and workflow states with typed DTOs and Trellis/pure transition gates, rejecting illegal transitions before side effects and emitting replay diagnostics.
- [x] [serial] r[molten.haskell_patterns.parser_combinator_dsls] Build deterministic parser-combinator-style components for Preserves pattern subsets, transcript stanzas, policy fixtures, oracle predicates, redaction selectors, and canonical diff filters where implemented.
- [x] [serial] r[molten.haskell_patterns.optic_redaction_diff] Implement optic-inspired structured traversal for redaction, canonical diffs, evidence filtering, report rendering, and selective snapshot comparison where implemented.
- [x] [parallel] r[molten.haskell_patterns.strictness_resource_guards] Add resource/strictness tests for deferred work, queues, traces, property generators, transcript rendering, report exports, content materialization, and snapshot materialization.

## Phase 5: Integration tests

- [x] [serial] r[molten.haskell_patterns.pure_transition_tests] Add tests proving representative core transitions run without filesystem, network, process, clock, random, Steel, Wasm, database, or thread-scheduling access.
- [x] [serial] r[molten.haskell_patterns.handler_profile_tests] Add tests proving the same effect manifest can bind to local, record, replay, chaos, profiling, pure, and production profiles with policy/evidence differences made explicit.
- [x] [serial] r[molten.haskell_patterns.transaction_rollback_tests] Add tests proving failed or denied turns roll back staged state, assertions, messages, effect intents, resource records, and traces as specified for implemented surfaces.
- [x] [parallel] r[molten.haskell_patterns.redaction_diff_tests] Add tests proving redaction preserves safe canonical structure, emits redaction evidence, and produces minimal safe first-divergence diffs where implemented.
