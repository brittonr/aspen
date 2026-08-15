# Catalog Specification Delta

## Requirements

### Requirement: Replay rollups summarize verification evidence
r[molten.determinism.replay_rollup.schema] Molten SHOULD emit `deterministic-replay-rollup-v1` evidence over bounded sets of generic replay verification receipts.

#### Scenario: Mixed replay receipts are summarized
- GIVEN one passing replay verification receipt and one denying replay verification receipt
- WHEN a replay rollup is generated
- THEN the rollup records total, pass, deny, and divergence counts
- AND the rollup decision is `deny`

### Requirement: Replay rollups reject stale inputs
r[molten.determinism.replay_rollup.validation] Molten SHOULD make replay rollups deny when an expected replay receipt ref does not match the supplied receipt value or when an input is not a replay verification receipt.

#### Scenario: Mismatched replay receipt ref denies
- GIVEN a replay rollup input with an expected ref for different content
- WHEN the rollup is generated
- THEN the rollup decision is `deny`
- AND diagnostics include the expected and actual refs

### Requirement: Replay rollups are catalog-searchable
r[molten.determinism.replay_rollup.catalog] The evidence ledger and catalog SHOULD classify replay rollups by artifact kind, decision, pass count, deny count, total count, and divergence kinds present.

#### Scenario: Replay rollup is found by decision
- GIVEN an imported replay rollup
- WHEN catalog search filters by `deterministic-replay-rollup` and `replay-rollup-decision:pass`
- THEN the replay rollup is returned

### Requirement: Replay rollup readback is tested
r[molten.determinism.replay_rollup.tests] Molten SHOULD test replay rollup generation, stale input denial, catalog search, and replay MCP readback while preserving evidence-only semantics.

#### Scenario: Replay rollup MCP readback is evidence only
- GIVEN an imported replay rollup
- WHEN replay evidence MCP search filters by rollup stage
- THEN the rollup is returned with read-only MCP receipt evidence
- AND the rollup does not replace individual replay verification or gate validation
