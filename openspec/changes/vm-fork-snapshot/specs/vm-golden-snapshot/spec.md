## ADDED Requirements

### Requirement: Automatic golden snapshot creation

When `enable_snapshots` is true in `CloudHypervisorWorkerConfig`, the VmPool SHALL cold-boot the first VM, wait for it to reach the Idle state (aspen-node joined cluster), pause it, create a snapshot at `snapshot_path`, then resume it as the first pool member.

#### Scenario: First boot creates golden snapshot

- **WHEN** the VmPool initializes with `enable_snapshots: true`
- **AND** no golden snapshot exists at `snapshot_path`
- **THEN** the pool SHALL cold-boot one VM
- **AND** wait for it to reach `VmState::Idle`
- **AND** pause the VM via `vm.pause` API
- **AND** create a snapshot via `vm.snapshot` API to `snapshot_path`
- **AND** resume the VM via `vm.resume` API
- **AND** the VM SHALL be placed in the idle pool

#### Scenario: Golden snapshot already exists

- **WHEN** the VmPool initializes with `enable_snapshots: true`
- **AND** a valid golden snapshot exists at `snapshot_path`
- **THEN** the pool SHALL skip cold-boot
- **AND** restore VMs from the existing snapshot to fill the pool

### Requirement: Golden snapshot validation

On pool initialization, the golden snapshot SHALL be validated by checking that the snapshot directory exists, the memory backing file exists, and the embedded cluster ticket matches the current cluster ticket.

#### Scenario: Valid snapshot passes validation

- **WHEN** the snapshot directory exists
- **AND** the memory backing file exists
- **AND** the snapshot's cluster ticket matches the current ticket
- **THEN** validation SHALL succeed
- **AND** the pool SHALL use the snapshot for subsequent VM creation

#### Scenario: Stale ticket invalidates snapshot

- **WHEN** the snapshot directory exists
- **AND** the snapshot's embedded cluster ticket differs from the current ticket
- **THEN** validation SHALL fail
- **AND** the snapshot directory SHALL be deleted
- **AND** the pool SHALL cold-boot and create a new golden snapshot

#### Scenario: Missing memory file invalidates snapshot

- **WHEN** the snapshot directory exists
- **AND** the memory backing file is missing or corrupted
- **THEN** validation SHALL fail
- **AND** the pool SHALL regenerate the snapshot via cold-boot

### Requirement: Snapshot storage structure

The golden snapshot SHALL be stored at `{state_dir}/snapshots/golden/` containing: `state.json` (Cloud Hypervisor VM state), `memory` (guest memory backing file), and `ticket.txt` (cluster ticket at snapshot time).

#### Scenario: Snapshot directory layout

- **WHEN** a golden snapshot is created
- **THEN** the directory `{state_dir}/snapshots/golden/` SHALL contain:
  - `state.json` — Cloud Hypervisor serialized VM state
  - `memory` — guest RAM backing file
  - `ticket.txt` — the cluster ticket embedded at snapshot time

### Requirement: Snapshot regeneration on failure

If a restored VM fails to reach Idle state or fails the post-restore VirtioFS health probe, the failure SHALL be counted. After `max_restore_failures` (default 3) consecutive failures, the golden snapshot SHALL be invalidated and regenerated.

#### Scenario: Single restore failure is retried

- **WHEN** a VM is restored from the golden snapshot
- **AND** the restore fails (VM unreachable, VirtioFS probe fails, or boot timeout)
- **AND** the consecutive failure count is below `max_restore_failures`
- **THEN** the failure counter SHALL be incremented
- **AND** the pool SHALL fall back to cold-boot for this acquisition
- **AND** the golden snapshot SHALL NOT be deleted yet

#### Scenario: Consecutive failures trigger regeneration

- **WHEN** `max_restore_failures` consecutive restores fail
- **THEN** the golden snapshot SHALL be deleted
- **AND** the failure counter SHALL be reset
- **AND** the next `pool.maintain()` cycle SHALL create a new golden snapshot via cold-boot

#### Scenario: Successful restore resets failure counter

- **WHEN** a VM is restored from the golden snapshot
- **AND** the restore succeeds (VM reaches Idle, VirtioFS probe passes)
- **THEN** the consecutive failure counter SHALL be reset to zero
