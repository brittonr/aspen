## Why

The forge web CI pages show pipeline status but lack the operational detail needed to actually diagnose problems. You can see that a pipeline failed, but not why — error messages aren't rendered, job durations are missing from list views, there's no way to cancel a stuck run, artifacts are invisible, and commit pages have no CI context at all. The API already exposes all this data (`CiCancelRun`, `CiListArtifacts`, `CiGetJobOutput`, `CommitStatus` in Forge KV, `error_message` on `PipelineRun`, `CiJobInfo.error`); the web UI just doesn't surface it.

## What Changes

- Render pipeline and job error messages on the run detail page
- Show duration on pipeline list rows and job rows (elapsed for running, total for completed)
- Add a cancel button on running pipeline detail pages (POST to new route, calls `CiCancelRun`)
- Add a re-trigger button on completed pipeline detail pages (POST, calls `CiTriggerPipeline` with same repo/ref)
- Show CI status badges on commit detail pages by reading `CommitStatus` from Forge KV
- Add an artifacts section to the job log page listing build artifacts with sizes and download links
- Add a stage timeline/progress visualization to the pipeline detail page — horizontal bar showing stage progression with color-coded status
- Add line numbers to the job log viewer
- Add a "full output" toggle that fetches complete stdout/stderr via `CiGetJobOutput`
- Add CI status per-branch on the repo overview branches list using `CiGetRefStatus`

## Capabilities

### New Capabilities

- `ci-observability-actions`: Cancel and re-trigger pipeline actions from the web UI
- `ci-artifacts-web`: Artifact listing and download links on job log pages
- `ci-commit-status-web`: CI status badges rendered on commit detail pages
- `ci-stage-timeline`: Visual stage progress bar on pipeline detail pages
- `ci-log-enhancements`: Line numbers, full output mode, and improved log rendering

### Modified Capabilities

- `ci-dashboard`: Add duration display, error message rendering, and branch CI status to existing pipeline list and detail pages

## Impact

- **Code**: `crates/aspen-forge-web/src/templates.rs` (new template functions, CSS), `routes.rs` (new POST routes for cancel/retrigger), `state.rs` (new methods wrapping existing client API calls)
- **APIs consumed**: `CiCancelRun`, `CiTriggerPipeline`, `CiListArtifacts`, `CiGetArtifact`, `CiGetJobOutput`, `CiGetRefStatus`, `CommitStatus` KV scan
- **No new API endpoints** — all data already available via existing `ClientRpcRequest` variants
- **NixOS test**: `nix/tests/forge-web-dashboard.nix` needs new subtests for cancel, artifacts, commit badges
