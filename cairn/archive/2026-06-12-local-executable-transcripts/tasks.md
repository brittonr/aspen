## Phase 1: Canonical transcript model

- [x] [serial] r[molten.local_transcripts.artifact_dto] Define canonical `transcript-artifact-v1` records with markdown source refs, parsed stanza refs, dependency closure, handler profile, policy/capability/revocation refs, seed/config refs, expected refs, and checks.
- [x] [serial] r[molten.local_transcripts.stanza_dto] Define canonical `transcript-stanza-v1` records for `molten-cli`, `preserves`, `artifact`, `policy`, `expect`, and `comment` stanzas.
- [x] [serial] r[molten.local_transcripts.modifier_dto] Support bounded stanza modifiers for expected error, known bug, hidden output, skip, required feature, seed, and handler profile override.
- [x] [parallel] r[molten.local_transcripts.no_ucm_compat] Document that Unison transcripts are prior art only and Molten does not adopt UCM syntax, codebase semantics, hash formats, or typechecker behavior.

## Phase 2: Parser and rendering

- [x] [serial] r[molten.local_transcripts.markdown_parser] Parse markdown fenced blocks into deterministic stanza records while preserving prose/comment anchors for rendering.
- [x] [serial] r[molten.local_transcripts.canonical_source_identity] Treat canonical source bytes and parsed stanza records as identity, never local paths, mtimes, cwd, env vars, or wall-clock time.
- [x] [parallel] r[molten.local_transcripts.render_docs] Render transcript artifacts and run results back to markdown, omitting `hide` output from display while preserving evidence refs.
- [x] [parallel] r[molten.local_transcripts.parser_errors] Emit canonical diagnostics for unknown stanza kinds, malformed modifiers, oversized stanza payloads, and unsupported ambient shell blocks.

## Phase 3: Fresh local runner

- [x] [serial] r[molten.local_transcripts.fresh_runner] Implement a fresh-state deterministic runner with isolated registry, ledger, typed-storage, cache, and scratch roots.
- [x] [serial] r[molten.local_transcripts.restricted_cli_dispatch] Implement a restricted `molten-cli` argument-vector dispatcher for admitted local test commands without falling back to a host shell.
- [x] [serial] r[molten.local_transcripts.expectation_engine] Compare canonical expectations for Preserves value refs, artifact/cache/storage/schema refs, receipt kind/decision, diagnostics, output absence, and trace-pattern refs.
- [x] [parallel] r[molten.local_transcripts.saved_state] Add a `save` mode for inspection while keeping `fork` and `in-place` denied or placeholder-only in this slice.

## Phase 4: Receipts, policy, and effects

- [x] [serial] r[molten.local_transcripts.run_receipt_dto] Emit and parse canonical `transcript-run-receipt-v1` records for run start, stanza outcomes, render, cache-hit/cache-miss, denial, expected failure, known bug, and final result.
- [x] [serial] r[molten.local_transcripts.effect_admission] Deny production effects unless a stanza/transcript declares an admitted handler profile and policy/capability evidence.
- [x] [parallel] r[molten.local_transcripts.hidden_evidence] Ensure hidden/noisy output affects only rendering and remains present in canonical receipts/evidence.
- [x] [parallel] r[molten.local_transcripts.ledger_classification] Classify transcript artifacts and transcript run receipts in the local evidence ledger.

## Phase 5: Evaluation cache integration

- [x] [serial] r[molten.local_transcripts.cache_key] Use `eval-cache` operation `transcript-run` with keys binding transcript artifact refs, dependency closure, handler profile, policy/capability/revocation refs, runner/tool version, seed/config refs, and expected refs.
- [x] [serial] r[molten.local_transcripts.cache_hit_admission] Admit cache hits only for deterministic transcript tiers and deny semantic use of `production-effectful-trace-only` entries.
- [x] [parallel] r[molten.local_transcripts.cache_receipts] Bind cache hit/miss/stale-deny receipts into final transcript run receipts.
- [x] [parallel] r[molten.local_transcripts.cache_invalidation_hooks] Expose dependency/policy invalidation hooks for transcript-derived cache entries.

## Phase 6: CLI

- [x] [serial] r[molten.local_transcripts.cli_parse_run] Add `molten test transcript parse` and `run` commands with full ref display and receipt/output file options.
- [x] [serial] r[molten.local_transcripts.cli_show_render] Add `show` and `render` commands for transcript artifacts and run receipts.
- [x] [parallel] r[molten.local_transcripts.cli_no_path_identity] Ensure CLI paths are local IO handles only and never part of transcript/cache identity except through canonical bytes.
- [x] [parallel] r[molten.local_transcripts.cli_failures] Emit canonical failure artifacts for parse/run/render failures.

## Phase 7: Tests and properties

- [x] [serial] r[molten.local_transcripts.parse_tests] Add tests for stanza ordering, modifier parsing, stable transcript refs, and malformed stanza denials.
- [x] [serial] r[molten.local_transcripts.runner_tests] Add tests for a transcript that installs an artifact, runs a local command, and matches canonical output/receipt expectations.
- [x] [serial] r[molten.local_transcripts.expected_error_tests] Add tests for expected error and known bug stanza behavior.
- [x] [serial] r[molten.local_transcripts.cache_tests] Add tests proving deterministic transcript runs can hit the evaluation cache and stale policy/cache mismatches deny.
- [x] [parallel] r[molten.local_transcripts.render_tests] Add tests that hidden output is omitted from rendered docs but preserved in canonical evidence.
- [x] [parallel] r[molten.local_transcripts.property_tests] Add Hegel properties for stanza ordering, fresh-run determinism, stable transcript identity, and denied ambient state access.
