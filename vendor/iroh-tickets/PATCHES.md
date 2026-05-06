# Aspen patches for `iroh-tickets`

Vendored from crates.io `iroh-tickets 0.4.0`.

Local delta:

- Disable default features on `postcard`.
- Enable only `alloc` and `use-std`.

Reason: postcard's default feature set enables `heapless-cas`, which pulls
`heapless 0.7` with `atomic-polyfill 1.0.3`. `atomic-polyfill` is unmaintained
(`RUSTSEC-2023-0089`). `iroh-tickets` only needs std/alloc ticket
serialization, not postcard's heapless CAS support.

Remove this patch once upstream `iroh-tickets` disables postcard default
features or otherwise drops the `atomic-polyfill` edge.
