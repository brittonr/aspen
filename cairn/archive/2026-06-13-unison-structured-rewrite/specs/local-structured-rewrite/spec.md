# Local Structured Rewrite Delta Spec

## Requirements

### Requirement: Structured rewrite query DTOs MUST be canonical and bounded
r[molten.rewrite.query_model] Molten MUST define structured query DTOs over artifact kinds, dependency scopes, canonical paths, patterns, bindings, and constraints.
r[molten.local_rewrite.query_dto] Molten MUST define local structured rewrite query DTOs over artifact kinds, dependency scope roots, canonical paths, patterns, policy/capability refs, and hidden refs.

#### Scenario: Query DTO binds scope and pattern
- GIVEN a structured rewrite query over local artifacts
- WHEN the query DTO is emitted
- THEN it binds artifact kinds, root refs, dependency traversal, canonical pattern, policy refs, capability refs, and hidden refs.

### Requirement: Structural find MUST use a bounded Preserves/schema pattern subset
r[molten.rewrite.preserves_patterns] Molten MUST support a bounded Preserves/schema/manifest pattern subset for first structural find operations.
r[molten.local_rewrite.pattern_subset] Molten MUST support a bounded Preserves/schema pattern subset for first structural find operations.
r[molten.rewrite.find_tests] Molten MUST add tests for structural find over schema and protocol manifest artifacts.

#### Scenario: Structural find matches schema shape
- GIVEN a schema or manifest artifact with a matching canonical shape
- WHEN structured find runs with the bounded pattern subset
- THEN the result identifies matching artifact refs, paths, value refs, and previews.

### Requirement: Visibility policy MUST filter hidden or unauthorized refs before returning results
r[molten.rewrite.visibility_policy] Molten MUST apply capability and policy visibility limits before returning query results.
r[molten.local_rewrite.visibility_filter] Molten MUST apply local hidden-ref visibility filters before returning matches.

#### Scenario: Hidden ref is filtered
- GIVEN a query with hidden refs
- WHEN structured find evaluates matching artifacts
- THEN hidden artifacts are omitted before results are returned.

### Requirement: Text-only replacement MUST NOT bypass canonical artifact validation
r[molten.rewrite.no_text_only] Molten MUST document that text search/replace cannot bypass canonical artifact validation.
r[molten.local_rewrite.no_text_only] Molten MUST document and enforce that text-only search/replace cannot bypass canonical artifact validation.

#### Scenario: Rewrite remains canonical
- GIVEN a replacement that changes rendered text
- WHEN preview or apply runs
- THEN Molten validates canonical artifact payloads and emits receipts instead of mutating source text in place.

### Requirement: Rewrite plans MUST bind matches, replacement, expected artifacts, impact, validation, rollback, and evidence
r[molten.rewrite.plan_model] Molten MUST define rewrite plan DTOs with matched artifacts, replacement template/transformer, expected new artifacts, validation requirements, rollback, and evidence refs.
r[molten.local_rewrite.plan_dto] Molten MUST define rewrite plan DTOs with query, replacement, matched artifacts, expected diffs, impact refs, transcript refs, migration refs, policy refs, and checks.

#### Scenario: Plan DTO binds query and diffs
- GIVEN a rewrite preview with matched artifacts
- WHEN a rewrite plan is emitted
- THEN it binds the query, replacement, match refs, expected diffs, impacted refs, transcript refs, schema migration refs, policy refs, and checks.

### Requirement: Preview MUST be dry-run and include canonical diffs plus impact sets
r[molten.rewrite.preview] Molten MUST implement dry-run preview with canonical structural diffs, rendered diffs when available, and dependency impact sets.
r[molten.local_rewrite.preview] Molten MUST implement dry-run preview with canonical structural diffs and reverse-dependency impact sets.

#### Scenario: Preview does not mutate artifacts
- GIVEN a rewrite plan over an immutable artifact
- WHEN preview runs
- THEN old artifact content remains unchanged and preview emits canonical diffs plus impacted refs.

### Requirement: Rewrite preview and apply MUST require explicit policy/capability admission
r[molten.rewrite.policy_admission] Molten MUST gate rewrite plan admission through Nickel/Basalt/Trellis policy and required capabilities.
r[molten.local_rewrite.policy_admission] Molten MUST require explicit policy/capability refs before preview/apply.

#### Scenario: Missing capability denies preview
- GIVEN a rewrite plan without capability refs
- WHEN preview or apply is requested
- THEN Molten denies before artifact creation or metadata mutation.

### Requirement: Rewrite actions MUST emit canonical receipts
r[molten.rewrite.receipts] Molten MUST emit Cairn receipts for query, preview, plan admission, artifact creation, validation, and metadata application.
r[molten.local_rewrite.receipts] Molten MUST emit canonical receipts for query, preview, and apply.

#### Scenario: Query preview and apply are receipted
- GIVEN a structured rewrite workflow
- WHEN query, preview, and apply operations run
- THEN each operation emits a canonical receipt binding subject refs, decision, diagnostics, checks, and related refs.

### Requirement: Applying rewrites MUST create new immutable artifacts
r[molten.rewrite.artifact_creation] Molten MUST apply admitted rewrites by creating new immutable artifacts rather than mutating old ones.
r[molten.local_rewrite.artifact_creation] Molten MUST apply admitted rewrites by installing new immutable artifacts rather than mutating old ones.
r[molten.rewrite.apply_tests] Molten MUST add tests that rewrites create new artifact ids and preserve old artifacts.
r[molten.local_rewrite.apply_tests] Molten MUST add tests that rewrites create new artifact ids and preserve old artifacts.

#### Scenario: Apply preserves old artifact
- GIVEN an admitted rewrite over an artifact
- WHEN apply runs
- THEN a new artifact ref is installed and the original artifact payload remains addressable and unchanged.

### Requirement: Rewrite results MUST feed upgrade-session planning
r[molten.rewrite.upgrade_session_hook] Molten MUST feed admitted rewrite plans into structured upgrade sessions as task sources.
r[molten.local_rewrite.upgrade_hook] Molten MUST feed applied rewrite results into upgrade-session task plans.

#### Scenario: Applied rewrite builds upgrade plan
- GIVEN an applied rewrite with installed artifact refs
- WHEN the upgrade-session hook runs
- THEN the generated plan includes install tasks and metadata/cutover checks bound to rewrite evidence.

### Requirement: Rewrite plans MUST bind transcript and evaluation-cache validation refs
r[molten.rewrite.transcript_validation] Molten MUST rerun selected executable transcripts or reuse valid evaluation-cache results before cutover.
r[molten.local_rewrite.transcript_hook] Molten MUST bind transcript validation refs into plans/receipts for later rerun or eval-cache reuse.

#### Scenario: Transcript refs are preserved for cutover validation
- GIVEN a rewrite plan with transcript validation refs
- WHEN preview/apply receipts are emitted
- THEN those refs remain bound for later rerun or evaluation-cache reuse before cutover.

### Requirement: Rewrite plans MUST propose schema migration follow-up when schemas change
r[molten.rewrite.schema_migration_hook] Molten MUST use rewrite plans to propose typed-storage migration recipe artifacts where schema identity changes.
r[molten.local_rewrite.schema_migration_hook] Molten MUST bind schema migration recipe refs into plans/receipts for typed-storage follow-up.

#### Scenario: Schema migration refs are preserved
- GIVEN a rewrite plan that affects schema identity
- WHEN the plan or receipt is emitted
- THEN schema migration recipe refs are bound for typed-storage follow-up.

### Requirement: Rewrite CLI MUST expose find, preview, apply, and show commands
r[molten.local_rewrite.cli] Molten MUST add `molten test rewrite find`, `preview`, `apply`, and `show` commands.

#### Scenario: Rewrite commands render canonical receipts
- GIVEN a local rewrite command invocation
- WHEN find, preview, apply, or show runs
- THEN the command renders canonical rewrite artifacts or receipts without bypassing the pure rewrite core.

### Requirement: Rewrite policy and property tests MUST cover denial and deterministic invariants
r[molten.rewrite.policy_tests] Molten MUST add tests that unauthorized artifact scopes are hidden or denied.
r[molten.local_rewrite.policy_tests] Molten MUST add tests that missing capabilities deny preview/apply and hidden refs are filtered.
r[molten.rewrite.property_tests] Molten MUST add Hegel property tests for preview/apply consistency, path stability, and no-in-place-mutation invariants.
r[molten.local_rewrite.property_tests] Molten MUST add Hegel properties for preview/apply consistency, path stability, and no-in-place-mutation invariants.

#### Scenario: Preview/apply paths are stable
- GIVEN the same rewrite input generated twice
- WHEN preview runs twice and apply runs once
- THEN preview paths and new payload refs are stable and the old artifact is not mutated in place.
