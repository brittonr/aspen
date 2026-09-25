# Tasks: Octet burn-down, source files within the 300-line limit

## Phase 1: Implementation

- [x] [serial] Split the 303 over-long Rust files into the `include!` parts layout at item boundaries, with nested parts for oversize inline modules and continuation blocks for oversize inherent impls. a[octet-burndown-size-files.zero] a[octet-burndown-size-files.verbatim]
- [x] [serial] Read `Cargo.lock` at run time in the facade-boundary test, and make the source-scanning tests concatenate their target's parts. a[octet-burndown-size-files.zero] a[octet-burndown-size-files.behavior]
- [x] [serial] Re-point scan lists, literal checks, tracey records, evidence-matrix targets, and Cairn policy roots at the moved code. a[octet-burndown-size-files.tooling]

## Phase 2: Validation

- [x] [serial] Positive: the pinned Octet summaries report 0 `excessive_file_length`, and no family grew. a[octet-burndown-size-files.zero]
- [x] [serial] Include-expansion comparison against the base tree. a[octet-burndown-size-files.verbatim]
- [x] [serial] Negative: the diff contains no new `allow`, baseline, `dylint.toml` change, or rename. a[octet-burndown-size-files.no-suppression]
- [x] [serial] Run fmt, clippy, the workspace tests, the byte-identity comparisons, and the touched-surface flake checks. a[octet-burndown-size-files.behavior] a[octet-burndown-size-files.tooling]
