# Forge runtime service wrapper

- Change: `define-runtime-service-core`
- Task: Forge runtime service wrapper
- Started: `2026-05-07T00:24:21Z`
- Completed: `2026-05-07T00:27:01Z`

## Implemented

Added `crates/aspen-forge/src/runtime_service.rs` and re-exported its public wrapper helpers from `aspen-forge`.

The wrapper exposes Forge as a linked native runtime service without changing Forge internals:

- `forge_runtime_manifest()` returns a `NativeServiceManifest` for Forge.
- `forge_runtime_service_factory()` returns a linked `NativeBuiltInServiceFactory` with `NativeLoadingPolicy::LinkedBuiltInOnly`.
- `forge_runtime_routes()` declares Git, repo/RPC, and health routes owned by `forge`.
- `forge_health_receipt()` and `forge_lifecycle_receipt()` produce `RuntimeReceipt` values with redacted/opaque diagnostics and capability handle summaries.
- `aspen-forge` now depends on `aspen-runtime-core` for the portable runtime model types.

A stale test-only API call in `crates/aspen-forge/src/identity/nostr_auth.rs` was also updated from `SecretKey::generate(&mut ::rand::rng())` to `SecretKey::generate()` so `aspen-forge --all-targets` can compile against the pinned `iroh-base` API.

## Verification

```console
$ rustfmt crates/aspen-forge/src/runtime_service.rs crates/aspen-forge/src/lib.rs crates/aspen-forge/src/identity/nostr_auth.rs
$ CARGO_TARGET_DIR=target/agent cargo check -p aspen-forge --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.21s
```
