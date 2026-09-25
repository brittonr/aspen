# Tasks: Octet burn-down, collection growth in runtime and adapters

## Phase 1: Implementation

- [x] [serial] Apply structural repairs (iterator chains, exact reservations) to the runtime and adapter sites. a[octet-burndown-safety-collections-runtime.behavior]
- [x] [serial] Bound externally sized growth with existing admitted limits, or a named constant where none exists. a[octet-burndown-safety-collections-runtime.bounds]

## Phase 2: Validation

- [x] [serial] Positive: the pinned Octet summaries report 0 `unbounded_collection_growth`, and no family grew. a[octet-burndown-safety-collections-runtime.zero]
- [x] [serial] At-limit and one-past tests for every new bound. a[octet-burndown-safety-collections-runtime.bounds]
- [x] [serial] Negative: the diff contains no new `allow`, baseline, or `dylint.toml` change. a[octet-burndown-safety-collections-runtime.no-suppression]
- [x] [serial] Run fmt, clippy, focused and workspace tests, and the harness receipt comparison. a[octet-burndown-safety-collections-runtime.behavior]
