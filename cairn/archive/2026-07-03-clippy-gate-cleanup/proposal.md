## Why

The current standard Rust quality gate fails even though focused replay tests, the full test suite, and formatting pass. Before committing replay evidence or refreshing release artifacts, the repository needs a green `cargo clippy --all-targets -- -D warnings` result so release review can treat the Rust validation rail as current and reproducible.

## What Changes

- Fix the repo-wide clippy-denied diagnostics without changing runtime semantics.
- Keep fixes scoped to lint cleanup: unused aliases, unnecessary mutability, collapsible branches, duplicated branches, item ordering, and needless borrows.
- Record the clippy run as validation evidence before downstream replay and release-evidence work proceeds.

## Impact

- **Files**: lint-only Rust cleanup in affected modules and tests.
- **Testing**: `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets -- -D warnings`.
