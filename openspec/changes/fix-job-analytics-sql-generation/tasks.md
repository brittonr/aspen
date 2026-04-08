## 1. Normalize predicate assembly

- [x] 1.1 Add a shared helper in `crates/aspen-jobs/src/analytics.rs` for appending optional `WHERE` clauses and `AND` joins
- [x] 1.2 Refactor `build_sql_average_duration()` to use the shared predicate helper
- [x] 1.3 Refactor `build_sql_queue_depth()` to use the shared predicate helper

## 2. Audit the remaining analytics builders

- [x] 2.1 Check the other analytics SQL builders for hand-built `WHERE`/`AND` fragments and move them to the same helper where appropriate
- [x] 2.2 Confirm the generated SQL still matches the intended semantics for success-rate, throughput, and failure-analysis queries

## 3. Add regression tests

- [x] 3.1 Add a unit test for average-duration with `status` present and `job_type` absent
- [x] 3.2 Add a unit test for average-duration with `job_type` only and with both filters present
- [x] 3.3 Add a unit test for queue-depth with `priority` present
- [x] 3.4 Add structural assertions that generated analytics SQL contains at most one `WHERE` token in the affected queries

## 4. Verify and document the behavior

- [x] 4.1 Run the `aspen-jobs` test suite covering analytics SQL generation
- [x] 4.2 Confirm the debug-logged SQL for representative queries is syntactically valid
- [x] 4.3 Note any follow-up work needed around escaping or parameter binding if these queries move from advisory strings to executed SQL
