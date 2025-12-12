//! Test: Enhanced Health Endpoint
//!
//! Validates that the enhanced /health endpoint returns detailed component status.

use std::sync::Arc;

use aspen::api::{ClusterController, KeyValueStore};
use aspen::cluster::bootstrap::bootstrap_node;
use aspen::cluster::config::{ControlBackend, IrohConfig, NodeConfig};
use aspen::raft::RaftActorMessage;
use axum::{Router, routing::get};
use ractor::ActorRef;
use reqwest::StatusCode;
use serde_json::Value;

// Test helper to start HTTP server with health endpoint
#[allow(clippy::too_many_arguments)]
async fn start_test_server(
    http_addr: std::net::SocketAddr,
    node_id: u64,
    raft_actor: ActorRef<RaftActorMessage>,
    ractor_port: u16,
    controller: Arc<dyn ClusterController>,
    kv: Arc<dyn KeyValueStore>,
    network_factory: Arc<aspen::raft::network::IrpcRaftNetworkFactory>,
    iroh_manager: Arc<aspen::cluster::IrohEndpointManager>,
    data_dir: Option<std::path::PathBuf>,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
    // This is a simplified version of the AppState from aspen-node.rs
    #[derive(Clone)]
    struct AppState {
        node_id: u64,
        raft_actor: ActorRef<RaftActorMessage>,
        _ractor_port: u16,
        controller: Arc<dyn ClusterController>,
        _kv: Arc<dyn KeyValueStore>,
        _network_factory: Arc<aspen::raft::network::IrpcRaftNetworkFactory>,
        _iroh_manager: Arc<aspen::cluster::IrohEndpointManager>,
        data_dir: Option<std::path::PathBuf>,
    }

    let app_state = AppState {
        node_id,
        raft_actor,
        _ractor_port: ractor_port,
        controller,
        _kv: kv,
        _network_factory: network_factory,
        _iroh_manager: iroh_manager,
        data_dir,
    };

    // Import the health handler directly from aspen-node binary
    // Since we can't import from a binary, we need to inline the health check logic here
    use axum::{Json, extract::State, response::IntoResponse};
    use ractor::call_t;
    use serde::Serialize;

    #[derive(Serialize)]
    struct DetailedHealthResponse {
        status: String,
        checks: HealthChecks,
        node_id: u64,
        raft_node_id: Option<u64>,
    }

    #[derive(Serialize)]
    struct HealthChecks {
        raft_actor: HealthCheckStatus,
        raft_cluster: HealthCheckStatus,
        disk_space: HealthCheckStatus,
        storage: HealthCheckStatus,
    }

    #[derive(Serialize)]
    struct HealthCheckStatus {
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    }

    async fn health(State(ctx): State<AppState>) -> impl IntoResponse {
        // Check 1: Raft actor responsiveness
        let (raft_actor_check, raft_node_id) =
            match call_t!(ctx.raft_actor, RaftActorMessage::GetNodeId, 25) {
                Ok(id) => (
                    HealthCheckStatus {
                        status: "ok".to_string(),
                        message: None,
                    },
                    Some(id),
                ),
                Err(err) => (
                    HealthCheckStatus {
                        status: "error".to_string(),
                        message: Some(format!("Raft actor not responding: {}", err)),
                    },
                    None,
                ),
            };

        // Check 2: Raft cluster has leader
        let raft_cluster_check = match ctx.controller.get_metrics().await {
            Ok(metrics) => {
                if metrics.current_leader.is_some() {
                    HealthCheckStatus {
                        status: "ok".to_string(),
                        message: Some(format!("Leader: {:?}", metrics.current_leader)),
                    }
                } else {
                    HealthCheckStatus {
                        status: "warning".to_string(),
                        message: Some("No leader elected".to_string()),
                    }
                }
            }
            Err(_) => HealthCheckStatus {
                status: "warning".to_string(),
                message: Some("Unable to get Raft metrics".to_string()),
            },
        };

        // Check 3: Disk space availability
        let disk_space_check = if let Some(ref data_dir) = ctx.data_dir {
            match aspen::utils::check_disk_space(data_dir) {
                Ok(disk_space) => {
                    if disk_space.usage_percent >= aspen::utils::DISK_USAGE_THRESHOLD_PERCENT {
                        HealthCheckStatus {
                            status: "error".to_string(),
                            message: Some(format!(
                                "Disk usage critical: {}% (threshold: {}%)",
                                disk_space.usage_percent,
                                aspen::utils::DISK_USAGE_THRESHOLD_PERCENT
                            )),
                        }
                    } else if disk_space.usage_percent >= 80 {
                        HealthCheckStatus {
                            status: "warning".to_string(),
                            message: Some(format!(
                                "Disk usage high: {}%",
                                disk_space.usage_percent
                            )),
                        }
                    } else {
                        HealthCheckStatus {
                            status: "ok".to_string(),
                            message: Some(format!("Disk usage: {}%", disk_space.usage_percent)),
                        }
                    }
                }
                Err(e) => HealthCheckStatus {
                    status: "warning".to_string(),
                    message: Some(format!("Unable to check disk space: {}", e)),
                },
            }
        } else {
            HealthCheckStatus {
                status: "ok".to_string(),
                message: Some("In-memory storage (no disk check needed)".to_string()),
            }
        };

        // Check 4: Storage writability
        let storage_check = match ctx.controller.get_metrics().await {
            Ok(_) => HealthCheckStatus {
                status: "ok".to_string(),
                message: None,
            },
            Err(e) => HealthCheckStatus {
                status: "warning".to_string(),
                message: Some(format!("Storage health check failed: {}", e)),
            },
        };

        // Determine overall health status
        let is_critical_failure =
            raft_actor_check.status == "error" || disk_space_check.status == "error";
        let has_warnings = raft_cluster_check.status == "warning"
            || disk_space_check.status == "warning"
            || storage_check.status == "warning";

        let (overall_status, http_status) = if is_critical_failure {
            ("unhealthy".to_string(), StatusCode::SERVICE_UNAVAILABLE)
        } else if has_warnings {
            ("degraded".to_string(), StatusCode::OK)
        } else {
            ("healthy".to_string(), StatusCode::OK)
        };

        let response = DetailedHealthResponse {
            status: overall_status,
            checks: HealthChecks {
                raft_actor: raft_actor_check,
                raft_cluster: raft_cluster_check,
                disk_space: disk_space_check,
                storage: storage_check,
            },
            node_id: ctx.node_id,
            raft_node_id,
        };

        (http_status, Json(response)).into_response()
    }

    let app = Router::new()
        .route("/health", get(health))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(http_addr).await?;

    let server_task = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("server failed");
    });

    Ok(server_task)
}

#[tokio::test]
async fn test_health_endpoint_detailed_response() -> anyhow::Result<()> {
    // Start a single node
    let temp_dir = tempfile::tempdir()?;

    let config = NodeConfig {
        node_id: 1,
        control_backend: ControlBackend::RaftActor,
        host: "127.0.0.1".to_string(),
        http_addr: "127.0.0.1:49000".parse()?, // Fixed port for testing
        ractor_port: 0,
        data_dir: Some(temp_dir.path().to_path_buf()),
        cookie: "health-test".to_string(),
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle = bootstrap_node(config.clone()).await?;

    // Build controller and KV store
    let controller: Arc<dyn ClusterController> = Arc::new(aspen::raft::RaftControlClient::new(
        handle.raft_actor.clone(),
    ));
    let kv: Arc<dyn KeyValueStore> =
        Arc::new(aspen::node::NodeClient::new(handle.raft_actor.clone()));

    // Start HTTP server
    let _server_task = start_test_server(
        config.http_addr,
        config.node_id,
        handle.raft_actor.clone(),
        config.ractor_port,
        controller,
        kv,
        handle.network_factory.clone(),
        handle.iroh_manager.clone(),
        config.data_dir.clone(),
    )
    .await?;

    // Health endpoint URL
    let health_url = "http://127.0.0.1:49000/health";

    // Wait for the server to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Call the health endpoint
    let client = reqwest::Client::new();
    let response = client.get(health_url).send().await?;

    // Should return 200 or 503 (depending on cluster state)
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "health endpoint should return 200 or 503, got {}",
        response.status()
    );

    // Parse JSON response
    let json: Value = response.json().await?;

    // Verify structure
    assert!(json.get("status").is_some(), "should have status field");
    assert!(json.get("checks").is_some(), "should have checks field");
    assert!(json.get("node_id").is_some(), "should have node_id field");

    // Verify checks structure
    let checks = json.get("checks").unwrap();
    assert!(
        checks.get("raft_actor").is_some(),
        "should have raft_actor check"
    );
    assert!(
        checks.get("raft_cluster").is_some(),
        "should have raft_cluster check"
    );
    assert!(
        checks.get("disk_space").is_some(),
        "should have disk_space check"
    );
    assert!(checks.get("storage").is_some(), "should have storage check");

    // Verify each check has a status
    assert!(
        checks["raft_actor"].get("status").is_some(),
        "raft_actor check should have status"
    );
    assert!(
        checks["raft_cluster"].get("status").is_some(),
        "raft_cluster check should have status"
    );
    assert!(
        checks["disk_space"].get("status").is_some(),
        "disk_space check should have status"
    );
    assert!(
        checks["storage"].get("status").is_some(),
        "storage check should have status"
    );

    // Verify node_id matches
    assert_eq!(json["node_id"], 1, "node_id should be 1");

    // Verify overall status is one of the valid values
    let status = json["status"].as_str().unwrap();
    assert!(
        status == "healthy" || status == "degraded" || status == "unhealthy",
        "status should be healthy, degraded, or unhealthy, got: {}",
        status
    );

    println!(
        "Health endpoint response: {}",
        serde_json::to_string_pretty(&json)?
    );

    // Cleanup
    handle.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_health_endpoint_disk_space_check() -> anyhow::Result<()> {
    // Start a node with data directory
    let temp_dir = tempfile::tempdir()?;

    let config = NodeConfig {
        node_id: 2,
        control_backend: ControlBackend::RaftActor,
        host: "127.0.0.1".to_string(),
        http_addr: "127.0.0.1:49001".parse()?, // Fixed port for testing
        ractor_port: 0,
        data_dir: Some(temp_dir.path().to_path_buf()),
        cookie: "health-disk-test".to_string(),
        heartbeat_interval_ms: 500,
        election_timeout_min_ms: 1500,
        election_timeout_max_ms: 3000,
        iroh: IrohConfig::default(),
        peers: vec![],
        storage_backend: aspen::raft::storage::StorageBackend::default(),
        redb_log_path: None,
        redb_sm_path: None,
        sqlite_log_path: None,
        sqlite_sm_path: None,
        supervision_config: aspen::raft::supervision::SupervisionConfig::default(),
        raft_mailbox_capacity: 1000,
    };

    let handle = bootstrap_node(config.clone()).await?;

    // Build controller and KV store
    let controller: Arc<dyn ClusterController> = Arc::new(aspen::raft::RaftControlClient::new(
        handle.raft_actor.clone(),
    ));
    let kv: Arc<dyn KeyValueStore> =
        Arc::new(aspen::node::NodeClient::new(handle.raft_actor.clone()));

    // Start HTTP server
    let _server_task = start_test_server(
        config.http_addr,
        config.node_id,
        handle.raft_actor.clone(),
        config.ractor_port,
        controller,
        kv,
        handle.network_factory.clone(),
        handle.iroh_manager.clone(),
        config.data_dir.clone(),
    )
    .await?;

    let health_url = "http://127.0.0.1:49001/health";

    // Wait for the server to be ready
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Call health endpoint
    let client = reqwest::Client::new();
    let response = client.get(health_url).send().await?;
    let json: Value = response.json().await?;

    // Disk space check should exist and have a message
    let disk_check = &json["checks"]["disk_space"];
    assert!(
        disk_check.get("status").is_some(),
        "disk_space should have status"
    );
    assert!(
        disk_check.get("message").is_some(),
        "disk_space should have message with percentage"
    );

    // Message should contain percentage
    let message = disk_check["message"].as_str().unwrap();
    assert!(
        message.contains("%"),
        "disk space message should contain percentage"
    );

    println!(
        "Disk space check: {}",
        serde_json::to_string_pretty(&disk_check)?
    );

    // Cleanup
    handle.shutdown().await?;

    Ok(())
}
