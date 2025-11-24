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
use server::ServerConfig;
use state::{InfrastructureFactory, ProductionInfrastructureFactory};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();

    // Load centralized configuration
    let config = AppConfig::load().expect("Failed to load configuration");

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

    let node_id = endpoint.id().to_string();

    // Create application state using factory pattern (enables dependency injection)
    println!("Initializing application infrastructure...");
    let factory = ProductionInfrastructureFactory::new();
    let state = factory
        .build_app_state(&config, module, endpoint.clone(), node_id)
        .await
        .expect("Failed to build application state");

    // Display work queue ticket for worker connections
    let work_ticket = state.infrastructure().work_queue().get_ticket();
    println!("✓ Application infrastructure initialized");
    println!("Work Queue Ticket: {}", work_ticket);
    println!();

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
