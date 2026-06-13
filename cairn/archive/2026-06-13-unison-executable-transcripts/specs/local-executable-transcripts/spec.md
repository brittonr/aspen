## ADDED Requirements

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
