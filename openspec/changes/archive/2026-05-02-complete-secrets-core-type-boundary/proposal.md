# Complete secrets core type boundary

## Why

`aspen-secrets` had already shed broad auth, KV, and handler runtime dependencies, but the portable KV, Transit, PKI, and limit contracts still lived inside the runtime secrets crate. This made downstream users depend on the service/runtime shell even when they only needed stable request/response DTOs and in-memory state types.

## What Changes

- Add `aspen-secrets-core` as the owner of portable secrets constants plus KV, Transit, and PKI type/state contracts.
- Keep `aspen-secrets` compatibility modules and root re-exports so existing imports continue to compile.
- Leave runtime stores, cryptographic execution, SOPS file IO, auth runtime helpers, trust integration, and handler adapters in runtime crates.

## Impact

- Files: workspace manifest, `crates/aspen-secrets-core/**`, `crates/aspen-secrets/**`, this manifest, and trust/crypto/secrets extraction docs/specs.
- APIs: new `aspen_secrets_core::{constants, kv, transit, pki}` module surface; existing `aspen_secrets::{constants, kv::types, transit::types, pki::types}` paths remain compatibility re-exports.
- Dependencies: `aspen-secrets-core --no-default-features` depends only on `serde` for normal builds.
- Testing: core check/test, runtime secrets check/test, handler compatibility checks, node secrets bundle, negative dependency grep, OpenSpec validation, markdownlint, and diff checks.
