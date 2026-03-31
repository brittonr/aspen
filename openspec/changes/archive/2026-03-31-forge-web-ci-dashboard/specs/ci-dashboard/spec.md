## ADDED Requirements

### Requirement: Pipeline run list page

The system SHALL display a list of CI pipeline runs with status, repo, ref, and creation time. The list page SHALL be accessible at `/ci` (all repos) and `/{repo_id}/ci` (single repo). Runs SHALL be ordered by creation time descending. The list SHALL display at most 50 runs per page.

#### Scenario: View all pipeline runs

- **WHEN** user navigates to `/ci`
- **THEN** the page displays up to 50 recent pipeline runs across all repositories, each showing run ID (truncated), repo name, ref name, status badge, and creation time

#### Scenario: View pipeline runs for a repo

- **WHEN** user navigates to `/{repo_id}/ci`
- **THEN** the page displays up to 50 recent pipeline runs for that repo only, with the repo navigation tabs visible

#### Scenario: No pipeline runs exist

- **WHEN** user navigates to `/ci` and no pipelines have run
- **THEN** the page displays a "No pipeline runs yet" message

#### Scenario: CI feature not available on cluster

- **WHEN** user navigates to `/ci` and the cluster does not have CI enabled
- **THEN** the page displays an informative message that CI is not available, similar to the existing ForgeUnavailable page

### Requirement: Pipeline run detail page

The system SHALL display a pipeline run's stages and jobs at `/{repo_id}/ci/{run_id}`. Each stage SHALL show its name and status. Each job within a stage SHALL show name, status badge, duration (if started), and a link to its log page.

#### Scenario: View a running pipeline

- **WHEN** user navigates to `/{repo_id}/ci/{run_id}` for a running pipeline
- **THEN** the page shows all stages with their current status, jobs with status badges and elapsed time, and auto-refreshes every 5 seconds via meta-refresh

#### Scenario: View a completed pipeline

- **WHEN** user navigates to `/{repo_id}/ci/{run_id}` for a succeeded or failed pipeline
- **THEN** the page shows all stages and jobs with final statuses, durations, and error messages for failed jobs. The page does NOT auto-refresh.

#### Scenario: Pipeline not found

- **WHEN** user navigates to `/{repo_id}/ci/{run_id}` with an invalid run ID
- **THEN** the page displays a 404 "Pipeline run not found" error

### Requirement: Job log viewer

The system SHALL display job logs at `/{repo_id}/ci/{run_id}/{job_id}`. Logs SHALL be fetched via `CiGetJobLogs` in chunks. ANSI escape codes in log output SHALL be converted to colored HTML spans. The viewer SHALL display a "Load more" link when additional chunks are available.

#### Scenario: View job logs

- **WHEN** user navigates to `/{repo_id}/ci/{run_id}/{job_id}`
- **THEN** the page displays the job name, status, duration, and log output with ANSI colors rendered as HTML. Up to 100 log chunks are shown initially.

#### Scenario: Load additional log chunks

- **WHEN** user clicks "Load more" on a log page with more than 100 chunks
- **THEN** the page reloads with the next batch of 100 chunks appended, starting from the `start_index` query parameter

#### Scenario: Job still running

- **WHEN** user views logs for a job with status "running"
- **THEN** the page auto-refreshes every 5 seconds to show new log output. The page scrolls to show the most recent output.

#### Scenario: Job not found

- **WHEN** user navigates to a log page with an invalid job ID
- **THEN** the page displays a "Job not found" message

### Requirement: CI status badge on repo overview

The system SHALL display the latest CI pipeline status as a badge on the repo overview page. The badge SHALL show the status of the most recent pipeline run for the repo's default branch. If no CI runs exist or CI is unavailable, the badge SHALL be omitted silently.

#### Scenario: Repo with passing CI

- **WHEN** user views a repo overview page and the latest pipeline for the default branch succeeded
- **THEN** a green "CI: passing" badge is displayed next to the branch name, linking to the pipeline run detail

#### Scenario: Repo with failing CI

- **WHEN** user views a repo overview page and the latest pipeline for the default branch failed
- **THEN** a red "CI: failing" badge is displayed, linking to the pipeline run detail

#### Scenario: Repo with running CI

- **WHEN** user views a repo overview page and a pipeline is currently running for the default branch
- **THEN** a yellow "CI: running" badge is displayed, linking to the pipeline run detail

#### Scenario: Repo with no CI

- **WHEN** user views a repo overview page and no CI runs exist for that repo
- **THEN** no CI badge is displayed. No error or placeholder is shown.

### Requirement: Navigation integration

The system SHALL add CI navigation links to the existing page structure. The main nav bar SHALL include a link to `/ci`. Each repo's tab bar SHALL include a "CI" tab linking to `/{repo_id}/ci`.

#### Scenario: Navigate to CI from nav bar

- **WHEN** user clicks "CI" in the main navigation bar
- **THEN** the browser navigates to `/ci` showing all pipeline runs

#### Scenario: Navigate to CI from repo tabs

- **WHEN** user clicks the "CI" tab on a repo page
- **THEN** the browser navigates to `/{repo_id}/ci` showing that repo's pipeline runs
