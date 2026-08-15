## Phase 1: Transcript artifact and syntax

- [x] [serial] r[molten.transcripts.artifact_model] Define transcript artifacts with markdown source, stanza list, dependency refs, handler profile, policy refs, and expected output refs.
- [x] [serial] r[molten.transcripts.stanza_kinds] Define initial stanza kinds for molten-config/config fixtures, molten-cli, preserves, policy, artifact, expect, and comment/prose blocks where implemented.
- [x] [serial] r[molten.transcripts.modifiers] Support stanza modifiers for expected error, known bug, hidden output, skip, required feature, seed, and handler profile.
- [x] [parallel] r[molten.transcripts.no_ucm_compat] Document that Unison transcripts are prior art only and Molten does not adopt UCM transcript compatibility.

## Phase 2: Runner modes and output

- [x] [serial] r[molten.transcripts.fresh_runner] Implement a fresh-state deterministic local transcript runner.
- [x] [serial] r[molten.transcripts.canonical_expectations] Compare expected canonical Preserves values, trace patterns, receipt patterns, diagnostics, and artifact ids.
- [x] [parallel] r[molten.transcripts.saved_state] Add optional save mode for inspection while keeping fork and in-place denied or placeholder-only.
- [x] [parallel] r[molten.transcripts.rendered_output] Render human-readable output from canonical run records without making text the only oracle.

## Phase 3: Policy, cache, and docs

- [x] [serial] r[molten.transcripts.run_receipts] Emit canonical receipts for transcript run start, stanza outcomes, expected failures, known bugs, cache hit/miss/stale, denial, and final result.
- [x] [serial] r[molten.transcripts.effect_admission] Deny transcript production effects unless declared in the handler profile and admitted by policy.
- [x] [parallel] r[molten.transcripts.eval_cache] Integrate deterministic transcript results with the evaluation cache.
- [x] [parallel] r[molten.transcripts.docs_render] Render transcript artifacts as documentation with hidden/noisy stanzas omitted from display but preserved in evidence.

## Phase 4: Tests

- [x] [serial] r[molten.transcripts.basic_tests] Add tests for a transcript that installs an artifact, runs a local command, and matches a canonical trace/output.
- [x] [serial] r[molten.transcripts.expected_error_tests] Add tests for expected error and known bug stanza behavior.
- [x] [parallel] r[molten.transcripts.reproducibility_tests] Add tests proving fresh deterministic runs produce the same output artifact id and stale cache/policy mismatches deny.
- [x] [parallel] r[molten.transcripts.property_tests] Add Hegel property tests for stanza ordering, output matching, fresh-run determinism, stable transcript identity, and denied ambient state access.
