## ADDED Requirements

### Requirement: Cancel a running pipeline from the web UI

The forge web SHALL provide a cancel button on the pipeline detail page for pipelines in a non-terminal state (running, pending, initializing, checking_out). The button SHALL submit a POST request to `/{repo_id}/ci/{run_id}/cancel`. On success, the server SHALL call `CiCancelRun` and redirect to the pipeline detail page. The cancel button SHALL be styled as a destructive action (red outline).

#### Scenario: Cancel a running pipeline

- **WHEN** user views a pipeline detail page with status "running"
- **AND** clicks the "Cancel" button
- **THEN** the server SHALL call `CiCancelRun` with the run ID
- **AND** redirect to `/{repo_id}/ci/{run_id}`
- **AND** the pipeline status SHALL show as "cancelled"

#### Scenario: Cancel button not shown for completed pipelines

- **WHEN** user views a pipeline detail page with status "succeeded" or "failed"
- **THEN** the cancel button SHALL NOT be displayed

#### Scenario: Cancel fails gracefully

- **WHEN** user submits a cancel request for a pipeline that has already completed
- **THEN** the server SHALL display an error message on the pipeline detail page
- **AND** SHALL NOT crash or return a 500

### Requirement: Re-trigger a pipeline from the web UI

The forge web SHALL provide a re-trigger button on the pipeline detail page for pipelines in a terminal state (succeeded, failed, cancelled, checkout_failed). The button SHALL submit a POST request to `/{repo_id}/ci/{run_id}/retrigger`. On success, the server SHALL call `CiTriggerPipeline` with the original repo_id and ref_name, and redirect to the new pipeline's detail page.

#### Scenario: Re-trigger a failed pipeline

- **WHEN** user views a pipeline detail page with status "failed"
- **AND** clicks the "Re-trigger" button
- **THEN** the server SHALL call `CiTriggerPipeline` with the same repo_id and ref_name
- **AND** redirect to the new pipeline's detail page at `/{repo_id}/ci/{new_run_id}`

#### Scenario: Re-trigger button not shown for running pipelines

- **WHEN** user views a pipeline detail page with status "running"
- **THEN** the re-trigger button SHALL NOT be displayed

#### Scenario: Re-trigger fails gracefully

- **WHEN** user submits a re-trigger request and the cluster rejects it (e.g., concurrent run limit)
- **THEN** the server SHALL display an error message
- **AND** SHALL NOT redirect to a nonexistent run
