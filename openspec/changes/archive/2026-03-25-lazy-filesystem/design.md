## Context

AspenFs is already lazier than it looks at first glance. Here's what actually happens today on the hot path:

**readdir()** — calls `kv_scan(prefix)` which returns keys (not values). Content is never fetched during directory listing. After readdir, `queue_dir_prefetch()` queues metadata prefetch (mtime, size) for listed entries. This metadata prefetch runs on the next FUSE operation via `execute_pending()`. The readdir itself is metadata-only.

**open()** — calls `prefetch_file_start()` which immediately reads the first 128KB of the file into the data cache. This is the main eager behavior. If a tool opens 500 files but only reads 10, we've fetched 500×128KB = 62.5MB unnecessarily.

**read(offset, size)** — calls `chunked_read_range()` which only fetches the KV entries (chunks) overlapping the requested byte range. For a 10MB file stored as 20×512KB chunks, reading bytes 0-1000 only fetches chunk 0. This is already lazy at the chunk level.

**cache** — data cache (5s TTL, 64MB, 1000 entries), metadata cache (2s TTL, 5000 entries), scan cache (1s TTL, 500 entries). After first access, subsequent reads serve from cache until TTL expires.

So the actual gap vs ix.dev's approach:

1. **Eager prefetch on open()** — fetches 128KB per open regardless of whether the file will be read
2. **TTL-based cache invalidation** — forces re-fetch every 5s even if content hasn't changed
3. **No offline resilience** — cache miss when cluster is unreachable = error, not stale data

The question: is this gap worth addressing?

## Goals / Non-Goals

**Goals:**

- Quantify the actual overhead of eager prefetch on realistic workloads (CI builds, agent workspace access)
- Remove the eager prefetch on `open()` if measurements show it wastes bandwidth
- Replace TTL-based data cache expiry with content-hash validation for files backed by iroh-blobs
- Add graceful degradation when the cluster is unreachable (serve stale cache)
- Keep sequential readahead — it's genuinely useful for `cat`, `rustc`, and other tools that read files linearly

**Non-Goals:**

- Rewriting the FUSE layer from scratch
- Lazy metadata (stat/getattr) — metadata is already small and cheap to cache
- Lazy directory listing — readdir already returns only keys, not content
- Per-file deduplication across mounts (iroh-blobs already handles this at the blob level)
- Streaming files from remote nodes on the fly (the client already uses QUIC streams)

## Decisions

### 1. Remove eager prefetch on open(), keep sequential readahead

**Decision:** Delete the `prefetch_file_start()` call from the `open()` handler. Keep the sequential readahead logic in `record_read()` that triggers after 3 sequential reads.

**Rationale:** The eager prefetch is the only truly eager behavior. Removing it means the first `read()` call is a cache miss, adding one round-trip (~2-5ms over QUIC). But for files that are opened and never read (very common — tools like `find`, `stat`, `ls` open files to get metadata without reading content), we save 128KB of network transfer per file.

**Trade-off:** Single-read files (open → read all → close) lose the prefetch advantage — first read is now a cache miss. But `chunked_read_range()` already fetches exactly the requested range, so the penalty is one extra round-trip, not a full re-architecture. Sequential readahead still kicks in for multi-read access patterns.

**Alternative considered:** Make prefetch configurable (eager/lazy/off). Rejected because it adds configuration surface without clear benefit — lazy is strictly better for remote filesystems.

### 2. Content-hash cache validation using iroh-blobs BLAKE3

**Decision:** For files backed by iroh-blobs (known BLAKE3 hash), extend cache entries with the content hash. On cache hit with expired TTL, check if the hash has changed via a lightweight RPC (just the hash, not the content). If unchanged, extend the cache entry. If changed, invalidate and re-fetch.

**Rationale:** TTL-based expiry forces re-fetch every 5s even for files that haven't changed. In a CI build, most source files don't change between runs. A hash-check RPC is ~100 bytes on the wire (32-byte hash + overhead) vs re-fetching the file content (potentially megabytes). This turns the cache from "expires every 5s" into "valid until content actually changes."

**Trade-off:** Adds one hash-check round-trip on stale cache entries. For files that change frequently, this is wasted — you do the hash check then re-fetch anyway. But most files in a workspace are read-only during a build.

**Implementation:** Add a `content_hash: Option<[u8; 32]>` field to cache entries. On cache lookup with expired TTL: if hash is known, send `HashCheck { key, expected_hash }` RPC. Response is either `Unchanged` (extend TTL) or `Changed { new_hash }` (invalidate). If the RPC fails (cluster unreachable), fall through to offline mode.

### 3. Offline mode: serve stale on cluster failure

**Decision:** When the cluster is unreachable and a cache entry exists (even if TTL-expired), serve the stale cached data with a warning log. Set a `degraded: AtomicBool` flag on AspenFs that the TUI/CLI can display.

**Rationale:** Agent workloads run for hours. A transient cluster partition shouldn't crash the agent's build. Stale data is better than an error for files that haven't changed. The risk of serving truly stale data (file changed but cluster is down) is low for typical CI workloads where the workspace is mostly static during a build.

**Trade-off:** Writes during offline mode will fail (can't reach Raft). This is acceptable — offline mode is read-only degradation. An agent that can't write can at least continue reading existing files.

### 4. Access pattern statistics for validation

**Decision:** Add per-mount counters: `files_opened`, `files_read`, `bytes_fetched`, `bytes_cached_hit`, `prefetch_wasted_bytes`. Expose via AspenFs stats. Use these to validate that lazy fetch is worth the change before merging.

**Rationale:** We're making an assumption that "most files opened are never read." If measurement shows that 95% of opened files are also read, the eager prefetch was actually helpful and we should keep it. Instrument first, then decide.

## Risks / Trade-offs

**[First-read latency increases for all files]** → Today, `open()` prefetches so the first `read()` is usually a cache hit (~0ms). With lazy fetch, the first `read()` is a cache miss (~2-5ms QUIC round-trip). Mitigation: this is the same latency every subsequent read pays after TTL expiry today. Sequential readahead still handles the common case of reading a whole file.

**[Hash-check RPC adds a new request type]** → The client API needs a new `HashCheck` request. Mitigation: this is a lightweight addition — one new enum variant in `ClientRpcRequest`/`ClientRpcResponse`. The hash is already stored in iroh-blobs; the server just looks it up.

**[Offline mode can serve stale data]** → If a file changed on the cluster but the client's connection is down, reads return old content. Mitigation: log at warn level, set degraded flag, re-validate all stale entries on reconnection. For CI builds (the primary use case), workspace files are written once at job start and read many times — they don't change mid-build.

**[Low impact if prefetch waste is small]** → If measurement shows eager prefetch only wastes 1-2% of bandwidth, the change isn't worth the complexity. Mitigation: implement access statistics first (task group 1), measure on real workloads, gate the rest of the work on the results.

## Open Questions

- Should hash-check validation be synchronous (blocking the read) or async (serve stale, validate in background)? Async is faster but can serve briefly stale data.
- Should offline mode have a max staleness (e.g., serve stale up to 1 hour, then error)? Or unlimited staleness?
- What percentage of files opened during a `cargo build` or `nix build` are actually read? This determines whether lazy fetch matters at all.
