//! Work queue API handlers (REST API for workers and control plane operations)

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::state::AppState;
use crate::domain::{JobSubmission, JobStatus};

/// Hiqlite health check endpoint
///
/// This endpoint now goes through the domain layer (HealthService) instead of
/// accessing infrastructure directly, maintaining proper architectural boundaries.
pub async fn hiqlite_health(State(state): State<AppState>) -> impl IntoResponse {
    let health_service = state.services().health();

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
    pub url: String,
}

/// Publish a new work item to the queue
pub async fn queue_publish(
    State(state): State<AppState>,
    Json(req): Json<PublishWorkRequest>,
) -> impl IntoResponse {
    let job_service = state.services().job_lifecycle();

    let submission = JobSubmission { url: req.url };

    match job_service.submit_job(submission).await {
        Ok(job_id) => (StatusCode::OK, format!("Job {} published", job_id)).into_response(),
        Err(e) => {
            // Return 400 BAD_REQUEST for validation errors
            if e.to_string().contains("cannot be empty") {
                (StatusCode::BAD_REQUEST, format!("Validation error: {}", e)).into_response()
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to publish work: {}", e)).into_response()
            }
        }
    }
}

/// Claim an available work item from the queue
pub async fn queue_claim(State(state): State<AppState>) -> impl IntoResponse {
    let job_service = state.services().job_lifecycle();

    match job_service.claim_work().await {
        Ok(Some(work_item)) => Json(work_item).into_response(),
        Ok(None) => (StatusCode::NO_CONTENT, "No work available").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to claim work: {}", e),
        )
            .into_response(),
    }
}

/// List all work items in the queue
pub async fn queue_list(State(state): State<AppState>) -> impl IntoResponse {
    let job_service = state.services().job_lifecycle();

    match job_service.list_all_work().await {
        Ok(work_items) => Json(work_items).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list work: {}", e),
        )
            .into_response(),
    }
}

/// Get queue statistics
pub async fn queue_stats(State(state): State<AppState>) -> impl IntoResponse {
    let job_service = state.services().job_lifecycle();
    let stats = job_service.get_queue_stats().await;
    Json(stats).into_response()
}

/// Status update request
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: JobStatus,
}

/// Update the status of a specific work item
pub async fn queue_update_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Json(req): Json<UpdateStatusRequest>,
) -> impl IntoResponse {
    let job_service = state.services().job_lifecycle();

    match job_service.update_work_status(&job_id, req.status).await {
        Ok(()) => (StatusCode::OK, "Status updated").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update status: {}", e),
        )
            .into_response(),
    }
}
