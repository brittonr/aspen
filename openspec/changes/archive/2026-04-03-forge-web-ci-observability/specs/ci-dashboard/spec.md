## MODIFIED Requirements

### Requirement: Pipeline run list page

The system SHALL display a list of CI pipeline runs with status, repo, ref, creation time, and duration. The list page SHALL be accessible at `/ci` (all repos) and `/{repo_id}/ci` (single repo). Runs SHALL be ordered by creation time descending. The list SHALL display at most 50 runs per page. Each run row SHALL show the duration: elapsed time for running pipelines, total duration for completed pipelines.

#### Scenario: View all pipeline runs

- **WHEN** user navigates to `/ci`
- **THEN** the page displays up to 50 recent pipeline runs across all repositories, each showing run ID (truncated), repo name, ref name, status badge, creation time, and duration

#### Scenario: View pipeline runs for a repo

- **WHEN** user navigates to `/{repo_id}/ci`
- **THEN** the page displays up to 50 recent pipeline runs for that repo only, with the repo navigation tabs visible

#### Scenario: No pipeline runs exist

- **WHEN** user navigates to `/ci` and no pipelines have run
- **THEN** the page displays a "No pipeline runs yet" message

#### Scenario: CI feature not available on cluster

- **WHEN** user navigates to `/ci` and the cluster does not have CI enabled
- **THEN** the page displays an informative message that CI is not available, similar to the existing ForgeUnavailable page

#### Scenario: Running pipeline shows elapsed duration

- **WHEN** a pipeline has status "running" and was created 2 minutes ago
- **THEN** the duration column SHALL display "2m" (computed from created_at_ms to current time via meta-refresh)

#### Scenario: Completed pipeline shows total duration

- **WHEN** a pipeline has status "succeeded" with created_at_ms and completed_at_ms 45 seconds apart
- **THEN** the duration column SHALL display "45s"

### Requirement: Pipeline run detail page

The system SHALL display a pipeline run's stages and jobs at `/{repo_id}/ci/{run_id}`. Each stage SHALL show its name and status. Each job within a stage SHALL show name, status badge, duration (if started), and a link to its log page. The page SHALL display error messages for failed pipelines and failed jobs.

#### Scenario: View a running pipeline

- **WHEN** user navigates to `/{repo_id}/ci/{run_id}` for a running pipeline
- **THEN** the page shows all stages with their current status, jobs with status badges and elapsed time, and auto-refreshes every 5 seconds via meta-refresh

#### Scenario: View a completed pipeline

- **WHEN** user navigates to `/{repo_id}/ci/{run_id}` for a succeeded or failed pipeline
- **THEN** the page shows all stages and jobs with final statuses, durations, and error messages for failed jobs. The page does NOT auto-refresh.

#### Scenario: Pipeline not found

- **WHEN** user navigates to `/{repo_id}/ci/{run_id}` with an invalid run ID
- **THEN** the page displays a 404 "Pipeline run not found" error

#### Scenario: Failed pipeline shows error message

- **WHEN** user views a failed pipeline detail page where `CiGetStatusResponse.error` is set
- **THEN** the page SHALL display the error message in a red callout box below the status badge

#### Scenario: Failed job shows error message

- **WHEN** user views a pipeline detail page with a failed job where `CiJobInfo.error` is set
- **THEN** the job row SHALL display the error message text below the job name

### Requirement: CI status per branch on repo overview

The repo overview page SHALL display CI status indicators next to each branch in the branches list. The status SHALL be fetched via `CiGetRefStatus` for each branch. If no CI status is available for a branch, no indicator SHALL be shown.

#### Scenario: Branch with passing CI

- **WHEN** user views the repo overview and branch "main" has a latest CI run with status "succeeded"
- **THEN** a small green dot or checkmark SHALL appear next to "main" in the branch list

#### Scenario: Branch with no CI runs

- **WHEN** user views the repo overview and branch "feature-x" has no CI runs
- **THEN** no CI indicator SHALL appear next to "feature-x"

## ADDED Requirements

### Requirement: Duration column in pipeline list

`CiRunInfo` SHALL include an optional `completed_at_ms` field (`Option<u64>`) to enable duration computation in the pipeline list without requiring per-row detail fetches. The field SHALL be populated by the CI handler when building list responses.

#### Scenario: List response includes completed_at_ms

- **WHEN** the CI handler builds a `CiListRunsResponse` for a completed pipeline with `completed_at: Some(1700000060000)` and `created_at: 1700000000000`
- **THEN** the `CiRunInfo` entry SHALL include `completed_at_ms: Some(1700000060000)`

#### Scenario: List response for running pipeline

- **WHEN** the CI handler builds a `CiListRunsResponse` for a running pipeline
- **THEN** the `CiRunInfo` entry SHALL include `completed_at_ms: None`
