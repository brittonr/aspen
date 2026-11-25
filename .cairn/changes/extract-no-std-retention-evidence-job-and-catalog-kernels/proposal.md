## Why

After extracting runtime, authority, protocol, and coordination kernels, another ~44,900 LOC of pure deterministic planning, validation, and evidence-composition logic remain trapped in the monolithic `std` root `molten` crate solely because of `preserves::IOValue` and nominal `PathBuf` labels (plus heavy `parts/**/body.rs` `include!()` fragmentation):

1. **Retention & Custody Planner (~9,100 LOC):** `src/retention/` owns pure GC/retention/pin planning and custody decisions (zero I/O; blocked only by `preserves::IOValue` and `PathBuf` string labels).
2. **Evidence, Proof Composition, Traceability & Provenance Core (~12,830 LOC):** `src/evidence/`, `src/testing/{traceability,hardening,drift,prooftrace}`, and `src/provenance/` own pure aggregate/layered proof composition, verification-run receipt derivation, tamper matrices, and lineage chains (zero I/O; blocked only by `preserves::IOValue` and `PathBuf` labels).
3. **Resource Governance & Backpressure Core (~3,355 LOC):** `src/resources/` owns pure resource-limit and backpressure admission with zero I/O.
4. **Job DAG Planning & Routing Core (~6,000+ LOC of 9,232):** `src/job/parts/dag/` mixes pure DAG acyclicity, typed signature/output-reference routing, and deterministic topological scheduling with a small set of `std::fs`/`cap_std` and Nickel authoring adapters across 42 `include!()` shards.
5. **Catalog, Upgrades, Plugins, Secrets Policy & Peer Bootstrap (~13,645 LOC):** `src/catalog/`, `src/upgrades/`, `src/plugin/`, `src/secrets/`, and `src/peer/` own pure catalog query semantics, upgrade quarantine/migration planners, plugin manifest admission, secret-policy redaction/classification rules, and peer bootstrap negotiation with zero I/O (blocked only by `preserves::IOValue` and `PathBuf` labels).

## What Changes

- Extract five `#![no_std]` + `extern crate alloc` workspace crates built atop `molten-preserves-core`:
  - `crates/molten-retention-core` (retention/custody/pin/GC planning)
  - `crates/molten-evidence-proof-core` (evidence chains, provenance, aggregate/layered proofs, traceability derivation, hardening/drift receipts)
  - `crates/molten-resources-core` (resource-limit and backpressure admission)
  - `crates/molten-job-dag-core` (pure JobDag graph validation, topological scheduling, and exact output-ref routing, keeping Nickel evaluation and `cap_std` I/O in `src/job/`)
  - `crates/molten-catalog-upgrades-core` (pure catalog queries, upgrade migration/quarantine plans, plugin manifest admission, secret-policy classification, and peer bootstrap negotiation)
- Replace nominal `PathBuf` fields in pure data structures with validated UTF-8 path-segment strings (`alloc::string::String`), eliminating accidental ambient path types from pure kernels.
- Replace mechanical `parts/**/body.rs` `include!()` sharding across these five domains with genuine encapsulated Rust modules and gate all five crates with `#![no_std]` compilation, unit/property tests, and workspace Octet in `flake.nix`.

## Impact

- **Files**: `Cargo.toml`, new workspace crates under `crates/molten-{retention,evidence-proof,resources,job-dag,catalog-upgrades}-core/`, thin `std` adapters/re-exports in `src/retention/`, `src/evidence/`, `src/testing/`, `src/provenance/`, `src/resources/`, `src/job/`, `src/catalog/`, `src/upgrades/`, `src/plugin/`, `src/secrets/`, `src/peer/`, `flake.nix`, `build-plan.json`, and `docs/architecture.md`.
- **Verification**: Hermetic `#![no_std]` compile gates for all five crates, canonical packed Preserves byte-parity checks against existing suites, Clippy, workspace Octet, and strict Cairn validation.
- **Non-claims**: Moving these planning and evidence cores to `#![no_std]` enforces compile-time purity and modular encapsulation; it does not replace runtime storage adapters or qualify external deployments.

## Out of Scope

- Building `crates/molten-preserves-core` (prerequisite in `extract-no-std-preserves-packed-kernel`).
- Repairing `layered_proof_diagnostics` and `tools/tracey/inherited_debt_guard.rs` logic bugs (tracked in `harden-traceability-proof-composition-and-static-audits`; both changes coordinate cleanly on `molten-evidence-proof-core`).
