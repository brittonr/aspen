## Phase 1: Query and pattern model

- [x] [serial] r[molten.local_rewrite.query_dto] Define local structured rewrite query DTOs over artifact kinds, dependency scope roots, canonical paths, patterns, policy/capability refs, and hidden refs.
- [x] [serial] r[molten.local_rewrite.pattern_subset] Support a bounded Preserves/schema pattern subset for first structural find operations.
- [x] [parallel] r[molten.local_rewrite.visibility_filter] Apply local hidden-ref visibility filters before returning matches.
- [x] [parallel] r[molten.local_rewrite.no_text_only] Document and enforce that text-only search/replace cannot bypass canonical artifact validation.

## Phase 2: Preview and plan receipts

- [x] [serial] r[molten.local_rewrite.plan_dto] Define rewrite plan DTOs with query, replacement, matched artifacts, expected diffs, impact refs, transcript refs, migration refs, policy refs, and checks.
- [x] [serial] r[molten.local_rewrite.preview] Implement dry-run preview with canonical structural diffs and reverse-dependency impact sets.
- [x] [serial] r[molten.local_rewrite.policy_admission] Require explicit policy/capability refs before preview/apply.
- [x] [parallel] r[molten.local_rewrite.receipts] Emit canonical receipts for query, preview, and apply.

## Phase 3: Apply and integration hooks

- [x] [serial] r[molten.local_rewrite.artifact_creation] Apply admitted rewrites by installing new immutable artifacts rather than mutating old ones.
- [x] [serial] r[molten.local_rewrite.upgrade_hook] Feed applied rewrite results into upgrade-session task plans.
- [x] [parallel] r[molten.local_rewrite.transcript_hook] Bind transcript validation refs into plans/receipts for later rerun or eval-cache reuse.
- [x] [parallel] r[molten.local_rewrite.schema_migration_hook] Bind schema migration recipe refs into plans/receipts for typed-storage follow-up.

## Phase 4: CLI and tests

- [x] [serial] r[molten.local_rewrite.cli] Add `molten test rewrite find`, `preview`, `apply`, and `show` commands.
- [x] [serial] r[molten.local_rewrite.apply_tests] Add tests that rewrites create new artifact ids and preserve old artifacts.
- [x] [parallel] r[molten.local_rewrite.policy_tests] Add tests that missing capabilities deny preview/apply and hidden refs are filtered.
- [x] [parallel] r[molten.local_rewrite.property_tests] Add Hegel properties for preview/apply consistency, path stability, and no-in-place-mutation invariants.
