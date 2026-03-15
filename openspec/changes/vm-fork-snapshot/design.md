## Context

The `aspen-ci-executor-vm` crate manages a pool of Cloud Hypervisor microVMs. Today's lifecycle:

1. `VmPool::acquire()` pops an idle VM or cold-boots a new one
2. Cold boot: spawn virtiofsd → spawn cloud-hypervisor → wait for API socket → boot → wait for Running state → wait for aspen-node cluster join
3. Each VM runs `aspen-node --worker-only`, joins the cluster via Iroh, processes jobs
4. After job: VM is destroyed (`should_destroy_after_job = true`) to prevent overlay state leakage
5. Next job: repeat from step 1

Cold boot takes 5-15 seconds depending on hardware. The config already has `enable_snapshots: bool` and `snapshot_path: Option<PathBuf>`. The API client already wraps `vm.snapshot` and `vm.restore`. None of this is wired together.

Cloud Hypervisor snapshot/restore serializes VM state (CPU registers, device state, memory) to a directory. Restore loads it back. With `shared=on` memory (already configured) and a `backing-file`, memory can be mapped copy-on-write so restored VMs share physical pages with the snapshot.

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

## Risks / Trade-offs

**[VirtioFS socket reconnection is fragile]** → The vhost-user protocol is stateful. Snapshot/restore of VMs with active vhost-user connections is not well-tested upstream. Mitigation: test extensively with Cloud Hypervisor's test suite. Fallback: if socket reconnection fails, tear down the fork and cold-boot a fresh VM. The pool already handles this (dead VM eviction).

**[Memory backing file consumes disk space]** → A 24GB golden snapshot memory file uses 24GB on disk (even though most pages are zero). Mitigation: use a sparse file (most pages are zero-filled and don't consume disk blocks). Or use `fallocate` with `FALLOC_FL_PUNCH_HOLE` to reclaim zero pages after snapshot creation.

**[Snapshot memory becomes stale as host memory pressure increases]** → The kernel may evict snapshot pages from the page cache under memory pressure, causing page faults on restore. Mitigation: `mlock` the snapshot memory file (or its hot pages) if memory is available. Accept higher restore latency under pressure — it's still faster than cold boot.

**[Golden snapshot includes Iroh connection state that may be stale]** → Restored VMs inherit TCP/QUIC connections from the snapshot that are no longer valid. Mitigation: aspen-node's Iroh stack handles reconnection automatically. The worker will re-register after restore. Budget 100-500ms for reconnection.

**[Speculative execution wastes resources on unsuccessful forks]** → N forks use N× CPU. Mitigation: speculative execution is opt-in, default count is 1 (no speculation). Resource limits via existing `MAX_CI_VMS_PER_NODE` cap.

## Open Questions

- Should the golden snapshot be per-node or shared across nodes? Per-node is simpler (no snapshot distribution). Shared via iroh-blobs would allow heterogeneous clusters to share golden images.
- How does snapshot/restore interact with the tmpfs rw-store overlay inside the VM? The overlay state is in the snapshot's memory. Restoring the snapshot restores the overlay. Need to verify overlayfs doesn't break on resume.
- What's the minimum Cloud Hypervisor version that supports reliable snapshot/restore with VirtioFS? Need to test with v49.0 (currently vendored).
