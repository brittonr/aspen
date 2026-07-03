# Tasks: clippy-gate-cleanup

## Phase 1: Baseline and cleanup

- [x] [serial] r[molten.project.clippy_gate.current_warning_free] Record the current clippy denial classes before edits.
- [x] [parallel] r[molten.project.clippy_gate.current_warning_free] Fix unused aliases, unnecessary mutability, collapsible branches, identical branches, item ordering, and needless borrows without semantic changes.

## Phase 2: Validation

- [x] [serial] r[molten.project.clippy_gate.current_warning_free] Run `cargo fmt --check`.
- [x] [serial] r[molten.project.clippy_gate.current_warning_free] Run `cargo test`.
- [x] [serial] r[molten.project.clippy_gate.current_warning_free] Run `cargo clippy --all-targets -- -D warnings` and record the passing evidence.
