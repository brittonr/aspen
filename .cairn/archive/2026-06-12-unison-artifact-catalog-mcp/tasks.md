## Phase 1: Catalog query core

- [x] [serial] r[molten.catalog.query_core] Define catalog query APIs over artifact summaries, dependencies, schemas, effects, policies, receipts, transcripts, and upgrades.
- [x] [serial] r[molten.catalog.visibility_filter] Apply policy-filtered visibility to catalog queries before returning results.
- [x] [serial] r[molten.catalog.short_ids] Implement unambiguous short artifact id resolution for UI/CLI only, expanding to full ids before operations.
- [x] [parallel] r[molten.catalog.no_name_identity] Document that catalog names and aliases are metadata, not artifact identity.

## Phase 2: Rendered views and search

- [x] [serial] r[molten.catalog.artifact_views] Render artifact summary, dependency graph, schema, effect, policy/evidence, transcript, and upgrade views.
- [x] [serial] r[molten.catalog.semantic_search] Implement search by text, kind, schema, structural fingerprint, effect, capability, dependency, receipt, provenance, transcript status, and upgrade status.
- [x] [parallel] r[molten.catalog.docs_links] Render docs and transcript output with links to exact artifact ids and receipt refs.
- [x] [parallel] r[molten.catalog.query_traces] Emit trace records for catalog queries that include caller, query hash, visibility decision, and result hash.

## Phase 3: MCP server

- [x] [serial] r[molten.catalog.mcp_readonly] Add read-only MCP tools for list/view/search artifacts, dependencies, dependents, schemas, effects, receipts, transcripts, upgrades, and short-id resolution.
- [x] [serial] r[molten.catalog.mcp_auth] Require capability and visibility checks for every MCP tool call.
- [x] [parallel] r[molten.catalog.mcp_mutating_plan] Define policy-gated mutating MCP tools for dry-run install, transcript run, rewrite preview, upgrade creation, and remote sync planning without implementing ambient authority.
- [x] [parallel] r[molten.catalog.local_cli] Expose the same query core through local CLI inspection commands.

## Phase 4: Tests

- [x] [serial] r[molten.catalog.search_tests] Add tests for semantic search by schema, effect, dependency, receipt, and transcript status.
- [x] [serial] r[molten.catalog.visibility_tests] Add tests that unauthorized artifacts and receipt fields are hidden or denied.
- [x] [parallel] r[molten.catalog.short_id_tests] Add tests for unambiguous and ambiguous short id resolution.
- [x] [parallel] r[molten.catalog.mcp_tests] Add MCP tool tests for read-only inspection and denied mutating/unauthorized calls.
