//! Dashboard handlers for cluster monitoring and job management

use askama::Template;
use axum::{
    Form,
    extract::{Query, State},
    response::{Html, IntoResponse, Response},
    http::{StatusCode, header},
};
use serde::Deserialize;

use crate::state::AppState;
use crate::domain::{ClusterStatusService, JobLifecycleService, JobSortOrder, JobSubmission, format_duration, format_time_ago};
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
    // Use domain service for cluster health
    let cluster_service = ClusterStatusService::new(
        state.hiqlite().clone(),
        state.work_queue().clone(),
    );

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
    let stats = state.work_queue().stats().await;

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

    Html(html)
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
    let mut work_items = state.work_queue().list_work().await.unwrap_or_default();

    // Sort based on query parameter
    let sort_by = query.sort.as_deref().unwrap_or("time");
    match sort_by {
        "status" => {
            work_items.sort_by(|a, b| {
                // Sort by status, then by updated_at
                match format!("{:?}", a.status).cmp(&format!("{:?}", b.status)) {
                    std::cmp::Ordering::Equal => b.updated_at.cmp(&a.updated_at),
                    other => other,
                }
            });
        }
        "job_id" => {
            work_items.sort_by(|a, b| a.job_id.cmp(&b.job_id));
        }
        _ => {
            // Default to time (most recent first)
            work_items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        }
    }

    // Take first 20 jobs after sorting
    let recent: Vec<_> = work_items.iter().take(20).collect();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut rows = String::new();
    for item in recent {
        let status_class = match item.status {
            work_queue::WorkStatus::Pending => "status-pending",
            work_queue::WorkStatus::Claimed => "status-claimed",
            work_queue::WorkStatus::InProgress => "status-in-progress",
            work_queue::WorkStatus::Completed => "status-completed",
            work_queue::WorkStatus::Failed => "status-failed",
        };

        let status_text = format!("{:?}", item.status);
        let node_id = item.claimed_by.as_deref()
            .map(|id| format!("<span class=\"node-id\">{}</span>", &id[..16]))
            .unwrap_or_else(|| "-".to_string());

        // Extract URL from payload
        let url = item.payload.get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("-");

        // Calculate duration
        let duration_seconds = item.updated_at - item.created_at;
        let (duration_text, duration_class) = if duration_seconds == 0 {
            ("-".to_string(), "")
        } else if duration_seconds < 1 {
            (format!("{}ms", duration_seconds * 1000), "fast")
        } else if duration_seconds < 5 {
            (format!("{}s", duration_seconds), "fast")
        } else if duration_seconds < 30 {
            (format!("{}s", duration_seconds), "medium")
        } else {
            (format!("{}s", duration_seconds), "slow")
        };

        // Time ago formatting
        let time_ago_seconds = now - item.updated_at;
        let time_ago = if time_ago_seconds < 5 {
            "just now".to_string()
        } else if time_ago_seconds < 60 {
            format!("{}s ago", time_ago_seconds)
        } else if time_ago_seconds < 3600 {
            format!("{}m ago", time_ago_seconds / 60)
        } else if time_ago_seconds < 86400 {
            format!("{}h ago", time_ago_seconds / 3600)
        } else {
            format!("{}d ago", time_ago_seconds / 86400)
        };

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
            item.job_id,
            status_class,
            status_text,
            url,
            url,
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

    Html(html)
}

/// Dashboard control plane nodes endpoint (HTMX partial)
pub async fn dashboard_control_plane_nodes(State(state): State<AppState>) -> impl IntoResponse {
    let health_check = state.hiqlite().health_check().await.ok();
    let node_count = health_check.as_ref().map(|h| h.node_count).unwrap_or(0);
    let has_leader = health_check.as_ref().map(|h| h.has_leader).unwrap_or(false);

    // For now, we'll show basic node info from hiqlite
    // In the future, we could query hiqlite for detailed node information
    let mut nodes_html = String::new();

    if node_count > 0 {
        for i in 1..=node_count {
            let is_leader = has_leader && i == 1; // Simplified - assume node 1 is leader if there is one
            nodes_html.push_str(&format!(
                r#"<div class="node-item{}">
    <div class="node-label">
        Node {}
        {}
    </div>
    <div class="node-id">Status: Active</div>
</div>
"#,
                if is_leader { " leader" } else { "" },
                i,
                if is_leader { r#"<span class="node-badge badge-leader">Leader</span>"# } else { r#"<span class="node-badge badge-follower">Follower</span>"# }
            ));
        }
    } else {
        nodes_html.push_str(r#"<p style="color: #94a3b8; text-align: center;">No control plane nodes detected</p>"#);
    }

    let html = format!(
        r#"<div class="node-list">
    {}
</div>
<div class="refresh-indicator">Auto-refreshing every 2s</div>"#,
        nodes_html
    );

    Html(html)
}

/// Dashboard workers endpoint (HTMX partial)
pub async fn dashboard_workers(State(state): State<AppState>) -> impl IntoResponse {
    let work_items = state.work_queue().list_work().await.unwrap_or_default();

    // Aggregate worker statistics
    let mut worker_stats: std::collections::HashMap<String, (usize, usize, i64)> = std::collections::HashMap::new();

    for item in &work_items {
        if let Some(node_id) = &item.claimed_by {
            let entry = worker_stats.entry(node_id.clone()).or_insert((0, 0, 0));
            match item.status {
                work_queue::WorkStatus::Completed => entry.1 += 1,
                work_queue::WorkStatus::InProgress | work_queue::WorkStatus::Claimed => entry.0 += 1,
                _ => {}
            }
            // Track most recent activity
            if item.updated_at > entry.2 {
                entry.2 = item.updated_at;
            }
        }
    }

    let mut workers_html = String::new();

    if worker_stats.is_empty() {
        workers_html.push_str(r#"<p style="color: #94a3b8; text-align: center;">No workers have claimed jobs yet</p>"#);
    } else {
        // Sort by most recent activity
        let mut workers: Vec<_> = worker_stats.iter().collect();
        workers.sort_by(|a, b| b.1.2.cmp(&a.1.2));

        for (node_id, (active, completed, last_seen)) in workers {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            let seconds_ago = now - last_seen;
            let is_active = seconds_ago < 30; // Active if seen in last 30 seconds

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
                if is_active { " active" } else { "" },
                &node_id[..std::cmp::min(32, node_id.len())],
                active,
                completed,
                if seconds_ago < 5 {
                    "just now".to_string()
                } else if seconds_ago < 60 {
                    format!("{}s ago", seconds_ago)
                } else {
                    format!("{}m ago", seconds_ago / 60)
                }
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

    Html(html)
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
) -> impl IntoResponse {
    let url = job.url;
    let id = state.add_job(url.clone()).await;
    let job_id = format!("job-{}", id);

    // Publish work to hiqlite-backed queue
    let payload = serde_json::json!({
        "id": id,
        "url": url,
    });

    state
        .work_queue()
        .publish_work(job_id.clone(), payload)
        .await
        .expect("Failed to publish work to hiqlite");

    tracing::info!(job_id = %job_id, url = %url, "Job published from dashboard");

    // Return the updated jobs list sorted by time (most recent first)
    dashboard_recent_jobs(State(state), Query(SortQuery { sort: Some("time".to_string()) })).await
}
