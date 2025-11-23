use axum::{
    Router,
    routing::{get, post},
};
use flawless_utils::DeployedModule;
use iroh_tickets::endpoint::EndpointTicket;

// Internal modules
mod iroh_service;
mod iroh_api;
mod hiqlite_service;
mod work_queue;
mod state;
mod domain;
mod repositories;
mod handlers;

use iroh_service::IrohService;
use hiqlite_service::HiqliteService;
use work_queue::WorkQueue;
use state::AppState;
use handlers::*;

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

    // Create shared application state with modular architecture
    let state = AppState::from_infrastructure(module, iroh_service, hiqlite_service, work_queue);

    // Build Axum router with all routes
    let app = Router::new()
        // Job UI routes (legacy)
        .route("/", get(index))
        .route("/timeout", get(timeout))
        .route("/new-job", post(new_job))
        .route("/list", get(list))
        .route("/ui-update", post(ui_update))
        .route("/assets/{*path}", get(handle_assets))
        // Dashboard routes (HTMX monitoring UI)
        .route("/dashboard", get(dashboard))
        .route("/dashboard/cluster-health", get(dashboard_cluster_health))
        .route("/dashboard/queue-stats", get(dashboard_queue_stats))
        .route("/dashboard/recent-jobs", get(dashboard_recent_jobs))
        .route("/dashboard/control-plane-nodes", get(dashboard_control_plane_nodes))
        .route("/dashboard/workers", get(dashboard_workers))
        .route("/dashboard/submit-job", post(dashboard_submit_job))
        // Iroh API routes (P2P blob storage and gossip)
        .route("/iroh/blob/store", post(iroh_api::store_blob))
        .route("/iroh/blob/{hash}", get(iroh_api::retrieve_blob))
        .route("/iroh/gossip/join", post(iroh_api::join_gossip_topic))
        .route("/iroh/gossip/broadcast", post(iroh_api::broadcast_gossip))
        .route("/iroh/gossip/subscribe/{topic_id}", get(iroh_api::subscribe_gossip))
        .route("/iroh/connect", post(iroh_api::connect_peer))
        .route("/iroh/info", get(iroh_api::endpoint_info))
        // Work Queue API routes (for workers)
        .route("/queue/publish", post(queue_publish))
        .route("/queue/claim", post(queue_claim))
        .route("/queue/list", get(queue_list))
        .route("/queue/stats", get(queue_stats))
        .route("/queue/status/{job_id}", post(queue_update_status))
        // Hiqlite API routes (health check)
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
