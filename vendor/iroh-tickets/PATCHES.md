# Aspen patches for `iroh-tickets`

Vendored from crates.io `iroh-tickets 0.4.0`.

Local delta:

- Disable default features on `postcard`.
- Enable only `alloc` and `use-std`.
- Relax `iroh-base` from `0.97` to `0.98` so the Aspen workspace can use the
  Hickory-advisory remediation line.

Reason: postcard's default feature set enables `heapless-cas`, which pulls
`heapless 0.7` with `atomic-polyfill 1.0.3`. `atomic-polyfill` is unmaintained
(`RUSTSEC-2023-0089`). `iroh-tickets` only needs std/alloc ticket
serialization, not postcard's heapless CAS support. The Iroh `0.98` edge is
needed to get onto fixed Hickory `0.26.1` releases.

Remove this patch once upstream `iroh-tickets` disables postcard default
features or otherwise drops the `atomic-polyfill` edge, and Aspen no longer needs
to carry local Hickory-remediation dependency alignment.
