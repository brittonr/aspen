## ADDED Requirements

### Requirement: Test runs emit machine-readable harness health reports

The harness SHALL emit machine-readable reports for completed runs that capture retries, retry-only passes, skipped suites, timeout counts, and runtime hotspots. A suite that passes only after retry SHALL be reported as unstable rather than indistinguishable from a clean first-pass success.

#### Scenario: Retry-only pass is reported as unstable

- **WHEN** a suite fails on its initial attempt and passes on a configured retry
- **THEN** the run report SHALL record the suite as a retry-only pass with its retry count and final outcome

#### Scenario: Slow suites are ranked in the run report

- **WHEN** a run completes
- **THEN** the report SHALL include the slowest suites or tests and any timeout counts observed during the run

### Requirement: Harness reports summarize coverage by layer

The harness SHALL summarize coverage by subsystem and test layer using suite metadata. The summary SHALL identify missing higher-layer coverage and explain skipped suites when prerequisites are unavailable.

#### Scenario: Missing higher-layer coverage is visible

- **WHEN** suite metadata shows a subsystem has unit and simulation coverage but no real-network or VM suite
- **THEN** the summary SHALL mark the missing layers for that subsystem

#### Scenario: Skipped suites include the reason

- **WHEN** a suite is skipped because its prerequisites are unavailable
- **THEN** the report SHALL record the skipped suite, the reason for the skip, and the prerequisite that was not satisfied
