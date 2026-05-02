## Implementation

- [x] Gate secrets handler runtime-context factory APIs behind `runtime-adapter`.
- [x] Replace `aspen-core` KV imports in `aspen-secrets-handler` with `aspen-traits` and `aspen-kv-types`.
- [x] Preserve aggregate/node compatibility by enabling the adapter through `aspen-rpc-handlers`' `secrets` feature.

## Verification

- [x] Capture portable handler no-default build evidence.
- [x] Capture runtime adapter and aggregate compatibility evidence.
- [x] Capture handler/secrets test evidence and negative dependency/source guards.
- [x] Validate and archive this OpenSpec change.
