# Tasks: Octet burn-down, explicit input structs for long parameter lists

## Phase 1: Implementation

- [x] [serial] Move the flagged parameter lists into explicit input structs without renaming any function. a[octet-burndown-size-parameters.zero] a[octet-burndown-size-parameters.no-rename]
- [x] [serial] Keep `path_segment_repetition`, `function_length`, and `excessive_file_length` level. a[octet-burndown-size-parameters.zero]

## Phase 2: Validation

- [x] [serial] Positive: the pinned Octet summaries report 0 `too_many_parameters`, and no family grew. a[octet-burndown-size-parameters.zero]
- [x] [serial] Negative: the diff contains no new `allow`, baseline, or `dylint.toml` change. a[octet-burndown-size-parameters.no-suppression]
- [x] [serial] Run fmt, clippy, the workspace tests, and the byte-identity comparisons. a[octet-burndown-size-parameters.behavior]
