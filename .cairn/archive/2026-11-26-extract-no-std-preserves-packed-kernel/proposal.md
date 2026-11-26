## Why

The single largest architectural bottleneck trapping ~55,000+ LOC of completely pure deterministic state machines (`runtime`, `retention`, `protocol`, `service`, `capability`, `authority`, `resources`, `coordination`, `evidence`, `testing`, `job`, `catalog`, `upgrades`, `plugin`, `secrets`, `peer`) inside the monolithic `std` root `molten` crate is `preserves = "5.0.0-rc.10"`, which requires `std`.

Yet Aspen's own non-negotiable Identity law depends only on **canonical packed Preserves** (binary wire tags `0x80..0xB7`, strict canonical round-trip validation, bounded decoding, BLAKE3 hashing, lowercase `blake3:` content references, and bounded structural traversal including `Embedded` values)—not on `std`-bound text parsing, pretty-printing, or I/O streams. `src/preserves/packed/bounds.rs` already implements byte-level packed bounds checking without depending on `preserves::IOValue`.

## What Changes

- Extract a new `#![no_std]` + `extern crate alloc` workspace crate `crates/molten-preserves-core` built on `src/preserves/packed/bounds.rs` and `blake3` (`default-features = false`).
- Implement canonical packed Preserves representation (`PackedValue`), strict bounded encoder/decoder (`0x80..0xB7`), domain-separated BLAKE3 identity hashing, strict lowercase `ContentRef` validation, unique `<some>`/`<none>` optional-reference encoding, and `Embedded`-aware bounded structural traversal (covering ambient-token, sensitive-marker, and content-reference scanning).
- Keep human-readable Preserves text parsing/rendering and `preserves::IOValue` interop strictly inside a thin `std` adapter in `src/preserves/`, verified for byte-for-byte canonical equivalence against `preserves = "5.0.0-rc.10"` via differential property tests.
- Add a hermetic `#![no_std]` CI compile gate and differential equivalence suite for `crates/molten-preserves-core`.

## Impact

- **Files**: `Cargo.toml`, `crates/molten-preserves-core/` (new workspace member), `src/preserves/` (thin `std` bridge + differential tests), `flake.nix`, `build-plan.json`, and `docs/architecture.md`.
- **Verification**: `#![no_std]` compile check, bounded Hegel differential tests proving byte-for-byte canonical packed equivalence against `preserves = "5.0.0-rc.10"` across records, sequences, sets, dictionaries, integers, IEEE floats, strings, symbols, byte strings, booleans, annotations, and `Embedded` values, plus negative tests for non-canonical bytes, depth/node exhaustion, uppercase digests, and `Embedded` security-scan traversal.
- **Non-claims**: Providing a `no_std` canonical packed Preserves kernel establishes byte-level wire determinism and unlocks downstream `no_std` kernel extraction; it does not replace external Preserves schema authoring or grant authority.

## Out of Scope

- Migrating downstream Tier 2 subsystems onto `molten-preserves-core` (tracked in `extract-no-std-dataspace-authority-and-protocol-kernels` and `extract-no-std-retention-evidence-job-and-catalog-kernels`).
- Supporting human-readable Preserves text syntax inside `no_std` (text syntax remains in `std` CLI/adapters).
