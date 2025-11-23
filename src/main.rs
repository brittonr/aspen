use std::{sync::Arc, time::Duration};

use askama::Template;
use axum::{
    Form, Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use flawless_utils::DeployedModule;
use iroh_tickets::endpoint::EndpointTicket;
use module1::Job;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

mod iroh_service;
mod iroh_api;
mod hiqlite_service;
mod work_queue;

use iroh_service::IrohService;
use hiqlite_service::HiqliteService;
use work_queue::WorkQueue;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();

    // Initialize hiqlite distributed state store
    println!("Initializing hiqlite distributed state...");
    let hiqlite_service = HiqliteService::new()
        .await
        .expect("Failed to initialize hiqlite");
    hiqlite_service
        .initialize_schema()
        .await
        .expect("Failed to initialize hiqlite schema");
    println!("✓ Hiqlite initialized");

    // Each node can have its own flawless server
    let flawless_url = std::env::var("FLAWLESS_URL")
        .unwrap_or_else(|_| "http://localhost:27288".to_string());

    tracing::info!("Connecting to flawless server at {}", flawless_url);
    let flawless = flawless_utils::Server::new(&flawless_url, None);
    let flawless_module = flawless_utils::load_module_from_build!("module1");
    let module = flawless.deploy(flawless_module).await.unwrap();

    // Initialize iroh endpoint with HTTP/3 ALPN
    let endpoint = iroh::Endpoint::builder()
        .alpns(vec![b"iroh+h3".to_vec()])
        .bind()
        .await
        .expect("Failed to create iroh endpoint");

    println!("Iroh Endpoint ID: {}", endpoint.id());
    println!("Waiting for endpoint to be online...");

    // Wait for direct addresses and relay connection
    endpoint.online().await;

    // Generate and display endpoint ticket
    let ticket = EndpointTicket::new(endpoint.addr());
    let base_url = format!("iroh+h3://{}", ticket);
    println!("==================================================");
    println!("Iroh Endpoint Ticket:");
    println!("  {}/", base_url);
    println!("==================================================");
    println!("Workflows will connect using this P2P URL");
    println!();

    // Set environment variable for workflows to use
    unsafe {
        std::env::set_var("SERVER_BASE_URL", &base_url);
    }

    // Initialize iroh service
    let iroh_blob_path = std::env::var("IROH_BLOBS_PATH")
        .unwrap_or_else(|_| "./data/iroh-blobs".to_string());
    let iroh_service = IrohService::new(iroh_blob_path.into(), endpoint.clone());

    // Initialize work queue with hiqlite backend
    println!("Initializing distributed work queue...");
    let node_id = endpoint.id().to_string();
    let work_queue = WorkQueue::new(endpoint.clone(), node_id, hiqlite_service.clone())
        .await
        .expect("Failed to initialize work queue");
    let work_ticket = work_queue.get_ticket();
    println!("✓ Work queue initialized");
    println!("Work Queue Ticket: {}", work_ticket);
    println!();

    // Create shared application state
    let state = AppState::new(module, iroh_service, hiqlite_service, work_queue);

    let app = Router::new()
        .route("/", get(index))
        .route("/timeout", get(timeout))
        .route("/new-job", post(new_job))
        .route("/list", get(list))
        .route("/ui-update", post(ui_update))
        .route("/assets/{*path}", get(handle_assets))
        // Dashboard routes
        .route("/dashboard", get(dashboard))
        .route("/dashboard/cluster-health", get(dashboard_cluster_health))
        .route("/dashboard/queue-stats", get(dashboard_queue_stats))
        .route("/dashboard/recent-jobs", get(dashboard_recent_jobs))
        .route("/dashboard/control-plane-nodes", get(dashboard_control_plane_nodes))
        .route("/dashboard/workers", get(dashboard_workers))
        .route("/dashboard/submit-job", post(dashboard_submit_job))
        // Iroh API routes
        .route("/iroh/blob/store", post(iroh_api::store_blob))
        .route("/iroh/blob/{hash}", get(iroh_api::retrieve_blob))
        .route("/iroh/gossip/join", post(iroh_api::join_gossip_topic))
        .route("/iroh/gossip/broadcast", post(iroh_api::broadcast_gossip))
        .route("/iroh/gossip/subscribe/{topic_id}", get(iroh_api::subscribe_gossip))
        .route("/iroh/connect", post(iroh_api::connect_peer))
        .route("/iroh/info", get(iroh_api::endpoint_info))
        // Work Queue API routes
        .route("/queue/publish", post(queue_publish))
        .route("/queue/claim", post(queue_claim))
        .route("/queue/list", get(queue_list))
        .route("/queue/stats", get(queue_stats))
        .route("/queue/status/{job_id}", post(queue_update_status))
        // Hiqlite API routes
        .route("/hiqlite/health", get(hiqlite_health))
        .with_state(state.clone());

    // Clone router for dual listeners
    let local_app = app.clone();
    let p2p_app = app;

    // Spawn localhost HTTP listener for workflows and Web UI
    let http_port = std::env::var("HTTP_PORT")
        .unwrap_or_else(|_| "3020".to_string())
        .parse::<u16>()
        .expect("HTTP_PORT must be a valid port number");

    tokio::spawn(async move {
        let addr = format!("0.0.0.0:{}", http_port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect(&format!("Failed to bind {}", addr));
        println!("Local HTTP server: http://0.0.0.0:{}", http_port);
        println!("  - WASM workflows can connect here");
        println!("  - Web UI accessible in browser");
        axum::serve(listener, local_app)
            .await
            .expect("Local HTTP server failed");
    });

    // NOTE: Worker loop removed - now handled by separate worker binary
    // Control plane is now a pure API server (no job execution)
    //
    // To run jobs, start the worker binary:
    //   CONTROL_PLANE_TICKET="iroh+h3://..." worker
    //
    // Or to run an embedded worker (backward compat), add back the worker loop
    // See src/bin/worker.rs for reference implementation

    // Main thread runs P2P listener for distributed communication
    println!("Starting HTTP/3 server over iroh for P2P...");
    println!("  - Remote instances can connect via P2P");
    println!("  - Blob storage and gossip coordination");

    match h3_iroh::axum::serve(endpoint, p2p_app).await {
        Ok(_) => {
            tracing::info!("HTTP/3 over iroh server shut down gracefully");
            Ok(())
        }
        Err(e) => {
            tracing::error!(error = %e, "HTTP/3 over iroh server failed - relay connection may be lost");
            Err(e.into())
        }
    }
}

#[derive(Debug, Clone)]
struct AppState {
    inner: Arc<StateInner>,
}

#[derive(Debug)]
struct StateInner {
    module: DeployedModule,
    progress: Mutex<Vec<JobProgress>>,
    iroh: IrohService,
    hiqlite: HiqliteService,
    work_queue: WorkQueue,
}

impl AppState {
    fn new(module: DeployedModule, iroh: IrohService, hiqlite: HiqliteService, work_queue: WorkQueue) -> Self {
        AppState {
            inner: Arc::new(StateInner {
                module,
                progress: Mutex::new(Vec::new()),
                iroh,
                hiqlite,
                work_queue,
            }),
        }
    }

    fn module(&self) -> &DeployedModule {
        &self.inner.module
    }

    pub fn iroh(&self) -> &IrohService {
        &self.inner.iroh
    }

    pub fn hiqlite(&self) -> &HiqliteService {
        &self.inner.hiqlite
    }

    pub fn work_queue(&self) -> &WorkQueue {
        &self.inner.work_queue
    }

    async fn progress(&self) -> Vec<JobProgress> {
        self.inner.progress.lock().await.clone()
    }

    async fn add_job(&self, url: String) -> usize {
        let mut progress = self.inner.progress.lock().await;
        progress.push(JobProgress {
            url,
            list: Vec::new(),
        });
        progress.len() - 1
    }

    async fn update_job(&self, id: usize, status: Status, url: String) {
        let mut progress = self.inner.progress.lock().await;
        let job = progress.get_mut(id).unwrap();

        let list_element = if url == "last" {
            job.list.last_mut()
        } else {
            None
        };

        match list_element {
            Some(list_element) => {
                list_element.0 = status.to_color();
                list_element.1 = status.to_string();
            }
            None => job.list.push((status.to_color(), status.to_string(), url)),
        }
    }
}

async fn timeout() -> impl IntoResponse {
    tokio::time::sleep(Duration::from_secs(60)).await;
    Html("Response after 60 seconds")
}

#[derive(Debug, Clone)]
struct JobProgress {
    url: String,
    // color, status, url
    list: Vec<(String, String, String)>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

async fn index() -> impl IntoResponse {
    Html(IndexTemplate.render().expect("index renders"))
}

#[derive(Deserialize)]
struct NewJob {
    url: String,
}

async fn new_job(State(state): State<AppState>, Form(job): Form<NewJob>) -> impl IntoResponse {
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

    // Job is now in the queue - background workers will claim and process it
    tracing::info!(job_id = %job_id, url = %url, "Job published to distributed queue");

    list(State(state)).await
}

async fn ui_update(
    State(state): State<AppState>,
    Json(ui_update): Json<UpdateUI>,
) -> impl IntoResponse {
    // Update local UI state
    state
        .update_job(ui_update.id, ui_update.status.clone(), ui_update.url.clone())
        .await;

    // Sync to hiqlite as the single source of truth
    let job_id = format!("job-{}", ui_update.id);
    let work_status = match ui_update.status {
        Status::Request | Status::Parse => work_queue::WorkStatus::InProgress,
        Status::Done => work_queue::WorkStatus::Completed,
        Status::Error => work_queue::WorkStatus::Failed,
    };

    // Update hiqlite workflow state
    if let Err(e) = state.work_queue().update_status(&job_id, work_status).await {
        tracing::warn!("Failed to sync workflow status to hiqlite: {}", e);
    }

    Html("OK")
}

static STYLE_CSS: &str = include_str!("../assets/styles.css");

async fn handle_assets(Path(path): Path<String>) -> impl IntoResponse {
    let mut headers = HeaderMap::new();

    if path == "styles.css" {
        headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());
        (StatusCode::OK, headers, STYLE_CSS)
    } else {
        (StatusCode::NOT_FOUND, headers, "")
    }
}

#[derive(Template)]
#[template(path = "list.html")]
struct ListTemplate {
    progress: Vec<JobProgress>,
}

async fn list(State(state): State<AppState>) -> impl IntoResponse {
    let progress = state.progress().await;
    // Display max 10 last requests.
    let progress = progress
        .iter()
        .map(|job| {
            let url = job.url.clone();
            let list = job.list.iter().rev().take(10).map(|e| e.clone()).collect();
            JobProgress { url, list }
        })
        .collect();
    let template = ListTemplate { progress };
    Html(template.render().expect("index renders"))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUI {
    pub id: usize,
    pub status: Status,
    pub url: String,
    pub urls_left: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Status {
    // Request in progress.
    Request,
    // Parsing in progress.
    Parse,
    // Finished.
    Done,
    // Error
    Error,
}

impl Status {
    fn to_string(&self) -> String {
        match self {
            Status::Request => "REQ".to_string(),
            Status::Parse => "PARS".to_string(),
            Status::Done => "DONE".to_string(),
            Status::Error => "ERR".to_string(),
        }
    }
    fn to_color(&self) -> String {
        match self {
            Status::Request => "orange".to_string(),
            Status::Parse => "yellow".to_string(),
            Status::Done => "green".to_string(),
            Status::Error => "red".to_string(),
        }
    }
}

async fn hiqlite_health(State(state): State<AppState>) -> impl IntoResponse {
    match state.hiqlite().health_check().await {
        Ok(health) => Json(health).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Health check failed: {}", e),
        )
            .into_response(),
    }
}

// Work Queue API Handlers

#[derive(Debug, Deserialize)]
struct PublishWorkRequest {
    job_id: String,
    payload: serde_json::Value,
}

async fn queue_publish(
    State(state): State<AppState>,
    Json(req): Json<PublishWorkRequest>,
) -> impl IntoResponse {
    match state.work_queue().publish_work(req.job_id, req.payload).await {
        Ok(()) => (StatusCode::OK, "Work item published").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to publish work: {}", e),
        )
            .into_response(),
    }
}

async fn queue_claim(State(state): State<AppState>) -> impl IntoResponse {
    match state.work_queue().claim_work().await {
        Ok(Some(work_item)) => Json(work_item).into_response(),
        Ok(None) => (StatusCode::NO_CONTENT, "No work available").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to claim work: {}", e),
        )
            .into_response(),
    }
}

async fn queue_list(State(state): State<AppState>) -> impl IntoResponse {
    match state.work_queue().list_work().await {
        Ok(work_items) => Json(work_items).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list work: {}", e),
        )
            .into_response(),
    }
}

async fn queue_stats(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.work_queue().stats().await;
    Json(stats).into_response()
}

// Dashboard Handlers

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate;

async fn dashboard() -> impl IntoResponse {
    Html(DashboardTemplate.render().expect("dashboard renders"))
}

async fn dashboard_cluster_health(State(state): State<AppState>) -> impl IntoResponse {
    let health_check = state.hiqlite().health_check().await.ok();
    let is_healthy = health_check.as_ref().map(|h| h.is_healthy).unwrap_or(false);
    let node_count = health_check.as_ref().map(|h| h.node_count).unwrap_or(0);
    let has_leader = health_check.as_ref().map(|h| h.has_leader).unwrap_or(false);

    // Count active workers from recent jobs
    let work_items = state.work_queue().list_work().await.unwrap_or_default();
    let active_workers: std::collections::HashSet<String> = work_items
        .iter()
        .filter_map(|item| item.claimed_by.clone())
        .collect();
    let worker_count = active_workers.len();

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
        if is_healthy { "healthy" } else { "unhealthy" },
        node_count,
        if has_leader { ", leader elected" } else { ", no leader" },
        if is_healthy { "Healthy" } else { "Unhealthy" },
        worker_count
    );

    Html(html)
}

async fn dashboard_queue_stats(State(state): State<AppState>) -> impl IntoResponse {
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

#[derive(Debug, Deserialize)]
struct SortQuery {
    sort: Option<String>,
}

async fn dashboard_recent_jobs(
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

async fn dashboard_control_plane_nodes(State(state): State<AppState>) -> impl IntoResponse {
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

async fn dashboard_workers(State(state): State<AppState>) -> impl IntoResponse {
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

async fn dashboard_submit_job(
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

#[derive(Debug, Deserialize)]
struct UpdateStatusRequest {
    status: work_queue::WorkStatus,
}

async fn queue_update_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
    Json(req): Json<UpdateStatusRequest>,
) -> impl IntoResponse {
    match state.work_queue().update_status(&job_id, req.status).await {
        Ok(()) => (StatusCode::OK, "Status updated").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update status: {}", e),
        )
            .into_response(),
    }
}
