## Why

Source audit of `src/` identified ~12,000+ LOC of high-value, pure deterministic state machines and admission cores that have **zero I/O (`tokio`, `redb`, `iroh`, `wasmtime`, `std::fs`, `std::net`, `std::process`) and zero `preserves::IOValue` dependency**, yet still reside inside the monolithic 425K-LOC `std` root `molten` crate:

1. **Replicated Raft state machine (~5,560 LOC):** `src/fabric_consistency/raft/transition/`, `raft/ports.rs`, and the pure state/model portions of `raft/parts/` (`docs/architecture.md:446` explicitly notes replicated Raft still sits in the root crate). Extracting it into a `#![no_std]` + `alloc` crate decouples consensus safety from Tokio, Iroh, and Redb and lets Hegel, Quint, and ChaosControl drive the exact `no_std` kernel directly.
2. **Wasm component & optimization policy (~1,880 LOC):** `src/wasm/component/admission/`, `src/wasm/component/profile.rs`, and `src/wasm/performance/policy/` perform pure import/export/profile admission with zero `wasmtime` dependency.
3. **Reliability workload & fault-profile models (~1,970 LOC):** `src/reliability/model/` and `src/reliability/service/profiles/` own pure seeded operation planning, history projection, and fault-profile validation (`serde` + `alloc`; `serde_json` stays in the shell).
4. **Fast-path consensus core (~1,025 LOC):** `src/fabric_consistency/fastpath/` is already pure with zero `std`/Preserves dependencies.
5. **Harness composition planner (~1,030 LOC):** `src/harness/composition/` is pure with zero I/O or external crate dependencies.
6. **Runtime admission & nominal authority grammar (~570 LOC):** `src/runtime/admission/mod.rs` and `src/authority/nominal.rs` own pure grant/policy evaluation and nominal syntax rules with zero `std`/Preserves dependencies.

## What Changes

- Extract `crates/molten-raft-core` (`#![no_std]` + `alloc`) owning replicated Raft state, types, log/quorum/snapshot/rejoin/ReadIndex transitions, and pure effect plans, while keeping Tokio timers, Iroh transport, Redb persistence, and Preserves wire encoding in thin `src/fabric_consistency/raft/` adapters.
- Extract `#![no_std]` + `alloc` workspace crates (or `no_std` modules inside `molten-core` once `convert-molten-core-to-no-std` lands) for:
  - `crates/molten-wasm-policy-core` (Wasm component admission, profile validation, optimization policy)
  - `crates/molten-reliability-model-core` (seeded register workload generator, history projection, fault-profile bounds)
  - `crates/molten-fastpath-core` (if fast-path acceleration remains active)
  - `crates/molten-composition-core` (harness composition planning)
  - `crates/molten-runtime-admission-core` (runtime capability/policy admission & nominal authority grammar)
- Replace `crate::error::Failure` coupling in these pure kernels with typed `no_std` core error enums implementing `core::error::Error`, preserving error classes at the `molten` shell boundary.
- Add hermetic `#![no_std]` compile gates, unit tests, and Hegel property tests for every extracted kernel in `flake.nix` and workspace Octet.

## Impact

- **Files**: `Cargo.toml`, new workspace crates under `crates/molten-{raft,wasm-policy,reliability-model,fastpath,composition,runtime-admission}-core/`, thin re-export/adapter shells in `src/fabric_consistency/`, `src/wasm/`, `src/reliability/`, `src/harness/composition/`, `src/runtime/admission/`, `src/authority/nominal.rs`, `flake.nix`, `build-plan.json`, and `docs/architecture.md`.
- **Verification**: Hermetic `#![no_std]` compilation checks for every extracted crate, full unit/property tests, Clippy, workspace Octet, and strict Cairn validation.
- **Non-claims**: Extracting `no_std` kernels guarantees compile-time isolation from OS I/O; it does not by itself fix Raft protocol logic bugs (tracked in `repair-live-raft-liveness-safety-and-transport-auth`) or qualify live clusters.

## Out of Scope

- Fixing Raft live host timer/ReadProbe/reconciliation bugs (tracked in `repair-live-raft-liveness-safety-and-transport-auth`; both changes coordinate cleanly on `molten-raft-core`).
- Extracting Tier 2 kernels that currently depend on `preserves::IOValue` (tracked separately after `extract-no-std-preserves-packed-kernel`).
