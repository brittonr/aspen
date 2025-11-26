// Use modules from library crate
use mvm_ci::config::AppConfig;
use mvm_ci::server::ServerConfig;
use mvm_ci::state::{InfrastructureFactory, ProductionInfrastructureFactory};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();

    // Load centralized configuration with layered approach
    // Priority: Environment variables > CONFIG_FILE > ./config.toml > ./config/default.toml > defaults
    let config = AppConfig::load_with_layers().expect("Failed to load configuration");

    // Each node can have its own flawless server (optional)
    let flawless_url = config.flawless.flawless_url.clone();

    let module = if std::env::var("SKIP_FLAWLESS").is_ok() {
        tracing::info!("Skipping Flawless deployment (SKIP_FLAWLESS set)");
        None
    } else {
        tracing::info!("Connecting to flawless server at {}", flawless_url);
        match async {
            let flawless = flawless_utils::Server::new(&flawless_url, None);
            let flawless_module = flawless_utils::load_module_from_build!("module1");
            flawless.deploy(flawless_module).await
        }.await {
            Ok(m) => Some(m),
            Err(e) => {
                tracing::warn!("Failed to deploy Flawless module: {}. Continuing without Flawless support.", e);
                None
            }
        }
    };

    // Initialize iroh endpoint with HTTP/3 ALPN
    let endpoint = iroh::Endpoint::builder()
        .alpns(vec![config.network.iroh_alpn_bytes()])
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

    // Start VM service if available
    #[cfg(feature = "vm-backend")]
    {
        println!("Starting VM Service...");
        if let Err(e) = state.vm_service().start().await {
            println!("WARNING: VM Service failed to start: {}. Continuing without VM support.", e);
        }
    }
    #[cfg(not(feature = "vm-backend"))]
    {
        println!("VM backend not available (feature disabled)");
    }

    // Display work queue ticket for worker connections
    let work_ticket = state.work_queue().get_ticket();

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
