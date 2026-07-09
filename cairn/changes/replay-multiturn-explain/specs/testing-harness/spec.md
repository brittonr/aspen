## ADDED Requirements

### Requirement: Multi-turn replay comparison is canonical
r[molten.determinism.multiturn_replay.core] Molten MUST provide a deterministic replay comparison core that compares run identity, ordered turn journal refs, ordered effect request and response refs, output refs, and final-state refs by canonical content refs rather than rendered logs.

#### Scenario: Matching multi-turn replay passes
- GIVEN expected and actual replay summaries with the same run identity, turn journal refs, effect log refs, output refs, and final-state refs
- WHEN multi-turn replay comparison runs
- THEN the replay decision is `pass`
- AND the emitted replay receipt binds the compared summary refs.

#### Scenario: Changed turn ref denies
- GIVEN expected and actual replay summaries with matching run identity but a different turn journal ref at one position
- WHEN multi-turn replay comparison runs
- THEN the replay decision is `deny`
- AND the first-divergence evidence identifies the divergent turn position before downstream final-state drift.

### Requirement: First-divergence records include path metadata
r[molten.determinism.multiturn_replay.first_divergence_path] First-divergence records MUST bind divergence kind, turn index, event index when available, boundary kind, actor/session/vat identifier when present, field path, handler profile ref, expected ref, actual ref, and redaction status.

#### Scenario: Effect response divergence names boundary path
- GIVEN a replay whose first mismatch is an effect response boundary in a later turn
- WHEN replay comparison denies
- THEN first-divergence evidence records the effect-response boundary kind, turn index, event index, handler profile ref, expected response ref, actual response ref, and safe redaction status.

#### Scenario: Raw payload is not exposed by default
- GIVEN a first-divergence record for a sensitive trace boundary
- WHEN the record is rendered without trace privacy authority
- THEN the rendered diagnostic shows safe refs and path metadata only
- AND raw payload materialization requires separate privacy evidence.

### Requirement: Replay explain emits canonical evidence
r[molten.determinism.multiturn_replay.explain_cli] The replay explain CLI MUST emit canonical explain evidence for replay comparisons or deny receipts before rendering human-readable summaries.

#### Scenario: Explain summarizes deny receipt
- GIVEN a replay deny receipt with first-divergence evidence
- WHEN `molten test replay explain` is run
- THEN the command emits an explain receipt bound to the deny receipt and first-divergence ref
- AND the rendered summary is diagnostic-only.

#### Scenario: Explain rejects malformed replay evidence
- GIVEN malformed or stale replay evidence
- WHEN `molten test replay explain` is run
- THEN the command fails closed with canonical failure evidence
- AND no rendered summary is accepted as replay verification.

### Requirement: Large replay traces support prefix comparison
r[molten.determinism.multiturn_replay.merkle_prefix] Molten SHOULD compare manifest-backed large replay traces by summary roots and narrowed turn or boundary refs before materializing full trace contents.

#### Scenario: Prefix comparison narrows divergent turn
- GIVEN two large replay trace manifests whose summary roots differ
- WHEN prefix comparison runs
- THEN the comparator identifies the first divergent turn or boundary ref
- AND any partial fetch is covered by chunk range receipt evidence.

#### Scenario: Tampered manifest denies before comparison
- GIVEN a replay trace manifest whose stored bytes do not match its declared content ref
- WHEN prefix comparison attempts to read it
- THEN comparison denies before using the tampered bytes
- AND diagnostics bind the failed manifest ref.

### Requirement: Multi-turn replay behavior is tested
r[molten.determinism.multiturn_replay.tests] Molten SHOULD test positive multi-turn replay stability, negative first-divergence path diagnostics, explain CLI receipts, manifest-backed prefix comparison, and redaction-safe rendering.

#### Scenario: Test matrix covers semantic boundaries
- GIVEN a multi-turn replay fixture with tamper variants for scheduler, input, effect request, effect response, policy decision, hostcall decision, actor output, receipt, output, and state refs
- WHEN replay tests verify each tamper variant
- THEN each case denies with the expected first-divergence kind and path metadata.
