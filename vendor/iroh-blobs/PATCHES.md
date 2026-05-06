# Aspen patches for `iroh-blobs`

Vendored from crates.io `iroh-blobs 0.99.0`.

Local delta:

- Disable default features on `postcard`.
- Enable only the features `iroh-blobs` actually uses: `alloc`, `use-std`, and
  `experimental-derive`.
- Relax `iroh` / `iroh-base` dependency constraints from `0.97` to `0.98` so the
  Aspen workspace can use the Hickory-advisory remediation line.
- Update vendored/test call sites for `SecretKey::generate()` after the Iroh
  `0.98` API change.

Reason: postcard's default feature set enables `heapless-cas`, which pulls
`heapless 0.7` with `atomic-polyfill 1.0.3`. `atomic-polyfill` is unmaintained
(`RUSTSEC-2023-0089`). `iroh-blobs` uses postcard for std/alloc serialization
and derive helpers, not postcard's heapless CAS support. The Iroh `0.98` edge is
needed to get onto fixed Hickory `0.26.1` releases.

Remove this patch once upstream `iroh-blobs` disables postcard default features
or otherwise drops the `atomic-polyfill` edge, and Aspen no longer needs to carry
local Hickory-remediation dependency alignment.
