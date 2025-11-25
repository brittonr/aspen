// Use modules from library crate
use mvm_ci::config::AppConfig;
use mvm_ci::server::ServerConfig;
use mvm_ci::state::{InfrastructureFactory, ProductionInfrastructureFactory};

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
    mvm_ci::server::wait_for_online(&endpoint)
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

    // Start VM Manager background tasks
    println!("Starting VM Manager...");
    state.infrastructure()
        .vm_manager()
        .start()
        .await
        .expect("Failed to start VM Manager");

    // Display work queue ticket for worker connections
    let work_ticket = state.infrastructure().work_queue().get_ticket();

    println!();
    println!("╔═══════════════════════════════════════════════════════════════════════════╗");
    println!("║                    MVM-CI Control Plane Started                          ║");
    println!("╚═══════════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("✓ Application infrastructure initialized");
    println!("✓ Hiqlite Raft cluster ready");
    println!("✓ Iroh P2P endpoint online");
    println!("✓ VM Manager started");
    println!();
    println!("╭───────────────────────────────────────────────────────────────────────────╮");
    println!("│ Control Plane Ticket (use this to connect workers):                      │");
    println!("╰───────────────────────────────────────────────────────────────────────────╯");
    println!();
    println!("  {}", work_ticket);
    println!();
    println!("╭───────────────────────────────────────────────────────────────────────────╮");
    println!("│ To start workers:                                                         │");
    println!("╰───────────────────────────────────────────────────────────────────────────╯");
    println!();
    println!("  WASM Worker (Flawless):");
    println!("    WORKER_TYPE=wasm \\");
    println!("    CONTROL_PLANE_TICKET=\"{}\" \\", work_ticket);
    println!("    ./target/debug/worker");
    println!();
    println!("  Firecracker Worker (MicroVM):");
    println!("    WORKER_TYPE=firecracker \\");
    println!("    CONTROL_PLANE_TICKET=\"{}\" \\", work_ticket);
    println!("    FIRECRACKER_DEFAULT_MEMORY_MB=1024 \\");
    println!("    FIRECRACKER_DEFAULT_VCPUS=2 \\");
    println!("    ./target/debug/worker");
    println!();
    println!("╭───────────────────────────────────────────────────────────────────────────╮");
    println!("│ Web Interfaces:                                                           │");
    println!("╰───────────────────────────────────────────────────────────────────────────╯");
    println!();
    println!("  Dashboard:  http://localhost:3020/");
    println!("  Queue:      http://localhost:3020/queue/list");
    println!("  Workers:    http://localhost:3020/api/workers");
    println!("  Health:     http://localhost:3020/api/health");
    println!();
    println!("═══════════════════════════════════════════════════════════════════════════");
    println!();

    // Start dual-listener server (localhost HTTP + P2P iroh+h3)
    let server_config = ServerConfig {
        app_config: config,
        endpoint,
        state,
    };

    let handle = mvm_ci::server::start(server_config)
        .await
        .expect("Failed to start server");

    // Run server (blocks until shutdown or error)
    handle.run().await.map_err(|e| e.into())
}
