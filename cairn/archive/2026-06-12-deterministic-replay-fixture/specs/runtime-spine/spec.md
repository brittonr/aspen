# Runtime Spine Delta: deterministic replay fixture

### Requirement: Replay fixture identity binds deterministic inputs
r[molten.determinism.replay_fixture.identity] The deterministic replay fixture MUST define a canonical `deterministic-run-identity-v1` record that binds artifact refs, dependency-closure refs, initial-state refs, schema refs, policy refs, capability and revocation refs, handler-profile refs, seed or effect-log refs, runtime/tool version refs, and any scenario label that affects execution.

#### Scenario: Changed identity input is rejected
- GIVEN a recorded deterministic fixture identity
- WHEN replay is requested with a different artifact, profile, policy, seed, effect-log, initial-state, or version ref that affects execution
- THEN replay verification denies before accepting matching output evidence
- AND the denial identifies the changed identity boundary

### Requirement: Fixture record binds journals and effects
r[molten.determinism.replay_fixture.record] The deterministic replay fixture MUST emit a canonical `deterministic-fixture-record-v1` that binds the run identity, ordered turn journal refs, ordered effect-log refs, output refs, and final state ref needed to replay the run without ambient observations.

#### Scenario: Record contains enough evidence to replay
- GIVEN a bounded local deterministic fixture run
- WHEN the fixture record is produced
- THEN it contains or references the identity, turn journals, effect request/response pairs, outputs, receipts, and final state needed for verification

#### Scenario: Rendered output is not the replay oracle
- GIVEN a fixture record with human-readable rendering
- WHEN replay verification runs
- THEN verification compares canonical Preserves refs rather than trusting rendered text

### Requirement: Replay verifier compares semantic boundaries in order
r[molten.determinism.replay_fixture.verify] Replay fixture verification MUST emit `deterministic-replay-verify-v1` and MUST compare scheduler selection, input refs, effect request refs, effect response refs, policy-decision refs, committed action refs, receipt refs, output refs, and after-state refs in deterministic turn order.

#### Scenario: Matching replay passes
- GIVEN a recorded fixture and the same run identity inputs
- WHEN replay verification processes every recorded turn and effect response
- THEN the verify receipt passes and binds matching output and final-state refs

#### Scenario: Replay stops at the first mismatched boundary
- GIVEN a recorded fixture with a tampered turn, effect, receipt, output, or after-state ref
- WHEN replay verification reaches the first mismatch
- THEN verification stops before processing downstream differences
- AND emits a deny receipt that points to the first mismatched boundary

### Requirement: First divergence evidence is canonical and safe
r[molten.determinism.replay_fixture.first_divergence] Replay fixture verification MUST emit `deterministic-first-divergence-v1` evidence for the first mismatch, including divergence kind, turn id, actor/session/vat id when available, log position, handler-profile ref, expected canonical ref, actual canonical ref when safe, and redacted diagnostics for secret or capability-bearing boundaries.

#### Scenario: Effect response divergence is reported
- GIVEN a recorded fixture whose effect response is changed before verification
- WHEN replay compares the recorded and replayed response refs
- THEN verification reports an effect-response divergence with expected and actual refs when safe

#### Scenario: Sensitive divergence is redacted
- GIVEN a mismatch involving a secret or capability-bearing value
- WHEN first-divergence evidence is rendered or exported without reveal authority
- THEN diagnostics include safe commitments or redaction markers rather than plaintext secret or capability material

### Requirement: Replay profile denies live external effects
r[molten.determinism.replay_fixture.no_live_effects] Replay fixture verification MUST inject recorded effect responses and MUST deny live external clock, random, filesystem, network, environment, process, and storage observations that are not represented by the fixture effect log.

#### Scenario: Missing recorded effect response fails closed
- GIVEN replay execution reaches an effect request with no matching recorded request/response pair
- WHEN the replay handler profile handles the request
- THEN it denies the request before consulting the live external source
- AND records replay-denial evidence

### Requirement: Replay fixture CLI is evidence-oriented
r[molten.determinism.replay_fixture.cli] Molten SHOULD expose `molten test replay-fixture` commands for recording, verifying, tampering for negative tests, and showing canonical replay evidence without granting authority or bypassing normal gates.

#### Scenario: Fixture verify writes a receipt
- GIVEN a recorded replay fixture on disk
- WHEN an operator runs fixture verification with a receipt output path
- THEN Molten writes a canonical replay verification receipt
- AND the receipt is evidence only, not authority, policy admission, transport trust, provenance trust, source-gate trust, or release trust

### Requirement: Replay fixture tests cover pass and denial paths
r[molten.determinism.replay_fixture.tests] Molten SHOULD include tests for unchanged replay pass, changed identity input, changed effect response, changed policy or receipt boundary, live-effect denial under replay profile, and canonical readback of produced records.

#### Scenario: Tamper matrix catches first divergence
- GIVEN negative replay fixtures for identity, effect response, policy or receipt, output, and state-hash tampering
- WHEN the tests verify each fixture
- THEN each denial reports the expected first-divergence kind without accepting downstream matching refs as pass evidence
