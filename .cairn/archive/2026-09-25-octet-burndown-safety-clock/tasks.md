# Tasks: Octet burn-down, ambient clock and structural-scan recursion

## Phase 1: Implementation

- [x] [serial] Add the two reasoned live-clock allows and repair `await_ticks` through `now_ticks`. a[octet-burndown-safety-clock.allows]
- [x] [serial] Add `TickDeadline` and `SupervisionDeadline`, and route the six supervision timeouts through them. a[octet-burndown-safety-clock.behavior]
- [x] [serial] Replace the recursive structural scan with an explicit bounded frame stack. a[octet-burndown-safety-clock.behavior]

## Phase 2: Validation

- [x] [serial] Positive: the pinned Octet summaries report 0 `ambient_clock` and 0 `no_recursion`, and no family grew. a[octet-burndown-safety-clock.zero]
- [x] [serial] At-limit and one-past tests for the deadline, the supervision bound, and the scan bounds. a[octet-burndown-safety-clock.bounds]
- [x] [serial] Negative: the diff adds no allow besides the two reasoned live-clock allows, and no baseline or `dylint.toml` change. a[octet-burndown-safety-clock.allows]
- [x] [serial] Run fmt, clippy, focused and workspace tests, and the fixture comparisons. a[octet-burndown-safety-clock.behavior]
