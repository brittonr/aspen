## 1. CI State Layer

- [x] 1.1 Add `ansi-to-html` crate to `aspen-forge-web/Cargo.toml`
- [x] 1.2 Add `AppState` methods for CI RPCs: `list_runs(repo_id, status, limit)`, `get_run_status(run_id)`, `get_job_logs(run_id, job_id, start_index, limit)`, `get_job_output(run_id, job_id)`
- [x] 1.3 Add `AppState::get_latest_ci_status(repo_id)` convenience method (list_runs limit=1 for default branch, returns Option)

## 2. CI Templates

- [x] 2.1 Add `pipeline_list` maud template: table of runs with status badge, repo link, ref, time. Include status filter links (all/running/succeeded/failed)
- [x] 2.2 Add `pipeline_detail` maud template: stage sections with job tables, status badges, duration, link to logs. Meta-refresh when running.
- [x] 2.3 Add `job_log_viewer` maud template: job header (name, status, duration), pre-formatted log output with ANSI→HTML conversion, "Load more" link. Meta-refresh when job running.
- [x] 2.4 Add CSS for CI pages: pipeline status badges (pending=gray, running=yellow, succeeded=green, failed=red, cancelled=gray), log viewer (dark pre block, monospace, ANSI color classes), stage sections

## 3. CI Routes

- [x] 3.1 Add route handlers: `ci_list_all`, `ci_list_repo`, `ci_run_detail`, `ci_job_logs` in `routes.rs`
- [x] 3.2 Wire routes into `dispatch()`: `/ci`, `/{repo_id}/ci`, `/{repo_id}/ci/{run_id}`, `/{repo_id}/ci/{run_id}/{job_id}`
- [x] 3.3 Add "CI" tab to `repo_tabs()` function
- [x] 3.4 Add "CI" link to main nav bar in `base_layout()`
- [x] 3.5 Add CI status badge to `repo_overview` template using `get_latest_ci_status`

## 4. Cluster Overview

- [x] 4.1 Add `AppState` methods for cluster RPCs: `get_health()`, `get_cluster_metrics()`
- [x] 4.2 Add `cluster_overview` maud template: summary banner (node count, healthy count, leader ID), node table (ID, role, health badge, uptime). Meta-refresh 10s.
- [x] 4.3 Add route handler `cluster_overview` and wire into dispatch at `/cluster`
- [x] 4.4 Add "Cluster" link to main nav bar in `base_layout()`
- [x] 4.5 Add CSS for cluster page: health status badges, role labels, summary cards

## 5. Service Registration

- [x] 5.1 Add optional `net` feature to `aspen-forge-web/Cargo.toml` with `aspen-client-api` net types
- [x] 5.2 Add `register_service()` function: sends `NetPublish` RPC with service name `forge-web`, endpoint info, and TCP port
- [x] 5.3 Add `deregister_service()` function: sends `NetUnpublish` RPC
- [x] 5.4 Wire registration into `main.rs`: call `register_service` after client connect, install ctrl-c handler that calls `deregister_service` before exit

## 6. Tests

- [x] 6.1 Add unit tests for ANSI→HTML conversion edge cases (nested colors, reset, 256-color, no escapes)
- [x] 6.2 Add route dispatch tests for new CI and cluster paths (status codes, content types)
- [x] 6.3 Add template rendering tests for CI pages (empty states, running states, completed states)
