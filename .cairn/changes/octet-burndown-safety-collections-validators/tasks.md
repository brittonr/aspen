# Tasks: Octet burn-down, collection growth in validators

## Phase 1: Implementation

- [x] [serial] Rewrite filter/map loops as iterator chains that preserve order and first-error propagation. a[octet-burndown-safety-collections-validators.behavior]
- [x] [serial] Reserve exactly the per-loop maximum over already-bounded inputs, naming each multiplier. a[octet-burndown-safety-collections-validators.zero]

## Phase 2: Validation

- [x] [serial] Positive: the pinned Octet summaries list no collection-growth site in this slice's files, and no family grew. a[octet-burndown-safety-collections-validators.zero]
- [x] [serial] Negative: the diff contains no new `allow`, baseline, or `dylint.toml` change. a[octet-burndown-safety-collections-validators.no-suppression]
- [x] [serial] Run fmt, clippy with `-D warnings`, focused tests, and the workspace tests. a[octet-burndown-safety-collections-validators.behavior]
