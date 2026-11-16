## Why

Lifecycle generation and incarnation are distinct fencing domains, but adjacent `u64` fields allow callers to interchange them at compile time. Runtime stale checks remain necessary; typed internal interfaces prevent an accidental swap before those checks.

## What Changes

- Introduce distinct full-`u64` `Generation` and `Incarnation` values in system-extension lifecycle, manifest, dispatch, facade, recovery, and native-host models. Explicitly convert at existing numeric APIs and canonical Preserves boundaries without changing their bytes or content references.
- Preserve absent zero, checked overflow, and stale-generation/incarnation denial. This type separation does not confer authority or alter other generation domains.

## Impact

- **Files**: `crates/molten-core/src/system_extension/`, system-extension serialization and host adapters, and their callers and tests in `src/` and `tests/`.
- **Testing**: Compile-fail interface examples, positive and negative boundary tests, byte/ref golden assertions, `cargo test -p molten-core system_extension`, `cargo check -p molten --lib --bins --tests`, nightly rustfmt, and strict Cairn gates.
