//! Worker management API handlers (REST API for worker lifecycle operations)

#![allow(dead_code)] // Used via wildcard import in router

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::domain::types::{WorkerRegistration, WorkerHeartbeat, WorkerType};

/// Worker registration request
#[derive(Debug, Deserialize)]
pub struct RegisterWorkerRequest {
    pub worker_type: WorkerType,
    pub endpoint_id: String,
    pub cpu_cores: Option<u32>,
    pub memory_mb: Option<u64>,
    pub metadata: serde_json::Value,
}

/// Worker registration response
#[derive(Debug, Serialize)]
pub struct RegisterWorkerResponse {
    pub worker_id: String,
    pub message: String,
}

/// Register a new worker
///
/// Called by worker binaries on startup to register with the control plane.
/// Generates a unique worker ID and records worker capabilities.
pub async fn worker_register(
    State(state): State<AppState>,
    Json(req): Json<RegisterWorkerRequest>,
) -> impl IntoResponse {
    let worker_service = state.services().worker_management();

    let registration = WorkerRegistration {
        worker_type: req.worker_type,
        endpoint_id: req.endpoint_id,
        cpu_cores: req.cpu_cores,
        memory_mb: req.memory_mb,
        metadata: req.metadata,
    };

    match worker_service.register_worker(registration).await {
        Ok(worker) => {
            let response = RegisterWorkerResponse {
                worker_id: worker.id.clone(),
                message: format!("Worker {} registered successfully", worker.id),
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to register worker: {}", e),
        )
            .into_response(),
    }
}

/// Worker heartbeat request
#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub active_jobs: u32,
    pub cpu_cores: Option<u32>,
    pub memory_mb: Option<u64>,
}

/// Send worker heartbeat
///
/// Called periodically by workers to indicate they're alive and update their status.
pub async fn worker_heartbeat(
    State(state): State<AppState>,
    Path(worker_id): Path<String>,
    Json(req): Json<HeartbeatRequest>,
) -> impl IntoResponse {
    let worker_service = state.services().worker_management();

    let heartbeat = WorkerHeartbeat {
        worker_id: worker_id.clone(),
        active_jobs: req.active_jobs,
        cpu_cores: req.cpu_cores,
        memory_mb: req.memory_mb,
    };

    match worker_service.handle_heartbeat(heartbeat).await {
        Ok(_) => (StatusCode::OK, "Heartbeat recorded").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to record heartbeat: {}", e),
        )
            .into_response(),
    }
}

/// List all workers
///
/// Returns all workers regardless of status.
pub async fn worker_list(State(state): State<AppState>) -> impl IntoResponse {
    let worker_service = state.services().worker_management();

    match worker_service.list_all_workers().await {
        Ok(workers) => Json(workers).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list workers: {}", e),
        )
            .into_response(),
    }
}

/// Get worker details
///
/// Returns information about a specific worker.
pub async fn worker_get(
    State(state): State<AppState>,
    Path(worker_id): Path<String>,
) -> impl IntoResponse {
    let worker_service = state.services().worker_management();

    match worker_service.get_worker(&worker_id).await {
        Ok(Some(worker)) => Json(worker).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, format!("Worker {} not found", worker_id)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get worker: {}", e),
        )
            .into_response(),
    }
}

/// Mark worker as draining (graceful shutdown)
///
/// Sets worker status to Draining, preventing new job assignments
/// while allowing current jobs to complete.
pub async fn worker_drain(
    State(state): State<AppState>,
    Path(worker_id): Path<String>,
) -> impl IntoResponse {
    let worker_service = state.services().worker_management();

    match worker_service.mark_worker_draining(&worker_id).await {
        Ok(_) => (
            StatusCode::OK,
            format!("Worker {} marked as draining", worker_id),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to drain worker: {}", e),
        )
            .into_response(),
    }
}

/// Get worker pool statistics
///
/// Returns aggregate statistics for the entire worker pool.
pub async fn worker_stats(State(state): State<AppState>) -> impl IntoResponse {
    let worker_service = state.services().worker_management();

    match worker_service.get_worker_stats().await {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get worker stats: {}", e),
        )
            .into_response(),
    }
}
