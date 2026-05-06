# Local Aspen patches

- Replaced the optional `rpc` feature's `bincode 1.x` dependency with `postcard` (`use-std`) to remove `RUSTSEC-2025-0141` from Aspen's locked dependency graph.
- Updated madsim RPC/ERPC serialization call sites from `bincode::{serialize,deserialize}` to `postcard::{to_stdvec,from_bytes}`.

Remove this vendor patch when upstream madsim drops `bincode 1.x` or otherwise moves to a maintained serializer.
