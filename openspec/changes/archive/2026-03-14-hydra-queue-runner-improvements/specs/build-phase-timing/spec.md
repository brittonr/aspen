## ADDED Requirements

### Requirement: Nix build jobs report per-phase timing

The NixBuildWorker SHALL measure and report the duration of each build phase: importing requisites, building the derivation, and uploading outputs. Phase durations SHALL be stored as millisecond values in the job result metadata.

#### Scenario: Successful build reports all phases

- **WHEN** a nix build job completes successfully
- **THEN** `JobOutput.metadata` SHALL contain keys `nix_import_time_ms`, `nix_build_time_ms`, and `nix_upload_time_ms`
- **AND** each value SHALL be a string representation of the duration in milliseconds

#### Scenario: Build fails during build phase

- **WHEN** a nix build job fails during the build phase
- **THEN** `JobOutput.metadata` SHALL contain `nix_import_time_ms` for the completed import phase
- **AND** `nix_build_time_ms` SHALL reflect the time spent before failure
- **AND** `nix_upload_time_ms` SHALL be `"0"`

#### Scenario: Build skips upload

- **WHEN** a nix build job completes but `publish_to_cache` is false
- **THEN** `nix_upload_time_ms` SHALL be `"0"`
- **AND** `nix_import_time_ms` and `nix_build_time_ms` SHALL still be reported

### Requirement: Phase timings aggregate per worker

The system SHALL aggregate phase timing statistics per worker for operational monitoring. Aggregates SHALL include total time spent in each phase across all builds on that worker.

#### Scenario: Query worker timing statistics

- **WHEN** an operator queries worker statistics
- **THEN** the response SHALL include `total_import_time_ms`, `total_build_time_ms`, and `total_upload_time_ms` summed across all completed nix builds on that worker

#### Scenario: Phase timing computation is deterministic

- **WHEN** phase timing aggregation is computed
- **THEN** the summation logic SHALL be implemented as a pure function using saturating arithmetic
- **AND** the function SHALL reside in a verified module
