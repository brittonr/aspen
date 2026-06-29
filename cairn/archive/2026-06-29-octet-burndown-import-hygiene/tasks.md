## Tasks

- [x] [serial] r[molten.octet_burndown.import_hygiene] Capture the latest no-disabled Octet probe and identify the highest-value `non_trait_imports` hotspots before the first category slice.
- [x] [serial] r[molten.octet_burndown.import_hygiene] Refactor one import-hygiene hotspot per accepted slice using local qualification, narrower imports, or clearer private helper boundaries without public semantic drift.
- [x] [serial] r[molten.octet_burndown.import_hygiene] Run focused Rust validation plus a no-disabled Octet probe after each accepted slice and record before/after `non_trait_imports` counts.
- [x] [serial] r[molten.octet_burndown.import_hygiene] Remove or narrow the `non_trait_imports` disabled-lint caveat only after refreshed evidence proves the family is clean or explicitly scoped.

Evidence: baseline `cargo test harness` passed. The latest pre-slice no-disabled probe was `target/octet-burndown/source-scope-classification-v2-0/summary.txt` with `non_trait_imports` 3687. The slice qualified `src/harness/runner.rs` owner paths through module-local namespaces without changing harness behavior. Validation passed with `cargo fmt --check`, `cargo test harness`, `cargo clippy --all-targets -- -D warnings`, and no-disabled probe `target/octet-burndown/import-hygiene-runner-0/summary.txt`, reducing `non_trait_imports` to 3607 and total findings to 6818. The `non_trait_imports` caveat remains active because the family is not yet clean.
