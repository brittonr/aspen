## Why

3 of 9 NixOS VM integration tests fail with pre-existing bugs unrelated to test infrastructure. These are the last blockers to a fully green test suite: Transit datakey returns no plaintext, multinode deploy build times out in nested QEMU, and snix bridge import fails through the gRPC adapter.

## What Changes

- Fix Transit datakey `key_type` parameter handling in the WASM secrets plugin so plaintext is returned by default (not only when `key_type == "plaintext"`)
- Fix SOPS client to send `"plaintext"` as the datakey return mode instead of `"aes256-gcm"`
- Switch `ci-dogfood-deploy-multinode` test to use `validate_only` deploy mode, matching the single-node pattern — the test exists to exercise rolling deploy coordination, not nix build performance
- Debug and fix `snix-bridge-virtiofs` import failure in the gRPC bridge's in-memory PathInfoService/BlobService adapter layer

## Capabilities

### New Capabilities

- `transit-datakey-fix`: Fix Transit data key generation to return plaintext by default, matching HashiCorp Vault semantics
- `multinode-deploy-timeout-fix`: Restructure multinode deploy test to avoid nix build timeout in nested QEMU
- `snix-bridge-import-fix`: Fix snix-store import through the gRPC bridge's in-memory backends

### Modified Capabilities

## Impact

- `aspen-plugins/crates/aspen-secrets-plugin/src/transit.rs` — datakey plaintext condition
- `crates/aspen-secrets/src/sops/client.rs` — SOPS client key_type parameter
- `nix/tests/ci-dogfood-deploy-multinode.nix` — test config and assertions
- `crates/aspen-snix/src/` — blob_service.rs, pathinfo_service.rs (pending investigation)
- `crates/aspen-snix-bridge/src/main.rs` — potential fixes to in-memory backend wiring
