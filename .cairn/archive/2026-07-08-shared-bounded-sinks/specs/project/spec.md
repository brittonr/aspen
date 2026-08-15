## ADDED Requirements

### Requirement: Shared bounded helpers use checked arithmetic
r[molten.shared_bounded_sinks.checked_counts] Repository-owned bounded collection helpers MUST calculate counts with checked arithmetic before mutating a collection and MUST fail closed when the next count would overflow or exceed the configured limit.

#### Scenario: One-past-limit push does not mutate
- GIVEN a bounded vector with item count equal to its configured limit
- WHEN a caller attempts to push one more item through the shared helper
- THEN the helper returns an error
- AND the vector contents remain unchanged.

### Requirement: Diagnostic sinks share bounded behavior
r[molten.shared_bounded_sinks.diagnostic_sink] New diagnostic accumulation code SHOULD use the shared bounded diagnostic sink behavior unless a subsystem documents a stricter local invariant.

#### Scenario: Diagnostic overflow denies consistently
- GIVEN a diagnostic sink at its configured maximum
- WHEN a subsystem attempts to add another diagnostic
- THEN the sink fails closed with deterministic diagnostics
- AND the subsystem does not silently drop or append the diagnostic.

### Requirement: Equivalent bounded helpers migrate to shared utilities
r[molten.shared_bounded_sinks.migration] Duplicated bounded push, extend, count, and diagnostic helpers SHOULD migrate to shared utilities when behavior is equivalent and local invariants do not require a stricter subsystem helper.

#### Scenario: Equivalent helper calls shared core
- GIVEN a subsystem helper that previously checked count limits before pushing into a vector
- WHEN the behavior is equivalent to the shared bounded helper
- THEN the subsystem delegates to the shared core
- AND preserves fail-closed no-mutation behavior.

### Requirement: Bounded helper migrations preserve evidence shape
r[molten.shared_bounded_sinks.hash_stability] Refactoring duplicated bounded helpers into shared utilities MUST preserve canonical receipt values when the only change is helper mechanics.

#### Scenario: Migrated receipt hash remains stable
- GIVEN a representative receipt fixture built before helper migration
- WHEN the same semantic input is built after migration
- THEN the canonical receipt ref is unchanged or the change records an explicit evidence migration note.

### Requirement: Bound-denial behavior is negatively covered
r[molten.shared_bounded_sinks.negative_bounds] Shared bounded helpers MUST include negative tests for one-past-limit, arithmetic overflow, extend overflow, and no-mutation-on-error cases.

#### Scenario: Extend overflow leaves destination unchanged
- GIVEN a destination collection and an incoming slice whose combined count exceeds the maximum
- WHEN bounded extend runs
- THEN it denies before appending any incoming item.
