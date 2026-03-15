## Why

Aspen CI rebuilds from scratch on every run. A `cargo build` that touches one file re-runs every `rustc` invocation. Nix caches at the derivation boundary (whole package outputs), but individual build steps within a derivation are never cached. The FUSE layer already intercepts every file I/O operation — it knows which files each process reads — but discards that information. By recording file reads and hashing their contents, Aspen can skip re-execution of any process whose inputs haven't changed, without requiring build system changes (no Bazel/Buck2 migration).

## What Changes

- Add per-process file read tracking to the AspenFs FUSE layer, recording which paths and blob hashes each process reads during execution
- Introduce a content-addressed execution cache that maps `BLAKE3(command + args + env + sorted input hashes)` to cached outputs (stdout, stderr, exit code, written files as blobs)
- Add a cache lookup path that checks for cached results before launching a process, replaying outputs on hit
- Add recursive process tree tracking so child processes (individual `rustc` calls within `cargo build`) each get their own cache entries
- Store cached execution results in the cluster KV (cache index) and iroh-blobs (output blobs), shared across all nodes and VMs

## Capabilities

### New Capabilities

- `execution-cache-index`: Cache index mapping input content hashes to output blob references, stored in Raft KV with TTL and eviction
- `fuse-read-tracking`: Per-process file read tracking in AspenFs that records which files and blob hashes each PID reads during execution
- `cached-process-execution`: Cache lookup before process launch, output replay on hit, output capture and cache population on miss

### Modified Capabilities

- `kv-branch-fuse`: AspenFs needs per-inode read tracking hooks integrated into the existing FUSE read path

## Impact

- `crates/aspen-fuse/` — read tracking hooks in `fs/operations.rs`, new `read_tracker.rs` module
- `crates/aspen-cache/` or new `crates/aspen-exec-cache/` — cache index and lookup logic
- `crates/aspen-ci-executor-vm/` — wire cached execution into VM job runner
- `crates/aspen-ci-executor-shell/` — wire cached execution into shell job runner
- Cluster KV schema: new `_exec_cache:` key prefix for cache entries
- iroh-blobs: output artifacts stored as content-addressed blobs (already supported)
