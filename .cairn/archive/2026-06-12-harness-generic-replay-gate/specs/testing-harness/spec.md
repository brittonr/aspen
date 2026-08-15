# Testing Harness Delta: generic replay gate evidence

### Requirement: Harness gates emit generic replay evidence
r[molten.determinism.harness_generic_replay.emit] Harness pass gates SHOULD emit generic `deterministic-replay-verify-v1` evidence from the deterministic replay comparison used for gate acceptance.

#### Scenario: Gate receipt embeds replay verification
- GIVEN a harness report that validates and replays successfully
- WHEN a gate receipt is emitted
- THEN the receipt includes generic replay verification evidence with a pass decision
- AND the generic receipt binds expected report, actual report, and final-state refs

### Requirement: Gate artifact refs bind generic replay evidence
r[molten.determinism.harness_generic_replay.artifact_ref] Gate receipts SHOULD list the generic deterministic replay verification receipt ref as an artifact ref.

#### Scenario: Replay verify ref is indexed
- GIVEN a gate receipt with embedded generic replay evidence
- WHEN artifact refs are inspected
- THEN an artifact ref with kind `deterministic-replay-verify` points to the embedded replay verification value

### Requirement: Gate parsing validates embedded generic replay evidence
r[molten.determinism.harness_generic_replay.parse] Gate receipt parsing MUST validate that embedded generic replay evidence is a pass receipt, has no divergence, and binds the same report and final-state refs as the gate replay block.

#### Scenario: Tampered generic replay receipt is rejected
- GIVEN a gate receipt whose embedded generic replay receipt has a changed report ref, final-state ref, decision, or divergence
- WHEN the gate receipt is parsed
- THEN parsing fails closed before accepting the gate receipt

### Requirement: Harness generic replay evidence is tested
r[molten.determinism.harness_generic_replay.tests] Molten SHOULD test that harness gate receipts contain and validate generic replay evidence while preserving existing replay checks.

#### Scenario: Generic replay evidence remains evidence-only
- GIVEN a gate receipt with a generic replay verification receipt
- WHEN tests inspect the gate receipt
- THEN they find the generic replay evidence and artifact ref
- AND report validation, policy, capability, resource, chain, turn journal, and source-gate checks remain required separately
