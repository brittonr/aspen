## 1. Measure First (gate all subsequent work on results)

- [ ] 1.1 Add `FuseAccessStats` struct to `aspen-fuse`: counters for `files_opened`, `files_read`, `bytes_fetched_from_cluster`, `bytes_served_from_cache`, `prefetch_bytes_fetched`, track per-mount
- [ ] 1.2 Hook counters into `open()` (increment `files_opened`), `read()` (increment `files_read` on first read per inode), cache hit/miss paths
- [ ] 1.3 Add `prefetch_file_start()` byte counter to track how many prefetch bytes are actually used vs wasted (prefetched but never read before eviction)
- [ ] 1.4 Expose stats via `AspenFs::stats()` method and log summary on `destroy()` (unmount)
- [ ] 1.5 Run a real CI build (nix build, cargo build) with instrumented FUSE, collect stats
- [ ] 1.6 Decision gate: if <30% of opened files are read, proceed with lazy fetch. If >70% are read, the eager prefetch is justified — stop here and close the change

## 2. Lazy File Fetch

- [ ] 2.1 Remove `prefetch_file_start()` call from `open()` handler in `fs/operations.rs`
- [ ] 2.2 Verify `read()` still works correctly as the first data fetch (chunked_read_range handles cache miss → cluster fetch)
- [ ] 2.3 Verify sequential readahead in `prefetch.rs` still triggers after 3 sequential reads (no regression from removing open-time prefetch)
- [ ] 2.4 Update `prefetch_file_start()` doc to note it's no longer called from `open()` but remains available for explicit prefetch
- [ ] 2.5 Unit test: open 100 files, read 10, verify only 10 file fetches occurred
- [ ] 2.6 Benchmark: measure first-read latency with and without eager prefetch on a remote cluster (expect 2-5ms increase)

## 3. Content-Hash Cache Validation

- [ ] 3.1 Add `content_hash: Option<[u8; 32]>` field to data cache entries in `cache.rs`
- [ ] 3.2 Populate `content_hash` when data is fetched from iroh-blob-backed files (hash is known from blob store)
- [ ] 3.3 Add `HashCheck { key: String, expected_hash: [u8; 32] }` request variant to `ClientRpcRequest` in `aspen-client-api`
- [ ] 3.4 Add `HashCheckResult { changed: bool, new_hash: Option<[u8; 32]> }` response variant to `ClientRpcResponse`
- [ ] 3.5 Implement `HashCheck` handler on the server side: look up current blob hash for key, compare with expected
- [ ] 3.6 Implement stale-cache revalidation in `AspenFs::kv_read()`: on expired cache entry with hash, send `HashCheck` → if unchanged extend TTL to 60s, if changed re-fetch
- [ ] 3.7 Add `CACHE_REVALIDATED_TTL` constant (60s default) to `constants.rs`
- [ ] 3.8 Add `HASH_CHECK_TIMEOUT_MS` constant (100ms default) — fall through to full fetch on timeout
- [ ] 3.9 Unit test: write file → cache it → expire TTL → hash-check returns unchanged → verify no re-fetch, TTL extended
- [ ] 3.10 Unit test: write file → cache it → change file on cluster → expire TTL → hash-check returns changed → verify re-fetch occurs

## 4. Offline Mode

- [ ] 4.1 Add `is_degraded: AtomicBool` field to `AspenFs`
- [ ] 4.2 In `FuseSyncClient::rpc_call()`, on connection error: set `is_degraded = true`, return error
- [ ] 4.3 In `FuseSyncClient::rpc_call()`, on success: set `is_degraded = false`
- [ ] 4.4 In `AspenFs::kv_read()`, on cluster error: check data cache for stale entry → if found, serve stale with warn log → if not found, return EIO
- [ ] 4.5 On reconnection (degraded → healthy transition): queue hash-check revalidation for all stale-served entries
- [ ] 4.6 Ensure writes still fail immediately in offline mode (no buffering)
- [ ] 4.7 Unit test: disconnect cluster → read cached file → verify stale data returned → reconnect → verify revalidation occurs
- [ ] 4.8 Unit test: disconnect cluster → read uncached file → verify EIO returned

## 5. Branch Integration

- [ ] 5.1 Verify branch dirty map reads bypass all cache/lazy logic (they're in-memory, always immediate)
- [ ] 5.2 Verify branch fall-through reads to base store use the lazy fetch + hash-check path
- [ ] 5.3 Verify branch reads work in offline mode (dirty map reads succeed, fall-through reads serve stale)
- [ ] 5.4 Unit test: branch with dirty key → read → verify immediate (no cache, no RPC). Branch without dirty key → read → verify falls through to lazy base store path

## 6. Documentation and Constants

- [ ] 6.1 Update `crates/aspen-fuse/src/lib.rs` module doc to describe lazy fetch behavior
- [ ] 6.2 Add `CACHE_REVALIDATED_TTL`, `HASH_CHECK_TIMEOUT_MS`, `OFFLINE_STALE_WARN_INTERVAL` to `constants.rs` with compile-time assertions
- [ ] 6.3 Update prefetch.rs module doc to clarify that open-time prefetch is removed, only sequential readahead remains
