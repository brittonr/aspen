## ADDED Requirements

### Requirement: Nix builds check failure cache before dispatch

The CI pipeline executor SHALL check the derivation failure cache before dispatching a nix build job. Cached failures SHALL short-circuit the build and propagate failure to dependents.

#### Scenario: Cached failure skips build

- **WHEN** a CI pipeline stage contains a nix build job
- **AND** the derivation's output paths are found in `_ci:failed-paths:`
- **THEN** the job SHALL be marked `CachedFailure` without worker dispatch
- **AND** downstream pipeline stages SHALL be skipped per existing failure propagation rules

#### Scenario: Pipeline retry clears failure cache

- **WHEN** an operator retries a failed CI pipeline
- **THEN** failure cache entries for all jobs in the pipeline SHALL be cleared before re-execution

### Requirement: Nix build results include phase timings

CI pipeline run records SHALL include per-job phase timing data from nix builds, enabling operators to identify bottlenecks across pipeline history.

#### Scenario: Pipeline run record includes timings

- **WHEN** a CI pipeline containing nix build jobs completes
- **THEN** each nix build job's record in the run data SHALL include `nix_import_time_ms`, `nix_build_time_ms`, and `nix_upload_time_ms`
