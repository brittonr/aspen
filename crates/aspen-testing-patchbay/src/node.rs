//! Minimal Aspen node for patchbay testing.
//!
//! Provides a stripped-down node (iroh endpoint + Raft + KV store) that
//! boots inside a patchbay device namespace. A channel-based `NodeHandle`
//! lets the test thread interact with the spawned node.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_cluster_types::ClusterNode;
use aspen_cluster_types::InitRequest;
use aspen_raft::StateMachineVariant;
use aspen_raft::network::IrpcRaftNetworkFactory;
use aspen_raft::node::RaftNode;
use aspen_raft::storage::InMemoryLogStore;
use aspen_raft::storage::InMemoryStateMachine;
use aspen_raft::types::AppTypeConfig;
use aspen_raft::types::NodeId;
use aspen_traits::ClusterController;
use aspen_traits::KeyValueStore;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::SecretKey;
use openraft::Raft;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::info;

use crate::transport::TestTransport;

/// Maximum pending commands in the node's command channel.
const CMD_CHANNEL_CAPACITY: usize = 64;

/// Commands sent from the test thread to a spawned node.
pub enum NodeCommand {
    /// Get this node's iroh EndpointAddr for peer discovery.
    GetAddr { reply: oneshot::Sender<EndpointAddr> },
    /// Register peer addresses (call before init_cluster).
    AddPeers {
        peers: HashMap<NodeId, EndpointAddr>,
        reply: oneshot::Sender<Result<()>>,
    },
    /// Initialize the Raft cluster with the given member set.
    InitCluster {
        members: Vec<ClusterNode>,
        reply: oneshot::Sender<Result<()>>,
    },
    /// Write a KV pair through Raft consensus.
    WriteKv {
        key: String,
        value: String,
        reply: oneshot::Sender<Result<()>>,
    },
    /// Read a value from the local state machine.
    ReadKv {
        key: String,
        reply: oneshot::Sender<Result<Option<String>>>,
    },
    /// Get the current Raft leader (if known).
    GetLeader { reply: oneshot::Sender<Option<NodeId>> },
    /// Get the last applied log index.
    GetAppliedIndex { reply: oneshot::Sender<Option<u64>> },
    /// Shut down the node.
    Shutdown { reply: oneshot::Sender<()> },
}

/// Handle for communicating with a spawned Aspen node from the test thread.
///
/// All operations are async and communicate over channels to the node
/// running inside a patchbay device namespace.
#[derive(Clone)]
pub struct NodeHandle {
    /// This node's ID.
    pub node_id: NodeId,
    cmd_tx: mpsc::Sender<NodeCommand>,
}

impl NodeHandle {
    /// Get this node's iroh EndpointAddr.
    pub async fn addr(&self) -> Result<EndpointAddr> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(NodeCommand::GetAddr { reply: tx }).await?;
        Ok(rx.await?)
    }

    /// Register peer addresses for Raft networking.
    pub async fn add_peers(&self, peers: HashMap<NodeId, EndpointAddr>) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(NodeCommand::AddPeers { peers, reply: tx }).await?;
        rx.await?
    }

    /// Initialize the Raft cluster.
    pub async fn init_cluster(&self, members: Vec<ClusterNode>) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(NodeCommand::InitCluster { members, reply: tx }).await?;
        rx.await?
    }

    /// Write a KV pair.
    pub async fn write_kv(&self, key: &str, value: &str) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(NodeCommand::WriteKv {
                key: key.to_string(),
                value: value.to_string(),
                reply: tx,
            })
            .await?;
        rx.await?
    }

    /// Read a value from this node's state machine.
    pub async fn read_kv(&self, key: &str) -> Result<Option<String>> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(NodeCommand::ReadKv {
                key: key.to_string(),
                reply: tx,
            })
            .await?;
        rx.await?
    }

    /// Get the current leader as seen by this node.
    pub async fn get_leader(&self) -> Result<Option<NodeId>> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(NodeCommand::GetLeader { reply: tx }).await?;
        Ok(rx.await?)
    }

    /// Get the last applied log index.
    pub async fn applied_index(&self) -> Result<Option<u64>> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(NodeCommand::GetAppliedIndex { reply: tx }).await?;
        Ok(rx.await?)
    }

    /// Shut down the node.
    pub async fn shutdown(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        let _ = self.cmd_tx.send(NodeCommand::Shutdown { reply: tx }).await;
        let _ = rx.await;
        Ok(())
    }
}

/// Bootstrap a minimal Aspen node and return its handle + command loop future.
///
/// The command loop future should be spawned on the device's namespace runtime
/// via `device.spawn()` to ensure the iroh endpoint binds to the correct
/// network namespace.
pub async fn bootstrap_node(node_id: NodeId) -> Result<(NodeHandle, impl std::future::Future<Output = ()>)> {
    // Create iroh endpoint inside the namespace
    let secret_key = SecretKey::generate(&mut rand::rng());
    let endpoint = Endpoint::builder()
        .secret_key(secret_key.clone())
        .relay_mode(iroh::RelayMode::Disabled)
        .alpns(vec![
            #[allow(deprecated)]
            aspen_transport::RAFT_ALPN.to_vec(),
        ])
        .bind()
        .await
        .context("failed to bind iroh endpoint")?;

    let node_addr = endpoint.addr();
    info!(
        node_id = node_id.0,
        node_key = %endpoint.id(),
        "iroh endpoint bound in namespace"
    );

    // Create in-memory storage (sufficient for network-layer testing)
    let log_store = InMemoryLogStore::default();
    let state_machine = InMemoryStateMachine::new();
    let sm_variant = StateMachineVariant::InMemory(Arc::clone(&state_machine));

    // Create test transport wrapper
    let transport = Arc::new(TestTransport::new(endpoint.clone(), node_addr.clone(), secret_key));

    // Create Raft network factory with empty initial peers
    let network_factory = IrpcRaftNetworkFactory::new(transport.clone(), HashMap::new(), false);

    // Raft config tuned for test speed
    let config = Arc::new(
        openraft::Config {
            cluster_name: "patchbay-test".to_string(),
            heartbeat_interval: 1000,
            election_timeout_min: 3000,
            election_timeout_max: 5000,
            ..Default::default()
        }
        .validate()
        .context("invalid raft config")?,
    );

    // Create Raft instance
    let raft = Raft::new(node_id, config, network_factory.clone(), log_store, state_machine.clone())
        .await
        .context("failed to create Raft instance")?;

    // Start Raft RPC server for incoming connections
    let raft_server = aspen_raft::server::RaftRpcServer::spawn(Arc::new(endpoint.clone()), raft.clone());

    // RaftNode wrapper for KV operations
    let raft_node = RaftNode::new(node_id, Arc::new(raft.clone()), sm_variant);

    // Command channel
    let (cmd_tx, cmd_rx) = mpsc::channel(CMD_CHANNEL_CAPACITY);

    let handle = NodeHandle { node_id, cmd_tx };

    let cmd_loop = run_command_loop(node_id, raft, raft_node, network_factory, node_addr, cmd_rx, raft_server);

    Ok((handle, cmd_loop))
}

/// Run the command loop for a spawned node.
async fn run_command_loop(
    node_id: NodeId,
    raft: Raft<AppTypeConfig>,
    raft_node: RaftNode,
    network_factory: IrpcRaftNetworkFactory<TestTransport>,
    endpoint_addr: EndpointAddr,
    mut cmd_rx: mpsc::Receiver<NodeCommand>,
    _raft_server: aspen_raft::server::RaftRpcServer,
) {
    use aspen_kv_types::ReadConsistency;
    use aspen_kv_types::ReadRequest;
    use aspen_kv_types::WriteCommand;
    use aspen_kv_types::WriteRequest;

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            NodeCommand::GetAddr { reply } => {
                let _ = reply.send(endpoint_addr.clone());
            }
            NodeCommand::AddPeers { peers, reply } => {
                network_factory.update_peers(peers).await;
                let _ = reply.send(Ok(()));
            }
            NodeCommand::InitCluster { members, reply } => {
                let request = InitRequest {
                    initial_members: members,
                };
                let result =
                    raft_node.init(request).await.map(|_| ()).map_err(|e| anyhow::anyhow!("init failed: {}", e));
                let _ = reply.send(result);
            }
            NodeCommand::WriteKv { key, value, reply } => {
                let request = WriteRequest {
                    command: WriteCommand::Set { key, value },
                };
                let result =
                    raft_node.write(request).await.map(|_| ()).map_err(|e| anyhow::anyhow!("write failed: {}", e));
                let _ = reply.send(result);
            }
            NodeCommand::ReadKv { key, reply } => {
                let request = ReadRequest {
                    key,
                    consistency: ReadConsistency::Stale,
                };
                let result = raft_node
                    .read(request)
                    .await
                    .map(|r| r.kv.map(|kv| kv.value))
                    .map_err(|e| anyhow::anyhow!("read failed: {}", e));
                let _ = reply.send(result);
            }
            NodeCommand::GetLeader { reply } => {
                let metrics = raft.metrics().borrow().clone();
                let _ = reply.send(metrics.current_leader);
            }
            NodeCommand::GetAppliedIndex { reply } => {
                let metrics = raft.metrics().borrow().clone();
                let index = metrics.last_applied.map(|li| li.index);
                let _ = reply.send(index);
            }
            NodeCommand::Shutdown { reply } => {
                info!(node_id = node_id.0, "shutting down patchbay test node");
                let _ = raft.shutdown().await;
                let _ = reply.send(());
                break;
            }
        }
    }
}
