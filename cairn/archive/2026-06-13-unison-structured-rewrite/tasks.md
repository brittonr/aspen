## Phase 1: Query and pattern model

- [x] [serial] r[molten.rewrite.query_model] Define structured query DTOs over artifact kinds, dependency scopes, canonical paths, patterns, bindings, and constraints.
- [x] [serial] r[molten.rewrite.preserves_patterns] Support a bounded Preserves/schema/manifest pattern subset for first structural find operations.
- [x] [parallel] r[molten.rewrite.visibility_policy] Apply capability and policy visibility limits before returning query results.
- [x] [parallel] r[molten.rewrite.no_text_only] Document that text search/replace cannot bypass canonical artifact validation.

## Phase 2: Rewrite plans

- [x] [serial] r[molten.rewrite.plan_model] Define rewrite plan DTOs with matched artifacts, replacement template/transformer, expected new artifacts, validation requirements, rollback, and evidence refs.
- [x] [serial] r[molten.rewrite.preview] Implement dry-run preview with canonical structural diffs, rendered diffs when available, and dependency impact sets.
- [x] [serial] r[molten.rewrite.policy_admission] Gate rewrite plan admission through Nickel/Basalt/Trellis policy and required capabilities.
- [x] [parallel] r[molten.rewrite.receipts] Emit Cairn receipts for query, preview, plan admission, artifact creation, validation, and metadata application.

## Phase 3: Integration

- [x] [serial] r[molten.rewrite.artifact_creation] Apply admitted rewrites by creating new immutable artifacts rather than mutating old ones.
- [x] [serial] r[molten.rewrite.upgrade_session_hook] Feed admitted rewrite plans into structured upgrade sessions as task sources.
- [x] [parallel] r[molten.rewrite.transcript_validation] Rerun selected executable transcripts or reuse valid evaluation-cache results before cutover.
- [x] [parallel] r[molten.rewrite.schema_migration_hook] Use rewrite plans to propose typed-storage migration recipe artifacts where schema identity changes.

## Phase 4: Tests

- [x] [serial] r[molten.rewrite.find_tests] Add tests for structural find over schema and protocol manifest artifacts.
- [x] [serial] r[molten.rewrite.apply_tests] Add tests that rewrites create new artifact ids and preserve old artifacts.
- [x] [parallel] r[molten.rewrite.policy_tests] Add tests that unauthorized artifact scopes are hidden or denied.
- [x] [parallel] r[molten.rewrite.property_tests] Add Hegel property tests for preview/apply consistency, path stability, and no-in-place-mutation invariants.
