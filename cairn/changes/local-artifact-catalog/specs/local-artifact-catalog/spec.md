# local artifact catalog Delta Spec

## ADDED Requirements

### Requirement: Define `catalog-summary-v1` records for artifact refs, kinds, payload refs, names, schemas, dependencies, dependents, effects, policy, evidence, classifications, visibility, and checks
r[molten.local_catalog.summary_dto] Define `catalog-summary-v1` records for artifact refs, kinds, payload refs, names, schemas, dependencies, dependents, effects, policy, evidence, classifications, visibility, and checks.

### Requirement: Define `catalog-query-v1`, `catalog-result-v1`, and `catalog-receipt-v1` records for list/view/search/deps/dependents/short-id operations
r[molten.local_catalog.query_result_dto] Define `catalog-query-v1`, `catalog-result-v1`, and `catalog-receipt-v1` records for list/view/search/deps/dependents/short-id operations.

### Requirement: Define `short-id-resolution-v1` with prefix, full ref, candidate count, visible candidates, decision, and ambiguity checks
r[molten.local_catalog.short_id_dto] Define `short-id-resolution-v1` with prefix, full ref, candidate count, visible candidates, decision, and ambiguity checks.

### Requirement: Document and enforce that names, aliases, tags, channels, paths, mtimes, and short ids are display handles only, never artifact identity
r[molten.local_catalog.no_name_identity] Document and enforce that names, aliases, tags, channels, paths, mtimes, and short ids are display handles only, never artifact identity.

### Requirement: Build catalog summaries from the artifact registry index, dependency/reverse indexes, payload refs, schema refs, effect refs, policy refs, and evidence refs
r[molten.local_catalog.registry_summaries] Build catalog summaries from the artifact registry index, dependency/reverse indexes, payload refs, schema refs, effect refs, policy refs, and evidence refs.

### Requirement: Merge optional local ledger artifact-kind classifications and receipt refs into catalog views without requiring a ledger for registry-only queries
r[molten.local_catalog.ledger_classifications] Merge optional local ledger artifact-kind classifications and receipt refs into catalog views without requiring a ledger for registry-only queries.

### Requirement: Implement dependency and dependent views over explicit registry edges and optional scoped closure/impact expansion
r[molten.local_catalog.dependency_views] Implement dependency and dependent views over explicit registry edges and optional scoped closure/impact expansion.

### Requirement: Render known schema/cache/transcript/rewrite/upgrade/chunk/harness/evidence records by parsing canonical Preserves shapes where available
r[molten.local_catalog.subsystem_views] Render known schema/cache/transcript/rewrite/upgrade/chunk/harness/evidence records by parsing canonical Preserves shapes where available.

### Requirement: Implement bounded conjunctive search by full ref, artifact kind, ledger kind, schema ref, structural fingerprint, effect/capability/policy/evidence ref, dependency/dependent ref, receipt operation/decision, and public text term
r[molten.local_catalog.semantic_search] Implement bounded conjunctive search by full ref, artifact kind, ledger kind, schema ref, structural fingerprint, effect/capability/policy/evidence ref, dependency/dependent ref, receipt operation/decision, and public text term.

### Requirement: Apply explicit hidden-ref filtering before summaries, views, search results, short-id candidates, and receipts are emitted
r[molten.local_catalog.visibility_filter] Apply explicit hidden-ref filtering before summaries, views, search results, short-id candidates, and receipts are emitted.

### Requirement: Render confidential/secret/protected markers as redaction markers and reserve policy/capability/redaction refs in catalog records
r[molten.local_catalog.redacted_rendering] Render confidential/secret/protected markers as redaction markers and reserve policy/capability/redaction refs in catalog records.

### Requirement: Emit deterministic catalog receipts that bind query refs, result refs, visible refs, diagnostics, and checks
r[molten.local_catalog.query_traces] Emit deterministic catalog receipts that bind query refs, result refs, visible refs, diagnostics, and checks.

### Requirement: Implement short-id prefix resolution with minimum length, ambiguity denial, visible candidate filtering, and full-ref expansion before downstream operations
r[molten.local_catalog.short_id_resolution] Implement short-id prefix resolution with minimum length, ambiguity denial, visible candidate filtering, and full-ref expansion before downstream operations.

### Requirement: Add `molten test catalog list` and `view` commands with receipt output support
r[molten.local_catalog.cli_list_view] Add `molten test catalog list` and `view` commands with receipt output support.

### Requirement: Add `search`, `deps`, `dependents`, and `short-id` commands with receipt output support
r[molten.local_catalog.cli_search_graph] Add `search`, `deps`, `dependents`, and `short-id` commands with receipt output support.

### Requirement: Add CLI flags for redacted payload rendering and hidden refs without leaking filtered refs in normal output
r[molten.local_catalog.cli_redaction] Add CLI flags for redacted payload rendering and hidden refs without leaking filtered refs in normal output.

### Requirement: Add tests for summaries over artifacts with schemas, dependencies, effects, policy refs, evidence refs, names, and ledger classifications
r[molten.local_catalog.summary_tests] Add tests for summaries over artifacts with schemas, dependencies, effects, policy refs, evidence refs, names, and ledger classifications.

### Requirement: Add tests for semantic search by schema, effect, dependency, receipt decision, transcript/rewrite/upgrade status, and public text term
r[molten.local_catalog.search_tests] Add tests for semantic search by schema, effect, dependency, receipt decision, transcript/rewrite/upgrade status, and public text term.

### Requirement: Add tests for unambiguous short ids, ambiguous denials, minimum prefix length, and hidden candidate filtering
r[molten.local_catalog.short_id_tests] Add tests for unambiguous short ids, ambiguous denials, minimum prefix length, and hidden candidate filtering.

### Requirement: Add tests that unauthorized/hidden artifacts and redacted fields are omitted or marker-rendered before output
r[molten.local_catalog.visibility_tests] Add tests that unauthorized/hidden artifacts and redacted fields are omitted or marker-rendered before output.

### Requirement: Add Hegel properties for deterministic summaries, no-name-identity invariants, short-id ambiguity monotonicity, and visibility filtering safety
r[molten.local_catalog.property_tests] Add Hegel properties for deterministic summaries, no-name-identity invariants, short-id ambiguity monotonicity, and visibility filtering safety.

