//! Aspen node binary - cluster node entry point.
//!
//! Production binary for running Aspen cluster nodes with Iroh Client RPC for
//! all cluster operations and key-value access. Supports Raft control plane backend
//! and both in-memory and persistent storage. Configuration is loaded from
//! environment variables, TOML files, or CLI arguments.
//!
//! # Architecture
//!
//! - Iroh Client RPC: All client API operations via QUIC/P2P
//! - Raft control plane: Distributed consensus for cluster management
//! - Graceful shutdown: SIGTERM/SIGINT handling with coordinated cleanup
//! - Configuration layers: Environment < TOML < CLI args
//!
//! # Client API (Iroh Client RPC)
//!
//! All client operations are accessed via Iroh Client RPC (ALPN: `aspen-tui`).
//! See `ClientRpcRequest` in `src/client_rpc.rs` for the full API.
//!
//! Control Plane:
//! - InitCluster - Initialize new cluster
//! - AddLearner - Add learner node
//! - ChangeMembership - Promote learners to voters
//! - GetClusterState - Get current cluster topology
//!
//! Key-Value:
//! - ReadKey - Read key (linearizable)
//! - WriteKey - Write key-value (replicated)
//! - DeleteKey - Delete key
//! - ScanKeys - Scan keys by prefix
//!
//! Monitoring:
//! - GetHealth - Health check
//! - GetMetrics - Prometheus-compatible metrics
//! - GetRaftMetrics - Detailed Raft metrics
//!
//! # Tiger Style
//!
//! - Explicit types: u64 for node_id, addresses are type-safe
//! - Fixed limits: Raft batch sizes are bounded
//! - Resource management: Arc for shared state, graceful shutdown cleans up
//! - Error handling: Anyhow for application errors
//! - Fail fast: Configuration validation before server starts
//!
//! # Usage
//!
//! ```bash
//! # Start node with TOML config
//! aspen-node --config /etc/aspen/node.toml
//!
//! # Start node with CLI args
//! aspen-node --node-id 1
//!
//! # Environment variables
//! export ASPEN_NODE_ID=1
//! aspen-node
//! ```

mod args;
mod config;
mod node_mode;
mod setup;
mod signals;
mod worker_only;

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen::ClientProtocolHandler;
use aspen::cluster::bootstrap::initialize_blob_replication;
use aspen_core::context::InMemoryWatchRegistry;
use aspen_core::context::WatchRegistry;
use tracing::error;
use tracing::info;

use crate::config::initialize_and_load_config;
use crate::node_mode::bootstrap_node_and_generate_token;
use crate::node_mode::extract_node_components;
use crate::setup::print_cluster_ticket;
use crate::setup::setup_client_protocol;
use crate::setup::setup_router;
use crate::setup::start_dns_server;
use crate::signals::shutdown_signal;

/// Main entry point for aspen-node.
///
/// Uses a custom tokio runtime with increased worker thread stack size (16 MiB)
/// to handle deep async call chains in Raft replication and CI pipeline execution.
/// The default 2 MiB stack is insufficient for debug builds with complex async
/// state machines spanning multiple subsystems (raft -> kv -> workflow -> jobs).
fn main() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(16 * 1024 * 1024) // 16 MiB stack per worker
        .build()
        .context("failed to build tokio runtime")?;

    runtime.block_on(async_main())
}

async fn async_main() -> Result<()> {
    let (args, config) = initialize_and_load_config().await?;

    // Display version info with git hash
    let git_hash = env!("GIT_HASH", "GIT_HASH not set");
    let build_time = env!("BUILD_TIME", "BUILD_TIME not set");

    // Check for worker-only mode (via flag or environment variable)
    #[cfg(feature = "ci")]
    {
        let worker_only_mode = args.worker_only
            || std::env::var("ASPEN_MODE")
                .map(|v| v.to_lowercase() == "ci_worker" || v.to_lowercase() == "worker_only")
                .unwrap_or(false);

        if worker_only_mode {
            info!(
                git_hash = git_hash,
                build_time = build_time,
                "starting aspen CI worker v{} ({}) in worker-only mode",
                env!("CARGO_PKG_VERSION"),
                git_hash
            );
            return worker_only::run_worker_only_mode(args, config).await;
        }
    }

    // Without "ci" feature, warn if worker-only mode was requested
    #[cfg(not(feature = "ci"))]
    if args.worker_only {
        warn!("worker-only mode requested but 'ci' feature is not enabled - running as regular node");
    }

    info!(
        node_id = config.node_id,
        control_backend = ?config.control_backend,
        sharding_enabled = config.sharding.is_enabled,
        git_hash = git_hash,
        build_time = build_time,
        "starting aspen node v{} ({})",
        env!("CARGO_PKG_VERSION"),
        git_hash
    );

    // Bootstrap the node based on sharding configuration
    let mut node_mode = bootstrap_node_and_generate_token(&args, &config).await?;

    // Extract controller, kv_store, and primary_raft_node from node_mode
    let (controller, kv_store, primary_raft_node, network_factory) = extract_node_components(&config, &node_mode)?;

    // Initialize blob replication if enabled (non-sharded mode only)
    if node_mode.blob_replication_mut().is_some() {
        let blob_store = node_mode.blob_store().cloned();
        let endpoint = Some(node_mode.iroh_manager().endpoint().clone());
        let shutdown_token = node_mode.shutdown_token();
        let node_config = node_mode.config().clone();

        let blob_events = blob_store.as_ref().and_then(|bs| bs.broadcaster()).map(|b| b.subscribe());

        let replication_resources = initialize_blob_replication(
            &node_config,
            blob_store,
            endpoint,
            primary_raft_node.clone(),
            blob_events,
            shutdown_token,
        )
        .await;

        let has_manager = replication_resources.replication_manager.is_some();
        if let Some(blob_replication) = node_mode.blob_replication_mut() {
            *blob_replication = replication_resources;

            if has_manager {
                let metrics_rx = primary_raft_node.raft().metrics();
                let extractor: aspen_blob::replication::topology_watcher::NodeInfoExtractor<_> =
                    Box::new(|metrics: &openraft::RaftMetrics<aspen_raft::types::AppTypeConfig>| {
                        metrics
                            .membership_config
                            .membership()
                            .nodes()
                            .map(|(node_id, node)| {
                                aspen_blob::replication::NodeInfo::new(u64::from(*node_id), node.iroh_addr.id)
                            })
                            .collect()
                    });
                blob_replication.wire_topology_watcher(metrics_rx, extractor);
            }
        }
    }

    // Create shared watch registry for tracking active subscriptions
    let watch_registry: Arc<dyn WatchRegistry> = Arc::new(InMemoryWatchRegistry::new());

    let (_token_verifier, client_context, _worker_service_handle, worker_coordinator) = setup_client_protocol(
        &args,
        &config,
        &node_mode,
        &controller,
        &kv_store,
        &primary_raft_node,
        &network_factory,
        watch_registry.clone(),
    )
    .await?;

    #[cfg(feature = "wasm-plugins")]
    let mut client_handler = ClientProtocolHandler::new(client_context);
    #[cfg(not(feature = "wasm-plugins"))]
    let client_handler = ClientProtocolHandler::new(client_context);

    #[cfg(feature = "wasm-plugins")]
    client_handler.load_wasm_plugins().await;

    // Spawn the Router with all protocol handlers
    let router = setup_router(&config, &node_mode, client_handler, watch_registry, kv_store.clone());

    // Get fresh endpoint address (may have discovered more addresses since startup)
    let endpoint_addr = node_mode.iroh_manager().endpoint().addr();
    info!(
        endpoint_id = %endpoint_addr.id,
        sharding = config.sharding.is_enabled,
        "Iroh Router spawned - all client API available via Iroh Client RPC (ALPN: aspen-tui)"
    );

    // Start DNS protocol server if enabled
    start_dns_server(&config).await;

    // Generate and print cluster ticket (V2 with direct addresses)
    print_cluster_ticket(&config, &endpoint_addr);

    // Wait for shutdown signal
    shutdown_signal().await;

    // Stop distributed worker coordinator if started
    if let Some(ref coordinator) = worker_coordinator {
        info!("stopping distributed worker coordinator");
        if let Err(e) = coordinator.stop().await {
            error!(error = %e, "failed to stop distributed worker coordinator");
        }
    }

    // Gracefully shutdown Iroh Router
    info!("shutting down Iroh Router");
    router.shutdown().await?;

    // Gracefully shutdown the node
    node_mode.shutdown().await?;

    Ok(())
}
