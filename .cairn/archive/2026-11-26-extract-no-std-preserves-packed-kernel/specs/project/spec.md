# Project deltas

## ADDED Requirements

### Requirement: molten-preserves-core compiles under no_std and is gated in workspace CI
r[molten.project.molten_preserves_core_no_std_gate] `crates/molten-preserves-core` MUST declare `#![no_std]` with `extern crate alloc`, depend only on `no_std`-compatible crates (`blake3` with `default-features = false`), and pass hermetic `no_std` compilation, unit/differential property tests, and workspace Octet checks in `flake.nix`.

#### Scenario: Clean no_std compilation succeeds without std
- GIVEN `crates/molten-preserves-core` built with `--no-default-features`
- WHEN the hermetic Nix check compiles the crate
- THEN compilation MUST succeed under `#![no_std]` with zero `std` imports in non-test code.
