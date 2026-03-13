## ADDED Requirements

### Requirement: CI jobs use branch overlay for workspace isolation

CI job executors SHALL create a `BranchOverlay` for each job's workspace. All KV writes during job execution SHALL go through the branch. On job success, the branch SHALL be committed. On job failure, the branch SHALL be dropped with no cleanup.

#### Scenario: Successful job commits workspace

- **WHEN** a CI job runs inside a branch overlay
- **AND** the job writes build artifacts to its workspace prefix
- **AND** the job completes successfully
- **THEN** all workspace writes SHALL be committed to the base store atomically

#### Scenario: Failed job leaves no orphaned keys

- **WHEN** a CI job runs inside a branch overlay
- **AND** the job writes partial results to its workspace prefix
- **AND** the job fails
- **THEN** the branch SHALL be dropped
- **AND** no workspace keys SHALL exist in the base store

#### Scenario: Job crash leaves no orphaned keys

- **WHEN** a CI job is running inside a branch overlay
- **AND** the node crashes mid-job
- **THEN** the in-memory branch state SHALL be lost
- **AND** the base store SHALL contain no partial workspace keys from the crashed job
- **AND** pipeline recovery SHALL reschedule the job

### Requirement: Parallel pipeline jobs use independent branches

Each parallel job within a CI pipeline stage SHALL use its own `BranchOverlay`. Branches SHALL be independent — one job's failure SHALL NOT affect another job's branch.

#### Scenario: Independent parallel branches

- **WHEN** jobs "build", "lint", and "test" run in parallel
- **AND** each has its own branch overlay
- **AND** "test" fails
- **THEN** the "test" branch SHALL be dropped
- **AND** the "build" and "lint" branches SHALL remain unaffected
- **AND** "build" and "lint" MAY still commit on success
