//! Deploy and verify operations — trigger rolling deploy, verify artifact.

use std::time::Duration;

use aspen_client::AspenClient;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use tracing::info;
use tracing::warn;

use crate::error::DeployFailedSnafu;
use crate::error::DogfoodResult;
use crate::error::TimeoutSnafu;

/// Trigger a rolling deployment and wait for completion.
///
/// Finds the latest successful CI pipeline, extracts the Nix store path
/// from the build-node job result, and issues `ClusterDeploy`.
pub async fn trigger_and_wait(ticket: &str, run_id: Option<&str>) -> DogfoodResult<()> {
    let client = connect(ticket).await?;
    let result = async {
        // Find the store path from the build run dogfood just observed. Falling
        // back to the latest successful run keeps the standalone `deploy`
        // subcommand usable, but the full path should not race status indexes.
        let store_path = find_build_artifact(&client, ticket, run_id).await?;
        info!("  artifact: {store_path}");

        // Trigger deploy
        info!("  starting rolling deployment...");
        let resp = send(
            &client,
            ClientRpcRequest::ClusterDeploy {
                artifact: store_path.clone(),
                strategy: "rolling".to_string(),
                max_concurrent: 1,
                health_timeout_secs: 120,
                expected_binary: None,
            },
            "ClusterDeploy",
            ticket,
        )
        .await?;

        let deploy_id = match resp {
            ClientRpcResponse::ClusterDeployResult(r) if r.is_accepted => {
                let id = r.deploy_id.unwrap_or_else(|| "unknown".to_string());
                info!("  deploy accepted: {id}");
                id
            }
            ClientRpcResponse::ClusterDeployResult(r) => {
                return DeployFailedSnafu {
                    reason: r.error.unwrap_or_else(|| "deploy rejected".to_string()),
                }
                .fail();
            }
            other => {
                return DeployFailedSnafu {
                    reason: format!("unexpected response: {other:?}"),
                }
                .fail();
            }
        };

        // Poll deploy status until terminal
        poll_deploy_status(&client, &deploy_id, ticket, 1200).await
    }
    .await;
    client.shutdown().await;
    result
}

/// Verify the deployment by checking cluster health and node metrics.
pub async fn verify_deployment(ticket: &str) -> DogfoodResult<()> {
    let client = connect(ticket).await?;
    let result = async {
        // Check deploy status is "completed"
        let resp = send(&client, ClientRpcRequest::ClusterDeployStatus, "ClusterDeployStatus", ticket).await?;

        if let ClientRpcResponse::ClusterDeployStatusResult(status) = &resp {
            match status.status.as_deref() {
                Some("completed") => {
                    info!("  deploy status: completed");
                    for node in &status.nodes {
                        info!("    node {}: {}", node.node_id, node.status);
                    }
                }
                Some(other) => {
                    warn!("  deploy status: {other}");
                }
                None if !status.is_found => {
                    info!("  no active deployment found (may have already completed)");
                }
                None => {}
            }
        }

        // Health check the cluster
        info!("  verifying cluster health...");
        let health_resp = send(&client, ClientRpcRequest::GetHealth, "GetHealth", ticket).await?;

        match health_resp {
            ClientRpcResponse::Health(h) if h.status == "healthy" => {
                info!("  node {} healthy, uptime {}s", h.node_id, h.uptime_seconds);
                Ok(())
            }
            ClientRpcResponse::Health(h) => DeployFailedSnafu {
                reason: format!("node {} reports status '{}'", h.node_id, h.status),
            }
            .fail(),
            other => DeployFailedSnafu {
                reason: format!("unexpected health response: {other:?}"),
            }
            .fail(),
        }
    }
    .await;
    client.shutdown().await;
    result
}

// ── Internals ────────────────────────────────────────────────────────

/// Find the Nix store path from the latest successful CI build.
async fn find_build_artifact(client: &AspenClient, ticket: &str, run_id: Option<&str>) -> DogfoodResult<String> {
    let run_id = match run_id {
        Some(run_id) => run_id.to_string(),
        None => find_latest_successful_run_id(client, ticket).await?,
    };

    // Get pipeline status to find the build-node job
    let status_resp =
        send(client, ClientRpcRequest::CiGetStatus { run_id: run_id.clone() }, "CiGetStatus", ticket).await?;

    let job_id = match &status_resp {
        ClientRpcResponse::CiGetStatusResult(status) => {
            find_build_node_job_id(status).ok_or_else(|| crate::error::DogfoodError::DeployFailed {
                reason: format!("no build-node job found in pipeline {run_id}"),
            })?
        }
        other => {
            return DeployFailedSnafu {
                reason: format!("unexpected CiGetStatus response: {other:?}"),
            }
            .fail();
        }
    };

    // Read job result from KV to get the output store path
    let kv_key = format!("__jobs:{job_id}");
    let kv_resp = send(client, ClientRpcRequest::ReadKey { key: kv_key.clone() }, "ReadKey", ticket).await?;

    let store_path = match kv_resp {
        ClientRpcResponse::ReadResult(read) if read.was_found => {
            let bytes = read.value.ok_or_else(|| crate::error::DogfoodError::DeployFailed {
                reason: format!("job result key '{kv_key}' has no value"),
            })?;
            let value_str = String::from_utf8(bytes).map_err(|_| crate::error::DogfoodError::DeployFailed {
                reason: "job result is not valid UTF-8".to_string(),
            })?;
            extract_store_path(&value_str)?
        }
        ClientRpcResponse::ReadResult(_) => {
            return DeployFailedSnafu {
                reason: format!("job result key '{kv_key}' not found in KV"),
            }
            .fail();
        }
        other => {
            return DeployFailedSnafu {
                reason: format!("unexpected ReadKey response: {other:?}"),
            }
            .fail();
        }
    };

    Ok(store_path)
}

async fn find_latest_successful_run_id(client: &AspenClient, ticket: &str) -> DogfoodResult<String> {
    let resp = send(
        client,
        ClientRpcRequest::CiListRuns {
            repo_id: None,
            status: Some("succeeded".to_string()),
            limit: Some(1),
        },
        "CiListRuns",
        ticket,
    )
    .await?;

    match resp {
        ClientRpcResponse::CiListRunsResult(list) => {
            list.runs.first().map(|r| r.run_id.clone()).ok_or_else(|| crate::error::DogfoodError::DeployFailed {
                reason: "no successful pipeline runs found".to_string(),
            })
        }
        other => DeployFailedSnafu {
            reason: format!("unexpected CiListRuns response: {other:?}"),
        }
        .fail(),
    }
}

/// Extract the build-node job ID from pipeline stages.
fn find_build_node_job_id(status: &aspen_client_api::CiGetStatusResponse) -> Option<String> {
    for stage in &status.stages {
        if stage.name == "build" {
            for job in &stage.jobs {
                if job.name == "build-node" {
                    return Some(job.id.clone());
                }
            }
        }
    }
    // Fallback: first job in the build stage
    for stage in &status.stages {
        if stage.name == "build"
            && let Some(job) = stage.jobs.first()
        {
            return Some(job.id.clone());
        }
    }
    None
}

/// Extract the Nix store path from a serialized job result JSON.
fn extract_store_path(value: &str) -> DogfoodResult<String> {
    // The value is a JSON string containing job data with result.Success.data.output_paths
    let job: serde_json::Value = serde_json::from_str(value).map_err(|_| crate::error::DogfoodError::DeployFailed {
        reason: "failed to parse job result JSON".to_string(),
    })?;

    // Try output_paths first
    if let Some(paths) = job.pointer("/result/Success/data/output_paths").and_then(|v| v.as_array())
        && let Some(path) = paths.first().and_then(|v| v.as_str())
    {
        return Ok(path.to_string());
    }

    // Fallback: uploaded_store_paths
    if let Some(paths) = job.pointer("/result/Success/data/uploaded_store_paths").and_then(|v| v.as_array()) {
        for sp in paths {
            if let Some(store_path) = sp.get("store_path").and_then(|v| v.as_str()) {
                return Ok(store_path.to_string());
            }
        }
    }

    DeployFailedSnafu {
        reason: "no output store path found in job result".to_string(),
    }
    .fail()
}

/// Poll deploy status until terminal state.
async fn poll_deploy_status(
    client: &AspenClient,
    deploy_id: &str,
    ticket: &str,
    timeout_secs: u64,
) -> DogfoodResult<()> {
    let start = tokio::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let mut delay = Duration::from_secs(2);
    let max_delay = Duration::from_secs(10);

    loop {
        let resp = send(client, ClientRpcRequest::ClusterDeployStatus, "ClusterDeployStatus", ticket).await?;

        if let ClientRpcResponse::ClusterDeployStatusResult(status) = &resp {
            match status.status.as_deref() {
                Some("completed") => {
                    info!("  deployment completed");
                    return Ok(());
                }
                Some("failed") | Some("rolled_back") => {
                    return DeployFailedSnafu {
                        reason: status.error.clone().unwrap_or_else(|| "deployment failed".to_string()),
                    }
                    .fail();
                }
                Some(s) => {
                    info!("  deploy status: {s} (elapsed: {:?})", status.elapsed_ms.map(|ms| format!("{ms}ms")));
                }
                None => {}
            }
        }

        if start.elapsed() > timeout {
            return TimeoutSnafu {
                operation: format!("deployment {deploy_id}"),
                timeout_secs,
            }
            .fail();
        }

        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(max_delay);
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
