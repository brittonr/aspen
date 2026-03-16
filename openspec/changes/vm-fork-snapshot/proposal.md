## Why

Every CI job in `aspen-ci-executor-vm` cold-boots a fresh Cloud Hypervisor VM: kernel load, initrd unpack, systemd startup, aspen-node launch, Iroh cluster join. Even with pre-warmed VMs in the pool, each job gets a fresh VM (because `should_destroy_after_job` defaults to true to prevent overlay state leakage). This takes seconds per job.

Cloud Hypervisor already supports `vm.snapshot` and `vm.restore` — the `VmApiClient` has `snapshot()` and `restore()` methods, the config has `enable_snapshots` and `snapshot_path` fields — but none of this is wired into the pool's acquire path. The snapshot/restore APIs exist but are never called.

ix.dev forks running VMs in 26ms with full state (memory + disk + processes). The key insight: boot once, snapshot at the "idle and ready" point, then restore from snapshot for every job. COW memory sharing means forked VMs don't duplicate the parent's RAM.

## What Changes

- Create a golden snapshot after the first VM boots and reaches the Idle state (aspen-node joined cluster, ready for work)
- Wire `vm.restore` into the pool's `acquire()` path so new VMs start from the golden snapshot instead of cold-booting
- Add copy-on-write memory backing via shared memory files so restored VMs share base pages with the snapshot and only allocate on write
- Add COW disk overlay so each restored VM gets an independent filesystem view without copying the base image
- Extend the `VmPool` with a snapshot-aware lifecycle: boot golden VM → snapshot → restore forks → destroy forks after jobs
- Add a speculative execution mode: fork multiple VMs from the same snapshot, run different strategies in parallel, commit the winner's results

## Capabilities

### New Capabilities

- `vm-golden-snapshot`: Automated creation and management of golden VM snapshots at the "idle and cluster-joined" state, with consecutive-failure-based auto-invalidation
- `vm-snapshot-restore-pool`: Snapshot-based VM acquisition in VmPool — restore from golden snapshot instead of cold-booting, with post-restore VirtioFS health probe to verify host-side daemon reconnection
- `vm-cow-memory`: Copy-on-write memory backing for restored VMs using shared memory files and dirty page tracking, with memory-pressure-aware gating via existing `MemoryWatcher`
- `vm-speculative-execution`: Fork multiple VMs from the same snapshot to run parallel strategies, commit the first success, with adaptive fork count based on memory pressure

### Modified Capabilities

- `ci`: CI job spec gains snapshot-related options (force cold boot, speculative execution count)

## Impact

- `crates/aspen-ci-executor-vm/src/pool.rs` — snapshot-aware acquire/release, golden snapshot lifecycle
- `crates/aspen-ci-executor-vm/src/vm/lifecycle.rs` — restore-from-snapshot path alongside cold-boot path
- `crates/aspen-ci-executor-vm/src/vm/provisioning.rs` — golden snapshot creation after first successful boot
- `crates/aspen-ci-executor-vm/src/config.rs` — snapshot storage paths, COW memory config, speculative execution settings
- `crates/aspen-ci-executor-vm/src/api_client.rs` — already has `snapshot()`/`restore()`, may need `prefault` tuning
- `crates/aspen-ci-core/` — job spec additions for snapshot and speculative execution options
- Cloud Hypervisor dependency: requires `--memory shared=on` (already configured) and `--memory backing-file` for COW
- Host filesystem: snapshot storage at `{state_dir}/snapshots/`, COW overlay files at `{state_dir}/cow/`
