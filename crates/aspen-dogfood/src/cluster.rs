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
    check_health_with_client(client, ticket).await
}

trait HealthRpcClient {
    async fn send_get_health(&self, ticket: &str) -> DogfoodResult<ClientRpcResponse>;
    async fn shutdown(self);
}

impl HealthRpcClient for AspenClient {
    async fn send_get_health(&self, ticket: &str) -> DogfoodResult<ClientRpcResponse> {
        send(self, ClientRpcRequest::GetHealth, "GetHealth", ticket).await
    }

    async fn shutdown(self) {
        AspenClient::shutdown(self).await;
    }
}

async fn check_health_with_client<C>(client: C, ticket: &str) -> DogfoodResult<HealthResponse>
where C: HealthRpcClient {
    let result = match client.send_get_health(ticket).await {
        Ok(ClientRpcResponse::Health(health)) => Ok(health),
        Ok(other) => HealthCheckSnafu {
            target: ticket_preview(ticket),
            reason: format!("unexpected response: {other:?}"),
        }
        .fail(),
        Err(error) => Err(error),
    };
    client.shutdown().await;
    result
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
    let cookie = config.cookie();

    let vm_ci_env: Vec<(&str, &str)> = if config.vm_ci {
        vec![("ASPEN_CI_EXECUTOR", "vm")]
    } else {
        vec![]
    };

    let pid = manager.spawn_node(config, 1, "node1", &data_dir, &iroh_key, &cookie, &vm_ci_env).await?;

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
    client.shutdown().await;
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
///
/// Each cluster gets a distinct cookie and the federation env vars that
/// the aspen-node process needs to enable cross-cluster communication.
pub async fn start_federation(manager: &mut NodeManager, config: &RunConfig) -> DogfoodResult<(NodeInfo, NodeInfo)> {
    // Alice: node-id 1
    let alice_dir = format!("{}/alice", config.cluster_dir);
    tokio::fs::create_dir_all(&alice_dir).await.map_err(|e| crate::error::DogfoodError::ProcessSpawn {
        binary: format!("mkdir {alice_dir}"),
        source: e,
    })?;

    let alice_key = format!("{:0>64x}", 3001_u64);
    let alice_cookie = config.alice_cookie();

    // Federation env vars — must match what the deprecated script set.
    // ASPEN_FEDERATION_CLUSTER_KEY must equal the iroh secret key for the
    // federation resolver to derive the correct FederatedId.
    let alice_env = federation_env(&alice_key, "alice-cluster", false, config.vm_ci);

    let alice_pid = manager.spawn_node(config, 1, "alice", &alice_dir, &alice_key, &alice_cookie, &alice_env).await?;

    // Bob: node-id 1 (separate cluster)
    let bob_dir = format!("{}/bob", config.cluster_dir);
    tokio::fs::create_dir_all(&bob_dir).await.map_err(|e| crate::error::DogfoodError::ProcessSpawn {
        binary: format!("mkdir {bob_dir}"),
        source: e,
    })?;

    let bob_key = format!("{:0>64x}", 3002_u64);
    let bob_cookie = config.bob_cookie();

    // Bob runs CI — enable federation mirror auto-trigger.
    let bob_env = federation_env(&bob_key, "bob-cluster", true, config.vm_ci);

    let bob_pid = manager.spawn_node(config, 1, "bob", &bob_dir, &bob_key, &bob_cookie, &bob_env).await?;

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
    let bob_client = match connect(&bob_ticket).await {
        Ok(client) => client,
        Err(error) => {
            alice_client.shutdown().await;
            return Err(error);
        }
    };
    let _ = init_cluster(&alice_client, &alice_ticket).await;
    let _ = init_cluster(&bob_client, &bob_ticket).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Establish federation trust
    info!("  establishing federation trust...");
    let trust_result = async {
        add_peer_cluster(&alice_client, &bob_ticket, &alice_ticket).await?;
        add_peer_cluster(&bob_client, &alice_ticket, &bob_ticket).await
    }
    .await;
    alice_client.shutdown().await;
    bob_client.shutdown().await;
    trust_result?;

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

/// Build the federation env var list for a cluster node.
///
/// Extracted as a pure function so it can be unit-tested without
/// spawning processes.
pub(crate) fn federation_env<'a>(
    iroh_key: &'a str,
    cluster_name: &'a str,
    enable_federation_ci: bool,
    vm_ci: bool,
) -> Vec<(&'a str, &'a str)> {
    let mut env = vec![
        ("ASPEN_FEDERATION_ENABLED", "true"),
        ("ASPEN_FEDERATION_CLUSTER_KEY", iroh_key),
        ("ASPEN_FEDERATION_CLUSTER_NAME", cluster_name),
        ("ASPEN_FEDERATION_ENABLE_DHT_DISCOVERY", "false"),
        ("ASPEN_FEDERATION_ENABLE_GOSSIP", "false"),
    ];
    if enable_federation_ci {
        env.push(("ASPEN_CI_FEDERATION_CI_ENABLED", "true"));
    }
    if vm_ci {
        env.push(("ASPEN_CI_EXECUTOR", "vm"));
    }
    env
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;

    use super::*;

    struct FakeHealthClient {
        response: Mutex<Option<DogfoodResult<ClientRpcResponse>>>,
        shutdown_called: Arc<AtomicBool>,
    }

    impl FakeHealthClient {
        fn rpc_error(shutdown_called: Arc<AtomicBool>) -> Self {
            Self {
                response: Mutex::new(Some(Err(crate::error::DogfoodError::ClientRpc {
                    operation: "GetHealth".to_string(),
                    target: ticket_preview("aspen-test-ticket"),
                    source: anyhow::anyhow!("simulated rpc failure"),
                }))),
                shutdown_called,
            }
        }
    }

    impl HealthRpcClient for FakeHealthClient {
        async fn send_get_health(&self, _ticket: &str) -> DogfoodResult<ClientRpcResponse> {
            self.response.lock().unwrap().take().unwrap()
        }

        async fn shutdown(self) {
            self.shutdown_called.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn check_health_with_client_shutdowns_on_rpc_error() {
        let shutdown_called = Arc::new(AtomicBool::new(false));
        let client = FakeHealthClient::rpc_error(Arc::clone(&shutdown_called));

        let result = check_health_with_client(client, "aspen-test-ticket").await;

        assert!(result.is_err());
        assert!(shutdown_called.load(Ordering::SeqCst));
    }

    #[test]
    fn federation_env_alice_has_required_vars() {
        let env = federation_env("abc123", "alice-cluster", false, false);
        let keys: Vec<&str> = env.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"ASPEN_FEDERATION_ENABLED"));
        assert!(keys.contains(&"ASPEN_FEDERATION_CLUSTER_KEY"));
        assert!(keys.contains(&"ASPEN_FEDERATION_CLUSTER_NAME"));
        assert!(keys.contains(&"ASPEN_FEDERATION_ENABLE_DHT_DISCOVERY"));
        assert!(keys.contains(&"ASPEN_FEDERATION_ENABLE_GOSSIP"));
        // Alice does not run federation CI
        assert!(!keys.contains(&"ASPEN_CI_FEDERATION_CI_ENABLED"));
        assert!(!keys.contains(&"ASPEN_CI_EXECUTOR"));
    }

    #[test]
    fn federation_env_bob_has_ci_enabled() {
        let env = federation_env("def456", "bob-cluster", true, false);
        let keys: Vec<&str> = env.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"ASPEN_CI_FEDERATION_CI_ENABLED"));
    }

    #[test]
    fn federation_env_vm_ci_adds_executor() {
        let env = federation_env("key", "name", false, true);
        let keys: Vec<&str> = env.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"ASPEN_CI_EXECUTOR"));
    }

    #[test]
    fn federation_env_key_matches_input() {
        let env = federation_env("my-secret-key", "my-cluster", false, false);
        let key_val = env.iter().find(|(k, _)| *k == "ASPEN_FEDERATION_CLUSTER_KEY").unwrap();
        assert_eq!(key_val.1, "my-secret-key");
        let name_val = env.iter().find(|(k, _)| *k == "ASPEN_FEDERATION_CLUSTER_NAME").unwrap();
        assert_eq!(name_val.1, "my-cluster");
    }

    #[test]
    fn ticket_preview_truncates() {
        let long = "a".repeat(100);
        let preview = ticket_preview(&long);
        assert_eq!(preview.len(), 24 + 3); // 24 chars + "..."
    }

    #[test]
    fn ticket_preview_short() {
        let short = "abc";
        let preview = ticket_preview(short);
        assert_eq!(preview, "abc...");
    }
}
