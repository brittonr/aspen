# Design

## Crate-Level `#![no_std]` Boundary

- Add `#![cfg_attr(not(feature = "std"), no_std)]` (or unconditional `#![no_std]` with `#[cfg(test)] extern crate std;`) and `extern crate alloc;` in `crates/molten-core/src/lib.rs`.
- Replace all `std::` paths across `crates/molten-core/src/`:
  - `std::net::*` in `fabric_transport/receive.rs` → `core::net::*`.
  - `std::hint::*` in `release_dependency/explanation/comparison.rs` → `core::hint::*`.
  - `std::collections::{BTreeMap, BTreeSet, VecDeque}` → `alloc::collections::*`, `std::string::*` → `alloc::string::*`, `std::vec::*` → `alloc::vec::*`, `std::format!` → `alloc::format!`, `std::fmt::*` / `std::error::Error` → `core::fmt::*` / `core::error::Error` (stabilized in `core` since Rust 1.81).
- Update `crates/molten-core/Cargo.toml`:
  - `blake3 = { version = "1", default-features = false }`
  - `serde = { version = "1", default-features = false, features = ["derive", "alloc"] }`
  - Define `[features] default = ["std"]` and `std = ["dep:basalt", "dep:kamacite-core", "blake3/std", "serde/std"]`, gating `pub mod world_branch_authority` and `pub mod live_binding` on `#[cfg(feature = "std")]` until upstream `basalt` and `kamacite-core` support `no_std + alloc`.

## Compiler-Enforced Purity Gate

- Add a dedicated hermetic check in `flake.nix` (and Cargo CI) that builds `molten-core` with `--no-default-features` (and `--no-default-features --features executable-extents`) and verifies `#![no_std]` cleanly—optionally cross-checking against a `no_std` target or explicit `extern crate std` denial lint.
- Keep `Cargo.lock` and unit2nix `build-plan.json` synchronized per `docs/reproducible-dependencies.md`.
- Update `docs/modularity-boundaries.md` to document exact `no_std + alloc` vs `std`-gated module boundaries truthfully.
