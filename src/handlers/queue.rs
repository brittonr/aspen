//! Work queue API handlers (REST API for workers and control plane operations)

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::state::AppState;
use crate::work_queue;

/// Hiqlite health check endpoint
pub async fn hiqlite_health(State(state): State<AppState>) -> impl IntoResponse {
    match state.infrastructure().hiqlite().health_check().await {
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
    pub job_id: String,
    pub payload: serde_json::Value,
}

/// Publish a new work item to the queue
pub async fn queue_publish(
    State(state): State<AppState>,
    Json(req): Json<PublishWorkRequest>,
) -> impl IntoResponse {
    match state.infrastructure().work_queue().publish_work(req.job_id, req.payload).await {
        Ok(()) => (StatusCode::OK, "Work item published").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to publish work: {}", e),
        )
            .into_response(),
    }
}

/// Claim an available work item from the queue
pub async fn queue_claim(State(state): State<AppState>) -> impl IntoResponse {
    match state.infrastructure().work_queue().claim_work().await {
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
    match state.infrastructure().work_queue().list_work().await {
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
    let stats = state.infrastructure().work_queue().stats().await;
    Json(stats).into_response()
}

/// Status update request
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: work_queue::WorkStatus,
}

/// Update the status of a specific work item
pub async fn queue_update_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Json(req): Json<UpdateStatusRequest>,
) -> impl IntoResponse {
    match state.infrastructure().work_queue().update_status(&job_id, req.status).await {
        Ok(()) => (StatusCode::OK, "Status updated").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update status: {}", e),
        )
            .into_response(),
    }
}
