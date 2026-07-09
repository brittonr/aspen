# Local Executable Transcripts Delta: Receipt Oracles

## ADDED Requirements

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