# Tasks: Admit a bounded record pattern subset

## Consumer and bounds

- [ ] [serial] Name the consumer subscription that needs a record pattern today and record the record shape it matches. r[molten.runtime_spine.record_pattern_subset]
- [ ] [serial] Fix the admitted bounds: maximum record depth, maximum declared entries, and the binding byte limit, all enforced at pattern admission. r[molten.runtime_spine.record_pattern_subset]

## Matching

- [ ] [serial] Add the record pattern form with type check, increasing declared-position order, at-least matching, and speculative binding rollback. r[molten.runtime_spine.record_pattern_subset]
- [ ] [serial] Keep identity paths exact: no identity, receipt, or canonical-ref computation consumes a pattern binding. r[molten.runtime_spine.record_pattern_subset]

## Validation

- [ ] [parallel] Add positive tests: a record with an extra field matches, bindings arrive in declared order, and a nested record binds its sub-portion. r[molten.runtime_spine.record_pattern_subset]
- [ ] [parallel] Add negative tests: a missing declared key denies, a wrong type at a declared key denies, an undeclared form denies before routing, an over-bound pattern denies at admission, and a mismatched nested sub-pattern leaves no binding behind. r[molten.runtime_spine.record_pattern_subset]
- [ ] [parallel] Add the identity non-interference test: the same record admitted by routing keeps one unchanged canonical value ref. r[molten.runtime_spine.record_pattern_subset]
- [ ] [serial] Run `cargo test -p molten` and focused Clippy before and after the change, then the workspace checks the repository requires for runtime edits. r[molten.runtime_spine.record_pattern_subset]
- [ ] [serial] Document the admitted pattern subset, its bounds, and the routing-only non-claim in `docs/architecture.md`. r[molten.runtime_spine.record_pattern_subset]
