# Tasks: Make README `cargo run --` commands resolve the molten binary

## Phase 1: Implementation

- [x] [serial] Add `default-run = "molten"` to the root package. a[readme-default-run.commands]
- [x] [serial] Rewrite the README current-state paragraph with the measured counts. a[readme-default-run.state]

## Phase 2: Validation

- [x] [serial] Spot-run three README `cargo run --` commands. a[readme-default-run.commands]
- [x] [serial] Confirm `Cargo.lock` and both build plans are unchanged and the flake still evaluates the plans. a[readme-default-run.plans]
