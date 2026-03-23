## Why

The recent multi-lens codebase audit revealed a severe violation of the Aspen Tiger Style principles regarding safety and error handling. Specifically, the mandate dictates: *"No `.unwrap()`/`.expect()` in production... No `panic!()`/`todo!()`/`unimplemented!()` in production."*

However, the audit found:

- ~3,414 instances of `.unwrap()` in production `src/` directories.
- ~1,304 instances of `.expect()`.
- ~491 instances of `panic!()`.

These represent unhandled error paths and unproven assumptions. In a distributed system orchestration layer like Aspen, unhandled panics can lead to sudden node crashes, compromised liveness guarantees, and inconsistent state during partial failures. This change aims to eradicate these code smells and enforce rigorous error handling using the `snafu` crate.

## What Changes

- **The Great Unwrap Purge:** Systematically replace `.unwrap()` and `.expect()` calls in all production `src/` directories with proper `Result` propagation (`?`) and `snafu::context()`.
- **Panic Eradication:** Replace `panic!()` and `todo!()` macros in production code with cleanly modeled error states or exhaustive match statements.
- **Error Types:** Expand existing `snafu` error enums in the relevant crates (or create new ones where absent) to accommodate the new error paths discovered during the purge.
- **Testing Exemption:** `unwrap()`, `expect()`, and `panic!()` will remain permissible in `tests/` directories and `#[test]` macros, as they are standard practice for asserting test preconditions.

## Capabilities

### Modified Capabilities

- `error-handling`: Enhances system resilience by replacing fatal panics with propagated, actionable error contexts across the cluster. Node processes will gracefully handle I/O, networking, and consensus failures rather than crashing unexpectedly.

## Impact

- **Files**: Broad impact across `crates/**/src/*.rs`.
- **APIs**: Internal function signatures will change to return `Result<T, Error>` where they previously panicked. Public/Client Iroh RPC APIs may need minor adjustments to return newly categorized errors.
- **Dependencies**: No new dependencies. Heavier reliance on `snafu` and `anyhow`.
- **Testing**: Tests must be updated to expect and handle `Result` types where functions have been modified to return errors instead of panicking.
