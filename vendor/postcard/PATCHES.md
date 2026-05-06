# Aspen patches for `postcard`

Vendored from crates.io `postcard 1.1.3`.

Local delta:

- Remove `heapless-cas` from the crate's default feature set.

Reason: `heapless-cas` pulls `heapless 0.7` with `atomic-polyfill 1.0.3`, which
is unmaintained (`RUSTSEC-2023-0089`). Aspen and its selected upstream users of
postcard in this graph use std/alloc serialization and derive support; no
verified selected path requires postcard's heapless CAS integration.

Aspen also patches known direct upstream call sites (`iroh-blobs`,
`iroh-metrics`, `iroh-tickets`) to disable postcard defaults explicitly, but
this crate-level patch covers other default-feature consumers in the locked graph
such as `irpc` and `wasmtime`.

Remove this patch once upstream postcard changes its default feature policy, or
all selected parents disable postcard default features or move to a postcard /
heapless stack that no longer selects `atomic-polyfill`.
