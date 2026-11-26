## Why

`crates/molten-core` owns 74,066 non-test LOC across 52 pure decision-core modules, and 10 of Aspen's own upstream OnixResearch kernel crates (`artifact-auth-core`, `artifact-binding-core`, `choregraph-history`, `content-identity-core`, `executable-extent-core`, `replay-ledger-core`, `revocation-view-core`, `schema-identity-core`, `schema-migration-core`, and `transactional-reconciliation-core`) are already `#![no_std]`.

Yet `crates/molten-core` itself still compiles against `std`, giving no compiler-enforced barrier against ambient filesystem, network, wall-clock, environment, or thread-local entropy leaks. Source inspection confirms that 50 of its 52 modules already require only `core` + `alloc`: the only two non-`core`/`alloc` `std` imports in the entire crate are `std::net` in `crates/molten-core/src/fabric_transport/receive.rs` (available as `core::net` in Edition 2024) and `std::hint` in `crates/molten-core/src/release_dependency/explanation/comparison.rs` (`core::hint`), while only two dependencies (`basalt` in `world_branch_authority` and `kamacite-core` in `live_binding`) currently lack `no_std` support.

## What Changes

- Convert `crates/molten-core` to `#![no_std]` with `extern crate alloc`, replacing `std::collections`, `std::net`, `std::hint`, and `std::*` imports with `core::*` and `alloc::*` equivalents.
- Switch `blake3` (`default-features = false`) and `serde` (`default-features = false, features = ["derive", "alloc"]`) in `crates/molten-core/Cargo.toml` so default builds pull no `std` features from core dependencies.
- Isolate `basalt` (`world_branch_authority`) and `kamacite-core` (`live_binding`) behind an explicit `std` Cargo feature (or update them once upstream `no_std + alloc` cohorts land) so `cargo check -p molten-core --no-default-features` compiles cleanly under `#![no_std]`.
- Wire a hermetic `no_std` compile-verification check for `molten-core` into `flake.nix` and CI so `std` dependencies cannot silently leak back into pure decision cores.

## Impact

- **Files**: `crates/molten-core/Cargo.toml`, `crates/molten-core/src/lib.rs`, `crates/molten-core/src/**/*.rs`, `Cargo.toml`, `flake.nix`, `build-plan.json`, `docs/modularity-boundaries.md`, and `AGENTS.md`.
- **Verification**: Hermetic `no_std` compile check (`--no-default-features`), full `molten-core` test suite under default and `--no-default-features` feature sets, Clippy, Octet, and strict Cairn validation.
- **Non-claims**: Compiling `molten-core` under `#![no_std]` enforces absence of OS ambient I/O at compile time; it does not prove semantic correctness, replace runtime authority gates, or qualify embedded hardware targets.

## Out of Scope

- Extracting `molten-preserves-core` or Tier 1 / Tier 2 kernels out of `src/` (tracked in separate extraction Cairn changes).
- Upstream `no_std` conversion of `basalt` or `kamacite-core` repositories (tracked separately with their upstream owners; feature-gating in Aspen unblocks `molten-core` immediately).
