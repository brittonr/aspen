# Catalog Specification Delta

## Requirements

### Requirement: Replay indexes group replay evidence
r[molten.determinism.replay_index.schema] Molten SHOULD emit `deterministic-replay-index-v1` evidence over bounded sets of generic replay verification receipts and replay rollups.

#### Scenario: Raw receipts and rollups are indexed
- GIVEN a replay rollup and a raw replay verification receipt
- WHEN a replay index is generated
- THEN the index records total, pass, deny, raw-receipt, rollup, and divergence counts
- AND the index records referenced replay receipt and rollup refs

### Requirement: Replay indexes reject stale inputs
r[molten.determinism.replay_index.validation] Molten SHOULD make replay indexes deny when an expected replay evidence ref does not match the supplied value or when an input is not a replay verify receipt or replay rollup.

#### Scenario: Mismatched replay rollup ref denies
- GIVEN a replay index input with an expected ref for different content
- WHEN the index is generated
- THEN the index decision is `deny`
- AND diagnostics include the expected and actual refs
- AND the mismatched input is not counted as valid replay evidence

### Requirement: Replay indexes are catalog-searchable
r[molten.determinism.replay_index.catalog] The evidence ledger and catalog SHOULD classify replay indexes by artifact kind, decision, total count, pass count, deny count, raw receipt count, rollup count, divergence kinds, report refs, final-state refs, receipt refs, and rollup refs.

#### Scenario: Replay index is found by stage and final state
- GIVEN an imported replay index
- WHEN catalog search filters by `deterministic-replay-index`, `replay-index-decision`, and a final-state ref
- THEN the replay index is returned

### Requirement: Replay indexes have MCP readback
r[molten.determinism.replay_index.mcp] Replay evidence MCP search SHOULD return replay indexes through read-only search filters while preserving evidence-only semantics.

#### Scenario: Replay index MCP readback is evidence only
- GIVEN an imported replay index
- WHEN replay evidence MCP search filters by `stage=index`
- THEN the index is returned with read-only MCP receipt evidence
- AND the index does not replace individual replay verification, rollup, harness gate, policy, source-gate, release, provenance, transport, or authority checks

### Requirement: Replay index behavior is tested
r[molten.determinism.replay_index.tests] Molten SHOULD test replay index generation, stale input denial, catalog search, and replay MCP readback.

#### Scenario: Replay index validation covers denial and discovery
- GIVEN mixed replay evidence and a stale input case
- WHEN tests generate and import replay indexes
- THEN passing discovery and deny diagnostics are both covered
