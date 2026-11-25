# Design

## Extraction Pattern for Tier 1 Zero-Preserves Kernels

Each extracted crate declares `#![no_std]` with `extern crate alloc`, has zero `tokio`/`redb`/`iroh`/`wasmtime`/`preserves`/`std::fs`/`std::net`/`std::time` dependencies, uses typed `core::error::Error` enums (mapped to `MoltenError`/`Failure` at the `molten` shell boundary without losing error class), and eliminates `parts/**/body.rs` `include!()` shards in favor of normal encapsulated Rust modules.

### 1. `crates/molten-raft-core` (~5,560 LOC)
- Owns `ReplicaState`, `ReplicaRole`, `ReplicaEvent`, `ReplicaEffect`, `LogEntry`, `Snapshot`, `Profile`, and all pure transitions (`election`, `client`, `message/{replication,vote,read,snapshot}`, `rejoin/`, `validation`, `bounds`, `support`, `ports` traits).
- `src/fabric_consistency/raft/` retains only the imperative shells (`time/`, `iroh/`, `durability/`, `executor/`, `live_cluster/`, and Preserves wire codecs) calling `molten_raft_core`.
- Pure `no_std` unit and Hegel property tests run directly against `molten-raft-core` without booting Tokio runtimes or sockets.

### 2. `crates/molten-wasm-policy-core` (~1,880 LOC)
- Extracts pure Wasm component import/export admission (`src/wasm/component/admission/`), component profiles (`src/wasm/component/profile.rs`, reading embedded JSON from `config/`, not `docs/`), and optimization profile policy (`src/wasm/performance/policy/`).
- Keeps `wasmtime` compilation, instantiation, fuel runtime, and hostcalls strictly in `src/wasm/`.

### 3. `crates/molten-reliability-model-core` (~1,970 LOC)
- Extracts pure register profile admission, deterministic seeded operation generator, semantic-history projection, and fault-profile timing/partition validation from `src/reliability/model/` and `src/reliability/service/profiles/` using `serde` (`default-features = false, features = ["derive", "alloc"]`), keeping `serde_json` file I/O in `src/reliability/`.

### 4. `crates/molten-fastpath-core`, `crates/molten-composition-core`, and `crates/molten-runtime-admission-core`
- Extracts `src/fabric_consistency/fastpath/` (~1,025 LOC), `src/harness/composition/` (~1,030 LOC), and `src/runtime/admission/mod.rs` + `src/authority/nominal.rs` (~570 LOC) into `#![no_std]` + `alloc` crates with thin re-exports in `molten` preserving existing public facade paths.

## CI Enforcement

- Register all extracted crates in workspace `Cargo.toml`, `[workspace.metadata.octet]`, `.config/nextest.toml`, and `flake.nix` with dedicated `#![no_std]` compilation and unit test checks.
