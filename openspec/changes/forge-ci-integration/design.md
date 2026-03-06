## Context

Aspen has three self-hosting pillars built independently:

1. **Forge** (~9.4K LOC): Stores git objects in iroh-blobs, refs in Raft KV, supports `git push aspen://` via `git-remote-aspen`. Gossip broadcasts `Announcement::RefUpdate` on push.
2. **CI** (~4.9K LOC): `TriggerService` subscribes to gossip, `PipelineOrchestrator` converts Nickel configs to jobs, three executor backends (shell, nix, vm).
3. **Nix Cache** (~3.8K LOC): snix backend stores NAR/PathInfo in KV+blobs, cache gateway serves HTTP for `nix-store --substituters`.

The `TriggerService` already has the right architecture — it accepts `ConfigFetcher` and `PipelineStarter` traits. The gap is concrete implementations that wire these to ForgeNode (for reading `.aspen/ci.ncl` from git objects) and PipelineOrchestrator (for starting runs).

## Goals / Non-Goals

**Goals:**

- `git push aspen://` on a watched repo triggers CI pipeline automatically
- CI Nix builds publish output store paths to the cluster's distributed binary cache
- Pipeline status is queryable by repo + ref via CLI
- Node startup automatically wires Forge→CI when both features are enabled
- End-to-end NixOS VM test proving the full path works

**Non-Goals:**

- PR/merge-request workflow (future — needs Forge review primitives first)
- GitHub webhook ingestion (future — Forge bridge handles external sync separately)
- Build log streaming via pub/sub (can use existing ephemeral pub/sub, not scoped here)
- Multi-cluster CI federation (future — single cluster first)
- CI dashboard in TUI (future — CLI status is sufficient for self-hosting)

## Decisions

### 1. ConfigFetcher reads git objects via ForgeNode

**Decision**: Implement `ForgeConfigFetcher` that calls `ForgeNode::read_file_at_commit(repo_id, commit_hash, path)` to read `.aspen/ci.ncl`.

**Rationale**: ForgeNode already stores git objects (trees, blobs) in iroh-blobs with SHA-1 indexing. Reading a file at a commit requires walking commit→tree→blob, which ForgeNode's `git` module already supports. Adding a `read_file_at_commit` convenience method is ~30 lines.

**Alternative considered**: Checking out the repo to disk and reading the file. Rejected — unnecessary I/O, doesn't work on nodes without git installed, and breaks the "stateless layer" principle.

### 2. PipelineStarter drives PipelineOrchestrator directly

**Decision**: Implement `OrchestratorPipelineStarter` that holds `Arc<PipelineOrchestrator>` and calls `orchestrator.start_run(event)`.

**Rationale**: The orchestrator already handles run creation, job submission, and status tracking. The starter is a thin adapter (~20 lines). No need for an intermediate queue — the trigger service already has a bounded channel (`MAX_PENDING_TRIGGERS = 100`).

**Alternative considered**: RPC-based trigger (CI handler receives `CiTriggerPipeline` RPC). Rejected for intra-node trigger because it adds network round-trip and auth complexity. The existing `CiTriggerPipeline` RPC remains available for external/CLI triggers.

### 3. Cache publish uses snix `PathInfoService::put()` + `BlobService::put()`

**Decision**: After `NixBuildWorker` collects artifacts, upload NARs to `BlobService` and register `PathInfo` entries in `PathInfoService`. This makes store paths available via the cache gateway's HTTP substituter interface.

**Rationale**: The snix backend (`RaftBlobService`, `RaftDirectoryService`, `RaftPathInfoService`) already implements the tvix/nix-compat store traits. The cache gateway already translates HTTP `GET /nix-cache-info` and `GET /<hash>.narinfo` into store queries. Publishing is just the write side of the same interface.

**Alternative considered**: Writing NARs directly to KV with the `_nix:nar:` prefix. Rejected — bypasses the snix abstraction layer and would need duplicate NAR parsing logic.

### 4. Hook event as secondary trigger path (not primary)

**Decision**: Add `HookEventType::ForgePushCompleted` variant and emit it from the Forge push handler. The primary trigger path remains gossip `Announcement::RefUpdate`. The hook event provides integration with the generic hook system (subscriptions, WASM plugin notifications).

**Rationale**: Gossip is the right primary path — it's already wired into `CiTriggerHandler` via `AnnouncementCallback`, works across nodes, and doesn't require the hook system to be enabled. The hook event is additive for users who want custom push notifications.

### 5. Feature gating with `ci` + `forge` compound detection

**Decision**: No new `ci-forge` feature flag. Instead, use `#[cfg(all(feature = "ci", feature = "forge"))]` in the node startup code to wire the integration. The `ForgeConfigFetcher` lives in `aspen-ci` behind `#[cfg(feature = "forge")]`.

**Rationale**: Compound features create combinatorial complexity. The integration code is <100 lines in the node binary — a simple cfg gate is sufficient. Both features are already in `full`.

### 6. Pipeline status indexed by repo+ref in KV

**Decision**: Pipeline runs are already stored at `_ci:runs:{run_id}` and indexed at `_ci:runs:by-repo:{repo_hex}:{timestamp}:{run_id}`. Add a latest-ref index at `_ci:ref-status:{repo_hex}:{ref_name}` pointing to the latest run_id for that ref.

**Rationale**: Enables `aspen-cli ci status <repo> [ref]` to quickly find the latest pipeline run for a ref without scanning all runs. The ref-status key is updated atomically when a run starts and when it completes.

## Risks / Trade-offs

**[Risk] ConfigFetcher fails on large repos** → Mitigation: `read_file_at_commit` reads only the targeted blob, not the whole tree. Bounded by `MAX_VALUE_SIZE` (1MB) for the config file.

**[Risk] Cache publish adds latency to CI builds** → Mitigation: Publish is async — NAR upload happens after the build job is marked successful. Build status reflects build result, not publish result. Publish failures are logged and retried, not propagated.

**[Risk] Gossip trigger on follower nodes** → Mitigation: `TriggerService` runs on all nodes but only the leader can write pipeline state to KV. Non-leader trigger attempts get `NOT_LEADER` error from orchestrator and are silently dropped (gossip will also reach the leader).

**[Risk] Circular dependency: CI builds Aspen which pushes to Forge which triggers CI** → Mitigation: Pipeline configs specify trigger refs (`refs/heads/main`). CI pushes to artifact refs (e.g., `refs/ci/run-123/artifacts`) which don't match trigger patterns.

**[Trade-off] In-process wiring vs RPC-based** → We chose in-process for performance and simplicity. Trade-off: CI and Forge must run in the same node process. Acceptable because self-hosting runs all features on every node.
