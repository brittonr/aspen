# Local Structured Rewrite Specification

## Purpose

Defines Molten's local structured rewrite capability for canonical artifact queries, dry-run previews, immutable artifact creation, and upgrade-session integration.

## Requirements

### Requirement: Structured rewrite query DTOs MUST be canonical and bounded
r[molten.rewrite.query_model] Molten MUST define structured query DTOs over artifact kinds, dependency scopes, canonical paths, patterns, bindings, and constraints.

#### Scenario: Structured query model is bounded
- GIVEN a structured artifact query
- WHEN its DTO is defined
- THEN artifact kinds, dependency scopes, canonical paths, patterns, bindings, and constraints remain explicit.

### Requirement: Local query DTOs bind policy and hidden references
r[molten.local_rewrite.query_dto] Molten MUST define local structured rewrite query DTOs over artifact kinds, dependency scope roots, canonical paths, patterns, policy/capability refs, and hidden refs.

#### Scenario: Query DTO binds scope and pattern
- GIVEN a structured rewrite query over local artifacts
- WHEN the query DTO is emitted
- THEN it binds artifact kinds, root refs, dependency traversal, canonical pattern, policy refs, capability refs, and hidden refs.

### Requirement: Structural find MUST use a bounded Preserves/schema pattern subset
r[molten.rewrite.preserves_patterns] Molten MUST support a bounded Preserves/schema/manifest pattern subset for first structural find operations.

#### Scenario: Initial structural patterns have a bounded vocabulary
- GIVEN a first structural find operation
- WHEN its supported patterns are selected
- THEN they are limited to the declared Preserves/schema/manifest subset.

### Requirement: Local structural find uses the bounded pattern subset
r[molten.local_rewrite.pattern_subset] Molten MUST support a bounded Preserves/schema pattern subset for first structural find operations.

#### Scenario: Local pattern admission preserves the subset
- GIVEN a local structural query
- WHEN its pattern is admitted
- THEN only the bounded Preserves/schema pattern subset is supported.

### Requirement: Structural find has schema and manifest test coverage
r[molten.rewrite.find_tests] Molten MUST add tests for structural find over schema and protocol manifest artifacts.

#### Scenario: Structural find matches schema shape
- GIVEN a schema or manifest artifact with a matching canonical shape
- WHEN structured find runs with the bounded pattern subset
- THEN the result identifies matching artifact refs, paths, value refs, and previews.

### Requirement: Visibility policy MUST filter hidden or unauthorized refs before returning results
r[molten.rewrite.visibility_policy] Molten MUST apply capability and policy visibility limits before returning query results.

#### Scenario: Query visibility applies before publication
- GIVEN a query matching an unauthorized artifact scope
- WHEN query results are produced
- THEN capability and policy visibility limits apply before the results are returned.

### Requirement: Local find filters hidden references
r[molten.local_rewrite.visibility_filter] Molten MUST apply local hidden-ref visibility filters before returning matches.

#### Scenario: Hidden ref is filtered
- GIVEN a query with hidden refs
- WHEN structured find evaluates matching artifacts
- THEN hidden artifacts are omitted before results are returned.

### Requirement: Text-only replacement MUST NOT bypass canonical artifact validation
r[molten.rewrite.no_text_only] Molten MUST document that text search/replace cannot bypass canonical artifact validation.

#### Scenario: Canonical validation cannot be bypassed by text replacement
- GIVEN documentation for text search and replacement
- WHEN rewrite usage is described
- THEN it states that canonical artifact validation cannot be bypassed.

### Requirement: Local text replacement enforces canonical validation
r[molten.local_rewrite.no_text_only] Molten MUST document and enforce that text-only search/replace cannot bypass canonical artifact validation.

#### Scenario: Rewrite remains canonical
- GIVEN a replacement that changes rendered text
- WHEN preview or apply runs
- THEN Molten validates canonical artifact payloads and emits receipts instead of mutating source text in place.

### Requirement: Rewrite plans MUST bind matches, replacement, expected artifacts, impact, validation, rollback, and evidence
r[molten.rewrite.plan_model] Molten MUST define rewrite plan DTOs with matched artifacts, replacement template/transformer, expected new artifacts, validation requirements, rollback, and evidence refs.

#### Scenario: Rewrite plan models bind validation and rollback
- GIVEN a proposed structured rewrite
- WHEN its plan DTO is defined
- THEN matches, replacement, expected artifacts, validation, rollback, and evidence refs are bound.

### Requirement: Local rewrite plans bind query, diffs, impact, and checks
r[molten.local_rewrite.plan_dto] Molten MUST define rewrite plan DTOs with query, replacement, matched artifacts, expected diffs, impact refs, transcript refs, migration refs, policy refs, and checks.

#### Scenario: Plan DTO binds query and diffs
- GIVEN a rewrite preview with matched artifacts
- WHEN a rewrite plan is emitted
- THEN it binds the query, replacement, match refs, expected diffs, impacted refs, transcript refs, schema migration refs, policy refs, and checks.

### Requirement: Preview MUST be dry-run and include canonical diffs plus impact sets
r[molten.rewrite.preview] Molten MUST implement dry-run preview with canonical structural diffs, rendered diffs when available, and dependency impact sets.

#### Scenario: Structural preview retains available rendered diffs
- GIVEN an immutable artifact and a proposed rewrite
- WHEN dry-run preview runs
- THEN it returns canonical structural diffs, rendered diffs when available, and dependency impact sets without applying the rewrite.

### Requirement: Local previews report reverse-dependency impact
r[molten.local_rewrite.preview] Molten MUST implement dry-run preview with canonical structural diffs and reverse-dependency impact sets.

#### Scenario: Preview does not mutate artifacts
- GIVEN a rewrite plan over an immutable artifact
- WHEN preview runs
- THEN old artifact content remains unchanged and preview emits canonical diffs plus impacted refs.

### Requirement: Rewrite preview and apply MUST require explicit policy/capability admission
r[molten.rewrite.policy_admission] Molten MUST gate rewrite plan admission through Nickel/Basalt/Trellis policy and required capabilities.

#### Scenario: Rewrite plan admission checks policy and capabilities
- GIVEN a structured rewrite plan
- WHEN plan admission is evaluated
- THEN Nickel/Basalt/Trellis policy and required capabilities gate admission.

### Requirement: Local preview and apply require explicit authority references
r[molten.local_rewrite.policy_admission] Molten MUST require explicit policy/capability refs before preview/apply.

#### Scenario: Missing capability denies preview
- GIVEN a rewrite plan without capability refs
- WHEN preview or apply is requested
- THEN Molten denies before artifact creation or metadata mutation.

### Requirement: Rewrite actions MUST emit canonical receipts
r[molten.rewrite.receipts] Molten MUST emit Cairn receipts for query, preview, plan admission, artifact creation, validation, and metadata application.

#### Scenario: Rewrite workflow stages retain receipt evidence
- GIVEN query, preview, admission, creation, validation, and metadata-application stages
- WHEN a rewrite workflow performs a stage
- THEN it emits the required Cairn receipt for that stage.

### Requirement: Local query, preview, and apply emit canonical receipts
r[molten.local_rewrite.receipts] Molten MUST emit canonical receipts for query, preview, and apply.

#### Scenario: Query preview and apply are receipted
- GIVEN a structured rewrite workflow
- WHEN query, preview, and apply operations run
- THEN each operation emits a canonical receipt binding subject refs, decision, diagnostics, checks, and related refs.

### Requirement: Applying rewrites MUST create new immutable artifacts
r[molten.rewrite.artifact_creation] Molten MUST apply admitted rewrites by creating new immutable artifacts rather than mutating old ones.

#### Scenario: Rewrite creation leaves the old artifact immutable
- GIVEN an admitted rewrite over an existing artifact
- WHEN the replacement artifact is created
- THEN the existing artifact is not mutated in place.

### Requirement: Local rewrite installation preserves immutable artifacts
r[molten.local_rewrite.artifact_creation] Molten MUST apply admitted rewrites by installing new immutable artifacts rather than mutating old ones.

#### Scenario: Local apply installs a new artifact
- GIVEN an admitted local rewrite
- WHEN apply installs its result
- THEN it installs a new immutable artifact rather than mutating the old one.

### Requirement: Rewrite application tests cover artifact identity preservation
r[molten.rewrite.apply_tests] Molten MUST add tests that rewrites create new artifact ids and preserve old artifacts.

#### Scenario: Rewrite application identity behavior is tested
- GIVEN rewrite application tests
- WHEN they exercise applying a replacement
- THEN they check new artifact ids and preservation of the old artifacts.

### Requirement: Local rewrite application tests preserve old artifacts
r[molten.local_rewrite.apply_tests] Molten MUST add tests that rewrites create new artifact ids and preserve old artifacts.

#### Scenario: Apply preserves old artifact
- GIVEN an admitted rewrite over an artifact
- WHEN apply runs
- THEN a new artifact ref is installed and the original artifact payload remains addressable and unchanged.

### Requirement: Rewrite results MUST feed upgrade-session planning
r[molten.rewrite.upgrade_session_hook] Molten MUST feed admitted rewrite plans into structured upgrade sessions as task sources.

#### Scenario: Admitted rewrite plans feed structured upgrade sessions
- GIVEN an admitted rewrite plan
- WHEN an upgrade session is planned
- THEN the rewrite plan is available as a structured task source.

### Requirement: Local applied rewrites feed upgrade-session plans
r[molten.local_rewrite.upgrade_hook] Molten MUST feed applied rewrite results into upgrade-session task plans.

#### Scenario: Applied rewrite builds upgrade plan
- GIVEN an applied rewrite with installed artifact refs
- WHEN the upgrade-session hook runs
- THEN the generated plan includes install tasks and metadata/cutover checks bound to rewrite evidence.

### Requirement: Rewrite plans MUST bind transcript and evaluation-cache validation refs
r[molten.rewrite.transcript_validation] Molten MUST rerun selected executable transcripts or reuse valid evaluation-cache results before cutover.

#### Scenario: Cutover uses selected transcript validation
- GIVEN an executable transcript selected for a rewrite
- WHEN cutover validation is performed
- THEN the transcript is rerun or a valid evaluation-cache result is reused.

### Requirement: Local rewrite evidence preserves transcript references
r[molten.local_rewrite.transcript_hook] Molten MUST bind transcript validation refs into plans/receipts for later rerun or eval-cache reuse.

#### Scenario: Transcript refs are preserved for cutover validation
- GIVEN a rewrite plan with transcript validation refs
- WHEN preview/apply receipts are emitted
- THEN those refs remain bound for later rerun or evaluation-cache reuse before cutover.

### Requirement: Rewrite plans MUST propose schema migration follow-up when schemas change
r[molten.rewrite.schema_migration_hook] Molten MUST use rewrite plans to propose typed-storage migration recipe artifacts where schema identity changes.

#### Scenario: Schema identity changes propose migration recipes
- GIVEN a rewrite that changes schema identity
- WHEN follow-up is proposed
- THEN the rewrite plan proposes typed-storage migration recipe artifacts.

### Requirement: Local rewrite evidence binds schema-migration references
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

#### Scenario: Unauthorized rewrite scopes have negative test coverage
- GIVEN rewrite policy tests
- WHEN they exercise unauthorized artifact scopes
- THEN the scopes are checked to be hidden or denied.

### Requirement: Local rewrite policy tests cover capability and visibility denial
r[molten.local_rewrite.policy_tests] Molten MUST add tests that missing capabilities deny preview/apply and hidden refs are filtered.

#### Scenario: Local authority and visibility boundaries are tested
- GIVEN local rewrite policy tests
- WHEN capabilities are missing or references are hidden
- THEN the tests cover preview/apply denial and hidden-ref filtering.

### Requirement: Rewrite properties cover determinism and immutability
r[molten.rewrite.property_tests] Molten MUST add Hegel property tests for preview/apply consistency, path stability, and no-in-place-mutation invariants.

#### Scenario: Rewrite invariants have Hegel property coverage
- GIVEN generated structured rewrite inputs
- WHEN Hegel properties exercise preview and apply
- THEN they cover consistency, path stability, and absence of in-place mutation.

### Requirement: Local rewrite properties preserve consistency and immutable payloads
r[molten.local_rewrite.property_tests] Molten MUST add Hegel properties for preview/apply consistency, path stability, and no-in-place-mutation invariants.

#### Scenario: Preview/apply paths are stable
- GIVEN the same rewrite input generated twice
- WHEN preview runs twice and apply runs once
- THEN preview paths and new payload refs are stable and the old artifact is not mutated in place.

### Requirement: Rewrite upgrade plans require explicit authority and authenticated evidence
r[molten.local_rewrite.upgrade_hook_authority] The rewrite upgrade-session hook MUST require explicit operator-supplied initiator, capability and policy refs and authenticated source-gate evidence whose strict current-workspace validation passes. It MUST NOT derive authority refs from labels or session ids. Missing, unauthenticated, stale or unavailable evidence and invalid refs MUST fail before applying the rewrite or writing an upgrade plan.

#### Scenario: Missing evidence or refs leaves no plan
- GIVEN `molten rewrite apply --upgrade-plan-out` without an authenticated source state root, with a forged gate or trust file, or without explicit initiator, capability or policy refs
- WHEN the command runs
- THEN it fails, writes no upgrade plan and installs no rewritten artifact

#### Scenario: Explicit refs and authenticated evidence produce a plan
- GIVEN a valid rewrite request, explicit canonical refs and an authenticated current source gate
- WHEN the combined upgrade-plan entrypoint admits and applies the rewrite
- THEN the plan binds those refs, install tasks and the authenticated gate

#### Scenario: Valid produced gate is stale for the workspace
- GIVEN an authenticated produced gate whose receipt structure passes but whose workspace config hash is stale, or no current workspace is available
- WHEN the combined upgrade-plan entrypoint runs
- THEN admission fails before any replacement installation or upgrade-plan publication
