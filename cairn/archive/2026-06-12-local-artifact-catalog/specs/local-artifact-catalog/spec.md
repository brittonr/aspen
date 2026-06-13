# local artifact catalog Delta Spec

## ADDED Requirements

### Requirement: System MUST Define `catalog-summary-v1` records for artifact refs, kinds, payload refs, names, schemas, dependencies, dependents, effects, policy, evidence, classifications, visibility, and checks
r[molten.local_catalog.summary_dto] The system MUST Define `catalog-summary-v1` records for artifact refs, kinds, payload refs, names, schemas, dependencies, dependents, effects, policy, evidence, classifications, visibility, and checks.

### Requirement: System MUST Define `catalog-query-v1`, `catalog-result-v1`, and `catalog-receipt-v1` records for list/view/search/deps/dependents/short-id operations
r[molten.local_catalog.query_result_dto] The system MUST Define `catalog-query-v1`, `catalog-result-v1`, and `catalog-receipt-v1` records for list/view/search/deps/dependents/short-id operations.

### Requirement: System MUST Define `short-id-resolution-v1` with prefix, full ref, candidate count, visible candidates, decision, and ambiguity checks
r[molten.local_catalog.short_id_dto] The system MUST Define `short-id-resolution-v1` with prefix, full ref, candidate count, visible candidates, decision, and ambiguity checks.

### Requirement: System MUST Document and enforce that names, aliases, tags, channels, paths, mtimes, and short ids are display handles only, never artifact identity
r[molten.local_catalog.no_name_identity] The system MUST Document and enforce that names, aliases, tags, channels, paths, mtimes, and short ids are display handles only, never artifact identity.

### Requirement: System MUST Build catalog summaries from the artifact registry index, dependency/reverse indexes, payload refs, schema refs, effect refs, policy refs, and evidence refs
r[molten.local_catalog.registry_summaries] The system MUST Build catalog summaries from the artifact registry index, dependency/reverse indexes, payload refs, schema refs, effect refs, policy refs, and evidence refs.

### Requirement: System MUST Merge optional local ledger artifact-kind classifications and receipt refs into catalog views without requiring a ledger for registry-only queries
r[molten.local_catalog.ledger_classifications] The system MUST Merge optional local ledger artifact-kind classifications and receipt refs into catalog views without requiring a ledger for registry-only queries.

### Requirement: System MUST Implement dependency and dependent views over explicit registry edges and optional scoped closure/impact expansion
r[molten.local_catalog.dependency_views] The system MUST Implement dependency and dependent views over explicit registry edges and optional scoped closure/impact expansion.

### Requirement: System MUST Render known schema/cache/transcript/rewrite/upgrade/chunk/harness/evidence records by parsing canonical Preserves shapes where available
r[molten.local_catalog.subsystem_views] The system MUST Render known schema/cache/transcript/rewrite/upgrade/chunk/harness/evidence records by parsing canonical Preserves shapes where available.

### Requirement: System MUST Implement bounded conjunctive search by full ref, artifact kind, ledger kind, schema ref, structural fingerprint, effect/capability/policy/evidence ref, dependency/dependent ref, receipt operation/decision, and public text term
r[molten.local_catalog.semantic_search] The system MUST Implement bounded conjunctive search by full ref, artifact kind, ledger kind, schema ref, structural fingerprint, effect/capability/policy/evidence ref, dependency/dependent ref, receipt operation/decision, and public text term.

### Requirement: System MUST Apply explicit hidden-ref filtering before summaries, views, search results, short-id candidates, and receipts are emitted
r[molten.local_catalog.visibility_filter] The system MUST Apply explicit hidden-ref filtering before summaries, views, search results, short-id candidates, and receipts are emitted.

### Requirement: System MUST Render confidential/secret/protected markers as redaction markers and reserve policy/capability/redaction refs in catalog records
r[molten.local_catalog.redacted_rendering] The system MUST Render confidential/secret/protected markers as redaction markers and reserve policy/capability/redaction refs in catalog records.

### Requirement: System MUST Emit deterministic catalog receipts that bind query refs, result refs, visible refs, diagnostics, and checks
r[molten.local_catalog.query_traces] The system MUST Emit deterministic catalog receipts that bind query refs, result refs, visible refs, diagnostics, and checks.

### Requirement: System MUST Implement short-id prefix resolution with minimum length, ambiguity denial, visible candidate filtering, and full-ref expansion before downstream operations
r[molten.local_catalog.short_id_resolution] The system MUST Implement short-id prefix resolution with minimum length, ambiguity denial, visible candidate filtering, and full-ref expansion before downstream operations.

### Requirement: System MUST Add `molten test catalog list` and `view` commands with receipt output support
r[molten.local_catalog.cli_list_view] The system MUST Add `molten test catalog list` and `view` commands with receipt output support.

### Requirement: System MUST Add `search`, `deps`, `dependents`, and `short-id` commands with receipt output support
r[molten.local_catalog.cli_search_graph] The system MUST Add `search`, `deps`, `dependents`, and `short-id` commands with receipt output support.

### Requirement: System MUST Add CLI flags for redacted payload rendering and hidden refs without leaking filtered refs in normal output
r[molten.local_catalog.cli_redaction] The system MUST Add CLI flags for redacted payload rendering and hidden refs without leaking filtered refs in normal output.

### Requirement: System MUST Add tests for summaries over artifacts with schemas, dependencies, effects, policy refs, evidence refs, names, and ledger classifications
r[molten.local_catalog.summary_tests] The system MUST Add tests for summaries over artifacts with schemas, dependencies, effects, policy refs, evidence refs, names, and ledger classifications.

### Requirement: System MUST Add tests for semantic search by schema, effect, dependency, receipt decision, transcript/rewrite/upgrade status, and public text term
r[molten.local_catalog.search_tests] The system MUST Add tests for semantic search by schema, effect, dependency, receipt decision, transcript/rewrite/upgrade status, and public text term.

### Requirement: System MUST Add tests for unambiguous short ids, ambiguous denials, minimum prefix length, and hidden candidate filtering
r[molten.local_catalog.short_id_tests] The system MUST Add tests for unambiguous short ids, ambiguous denials, minimum prefix length, and hidden candidate filtering.

### Requirement: System MUST Add tests that unauthorized/hidden artifacts and redacted fields are omitted or marker-rendered before output
r[molten.local_catalog.visibility_tests] The system MUST Add tests that unauthorized/hidden artifacts and redacted fields are omitted or marker-rendered before output.

### Requirement: System MUST Add Hegel properties for deterministic summaries, no-name-identity invariants, short-id ambiguity monotonicity, and visibility filtering safety
r[molten.local_catalog.property_tests] The system MUST Add Hegel properties for deterministic summaries, no-name-identity invariants, short-id ambiguity monotonicity, and visibility filtering safety.

