# Complete Secrets KV Traits Boundary

## Why

The previous trust/crypto/secrets slices removed concrete Iroh identity coupling from reusable crypto defaults and kept `aspen-auth` runtime helpers behind `aspen-secrets/auth-runtime`. The next public API blocker was that `aspen-secrets` storage adapters still exposed the broad `aspen_core::KeyValueStore` compatibility surface even though lighter KV traits and operation types already exist.

## What Changes

- Move `AspenSecretsBackend`, `MountRegistry`, and SOPS runtime KV manager signatures from `aspen_core::KeyValueStore` to `aspen_traits::KeyValueStore`.
- Import KV operation request/result/error types from `aspen-kv-types` instead of the `aspen-core` compatibility re-export.
- Preserve current runtime consumers and `aspen-secrets-handler` compatibility.
- Record evidence that `aspen-secrets --no-default-features` no longer has a normal dependency on `aspen-core` while retaining runtime checks.

## Non-Goals

- This change does not split a new `aspen-secrets-core` crate.
- This change does not promote `trust-crypto-secrets` beyond `workspace-internal`.
- This change does not decide publication or license policy.
