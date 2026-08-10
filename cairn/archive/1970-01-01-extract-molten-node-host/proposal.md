## Why

Molten keeps capability-rooted node state and local-store effects in its broad root package. That shape does not let Cargo enforce the node host boundary or reject CLI, presentation, and release-policy dependencies.

The node daemon also depends on many root-owned semantic modules. Moving the full daemon first would create a Cargo dependency cycle or transfer workload meaning into a host crate.

## What Changes

- Add one internal `molten-node-host` crate.
- Move the shared error type, capability-rooted node state, and local-store authority into that crate.
- Keep `molten::error`, `molten::node_state`, and `molten::local_store` as compatibility re-exports.
- Depend on `molten-core` without moving pure decisions or workload semantics into the host crate.
- Add positive, forbidden-dependency, missing-required-dependency, and malformed-manifest fixtures.
- Keep CLI parsing, operator presentation, test harnesses, NixOS validation, release policy, daemon orchestration, service semantics, and workload semantics in the root package for this first compatibility slice.

## Impact

- **Files**: `Cargo.toml`, `Cargo.lock`, `crates/molten-node-host/**`, root compatibility modules, focused Nix checks, README and node authority documentation, and Cairn lifecycle artifacts.
- **Compatibility**: Existing public paths, error variants, capability types, state layout, and filesystem behavior remain available.
- **Evidence**: Baseline and post-change tests, positive and negative dependency fixtures, Cargo/Clippy checks, Octet or repository source gates, Cairn gates, and Nix checks.
- **Non-claims**: This change does not prove filesystem race freedom, durability, distributed correctness, workload correctness, or release readiness. It does not move the complete node daemon or service runtime.
