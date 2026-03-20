## ADDED Requirements

### Requirement: Pipeline status reporting

The pipeline orchestrator SHALL invoke a `StatusReporter` on pipeline state transitions. The reporter SHALL be called when a pipeline is created (Pending) and when it reaches a terminal state (Success, Failed, Cancelled). Reporter failures SHALL be logged but SHALL NOT affect pipeline execution.

#### Scenario: Status reported on pipeline creation

- **WHEN** the orchestrator creates a new pipeline run via `track_run`
- **THEN** it SHALL call `status_reporter.report_status()` with state `Pending`
- **AND** the report SHALL include the repo ID, commit hash, ref name, run ID, and context `"ci/pipeline"`

#### Scenario: Status reported on pipeline success

- **WHEN** `sync_run_status` detects a pipeline has reached `Success`
- **THEN** it SHALL call `status_reporter.report_status()` with state `Success`

#### Scenario: Status reported on pipeline failure

- **WHEN** `sync_run_status` detects a pipeline has reached `Failed`
- **THEN** it SHALL call `status_reporter.report_status()` with state `Failure`

#### Scenario: Status reported on pipeline cancellation

- **WHEN** `sync_run_status` detects a pipeline has reached `Cancelled`
- **THEN** it SHALL call `status_reporter.report_status()` with state `Error`

#### Scenario: Reporter error does not block pipeline

- **WHEN** `status_reporter.report_status()` returns an error
- **THEN** the orchestrator SHALL log the error at warn level
- **AND** the pipeline run SHALL continue normally
- **AND** the pipeline status SHALL still be persisted to CI's KV namespace

### Requirement: Orchestrator accepts optional status reporter

The `PipelineOrchestrator` SHALL accept an optional `StatusReporter` at construction time. When no reporter is provided, status reporting SHALL be silently skipped.

#### Scenario: Orchestrator with reporter

- **WHEN** a `PipelineOrchestrator` is created with a `StatusReporter`
- **THEN** all pipeline state transitions SHALL invoke the reporter

#### Scenario: Orchestrator without reporter

- **WHEN** a `PipelineOrchestrator` is created without a `StatusReporter`
- **THEN** pipeline execution SHALL proceed identically to current behavior
- **AND** no commit statuses SHALL be written
