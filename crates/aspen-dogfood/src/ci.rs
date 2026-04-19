//! CI pipeline operations — trigger, poll status, fetch logs.

use std::time::Duration;

use aspen_client::AspenClient;
use aspen_client_api::CiGetStatusResponse;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use tracing::info;
use tracing::warn;

use crate::error::CiPipelineSnafu;
use crate::error::DogfoodResult;
use crate::error::TimeoutSnafu;

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
        .await?;

        if let ClientRpcResponse::CiGetStatusResult(status) = &resp
            && let Some(pipeline_status) = &status.status
        {
            match pipeline_status.as_str() {
                "succeeded" | "success" => {
                    print_pipeline_summary(status);
                    return Ok(());
                }
                "failed" | "cancelled" => {
                    print_pipeline_summary(status);
                    let log_detail = fetch_failure_logs(client, target, status).await;
                    return CiPipelineSnafu {
                        run_id: target.run_id,
                        status: pipeline_status,
                        detail: log_detail,
                    }
                    .fail();
                }
                _ => {}
            }
        }

        tokio::time::sleep(poll_backoff).await;
        poll_backoff = (poll_backoff * 2).min(max_backoff);
    }

    TimeoutSnafu {
        operation: format!("CI pipeline {}", target.run_id),
        timeout_secs,
    }
    .fail()
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
            if job.status != "failed" {
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

/// Fetch log chunks for a specific job from the KV log store.
async fn fetch_job_log_chunks(client: &AspenClient, target: JobLogTarget<'_>) -> String {
    debug_assert!(!target.run_id.is_empty(), "run id must not be empty");
    debug_assert!(!target.job_id.is_empty(), "job id must not be empty");
    debug_assert!(!target.ticket.is_empty(), "ticket must not be empty");

    let Ok(resp) = send(
        client,
        ClientRpcRequest::CiGetJobLogs {
            run_id: target.run_id.to_string(),
            job_id: target.job_id.to_string(),
            start_index: 0,
            limit: Some(200),
        },
        RpcContext {
            operation: "CiGetJobLogs",
            ticket: target.ticket,
        },
    )
    .await
    else {
        return String::new();
    };

    if let ClientRpcResponse::CiGetJobLogsResult(logs) = resp {
        let text: String = logs.chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join("");
        debug_assert!(text.is_empty() || !logs.chunks.is_empty(), "non-empty log text should come from chunks");
        return text;
    }

    String::new()
}

/// Print a compact summary of pipeline stages and jobs.
fn print_pipeline_summary(status: &CiGetStatusResponse) {
    debug_assert!(status.stages.iter().all(|stage| !stage.name.is_empty()), "stages should have names");
    let icon = |s: &str| match s {
        "succeeded" | "success" => "✅",
        "failed" => "❌",
        "cancelled" => "⏹️",
        "running" => "🔄",
        "pending" => "⏳",
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

// ── Helpers ──────────────────────────────────────────────────────────

async fn connect(ticket: &str) -> DogfoodResult<AspenClient> {
    AspenClient::connect(ticket, Duration::from_secs(10), None).await.map_err(|e| {
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
    client.send(request).await.map_err(|e| crate::error::DogfoodError::ClientRpc {
        operation: context.operation.to_string(),
        target: crate::cluster::ticket_preview(context.ticket),
        source: e,
    })
}
