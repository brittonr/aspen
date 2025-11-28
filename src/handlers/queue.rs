//! Work queue API handlers (REST API for workers and control plane operations)

#![allow(dead_code)] // Used via wildcard import in router

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::domain::{JobSubmission, JobStatus, HealthService, JobCommandService, JobQueryService};

/// Hiqlite health check endpoint
///
/// This endpoint now goes through the domain layer (HealthService) instead of
/// accessing infrastructure directly, maintaining proper architectural boundaries.
pub async fn hiqlite_health(
    State(health_service): State<Arc<HealthService>>,
) -> impl IntoResponse {

    match health_service.check_database_health().await {
        Ok(health) => Json(health).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Health check failed: {}", e),
        )
            .into_response(),
    }
}

/// Work item publication request
#[derive(Debug, Deserialize)]
pub struct PublishWorkRequest {
    #[serde(flatten)]
    pub payload: serde_json::Value,
}

/// Publish a new work item to the queue
pub async fn queue_publish(
    State(job_commands): State<Arc<JobCommandService>>,
    Json(req): Json<PublishWorkRequest>,
) -> impl IntoResponse {

    let submission = JobSubmission { payload: req.payload };

    match job_commands.submit_job(submission).await {
        Ok(job_id) => (StatusCode::OK, format!("Job {} published", job_id)).into_response(),
        Err(e) => {
            // Return 400 BAD_REQUEST for validation errors
            if e.to_string().contains("cannot be") {
                (StatusCode::BAD_REQUEST, format!("Validation error: {}", e)).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to publish work: {}", e)).into_response()
            }
        }
    }
}

/// Query parameters for claiming work
#[derive(Debug, serde::Deserialize)]
pub struct ClaimWorkQuery {
    /// Worker ID to assign the job to
    pub worker_id: Option<String>,
    /// Worker type for filtering compatible jobs
    pub worker_type: Option<String>,
}

/// Claim an available work item from the queue
///
/// Optional query parameters:
/// - worker_id: Worker ID to assign the job to
/// - worker_type: Worker type for filtering (wasm, firecracker)
pub async fn queue_claim(
    State((job_commands, job_queries)): State<(Arc<JobCommandService>, Arc<JobQueryService>)>,
    axum::extract::Query(query): axum::extract::Query<ClaimWorkQuery>,
) -> impl IntoResponse {

    // Parse worker_type if provided
    let worker_type = match query.worker_type.as_deref() {
        Some(type_str) => {
            match type_str.parse::<crate::domain::types::WorkerType>() {
                Ok(wt) => Some(wt),
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid worker_type: {}", e),
                    ).into_response();
                }
            }
        }
        None => None,
    };

    match job_commands.claim_job(query.worker_id.as_deref(), worker_type).await {
        Ok(Some(job_id)) => {
            // Need to fetch the full job for backward compatibility
            match job_queries.get_job(&job_id).await {
                Ok(Some(job)) => Json(job).into_response(),
                Ok(None) => (StatusCode::NOT_FOUND, "Job not found after claiming").into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get job: {}", e)).into_response(),
            }
        },
        Ok(None) => (StatusCode::NO_CONTENT, "No work available").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to claim work: {}", e),
        )
            .into_response(),
    }
}

/// List all work items in the queue
pub async fn queue_list(
    State(job_queries): State<Arc<JobQueryService>>,
) -> impl IntoResponse {

    match job_queries.list_all_jobs().await {
        Ok(work_items) => Json(work_items).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list work: {}", e),
        )
            .into_response(),
    }
}

/// Get queue statistics
pub async fn queue_stats(
    State(job_queries): State<Arc<JobQueryService>>,
) -> impl IntoResponse {
    let stats = job_queries.get_queue_stats().await;
    Json(stats).into_response()
}

/// Status update request
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: JobStatus,
    pub error_message: Option<String>,
}

/// Update the status of a specific work item
pub async fn queue_update_status(
    State(job_commands): State<Arc<JobCommandService>>,
    Path(job_id): Path<String>,
    Json(req): Json<UpdateStatusRequest>,
) -> impl IntoResponse {

    match job_commands.update_job_status(&job_id, req.status, req.error_message).await {
        Ok(()) => (StatusCode::OK, "Status updated").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update status: {}", e),
        )
            .into_response(),
    }
}
