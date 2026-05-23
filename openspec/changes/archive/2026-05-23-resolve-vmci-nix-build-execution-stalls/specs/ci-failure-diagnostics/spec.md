## MODIFIED Requirements

### Requirement: CI job failures include build stderr

When a CI nix build job fails or times out, the job result SHALL include bounded diagnostic output so that the failure cause is diagnosable without separate log retrieval.

#### Scenario: Nix evaluation error captured

- WHEN a nix build fails due to an evaluation error (e.g., missing attribute, syntax error)
- THEN the `CiGetJobLogs` response SHALL contain the nix evaluation error message from stderr

#### Scenario: Clippy warnings captured on failure

- WHEN a clippy check fails due to `-D warnings` and clippy lint violations
- THEN the `CiGetJobLogs` response SHALL contain the clippy warning text from stderr

#### Scenario: Stderr flushed before job completion

- WHEN a nix build subprocess exits with non-zero status
- THEN the CI executor SHALL drain all remaining stderr output before reporting the job as failed, with a bounded drain timeout

#### Scenario: Nix build timeout publishes failed job result [r[ci-failure-diagnostics.nix-build-timeout-publishes-result]]

- GIVEN a `ci_nix_build` job has a configured command timeout
- WHEN the underlying Nix build command exceeds that timeout
- THEN the executor MUST emit a bounded timeout marker
- AND it MUST cancel the command/process tree, bound stdout/stderr drain, and return a failed execution result
- AND the worker MUST publish a failed CI job result rather than leaving the job in `running` state until the dogfood wait timeout

### Requirement: Log streaming reliability

The log bridge between nix build stderr/progress markers and the KV log store SHALL not lose lines when the build process exits quickly or when a timeout/cancellation path finalizes the job.

#### Scenario: Fast-failing build logs captured

- WHEN a nix build fails within 2 seconds of starting (e.g., immediate eval error)
- THEN all stderr output SHALL be captured in the KV log store before the job result is written

#### Scenario: Timeout progress markers are retained [r[ci-failure-diagnostics.timeout-progress-retained]]

- GIVEN a CI command emits progress markers for start, heartbeat, timeout, watchdog timeout, or result publication
- WHEN the job fails or dogfood times out
- THEN `CiGetJobLogs` and VMCI diagnostics MUST retain the latest bounded marker set needed to identify the last phase
- AND retained markers MUST not include raw environment values, tickets, secret keys, or unbounded command arguments
