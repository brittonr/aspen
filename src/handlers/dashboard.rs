//! Dashboard handlers for cluster monitoring and job management

#![allow(dead_code)] // Used via wildcard import in router

use askama::Template;
use axum::{
    Form,
    extract::{Query, State},
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
    Html(DashboardTemplate.render().expect("dashboard renders"))
}

/// Dashboard cluster health endpoint (HTMX partial)
pub async fn dashboard_cluster_health(State(state): State<AppState>) -> impl IntoResponse {
    let cluster_service = state.services().cluster_status();

    match cluster_service.get_cluster_health().await {
        Ok(health) => {
            let view = ClusterHealthView::from(health);
            Html(view.render().expect("cluster_health renders"))
        }
        Err(e) => {
            tracing::error!("Failed to get cluster health: {}", e);
            let error = ErrorView::new("Error loading health status");
            Html(format!("<h2>Cluster Health</h2>{}", error.render().expect("error renders")))
        }
    }
}

/// Dashboard queue statistics endpoint (HTMX partial)
pub async fn dashboard_queue_stats(State(state): State<AppState>) -> impl IntoResponse {
    let job_service = state.services().job_lifecycle();
    let stats = job_service.get_queue_stats().await;
    let view = QueueStatsView::from(stats);
    Html(view.render().expect("queue_stats renders"))
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
            Html(view.render().expect("job_list renders"))
        }
        Err(e) => {
            tracing::error!("Failed to list jobs: {}", e);
            let error = ErrorView::new("Error loading jobs");
            Html(error.render().expect("error renders"))
        }
    }
}

/// Dashboard control plane nodes endpoint (HTMX partial)
pub async fn dashboard_control_plane_nodes(State(state): State<AppState>) -> impl IntoResponse {
    let cluster_service = state.services().cluster_status();

    match cluster_service.get_control_plane_nodes().await {
        Ok(nodes) => {
            let view = ControlPlaneNodesView::new(nodes);
            Html(view.render().expect("control_plane_nodes renders"))
        }
        Err(e) => {
            tracing::error!("Failed to get control plane nodes: {}", e);
            let error = ErrorView::new("Error loading control plane nodes");
            Html(error.render().expect("error renders"))
        }
    }
}

/// Dashboard workers endpoint (HTMX partial)
pub async fn dashboard_workers(State(state): State<AppState>) -> impl IntoResponse {
    let cluster_service = state.services().cluster_status();

    match cluster_service.get_worker_stats().await {
        Ok(workers) => {
            let view = WorkersView::new(workers);
            Html(view.render().expect("workers renders"))
        }
        Err(e) => {
            tracing::error!("Failed to get worker stats: {}", e);
            let error = ErrorView::new("Error loading worker statistics");
            Html(error.render().expect("error renders"))
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

    let submission = JobSubmission { url: job.url };

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
            Html(error.render().expect("error renders")).into_response()
        }
    }
}
