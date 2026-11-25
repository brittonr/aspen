# Design

## Common Extraction Contract (Atop `molten-preserves-core`)

Each crate in this package declares `#![no_std]` + `extern crate alloc`, depends on `molten-preserves-core` (plus `core`/`alloc`), uses typed `core::error::Error` enums, replaces nominal `std::path::PathBuf` labels with validated relative UTF-8 identifier strings (no OS path semantics inside pure kernels), and replaces all `parts/**/body.rs` `include!()` shards with normal Rust modules.

## Crate Decomposition

1. **`crates/molten-retention-core` (~9,100 LOC):** Extracts pure custody, retention-pin, reachability, and GC-plan transitions from `src/retention/parts/mod/`, leaving filesystem/Redb deletion shells in `src/retention/`.
2. **`crates/molten-evidence-proof-core` (~12,830 LOC):** Extracts `src/evidence/`, `src/provenance/`, and pure receipt builders/validators from `src/testing/{traceability,hardening,drift,prooftrace}`, keeping file scanning and CLI rendering in `src/testing/`.
3. **`crates/molten-resources-core` (~3,355 LOC):** Extracts `src/resources/` pure resource-limit, quota, and backpressure admission cores.
4. **`crates/molten-job-dag-core` (~6,000+ LOC):** Splits `src/job/parts/dag/` cleanly into a pure `#![no_std]` DAG validation, acyclicity, signature-binding, output-ref routing, and topological planning crate (`crates/molten-job-dag-core`) while leaving Nickel evaluation (`src/job/authoring/`), `cap_std`, and process orchestration in `src/job/`.
5. **`crates/molten-catalog-upgrades-core` (~13,645 LOC):** Extracts pure decision cores from `src/catalog/` (query/admission), `src/upgrades/` (quarantine/migration plans), `src/plugin/` (manifest admission), `src/secrets/` (classification/redaction policy), and `src/peer/` (bootstrap negotiation), leaving live I/O adapters in `src/`.

## CI & Compatibility

- Keep public re-exports in `src/lib.rs` intact while eliminating internal `include!()` sprawl.
- Register all five crates in `Cargo.toml`, `[workspace.metadata.octet]`, `.config/nextest.toml`, `flake.nix` (`no_std` compile gates + tests), and `build-plan.json`.
