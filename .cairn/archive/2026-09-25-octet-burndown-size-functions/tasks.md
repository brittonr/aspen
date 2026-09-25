# Tasks: Octet burn-down, functions within the 70-line limit

## Phase 1: Implementation

- [x] [serial] Split the 127 long functions into documented helpers, struct-update defaults, tables, and `clap::Args` structs, preserving call and side-effect order. a[octet-burndown-size-functions.zero] a[octet-burndown-size-functions.no-rename]
- [x] [serial] Keep touched files within the file-length limit by moving the assembly test and two helpers into sibling files, and name new helpers without repeated path segments. a[octet-burndown-size-functions.zero]

## Phase 2: Validation

- [x] [serial] Positive: the pinned Octet summaries report 0 `function_length`; the only other change is the five unmasked pre-existing public names. a[octet-burndown-size-functions.zero]
- [x] [serial] Negative: the diff contains no new `allow`, baseline, `dylint.toml` change, or exemption keyword. a[octet-burndown-size-functions.no-suppression]
- [x] [serial] Run fmt, clippy, the workspace tests, and the byte-identity comparisons. a[octet-burndown-size-functions.behavior]
