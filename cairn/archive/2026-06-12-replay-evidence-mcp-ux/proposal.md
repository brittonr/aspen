# Change: replay-evidence-mcp-ux

## Why

Generic deterministic replay evidence is now classified in the ledger and catalog, but operators using the catalog MCP surface still have to know the low-level text filters. Replay investigations need a read-only named tool that can find replay verification receipts and first-divergence records by decision, divergence kind, actor/handler refs, report refs, final-state refs, and output/effect refs.

## What

- Add a read-only catalog MCP tool for replay evidence search.
- Map operator-facing replay arguments to existing catalog classifications.
- Test readback through the MCP request/response/receipt path.

## Impact

Replay evidence becomes easier to inspect through the same catalog MCP interface used for other evidence classes. The tool remains read-only and evidence-only; it does not grant authority, policy admission, provenance trust, source-gate acceptance, or replay trust beyond the imported receipts.
