//! CI pipeline operations — trigger, poll status, fetch logs.

use std::time::Duration;

use aspen_client::AspenClient;
use aspen_client_api::CI_STATUS_CANCELLED;
use aspen_client_api::CI_STATUS_CHECKOUT_FAILED;
use aspen_client_api::CI_STATUS_FAILED;
use aspen_client_api::CI_STATUS_PENDING;
use aspen_client_api::CI_STATUS_RUNNING;
use aspen_client_api::CI_STATUS_SUCCESS;
use aspen_client_api::CI_TERMINAL_STATUS_LABELS;
use aspen_client_api::CiGetStatusResponse;
use aspen_client_api::CiJobInfo;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use tracing::info;
use tracing::warn;

use crate::error::CiPipelineSnafu;
use crate::error::DogfoodResult;
use crate::error::redact_credential_fragments;

const DIRECT_ROUTE_LOSS_STATUS: &str = "direct_route_lost";

fn is_direct_route_loss_error(error: &str) -> bool {
    error.contains("No address lookup configured")
        || error.contains("direct-only Iroh client has no direct address")
        || error.contains("ticket/bootstrap address was not registered")
}

fn ci_status_for_rpc_error(error: &str) -> &'static str {
    if is_direct_route_loss_error(error) {
        DIRECT_ROUTE_LOSS_STATUS
    } else {
        "status_rpc_failed"
    }
}

#[derive(Copy, Clone)]
pub struct WaitPipelineTarget<'a> {
    pub ticket: &'a str,
    pub repo_name: &'a str,
}

#[derive(Copy, Clone)]
struct RepoLookupTarget<'a> {
    repo_name: &'a str,
    ticket: &'a str,
}

#[derive(Copy, Clone)]
struct RepoRunLookup<'a> {
    repo_id: &'a str,
    ticket: &'a str,
}

#[derive(Copy, Clone)]
struct PipelineRunTarget<'a> {
    run_id: &'a str,
    ticket: &'a str,
}

#[derive(Copy, Clone)]
struct JobLogTarget<'a> {
    run_id: &'a str,
    job_id: &'a str,
    ticket: &'a str,
}

#[derive(Copy, Clone)]
struct RpcContext<'a> {
    operation: &'a str,
    ticket: &'a str,
}

const CI_RPC_TIMEOUT_SECS: u64 = 20;
const MAX_TIMEOUT_RUNNING_JOB_LOG_BYTES: usize = 4096;
const CI_JOB_LOG_PAGE_LIMIT: u32 = 200;
const MAX_FAILURE_JOB_LOG_BYTES: usize = 256 * 1024;
const MAX_FAILURE_JOB_LOG_PAGES: u32 = 64;
const CI_PROGRESS_MARKER: &str = "ASPEN_CI_COMMAND_PROGRESS";
const CI_NIX_BUILD_PHASES: &[&str] = &[
    "job_spec_parse_done",
    "nix_payload_transform_enter",
    "nix_payload_transformed",
    "nix_payload_transform_done",
    "working_dir_rewrite_enter",
    "working_dir_rewrite_done",
    "job_construct_enter",
    "job_construct_done",
    "active_log_job_enter",
    "active_log_job_done",
    "visibility_extender_spawn_enter",
    "visibility_extender_spawn_done",
    "executor_enter",
    "local_executor_execute_enter",
    "local_executor_payload_parse_enter",
    "local_executor_payload_parse_done",
    "local_executor_payload_validate_enter",
    "local_executor_payload_validate_done",
    "local_executor_execute_job_enter",
    "local_executor_payload_validated",
    "workspace_setup_enter",
    "workspace_materialization_enter",
    "source_blob_fetch_enter",
    "source_blob_fetch_done",
    "archive_decode_enter",
    "workspace_unpack_enter",
    "workspace_unpack_done",
    "archive_decode_done",
    "workspace_materialization_done",
    "workspace_materialization_failed",
    "workspace_preflight_enter",
    "workspace_preflight_done",
    "workspace_ready",
    "cache_proxy_start_enter",
    "cache_proxy_start_done",
    "cache_proxy_start_failed",
    "cache_proxy_start_timeout",
    "cache_proxy_skipped",
    "command_request_built",
    "command_execute_enter",
    "command_started",
    "command_running",
    "command_timeout",
    "executor_watchdog_timeout",
    "executor_job_timeout",
    "command_execute_returned",
    "local_executor_execute_job_returned",
    "local_executor_execute_job_failed",
    "result_publish_enter",
    "result_published",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunningJobPhaseSummary {
    latest_phase: Option<String>,
    missing_phase: Option<&'static str>,
}

#[allow(unknown_lints)]
#[allow(ambient_clock, reason = "dogfood CI polling needs monotonic elapsed timing")]
fn monotonic_now() -> tokio::time::Instant {
    tokio::time::Instant::now()
}

/// Wait for a CI pipeline to complete on the given repo.
///
/// First polls `CiListRuns` for an auto-triggered pipeline (up to 120s).
/// If none is found, triggers one via `CiTriggerPipeline`.
/// Then polls `CiGetStatus` with exponential backoff until terminal state.
///
/// Returns the run_id on success.
pub async fn wait_for_pipeline(target: WaitPipelineTarget<'_>, timeout_secs: u64) -> DogfoodResult<String> {
    debug_assert!(!target.ticket.is_empty(), "ticket must not be empty");
    debug_assert!(!target.repo_name.is_empty(), "repo name must not be empty");
    debug_assert!(timeout_secs > 0, "pipeline timeout must be positive");

    let client = connect(target.ticket).await?;
    let result = async {
        let repo_target = RepoLookupTarget {
            repo_name: target.repo_name,
            ticket: target.ticket,
        };
        let repo_id = resolve_repo_id(&client, repo_target).await?;
        let repo_run_lookup = RepoRunLookup {
            repo_id: &repo_id,
            ticket: target.ticket,
        };

        info!("  looking for auto-triggered pipeline...");
        let run_id = match find_recent_run(&client, repo_run_lookup, Duration::from_secs(120)).await {
            Some(id) => {
                info!("  found pipeline: {id}");
                id
            }
            None => {
                warn!("  no auto-triggered pipeline, triggering manually...");
                trigger_pipeline(&client, repo_run_lookup).await?
            }
        };

        info!("  waiting for pipeline {run_id}...");
        poll_pipeline(
            &client,
            PipelineRunTarget {
                run_id: &run_id,
                ticket: target.ticket,
            },
            timeout_secs,
        )
        .await?;
        Ok(run_id)
    }
    .await;

    client.shutdown().await;
    result
}

/// Resolve a repo name to a repo ID via `ForgeListRepos`.
async fn resolve_repo_id(client: &AspenClient, target: RepoLookupTarget<'_>) -> DogfoodResult<String> {
    match crate::forge::lookup_repo_id_with_client(client, target.repo_name, target.ticket).await? {
        Some(repo_id) => Ok(repo_id),
        None => crate::error::ForgeSnafu {
            operation: "resolve repo id",
            reason: format!("repo '{}' not found", target.repo_name),
        }
        .fail(),
    }
}

/// Poll `CiListRuns` looking for a run associated with this repo.
async fn find_recent_run(client: &AspenClient, target: RepoRunLookup<'_>, timeout_window: Duration) -> Option<String> {
    debug_assert!(!target.repo_id.is_empty(), "repo id must not be empty");
    debug_assert!(!target.ticket.is_empty(), "ticket must not be empty");
    debug_assert!(timeout_window > Duration::ZERO, "timeout must be positive");

    let start = monotonic_now();
    while start.elapsed() <= timeout_window {
        if let Ok(resp) = send(
            client,
            ClientRpcRequest::CiListRuns {
                repo_id: Some(target.repo_id.to_string()),
                status: None,
                limit: Some(5),
            },
            RpcContext {
                operation: "CiListRuns",
                ticket: target.ticket,
            },
        )
        .await
            && let ClientRpcResponse::CiListRunsResult(list) = resp
            && let Some(run) = list.runs.first()
        {
            debug_assert!(!run.run_id.is_empty(), "CI run IDs should not be empty");
            return Some(run.run_id.clone());
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    None
}

/// Manually trigger a pipeline and return the run_id.
async fn trigger_pipeline(client: &AspenClient, target: RepoRunLookup<'_>) -> DogfoodResult<String> {
    debug_assert!(!target.repo_id.is_empty(), "repo id must not be empty");
    debug_assert!(!target.ticket.is_empty(), "ticket must not be empty");

    let resp = send(
        client,
        ClientRpcRequest::CiTriggerPipeline {
            repo_id: target.repo_id.to_string(),
            ref_name: "refs/heads/main".to_string(),
            commit_hash: None,
        },
        RpcContext {
            operation: "CiTriggerPipeline",
            ticket: target.ticket,
        },
    )
    .await?;

    match resp {
        ClientRpcResponse::CiTriggerPipelineResult(r) if r.is_success => {
            let run_id = r.run_id.unwrap_or_else(|| "unknown".to_string());
            debug_assert!(!run_id.is_empty(), "successful trigger should yield a run id placeholder");
            info!("  triggered pipeline: {run_id}");
            Ok(run_id)
        }
        ClientRpcResponse::CiTriggerPipelineResult(r) => CiPipelineSnafu {
            run_id: "?",
            status: "trigger_failed",
            detail: r.error.unwrap_or_else(|| "unknown error".to_string()),
        }
        .fail(),
        other => CiPipelineSnafu {
            run_id: "?",
            status: "trigger_failed",
            detail: format!("unexpected response: {other:?}"),
        }
        .fail(),
    }
}

fn is_success_status(status: &str) -> bool {
    // `succeeded` is retained for legacy receipts/logs; `success` is the stable CI label.
    status == CI_STATUS_SUCCESS || status == "succeeded"
}

fn is_running_status(status: &str) -> bool {
    status == CI_STATUS_RUNNING || status == "running"
}

fn is_terminal_failure_status(status: &str) -> bool {
    CI_TERMINAL_STATUS_LABELS.contains(&status) && !is_success_status(status)
}

/// Poll pipeline status with exponential backoff until terminal state.
async fn poll_pipeline(client: &AspenClient, target: PipelineRunTarget<'_>, timeout_secs: u64) -> DogfoodResult<()> {
    debug_assert!(!target.run_id.is_empty(), "run id must not be empty");
    debug_assert!(!target.ticket.is_empty(), "ticket must not be empty");
    debug_assert!(timeout_secs > 0, "poll timeout must be positive");

    let start = monotonic_now();
    let poll_window = Duration::from_secs(timeout_secs);
    let mut poll_backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(10);

    while start.elapsed() <= poll_window {
        let resp = match send(
            client,
            ClientRpcRequest::CiGetStatus {
                run_id: target.run_id.to_string(),
            },
            RpcContext {
                operation: "CiGetStatus",
                ticket: target.ticket,
            },
        )
        .await
        {
            Ok(resp) => resp,
            Err(err) => {
                let detail = err.to_string();
                return CiPipelineSnafu {
                    run_id: target.run_id,
                    status: ci_status_for_rpc_error(&detail),
                    detail,
                }
                .fail();
            }
        };

        if let ClientRpcResponse::CiGetStatusResult(status) = &resp
            && let Some(pipeline_status) = &status.status
        {
            if is_success_status(pipeline_status) {
                print_pipeline_summary(status);
                return Ok(());
            }
            if is_terminal_failure_status(pipeline_status) {
                print_pipeline_summary(status);
                let log_detail = fetch_failure_logs(client, target, status).await;
                return CiPipelineSnafu {
                    run_id: target.run_id,
                    status: pipeline_status,
                    detail: log_detail,
                }
                .fail();
            }
        }

        tokio::time::sleep(poll_backoff).await;
        poll_backoff = (poll_backoff * 2).min(max_backoff);
    }

    let timeout_detail = match fetch_pipeline_timeout_summary(client, target).await {
        Some(summary) => summary,
        None => "pipeline status unavailable at timeout".to_string(),
    };

    CiPipelineSnafu {
        run_id: target.run_id,
        status: "timeout",
        detail: format!("timed out after {timeout_secs}s\n{timeout_detail}"),
    }
    .fail()
}

async fn fetch_pipeline_timeout_summary(client: &AspenClient, target: PipelineRunTarget<'_>) -> Option<String> {
    let resp = send(
        client,
        ClientRpcRequest::CiGetStatus {
            run_id: target.run_id.to_string(),
        },
        RpcContext {
            operation: "CiGetStatus",
            ticket: target.ticket,
        },
    )
    .await
    .ok()?;

    if let ClientRpcResponse::CiGetStatusResult(status) = resp {
        print_pipeline_summary(&status);
        let mut summary = format_pipeline_summary(&status);
        let running_logs = fetch_running_job_log_summaries(client, target, &status).await;
        if !running_logs.is_empty() {
            summary.push_str(&running_logs);
        }
        return Some(summary);
    }

    None
}

async fn fetch_running_job_log_summaries(
    client: &AspenClient,
    target: PipelineRunTarget<'_>,
    status: &CiGetStatusResponse,
) -> String {
    let mut summary = String::new();
    for stage in &status.stages {
        for job in &stage.jobs {
            if !is_running_status(&job.status) {
                continue;
            }
            let log_text = fetch_job_log_chunks(client, JobLogTarget {
                run_id: target.run_id,
                job_id: &job.id,
                ticket: target.ticket,
            })
            .await;
            append_running_job_log_summary(&mut summary, &stage.name, job, &log_text);
        }
    }
    summary
}

fn append_running_job_log_summary(summary: &mut String, stage_name: &str, job: &CiJobInfo, log_text: &str) {
    let redacted_log = redact_timeout_log_text(log_text);
    let phase_summary = summarize_running_job_phases(&redacted_log);

    summary.push_str(&format!("--- Running job diagnostics: {} (stage: {}) ---\n", job.name, stage_name));
    summary.push_str(&format!("job_id={}\n", job.id));
    summary.push_str(&format!("job_status={}\n", job.status));
    match &phase_summary.latest_phase {
        Some(phase) => summary.push_str(&format!("latest_phase={phase}\n")),
        None => summary.push_str("latest_phase=none\n"),
    }
    match phase_summary.missing_phase {
        Some(phase) => summary.push_str(&format!("missing_phase={phase}\n")),
        None => summary.push_str("missing_phase=none\n"),
    }

    if redacted_log.is_empty() {
        return;
    }
    summary.push_str(&format!("--- Running job log tail: {} (stage: {}) ---\n", job.name, stage_name));
    summary.push_str(&bounded_tail(&redacted_log, MAX_TIMEOUT_RUNNING_JOB_LOG_BYTES));
    if !summary.ends_with('\n') {
        summary.push('\n');
    }
}

fn summarize_running_job_phases(log_text: &str) -> RunningJobPhaseSummary {
    let latest_phase = latest_progress_phase(log_text).map(str::to_string);
    let missing_phase = latest_phase
        .as_deref()
        .and_then(next_expected_phase)
        .or(Some(CI_NIX_BUILD_PHASES[0]))
        .filter(|_| latest_phase.as_deref() != Some("result_published"));

    RunningJobPhaseSummary {
        latest_phase,
        missing_phase,
    }
}

fn latest_progress_phase(log_text: &str) -> Option<&str> {
    log_text
        .lines()
        .filter_map(|line| {
            if !line.contains(CI_PROGRESS_MARKER) {
                return None;
            }
            extract_marker_field(line, "phase")
        })
        .next_back()
}

fn next_expected_phase(latest_phase: &str) -> Option<&'static str> {
    CI_NIX_BUILD_PHASES
        .iter()
        .position(|phase| *phase == latest_phase)
        .and_then(|index| CI_NIX_BUILD_PHASES.get(index + 1).copied())
}

fn extract_marker_field<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    let prefix = format!("{field}=");
    let value = line.split_whitespace().find_map(|token| token.strip_prefix(&prefix))?;
    Some(value.trim_matches(|c| c == ',' || c == ';'))
}

fn redact_timeout_log_text(log_text: &str) -> String {
    let mut redacted = redact_credential_fragments(log_text);
    for flag in ["--iroh-secret-key", "--cluster-ticket", "--token", "--password"] {
        redacted = redact_flag_value(&redacted, flag);
    }
    redacted
}

fn redact_flag_value(input: &str, flag: &str) -> String {
    input.lines().map(|line| redact_flag_value_line(line, flag)).collect::<Vec<_>>().join("\n")
}

fn redact_flag_value_line(line: &str, flag: &str) -> String {
    let mut output = Vec::new();
    let mut redact_next = false;
    for token in line.split_whitespace() {
        if redact_next {
            output.push("[REDACTED]".to_string());
            redact_next = false;
            continue;
        }
        if token == flag {
            output.push(token.to_string());
            redact_next = true;
        } else if let Some((prefix, _value)) = token.split_once('=') {
            if prefix == flag {
                output.push(format!("{flag}=[REDACTED]"));
            } else {
                output.push(token.to_string());
            }
        } else {
            output.push(token.to_string());
        }
    }
    output.join(" ")
}

fn bounded_tail(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut start = text.len().saturating_sub(max_bytes);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    format!("... [truncated to last {max_bytes} bytes]\n{}", &text[start..])
}

/// Fetch logs from failed jobs for error reporting.
///
/// Tries `CiGetJobLogs` first (streamed build output). If no log chunks
/// are available, falls back to the job's `error` field from `CiGetStatus`
/// which contains the `JobResult::failure` message.
async fn fetch_failure_logs(
    client: &AspenClient,
    target: PipelineRunTarget<'_>,
    status: &CiGetStatusResponse,
) -> String {
    debug_assert!(!target.run_id.is_empty(), "run id must not be empty");
    debug_assert!(!target.ticket.is_empty(), "ticket must not be empty");

    for stage in &status.stages {
        for job in &stage.jobs {
            if !is_terminal_failure_status(&job.status) {
                continue;
            }

            let log_text = fetch_job_log_chunks(client, JobLogTarget {
                run_id: target.run_id,
                job_id: &job.id,
                ticket: target.ticket,
            })
            .await;
            if !log_text.is_empty() {
                debug_assert!(!job.name.is_empty(), "failed job should have a name");
                debug_assert!(!stage.name.is_empty(), "failed stage should have a name");
                return format!("--- Failed job: {} (stage: {}) ---\n{}", job.name, stage.name, log_text);
            }
            if let Some(ref error) = job.error {
                return format!("--- Failed job: {} (stage: {}) ---\n{}", job.name, stage.name, error);
            }
            return format!(
                "--- Failed job: {} (stage: {}) ---\nNo error details available. \
                 The job may have failed before any output was produced.",
                job.name, stage.name
            );
        }
    }

    "no failed jobs found in pipeline status".to_string()
}

/// Fetch a bounded tail of log chunks for a specific job from the KV log store.
async fn fetch_job_log_chunks(client: &AspenClient, target: JobLogTarget<'_>) -> String {
    debug_assert!(!target.run_id.is_empty(), "run id must not be empty");
    debug_assert!(!target.job_id.is_empty(), "job id must not be empty");
    debug_assert!(!target.ticket.is_empty(), "ticket must not be empty");

    let mut start_index = 0u32;
    let mut pages_fetched = 0u32;
    let mut text = String::new();

    loop {
        let Ok(resp) = send(
            client,
            ClientRpcRequest::CiGetJobLogs {
                run_id: target.run_id.to_string(),
                job_id: target.job_id.to_string(),
                start_index,
                limit: Some(CI_JOB_LOG_PAGE_LIMIT),
            },
            RpcContext {
                operation: "CiGetJobLogs",
                ticket: target.ticket,
            },
        )
        .await
        else {
            return text;
        };

        let ClientRpcResponse::CiGetJobLogsResult(logs) = resp else {
            return text;
        };
        if !logs.was_found || logs.chunks.is_empty() {
            return text;
        }

        append_bounded_log_chunks(&mut text, logs.chunks.iter().map(|chunk| chunk.content.as_str()));
        pages_fetched += 1;
        if !logs.has_more || pages_fetched >= MAX_FAILURE_JOB_LOG_PAGES {
            return text;
        }
        start_index = logs.last_index.saturating_add(1);
    }
}

fn append_bounded_log_chunks<'a>(text: &mut String, chunks: impl IntoIterator<Item = &'a str>) {
    for chunk in chunks {
        text.push_str(chunk);
        if text.len() > MAX_FAILURE_JOB_LOG_BYTES {
            let keep_from = text.len().saturating_sub(MAX_FAILURE_JOB_LOG_BYTES);
            let keep_from = text.char_indices().map(|(idx, _)| idx).find(|idx| *idx >= keep_from).unwrap_or(text.len());
            text.drain(..keep_from);
        }
    }
}

/// Print a compact summary of pipeline stages and jobs.
fn print_pipeline_summary(status: &CiGetStatusResponse) {
    debug_assert!(status.stages.iter().all(|stage| !stage.name.is_empty()), "stages should have names");
    let icon = |s: &str| match s {
        "succeeded" | CI_STATUS_SUCCESS => "✅",
        CI_STATUS_FAILED | CI_STATUS_CHECKOUT_FAILED => "❌",
        CI_STATUS_CANCELLED => "⏹️",
        CI_STATUS_RUNNING => "🔄",
        CI_STATUS_PENDING => "⏳",
        _ => "❓",
    };

    for stage in &status.stages {
        info!("  {} {}", icon(&stage.status), stage.name);
        for job in &stage.jobs {
            debug_assert!(!job.name.is_empty(), "jobs should have names");
            if let Some(ref error) = job.error {
                info!("      {} {} — {}", icon(&job.status), job.name, error);
            } else {
                info!("      {} {}", icon(&job.status), job.name);
            }
        }
    }
}

fn format_pipeline_summary(status: &CiGetStatusResponse) -> String {
    let mut summary = String::new();
    if let Some(ref pipeline_status) = status.status {
        summary.push_str(&format!("pipeline status: {pipeline_status}\n"));
    }
    for stage in &status.stages {
        summary.push_str(&format!("stage {}: {}\n", stage.name, stage.status));
        for job in &stage.jobs {
            if let Some(ref error) = job.error {
                summary.push_str(&format!("  job {}: {} — {}\n", job.name, job.status, error));
            } else {
                summary.push_str(&format!("  job {}: {}\n", job.name, job.status));
            }
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use aspen_client_api::CI_STATUS_CANCELLED;
    use aspen_client_api::CI_STATUS_CHECKING_OUT;
    use aspen_client_api::CI_STATUS_CHECKOUT_FAILED;
    use aspen_client_api::CI_STATUS_FAILED;
    use aspen_client_api::CI_STATUS_PENDING;
    use aspen_client_api::CI_STATUS_SUCCESS;
    use aspen_client_api::CiJobInfo;
    use aspen_client_api::CiStageInfo;

    use super::*;

    #[test]
    fn classifies_direct_route_loss_rpc_errors() {
        assert_eq!(
            ci_status_for_rpc_error("failed to connect to peer: Address Lookup failed: No address lookup configured"),
            DIRECT_ROUTE_LOSS_STATUS
        );
        assert_eq!(
            ci_status_for_rpc_error(
                "direct-only Iroh client has no direct address for bootstrap peer(s) abc; ticket/bootstrap address was not registered for later RPCs"
            ),
            DIRECT_ROUTE_LOSS_STATUS
        );
        assert_eq!(ci_status_for_rpc_error("connection refused"), "status_rpc_failed");
    }

    #[test]
    fn dogfood_ci_terminal_status_contract_matches_client_api() {
        assert!(is_success_status(CI_STATUS_SUCCESS));
        assert!(is_success_status("succeeded"));

        for label in CI_TERMINAL_STATUS_LABELS {
            if *label == CI_STATUS_SUCCESS {
                assert!(!is_terminal_failure_status(label), "success is terminal but not a failure");
            } else {
                assert!(is_terminal_failure_status(label), "dogfood must stop on terminal failure `{label}`");
            }
        }

        for label in [CI_STATUS_CHECKING_OUT, CI_STATUS_PENDING] {
            assert!(!is_success_status(label));
            assert!(!is_terminal_failure_status(label));
        }
    }

    #[test]
    fn checkout_failed_summary_is_operator_visible() {
        let status = CiGetStatusResponse {
            was_found: true,
            run_id: Some("run-1".to_string()),
            repo_id: Some("repo-1".to_string()),
            ref_name: Some("refs/heads/main".to_string()),
            commit_hash: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            status: Some(CI_STATUS_CHECKOUT_FAILED.to_string()),
            stages: vec![CiStageInfo {
                name: "checkout".to_string(),
                status: CI_STATUS_CHECKOUT_FAILED.to_string(),
                jobs: vec![CiJobInfo {
                    id: "job-1".to_string(),
                    name: "checkout-source".to_string(),
                    status: CI_STATUS_CHECKOUT_FAILED.to_string(),
                    started_at_ms: Some(1),
                    ended_at_ms: Some(2),
                    error: Some("source checkout failed".to_string()),
                }],
            }],
            created_at_ms: Some(1),
            completed_at_ms: Some(2),
            error: Some("checkout failed".to_string()),
        };

        let summary = format_pipeline_summary(&status);
        assert!(summary.contains("pipeline status: checkout_failed"));
        assert!(summary.contains("stage checkout: checkout_failed"));
        assert!(summary.contains("job checkout-source: checkout_failed — source checkout failed"));
        assert!(!summary.contains("aspen://"));
    }

    #[test]
    fn running_job_log_tail_is_bounded_in_timeout_summary() {
        let mut summary = String::new();
        let job = CiJobInfo {
            id: "job-1".to_string(),
            name: "build-cli".to_string(),
            status: CI_STATUS_RUNNING.to_string(),
            started_at_ms: Some(1),
            ended_at_ms: None,
            error: None,
        };
        let log_text = format!(
            "{}ASPEN_CI_COMMAND_PROGRESS phase=command_running job_id=job-1 elapsed_secs=3600\n",
            "x".repeat(5000)
        );

        append_running_job_log_summary(&mut summary, "build", &job, &log_text);

        assert!(summary.contains("--- Running job diagnostics: build-cli (stage: build) ---"));
        assert!(summary.contains("job_id=job-1"));
        assert!(summary.contains("job_status=running"));
        assert!(summary.contains("latest_phase=command_running"));
        assert!(summary.contains("missing_phase=command_timeout"));
        assert!(summary.contains("--- Running job log tail: build-cli (stage: build) ---"));
        assert!(summary.contains("truncated to last 4096 bytes"));
        assert!(summary.contains("ASPEN_CI_COMMAND_PROGRESS phase=command_running"));
        assert!(!summary.contains(&"x".repeat(5000)));
    }

    #[test]
    fn running_job_phase_summary_reports_missing_first_marker_when_silent() {
        let mut summary = String::new();
        let job = CiJobInfo {
            id: "job-1".to_string(),
            name: "build-cli".to_string(),
            status: CI_STATUS_RUNNING.to_string(),
            started_at_ms: Some(1),
            ended_at_ms: None,
            error: None,
        };

        append_running_job_log_summary(&mut summary, "build", &job, "");

        assert!(summary.contains("latest_phase=none"));
        assert!(summary.contains("missing_phase=job_spec_parse_done"));
        assert!(!summary.contains("--- Running job log tail:"));
    }

    #[test]
    fn running_job_phase_summary_handles_executor_enter_without_regressing_to_transform() {
        let mut summary = String::new();
        let job = CiJobInfo {
            id: "job-1".to_string(),
            name: "build-cli".to_string(),
            status: CI_STATUS_RUNNING.to_string(),
            started_at_ms: Some(1),
            ended_at_ms: None,
            error: None,
        };
        let log_text = "ASPEN_CI_COMMAND_PROGRESS phase=executor_enter job_id=job-1\n";

        append_running_job_log_summary(&mut summary, "build", &job, log_text);

        assert!(summary.contains("latest_phase=executor_enter"));
        assert!(summary.contains("missing_phase=local_executor_execute_enter"));
        assert!(!summary.contains("missing_phase=nix_payload_transformed"));
    }

    #[test]
    fn running_job_timeout_summary_redacts_sensitive_log_tail() {
        let mut summary = String::new();
        let secret_marker = "synthetic-dogfood-ticket-marker-abcdef";
        let job = CiJobInfo {
            id: "job-1".to_string(),
            name: "build-cli".to_string(),
            status: CI_STATUS_RUNNING.to_string(),
            started_at_ms: Some(1),
            ended_at_ms: None,
            error: None,
        };
        let log_text = format!(
            "remote aspen://{secret_marker}/repo --iroh-secret-key host-secret\n\
             ASPEN_CI_COMMAND_PROGRESS phase=command_started job_id=job-1 command=nix args_count=4 timeout_secs=1800\n\
             --cluster-ticket={secret_marker} --password hunter2\n"
        );

        append_running_job_log_summary(&mut summary, "build", &job, &log_text);

        assert!(summary.contains("latest_phase=command_started"));
        assert!(summary.contains("missing_phase=command_running"));
        assert!(summary.contains("aspen://<cluster-ticket>/repo"));
        assert!(summary.contains("--iroh-secret-key [REDACTED]"));
        assert!(summary.contains("--cluster-ticket=[REDACTED]"));
        assert!(summary.contains("--password [REDACTED]"));
        assert!(!summary.contains(secret_marker));
        assert!(!summary.contains("host-secret"));
        assert!(!summary.contains("hunter2"));
    }

    #[test]
    fn failure_log_collection_keeps_bounded_tail() {
        let old_prefix = "old-progress\n".repeat(40_000);
        let final_error = "ASPEN_CI_COMMAND_PROGRESS phase=command_execute_returned job_id=job-1\nactual final error\n";
        let mut text = String::new();

        append_bounded_log_chunks(&mut text, [old_prefix.as_str(), final_error]);

        assert!(text.len() <= MAX_FAILURE_JOB_LOG_BYTES);
        assert!(text.len() < old_prefix.len());
        assert!(text.contains("actual final error"));
        assert!(text.contains("phase=command_execute_returned"));
    }

    #[test]
    fn running_status_accepts_stable_and_legacy_labels() {
        assert!(is_running_status(CI_STATUS_RUNNING));
        assert!(is_running_status("running"));
        assert!(!is_running_status(CI_STATUS_PENDING));
    }

    #[test]
    fn failed_and_cancelled_remain_terminal_failures() {
        assert!(is_terminal_failure_status(CI_STATUS_FAILED));
        assert!(is_terminal_failure_status(CI_STATUS_CANCELLED));
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

async fn connect(ticket: &str) -> DogfoodResult<AspenClient> {
    AspenClient::connect_direct(ticket, Duration::from_secs(30), None).await.map_err(|e| {
        crate::error::DogfoodError::ClientRpc {
            operation: "connect".to_string(),
            target: crate::cluster::ticket_preview(ticket),
            source: e,
        }
    })
}

async fn send(
    client: &AspenClient,
    request: ClientRpcRequest,
    context: RpcContext<'_>,
) -> DogfoodResult<ClientRpcResponse> {
    let response =
        tokio::time::timeout(Duration::from_secs(CI_RPC_TIMEOUT_SECS), client.send(request))
            .await
            .map_err(|_| crate::error::DogfoodError::Timeout {
                operation: context.operation.to_string(),
                timeout_secs: CI_RPC_TIMEOUT_SECS,
            })?;

    response.map_err(|e| crate::error::DogfoodError::ClientRpc {
        operation: context.operation.to_string(),
        target: crate::cluster::ticket_preview(context.ticket),
        source: e,
    })
}
