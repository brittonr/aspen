# Project deltas

## ADDED Requirements

### Requirement: molten-core compiles cleanly under no_std with alloc
r[molten.project.molten_core_no_std] `crates/molten-core` MUST compile cleanly under `#![no_std]` + `extern crate alloc` when built with `--no-default-features`, using `core::net`, `core::hint`, `core::error::Error`, and `alloc` collections rather than `std`, with `blake3` and `serde` configured with `default-features = false` (`alloc`). Any module still requiring non-`no_std` upstream crates (`basalt`, `kamacite-core`) MUST be isolated behind an explicit `std` feature rather than forcing `std` onto the remaining 50 pure modules.

#### Scenario: Clean no_std compilation succeeds
- GIVEN `crates/molten-core` built with `--no-default-features`
- WHEN `cargo check -p molten-core --no-default-features` runs
- THEN compilation MUST succeed under `#![no_std]` with zero `std` imports in non-test code.

#### Scenario: Accidental std import fails the no_std CI gate
- GIVEN a new `std::fs`, `std::time`, or `std::env` import introduced into a `no_std` module of `crates/molten-core`
- WHEN the hermetic `no_std` check runs
- THEN compilation MUST fail closed before merge.
