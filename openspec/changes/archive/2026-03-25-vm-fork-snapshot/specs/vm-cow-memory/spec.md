## ADDED Requirements

### Requirement: Shared memory backing file

The golden snapshot's memory SHALL be stored as a file-backed shared memory region. Restored VMs SHALL map this file copy-on-write so that unmodified pages are shared across all forks via the kernel page cache.

#### Scenario: Multiple forks share base memory

- **WHEN** 4 VMs are restored from a 24GB golden snapshot
- **AND** each VM dirties 500MB of memory during job execution
- **THEN** total physical memory usage SHALL be approximately 24GB (shared base) + 4×500MB (dirty pages) = 26GB
- **AND** NOT 4×24GB = 96GB

#### Scenario: Restore with prefault disabled

- **WHEN** a VM is restored with `prefault: false`
- **THEN** memory pages SHALL be faulted on demand from the backing file
- **AND** initial restore SHALL complete without loading all 24GB into RAM

### Requirement: COW memory overlay per fork

Each restored VM SHALL use a private memory mapping on top of the shared backing file. Writes by the VM SHALL create private (dirty) pages that do not affect the shared backing file or other forks.

#### Scenario: Fork write isolation

- **WHEN** fork A writes to memory address 0x1000
- **AND** fork B reads from memory address 0x1000
- **THEN** fork B SHALL see the original snapshot value, not fork A's write

#### Scenario: Fork destroy reclaims dirty pages

- **WHEN** a forked VM is destroyed
- **THEN** its private dirty pages SHALL be freed
- **AND** the shared backing file SHALL remain unchanged

### Requirement: Memory backing file management

The memory backing file SHALL be stored as a sparse file to avoid consuming disk space for zero-filled pages. The file SHALL be created during golden snapshot creation and persist until the snapshot is invalidated.

#### Scenario: Sparse file does not consume full allocation

- **WHEN** a 24GB golden snapshot is created
- **AND** the guest only used 4GB of its memory
- **THEN** the memory backing file's disk usage SHALL be approximately 4GB
- **AND** the file's apparent size SHALL be 24GB

#### Scenario: Backing file persists across pool restarts

- **WHEN** the VmPool shuts down and restarts
- **AND** the golden snapshot is still valid
- **THEN** the memory backing file SHALL still be usable for restores

### Requirement: Memory resource bounds

Total memory consumed by restored VMs (shared base + all dirty pages) SHALL be bounded by a configurable limit. When the limit is reached, new fork requests SHALL fail with a capacity error rather than OOM-killing existing VMs.

#### Scenario: Memory limit prevents over-commitment

- **WHEN** total restored VM memory usage approaches the configured limit
- **AND** a new `acquire()` is called
- **THEN** the acquire SHALL return a capacity error
- **AND** no existing VM SHALL be affected

### Requirement: Memory-pressure-aware restore

The pool SHALL consult the existing `MemoryWatcher` (from `aspen-cluster`) before restoring VMs from snapshot. Memory pressure levels SHALL gate restore and pre-warming behavior.

#### Scenario: Critical pressure blocks new restores

- **WHEN** `MemoryWatcher` reports `MemoryPressureLevel::Critical` (usage ≥ 90%)
- **AND** `acquire()` is called
- **THEN** the acquire SHALL return a capacity error
- **AND** no new VM SHALL be created (neither restore nor cold-boot)
- **AND** existing running VMs SHALL NOT be affected

#### Scenario: Warning pressure stops pre-warming

- **WHEN** `MemoryWatcher` reports `MemoryPressureLevel::Warning` (usage ≥ 80%)
- **AND** `maintain()` runs to replenish the idle pool
- **THEN** `maintain()` SHALL skip creating new VMs
- **AND** existing idle VMs SHALL remain in the pool

#### Scenario: Normal pressure allows full operation

- **WHEN** `MemoryWatcher` reports `MemoryPressureLevel::Normal` (usage < 80%)
- **THEN** `acquire()` and `maintain()` SHALL operate normally
- **AND** restores and pre-warming SHALL proceed as configured
