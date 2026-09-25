# Tasks: Octet burn-down, safety `no_unwrap`

## Phase 1: Implementation

- [x] [serial] Convert the flagged integration tests and helpers to `Result` returns with `?` and a labelled `OrFail` step helper. a[octet-burndown-safety-unwrap.zero]
- [x] [serial] Repair the three library sites without changing their values or errors. a[octet-burndown-safety-unwrap.behavior]

## Phase 2: Validation

- [x] [serial] Positive: the pinned Octet root and lib summaries show 0 `no_unwrap` and 0 `no_panic`, and no other family grew. a[octet-burndown-safety-unwrap.zero]
- [x] [serial] Negative: the diff contains no new `allow`, baseline, or `dylint.toml` change. a[octet-burndown-safety-unwrap.no-suppression]
- [x] [serial] Run fmt, clippy with `-D warnings`, and the workspace tests. a[octet-burndown-safety-unwrap.behavior]
