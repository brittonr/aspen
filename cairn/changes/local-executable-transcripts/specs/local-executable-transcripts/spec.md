# local executable transcripts Delta Spec

## ADDED Requirements

### Requirement: Define canonical `transcript-artifact-v1` records with markdown source refs, parsed stanza refs, dependency closure, handler profile, policy/capability/revocation refs, seed/config refs, expected refs, and checks
r[molten.local_transcripts.artifact_dto] Define canonical `transcript-artifact-v1` records with markdown source refs, parsed stanza refs, dependency closure, handler profile, policy/capability/revocation refs, seed/config refs, expected refs, and checks.

### Requirement: Define canonical `transcript-stanza-v1` records for `molten-cli`, `preserves`, `artifact`, `policy`, `expect`, and `comment` stanzas
r[molten.local_transcripts.stanza_dto] Define canonical `transcript-stanza-v1` records for `molten-cli`, `preserves`, `artifact`, `policy`, `expect`, and `comment` stanzas.

### Requirement: Support bounded stanza modifiers for expected error, known bug, hidden output, skip, required feature, seed, and handler profile override
r[molten.local_transcripts.modifier_dto] Support bounded stanza modifiers for expected error, known bug, hidden output, skip, required feature, seed, and handler profile override.

### Requirement: Document that Unison transcripts are prior art only and Molten does not adopt UCM syntax, codebase semantics, hash formats, or typechecker behavior
r[molten.local_transcripts.no_ucm_compat] Document that Unison transcripts are prior art only and Molten does not adopt UCM syntax, codebase semantics, hash formats, or typechecker behavior.

### Requirement: Parse markdown fenced blocks into deterministic stanza records while preserving prose/comment anchors for rendering
r[molten.local_transcripts.markdown_parser] Parse markdown fenced blocks into deterministic stanza records while preserving prose/comment anchors for rendering.

### Requirement: Treat canonical source bytes and parsed stanza records as identity, never local paths, mtimes, cwd, env vars, or wall-clock time
r[molten.local_transcripts.canonical_source_identity] Treat canonical source bytes and parsed stanza records as identity, never local paths, mtimes, cwd, env vars, or wall-clock time.

### Requirement: Render transcript artifacts and run results back to markdown, omitting `hide` output from display while preserving evidence refs
r[molten.local_transcripts.render_docs] Render transcript artifacts and run results back to markdown, omitting `hide` output from display while preserving evidence refs.

### Requirement: Emit canonical diagnostics for unknown stanza kinds, malformed modifiers, oversized stanza payloads, and unsupported ambient shell blocks
r[molten.local_transcripts.parser_errors] Emit canonical diagnostics for unknown stanza kinds, malformed modifiers, oversized stanza payloads, and unsupported ambient shell blocks.

### Requirement: Implement a fresh-state deterministic runner with isolated registry, ledger, typed-storage, cache, and scratch roots
r[molten.local_transcripts.fresh_runner] Implement a fresh-state deterministic runner with isolated registry, ledger, typed-storage, cache, and scratch roots.

### Requirement: Implement a restricted `molten-cli` argument-vector dispatcher for admitted local test commands without falling back to a host shell
r[molten.local_transcripts.restricted_cli_dispatch] Implement a restricted `molten-cli` argument-vector dispatcher for admitted local test commands without falling back to a host shell.

### Requirement: Compare canonical expectations for Preserves value refs, artifact/cache/storage/schema refs, receipt kind/decision, diagnostics, output absence, and trace-pattern refs
r[molten.local_transcripts.expectation_engine] Compare canonical expectations for Preserves value refs, artifact/cache/storage/schema refs, receipt kind/decision, diagnostics, output absence, and trace-pattern refs.

### Requirement: Add a `save` mode for inspection while keeping `fork` and `in-place` denied or placeholder-only in this slice
r[molten.local_transcripts.saved_state] Add a `save` mode for inspection while keeping `fork` and `in-place` denied or placeholder-only in this slice.

### Requirement: Emit and parse canonical `transcript-run-receipt-v1` records for run start, stanza outcomes, render, cache-hit/cache-miss, denial, expected failure, known bug, and final result
r[molten.local_transcripts.run_receipt_dto] Emit and parse canonical `transcript-run-receipt-v1` records for run start, stanza outcomes, render, cache-hit/cache-miss, denial, expected failure, known bug, and final result.

### Requirement: Deny production effects unless a stanza/transcript declares an admitted handler profile and policy/capability evidence
r[molten.local_transcripts.effect_admission] Deny production effects unless a stanza/transcript declares an admitted handler profile and policy/capability evidence.

### Requirement: Ensure hidden/noisy output affects only rendering and remains present in canonical receipts/evidence
r[molten.local_transcripts.hidden_evidence] Ensure hidden/noisy output affects only rendering and remains present in canonical receipts/evidence.

### Requirement: Classify transcript artifacts and transcript run receipts in the local evidence ledger
r[molten.local_transcripts.ledger_classification] Classify transcript artifacts and transcript run receipts in the local evidence ledger.

### Requirement: Use `eval-cache` operation `transcript-run` with keys binding transcript artifact refs, dependency closure, handler profile, policy/capability/revocation refs, runner/tool version, seed/config refs, and expected refs
r[molten.local_transcripts.cache_key] Use `eval-cache` operation `transcript-run` with keys binding transcript artifact refs, dependency closure, handler profile, policy/capability/revocation refs, runner/tool version, seed/config refs, and expected refs.

### Requirement: Admit cache hits only for deterministic transcript tiers and deny semantic use of `production-effectful-trace-only` entries
r[molten.local_transcripts.cache_hit_admission] Admit cache hits only for deterministic transcript tiers and deny semantic use of `production-effectful-trace-only` entries.

### Requirement: Bind cache hit/miss/stale-deny receipts into final transcript run receipts
r[molten.local_transcripts.cache_receipts] Bind cache hit/miss/stale-deny receipts into final transcript run receipts.

### Requirement: Expose dependency/policy invalidation hooks for transcript-derived cache entries
r[molten.local_transcripts.cache_invalidation_hooks] Expose dependency/policy invalidation hooks for transcript-derived cache entries.

### Requirement: Add `molten test transcript parse` and `run` commands with full ref display and receipt/output file options
r[molten.local_transcripts.cli_parse_run] Add `molten test transcript parse` and `run` commands with full ref display and receipt/output file options.

### Requirement: Add `show` and `render` commands for transcript artifacts and run receipts
r[molten.local_transcripts.cli_show_render] Add `show` and `render` commands for transcript artifacts and run receipts.

### Requirement: Ensure CLI paths are local IO handles only and never part of transcript/cache identity except through canonical bytes
r[molten.local_transcripts.cli_no_path_identity] Ensure CLI paths are local IO handles only and never part of transcript/cache identity except through canonical bytes.

### Requirement: Emit canonical failure artifacts for parse/run/render failures
r[molten.local_transcripts.cli_failures] Emit canonical failure artifacts for parse/run/render failures.

### Requirement: Add tests for stanza ordering, modifier parsing, stable transcript refs, and malformed stanza denials
r[molten.local_transcripts.parse_tests] Add tests for stanza ordering, modifier parsing, stable transcript refs, and malformed stanza denials.

### Requirement: Add tests for a transcript that installs an artifact, runs a local command, and matches canonical output/receipt expectations
r[molten.local_transcripts.runner_tests] Add tests for a transcript that installs an artifact, runs a local command, and matches canonical output/receipt expectations.

### Requirement: Add tests for expected error and known bug stanza behavior
r[molten.local_transcripts.expected_error_tests] Add tests for expected error and known bug stanza behavior.

### Requirement: Add tests proving deterministic transcript runs can hit the evaluation cache and stale policy/cache mismatches deny
r[molten.local_transcripts.cache_tests] Add tests proving deterministic transcript runs can hit the evaluation cache and stale policy/cache mismatches deny.

### Requirement: Add tests that hidden output is omitted from rendered docs but preserved in canonical evidence
r[molten.local_transcripts.render_tests] Add tests that hidden output is omitted from rendered docs but preserved in canonical evidence.

### Requirement: Add Hegel properties for stanza ordering, fresh-run determinism, stable transcript identity, and denied ambient state access
r[molten.local_transcripts.property_tests] Add Hegel properties for stanza ordering, fresh-run determinism, stable transcript identity, and denied ambient state access.

