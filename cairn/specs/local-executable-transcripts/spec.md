# Local Executable Transcripts Specification

## Purpose

Defines the `local-executable-transcripts` capability.

## Requirements

### Requirement: Transcript artifact model
r[molten.transcripts.artifact_model] Molten MUST define executable transcript artifacts with markdown source identity, parsed stanza refs, dependency refs, handler profile, policy refs, capability/revocation refs, seed/config refs, and expected output refs.

#### Scenario: Transcript artifact binds source and stanzas
- GIVEN a markdown transcript with executable stanzas
- WHEN Molten parses it
- THEN the transcript artifact binds canonical source bytes, stanza records, dependency closure, policy refs, and expected refs.

### Requirement: Transcript stanza kinds
r[molten.transcripts.stanza_kinds] Molten MUST define initial stanza kinds for `molten-config`/config fixtures, `molten-cli`, `preserves`, `policy`, `artifact`, `expect`, and comment/prose blocks where implemented.

#### Scenario: Known stanza parses deterministically
- GIVEN a transcript containing `molten-cli`, `preserves`, `artifact`, `policy`, and `expect` fenced blocks
- WHEN Molten parses the transcript
- THEN each block becomes a canonical stanza record with stable ordering.

### Requirement: Transcript modifiers
r[molten.transcripts.modifiers] Molten MUST support bounded stanza modifiers for expected error, known bug, hidden output, skip, required feature, seed, and handler profile override where implemented.

#### Scenario: Expected error does not fail transcript
- GIVEN a stanza marked as an expected error
- WHEN the stanza returns the expected failure
- THEN the transcript records a passing outcome with diagnostics.

### Requirement: Unison transcripts are non-normative
r[molten.transcripts.no_ucm_compat] Molten MUST treat Unison transcripts as prior art only and MUST NOT claim UCM syntax, codebase semantics, hash format, or typechecker compatibility.

#### Scenario: Documentation avoids UCM compatibility claim
- GIVEN Molten transcript documentation mentions Unison transcripts
- WHEN it describes Molten transcript behavior
- THEN it states Molten-specific syntax, runtime, receipt, and cache semantics.

### Requirement: Fresh deterministic runner
r[molten.transcripts.fresh_runner] Molten MUST implement a fresh-state deterministic local transcript runner that uses isolated registry, ledger, typed-storage, cache, and scratch roots by default.

#### Scenario: Fresh run ignores ambient state
- GIVEN a transcript is run in fresh mode twice
- WHEN no transcript inputs change
- THEN both runs use isolated state and produce the same canonical transcript identity and deterministic outputs.

### Requirement: Canonical expectations
r[molten.transcripts.canonical_expectations] Molten MUST compare expected canonical Preserves values, trace patterns, receipt patterns, diagnostics, output absence, and artifact/cache/storage/schema refs rather than relying only on terminal text.

#### Scenario: Receipt expectation matches canonical record
- GIVEN an `expect` stanza names a receipt kind and decision
- WHEN the prior stanza emits a matching canonical receipt
- THEN the expectation passes without relying on rendered text alone.

### Requirement: Saved state modes
r[molten.transcripts.saved_state] Molten MUST support save mode for inspection and MUST deny or keep placeholder-only fork and in-place modes unless future policy explicitly admits them.

#### Scenario: In-place denied by default
- GIVEN a transcript run requests in-place mode
- WHEN no explicit policy admits ambient mutation
- THEN the run emits a denial receipt and does not allocate mutable runner state.

### Requirement: Rendered output is non-authoritative
r[molten.transcripts.rendered_output] Molten MUST render human-readable transcript output from canonical run records while preserving hidden/noisy output in evidence and not treating rendered text as the sole oracle.

#### Scenario: Hidden stanza omits display only
- GIVEN a stanza has the hidden-output modifier
- WHEN docs are rendered
- THEN rendered docs omit noisy output but canonical evidence still retains the stanza outcome.

### Requirement: Transcript run receipts
r[molten.transcripts.run_receipts] Molten MUST emit canonical transcript-run receipts for run start, stanza outcomes, expected failures, known bugs, cache hit/miss/stale decisions, render, denial, and final result.

#### Scenario: Known bug is recorded
- GIVEN a stanza marked as a known bug fails
- WHEN the transcript runner records the outcome
- THEN the final receipt includes the known-bug stanza evidence and diagnostics.

### Requirement: Transcript effect admission
r[molten.transcripts.effect_admission] Molten MUST deny production effects unless a transcript or stanza declares an admitted handler profile and policy/capability evidence.

#### Scenario: Ambient shell denied
- GIVEN a transcript contains an unsupported ambient shell block
- WHEN the parser or runner validates the transcript
- THEN it emits a canonical denial diagnostic before host effects occur.

### Requirement: Transcript evaluation cache
r[molten.transcripts.eval_cache] Molten MUST integrate deterministic transcript results with the evaluation cache using keys that bind transcript refs, dependency closure, handler profile, policy/capability/revocation refs, runner version, seed/config refs, and expected refs.

#### Scenario: Deterministic transcript hits cache
- GIVEN a transcript run has a cached deterministic passing receipt
- WHEN the transcript reruns with matching policy and dependency refs
- THEN the runner may return the cached transcript run receipt with cache-hit evidence.

### Requirement: Transcript documentation rendering
r[molten.transcripts.docs_render] Molten MUST render transcript artifacts as documentation with hidden/noisy stanzas omitted from display but preserved in canonical evidence.

#### Scenario: Rendered docs include final receipt ref
- GIVEN a transcript run has completed
- WHEN Molten renders the transcript
- THEN the rendered docs include human-readable decisions and final receipt refs derived from canonical evidence.

### Requirement: Basic transcript tests
r[molten.transcripts.basic_tests] Molten MUST test parsing, installing an artifact, running admitted local commands, and matching canonical output or receipt expectations.

#### Scenario: Transcript installs artifact and checks output
- GIVEN a transcript includes an artifact stanza, a local command stanza, and an expectation stanza
- WHEN the transcript test runs
- THEN the artifact installs, the local command executes through restricted dispatch, and the expectation matches canonical output.

### Requirement: Expected error tests
r[molten.transcripts.expected_error_tests] Molten MUST test expected-error and known-bug stanza behavior.

#### Scenario: Expected error test passes
- GIVEN a transcript stanza is marked expected-error
- WHEN the underlying command fails as expected
- THEN the test records a pass rather than aborting the transcript.

### Requirement: Reproducibility tests
r[molten.transcripts.reproducibility_tests] Molten MUST test that fresh deterministic transcript runs produce stable output artifact ids and that stale policy/cache mismatches deny semantic cache reuse.

#### Scenario: Repeated fresh run is stable
- GIVEN the same transcript and deterministic inputs
- WHEN fresh mode runs twice
- THEN canonical transcript refs, receipt shape, and deterministic outputs remain stable.

### Requirement: Transcript property tests
r[molten.transcripts.property_tests] Molten SHOULD use Hegel property tests for stanza ordering, output matching, fresh-run determinism, stable transcript identity, and denied ambient state access.

#### Scenario: Generated stanza ordering is stable
- GIVEN generated bounded transcript stanza sequences
- WHEN Molten parses them repeatedly
- THEN stanza ordering and transcript identity remain deterministic.

### Requirement: System MUST Define canonical `transcript-artifact-v1` records with markdown source refs, parsed stanza refs, dependency closure, handler profile, policy/capability/revocation refs, seed/config refs, expected refs, and checks
r[molten.local_transcripts.artifact_dto] The system MUST Define canonical `transcript-artifact-v1` records with markdown source refs, parsed stanza refs, dependency closure, handler profile, policy/capability/revocation refs, seed/config refs, expected refs, and checks.

### Requirement: System MUST Define canonical `transcript-stanza-v1` records for `molten-cli`, `preserves`, `artifact`, `policy`, `expect`, and `comment` stanzas
r[molten.local_transcripts.stanza_dto] The system MUST Define canonical `transcript-stanza-v1` records for `molten-cli`, `preserves`, `artifact`, `policy`, `expect`, and `comment` stanzas.

### Requirement: System MUST Support bounded stanza modifiers for expected error, known bug, hidden output, skip, required feature, seed, and handler profile override
r[molten.local_transcripts.modifier_dto] The system MUST Support bounded stanza modifiers for expected error, known bug, hidden output, skip, required feature, seed, and handler profile override.

### Requirement: System MUST Document that Unison transcripts are prior art only and Molten does not adopt UCM syntax, codebase semantics, hash formats, or typechecker behavior
r[molten.local_transcripts.no_ucm_compat] The system MUST Document that Unison transcripts are prior art only and Molten does not adopt UCM syntax, codebase semantics, hash formats, or typechecker behavior.

### Requirement: System MUST Parse markdown fenced blocks into deterministic stanza records while preserving prose/comment anchors for rendering
r[molten.local_transcripts.markdown_parser] The system MUST Parse markdown fenced blocks into deterministic stanza records while preserving prose/comment anchors for rendering.

### Requirement: System MUST Treat canonical source bytes and parsed stanza records as identity, never local paths, mtimes, cwd, env vars, or wall-clock time
r[molten.local_transcripts.canonical_source_identity] The system MUST Treat canonical source bytes and parsed stanza records as identity, never local paths, mtimes, cwd, env vars, or wall-clock time.

### Requirement: System MUST Render transcript artifacts and run results back to markdown, omitting `hide` output from display while preserving evidence refs
r[molten.local_transcripts.render_docs] The system MUST Render transcript artifacts and run results back to markdown, omitting `hide` output from display while preserving evidence refs.

### Requirement: System MUST Emit canonical diagnostics for unknown stanza kinds, malformed modifiers, oversized stanza payloads, and unsupported ambient shell blocks
r[molten.local_transcripts.parser_errors] The system MUST Emit canonical diagnostics for unknown stanza kinds, malformed modifiers, oversized stanza payloads, and unsupported ambient shell blocks.

### Requirement: System MUST Implement a fresh-state deterministic runner with isolated registry, ledger, typed-storage, cache, and scratch roots
r[molten.local_transcripts.fresh_runner] The system MUST Implement a fresh-state deterministic runner with isolated registry, ledger, typed-storage, cache, and scratch roots.

### Requirement: System MUST Implement a restricted `molten-cli` argument-vector dispatcher for admitted local test commands without falling back to a host shell
r[molten.local_transcripts.restricted_cli_dispatch] The system MUST Implement a restricted `molten-cli` argument-vector dispatcher for admitted local test commands without falling back to a host shell.

### Requirement: System MUST Compare canonical expectations for Preserves value refs, artifact/cache/storage/schema refs, receipt kind/decision, diagnostics, output absence, and trace-pattern refs
r[molten.local_transcripts.expectation_engine] The system MUST Compare canonical expectations for Preserves value refs, artifact/cache/storage/schema refs, receipt kind/decision, diagnostics, output absence, and trace-pattern refs.

### Requirement: System MUST Add a `save` mode for inspection while keeping `fork` and `in-place` denied or placeholder-only in this slice
r[molten.local_transcripts.saved_state] The system MUST Add a `save` mode for inspection while keeping `fork` and `in-place` denied or placeholder-only in this slice.

### Requirement: System MUST Emit and parse canonical `transcript-run-receipt-v1` records for run start, stanza outcomes, render, cache-hit/cache-miss, denial, expected failure, known bug, and final result
r[molten.local_transcripts.run_receipt_dto] The system MUST Emit and parse canonical `transcript-run-receipt-v1` records for run start, stanza outcomes, render, cache-hit/cache-miss, denial, expected failure, known bug, and final result.

### Requirement: System MUST Deny production effects unless a stanza/transcript declares an admitted handler profile and policy/capability evidence
r[molten.local_transcripts.effect_admission] The system MUST Deny production effects unless a stanza/transcript declares an admitted handler profile and policy/capability evidence.

### Requirement: System MUST Ensure hidden/noisy output affects only rendering and remains present in canonical receipts/evidence
r[molten.local_transcripts.hidden_evidence] The system MUST Ensure hidden/noisy output affects only rendering and remains present in canonical receipts/evidence.

### Requirement: System MUST Classify transcript artifacts and transcript run receipts in the local evidence ledger
r[molten.local_transcripts.ledger_classification] The system MUST Classify transcript artifacts and transcript run receipts in the local evidence ledger.

### Requirement: System MUST Use `eval-cache` operation `transcript-run` with keys binding transcript artifact refs, dependency closure, handler profile, policy/capability/revocation refs, runner/tool version, seed/config refs, and expected refs
r[molten.local_transcripts.cache_key] The system MUST Use `eval-cache` operation `transcript-run` with keys binding transcript artifact refs, dependency closure, handler profile, policy/capability/revocation refs, runner/tool version, seed/config refs, and expected refs.

### Requirement: System MUST Admit cache hits only for deterministic transcript tiers and deny semantic use of `production-effectful-trace-only` entries
r[molten.local_transcripts.cache_hit_admission] The system MUST Admit cache hits only for deterministic transcript tiers and deny semantic use of `production-effectful-trace-only` entries.

### Requirement: System MUST Bind cache hit/miss/stale-deny receipts into final transcript run receipts
r[molten.local_transcripts.cache_receipts] The system MUST Bind cache hit/miss/stale-deny receipts into final transcript run receipts.

### Requirement: System MUST Expose dependency/policy invalidation hooks for transcript-derived cache entries
r[molten.local_transcripts.cache_invalidation_hooks] The system MUST Expose dependency/policy invalidation hooks for transcript-derived cache entries.

### Requirement: System MUST Add `molten test transcript parse` and `run` commands with full ref display and receipt/output file options
r[molten.local_transcripts.cli_parse_run] The system MUST Add `molten test transcript parse` and `run` commands with full ref display and receipt/output file options.

### Requirement: System MUST Add `show` and `render` commands for transcript artifacts and run receipts
r[molten.local_transcripts.cli_show_render] The system MUST Add `show` and `render` commands for transcript artifacts and run receipts.

### Requirement: System MUST Ensure CLI paths are local IO handles only and never part of transcript/cache identity except through canonical bytes
r[molten.local_transcripts.cli_no_path_identity] The system MUST Ensure CLI paths are local IO handles only and never part of transcript/cache identity except through canonical bytes.

### Requirement: System MUST Emit canonical failure artifacts for parse/run/render failures
r[molten.local_transcripts.cli_failures] The system MUST Emit canonical failure artifacts for parse/run/render failures.

### Requirement: System MUST Add tests for stanza ordering, modifier parsing, stable transcript refs, and malformed stanza denials
r[molten.local_transcripts.parse_tests] The system MUST Add tests for stanza ordering, modifier parsing, stable transcript refs, and malformed stanza denials.

### Requirement: System MUST Add tests for a transcript that installs an artifact, runs a local command, and matches canonical output/receipt expectations
r[molten.local_transcripts.runner_tests] The system MUST Add tests for a transcript that installs an artifact, runs a local command, and matches canonical output/receipt expectations.

### Requirement: System MUST Add tests for expected error and known bug stanza behavior
r[molten.local_transcripts.expected_error_tests] The system MUST Add tests for expected error and known bug stanza behavior.

### Requirement: System MUST Add tests proving deterministic transcript runs can hit the evaluation cache and stale policy/cache mismatches deny
r[molten.local_transcripts.cache_tests] The system MUST Add tests proving deterministic transcript runs can hit the evaluation cache and stale policy/cache mismatches deny.

### Requirement: System MUST Add tests that hidden output is omitted from rendered docs but preserved in canonical evidence
r[molten.local_transcripts.render_tests] The system MUST Add tests that hidden output is omitted from rendered docs but preserved in canonical evidence.

### Requirement: System MUST Add Hegel properties for stanza ordering, fresh-run determinism, stable transcript identity, and denied ambient state access
r[molten.local_transcripts.property_tests] The system MUST Add Hegel properties for stanza ordering, fresh-run determinism, stable transcript identity, and denied ambient state access.

### Requirement: Executable transcript stanzas bind exact refs
r[molten.transcripts.exact_artifact_ref_expectations] Molten MUST require transcript execution stanzas to bind exact artifact refs or admitted name-resolution receipts plus schema refs, policy refs, effect manifest refs, handler profile refs, capability refs, and resource refs.

#### Scenario: Name resolution is pinned before replay
- GIVEN a transcript stanza starts from a human-readable artifact name
- WHEN Molten admits the transcript for execution
- THEN the transcript run evidence records the exact artifact ref or resolution receipt
- AND replay does not depend on the mutable name.

#### Scenario: Missing capability ref denies transcript execution
- GIVEN a transcript stanza requests execution that needs a capability
- WHEN no capability fixture or admission receipt is bound
- THEN Molten denies the stanza before side effects.

### Requirement: Transcript expectations are canonical receipt oracles
r[molten.transcripts.canonical_receipt_oracles] Molten MUST represent transcript expectations as canonical value refs, trace markers, receipt kinds, receipt decisions, and failure classes rather than raw terminal text.

#### Scenario: Receipt oracle matches passing run
- GIVEN a transcript expects a `gate-receipt-v1` pass decision
- WHEN the run emits a matching canonical receipt
- THEN Molten records a passing transcript expectation.

#### Scenario: Raw terminal text cannot replace receipt oracle
- GIVEN a transcript expects only a rendered stdout line for a policy gate decision
- WHEN no canonical receipt expectation is present
- THEN Molten denies normative pass evidence for that gate.

### Requirement: Rendered output is diagnostic-only by default
r[molten.transcripts.diagnostic_output_non_normative] Molten MUST treat stdout, stderr, logs, prose, hidden output, and rendered markdown as diagnostic-only unless the output is explicitly canonicalized as a Preserves value with a receipt binding.

#### Scenario: Canonical value output can be checked
- GIVEN a command emits a canonical Preserves value and the transcript expects its value ref
- WHEN the run completes
- THEN Molten may compare the canonical output as an oracle.

#### Scenario: Hidden output does not prove pass
- GIVEN a transcript hides command output for readability
- WHEN Molten evaluates pass evidence
- THEN hidden output contributes no normative proof unless a canonical receipt oracle is present.

### Requirement: Handler profile and seed bind transcript replay
r[molten.transcripts.handler_profile_seed_binding] Molten MUST bind handler profile refs, seeds, logical time, effect manifest refs, policy refs, and resource refs into transcript run keys, replay receipts, and evaluation-cache keys.

#### Scenario: Same run key replays deterministically
- GIVEN the same transcript stanzas, artifact refs, handler profile refs, seed, policy refs, and resource refs
- WHEN Molten replays the transcript
- THEN replay uses the same run key and compares canonical receipts.

#### Scenario: Profile mismatch denies cache reuse
- GIVEN a transcript cache entry was produced under handler profile H1
- WHEN a caller requests reuse under handler profile H2 without compatibility evidence
- THEN Molten denies the cache hit for normative transcript pass evidence.

### Requirement: Receipt transcript validation covers positive and negative paths
r[molten.transcripts.receipt_transcript_validation] Molten MUST include positive and negative fixtures for deterministic replay, expected failures, stale refs, handler profile mismatch, nondeterministic output, missing capabilities, hidden output, and UCM compatibility denial.

#### Scenario: Deterministic transcript fixture passes
- GIVEN a transcript with exact refs, admitted handlers, seed, and receipt oracles
- WHEN validation runs twice
- THEN both runs emit matching canonical receipt decisions.

#### Scenario: UCM compatibility claim denies
- GIVEN transcript metadata claims UCM syntax or codebase compatibility
- WHEN validation checks the transcript boundary
- THEN it denies the claim
- AND records that Unison transcripts are prior art only.
