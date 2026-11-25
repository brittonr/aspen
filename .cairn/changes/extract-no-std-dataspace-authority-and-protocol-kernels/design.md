# Design

## Common Extraction Contract (Atop `molten-preserves-core`)

Every extracted crate in this package:
- Declares `#![no_std]` with `extern crate alloc`, depending only on `molten-preserves-core` (and `core`/`alloc`/other `#![no_std]` Aspen kernels).
- Uses `molten_preserves_core::PackedValue` and canonical packed bytes for all value/receipt hashing, keeping `preserves::IOValue` and Preserves text rendering inside thin `std` adapters in `src/`.
- Uses real Rust module hierarchies (zero `parts/**/body.rs` `include!()` shards) and typed `core::error::Error` enums.
- Preserves byte-for-byte identical canonical packed Preserves outputs and BLAKE3 identities relative to pre-extraction behavior (except where paired semantic-fix Cairn changes explicitly correct bugs).

## Crate Decomposition

1. **`crates/molten-dataspace-core` (~8,340 LOC):** Extracts `src/runtime/{dataspace,vat,predicates,turn,envelope,spine}`. Replaces the single `std::sync` occurrence with `core::cell` or explicit state passing and decouples pure dataspace assertion/facet/turn logic from `std`-only `syndicate` host adapters (leaving any `syndicate` interop bridge in `src/runtime/`).
2. **`crates/molten-capability-authority-core` (~4,915 LOC):** Extracts `src/capability/` (claim subjects, tokens, attenuation), `src/authority/` (grant currentness, revocation), and `src/effects/` (effect envelopes and manifest profiles).
3. **`crates/molten-protocol-session-core` (~3,885 LOC):** Extracts `src/protocol/session` and `src/protocol/sans/io` into a genuinely `#![no_std]` sans-I/O protocol crate (isolating any `trellis` bridge behind an optional adapter or verifying `trellis` `no_std` readiness).
4. **`crates/molten-service-fsm-core` (~7,390 LOC):** Extracts `src/service/` (records, runtime, supervision/watchdog), `src/node/service/fsm.rs`, `src/node/profile/config.rs`, and `src/lifecycle/` into one pure lifecycle/supervision FSM kernel.
5. **`crates/molten-coordination-core` (~6,390 LOC):** Extracts `src/coordination/` and separates the pure state transitions of `src/raft/control/` (locks, leases, barriers, queues, elections, service registry, fencing tokens) from Redb persistence, leaving only the thin Redb persistence adapter in `src/raft/control/`.

## CI & Facade Compatibility

- Retain public re-exports in `src/lib.rs` (collapsing `#[doc(hidden)]` aliases over time under `align-formal-models-architecture-and-governance-toolchains`).
- Register all five crates in `Cargo.toml`, `[workspace.metadata.octet]`, `.config/nextest.toml`, `flake.nix` (`no_std` compile gates + tests), and `build-plan.json`.
