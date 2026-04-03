## ADDED Requirements

### Requirement: Job-not-found treated as incomplete

When syncing pipeline status, a job that cannot be found in the job store MUST NOT be treated as completed. Missing jobs indicate read failures or replication lag, not successful completion.

#### Scenario: Job lookup returns None during status sync

- **WHEN** `check_active_job_statuses` queries a job ID
- **AND** the job store returns `Ok(None)`
- **THEN** the job is treated as not-yet-completed (same as Pending/Running)
- **AND** the pipeline status remains non-terminal

#### Scenario: Job lookup fails with error during status sync

- **WHEN** `check_active_job_statuses` queries a job ID
- **AND** the read returns an error (e.g., forwarding failure)
- **THEN** the job is treated as not-yet-completed
- **AND** the pipeline status remains non-terminal
