# Aspen patches for `iroh-blobs`

Vendored from crates.io `iroh-blobs 0.99.0`.

Local delta:

- Disable default features on `postcard`.
- Enable only the features `iroh-blobs` actually uses: `alloc`, `use-std`, and
  `experimental-derive`.

Reason: postcard's default feature set enables `heapless-cas`, which pulls
`heapless 0.7` with `atomic-polyfill 1.0.3`. `atomic-polyfill` is unmaintained
(`RUSTSEC-2023-0089`). `iroh-blobs` uses postcard for std/alloc serialization
and derive helpers, not postcard's heapless CAS support.

Remove this patch once upstream `iroh-blobs` disables postcard default features
or otherwise drops the `atomic-polyfill` edge.
