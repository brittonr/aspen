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
use std::sync::Arc;

use crate::domain::{JobSortOrder, JobSubmission, ClusterStatusService, JobCommandService, JobQueryService};
use crate::views::{ClusterHealthView, QueueStatsView, JobListView, ControlPlaneNodesView, WorkersView};
use super::view_renderer::{render_template, render_service_result, render_error_with_wrapper, render_error};

/// Main dashboard template
#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate;

/// Dashboard main page
pub async fn dashboard() -> impl IntoResponse {
    render_template(DashboardTemplate)
}

/// Dashboard cluster health endpoint (HTMX partial)
pub async fn dashboard_cluster_health(
    State(cluster_service): State<Arc<ClusterStatusService>>,
) -> impl IntoResponse {
    let result = cluster_service.get_cluster_health().await;

    match result {
        Ok(health) => render_template(ClusterHealthView::from(health)),
        Err(e) => {
            tracing::error!("Failed to get cluster health: {}", e);
            render_error_with_wrapper(
                "Error loading health status",
                |error_html| format!("<h2>Cluster Health</h2>{}", error_html)
            )
        }
    }
}

/// Dashboard queue statistics endpoint (HTMX partial)
pub async fn dashboard_queue_stats(
    State(job_queries): State<Arc<JobQueryService>>,
) -> impl IntoResponse {
    let stats = job_queries.get_queue_stats().await;
    render_template(QueueStatsView::from(stats))
}

/// Query parameters for job sorting
#[derive(Debug, Deserialize)]
pub struct SortQuery {
    pub sort: Option<String>,
}

/// Dashboard recent jobs table endpoint (HTMX partial)
pub async fn dashboard_recent_jobs(
    State(job_queries): State<Arc<JobQueryService>>,
    Query(query): Query<SortQuery>,
) -> impl IntoResponse {
    // Parse sort order from query
    let sort_by = query.sort.as_deref().unwrap_or("time");
    let sort_order = JobSortOrder::from_str(sort_by);

    let result = job_queries.list_jobs_with_options(sort_order, 20).await;
    render_service_result(
        result,
        |jobs| JobListView::new(jobs),
        "Failed to list jobs"
    )
}

/// Dashboard control plane nodes endpoint (HTMX partial)
pub async fn dashboard_control_plane_nodes(
    State(cluster_service): State<Arc<ClusterStatusService>>,
) -> impl IntoResponse {
    let result = cluster_service.get_control_plane_nodes().await;
    render_service_result(
        result,
        |nodes| ControlPlaneNodesView::new(nodes),
        "Failed to get control plane nodes"
    )
}

/// Dashboard workers endpoint (HTMX partial)
pub async fn dashboard_workers(
    State(cluster_service): State<Arc<ClusterStatusService>>,
) -> impl IntoResponse {
    let result = cluster_service.get_worker_stats().await;
    render_service_result(
        result,
        |workers| WorkersView::new(workers),
        "Failed to get worker statistics"
    )
}

/// Job submission form data
#[derive(Deserialize)]
pub struct NewJob {
    pub url: String,
}

/// Dashboard job submission endpoint
pub async fn dashboard_submit_job(
    State((job_commands, job_queries)): State<(Arc<JobCommandService>, Arc<JobQueryService>)>,
    Form(job): Form<NewJob>,
) -> Response {
    let submission = JobSubmission {
        payload: serde_json::json!({ "url": job.url })
    };

    match job_commands.submit_job(submission).await {
        Ok(job_id) => {
            tracing::info!(job_id = %job_id, "Job submitted from dashboard");
            // Return the updated jobs list sorted by time (most recent first)
            dashboard_recent_jobs(State(job_queries), Query(SortQuery { sort: Some("time".to_string()) }))
                .await
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to submit job: {}", e);
            render_error(&format!("Error submitting job: {}", e))
        }
    }
}
