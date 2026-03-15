## 1. Golden Snapshot Infrastructure

- [ ] 1.1 Add snapshot storage paths to `CloudHypervisorWorkerConfig`: `snapshot_dir()` → `{state_dir}/snapshots/golden/`, `snapshot_memory_path()`, `snapshot_state_path()`, `snapshot_ticket_path()`
- [ ] 1.2 Implement `GoldenSnapshot` struct with `validate()` method: checks directory exists, memory file exists, ticket matches current ticket
- [ ] 1.3 Implement `GoldenSnapshot::create(vm: &ManagedCiVm)`: pause VM → `api.snapshot()` → write `ticket.txt` → resume VM
- [ ] 1.4 Implement `GoldenSnapshot::invalidate()`: delete snapshot directory, log reason
- [ ] 1.5 Wire golden snapshot creation into `VmPool::initialize()`: first VM cold-boots → reaches Idle → snapshot → resume → populate pool from restore
- [ ] 1.6 Unit tests for `GoldenSnapshot::validate()`: valid snapshot passes, missing memory fails, stale ticket fails

## 2. Snapshot Restore Path

- [ ] 2.1 Add `ManagedCiVm::restore_from_snapshot(snapshot: &GoldenSnapshot)` method: sets up fork-specific socket directory → starts virtiofsd + AspenFs daemons → calls `api.restore(source_url, prefault=false)` → transitions to Idle
- [ ] 2.2 Implement per-fork socket directory layout: `{state_dir}/cow/{fork_id}/` with socket paths matching golden VM's expected vhost-user paths
- [ ] 2.3 Implement per-fork AspenFs VirtioFS daemon startup with fork-specific KV prefix (`ci/workspaces/{fork_id}/`)
- [ ] 2.4 Implement per-fork virtiofsd startup for nix store at fork-specific socket paths
- [ ] 2.5 Wire `restore_from_snapshot` into `VmPool::acquire()`: check for valid golden snapshot → restore instead of cold-boot → fallback to cold-boot on restore failure
- [ ] 2.6 Wire `restore_from_snapshot` into `VmPool::maintain()`: replenish idle pool via restore instead of cold-boot
- [ ] 2.7 Integration test: create golden snapshot → restore from it → verify VM reaches Idle → verify it can process a simple command

## 3. Cold-Boot Fallback

- [ ] 3.1 Add `force_cold_boot` flag to `acquire()` and `ManagedCiVm`: bypasses snapshot restore, uses existing cold-boot path
- [ ] 3.2 Implement automatic fallback: if `restore_from_snapshot` fails, log error, mark snapshot for regeneration, cold-boot instead
- [ ] 3.3 Implement snapshot regeneration in `VmPool::maintain()`: if snapshot is marked invalid, next maintenance cycle cold-boots a VM and re-snapshots
- [ ] 3.4 Unit test: verify `acquire()` with invalid snapshot falls back to cold-boot and marks snapshot for regeneration

## 4. COW Memory

- [ ] 4.1 Configure Cloud Hypervisor golden snapshot with `--memory shared=on,backing-file={snapshot_dir}/memory` so the memory file can be mmap'd COW by restored VMs
- [ ] 4.2 Verify memory backing file is created as sparse (check disk usage vs apparent size after snapshot)
- [ ] 4.3 Configure `vm.restore` with `prefault: false` to enable demand-paging from backing file
- [ ] 4.4 Add memory resource tracking: estimate total dirty pages across restored VMs via `/proc/{ch_pid}/smaps` or Cloud Hypervisor API
- [ ] 4.5 Add `max_total_vm_memory_bytes` config bound: reject new `acquire()` calls when total estimated memory exceeds limit
- [ ] 4.6 Test: restore 4 VMs from 24GB snapshot, verify host RSS is well under 4×24GB

## 5. Fork Cleanup

- [ ] 5.1 Extend `ManagedCiVm::shutdown()` to clean up fork-specific resources: kill virtiofsd, shutdown AspenFs daemon, remove socket directory, delete COW overlay files
- [ ] 5.2 Extend `VmPool::destroy_vm()` to call fork cleanup before releasing semaphore permit
- [ ] 5.3 Ensure `should_destroy_after_job` works correctly with snapshot-restored VMs (destroy fork, don't destroy golden snapshot)
- [ ] 5.4 Test: restore → destroy → verify no leaked socket files, processes, or COW overlay files

## 6. Speculative Execution

- [ ] 6.1 Add `SpeculativeGroup` struct: holds Vec of (SharedVm, KvBranch) pairs, monitors completion, implements first-success-wins
- [ ] 6.2 Implement `VmPool::acquire_speculative(job_id, count)`: restore N VMs, each with its own KV branch, return `SpeculativeGroup`
- [ ] 6.3 Implement `SpeculativeGroup::wait_first_success()`: tokio::select across all fork completion channels, commit winner's branch, kill others, drop their branches
- [ ] 6.4 Implement `SpeculativeGroup::cleanup()`: destroy all forks, release all permits, remove all fork directories
- [ ] 6.5 Add `MAX_SPECULATIVE_FORKS` constant (default 8), cap requested count
- [ ] 6.6 Test: speculative group of 3 forks, simulate one success → verify winner commits, losers dropped, all resources cleaned

## 7. CI Job Spec Integration

- [ ] 7.1 Add `force_cold_boot: bool` field to CI job spec in `aspen-ci-core`
- [ ] 7.2 Add `speculative_count: Option<u32>` field to CI job spec in `aspen-ci-core`
- [ ] 7.3 Wire `force_cold_boot` into CloudHypervisorWorker's VM acquisition path
- [ ] 7.4 Wire `speculative_count` into CloudHypervisorWorker to call `acquire_speculative()` when count > 1
- [ ] 7.5 Add snapshot metrics to pool status: `is_snapshot_valid`, `snapshot_age_ms`, `restore_count`, `restore_avg_ms`, `cold_boot_fallback_count`

## 8. VirtioFS Snapshot Compatibility Testing

- [ ] 8.1 Test Cloud Hypervisor snapshot/restore with active VirtioFS connections on v49.0
- [ ] 8.2 Test that restored VM can read from nix store VirtioFS mount after restore
- [ ] 8.3 Test that restored VM can read/write workspace VirtioFS mount after restore
- [ ] 8.4 Test overlayfs (tmpfs rw-store) behavior after snapshot/restore — verify overlay state is preserved
- [ ] 8.5 Test aspen-node Iroh reconnection behavior after restore — verify worker re-registers within 500ms

## 9. End-to-End Integration

- [ ] 9.1 NixOS VM test: boot cluster → start CloudHypervisorWorker with snapshots enabled → verify golden snapshot created → submit CI job → verify job uses restored VM → verify job completes
- [ ] 9.2 Benchmark: measure cold-boot time vs snapshot-restore time, report in CI metrics
- [ ] 9.3 Stress test: restore 8 VMs simultaneously from same golden snapshot, run jobs, verify all complete correctly
