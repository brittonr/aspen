# Tasks: Octet burn-down, import hygiene

## Phase 1: Implementation

- [x] [serial] Run the codemod self-test and dry run and record the refusals. a[octet-burndown-import-hygiene.zero]
- [x] [serial] Qualify the 23 `non_trait_imports` sites by hand at their owner paths. a[octet-burndown-import-hygiene.zero]
- [x] [serial] Repair the 17 `explicit_defaults` sites with explicit owner constructors and named serde defaults. a[octet-burndown-import-hygiene.behavior]

## Phase 2: Validation

- [x] [serial] Positive: the pinned Octet root and lib summaries show 0 for both lints, and no other family moved. a[octet-burndown-import-hygiene.zero]
- [x] [serial] Negative: the diff contains no new `allow`, baseline, or `dylint.toml` change. a[octet-burndown-import-hygiene.no-suppression]
- [x] [serial] Run fmt, clippy with `-D warnings`, and the workspace tests. a[octet-burndown-import-hygiene.behavior]
- [x] [serial] Build the flake source-scan checks and compare them with the base failures. a[octet-burndown-import-hygiene.scans]
