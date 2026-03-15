## Context

Aspen CI VMs run build commands (`nix build`, `cargo build`) inside VirtioFS-mounted filesystems backed by AspenFs. Every file operation goes through the FUSE layer, which already has per-inode caching, prefetch tracking, and write buffering. The FUSE `read()` handler sees every byte a process reads, and the inode table maps every path to a KV key. iroh-blobs stores content by BLAKE3 hash.

Today, the FUSE layer discards read-access information after serving the request. Build commands re-execute from scratch even when inputs haven't changed. Nix caches whole derivation outputs via the binary cache (`aspen-cache` + `aspen-nix-cache-gateway`), but individual processes within a build (each `rustc`, `gcc`, linker invocation) are never cached.

ix.dev implements this at the hypervisor level with syscall tracing. Aspen can do it at the FUSE level instead — simpler, no kernel patches, and the infrastructure (content-addressed blobs, distributed KV) already exists.

## Goals / Non-Goals

**Goals:**

- Cache individual process executions by content-hashing their file inputs
- Share cached results across all nodes and VMs via the cluster KV and iroh-blobs
- Work transparently with any build tool (cargo, nix, make, gcc) — no build system integration required
- Compose with existing KV branching so cached execution works inside branch-isolated CI jobs
- Sub-millisecond cache lookups for cache hits (KV point read + blob fetch)

**Non-Goals:**

- Replacing Nix's derivation-level caching (the binary cache continues to work alongside this)
- Caching non-deterministic processes (network access, randomness, wall-clock time)
- Guaranteeing cache correctness for processes with hidden state (environment variables not tracked, signals, etc.)
- Filesystem-level copy-on-write or VM forking (separate from this change)
- Remote execution or build distribution (this is local caching with cluster-wide sharing)

## Decisions

### 1. FUSE-level read tracking, not syscall tracing

**Decision:** Track file reads inside AspenFs's `read()` and `open()` handlers, not via seccomp-bpf or ptrace.

**Rationale:** AspenFs already intercepts every file operation. Adding a per-PID read set to the existing FUSE handler is straightforward. seccomp-bpf requires kernel support and adds complexity for nested process trees. The FUSE approach works with any kernel and composes naturally with existing VirtioFS integration.

**Trade-off:** FUSE-level tracking sees file reads at the VFS granularity, not individual `read()` syscalls. A process that opens a file but only reads one byte still records the whole file as an input. This is conservative (over-counts inputs, reducing cache hit rate slightly) but safe.

**Alternative considered:** eBPF tracing on `sys_enter_openat`. Lower overhead but requires BPF support, CAP_BPF in the VM, and a separate tracing daemon. The FUSE layer is simpler and already exists.

### 2. New crate `aspen-exec-cache` for cache logic, hooks in `aspen-fuse`

**Decision:** Create a new `aspen-exec-cache` crate for cache index, lookup, and population logic. Add thin read-tracking hooks into `aspen-fuse`'s `read()` path.

**Rationale:** Keeps AspenFs focused on filesystem operations. The cache logic (key computation, eviction, output replay) is independent of the FUSE protocol and can be tested without a mounted filesystem.

### 3. Cache key: BLAKE3 over command + sorted input hashes

**Decision:** Cache key = `BLAKE3(command_bytes || arg_bytes || env_hash || sorted_input_file_hashes)`.

Input file hashes come from iroh-blobs (already BLAKE3). Environment hash covers a curated set: `PATH`, `HOME`, `NIX_STORE`, `CC`, `CXX`, `CFLAGS`, `RUSTFLAGS`, plus any explicitly opted-in variables. Timestamps are ignored.

**Rationale:** Composing existing BLAKE3 hashes is cheap. Sorting ensures deterministic key computation regardless of file access order. Curating env vars avoids cache-busting from irrelevant environment changes (e.g., `TERM`, `SHELL`).

### 4. Cache storage: KV index + iroh-blobs for outputs

**Decision:** Cache entries stored as `_exec_cache:{cache_key_hex}` → JSON metadata in Raft KV. Output files stored as iroh-blobs, referenced by BLAKE3 hash in the metadata.

```
KV entry:
  key:   _exec_cache:a1b2c3d4...
  value: {
    "exit_code": 0,
    "stdout_hash": "...",
    "stderr_hash": "...",
    "outputs": [{"path": "target/debug/main", "hash": "..."}],
    "created_at_ms": 1710000000000,
    "ttl_ms": 86400000
  }
```

**Rationale:** Matches existing patterns (binary cache uses `_cache:narinfo:` prefix, plugins use `__plugin:` prefix). iroh-blobs handles deduplication and P2P distribution automatically.

### 5. Opt-in per job, not global

**Decision:** Cached execution is opt-in via a flag on the CI job spec or an environment variable (`ASPEN_CACHED_EXEC=1`). It is not enabled globally on all FUSE mounts.

**Rationale:** Not all processes are deterministic. Build tools are good candidates. Interactive sessions, tests with randomness, and network-dependent processes are not. Opt-in prevents false cache hits from non-deterministic processes.

### 6. Process tree tracking via PID inheritance

**Decision:** When a tracked process forks, child PIDs inherit the parent's tracking session. Each child gets its own read set. The parent's cache entry records child cache keys as dependencies.

**Rationale:** `cargo build` spawns dozens of `rustc` processes. Each child should be cacheable independently. When only one source file changes, only the affected `rustc` invocation and the final linker step miss the cache — everything else hits.

**Implementation:** The FUSE layer already receives the caller's PID in the `Context` struct on every operation. Map PID → session via a `DashMap`. When a new PID appears that is a child of a tracked PID (via `/proc/{pid}/stat` ppid field), create a child session.

### 7. TTL-based eviction with LRU fallback

**Decision:** Cache entries have a configurable TTL (default 24h). When cache storage exceeds a size bound, evict least-recently-used entries. No manual cache invalidation API in v1.

**Rationale:** Build outputs become stale as dependencies evolve. TTL prevents serving results from weeks-old inputs. LRU eviction keeps storage bounded. Manual invalidation adds API surface without clear benefit in v1 — changing any input file automatically produces a new cache key.

## Risks / Trade-offs

**[Non-deterministic processes produce wrong cached results]** → Opt-in flag limits exposure. Document which process types are safe to cache (build tools, compilers, formatters) and which are not (tests with randomness, network clients). A process that reads `/dev/urandom` or calls `gettimeofday` will produce different outputs on re-execution even with identical file inputs — the cache will serve stale results. Mitigation: don't enable for test runners.

**[FUSE PID tracking misses process relationships]** → PID reuse and zombie processes can cause incorrect session association. Mitigation: clean up sessions after a configurable timeout (default 5 minutes). Validate PID→session mappings against `/proc/{pid}/stat` ppid periodically.

**[Cache storage grows unbounded]** → TTL + LRU eviction + max storage bound (configurable, default 10 GB per node). The KV index entries are small (~200 bytes each). The output blobs are managed by iroh-blobs GC.

**[Cache key collisions]** → BLAKE3 collision probability is negligible (2^-128). Not a practical risk.

**[Overhead of read tracking on non-cached workloads]** → When tracking is disabled (default), zero overhead. When enabled, one `DashMap::insert` per `open()` call. DashMap lookup is ~50ns. For a `cargo build` with ~500 file opens, that's ~25μs total — negligible vs. the build time.

## Open Questions

- Should cache entries be scoped per-branch (branch-local cache) or global? Branch-local prevents cross-contamination but reduces hit rate. Global maximizes sharing but could serve results from a different branch's state.
- Should we cache stderr/stdout separately or together? Separate allows replaying just stdout, but adds complexity.
- What's the right granularity for env var tracking? Too few vars → false cache hits. Too many → cache-busting on irrelevant changes.
