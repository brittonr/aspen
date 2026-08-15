# runtime-spine Spec Delta

## Requirements

### Requirement: Retention GC audit catalog search
r[molten.retention.gc_audit_catalog_search] Molten MUST classify retention GC plan, apply, execute, and audit artifacts in read-only catalog and MCP search results by stage, decision, subsystem, object ref, retention class, and chain refs while preserving normal retention deletion gates as the only destructive authority path.

#### Scenario: Catalog finds retention GC chains by scope
- GIVEN retention GC plan, apply, execute, and audit artifacts for a destructive candidate
- WHEN an operator searches the local catalog by object ref, subsystem, execution ref, or ledger kind
- THEN Molten returns the matching artifacts with retention GC classifications for plan, apply, execute, audit, and linked refs

#### Scenario: MCP search is read-only discovery
- GIVEN retention GC audit artifacts imported into the local ledger
- WHEN an MCP client calls the read-only `search_retention_gc` tool with stage, object, subsystem, decision, plan, apply, or execution filters
- THEN Molten returns catalog search evidence without mutating retention state or granting deletion authority

#### Scenario: Catalog discovery remains explanatory evidence
- GIVEN a passing catalog or MCP result for a retention GC audit chain
- WHEN a destructive subsystem later attempts deletion, tombstoning, redaction, compaction, or invalidation
- THEN the subsystem MUST still require matching plan/apply/execution gates plus normal destructive admission and MUST NOT treat catalog or MCP discovery as authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, or deletion trust
