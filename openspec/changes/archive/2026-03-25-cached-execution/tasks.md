## 1. Crate Scaffolding

- [x] 1.1 Create `crates/aspen-exec-cache/` with `Cargo.toml`, `src/lib.rs`, constants module, error types (snafu)
- [x] 1.2 Add `aspen-exec-cache` to workspace `Cargo.toml` and wire feature flag (`exec-cache`) in `aspen-core`
- [x] 1.3 Define `CacheKey` type (32-byte BLAKE3 hash), `CacheEntry` struct (exit_code, stdout_hash, stderr_hash, outputs vec, created_at_ms, ttl_ms), and serialization

## 2. Cache Index (Raft KV)

- [x] 2.1 Implement `ExecCacheIndex` with `get(key) -> Option<CacheEntry>` and `put(key, entry)` against Raft KV using `_exec_cache:` key prefix
- [x] 2.2 Implement TTL check in `get()` — return None for expired entries
- [x] 2.3 Implement cache key computation: `BLAKE3(command || args || env_hash || sorted_input_hashes)` as a pure verified function in `src/verified/`
- [x] 2.4 Implement environment hash: curated variable set (PATH, CC, CXX, CFLAGS, RUSTFLAGS, NIX_STORE), deterministic serialization
- [x] 2.5 Add resource bound constants: MAX_OUTPUT_BLOB_SIZE (1 GB), MAX_OUTPUT_FILES (10,000), MAX_INPUT_FILES (100,000), MAX_CACHE_STORAGE_BYTES (10 GB), DEFAULT_TTL_MS (24h)
- [x] 2.6 Unit tests for cache key determinism (same inputs different order → same key, timestamp changes → same key)

## 3. FUSE Read Tracking

- [x] 3.1 Add `ReadTracker` struct to `aspen-fuse`: `DashMap<u32, ReadSession>` mapping PID → session with (read_set: Vec<(String, Hash)>, parent_pid: Option<u32>, created_at: Instant)
- [x] 3.2 Add `tracking_enabled: AtomicBool` field to `AspenFs` — zero overhead when disabled
- [x] 3.3 Hook into `AspenFs::read()` in `fs/operations.rs`: when tracking enabled, record (path, content_hash) in the PID's read session. Use iroh-blob hash when available, BLAKE3 on-the-fly otherwise
- [x] 3.4 Hook into `AspenFs::open()`: create read session for new PIDs that are children of tracked sessions (check ppid via `/proc/{pid}/stat`)
- [x] 3.5 Implement session finalization: `finalize(pid) -> ReadSet` that returns the sorted, deduplicated read set and removes the session
- [x] 3.6 Implement stale session cleanup: background task that removes sessions for PIDs no longer in `/proc`, bounded by session timeout (5 min)
- [x] 3.7 Enforce per-session resource bounds: disable tracking (don't crash) when a session exceeds MAX_INPUT_FILES
- [x] 3.8 Unit tests for ReadTracker: session create/finalize, deduplication, child inheritance, stale cleanup, resource bound enforcement

## 4. Cached Process Execution

- [x] 4.1 Implement `CachedExecutor` in `aspen-exec-cache`: wraps a process launch with cache-check-before / capture-after logic
- [x] 4.2 Implement cache lookup path: compute key from command+args+env+read_set → check ExecCacheIndex → return cached result or miss
- [x] 4.3 Implement output capture on miss: capture stdout, stderr, exit code; scan FUSE write set for output files; store all as iroh-blobs; create cache entry
- [x] 4.4 Implement output materialization on hit: write cached output files to FUSE from iroh-blobs, return cached stdout/stderr/exit_code
- [x] 4.5 Implement cache lookup timeout (100ms default): fall through to execution if lookup is slow
- [x] 4.6 Integration test: run a deterministic command twice, verify second run hits cache and skips execution

## 5. Process Tree Tracking

- [x] 5.1 Extend `ReadSession` with `child_cache_keys: Vec<CacheKey>` to record completed child process results
- [x] 5.2 Implement PID→parent mapping via `/proc/{pid}/stat` ppid field lookup
- [x] 5.3 Implement recursive cache check: parent cache hit requires all child cache keys to also be valid
- [x] 5.4 Integration test: simulated cargo build (parent spawns 3 children, change one input, verify only affected child re-executes)

## 6. LRU Eviction

- [x] 6.1 Add `last_accessed_ms` field to `CacheEntry`, update on cache hit
- [x] 6.2 Implement background eviction task: scan `_exec_cache:` prefix, remove expired entries, then LRU evict if total size exceeds MAX_CACHE_STORAGE_BYTES
- [x] 6.3 Unprotect evicted output blobs from iroh-blobs GC
- [x] 6.4 Unit test: verify eviction removes oldest entries first, respects storage bound

## 7. CI Integration

- [x] 7.1 Add `cached_execution: bool` field to CI job spec in `aspen-ci-core`
- [x] 7.2 Wire `ASPEN_CACHED_EXEC=1` env var check in AspenFs mount setup for CI VMs
- [x] 7.3 Enable read tracking on AspenFs VirtioFS mounts when cached execution is active in `aspen-ci-executor-vm`
- [x] 7.4 Wire `CachedExecutor` into shell executor's process launch path in `aspen-ci-executor-shell`
- [x] 7.5 Add cache hit/miss metrics to CI job results (hit count, miss count, time saved)

## 8. Branch Integration

- [x] 8.1 Ensure read tracking in `AspenFs::read()` records the branch-resolved content hash (dirty map value if present, parent value otherwise)
- [x] 8.2 Test: cached execution inside a `@branch` path uses branch-local file contents for cache key, not base store contents

## 9. Verus Verification

- [x] 9.1 Add `src/verified/cache_key.rs` with pure cache key computation function
- [x] 9.2 Add `verus/cache_key_spec.rs` with ensures: determinism (same inputs → same output), sorted input invariant, env hash included
- [x] 9.3 Run `nix run .#verify-verus` and fix any proof failures
