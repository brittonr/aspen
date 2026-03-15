## Why

AspenFs mounts a remote KV store as a POSIX filesystem over QUIC. When a tool runs `readdir()`, the FUSE layer calls `kv_scan()` which fetches all keys under the prefix from the cluster. When a file is opened, `prefetch_file_start()` eagerly loads the first 128KB. For workspaces with thousands of files (a typical Rust repo has 5,000+ files in `target/`), this means readdir triggers a full scan RPC and open triggers an eager prefetch — even for files the tool will never read.

ix.dev solves this differently: their filesystem fetches file content on demand. `readdir()` returns metadata only (no content). `read()` fetches content from storage on first access. Previously read files stay cached. Files never accessed are never fetched.

Aspen already has most of the building blocks:

- **Readdir is already metadata-only** — `readdir()` scans KV keys and returns names/types without reading file content. This is already lazy at the data level.
- **Prefetch is speculative, not blocking** — `queue_dir_prefetch()` queues metadata prefetch after readdir, but `prefetch_file_start()` on `open()` eagerly reads the first 128KB before the caller asks for it.
- **Chunked storage** — files >512KB are split into chunks. `chunked_read_range()` only fetches the chunks overlapping the requested range. This is already lazy at the chunk level.
- **Read cache with TTL** — data cache (5s TTL, 64MB cap), metadata cache (2s TTL), scan cache (1s TTL). Previously fetched data serves from cache.

The gap is narrow: the eager prefetch on `open()` and the scan-based readdir that hits the cluster for every directory listing. For large remote workspaces where only a fraction of files are actually read, removing the eager prefetch and adding smarter scan caching could reduce network traffic and latency.

The question is whether this gap matters enough to justify the complexity. This proposal evaluates what would change and the expected impact.

## What Changes

- Make file `open()` truly lazy: remove `prefetch_file_start()` from the open path, let the first `read()` trigger the actual fetch
- Add scan result caching with longer TTL for remote workspaces where directory contents change infrequently
- Add content-hash-based cache validation: instead of TTL-based expiry, check if the content hash has changed (iroh-blobs hashes are free)
- Add offline mode: previously fetched files remain readable when the cluster is unreachable, with stale cache serving instead of errors
- Add access pattern statistics: track which files are actually read vs only stat'd, to quantify the benefit of lazy evaluation

## Capabilities

### New Capabilities

- `lazy-file-fetch`: Defer file content fetching from `open()` to first `read()`, eliminating speculative prefetch for files that are opened but never read
- `content-hash-cache-validation`: Use iroh-blobs BLAKE3 hashes to validate cached data instead of TTL-based expiry, enabling longer cache lifetimes without serving stale data
- `fuse-offline-mode`: Serve stale cached data when the cluster is unreachable, with degraded-mode indicators

### Modified Capabilities

- `kv-branch-fuse`: Branch-resolved reads need to work with the lazy fetch path and content-hash validation

## Impact

- `crates/aspen-fuse/src/fs/operations.rs` — remove `prefetch_file_start()` call from `open()`, adjust read path
- `crates/aspen-fuse/src/prefetch.rs` — remove eager file-start prefetch, keep sequential readahead
- `crates/aspen-fuse/src/cache.rs` — add content-hash validation, longer TTL for hash-validated entries, offline fallback
- `crates/aspen-fuse/src/client.rs` — add hash-check RPC (get content hash without fetching content), connection failure handling for offline mode
- `crates/aspen-fuse/src/constants.rs` — new constants for lazy mode, offline TTL, hash-check timeout
- `crates/aspen-client-api/` — add lightweight hash-check request type if not already present
