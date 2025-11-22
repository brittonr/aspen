use std::{sync::Arc, time::Duration};

use askama::Template;
use axum::{
    Form, Json, Router,
    extract::{Path, State},
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

    // Spawn background worker to claim and process jobs from distributed queue
    let worker_state = state.clone();
    tokio::spawn(async move {
        println!("Starting distributed work queue processor...");
        println!("  - Polling for jobs every 2 seconds");
        println!("  - Load balanced across all cluster nodes");

        loop {
            // Try to claim work from the distributed queue
            match worker_state.work_queue().claim_work().await {
                Ok(Some(work_item)) => {
                    tracing::info!(
                        job_id = %work_item.job_id,
                        "Claimed job from distributed queue"
                    );

                    // Immediately mark as in-progress to prevent re-claiming
                    if let Err(e) = worker_state.work_queue()
                        .update_status(&work_item.job_id, work_queue::WorkStatus::InProgress)
                        .await
                    {
                        tracing::error!(job_id = %work_item.job_id, error = %e, "Failed to mark job as in-progress");
                    }

                    // Parse the payload to get job details
                    let payload = &work_item.payload;
                    if let (Some(id), Some(url)) = (payload.get("id").and_then(|v| v.as_u64()), payload.get("url").and_then(|v| v.as_str())) {
                        let url = url.to_string();
                        let id = id as usize;

                        // Execute the workflow
                        tracing::info!(job_id = %work_item.job_id, url = %url, "Starting workflow execution");

                        if let Err(e) = worker_state
                            .module()
                            .start::<module1::start_crawler>(Job { id, url: url.clone() })
                            .await
                        {
                            tracing::error!(job_id = %work_item.job_id, error = %e, "Workflow execution failed");
                            worker_state.work_queue()
                                .update_status(&work_item.job_id, work_queue::WorkStatus::Failed)
                                .await
                                .ok();
                        } else {
                            tracing::info!(job_id = %work_item.job_id, "Workflow completed successfully");
                            worker_state.work_queue()
                                .update_status(&work_item.job_id, work_queue::WorkStatus::Completed)
                                .await
                                .ok();
                        }
                    }
                }
                Ok(None) => {
                    // No work available, wait before polling again
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to claim work from queue");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });

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
