# Tasks: Octet burn-down, bounded channels and fixed-width public integers

## Phase 1: Implementation

- [x] [serial] Bound the Raft inbox and control channels, with a typed full-queue denial and timer backpressure. a[octet-burndown-safety-ports.bounds]
- [x] [serial] Move public signatures to `u64`/`u32` with checked boundary conversions. a[octet-burndown-safety-ports.zero]

## Phase 2: Validation

- [x] [serial] Positive: the pinned Octet summaries report 0 for both families, and no family grew. a[octet-burndown-safety-ports.zero]
- [x] [serial] At-limit, one-past, and no-deadlock channel tests. a[octet-burndown-safety-ports.bounds]
- [x] [serial] Negative: the diff contains no new `allow`, baseline, or `dylint.toml` change. a[octet-burndown-safety-ports.no-suppression]
- [x] [serial] Run fmt, clippy, the workspace tests, and the byte-identity comparisons. a[octet-burndown-safety-ports.behavior]
