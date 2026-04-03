## 1. API: Add completed_at_ms to CiRunInfo

- [x] 1.1 Add `completed_at_ms: Option<u64>` field with `#[serde(default)]` to `CiRunInfo` in `crates/aspen-client-api/src/messages/ci.rs`
- [x] 1.2 Populate `completed_at_ms` in the CI handler that builds `CiListRunsResponse` from `PipelineRun` (in `crates/aspen-ci-handler/`)
- [x] 1.3 Update any existing tests that construct `CiRunInfo` to include the new field

## 2. Pipeline list: Duration display and error rendering

- [x] 2.1 Add a "Duration" column to the `pipeline_list` template in `templates.rs` — compute from `created_at_ms` and `completed_at_ms` (or show "running…" if None)
- [x] 2.2 Update `pipeline_list` tests to verify duration rendering for completed and running runs

## 3. Pipeline detail: Error messages

- [x] 3.1 Render `CiGetStatusResponse.error` as a red callout box below the status badge in `pipeline_detail` template
- [x] 3.2 Render `CiJobInfo.error` inline in the `job_row` template below the job name
- [x] 3.3 Add tests for error rendering in pipeline detail and job row templates

## 4. Pipeline detail: Cancel and re-trigger actions

- [x] 4.1 Add POST route `/{repo_id}/ci/{run_id}/cancel` in `routes.rs` — calls `state.cancel_run()`, redirects to detail page
- [x] 4.2 Add POST route `/{repo_id}/ci/{run_id}/retrigger` in `routes.rs` — calls `state.retrigger_run()`, redirects to new run detail
- [x] 4.3 Add `cancel_run(run_id)` and `retrigger_run(repo_id, ref_name)` methods to `AppState` in `state.rs`
- [x] 4.4 Add cancel button (red outline, destructive style) to `pipeline_detail` template for non-terminal runs
- [x] 4.5 Add re-trigger button to `pipeline_detail` template for terminal runs
- [x] 4.6 Add CSS for `.btn-cancel` (destructive red outline style)
- [x] 4.7 Add tests for cancel/retrigger button visibility based on pipeline status

## 5. Stage timeline visualization

- [x] 5.1 Add `stage_timeline` template function that renders a horizontal flexbox progress bar from `&[CiStageInfo]`
- [x] 5.2 Add CSS for `.stage-timeline`, `.stage-segment`, and per-status segment colors
- [x] 5.3 Insert `stage_timeline` call in `pipeline_detail` above the stage sections
- [x] 5.4 Add tests for timeline rendering with various stage status combinations

## 6. Commit page CI badges

- [x] 6.1 Add `get_commit_statuses(repo_id, commit_hash)` method to `AppState` — scans `forge:status:{repo_hex}:{commit_hex}:` prefix, returns `Vec<CommitStatus>`, returns empty on error
- [x] 6.2 Add `commit_status_badges` template function that renders badges for each `CommitStatus` with status-colored badges and links to pipeline runs
- [x] 6.3 Call `get_commit_statuses` in the `commit_detail` route handler and pass results to template
- [x] 6.4 Add CSS for `.commit-ci-badge` with state-based coloring (reuse ci-succeeded/ci-failed classes)
- [x] 6.5 Add tests for commit status badge rendering

## 7. Job log enhancements: Line numbers

- [x] 7.1 Refactor `job_log_viewer` template to render log output as a `<table>` with line number column and content column instead of bare `<pre>`
- [x] 7.2 Split log chunk content into lines and assign sequential line numbers across all chunks
- [x] 7.3 Add CSS for `.log-line-number` (muted, non-selectable, right-aligned)
- [x] 7.4 Add test that line numbers are sequential across multiple chunks

## 8. Job log enhancements: Full output mode

- [x] 8.1 Add `full` query parameter handling in `ci_job_logs` route handler — when `?full=1`, call `state.get_job_output()` instead of `get_job_logs()`
- [x] 8.2 Add `job_full_output_viewer` template function that renders stdout and stderr as separate sections with distinct backgrounds
- [x] 8.3 Add "View full output" / "View chunked logs" toggle links to both log templates
- [x] 8.4 Cap full output display at 1 MB with a truncation notice
- [x] 8.5 Add CSS for `.log-stderr` (red-tinted background) and `.log-stdout` sections
- [x] 8.6 Add tests for full output rendering with stdout-only and stdout+stderr cases

## 9. Artifacts on job log page

- [x] 9.1 Add `list_artifacts(job_id, run_id)` method to `AppState` wrapping `CiListArtifacts`
- [x] 9.2 Call `list_artifacts` in `ci_job_logs` handler and pass results to template
- [x] 9.3 Add `artifacts_section` template function — renders artifact table with name, size (human-readable), content type, and CLI download hint
- [x] 9.4 Add `format_bytes` helper for human-readable sizes (B, KB, MB, GB)
- [x] 9.5 Add CSS for `.artifacts-section` and `.artifact-row`
- [x] 9.6 Add tests for artifact rendering and `format_bytes` helper

## 10. Branch CI status on repo overview

- [x] 10.1 Add `get_ref_status(repo_id, ref_name)` method to `AppState` wrapping `CiGetRefStatus`
- [x] 10.2 Fetch CI status for each branch in `repo_overview` handler (best-effort, cap at 20 branches)
- [x] 10.3 Render small status dots next to branch names in the repo overview branches list
- [x] 10.4 Add CSS for `.branch-ci-dot` with status-based coloring
- [x] 10.5 Add tests for branch CI status rendering

## 11. NixOS VM test updates

- [x] 11.1 Add cancel button visibility test to `forge-web-dashboard.nix`
- [x] 11.2 Add commit status badge test (write a CommitStatus to KV, verify it appears on commit page)
- [x] 11.3 Add stage timeline rendering test
- [x] 11.4 Add duration column test in pipeline list
