# Project Delta

## ADDED Requirements

### Requirement: Release builds trap integer overflow in Molten-owned crates
r[molten.project.release_overflow_checks] Molten-owned crates MUST be built with integer overflow checks in release builds, in the Cargo release profile and in the Nix package build alike. Arithmetic that can legitimately reach a type bound MUST use checked or saturating operations with an explicit outcome, and code reachable from an `extern "C"` entry point MUST NOT rely on an overflow trap.

#### Scenario: Both release builds carry the overflow option
- GIVEN the Cargo release profile and the Nix package build of a Molten-owned crate
- WHEN its release configuration is inspected
- THEN both enable integer overflow checks.

#### Scenario: A bound is an outcome, not a trap
- GIVEN a Raft replica whose commit index is the maximum representable index
- WHEN it searches for a higher quorum index
- THEN it returns its current commit index without overflowing.
