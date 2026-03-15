## MODIFIED Requirements

### Requirement: Executor Backends

The system SHALL support multiple executor backends for running CI jobs. The shell and Nix executors SHALL run Nix evaluation and build commands through a `NixBuildSupervisor` that manages them as supervised child processes with configurable timeouts. The VM executor SHALL support snapshot-based fast startup when a golden snapshot is available.

#### Scenario: Shell executor

- GIVEN a job configured with `executor: shell`
- WHEN the job runs
- THEN commands SHALL execute in a shell on the worker node

#### Scenario: Nix executor

- GIVEN a job configured with `executor: nix`
- WHEN the job runs
- THEN the job SHALL execute inside a Nix build sandbox with specified dependencies

#### Scenario: VM executor

- GIVEN a job configured with `executor: vm`
- WHEN the job runs
- THEN the job SHALL execute inside an isolated virtual machine
- AND the VM SHALL be destroyed after the job completes

#### Scenario: VM executor with snapshot restore

- GIVEN a job configured with `executor: vm`
- AND a golden snapshot is available
- WHEN the job runs
- THEN the VM SHALL be restored from the golden snapshot instead of cold-booted
- AND VM acquisition time SHALL be under 500ms

#### Scenario: Nix build supervised with timeout

- GIVEN a shell or Nix executor job running a `nix build` command
- WHEN the build exceeds `nix_build_timeout_secs` (default: 1800)
- THEN the `NixBuildSupervisor` SHALL kill the child process
- AND the job SHALL be marked as failed with a timeout error

## ADDED Requirements

### Requirement: Job spec snapshot options

The CI job spec SHALL include optional fields for controlling VM snapshot behavior: `force_cold_boot: bool` (bypass snapshot restore), and `speculative_count: u32` (number of parallel fork-and-race VMs).

#### Scenario: Force cold boot bypasses snapshot

- **WHEN** a CI job spec includes `force_cold_boot: true`
- **THEN** the VmPool SHALL cold-boot a fresh VM even if a golden snapshot exists

#### Scenario: Speculative count enables parallel execution

- **WHEN** a CI job spec includes `speculative_count: 3`
- **THEN** the VmPool SHALL restore 3 VMs from the golden snapshot
- **AND** all 3 SHALL run the same job
- **AND** the first success SHALL be committed
