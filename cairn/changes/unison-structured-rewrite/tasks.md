## Phase 1: Query and pattern model

- [ ] [serial] r[molten.rewrite.query_model] Define structured query DTOs over artifact kinds, dependency scopes, canonical paths, patterns, bindings, and constraints.
- [ ] [serial] r[molten.rewrite.preserves_patterns] Support a bounded Preserves/schema/manifest pattern subset for first structural find operations.
- [ ] [parallel] r[molten.rewrite.visibility_policy] Apply capability and policy visibility limits before returning query results.
- [ ] [parallel] r[molten.rewrite.no_text_only] Document that text search/replace cannot bypass canonical artifact validation.

## Phase 2: Rewrite plans

- [ ] [serial] r[molten.rewrite.plan_model] Define rewrite plan DTOs with matched artifacts, replacement template/transformer, expected new artifacts, validation requirements, rollback, and evidence refs.
- [ ] [serial] r[molten.rewrite.preview] Implement dry-run preview with canonical structural diffs, rendered diffs when available, and dependency impact sets.
- [ ] [serial] r[molten.rewrite.policy_admission] Gate rewrite plan admission through Nickel/Basalt/Trellis policy and required capabilities.
- [ ] [parallel] r[molten.rewrite.receipts] Emit Cairn receipts for query, preview, plan admission, artifact creation, validation, and metadata application.

## Phase 3: Integration

- [ ] [serial] r[molten.rewrite.artifact_creation] Apply admitted rewrites by creating new immutable artifacts rather than mutating old ones.
- [ ] [serial] r[molten.rewrite.upgrade_session_hook] Feed admitted rewrite plans into structured upgrade sessions as task sources.
- [ ] [parallel] r[molten.rewrite.transcript_validation] Rerun selected executable transcripts or reuse valid evaluation-cache results before cutover.
- [ ] [parallel] r[molten.rewrite.schema_migration_hook] Use rewrite plans to propose typed-storage migration recipe artifacts where schema identity changes.

## Phase 4: Tests

- [ ] [serial] r[molten.rewrite.find_tests] Add tests for structural find over schema and protocol manifest artifacts.
- [ ] [serial] r[molten.rewrite.apply_tests] Add tests that rewrites create new artifact ids and preserve old artifacts.
- [ ] [parallel] r[molten.rewrite.policy_tests] Add tests that unauthorized artifact scopes are hidden or denied.
- [ ] [parallel] r[molten.rewrite.property_tests] Add Hegel property tests for preview/apply consistency, path stability, and no-in-place-mutation invariants.
