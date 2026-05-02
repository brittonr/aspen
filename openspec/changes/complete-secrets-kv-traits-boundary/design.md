# Design

## Context

`aspen-core` re-exports `aspen-traits` and `aspen-kv-types` for compatibility, but it is a broader compatibility surface than reusable secrets storage needs. `aspen-secrets` only needs the composite KV trait and typed KV requests/results/errors for its Aspen-backed storage adapters.

## Decision

`aspen-secrets` now depends directly on:

- `aspen-traits` for `KeyValueStore` and KV capability traits in tests.
- `aspen-kv-types` for `ReadRequest`, `WriteRequest`, `WriteCommand`, `DeleteRequest`, `ScanRequest`, and associated result/error types.

The runtime storage implementations remain in `aspen-secrets` for now, but their public signatures no longer name `aspen_core::KeyValueStore`.

## Compatibility

`aspen-core::KeyValueStore` is a re-export of the same trait, so existing runtime stores continue to satisfy the new signatures. `aspen-secrets-handler` and node bootstrap checks prove the compatibility path.

## Risks

- `aspen-secrets` still has heavyweight crypto/runtime dependencies unrelated to this storage API boundary. A future `aspen-secrets-core` or feature split should handle pure types/state-machine extraction.
- SOPS runtime manager still supports KV-backed runtime secrets, but now via the lightweight trait path rather than `aspen-core`.
