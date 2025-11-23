use std::sync::Arc;

// Internal modules
mod config;
mod domain;
mod handlers;
mod hiqlite_persistent_store;
mod hiqlite_service;
mod iroh_api;
mod iroh_service;
mod persistent_store;
mod repositories;
mod server;
mod services;
mod state;
mod views;
mod work_item_cache;
mod work_queue;
mod work_state_machine;

use config::AppConfig;
use hiqlite_persistent_store::HiqlitePersistentStore;
use hiqlite_service::HiqliteService;
use iroh_service::IrohService;
use server::ServerConfig;
use state::AppState;
use work_queue::WorkQueue;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();

    // Load centralized configuration
    let config = AppConfig::load().expect("Failed to load configuration");

    // Initialize hiqlite distributed state store
    println!("Initializing hiqlite distributed state...");
    let hiqlite_service = HiqliteService::new(config.storage.hiqlite_data_dir.clone())
        .await
        .expect("Failed to initialize hiqlite");
    hiqlite_service
        .initialize_schema()
        .await
        .expect("Failed to initialize hiqlite schema");
    println!("✓ Hiqlite initialized");

    // Each node can have its own flawless server
    let flawless_url = config.flawless.flawless_url.clone();

    tracing::info!("Connecting to flawless server at {}", flawless_url);
    let flawless = flawless_utils::Server::new(&flawless_url, None);
    let flawless_module = flawless_utils::load_module_from_build!("module1");
    let module = flawless.deploy(flawless_module).await.unwrap();

    // Initialize iroh endpoint with HTTP/3 ALPN
    let endpoint = iroh::Endpoint::builder()
        .alpns(vec![config.network.iroh_alpn.clone()])
        .bind()
        .await
        .expect("Failed to create iroh endpoint");

    println!("Iroh Endpoint ID: {}", endpoint.id());

    // Wait for direct addresses and relay connection
    server::wait_for_online(&endpoint)
        .await
        .expect("Failed to connect to relay");

    // Initialize iroh service
    let iroh_service = IrohService::new(config.storage.iroh_blobs_path.clone(), endpoint.clone());

    // Initialize work queue with persistent store abstraction
    println!("Initializing distributed work queue...");
    let node_id = endpoint.id().to_string();

    // Create persistent store adapter for Hiqlite
    let persistent_store = Arc::new(HiqlitePersistentStore::new(hiqlite_service.clone()));

    let work_queue = WorkQueue::new(endpoint.clone(), node_id, persistent_store)
        .await
        .expect("Failed to initialize work queue");
    let work_ticket = work_queue.get_ticket();
    println!("✓ Work queue initialized");
    println!("Work Queue Ticket: {}", work_ticket);
    println!();

    // Create shared application state with modular architecture
    // (Services implement traits but are stored as concrete types)
    let state = AppState::from_infrastructure(module, iroh_service, hiqlite_service, work_queue);

    // NOTE: Worker loop removed - now handled by separate worker binary
    // Control plane is now a pure API server (no job execution)
    //
    // To run jobs, start the worker binary:
    //   CONTROL_PLANE_TICKET="iroh+h3://..." worker
    //
    // Or to run an embedded worker (backward compat), add back the worker loop
    // See src/bin/worker.rs for reference implementation

    // Start dual-listener server (localhost HTTP + P2P iroh+h3)
    let server_config = ServerConfig {
        app_config: config,
        endpoint,
        state,
    };

    let handle = server::start(server_config)
        .await
        .expect("Failed to start server");

    // Run server (blocks until shutdown or error)
    handle.run().await.map_err(|e| e.into())
}
