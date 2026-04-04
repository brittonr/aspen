//! CI pipeline operations — trigger, poll status, fetch logs.

use std::time::Duration;

use aspen_client::AspenClient;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use tracing::info;
use tracing::warn;

use crate::error::CiPipelineSnafu;
use crate::error::DogfoodResult;
use crate::error::TimeoutSnafu;

/// Wait for a CI pipeline to complete on the given repo.
///
/// First polls `CiListRuns` for an auto-triggered pipeline (up to 120s).
/// If none is found, triggers one via `CiTriggerPipeline`.
/// Then polls `CiGetStatus` with exponential backoff until terminal state.
///
/// Returns the run_id on success.
pub async fn wait_for_pipeline(ticket: &str, repo_name: &str, timeout_secs: u64) -> DogfoodResult<String> {
    let client = connect(ticket).await?;

    // We need the repo_id. The repo_name is what we used to create it,
    // but CiTriggerPipeline takes repo_id. Try to resolve via ForgeListRepos.
    let repo_id = resolve_repo_id(&client, repo_name, ticket).await?;

    // Phase 1: look for an auto-triggered pipeline (push-triggered)
    info!("  looking for auto-triggered pipeline...");
    let run_id = match find_recent_run(&client, &repo_id, ticket, Duration::from_secs(120)).await {
        Some(id) => {
            info!("  found pipeline: {id}");
            id
        }
        None => {
            // Phase 2: manually trigger
            warn!("  no auto-triggered pipeline, triggering manually...");
            trigger_pipeline(&client, &repo_id, ticket).await?
        }
    };

    // Phase 3: poll until terminal state
    info!("  waiting for pipeline {run_id}...");
    poll_pipeline(&client, &run_id, ticket, timeout_secs).await?;

    Ok(run_id)
}

/// Resolve a repo name to a repo ID via `ForgeListRepos`.
async fn resolve_repo_id(client: &AspenClient, repo_name: &str, ticket: &str) -> DogfoodResult<String> {
    let resp = send(
        client,
        ClientRpcRequest::ForgeListRepos {
            limit: Some(100),
            offset: None,
        },
        "ForgeListRepos",
        ticket,
    )
    .await?;

    match resp {
        ClientRpcResponse::ForgeRepoListResult(list) => {
            for repo in &list.repos {
                if repo.name == repo_name {
                    return Ok(repo.id.clone());
                }
            }
            crate::error::ForgeSnafu {
                operation: "resolve repo id",
                reason: format!("repo '{repo_name}' not found"),
            }
            .fail()
        }
        other => crate::error::ForgeSnafu {
            operation: "resolve repo id",
            reason: format!("unexpected response: {other:?}"),
        }
        .fail(),
    }
}

/// Poll `CiListRuns` looking for a run associated with this repo.
async fn find_recent_run(client: &AspenClient, repo_id: &str, ticket: &str, timeout: Duration) -> Option<String> {
    let start = tokio::time::Instant::now();

    loop {
        if let Ok(resp) = send(
            client,
            ClientRpcRequest::CiListRuns {
                repo_id: Some(repo_id.to_string()),
                status: None,
                limit: Some(5),
            },
            "CiListRuns",
            ticket,
        )
        .await
        {
            if let ClientRpcResponse::CiListRunsResult(list) = resp {
                if let Some(run) = list.runs.first() {
                    return Some(run.run_id.clone());
                }
            }
        }

        if start.elapsed() > timeout {
            return None;
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

/// Manually trigger a pipeline and return the run_id.
async fn trigger_pipeline(client: &AspenClient, repo_id: &str, ticket: &str) -> DogfoodResult<String> {
    let resp = send(
        client,
        ClientRpcRequest::CiTriggerPipeline {
            repo_id: repo_id.to_string(),
            ref_name: "refs/heads/main".to_string(),
            commit_hash: None,
        },
        "CiTriggerPipeline",
        ticket,
    )
    .await?;

    match resp {
        ClientRpcResponse::CiTriggerPipelineResult(r) if r.is_success => {
            let run_id = r.run_id.unwrap_or_else(|| "unknown".to_string());
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
async fn poll_pipeline(client: &AspenClient, run_id: &str, ticket: &str, timeout_secs: u64) -> DogfoodResult<()> {
    let start = tokio::time::Instant::now();
    let mut delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(10);
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        let resp = send(
            client,
            ClientRpcRequest::CiGetStatus {
                run_id: run_id.to_string(),
            },
            "CiGetStatus",
            ticket,
        )
        .await?;

        if let ClientRpcResponse::CiGetStatusResult(status) = &resp {
            if let Some(pipeline_status) = &status.status {
                match pipeline_status.as_str() {
                    "succeeded" | "success" => {
                        print_pipeline_summary(status);
                        return Ok(());
                    }
                    "failed" | "cancelled" => {
                        print_pipeline_summary(status);

                        // Fetch last logs on failure
                        let log_detail = fetch_failure_logs(client, run_id, status, ticket).await;

                        return CiPipelineSnafu {
                            run_id,
                            status: pipeline_status,
                            detail: log_detail,
                        }
                        .fail();
                    }
                    _ => {
                        // Still running
                    }
                }
            }
        }

        if start.elapsed() > timeout {
            // Fetch last 50 log lines on timeout
            return TimeoutSnafu {
                operation: format!("CI pipeline {run_id}"),
                timeout_secs,
            }
            .fail();
        }

        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(max_delay);
    }
}

/// Fetch logs from failed jobs for error reporting.
///
/// Tries `CiGetJobLogs` first (streamed build output). If no log chunks
/// are available, falls back to the job's `error` field from `CiGetStatus`
/// which contains the `JobResult::failure` message.
async fn fetch_failure_logs(
    client: &AspenClient,
    run_id: &str,
    status: &aspen_client_api::CiGetStatusResponse,
    ticket: &str,
) -> String {
    // Find the first failed job
    for stage in &status.stages {
        for job in &stage.jobs {
            if job.status == "failed" {
                // Try to fetch streamed logs from KV store
                let log_text = fetch_job_log_chunks(client, run_id, &job.id, ticket).await;

                if !log_text.is_empty() {
                    return format!("--- Failed job: {} (stage: {}) ---\n{}", job.name, stage.name, log_text);
                }

                // Fall back to job error message from CiGetStatus
                if let Some(ref error) = job.error {
                    return format!("--- Failed job: {} (stage: {}) ---\n{}", job.name, stage.name, error);
                }

                // Last resort: just the job name
                return format!(
                    "--- Failed job: {} (stage: {}) ---\nNo error details available. \
                     The job may have failed before any output was produced.",
                    job.name, stage.name
                );
            }
        }
    }

    "no failed jobs found in pipeline status".to_string()
}

/// Fetch log chunks for a specific job from the KV log store.
async fn fetch_job_log_chunks(client: &AspenClient, run_id: &str, job_id: &str, ticket: &str) -> String {
    let Ok(resp) = send(
        client,
        ClientRpcRequest::CiGetJobLogs {
            run_id: run_id.to_string(),
            job_id: job_id.to_string(),
            start_index: 0,
            limit: Some(200),
        },
        "CiGetJobLogs",
        ticket,
    )
    .await
    else {
        return String::new();
    };

    if let ClientRpcResponse::CiGetJobLogsResult(logs) = resp {
        let text: String = logs.chunks.iter().map(|c| c.content.as_str()).collect::<Vec<_>>().join("");
        return text;
    }

    String::new()
}

/// Print a compact summary of pipeline stages and jobs.
fn print_pipeline_summary(status: &aspen_client_api::CiGetStatusResponse) {
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
    operation: &str,
    ticket: &str,
) -> DogfoodResult<ClientRpcResponse> {
    client.send(request).await.map_err(|e| crate::error::DogfoodError::ClientRpc {
        operation: operation.to_string(),
        target: crate::cluster::ticket_preview(ticket),
        source: e,
    })
}
