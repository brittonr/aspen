# Design

## Production-Anchored Formal Verification

- Author and wire a production Raft TLA+ model into `flake.nix` covering leader election, `AppendEntries` replication, log truncation on term conflict, commit advancement, snapshot compaction, and `ReadIndex` confirmation—deriving Leader Completeness and linearizable reads as checked safety invariants rather than assumptions in `Init`.
- Repair `verification/tla/ConsensusFastPath.tla` so `Quorum` intersection across competing superquorums, per-replica logs, and network message reordering are genuinely modeled (not centralized guard restatements), and scope its CI cost appropriately to the blocked fast-path acceleration package rather than presenting it as production Raft verification.
- Demote `nix/verified-node-replication-pilot.nix` from default `nix flake check` to an opt-in research app so default CI does not carry an extra Verus/Octet toolchain for an unused external crate.

## Real Modularity Instead of `include!()` Sharding

- Stop using mechanical 300-line `parts/**/body.rs` `include!()` splicing to satisfy file/sibling-count lints. Extract high-fanout subsystems out of the 425K-LOC root `molten` crate into genuine encapsulated Rust modules (with explicit `pub(crate)` visibility) and workspace crates (`molten-raft`, `molten-world`, `molten-cli`, `molten-testing`, etc.), starting with the worst `include!()` hotspots (`node/daemon.rs`, `retention/mod.rs`, `job/dag.rs`, `harness/schema.rs`, `chunk/store.rs`).
- Deprecate and collapse the 67 `#[doc(hidden)]` module pseudonyms in `src/lib.rs` and `src/parts/lib/p000/body.rs` (`quorum` → `raft_control_plane`, `codec`, `edgeway`, `vault`, etc.) so every subsystem has one canonical path and name, and rename `raft_control_plane` / `quorum` truthfully if it remains a single-process registry.
- Audit dead boundary planners in `crates/molten-core/src/{dependency,planning,policy,preserves_profile}.rs`: wire any genuinely needed checks into production shells and delete unused toy boolean stubs.
- Relocate generated Nickel JSON embedded by `src/wasm/` out of `docs/` into `config/**/generated/` and update `docs/modularity-boundaries.md` to truthfully describe `molten-core` external dependencies, `std::fs::` shell boundaries, and Nickel evaluation boundaries.

## Unified Governance Toolchains and Defect-Tracking Discipline

- Consolidate `flake.nix`, `flake.lock`, and `Cargo.toml` to a single pinned Cairn revision (eliminating the unpinned `../cairn` policy divergence and placeholder hashes in `cairn-policy/default.ncl`), a single pinned Octet revision, and a single pinned ChaosControl revision; isolate `wasmtime = "=33.0.0"` strictly behind an optional `component-rpc` workspace package or migrate `component-rpc` onto the primary Wasmtime 49 cohort.
- Delete tracked stale backup artifacts (`cairn-policy/generated/cairn-policy.json.stale-bak`) and add a hygiene check rejecting tracked `.stale-bak` files and `include_str!` references into `docs/`.
- Require any source-vs-spec discrepancy identified in `docs/technical/` to reference a tracked Cairn defect change rather than remaining an untracked disclaimer.
