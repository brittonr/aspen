## Why

`aspen-secrets --no-default-features` still pulled the runtime `aspen-auth` shell, which in turn pulled `aspen-core-shell`, concrete Iroh transport, and Redb through auth verifier/builder convenience helpers. That made the reusable secrets default boundary heavier than the trust/crypto/secrets extraction intent.

## What Changes

- Move default token parsing in `aspen-secrets` to portable `aspen-auth-core::CapabilityToken`.
- Add explicit `aspen-secrets/auth-runtime` for `TokenVerifier` and `TokenBuilder` convenience helpers backed by `aspen-auth`.
- Keep root node `secrets` compatibility by enabling `aspen-secrets/auth-runtime` from the runtime feature bundle.

## Impact

- **Files**: `crates/aspen-secrets`, root feature wiring, trust/crypto/secrets docs/spec evidence.
- **APIs**: `SecretsManager::{build_token_verifier,build_token_builder}` now require `auth-runtime`; token retrieval stays available by default via `aspen-auth-core`.
- **Testing**: no-default secrets tests, auth-runtime check, secrets-handler check, node runtime compatibility check, dependency-tree negative evidence.
