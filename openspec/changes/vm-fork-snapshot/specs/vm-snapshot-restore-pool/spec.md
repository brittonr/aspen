## ADDED Requirements

### Requirement: Snapshot-based VM acquisition

When a golden snapshot is available, `VmPool::acquire()` SHALL restore a new VM from the snapshot instead of cold-booting. The restored VM SHALL reach the Idle state without repeating kernel boot, systemd init, or aspen-node cluster join.

#### Scenario: Acquire restores from snapshot

- **WHEN** `acquire(job_id)` is called
- **AND** a valid golden snapshot exists
- **THEN** the pool SHALL create a new VM by calling `vm.restore` with the golden snapshot path
- **AND** the VM SHALL transition directly to `VmState::Idle`
- **AND** the total time from `acquire()` call to VM assignment SHALL be under 500ms on warm hardware

#### Scenario: Fallback to cold boot when no snapshot exists

- **WHEN** `acquire(job_id)` is called
- **AND** no golden snapshot exists (disabled, invalid, or not yet created)
- **THEN** the pool SHALL cold-boot a VM using the existing boot path
- **AND** the VM SHALL go through the full Creating → Booting → Idle lifecycle

#### Scenario: Fallback to cold boot when restore fails

- **WHEN** `acquire(job_id)` is called
- **AND** `vm.restore` returns an error
- **THEN** the pool SHALL log the failure
- **AND** fall back to cold-booting a fresh VM
- **AND** mark the golden snapshot for regeneration

### Requirement: Per-fork VirtioFS socket setup

Each restored VM SHALL have its own VirtioFS daemon instances. Before calling `vm.restore`, the pool SHALL start virtiofsd (nix store) and AspenFs VirtioFS daemon (workspace) at socket paths matching the golden snapshot's expected socket layout.

#### Scenario: Fork gets independent virtiofsd

- **WHEN** a VM is restored from the golden snapshot
- **THEN** a new virtiofsd process SHALL be started for the nix store share
- **AND** a new AspenFs VirtioFS daemon SHALL be started for the workspace share
- **AND** socket paths SHALL match the paths the golden VM's vhost-user connections expect
- **AND** the workspace daemon SHALL use a fork-specific KV prefix for isolation

#### Scenario: VirtioFS sockets ready before restore

- **WHEN** the pool prepares to restore a VM from snapshot
- **THEN** all VirtioFS sockets SHALL be created and ready to accept connections
- **AND** only then SHALL `vm.restore` be called

### Requirement: Pool pre-warming via snapshot restore

When `pool.maintain()` detects the idle pool is below `pool_size`, it SHALL restore VMs from the golden snapshot (if available) instead of cold-booting.

#### Scenario: Maintenance restores from snapshot

- **WHEN** the idle pool has 0 VMs
- **AND** `pool_size` is 2
- **AND** a valid golden snapshot exists
- **THEN** `maintain()` SHALL restore 2 VMs from the snapshot
- **AND** both SHALL be placed in the idle pool

### Requirement: Restored VM cleanup

When a restored VM is destroyed (after job completion or on error), the pool SHALL clean up the fork's VirtioFS daemons, socket files, and any COW memory overlay files.

#### Scenario: Fork cleanup on destroy

- **WHEN** a restored VM is destroyed
- **THEN** the VM's virtiofsd process SHALL be killed
- **AND** the VM's AspenFs VirtioFS daemon SHALL be shut down
- **AND** the fork's socket files SHALL be removed
- **AND** the fork's COW memory overlay (if any) SHALL be deleted
- **AND** the semaphore permit SHALL be released
