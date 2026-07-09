# Catalog Delta: Read-Only Discovery Views

## ADDED Requirements

### Requirement: Catalog provides linked semantic views
r[molten.catalog.share_like_linked_views] Molten MUST provide linked read-only catalog views over artifact refs, names, aliases, tags, channels, dependencies, dependents, schemas, effects, handler profiles, docs, transcripts, receipts, upgrade sessions, impact queries, and release snapshots.

#### Scenario: Artifact view links exact refs
- GIVEN an artifact has name metadata, dependency edges, schema refs, effect manifest refs, docs, and receipts
- WHEN a caller shows the artifact in the catalog
- THEN the view renders exact refs and links to related records
- AND names appear as metadata, not identity.

#### Scenario: Missing index is diagnostic only
- GIVEN a catalog index is stale or missing for a relation
- WHEN a caller asks for a linked view
- THEN Molten reports the missing or stale index
- AND does not invent dependency or trust facts from rendered text.

### Requirement: MCP catalog tools are read-only by default
r[molten.catalog.mcp_readonly_tools] Molten MUST expose MCP-style read-only tools for artifact search/show, dependency and dependent queries, receipt lookup, transcript lookup, impact queries, evidence explanation, and release snapshot inspection.

#### Scenario: Read-only dependency query succeeds
- GIVEN a caller invokes a dependency query tool with read authority
- WHEN the catalog has visible dependency edges
- THEN the tool returns structured results and redaction receipts.

#### Scenario: Mutation request through read-only tool denies
- GIVEN a caller invokes a read-only catalog MCP profile and asks it to update an alias or install an artifact
- WHEN the tool validates the request
- THEN it denies mutation
- AND points to the explicit gated subsystem path.

### Requirement: Catalog queries bind redaction decisions
r[molten.catalog.redaction_authorization] Molten MUST bind authorization and redaction decisions into catalog query receipts for private contents, sensitive policy outcomes, secret refs, capabilities, retention-sensitive records, and denied evidence details.

#### Scenario: Authorized private view shows content
- GIVEN a caller has admitted read authority for a private artifact
- WHEN the catalog renders that artifact
- THEN the query receipt records the authority evidence
- AND the view may include the authorized private fields.

#### Scenario: Public view redacts sensitive content
- GIVEN a public caller searches artifacts and a matching record contains secret refs or private capability details
- WHEN the catalog renders results
- THEN sensitive fields are redacted
- AND the query receipt records the redaction reason.

### Requirement: Catalog output grants no mutation authority
r[molten.catalog.no_catalog_mutation_authority] Molten MUST treat catalog and MCP discovery output as explanation evidence only; it MUST NOT grant install, alias update, policy change, capability, storage mutation, retention, release, transport, or execution authority.

#### Scenario: Catalog result supports operator decision only
- GIVEN a catalog query returns a candidate artifact ref
- WHEN an operator chooses to execute it
- THEN execution still requires the normal artifact, capability, policy, provenance, effect, resource, and source-gate admissions.

#### Scenario: Catalog receipt cannot authorize deletion
- GIVEN a catalog impact query lists no visible dependents
- WHEN a destructive retention operation is requested
- THEN the retention gate still requires retention and dependency impact evidence
- AND the catalog query alone is insufficient authority.

### Requirement: Catalog discovery validation covers positive and negative paths
r[molten.catalog.unison_discovery_validation] Molten MUST include positive and negative fixtures for linked views, read-only queries, redaction, private content denial, mutation attempts, stale indexes, and Unison Share or UCM API compatibility denial.

#### Scenario: Linked view fixture passes
- GIVEN visible artifacts, edges, transcripts, and receipts
- WHEN validation runs
- THEN the catalog returns stable linked refs with query receipt evidence.

#### Scenario: Unison API compatibility claim denies
- GIVEN a catalog endpoint claims compatibility with Unison Share or UCM APIs
- WHEN validation checks the non-claim boundary
- THEN it denies the claim
- AND records that those systems are prior art only.