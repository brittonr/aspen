# Forge runtime service source-anchor tests

- Change: `define-runtime-service-core`
- Task: Forge startup wiring, route registration, and secret-safe receipt tests
- Started: `2026-05-07T00:27:26Z`
- Completed: `2026-05-07T00:29:14Z`

## Implemented

Added focused tests in `crates/aspen-forge/src/runtime_service.rs`:

- `forge_runtime_factory_is_linked_native_startup_anchor` proves the Forge wrapper is a linked native built-in startup anchor, admits through `admit_native_factory()`, and projects to a `NativeBuiltIn` declaration with a built-in artifact.
- `forge_routes_are_owned_and_registered_in_manifest` proves the Git, repo/RPC, and health route IDs are registered in the manifest and owned by the `forge` unit.
- `forge_runtime_receipts_are_secret_safe` proves health and lifecycle receipts admit through `admit_receipt()`, contain no raw secret-shaped diagnostics, and summarize authority through capability handles only.

## Verification

```console
$ rustfmt crates/aspen-forge/src/runtime_service.rs
$ CARGO_TARGET_DIR=target/agent cargo test -p aspen-forge runtime_service --all-targets
running 3 tests
test runtime_service::tests::forge_runtime_factory_is_linked_native_startup_anchor ... ok
test runtime_service::tests::forge_routes_are_owned_and_registered_in_manifest ... ok
test runtime_service::tests::forge_runtime_receipts_are_secret_safe ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 360 filtered out; finished in 0.00s
```
