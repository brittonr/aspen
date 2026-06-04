## Why

Molten now has a local artifact catalog query core that can list, view, search, resolve short ids, and inspect dependency graphs with visibility filtering and redaction hooks. The next step is to expose that same inspection surface through deterministic read-only MCP-shaped requests without adding mutation authority or network/server behavior.

A local MCP request/response model lets tests, transcripts, future tool bridges, and agents call catalog inspection in a stable way while preserving Molten's full-ref identity, redacted-by-default rendering, hidden-ref filtering, and fail-closed behavior for mutating tool names.

## What Changes

- Add canonical `catalog-mcp-request-v1`, `catalog-mcp-response-v1`, and `catalog-mcp-receipt-v1` records.
- Add read-only tools: `catalog.list`, `catalog.view`, `catalog.search`, `catalog.deps`, `catalog.dependents`, and `catalog.short_id`.
- Route every request through the existing local catalog core and bind the core catalog receipt into the MCP receipt.
- Redact by default for view payloads and preserve hidden-ref filtering for all tools.
- Expand short ids to full refs before invoking catalog operations.
- Deny unknown or mutating tool names as data-bearing denial responses, not ambient side effects.
- Add CLI support through `molten test catalog mcp-call <request.preserves>` for deterministic local testing.

## Impact

This creates a safe read-only API boundary over Molten's local catalog. Future MCP servers or remote tool adapters can wrap these canonical request/response records without bypassing policy, redaction, or identity rules.
