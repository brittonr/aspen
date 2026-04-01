//! Cluster operations via `AspenClient` typed RPCs.

use std::time::Duration;

use aspen_client::AspenClient;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::HealthResponse;
use tracing::info;

use crate::RunConfig;
use crate::error::DogfoodResult;
use crate::error::HealthCheckSnafu;
use crate::node::NodeInfo;
use crate::node::NodeManager;
use crate::node::wait_for_ticket;

// ── Typed RPC wrappers ───────────────────────────────────────────────

/// Send `GetHealth` and extract the response.
pub async fn check_health(ticket: &str) -> DogfoodResult<HealthResponse> {
    let client = connect(ticket).await?;
    let resp = send(&client, ClientRpcRequest::GetHealth, "GetHealth", ticket).await?;

    match resp {
        ClientRpcResponse::Health(h) => Ok(h),
        other => HealthCheckSnafu {
            target: ticket_preview(ticket),
            reason: format!("unexpected response: {other:?}"),
        }
        .fail(),
    }
}

/// Send `InitCluster` RPC.
pub async fn init_cluster(client: &AspenClient, ticket: &str) -> DogfoodResult<()> {
    let _resp = send(client, ClientRpcRequest::InitCluster, "InitCluster", ticket).await?;
    Ok(())
}

/// Send `AddLearner` RPC (used for multi-node clusters).
#[allow(dead_code)]
pub async fn add_learner(client: &AspenClient, node_id: u64, addr: &str, ticket: &str) -> DogfoodResult<()> {
    let _resp = send(
        client,
        ClientRpcRequest::AddLearner {
            node_id,
            addr: addr.to_string(),
        },
        "AddLearner",
        ticket,
    )
    .await?;
    Ok(())
}

/// Send `ChangeMembership` RPC (used for multi-node clusters).
#[allow(dead_code)]
pub async fn change_membership(client: &AspenClient, members: Vec<u64>, ticket: &str) -> DogfoodResult<()> {
    let _resp = send(client, ClientRpcRequest::ChangeMembership { members }, "ChangeMembership", ticket).await?;
    Ok(())
}

/// Send `AddPeerCluster` RPC (federation).
pub async fn add_peer_cluster(client: &AspenClient, peer_ticket: &str, own_ticket: &str) -> DogfoodResult<()> {
    let _resp = send(
        client,
        ClientRpcRequest::AddPeerCluster {
            ticket: peer_ticket.to_string(),
        },
        "AddPeerCluster",
        own_ticket,
    )
    .await?;
    Ok(())
}

// ── Cluster lifecycle ────────────────────────────────────────────────

/// Start a single-node cluster: spawn, wait for ticket, health-check, init.
pub async fn start_single_node(manager: &mut NodeManager, config: &RunConfig) -> DogfoodResult<NodeInfo> {
    let data_dir = format!("{}/node1", config.cluster_dir);
    tokio::fs::create_dir_all(&data_dir).await.map_err(|e| crate::error::DogfoodError::ProcessSpawn {
        binary: format!("mkdir {data_dir}"),
        source: e,
    })?;

    let iroh_key = format!("{:0>64x}", 2001_u64);

    let vm_ci_env: Vec<(&str, &str)> = if config.vm_ci {
        vec![("ASPEN_CI_EXECUTOR", "vm")]
    } else {
        vec![]
    };

    let pid = manager.spawn_node(config, 1, "node1", &data_dir, &iroh_key, &vm_ci_env).await?;

    info!("  waiting for node to start...");
    let ticket = wait_for_ticket(&data_dir, Duration::from_secs(30)).await?;

    // Brief pause: the ticket is written right after the router spawns, but iroh's
    // QUIC endpoint needs a moment to fully bind and start accepting connections.
    tokio::time::sleep(Duration::from_secs(3)).await;

    info!("  health-checking node...");
    wait_for_healthy(&ticket, Duration::from_secs(60)).await?;

    info!("  initializing cluster...");
    let client = connect(&ticket).await?;
    // InitCluster may fail if already initialized — that's fine
    let _ = init_cluster(&client, &ticket).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let health = check_health(&ticket).await?;
    let endpoint_addr = health.iroh_node_id.unwrap_or_default();

    Ok(NodeInfo {
        pid,
        ticket,
        endpoint_addr,
    })
}

/// Start a federation: two independent clusters with peer trust.
pub async fn start_federation(manager: &mut NodeManager, config: &RunConfig) -> DogfoodResult<(NodeInfo, NodeInfo)> {
    // Alice: node-id 1
    let alice_dir = format!("{}/alice", config.cluster_dir);
    tokio::fs::create_dir_all(&alice_dir).await.map_err(|e| crate::error::DogfoodError::ProcessSpawn {
        binary: format!("mkdir {alice_dir}"),
        source: e,
    })?;

    let alice_key = format!("{:0>64x}", 3001_u64);
    let vm_ci_env: Vec<(&str, &str)> = if config.vm_ci {
        vec![("ASPEN_CI_EXECUTOR", "vm")]
    } else {
        vec![]
    };

    let alice_pid = manager.spawn_node(config, 1, "alice", &alice_dir, &alice_key, &vm_ci_env).await?;

    // Bob: node-id 1 (separate cluster)
    let bob_dir = format!("{}/bob", config.cluster_dir);
    tokio::fs::create_dir_all(&bob_dir).await.map_err(|e| crate::error::DogfoodError::ProcessSpawn {
        binary: format!("mkdir {bob_dir}"),
        source: e,
    })?;

    let bob_key = format!("{:0>64x}", 3002_u64);
    let bob_pid = manager.spawn_node(config, 1, "bob", &bob_dir, &bob_key, &vm_ci_env).await?;

    // Wait for both tickets
    info!("  waiting for alice...");
    let alice_ticket = wait_for_ticket(&alice_dir, Duration::from_secs(30)).await?;
    info!("  waiting for bob...");
    let bob_ticket = wait_for_ticket(&bob_dir, Duration::from_secs(30)).await?;

    // Let QUIC endpoints stabilize before connecting
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Health-check both
    wait_for_healthy(&alice_ticket, Duration::from_secs(60)).await?;
    wait_for_healthy(&bob_ticket, Duration::from_secs(60)).await?;

    // Initialize both clusters
    let alice_client = connect(&alice_ticket).await?;
    let _ = init_cluster(&alice_client, &alice_ticket).await;
    let bob_client = connect(&bob_ticket).await?;
    let _ = init_cluster(&bob_client, &bob_ticket).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Establish federation trust
    info!("  establishing federation trust...");
    add_peer_cluster(&alice_client, &bob_ticket, &alice_ticket).await?;
    add_peer_cluster(&bob_client, &alice_ticket, &bob_ticket).await?;

    let alice_health = check_health(&alice_ticket).await?;
    let bob_health = check_health(&bob_ticket).await?;

    Ok((
        NodeInfo {
            pid: alice_pid,
            ticket: alice_ticket,
            endpoint_addr: alice_health.iroh_node_id.unwrap_or_default(),
        },
        NodeInfo {
            pid: bob_pid,
            ticket: bob_ticket,
            endpoint_addr: bob_health.iroh_node_id.unwrap_or_default(),
        },
    ))
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Connect an `AspenClient` from a ticket string.
async fn connect(ticket: &str) -> DogfoodResult<AspenClient> {
    // 30s RPC timeout matches the CLI default. First QUIC connection to a
    // relay-disabled node may take 5-10s while iroh attempts relay then
    // falls back to direct addresses.
    AspenClient::connect(ticket, Duration::from_secs(30), None).await.map_err(|e| {
        crate::error::DogfoodError::ClientRpc {
            operation: "connect".to_string(),
            target: ticket_preview(ticket),
            source: e,
        }
    })
}

/// Send an RPC and map transport errors.
async fn send(
    client: &AspenClient,
    request: ClientRpcRequest,
    operation: &str,
    ticket: &str,
) -> DogfoodResult<ClientRpcResponse> {
    client.send(request).await.map_err(|e| crate::error::DogfoodError::ClientRpc {
        operation: operation.to_string(),
        target: ticket_preview(ticket),
        source: e,
    })
}

/// Poll `GetHealth` until the node reports "healthy".
///
/// Creates a fresh client per attempt. This avoids iroh caching a failed
/// connection state from early attempts when the node is still starting.
async fn wait_for_healthy(ticket: &str, timeout: Duration) -> DogfoodResult<()> {
    let start = tokio::time::Instant::now();
    let mut delay = Duration::from_secs(2);

    loop {
        match check_health(ticket).await {
            Ok(h) if h.status == "healthy" => return Ok(()),
            Ok(h) => {
                // Connected but not healthy yet (e.g. cluster not initialized).
                // For single-node startup the caller will init, so "unhealthy" is
                // fine as long as we can reach the node.
                info!("  node reachable (status={})", h.status);
                return Ok(());
            }
            Err(e) => {
                info!("  health check attempt failed: {e:#}");
            }
        }

        if start.elapsed() > timeout {
            return crate::error::HealthCheckSnafu {
                target: ticket_preview(ticket),
                reason: format!("not healthy after {}s", timeout.as_secs()),
            }
            .fail();
        }

        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(Duration::from_secs(10));
    }
}

/// Truncate a ticket for log display.
pub(crate) fn ticket_preview(ticket: &str) -> String {
    let len = ticket.len().min(24);
    format!("{}...", &ticket[..len])
}
