//! Dashboard handlers for cluster monitoring and job management

#![allow(dead_code)] // Used via wildcard import in router

use askama::Template;
use axum::{
    Form,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::state::AppState;
use crate::domain::{JobSortOrder, JobSubmission};
use crate::views::{ClusterHealthView, QueueStatsView, JobListView, ControlPlaneNodesView, WorkersView, ErrorView};

/// Main dashboard template
#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate;

/// Dashboard main page
pub async fn dashboard() -> impl IntoResponse {
    match DashboardTemplate.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Failed to render dashboard template: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to render dashboard",
            )
                .into_response()
        }
    }
}

/// Dashboard cluster health endpoint (HTMX partial)
pub async fn dashboard_cluster_health(State(state): State<AppState>) -> impl IntoResponse {
    let cluster_service = state.services().cluster_status();

    match cluster_service.get_cluster_health().await {
        Ok(health) => {
            let view = ClusterHealthView::from(health);
            match view.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Failed to render cluster health view: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to render cluster health",
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get cluster health: {}", e);
            let error = ErrorView::new("Error loading health status");
            match error.render() {
                Ok(error_html) => Html(format!("<h2>Cluster Health</h2>{}", error_html)).into_response(),
                Err(render_err) => {
                    tracing::error!("Failed to render error view: {}", render_err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to load cluster health",
                    )
                        .into_response()
                }
            }
        }
    }
}

/// Dashboard queue statistics endpoint (HTMX partial)
pub async fn dashboard_queue_stats(State(state): State<AppState>) -> impl IntoResponse {
    let job_service = state.services().job_lifecycle();
    let stats = job_service.get_queue_stats().await;
    let view = QueueStatsView::from(stats);
    match view.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Failed to render queue stats view: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to render queue statistics",
            )
                .into_response()
        }
    }
}

/// Query parameters for job sorting
#[derive(Debug, Deserialize)]
pub struct SortQuery {
    pub sort: Option<String>,
}

/// Dashboard recent jobs table endpoint (HTMX partial)
pub async fn dashboard_recent_jobs(
    State(state): State<AppState>,
    Query(query): Query<SortQuery>,
) -> impl IntoResponse {
    let job_service = state.services().job_lifecycle();

    // Parse sort order from query
    let sort_by = query.sort.as_deref().unwrap_or("time");
    let sort_order = JobSortOrder::from_str(sort_by);

    match job_service.list_jobs(sort_order, 20).await {
        Ok(jobs) => {
            let view = JobListView::new(jobs);
            match view.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Failed to render job list view: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to render job list",
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to list jobs: {}", e);
            let error = ErrorView::new("Error loading jobs");
            match error.render() {
                Ok(error_html) => Html(error_html).into_response(),
                Err(render_err) => {
                    tracing::error!("Failed to render error view: {}", render_err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to load jobs",
                    )
                        .into_response()
                }
            }
        }
    }
}

/// Dashboard control plane nodes endpoint (HTMX partial)
pub async fn dashboard_control_plane_nodes(State(state): State<AppState>) -> impl IntoResponse {
    let cluster_service = state.services().cluster_status();

    match cluster_service.get_control_plane_nodes().await {
        Ok(nodes) => {
            let view = ControlPlaneNodesView::new(nodes);
            match view.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Failed to render control plane nodes view: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to render control plane nodes",
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get control plane nodes: {}", e);
            let error = ErrorView::new("Error loading control plane nodes");
            match error.render() {
                Ok(error_html) => Html(error_html).into_response(),
                Err(render_err) => {
                    tracing::error!("Failed to render error view: {}", render_err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to load control plane nodes",
                    )
                        .into_response()
                }
            }
        }
    }
}

/// Dashboard workers endpoint (HTMX partial)
pub async fn dashboard_workers(State(state): State<AppState>) -> impl IntoResponse {
    let cluster_service = state.services().cluster_status();

    match cluster_service.get_worker_stats().await {
        Ok(workers) => {
            let view = WorkersView::new(workers);
            match view.render() {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Failed to render workers view: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to render worker statistics",
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get worker stats: {}", e);
            let error = ErrorView::new("Error loading worker statistics");
            match error.render() {
                Ok(error_html) => Html(error_html).into_response(),
                Err(render_err) => {
                    tracing::error!("Failed to render error view: {}", render_err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to load worker statistics",
                    )
                        .into_response()
                }
            }
        }
    }
}

/// Job submission form data
#[derive(Deserialize)]
pub struct NewJob {
    pub url: String,
}

/// Dashboard job submission endpoint
pub async fn dashboard_submit_job(
    State(state): State<AppState>,
    Form(job): Form<NewJob>,
) -> Response {
    // Use pre-built domain service
    let job_service = state.services().job_lifecycle();

    let submission = JobSubmission {
        payload: serde_json::json!({ "url": job.url })
    };

    match job_service.submit_job(submission).await {
        Ok(job_id) => {
            tracing::info!(job_id = %job_id, "Job submitted from dashboard");
            // Return the updated jobs list sorted by time (most recent first)
            dashboard_recent_jobs(State(state), Query(SortQuery { sort: Some("time".to_string()) }))
                .await
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to submit job: {}", e);
            let error = ErrorView::new(format!("Error: {}", e));
            match error.render() {
                Ok(error_html) => Html(error_html).into_response(),
                Err(render_err) => {
                    tracing::error!("Failed to render error view: {}", render_err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to submit job: {}", e),
                    )
                        .into_response()
                }
            }
        }
    }
}
