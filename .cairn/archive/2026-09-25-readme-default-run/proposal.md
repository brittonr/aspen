# Proposal: Make README `cargo run --` commands resolve the molten binary

## Why

The root package has two binaries, `molten` and `molten-native-extension-fixture`. Every README `cargo run -- …`
command fails with "`cargo run` could not determine which binary to run". That includes the strict Octet
source-gate sequence, which the 2026-09-25 strict run had to invoke with `--bin molten`. The README current-state
paragraph also still reports the 2026-09-24 counts, from before the burn-down slices.

## What Changes

- `Cargo.toml` `[package]` gains `default-run = "molten"`, so every documented `cargo run -- …` command resolves to
  the molten CLI without editing the 44 README commands.
- README's strict-gate current-state paragraph is rewritten with dated counts measured at `9e5f6db73`
  (`integration/stack-20260925`): 3209 workspace findings in 1761 sites, 1363 lib findings, every critical family at
  zero, `no-critical-findings` passing, and `strict-status-clean` still failing.

## Impact

- **Files**: `Cargo.toml`, `README.md`.
- **Testing**:
  - Three README commands spot-run.
  - `Cargo.lock` confirmed unchanged.
  - fmt, clippy, the workspace tests, Octet, and the build-plan staleness reasoning below.

## Out of Scope

- The build plans are unchanged: unit2nix's staleness check hashes only `Cargo.lock`, and `default-run` changes
  neither the lock file nor any unit.
- No behavior change to either binary.
