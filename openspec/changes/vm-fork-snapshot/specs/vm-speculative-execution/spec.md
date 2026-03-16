## ADDED Requirements

### Requirement: Speculative fork group

The VmPool SHALL support acquiring multiple VMs from the same golden snapshot as a speculative group. Each fork in the group SHALL have independent workspace state (KV branch) and independent memory (COW overlay).

#### Scenario: Acquire speculative group

- **WHEN** `acquire_speculative(job_id, count=3)` is called
- **AND** a valid golden snapshot exists
- **THEN** 3 VMs SHALL be restored from the snapshot
- **AND** each SHALL have its own KV branch for workspace isolation
- **AND** a `SpeculativeGroup` handle SHALL be returned

#### Scenario: Speculative count limited by pool capacity

- **WHEN** `acquire_speculative(job_id, count=5)` is called
- **AND** only 2 semaphore permits are available
- **THEN** only 2 VMs SHALL be created
- **AND** the `SpeculativeGroup` SHALL contain 2 forks, not 5

### Requirement: First-success-wins semantics

A speculative group SHALL monitor all forks. When the first fork completes successfully, its KV branch SHALL be committed and all other forks SHALL be killed and their branches dropped.

#### Scenario: First success commits and kills others

- **WHEN** a speculative group of 3 forks is running
- **AND** fork B completes with exit code 0
- **THEN** fork B's KV branch SHALL be committed
- **AND** forks A and C SHALL be killed
- **AND** forks A and C's KV branches SHALL be dropped without committing
- **AND** the `SpeculativeGroup` SHALL return fork B's result

#### Scenario: All forks fail

- **WHEN** a speculative group of 3 forks is running
- **AND** all 3 forks complete with non-zero exit codes
- **THEN** all KV branches SHALL be dropped
- **AND** the `SpeculativeGroup` SHALL return the first failure's error

### Requirement: Speculative group resource cleanup

When a speculative group completes (success or all-fail), all fork resources SHALL be cleaned up: VMs destroyed, virtiofsd processes killed, COW overlays deleted, semaphore permits released.

#### Scenario: Cleanup after speculative success

- **WHEN** a speculative group completes with one success
- **THEN** all 3 VMs (including the winner) SHALL be destroyed
- **AND** all fork-specific socket files SHALL be removed
- **AND** all semaphore permits SHALL be released

### Requirement: Speculative execution opt-in

Speculative execution SHALL be opt-in via job spec. The default speculative count SHALL be 1 (no speculation). Maximum speculative count SHALL be bounded by `MAX_SPECULATIVE_FORKS` (default 8).

#### Scenario: Default is no speculation

- **WHEN** a CI job does not specify `speculative_count`
- **THEN** the job SHALL acquire a single VM
- **AND** no speculative group SHALL be created

#### Scenario: Speculative count capped at maximum

- **WHEN** a CI job specifies `speculative_count: 20`
- **AND** `MAX_SPECULATIVE_FORKS` is 8
- **THEN** the speculative count SHALL be capped to 8

### Requirement: Adaptive speculative fork count

The effective fork count SHALL be adjusted based on host memory pressure. Under pressure, the pool reduces speculation to conserve resources for running VMs.

#### Scenario: Critical pressure disables speculation

- **WHEN** a CI job specifies `speculative_count: 4`
- **AND** `MemoryWatcher` reports `MemoryPressureLevel::Critical`
- **THEN** the effective fork count SHALL be 1 (no speculation)
- **AND** the job SHALL execute on a single VM

#### Scenario: Warning pressure halves speculation

- **WHEN** a CI job specifies `speculative_count: 4`
- **AND** `MemoryWatcher` reports `MemoryPressureLevel::Warning`
- **THEN** the effective fork count SHALL be 2 (halved, minimum 1)

#### Scenario: Normal pressure uses requested count

- **WHEN** a CI job specifies `speculative_count: 4`
- **AND** `MemoryWatcher` reports `MemoryPressureLevel::Normal`
- **THEN** the effective fork count SHALL be 4 (as requested, subject to `MAX_SPECULATIVE_FORKS` cap)
