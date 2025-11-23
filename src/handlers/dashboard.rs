//! Dashboard handlers for cluster monitoring and job management

use askama::Template;
use axum::{
    Form,
    extract::{Query, State},
    response::{Html, IntoResponse, Response},
    http::header,
};
use serde::Deserialize;

use crate::state::AppState;
use crate::domain::{JobSortOrder, JobSubmission, format_duration, format_time_ago};
use crate::work_queue;

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
    // Use pre-built domain service (no per-request construction!)
    let cluster_service = state.services().cluster_status();

    let health = match cluster_service.get_cluster_health().await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to get cluster health: {}", e);
            let error_html = r#"<h2>Cluster Health</h2><p>Error loading health status</p>"#.to_string();
            return ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], error_html);
        }
    };

    let html = format!(
        r#"<h2>Cluster Health</h2>
<div class="stat">
    <span class="stat-label">
        <span class="health-indicator health-{}"></span>
        Control Plane ({} nodes{})
    </span>
    <span class="stat-value">{}</span>
</div>
<div class="stat">
    <span class="stat-label">Active Workers</span>
    <span class="stat-value">{}</span>
</div>"#,
        if health.is_healthy { "healthy" } else { "unhealthy" },
        health.node_count,
        if health.has_leader { ", leader elected" } else { ", no leader" },
        if health.is_healthy { "Healthy" } else { "Unhealthy" },
        health.active_worker_count
    );

    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}

/// Dashboard queue statistics endpoint (HTMX partial)
pub async fn dashboard_queue_stats(State(state): State<AppState>) -> impl IntoResponse {
    // Use pre-built domain service
    let job_service = state.services().job_lifecycle();

    let stats = job_service.get_queue_stats().await;

    let html = format!(
        r#"<h2>Queue Statistics</h2>
<div class="stat">
    <span class="stat-label">Pending</span>
    <span class="stat-value pending">{}</span>
</div>
<div class="stat">
    <span class="stat-label">In Progress</span>
    <span class="stat-value in-progress">{}</span>
</div>
<div class="stat">
    <span class="stat-label">Completed</span>
    <span class="stat-value completed">{}</span>
</div>
<div class="stat">
    <span class="stat-label">Failed</span>
    <span class="stat-value failed">{}</span>
</div>"#,
        stats.pending,
        stats.in_progress,
        stats.completed,
        stats.failed
    );

    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
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
    // Use pre-built domain service
    let job_service = state.services().job_lifecycle();

    // Parse sort order from query
    let sort_by = query.sort.as_deref().unwrap_or("time");
    let sort_order = JobSortOrder::from_str(sort_by);

    // Use domain service to get enriched jobs
    let jobs = match job_service.list_jobs(sort_order, 20).await {
        Ok(jobs) => jobs,
        Err(e) => {
            tracing::error!("Failed to list jobs: {}", e);
            let error_html = r#"<p style="color: #ef4444;">Error loading jobs</p>"#.to_string();
            return ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], error_html);
        }
    };

    let mut rows = String::new();
    for job in jobs {
        let status_class = match job.status {
            work_queue::WorkStatus::Pending => "status-pending",
            work_queue::WorkStatus::Claimed => "status-claimed",
            work_queue::WorkStatus::InProgress => "status-in-progress",
            work_queue::WorkStatus::Completed => "status-completed",
            work_queue::WorkStatus::Failed => "status-failed",
        };

        let status_text = format!("{:?}", job.status);
        let node_id = job.claimed_by
            .map(|id| format!("<span class=\"node-id\">{}</span>", &id[..std::cmp::min(16, id.len())]))
            .unwrap_or_else(|| "-".to_string());

        let (duration_text, duration_class) = format_duration(job.duration_seconds);
        let time_ago = format_time_ago(job.time_ago_seconds);

        rows.push_str(&format!(
            r#"<tr>
    <td>{}</td>
    <td><span class="status-badge {}">{}</span></td>
    <td><div class="job-url" title="{}">{}</div></td>
    <td><span class="duration {}">{}</span></td>
    <td><span class="time-ago">{}</span></td>
    <td>{}</td>
</tr>
"#,
            job.job_id,
            status_class,
            status_text,
            job.url,
            job.url,
            duration_class,
            duration_text,
            time_ago,
            node_id
        ));
    }

    let html = format!(
        r#"<table>
    <thead>
        <tr>
            <th>Job ID</th>
            <th>Status</th>
            <th>URL</th>
            <th>Duration</th>
            <th>Updated</th>
            <th>Claimed By</th>
        </tr>
    </thead>
    <tbody>
        {}
    </tbody>
</table>
<div class="refresh-indicator">Auto-refreshing every 2s</div>"#,
        if rows.is_empty() { "<tr><td colspan=\"6\" style=\"text-align: center; color: #94a3b8;\">No jobs yet</td></tr>" } else { &rows }
    );

    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}

/// Dashboard control plane nodes endpoint (HTMX partial)
pub async fn dashboard_control_plane_nodes(State(state): State<AppState>) -> impl IntoResponse {
    // Use pre-built domain service
    let cluster_service = state.services().cluster_status();

    let nodes = match cluster_service.get_control_plane_nodes().await {
        Ok(nodes) => nodes,
        Err(e) => {
            tracing::error!("Failed to get control plane nodes: {}", e);
            let error_html = r#"<p style="color: #94a3b8; text-align: center;">Error loading control plane nodes</p>"#.to_string();
            return ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], error_html);
        }
    };

    let mut nodes_html = String::new();

    if nodes.is_empty() {
        nodes_html.push_str(r#"<p style="color: #94a3b8; text-align: center;">No control plane nodes detected</p>"#);
    } else {
        for node in nodes {
            nodes_html.push_str(&format!(
                r#"<div class="node-item{}">
    <div class="node-label">
        Node {}
        {}
    </div>
    <div class="node-id">Status: Active</div>
</div>
"#,
                if node.is_leader { " leader" } else { "" },
                node.node_number,
                if node.is_leader { r#"<span class="node-badge badge-leader">Leader</span>"# } else { r#"<span class="node-badge badge-follower">Follower</span>"# }
            ));
        }
    }

    let html = format!(
        r#"<div class="node-list">
    {}
</div>
<div class="refresh-indicator">Auto-refreshing every 2s</div>"#,
        nodes_html
    );

    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}

/// Dashboard workers endpoint (HTMX partial)
pub async fn dashboard_workers(State(state): State<AppState>) -> impl IntoResponse {
    // Use pre-built domain service
    let cluster_service = state.services().cluster_status();

    let workers = match cluster_service.get_worker_stats().await {
        Ok(workers) => workers,
        Err(e) => {
            tracing::error!("Failed to get worker stats: {}", e);
            let error_html = r#"<p style="color: #94a3b8; text-align: center;">Error loading worker statistics</p>"#.to_string();
            return ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], error_html);
        }
    };

    let mut workers_html = String::new();

    if workers.is_empty() {
        workers_html.push_str(r#"<p style="color: #94a3b8; text-align: center;">No workers have claimed jobs yet</p>"#);
    } else {
        for worker in workers {
            let time_ago = format_time_ago(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
                    - worker.last_seen_timestamp,
            );

            workers_html.push_str(&format!(
                r#"<div class="worker-item{}">
    <div class="node-label">{}</div>
    <div class="worker-stats">
        <div class="worker-stat-item">
            <span>Active Jobs:</span>
            <strong>{}</strong>
        </div>
        <div class="worker-stat-item">
            <span>Completed:</span>
            <strong>{}</strong>
        </div>
        <div class="worker-stat-item">
            <span>Last Seen:</span>
            <strong>{}</strong>
        </div>
    </div>
</div>
"#,
                if worker.is_active { " active" } else { "" },
                &worker.node_id[..std::cmp::min(32, worker.node_id.len())],
                worker.active_jobs,
                worker.completed_jobs,
                time_ago
            ));
        }
    }

    let html = format!(
        r#"<div class="node-list">
    {}
</div>
<div class="refresh-indicator">Auto-refreshing every 2s</div>"#,
        workers_html
    );

    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
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
            let error_html = format!(r#"<p style="color: #ef4444;">Error: {}</p>"#, e);
            ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], error_html).into_response()
        }
    }
}
