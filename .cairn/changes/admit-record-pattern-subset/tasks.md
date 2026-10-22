# Tasks: Admit a bounded record pattern subset

## Consumer and bounds

- [x] [serial] Name the consumer subscription that needs a record pattern today and record the record shape it matches. r[molten.runtime_spine.record_pattern_subset]
      The consumer is the remote dataspace's service-ready consumer: the two-peer harness's `consumer` actor observes
      the `service-ready` record. It subscribed to one exact value, so a producer that added a field broke the
      subscription. The record shape it matches is the `service-ready` record with its declared service position.
- [x] [serial] Fix the admitted bounds: maximum record depth, maximum declared entries, and the binding byte limit, all enforced at pattern admission. r[molten.runtime_spine.record_pattern_subset]
      The bounds are sixteen declared positions per record, four nested records, and the existing binding byte limit.
      They are checked when the subscription is registered, before the pattern routes a delivery.

## Matching

- [x] [serial] Add the record pattern form with type check, increasing declared-position order, at-least matching, and speculative binding rollback. r[molten.runtime_spine.record_pattern_subset]
      The form is `RuntimePattern::Record { label, entries }` with the AST form `runtime-pattern-record-v1`. The label is
      the type check, the declared positions increase in Preserves order, the candidate's undeclared positions are
      ignored, and a failed sub-pattern discards the bindings the visit produced.
- [x] [serial] Keep identity paths exact: no identity, receipt, or canonical-ref computation consumes a pattern binding. r[molten.runtime_spine.record_pattern_subset]
      The pattern matches route observations only. The canonical value ref of a matched record is unchanged, and the
      pattern never feeds an identity, a receipt hash, or a canonical-ref computation.

## Validation

- [x] [parallel] Add positive tests: a record with an extra field matches, bindings arrive in declared order, and a nested record binds its sub-portion. r[molten.runtime_spine.record_pattern_subset]
- [x] [parallel] Add negative tests: a missing declared key denies, a wrong type at a declared key denies, an undeclared form denies before routing, an over-bound pattern denies at admission, and a mismatched nested sub-pattern leaves no binding behind. r[molten.runtime_spine.record_pattern_subset]
- [x] [parallel] Add the identity non-interference test: the same record admitted by routing keeps one unchanged canonical value ref. r[molten.runtime_spine.record_pattern_subset]
- [x] [serial] Run `cargo test -p molten` and focused Clippy before and after the change, then the workspace checks the repository requires for runtime edits. r[molten.runtime_spine.record_pattern_subset]
- [x] [serial] Document the admitted pattern subset, its bounds, and the routing-only non-claim in `docs/architecture.md`. r[molten.runtime_spine.record_pattern_subset]
