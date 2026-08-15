# Catalog Delta: Artifact Catalog and MCP Introspection

### Requirement: Catalog query core
r[molten.catalog.query_core] Molten MUST provide catalog query APIs over artifact summaries, dependencies, schemas, effects, policies, receipts, transcripts, and upgrades.

#### Scenario: Query core lists summaries
- GIVEN a local artifact registry with installed artifacts
- WHEN a catalog list query runs
- THEN the result includes canonical artifact summaries
- AND the query receipt binds the query and result refs

### Requirement: Visibility filtering
r[molten.catalog.visibility_filter] Molten MUST apply policy and visibility filtering before returning catalog summaries, views, receipt payloads, short-id candidates, search results, or MCP responses.

#### Scenario: Hidden artifact is omitted
- GIVEN a hidden artifact ref in the visibility input
- WHEN catalog search runs
- THEN the hidden artifact is omitted from results
- AND the catalog receipt records visibility filtering checks

### Requirement: Short id expansion
r[molten.catalog.short_ids] Molten MUST treat short ids as UI/CLI conveniences only and expand them to a single full artifact ref before downstream operations.

#### Scenario: Ambiguous short id denies
- GIVEN more than one visible artifact matching a short prefix
- WHEN short-id resolution runs
- THEN the decision is `deny`
- AND no downstream operation receives the ambiguous prefix as identity

### Requirement: Names are not identity
r[molten.catalog.no_name_identity] Molten MUST document and enforce that names, aliases, tags, channels, paths, mtimes, and short ids are metadata, not artifact identity.

#### Scenario: Renaming does not change identity
- GIVEN an artifact with a display name
- WHEN the name pointer changes
- THEN the artifact ref remains content-addressed and unchanged
- AND catalog records mark names as metadata

### Requirement: Artifact views
r[molten.catalog.artifact_views] Molten MUST render artifact summaries, dependency graph, schema, effect, policy/evidence, transcript, receipt, and upgrade views using canonical refs and redacted-by-default content.

#### Scenario: Receipt view binds subject
- GIVEN an artifact with registry and ledger receipts
- WHEN a receipt view is requested for the artifact ref
- THEN only visible receipt records that bind the subject ref are returned
- AND secret-bearing content is redacted before rendering

### Requirement: Semantic search
r[molten.catalog.semantic_search] Molten MUST support bounded semantic search by text, kind, schema, structural fingerprint, effect, capability, dependency, dependent, receipt operation/decision, provenance/evidence, transcript status, and upgrade status.

#### Scenario: Search by transcript status
- GIVEN a transcript run receipt with decision `pass`
- WHEN catalog search filters by transcript status `pass`
- THEN the transcript receipt artifact is returned

#### Scenario: Search by upgrade status
- GIVEN an upgrade receipt with decision `pass`
- WHEN catalog search filters by upgrade status `pass`
- THEN the upgrade receipt artifact is returned

### Requirement: Docs and transcript links
r[molten.catalog.docs_links] Molten MUST render docs and transcript output with links to exact artifact ids and receipt refs rather than mutable names.

#### Scenario: Transcript view uses refs
- GIVEN a transcript artifact and run receipt
- WHEN it is shown through catalog or MCP views
- THEN the rendered output includes artifact and receipt refs
- AND no mutable name is treated as identity

### Requirement: Catalog query traces
r[molten.catalog.query_traces] Molten MUST emit deterministic catalog query receipts that include operation, query hash, visibility decision, result hash, diagnostics, and visible result refs.

#### Scenario: Query receipt is deterministic
- GIVEN the same catalog query over the same registry state
- WHEN the query runs twice
- THEN both query receipts bind the same query and result refs

### Requirement: Read-only MCP tools
r[molten.catalog.mcp_readonly] Molten MUST expose read-only MCP tools for list/view/search artifacts, dependencies, dependents, schemas, effects, receipts, transcripts, upgrades, and short-id resolution.

#### Scenario: Schema MCP search is read-only
- GIVEN an MCP request for `search_by_schema`
- WHEN the request is dispatched
- THEN it routes through the catalog query core
- AND the response binds a catalog receipt ref

### Requirement: MCP auth and visibility
r[molten.catalog.mcp_auth] Molten MUST perform capability and visibility checks for every MCP tool call and MUST keep hidden refs out of MCP responses.

#### Scenario: Hidden ref stays hidden over MCP
- GIVEN an MCP search request with a hidden ref
- WHEN the request is dispatched
- THEN the response omits the hidden artifact
- AND the MCP receipt includes visibility and capability checks

### Requirement: Mutating MCP tools are planned but not ambient
r[molten.catalog.mcp_mutating_plan] Molten MUST define future mutating MCP tools for dry-run install, transcript run, rewrite preview, upgrade creation, and remote sync planning as policy-gated operations, and MUST deny mutating tools without ambient authority in the read-only server.

#### Scenario: Mutating tool denies
- GIVEN an MCP request for a mutating catalog tool
- WHEN the read-only MCP dispatcher handles it
- THEN the decision is `deny`
- AND diagnostics state that the tool is outside the read-only allow-list

### Requirement: Local CLI inspection
r[molten.catalog.local_cli] Molten MUST expose the same query core through local CLI inspection commands without making registry paths part of canonical query identity.

#### Scenario: CLI search uses query core
- GIVEN a local CLI catalog search command
- WHEN it filters by kind, dependency, receipt, transcript, or upgrade status
- THEN it produces the same canonical catalog result and receipt shapes as the core query API

### Requirement: Search coverage
r[molten.catalog.search_tests] Molten MUST include tests for semantic search by schema, effect, dependency, receipt, transcript status, and upgrade status.

#### Scenario: Schema and effect tests pass
- GIVEN fixtures with schema and effect refs
- WHEN catalog and MCP searches run
- THEN matching artifacts are returned and nonmatching artifacts are omitted

### Requirement: Visibility coverage
r[molten.catalog.visibility_tests] Molten MUST include tests that unauthorized artifacts, hidden refs, and sensitive receipt fields are hidden or denied before rendering.

#### Scenario: Secret payload redacts
- GIVEN a secret-bearing payload
- WHEN an unprivileged catalog or MCP view renders it
- THEN the output includes a redaction marker
- AND plaintext is absent

### Requirement: Short id coverage
r[molten.catalog.short_id_tests] Molten MUST include tests for unambiguous resolution, ambiguous denial, minimum prefix length, and hidden candidate filtering.

#### Scenario: Hidden candidate ignored
- GIVEN a short prefix matching visible and hidden refs
- WHEN resolution runs with the hidden ref filtered
- THEN only visible candidates participate in the decision

### Requirement: MCP coverage
r[molten.catalog.mcp_tests] Molten MUST include MCP tool tests for read-only inspection, alias tool names, denied mutating calls, hidden refs, and catalog receipt binding.

#### Scenario: Alias tool binds receipt
- GIVEN a read-only alias tool such as `view_receipts`
- WHEN the MCP request succeeds
- THEN the response binds a catalog receipt
- AND the MCP receipt records the tool and response refs
