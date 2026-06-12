# Catalog Specification

## Purpose

Defines the `catalog` capability.

## Requirements

### Requirement: Canonical short-id prefix grammar
r[molten.catalog.short_id_canonical_prefixes] Molten MUST treat catalog short-id inputs as either canonical full content refs or lowercase hex prefixes without a `blake3:` scheme, and MUST NOT treat malformed ref-shaped strings as prefix searches.

#### Scenario: Ref-shaped malformed prefix denies
- GIVEN a catalog short-id input of `blake3:` or `blake3:<bad>`
- WHEN short-id resolution runs
- THEN the decision is `deny`
- AND candidate search is skipped with a malformed full-ref diagnostic

#### Scenario: Full canonical ref resolves exactly
- GIVEN a full canonical content ref visible in the catalog
- WHEN short-id resolution receives that full ref
- THEN the decision is `pass`
- AND the result expands to the same full ref

### Requirement: Short-id malformed denials
r[molten.catalog.short_id_malformed_denials] Molten MUST deny non-hex or uppercase short-id prefixes as canonical data-bearing denial results before downstream catalog operations receive them.

#### Scenario: Uppercase prefix denies
- GIVEN a short-id prefix containing uppercase hex characters
- WHEN short-id resolution runs
- THEN the decision is `deny`
- AND diagnostics state that short-id prefixes use lowercase hex characters

#### Scenario: Hidden-only prefix denies
- GIVEN a lowercase hex prefix that matches only hidden refs
- WHEN short-id resolution runs with those refs hidden
- THEN the decision is `deny`
- AND no hidden full ref is returned as the resolution

### Requirement: Replay receipts have ledger kinds
r[molten.determinism.replay_receipt_catalog.ledger_kind] The evidence ledger SHOULD classify `deterministic-replay-verify-v1` and `deterministic-first-divergence-v1` records with stable artifact kinds.

#### Scenario: Replay verify import is kinded
- GIVEN a generic replay verification receipt
- WHEN it is imported into the evidence ledger
- THEN the ledger artifact kind is `deterministic-replay-verify-receipt`

### Requirement: Replay verification records are catalog-searchable
r[molten.determinism.replay_receipt_catalog.classify_verify] The catalog SHOULD classify replay verification records by decision, divergence kind, expected/actual report refs when present, and state or output refs when present.

#### Scenario: Replay verify is found by final state
- GIVEN an imported replay verification receipt with expected report, actual report, and final-state refs
- WHEN catalog search filters by replay decision and final-state ref
- THEN the replay verification receipt is returned

### Requirement: First-divergence records are catalog-searchable
r[molten.determinism.replay_receipt_catalog.classify_divergence] The catalog SHOULD classify first-divergence records by divergence kind, actor/session/vat identifier when present, handler profile ref, expected ref, and actual ref.

#### Scenario: Divergence is found by kind
- GIVEN an imported deterministic first-divergence record for an effect-response mismatch
- WHEN catalog search filters by `replay-divergence:effect-response`
- THEN the first-divergence record is returned

### Requirement: Replay receipt catalog coverage is tested
r[molten.determinism.replay_receipt_catalog.tests] Molten SHOULD test ledger import and catalog search for generic replay verification and first-divergence evidence.

#### Scenario: Search returns replay evidence only
- GIVEN imported replay verification and first-divergence records
- WHEN catalog searches by replay decision, divergence, report refs, or final-state refs
- THEN matching replay evidence is returned without granting authority or replacing gate validation

### Requirement: Replay evidence MCP search is read-only
r[molten.catalog.replay_evidence_mcp.readonly_tool] Molten SHOULD expose generic deterministic replay evidence through a named read-only catalog MCP search tool.

#### Scenario: Replay MCP tool is allowed
- GIVEN a catalog MCP request for `search_replay_evidence`
- WHEN the MCP dispatcher checks the read-only allow-list
- THEN the request is allowed as a read-only catalog query
- AND mutating catalog tools remain denied

### Requirement: Replay evidence MCP filters map to catalog classifications
r[molten.catalog.replay_evidence_mcp.filter_args] Molten SHOULD map replay-specific MCP arguments to existing deterministic replay catalog classifications, including decision, divergence kind, actor identifier, handler profile ref, expected and actual report refs, final-state refs, output refs, and effect-log refs.

#### Scenario: Replay verify evidence is found by final state
- GIVEN an imported `deterministic-replay-verify-v1` record
- WHEN `search_replay_evidence` receives `stage`, `decision`, and `final-state-ref` filters
- THEN the MCP response includes the matching replay verification evidence

#### Scenario: First divergence evidence is found by divergence refs
- GIVEN an imported `deterministic-first-divergence-v1` record
- WHEN `search_replay_evidence` receives `stage`, `divergence`, `handler-profile-ref`, and `actual-ref` filters
- THEN the MCP response includes the matching first-divergence evidence

### Requirement: Replay evidence MCP search is evidence-only
r[molten.catalog.replay_evidence_mcp.tests] Molten SHOULD test replay evidence MCP readback and receipt binding without treating search results as authority, policy admission, provenance trust, source-gate acceptance, or replay verification.

#### Scenario: Replay MCP receipt binds readback only
- GIVEN replay evidence search through MCP
- WHEN the call succeeds
- THEN the MCP receipt binds the request, response, and catalog receipt
- AND the receipt keeps the read-only and mutating-tools-denied checks
