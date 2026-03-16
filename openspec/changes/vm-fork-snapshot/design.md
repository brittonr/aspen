## Context

The `aspen-ci-executor-vm` crate manages a pool of Cloud Hypervisor microVMs. Today's lifecycle:

1. `VmPool::acquire()` pops an idle VM or cold-boots a new one
2. Cold boot: spawn virtiofsd → spawn cloud-hypervisor → wait for API socket → boot → wait for Running state → wait for aspen-node cluster join
3. Each VM runs `aspen-node --worker-only`, joins the cluster via Iroh, processes jobs
4. After job: VM is destroyed (`should_destroy_after_job = true`) to prevent overlay state leakage
5. Next job: repeat from step 1

Cold boot takes 5-15 seconds depending on hardware. The config already has `enable_snapshots: bool` and `snapshot_path: Option<PathBuf>`. The API client already wraps `vm.snapshot` and `vm.restore`. None of this is wired together.

Cloud Hypervisor snapshot/restore serializes VM state (CPU registers, device state, memory) to a directory. Restore loads it back. With `shared=on` memory (already configured) and a `backing-file`, memory can be mapped copy-on-write so restored VMs share physical pages with the snapshot.

### Snapshot Boundary: What's Inside vs. Outside

The snapshot captures guest state only (CPU, memory, device state). The host-side processes that service the VM are NOT captured:

**Outside the snapshot (host-side, fresh per fork):**

- `virtiofsd` — host process serving `/nix/store` via vhost-user
- `AspenFs` VirtioFS daemon — host in-process daemon for workspace, backed by `FuseSyncClient` with its own Iroh connection to the cluster
- `workspace_client` (`FuseSyncClient`) — host-side Iroh/QUIC connection to the KV store
- Cloud Hypervisor process itself — manages the VM, created per fork
- TAP network device — host-side, created by Cloud Hypervisor on boot/restore

**Inside the snapshot (guest-side, restored from frozen state):**

- Guest kernel + systemd + all guest processes
- `aspen-node --worker-only` — including its in-process Iroh endpoint and QUIC connection state
- Guest virtio-fs driver state — vrings, shared memory mappings (must reconnect to new host-side daemons)
- Guest virtio-net driver state — resumed by Cloud Hypervisor's restored net backend
- tmpfs rw-store overlay — in guest memory, restored with the snapshot

**Consequence:** The heavy data path (nix store reads, workspace I/O, artifact uploads) flows through host-side VirtioFS daemons with fresh Iroh connections per fork. These work immediately after restore. Only the guest's control-plane Iroh connection (job queue, worker registration) is stale — and it self-heals via aspen-node's reconnection logic or systemd restart (`RestartSec = 1s`).

## Goals / Non-Goals

**Goals:**

- Reduce VM acquisition time from seconds to sub-200ms via snapshot restore
- Create a golden snapshot automatically after the first successful VM boot + cluster join
- Share base memory pages across restored VMs via COW (no per-VM 24GB allocation)
- Support multiple concurrent VMs restored from the same snapshot
- Enable speculative execution: fork N VMs, run different strategies, commit winner
- Compose with existing KV branching (each forked VM gets its own KV branch for workspace isolation)

**Non-Goals:**

- Sub-millisecond fork like ix.dev (that requires userfaultfd and a custom VMM; Cloud Hypervisor's restore path will be tens-to-hundreds of milliseconds, which is sufficient)
- Live migration of running VMs between hosts
- Incremental snapshots (full snapshot at the golden point is fine — it's done once)
- Modifying Cloud Hypervisor itself (use its existing API surface)
- Memory deduplication across VMs via KSM (kernel same-page merging is a separate optimization)

## Decisions

### 1. Golden snapshot at "Idle + cluster-joined" state

**Decision:** After the first VM cold-boots and reaches `VmState::Idle` (aspen-node has joined the cluster and registered as a worker), automatically pause it, snapshot it, then resume it as the first pool member. All subsequent VMs restore from this snapshot.

**Rationale:** The golden snapshot captures the most expensive work: kernel boot, systemd init, aspen-node startup, Iroh connection, worker registration. Restoring from this point skips all of it. The VM is in a clean state — no job has run, no workspace is dirty.

**Alternative considered:** Snapshot after NixOS boot but before aspen-node starts. Rejected because the Iroh connection and cluster join take ~1-2s and are deterministic — including them in the snapshot eliminates more latency.

**Trade-off:** The golden snapshot includes the cluster ticket and Iroh connection state. If the cluster ticket changes (cluster restart), the golden snapshot becomes invalid and must be regenerated. Mitigation: validate the snapshot's ticket on restore; if stale, delete and regenerate.

### 2. Cloud Hypervisor shared memory + file-backed restore for COW

**Decision:** Configure Cloud Hypervisor with `--memory shared=on,backing-file={state_dir}/snapshots/golden/memory`. On restore, map the backing file read-only and let the kernel COW individual pages on write.

**Rationale:** Cloud Hypervisor already requires `shared=on` for VirtioFS. Adding `backing-file` means snapshot memory is a regular file that can be mmap'd. Multiple restored VMs share the same physical pages via the page cache. Dirty pages are allocated per-VM. A 24GB VM that only dirties 500MB during a job uses 500MB of physical RAM, not 24GB.

**Alternative considered:** userfaultfd for demand-paging from the snapshot. Much faster (pages loaded on first access instead of prefaulted) but requires custom VMM modifications. Cloud Hypervisor's `prefault: false` on restore already avoids bulk page faulting — pages fault on demand from the backing file.

### 3. VirtioFS socket reuse via per-VM socket proxying

**Decision:** The golden snapshot includes virtiofsd socket connections. On restore, each forked VM needs its own virtiofsd sockets. Spawn new virtiofsd/AspenFs instances per fork and reconnect via Cloud Hypervisor's hotplug API, OR pre-create socket paths that the golden VM's vhost-user connections bind to, with per-fork socket directories.

**Rationale:** VirtioFS is the trickiest part of snapshot/restore. The vhost-user sockets in the snapshot point to the original golden VM's socket paths. Options:

- **(A) Re-create sockets at the same paths:** Not possible — multiple forks would collide.
- **(B) Per-fork socket directories:** Create `{state_dir}/cow/{fork_id}/` with symlinks or bind-mounts mapping to the golden snapshot's socket paths. Each fork gets its own virtiofsd instances at paths the restored VM expects.
- **(C) Hotplug after restore:** Remove the golden VM's fs devices, add new ones with fork-specific sockets. Requires a brief window where the VM has no filesystem.

**Decision: Option B** — per-fork socket directories. The golden snapshot's socket paths are within `{state_dir}/vms/{golden_id}/`. Each fork gets a directory that mirrors this layout with its own virtiofsd instances. Before restoring, start the fork's virtiofsd daemons at the expected socket paths.

### 4. Workspace isolation via KV branch per fork

**Decision:** Each forked VM gets its own KV branch (via `aspen-kv-branch`) for workspace isolation. The golden snapshot's workspace prefix is empty. On fork, the AspenFs VirtioFS daemon is started with a fork-specific KV prefix.

**Rationale:** The golden snapshot has a clean workspace. Each fork needs independent write space. KV branching provides COW semantics at the storage layer — matching the COW memory at the compute layer. On job success, the branch commits. On failure, the branch drops. No cleanup of orphaned keys.

### 5. Speculative execution via parallel forks

**Decision:** Add an optional `speculative_count: u32` to the CI job spec. When > 1, the pool forks N VMs from the golden snapshot, each running the same job with different strategies (e.g., different compiler flags, different parallelism levels). The first to succeed commits its results; others are killed.

**Rationale:** Fork-from-snapshot makes speculative execution cheap. Each fork shares base memory. The marginal cost of the Nth fork is just its dirty pages + CPU time. This is the foundation for search-based optimization: try many approaches, keep the best.

**Implementation:** `VmPool::acquire_speculative(job_id, count)` restores N VMs, each with its own KV branch. A `SpeculativeGroup` monitor watches all forks. On first success: commit that branch, kill others, drop their branches. On all-fail: report the best error.

### 6. Snapshot invalidation and regeneration

**Decision:** The golden snapshot is validated on pool initialization by checking: (1) snapshot directory exists, (2) memory backing file exists, (3) the embedded cluster ticket matches the current ticket. If any check fails, the snapshot is deleted and a cold boot + re-snapshot cycle runs.

**Rationale:** Cluster restarts change the ticket. Node upgrades change the kernel/initrd. The golden snapshot must match the current deployment. Validation is cheap (file existence + ticket comparison). Regeneration is the existing cold-boot path plus a snapshot step.

### 7. Post-restore VirtioFS health probe

**Decision:** After `vm.restore` succeeds, the pool SHALL verify the VirtioFS data path is functional by issuing a KV read through the fork's `workspace_client` (`FuseSyncClient`). If the probe fails, the fork is destroyed and the restore is considered failed.

**Rationale:** The snapshot captures the guest-side vhost-user device state, but the host-side VirtioFS daemons are fresh per fork. The vhost-user handshake between the restored guest driver and the new host daemon is the most fragile part of snapshot/restore. Since the `workspace_client` is a host-side object (not in the snapshot), the pool can probe the end-to-end path (host daemon → vhost-user → guest driver → vhost-user → host daemon → KV) without guest cooperation. A simple `scan_keys(prefix, 1)` call is sufficient.

**Consecutive failure tracking:** If `max_restore_failures` consecutive restores fail the VirtioFS probe, the golden snapshot is auto-invalidated. This uses a local counter (not distributed — snapshot validity is per-node). The verified layer encodes the invalidation decision:

```rust
// src/verified/snapshot.rs
pub fn should_invalidate_snapshot(
    restore_failures_consecutive: u32,
    max_restore_failures: u32,
) -> bool {
    restore_failures_consecutive >= max_restore_failures
}
```

### 8. Memory-pressure-aware restore via MemoryWatcher

**Decision:** Wire the existing `MemoryWatcher` (in `aspen-cluster`) into the snapshot restore path. At `Critical` pressure, `acquire()` rejects new restores with a capacity error. At `Warning` pressure, `maintain()` stops pre-warming additional VMs but existing forks continue.

**Rationale:** Restored VMs share base memory pages via the page cache. Under memory pressure, the kernel evicts those pages, causing page faults on the next access. Restoring additional VMs at that point makes things worse. The `MemoryWatcher` already polls `/proc/meminfo` and provides `MemoryPressureLevel::{Normal, Warning, Critical}` with `should_pause_jobs()` — we just need to check it in the restore path.

**Alternative considered:** Cluster-wide memory coordination via the distributed semaphore. Deferred — per-node `MemoryWatcher` is sufficient for Phase 1. Cluster-wide coordination can layer on top later via gossip-advertised memory stats and the existing `LeastLoadedStrategy` for job routing.

### 9. Golden snapshot distribution via iroh-blobs

**Decision:** Per-node snapshots for Phase 1. Optionally share golden snapshots across nodes via iroh-blobs in a future phase.

**Rationale:** After creating a golden snapshot, the memory backing file can be added to `iroh-blobs` (`blob_store.add_file(memory_path)`) to get a BLAKE3 hash. The hash is written to `{snapshot_dir}/blob-hash.txt`. Other nodes can fetch the snapshot from any peer that has it instead of cold-booting their own. Content-addressing means identical snapshots (same kernel, initrd, cluster config) are stored once across the cluster. The existing `BlobAnnouncement` gossip in `content_discovery.rs` announces availability.

This resolves the open question "per-node or shared?" — start per-node, add sharing when multi-node pools are needed.

### 10. Adaptive speculative fork count

**Decision:** When speculative execution is requested, the pool SHALL adjust the fork count based on memory pressure and optionally historical job variance. At `Warning` pressure, halve the requested count. At `Critical`, reduce to 1 (no speculation).

**Rationale:** Speculative forks share base memory but consume CPU and dirty page allocations. The `MemoryWatcher` provides pressure signals. Historical job profiles from `JobProfile.resource_samples` (in `aspen-jobs`) can inform whether speculation is worthwhile — low-variance builds gain nothing from parallel attempts.

```rust
// src/verified/snapshot.rs
pub fn compute_adaptive_fork_count(
    requested_count: u32,
    max_count: u32,
    pressure_level: u8, // 0=normal, 1=warning, 2=critical
) -> u32 {
    let count = match pressure_level {
        2 => 1,
        1 => (requested_count / 2).max(1),
        _ => requested_count,
    };
    count.min(max_count)
}
```

## Risks / Trade-offs

**[VirtioFS socket reconnection is fragile]** → The vhost-user protocol is stateful. The guest-side virtio-fs driver state is IN the snapshot, but the host-side daemons (virtiofsd, AspenFs) are NOT. The restored guest driver must handshake with fresh host daemons at the same socket paths. Mitigation: post-restore VirtioFS health probe via `workspace_client.scan_keys()` (Decision 7). Consecutive failure tracking auto-invalidates the snapshot after `max_restore_failures` (default 3). Fallback: cold-boot a fresh VM. The pool already handles dead VM eviction.

**[Memory backing file consumes disk space]** → A 24GB golden snapshot memory file uses 24GB on disk (even though most pages are zero). Mitigation: use a sparse file (most pages are zero-filled and don't consume disk blocks). Or use `fallocate` with `FALLOC_FL_PUNCH_HOLE` to reclaim zero pages after snapshot creation. Future: distribute via iroh-blobs (Decision 9) to avoid each node maintaining its own copy.

**[Snapshot memory becomes stale as host memory pressure increases]** → The kernel may evict snapshot pages from the page cache under pressure, causing page faults on restore. Mitigation: wire `MemoryWatcher` into the restore path (Decision 8). At `Critical` pressure, reject new restores. At `Warning`, stop pre-warming. Accept higher restore latency under moderate pressure — it's still faster than cold boot.

**[Guest Iroh connection state is stale after restore]** → The guest's `aspen-node --worker-only` has an Iroh endpoint in the snapshot whose QUIC connections are dead. However, this is only the control-plane connection (job queue, worker registration). The heavy data path (workspace I/O, nix store reads, artifact uploads) flows through host-side VirtioFS daemons with fresh Iroh connections per fork — those work immediately. The guest's stale Iroh self-heals: aspen-node handles reconnection automatically, and systemd restarts the process within 1 second if needed. Budget 1-2 seconds for re-registration, not 100-500ms.

**[Speculative execution wastes resources on unsuccessful forks]** → N forks use N× CPU. Mitigation: speculative execution is opt-in, default count is 1 (no speculation). Adaptive fork count (Decision 10) reduces speculation under memory pressure. Resource limits via existing `MAX_CI_VMS_PER_NODE` cap. Future: use historical `JobProfile` data to skip speculation for low-variance builds.

## Open Questions

- ~~Should the golden snapshot be per-node or shared across nodes?~~ **Resolved (Decision 9):** Per-node for Phase 1. iroh-blobs sharing available as a future optimization — content-addressing and gossip announcements are already in the codebase.
- How does snapshot/restore interact with the tmpfs rw-store overlay inside the VM? The overlay state is in guest memory (inside the snapshot). Restoring the snapshot restores the overlay. The overlay's upper layer (tmpfs) and lower layer (virtiofs) are both preserved in guest state. The lower layer reconnects to the new host-side virtiofsd via the vhost-user handshake. Need to verify overlayfs doesn't break when the lower layer reconnects to a different daemon.
- What's the minimum Cloud Hypervisor version that supports reliable snapshot/restore with VirtioFS? Need to test with v49.0 (currently vendored). The vhost-user reconnection after restore is the key capability to validate — the guest driver state is in the snapshot but the host daemon is new.
