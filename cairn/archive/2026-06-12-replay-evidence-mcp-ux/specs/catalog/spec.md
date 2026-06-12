# Catalog Specification Delta

## Requirements

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
