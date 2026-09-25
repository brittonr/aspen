# Tasks: Octet burn-down, sink parameters instead of borrowed `&mut Vec`

## Phase 1: Implementation

- [x] [serial] Move library `&mut Vec<T>` parameters to `crate::bounded::VecSink<T>` and binary helpers to `Extend<T>`. a[octet-burndown-size-borrowed.zero] a[octet-burndown-size-borrowed.no-rename]
- [x] [serial] Give owned vectors to the helpers that sort, deduplicate, or own a traversal stack, and inline the node-host push helpers with direct guards. a[octet-burndown-size-borrowed.zero]

## Phase 2: Validation

- [x] [serial] Positive: the pinned Octet summaries report 0 `borrowed_argument_types`, and no family grew. a[octet-burndown-size-borrowed.zero]
- [x] [serial] Negative: the diff contains no new `allow`, baseline, or `dylint.toml` change. a[octet-burndown-size-borrowed.no-suppression]
- [x] [serial] Run fmt, clippy, the workspace tests, and the byte-identity comparisons. a[octet-burndown-size-borrowed.behavior]
