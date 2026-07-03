## ADDED Requirements

### Requirement: Replay verification binds first semantic divergence
r[molten.determinism.replay_first_divergence.verify_receipt] Replay verification receipts MUST bind the replay decision, divergence kind, expected comparison refs, actual comparison refs, and a first-divergence ref when replay denies.

#### Scenario: Effect response tamper denies with divergence ref
- GIVEN a deterministic replay fixture whose effect response ref differs from the recorded baseline
- WHEN replay verification evaluates the supplied fixture
- THEN the replay receipt decision is `deny`
- AND the divergence kind is `effect-response`
- AND the receipt records a non-empty first-divergence ref.

### Requirement: First-divergence debug records stay evidence-only
r[molten.determinism.replay_first_divergence.debug_record] Deterministic first-divergence records MUST identify the compared field, expected ref, actual ref, and safe diagnostics without replacing replay verification, authority, policy, provenance, resource, transport, source-gate, retention, release, or harness gate evidence.

#### Scenario: Debug record cannot pass a replay gate
- GIVEN a first-divergence debug record emitted for a denying replay
- WHEN a gate requires passing replay verification evidence
- THEN the debug record alone is insufficient
- AND the original replay verify receipt remains the source of the replay decision.

### Requirement: Replay uses recorded effects only
r[molten.determinism.replay_first_divergence.recorded_effects_only] Deterministic replay MUST deny attempts to satisfy replay by issuing live external effects when a required recorded effect response is absent.

#### Scenario: Missing recorded effect denies replay
- GIVEN a deterministic replay fixture missing a required recorded effect response
- WHEN replay verification evaluates the fixture
- THEN the replay receipt decision is `deny`
- AND diagnostics include recorded-effects-only replay semantics.

### Requirement: Replay fixture CLI emits pass and tamper-denial receipts
r[molten.determinism.replay_first_divergence.cli_fixture] The replay-fixture CLI SHOULD record deterministic fixtures, generate tampered fixture variants, verify supplied fixtures, and write replay verification receipts that expose pass or deny decisions plus first-divergence refs.

#### Scenario: CLI verifies tampered fixture denial
- GIVEN a replay fixture recorded by the CLI
- WHEN an operator generates an effect-response tampered fixture and verifies it with `--receipt-out`
- THEN the command succeeds with a `deny` replay decision
- AND the receipt file binds the first-divergence ref.

### Requirement: Replay fixture divergence behavior is tested
r[molten.determinism.replay_first_divergence.tests] Molten SHOULD test unchanged replay pass behavior, tampered replay denial for each supported divergence kind, missing-recorded-effect denial, and CLI receipt output for first-divergence refs.

#### Scenario: Tamper matrix covers divergence kinds
- GIVEN replay fixture tests generate one tampered fixture per supported semantic comparison class
- WHEN the tests verify each fixture
- THEN each case denies with the expected divergence kind
- AND each denial binds safe canonical first-divergence evidence.
