# Testing harness deltas

## ADDED Requirements

### Requirement: Hermetic CI verifies no_std compilation and full tests for molten-core
r[molten.testing.molten_core_no_std_ci_gate] `flake.nix` and CI MUST execute both a `#![no_std]` (`--no-default-features`) compilation gate and full unit/property tests for `crates/molten-core`, ensuring zero behavioral drift between `no_std` and default builds.

#### Scenario: Both no_std and std feature profiles pass tests
- GIVEN the `molten-core` workspace crate
- WHEN the hermetic Nix check runs
- THEN both `--no-default-features` `#![no_std]` compilation and the full `molten-core` test suite MUST pass.
