//! Worker management handlers.
//!
//! Handles worker registration, deregistration, heartbeats, and status queries.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::WorkerDeregisterResultResponse;
use aspen_client_api::WorkerHeartbeatResultResponse;
use aspen_client_api::WorkerRegisterResultResponse;
use aspen_client_api::WorkerStatusResultResponse;
use aspen_core::KeyValueStore;
use tracing::debug;
use tracing::info;
use tracing::warn;

pub(crate) async fn handle_worker_status(
    worker_service: Option<&std::sync::Arc<aspen_cluster::worker_service::WorkerService>>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Getting worker status");

    let Some(service) = worker_service else {
        return Ok(ClientRpcResponse::WorkerStatusResult(WorkerStatusResultResponse {
            workers: vec![],
            total_workers: 0,
            idle_workers: 0,
            busy_workers: 0,
            offline_workers: 0,
            total_capacity: 0,
            used_capacity: 0,
            error: Some("Worker service not available".to_string()),
        }));
    };

    // Get stats and worker info from the service
    let stats = service.get_stats().await;
    let worker_info = service.get_worker_info().await;

    // Convert aspen_jobs::WorkerInfo to aspen_client_api::WorkerInfo
    let workers: Vec<aspen_client_api::WorkerInfo> = worker_info
        .into_iter()
        .map(|w| {
            let status = match w.status {
                aspen_jobs::WorkerStatus::Starting => "starting",
                aspen_jobs::WorkerStatus::Idle => "idle",
                aspen_jobs::WorkerStatus::Processing => "busy",
                aspen_jobs::WorkerStatus::Stopping => "stopping",
                aspen_jobs::WorkerStatus::Stopped => "offline",
                aspen_jobs::WorkerStatus::Failed(_) => "failed",
            };
            aspen_client_api::WorkerInfo {
                worker_id: w.id,
                status: status.to_string(),
                capabilities: w.job_types,
                capacity: 1, // Each worker has capacity of 1 concurrent job by default
                active_jobs: if w.current_job.is_some() { 1 } else { 0 },
                active_job_ids: w.current_job.into_iter().collect(),
                last_heartbeat: w.last_heartbeat.to_rfc3339(),
                total_processed: w.jobs_processed,
                total_failed: w.jobs_failed,
            }
        })
        .collect();

    Ok(ClientRpcResponse::WorkerStatusResult(WorkerStatusResultResponse {
        workers,
        total_workers: stats.total_workers as u32,
        idle_workers: stats.idle_workers as u32,
        busy_workers: stats.processing_workers as u32,
        offline_workers: stats.failed_workers as u32,
        total_capacity: stats.total_workers as u32, // 1 job per worker
        used_capacity: stats.processing_workers as u32,
        error: None,
    }))
}

pub(crate) async fn handle_worker_register(
    coordinator: Option<&std::sync::Arc<aspen_coordination::DistributedWorkerCoordinator<dyn KeyValueStore>>>,
    node_id: u64,
    worker_id: String,
    capabilities: Vec<String>,
    capacity: u32,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Registering worker: {} with capacity {}", worker_id, capacity);

    let Some(coordinator) = coordinator else {
        return Ok(ClientRpcResponse::WorkerRegisterResult(WorkerRegisterResultResponse {
            success: false,
            worker_token: None,
            error: Some("Worker coordinator not available".to_string()),
        }));
    };

    // Create WorkerInfo for the external worker
    let now_ms = aspen_coordination::now_unix_ms();
    let worker_info = aspen_coordination::WorkerInfo {
        worker_id: worker_id.clone(),
        node_id: format!("external-{}", node_id),
        peer_id: None,
        capabilities,
        load: 0.0,
        active_jobs: 0,
        max_concurrent: capacity as u32,
        queue_depth: 0,
        health: aspen_coordination::HealthStatus::Healthy,
        tags: vec![],
        last_heartbeat_ms: now_ms,
        started_at_ms: now_ms,
        total_processed: 0,
        total_failed: 0,
        avg_processing_time_ms: 0,
        groups: std::collections::HashSet::new(),
    };

    match coordinator.register_worker(worker_info).await {
        Ok(()) => {
            info!(worker_id = %worker_id, "external worker registered");
            // Generate a simple token (worker_id + timestamp for uniqueness)
            let token = format!("{}:{}", worker_id, now_ms);
            Ok(ClientRpcResponse::WorkerRegisterResult(WorkerRegisterResultResponse {
                success: true,
                worker_token: Some(token),
                error: None,
            }))
        }
        Err(e) => {
            warn!(worker_id = %worker_id, error = %e, "failed to register worker");
            Ok(ClientRpcResponse::WorkerRegisterResult(WorkerRegisterResultResponse {
                success: false,
                worker_token: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

pub(crate) async fn handle_worker_heartbeat(
    coordinator: Option<&std::sync::Arc<aspen_coordination::DistributedWorkerCoordinator<dyn KeyValueStore>>>,
    worker_id: String,
    active_jobs: Vec<String>,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Worker heartbeat: {} with {} active jobs", worker_id, active_jobs.len());

    let Some(coordinator) = coordinator else {
        return Ok(ClientRpcResponse::WorkerHeartbeatResult(WorkerHeartbeatResultResponse {
            success: false,
            jobs_to_process: vec![],
            error: Some("Worker coordinator not available".to_string()),
        }));
    };

    // Calculate load based on active jobs (assume max_concurrent from registration)
    // Note: In production, we'd track max_concurrent per worker, but for now estimate
    let active_count = active_jobs.len() as u32;
    let load = if active_count == 0 {
        0.0
    } else {
        (active_count as f32 / 10.0).min(1.0)
    };

    let stats = aspen_coordination::WorkerStats {
        load,
        active_jobs: active_count,
        queue_depth: 0,
        total_processed: 0, // These would be tracked if we had persistent state
        total_failed: 0,
        avg_processing_time_ms: 0,
        health: aspen_coordination::HealthStatus::Healthy,
    };

    match coordinator.heartbeat(&worker_id, stats).await {
        Ok(()) => {
            debug!(worker_id = %worker_id, "heartbeat processed");
            // Note: jobs_to_process would be filled by job assignment logic
            // For now, external workers poll for jobs separately
            Ok(ClientRpcResponse::WorkerHeartbeatResult(WorkerHeartbeatResultResponse {
                success: true,
                jobs_to_process: vec![],
                error: None,
            }))
        }
        Err(e) => {
            warn!(worker_id = %worker_id, error = %e, "failed to process heartbeat");
            Ok(ClientRpcResponse::WorkerHeartbeatResult(WorkerHeartbeatResultResponse {
                success: false,
                jobs_to_process: vec![],
                error: Some(e.to_string()),
            }))
        }
    }
}

pub(crate) async fn handle_worker_deregister(
    coordinator: Option<&std::sync::Arc<aspen_coordination::DistributedWorkerCoordinator<dyn KeyValueStore>>>,
    worker_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    debug!("Deregistering worker: {}", worker_id);

    let Some(coordinator) = coordinator else {
        return Ok(ClientRpcResponse::WorkerDeregisterResult(WorkerDeregisterResultResponse {
            success: false,
            error: Some("Worker coordinator not available".to_string()),
        }));
    };

    match coordinator.deregister_worker(&worker_id).await {
        Ok(()) => {
            info!(worker_id = %worker_id, "external worker deregistered");
            Ok(ClientRpcResponse::WorkerDeregisterResult(WorkerDeregisterResultResponse {
                success: true,
                error: None,
            }))
        }
        Err(e) => {
            warn!(worker_id = %worker_id, error = %e, "failed to deregister worker");
            Ok(ClientRpcResponse::WorkerDeregisterResult(WorkerDeregisterResultResponse {
                success: false,
                error: Some(e.to_string()),
            }))
        }
    }
}
