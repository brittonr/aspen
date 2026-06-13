# Local Structured Rewrite Specification

## Purpose

Defines the `local-structured-rewrite` capability.

## Requirements

### Requirement: System MUST Define local structured rewrite query DTOs over artifact kinds, dependency scope roots, canonical paths, patterns, policy/capability refs, and hidden refs
r[molten.local_rewrite.query_dto] The system MUST Define local structured rewrite query DTOs over artifact kinds, dependency scope roots, canonical paths, patterns, policy/capability refs, and hidden refs.

### Requirement: System MUST Support a bounded Preserves/schema pattern subset for first structural find operations
r[molten.local_rewrite.pattern_subset] The system MUST Support a bounded Preserves/schema pattern subset for first structural find operations.

### Requirement: System MUST Apply local hidden-ref visibility filters before returning matches
r[molten.local_rewrite.visibility_filter] The system MUST Apply local hidden-ref visibility filters before returning matches.

### Requirement: System MUST Document and enforce that text-only search/replace cannot bypass canonical artifact validation
r[molten.local_rewrite.no_text_only] The system MUST Document and enforce that text-only search/replace cannot bypass canonical artifact validation.

### Requirement: System MUST Define rewrite plan DTOs with query, replacement, matched artifacts, expected diffs, impact refs, transcript refs, migration refs, policy refs, and checks
r[molten.local_rewrite.plan_dto] The system MUST Define rewrite plan DTOs with query, replacement, matched artifacts, expected diffs, impact refs, transcript refs, migration refs, policy refs, and checks.

### Requirement: System MUST Implement dry-run preview with canonical structural diffs and reverse-dependency impact sets
r[molten.local_rewrite.preview] The system MUST Implement dry-run preview with canonical structural diffs and reverse-dependency impact sets.

### Requirement: System MUST Require explicit policy/capability refs before preview/apply
r[molten.local_rewrite.policy_admission] The system MUST Require explicit policy/capability refs before preview/apply.

### Requirement: System MUST Emit canonical receipts for query, preview, and apply
r[molten.local_rewrite.receipts] The system MUST Emit canonical receipts for query, preview, and apply.

### Requirement: System MUST Apply admitted rewrites by installing new immutable artifacts rather than mutating old ones
r[molten.local_rewrite.artifact_creation] The system MUST Apply admitted rewrites by installing new immutable artifacts rather than mutating old ones.

### Requirement: System MUST Feed applied rewrite results into upgrade-session task plans
r[molten.local_rewrite.upgrade_hook] The system MUST Feed applied rewrite results into upgrade-session task plans.

### Requirement: System MUST Bind transcript validation refs into plans/receipts for later rerun or eval-cache reuse
r[molten.local_rewrite.transcript_hook] The system MUST Bind transcript validation refs into plans/receipts for later rerun or eval-cache reuse.

### Requirement: System MUST Bind schema migration recipe refs into plans/receipts for typed-storage follow-up
r[molten.local_rewrite.schema_migration_hook] The system MUST Bind schema migration recipe refs into plans/receipts for typed-storage follow-up.

### Requirement: System MUST Add `molten test rewrite find`, `preview`, `apply`, and `show` commands
r[molten.local_rewrite.cli] The system MUST Add `molten test rewrite find`, `preview`, `apply`, and `show` commands.

### Requirement: System MUST Add tests that rewrites create new artifact ids and preserve old artifacts
r[molten.local_rewrite.apply_tests] The system MUST Add tests that rewrites create new artifact ids and preserve old artifacts.

### Requirement: System MUST Add tests that missing capabilities deny preview/apply and hidden refs are filtered
r[molten.local_rewrite.policy_tests] The system MUST Add tests that missing capabilities deny preview/apply and hidden refs are filtered.

### Requirement: System MUST Add Hegel properties for preview/apply consistency, path stability, and no-in-place-mutation invariants
r[molten.local_rewrite.property_tests] The system MUST Add Hegel properties for preview/apply consistency, path stability, and no-in-place-mutation invariants.
