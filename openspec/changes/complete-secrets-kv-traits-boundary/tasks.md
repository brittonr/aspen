# Tasks

## Implementation

- [x] Switch `aspen-secrets` storage signatures from `aspen_core::KeyValueStore` to `aspen_traits::KeyValueStore`.
- [x] Import KV operation types directly from `aspen-kv-types`.
- [x] Remove direct `aspen-core` usage from `crates/aspen-secrets`.

## Verification

- [x] Verify `aspen-secrets` no-default check/test compatibility.
- [x] Verify trust/runtime handler/node compatibility.
- [x] Capture dependency evidence proving no-default `aspen-secrets` does not depend on `aspen-core` or forbidden runtime shells.
- [x] Validate OpenSpec and markdown artifacts before archive.
