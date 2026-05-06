# Aspen patches for `iroh-metrics`

Vendored from crates.io `iroh-metrics 0.38.3`.

Local delta:

- Disable default features on runtime and dev `postcard` dependencies.
- Enable only `alloc` and `use-std`.

Reason: postcard's default feature set enables `heapless-cas`, which pulls
`heapless 0.7` with `atomic-polyfill 1.0.3`. `atomic-polyfill` is unmaintained
(`RUSTSEC-2023-0089`). `iroh-metrics` only serializes std/alloc metric data and
does not need postcard's heapless CAS support.

Remove this patch once upstream `iroh-metrics` disables postcard default
features or otherwise drops the `atomic-polyfill` edge.
