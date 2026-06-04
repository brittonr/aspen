## Phase 1: Reference boundary and core/effect split

- [ ] [serial] r[molten.haskell_patterns.reference_boundary] Document Haskell as non-normative prior art and avoid claims of Haskell, GHC, Cabal, Stack, QuickCheck, Hedgehog, STM, mtl, lens, or parser-combinator API compatibility.
- [ ] [serial] r[molten.haskell_patterns.pure_core_effect_shell] Structure core runtime transitions as pure deterministic functions over explicit state, events, decisions, profile identity, and Preserves inputs, with effects handled only by adapters.
- [ ] [serial] r[molten.haskell_patterns.effect_capability_handlers] Model effect manifests and handler profiles as capability-style bindings for local, record, replay, chaos, profiling, pure, and production execution.
- [ ] [parallel] r[molten.haskell_patterns.schema_versioning] Ensure Haskell-inspired DTOs and effects have canonical Preserves schemas and version refs at runtime/evidence boundaries.

## Phase 2: Transactional turns and typed authority

- [ ] [serial] r[molten.haskell_patterns.transactional_turns] Implement STM-style staged actor turns that atomically commit or roll back state deltas, assertions, messages, effect intents, resource records, and trace/evidence records.
- [ ] [serial] r[molten.haskell_patterns.effect_reserve_commit_abort] Define reserve/commit/abort effect records for adapters that require pre-commit preparation so replay can prove no invisible partial side effects.
- [ ] [serial] r[molten.haskell_patterns.newtype_authority] Introduce distinct Rust and Preserves types for actor/session/vat/peer/run/turn ids, artifact/schema/policy/receipt/evidence/effect-log refs, capability/secret/content/snapshot/trace refs, and profile/state markers.
- [ ] [parallel] r[molten.haskell_patterns.no_type_authority_confusion] Add tests or lints preventing accidental interchange of ids, refs, secret/capability markers, staged/committed state, and deterministic/replay/record/non-replayable profiles.

## Phase 3: Laws, properties, and conformance

- [ ] [serial] r[molten.haskell_patterns.property_laws] Define Hegel property law suites for replay identity, snapshot stability, transaction rollback, adapter laws, scheduler total order, Preserves canonical roundtrip, pattern binding order, redaction stability, authority monotonicity, revocation cleanup, and resource bounds.
- [ ] [serial] r[molten.haskell_patterns.shrink_replay_fixtures] Store generated inputs, seeds, shrink paths, and final shrunk counterexamples as canonical Preserves fixtures suitable for deterministic replay and repro bundles.
- [ ] [serial] r[molten.haskell_patterns.adapter_law_conformance] Require evidence-bearing adapters to pass law/conformance suites for storage, blobs/chunks, policy, network/Iroh, Wasm hostcalls, Steel orchestration, scheduler, and replay handlers.
- [ ] [parallel] r[molten.haskell_patterns.golden_canonical_traces] Add golden canonical Preserves traces, receipts, snapshots, and state-hash artifacts with review receipts for updates.

## Phase 4: Protocols, parsers, and traversals

- [ ] [serial] r[molten.haskell_patterns.typed_protocol_state] Represent protocol and workflow states with typed DTOs and Trellis/pure transition gates, rejecting illegal transitions before side effects and emitting replay diagnostics.
- [ ] [serial] r[molten.haskell_patterns.parser_combinator_dsls] Build deterministic parser-combinator-style components for Preserves pattern subsets, transcript stanzas, policy fixtures, oracle predicates, redaction selectors, and canonical diff filters.
- [ ] [serial] r[molten.haskell_patterns.optic_redaction_diff] Implement optic-inspired structured traversal for redaction, canonical diffs, evidence filtering, report rendering, and selective snapshot comparison.
- [ ] [parallel] r[molten.haskell_patterns.strictness_resource_guards] Add resource/strictness tests for deferred work, queues, traces, property generators, transcript rendering, report exports, content materialization, and snapshot materialization.

## Phase 5: Integration tests

- [ ] [serial] r[molten.haskell_patterns.pure_transition_tests] Add tests proving representative core transitions run without filesystem, network, process, clock, random, Steel, Wasm, database, or thread-scheduling access.
- [ ] [serial] r[molten.haskell_patterns.handler_profile_tests] Add tests proving the same effect manifest can bind to local, record, replay, chaos, profiling, pure, and production profiles with policy/evidence differences made explicit.
- [ ] [serial] r[molten.haskell_patterns.transaction_rollback_tests] Add tests proving failed or denied turns roll back staged state, assertions, messages, effect intents, resource records, and traces as specified.
- [ ] [parallel] r[molten.haskell_patterns.redaction_diff_tests] Add tests proving redaction preserves safe canonical structure, emits redaction evidence, and produces minimal safe first-divergence diffs.
