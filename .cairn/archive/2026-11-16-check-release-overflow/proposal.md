## Why

Issue #49: the workspace had no Cargo profile and the Nix package build sets no overflow option, so release builds of Molten wrapped silently on integer overflow. Audit finding F12 was this class of bug (retry arithmetic wrapping to zero). Two concrete sites remained: Raft's quorum-index search added one to the commit index unchecked, and the SpaceWasm allocator subtracted freed sizes unchecked inside an `extern "C"` function, where a panic aborts the process.

## What Changes

- Molten-owned crates (`molten`, `molten-core`, `molten-node-host`, `molten-release-policy`) trap integer overflow in release builds: Cargo release-profile package overrides and matching `-C overflow-checks=on` crate overrides in the Nix package build. Third-party crates keep their own release defaults in both builds.
- `highest_quorum_index` derives its first candidate with `checked_add`; at the maximum commit index there is no higher candidate.
- The SpaceWasm allocator records an accounting fault instead of subtracting past zero; after a fault every allocation is denied, and no arithmetic in the `extern "C"` entry points can panic.
- The decision, its scope and the unmeasured cost are recorded in the development reference.

## Impact

- **Files**: `Cargo.toml`, `flake.nix`, `src/fabric_consistency/raft/transition/message/replication.rs`, `src/spacewasm/allocation.rs`, `docs/development.md`.
- **Testing**: a Raft transition test at the maximum commit index, SpaceWasm allocator tests for a mismatched deallocation, and a flake evaluation check that every Molten-owned crate derivation carries the overflow option.
- **Owner / consumer**: workspace build configuration; every Molten release binary.
