## Context

`JobAnalytics::query()` currently computes the built-in analytics directly from KV data and only logs the SQL text produced by `build_sql(...)`. That means the broken SQL builder has stayed latent: the generated queries look plausible at a glance, but some optional-filter combinations are syntactically invalid.

Two concrete bugs were identified:

- `build_sql_average_duration()` can produce `FROM jobs AND status = ...` when `status` is present without `job_type`.
- `build_sql_queue_depth()` can produce `WHERE status = 'Queued' WHERE priority = ...` when `priority` is present.

The fix should make predicate assembly structurally correct instead of continuing to concatenate partial `WHERE` and `AND` fragments by hand.

## Goals / Non-Goals

**Goals:**

- Guarantee syntactically valid SQL for every optional-filter combination supported by the analytics query builders.
- Reduce the chance of future query-builder regressions by using a shared predicate-assembly helper.
- Add unit tests that cover the combinations the current tests miss.

**Non-Goals:**

- Replacing the direct KV analytics implementation with DataFusion in this change.
- Designing a full SQL parameter-binding abstraction.
- General SQL injection hardening for free-form custom SQL supplied by callers.

## Decisions

### D1: Build predicates as clauses, not preformatted `WHERE`/`AND` snippets

The current bug exists because each optional filter includes its own leading keyword, and the builder then concatenates them without knowing whether an earlier clause exists.

**Decision:** introduce a small helper that accepts a list of predicate clauses and appends them to the base query with exactly one `WHERE`, using `AND` between additional clauses.

That keeps the grammar correct regardless of which optional filters are present.

### D2: Keep the fix local to the analytics SQL builders

Only a handful of analytics queries currently construct optional predicates, and the rest of the file already separates query selection from string generation.

**Decision:** keep the helper in `crates/aspen-jobs/src/analytics.rs` and apply it to the affected builders first rather than introducing a workspace-wide SQL DSL.

### D3: Test the cross-product of optional filters

The existing tests only spot-check a few happy paths. They do not cover `status without job_type`, `priority present`, or any assertion about duplicate `WHERE` tokens.

**Decision:** add unit tests for all optional-filter combinations on the affected builders and assert structural properties such as:

- query contains at most one `WHERE`,
- `AND` only appears after `WHERE`,
- the expected predicates appear when each option is present.

## Risks / Trade-offs

- **[Risk]** A local helper may later diverge from other SQL-building code. -> Mitigation: keep it small and only expand it if more query builders need the same pattern.
- **[Risk]** The change improves syntax but not escaping or parameter binding. -> Mitigation: document that as a follow-up if these strings become executable against a real SQL engine.
- **[Risk]** Tests that assert exact query strings can become brittle. -> Mitigation: prefer structural assertions plus a few representative exact-match cases.
