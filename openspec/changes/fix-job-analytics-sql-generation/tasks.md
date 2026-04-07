## 1. Normalize predicate assembly

- [ ] 1.1 Add a shared helper in `crates/aspen-jobs/src/analytics.rs` for appending optional `WHERE` clauses and `AND` joins
- [ ] 1.2 Refactor `build_sql_average_duration()` to use the shared predicate helper
- [ ] 1.3 Refactor `build_sql_queue_depth()` to use the shared predicate helper

## 2. Audit the remaining analytics builders

- [ ] 2.1 Check the other analytics SQL builders for hand-built `WHERE`/`AND` fragments and move them to the same helper where appropriate
- [ ] 2.2 Confirm the generated SQL still matches the intended semantics for success-rate, throughput, and failure-analysis queries

## 3. Add regression tests

- [ ] 3.1 Add a unit test for average-duration with `status` present and `job_type` absent
- [ ] 3.2 Add a unit test for average-duration with `job_type` only and with both filters present
- [ ] 3.3 Add a unit test for queue-depth with `priority` present
- [ ] 3.4 Add structural assertions that generated analytics SQL contains at most one `WHERE` token in the affected queries

## 4. Verify and document the behavior

- [ ] 4.1 Run the `aspen-jobs` test suite covering analytics SQL generation
- [ ] 4.2 Confirm the debug-logged SQL for representative queries is syntactically valid
- [ ] 4.3 Note any follow-up work needed around escaping or parameter binding if these queries move from advisory strings to executed SQL
