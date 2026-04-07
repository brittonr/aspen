## Why

`crates/aspen-jobs/src/analytics.rs` builds SQL strings with optional filters, but two query builders compose invalid SQL when only some filters are present.

The bug is easy to miss today because `JobAnalytics::query()` still executes direct KV calculations for the built-in analytics queries and only logs the generated SQL. As soon as the planned SQL-backed execution path is wired in, these queries will fail for normal inputs.

## What We Know

Two concrete cases are broken:

1. `build_sql_average_duration()`
   - `type_filter` is prefixed with `WHERE`
   - `status_filter` is prefixed with `AND`
   - when `job_type` is `None` and `status` is `Some`, the final query becomes `FROM jobs AND status = ...`

2. `build_sql_queue_depth()`
   - the base query already contains `WHERE status = 'Queued'`
   - `priority_filter` is also prefixed with `WHERE`
   - when `priority` is `Some`, the final query becomes `WHERE status = 'Queued' WHERE priority = ...`

The existing `test_sql_generation()` only exercises success-rate, throughput, and custom SQL. It never covers the broken optional-filter combinations.

## What Changes

- Rewrite SQL filter construction so optional predicates are appended through a shared helper that joins clauses with `WHERE`/`AND` correctly.
- Add unit tests for all optional-filter combinations in `build_sql_average_duration()` and `build_sql_queue_depth()`.
- Consider parameterizing or escaping string values if this SQL text becomes executable instead of advisory.

## Scope

- **In scope**: SQL text generation in `crates/aspen-jobs/src/analytics.rs` and its test coverage.
- **Out of scope**: The direct KV implementations of the analytics calculations.

## Evidence

```text
crates/aspen-jobs/src/analytics.rs:203  let status_filter = status.map(|s| format!(" AND status = '{:?}'", s)).unwrap_or_default();
crates/aspen-jobs/src/analytics.rs:211  "FROM jobs{}{}"

crates/aspen-jobs/src/analytics.rs:232  let priority_filter = priority.map(|p| format!(" WHERE priority = '{:?}'", p)).unwrap_or_default();
crates/aspen-jobs/src/analytics.rs:239  "WHERE status = 'Queued'{}"
```
