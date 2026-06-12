# Tasks: replay-evidence-index

- [x] [serial] r[molten.determinism.replay_index.schema] Add deterministic replay index receipt generation for replay verify receipts and replay rollups.
- [x] [serial] r[molten.determinism.replay_index.validation] Deny stale, tampered, or unsupported replay index inputs without counting them as valid replay evidence.
- [x] [serial] r[molten.determinism.replay_index.catalog] Classify replay indexes in the ledger/catalog by kind, decision, counts, refs, and divergence kinds.
- [x] [serial] r[molten.determinism.replay_index.mcp] Make replay indexes discoverable through replay evidence MCP readback.
- [x] [parallel] r[molten.determinism.replay_index.tests] Cover replay index generation, stale input denial, catalog import/search, MCP readback, and evidence-only checks.
