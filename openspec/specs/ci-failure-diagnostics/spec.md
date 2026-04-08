# ci-failure-diagnostics Specification

## Purpose

TBD - created by archiving change fix-dogfood-ci-clippy. Update Purpose after archive.

## Requirements

### Requirement: CI job failures include build stderr

When a CI nix build job fails, the job result SHALL include the last 50 lines of build stderr so that the failure cause is diagnosable without separate log retrieval.

#### Scenario: Nix evaluation error captured

- **WHEN** a nix build fails due to an evaluation error (e.g., missing attribute, syntax error)
- **THEN** the `CiGetJobLogs` response SHALL contain the nix evaluation error message from stderr

#### Scenario: Clippy warnings captured on failure

- **WHEN** a clippy check fails due to `-D warnings` and clippy lint violations
- **THEN** the `CiGetJobLogs` response SHALL contain the clippy warning text from stderr

#### Scenario: Stderr flushed before job completion

- **WHEN** a nix build subprocess exits with non-zero status
- **THEN** the CI executor SHALL drain all remaining stderr output before reporting the job as failed, with a maximum drain timeout of 1 second

### Requirement: Dogfood pipeline prints failure details

The dogfood binary SHALL print actual failure output when a CI job fails, not just the job name and status.

#### Scenario: Failure log displayed on pipeline abort

- **WHEN** the dogfood pipeline detects a failed CI job
- **THEN** it SHALL print the first 50 lines of the failed job's log to stderr, prefixed with the job name

#### Scenario: Fallback to job result message

- **WHEN** `CiGetJobLogs` returns empty chunks for a failed job
- **THEN** the dogfood binary SHALL fall back to displaying the job's result message from `CiGetStatus`

### Requirement: Log streaming reliability

The log bridge between nix build stderr and the KV log store SHALL not lose lines when the build process exits quickly.

#### Scenario: Fast-failing build logs captured

- **WHEN** a nix build fails within 2 seconds of starting (e.g., immediate eval error)
- **THEN** all stderr output SHALL be captured in the KV log store before the job result is written
