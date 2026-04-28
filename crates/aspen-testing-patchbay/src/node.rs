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
use aspen_traits::KvRead;
use aspen_traits::KvWrite;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::SecretKey;
use openraft::Raft;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::debug;
use tracing::info;

use crate::transport::TestTransport;

/// Maximum pending commands in the node's command channel.
const CMD_CHANNEL_CAPACITY: usize = 64;

fn trust_disabled_config() -> aspen_cluster_types::TrustConfig {
    aspen_cluster_types::TrustConfig {
        enabled: false,
        threshold: None,
    }
}

struct CommandLoopContext {
    node_id: NodeId,
    raft: Raft<AppTypeConfig>,
    raft_node: RaftNode,
    network_factory: IrpcRaftNetworkFactory<TestTransport>,
    endpoint_addr: EndpointAddr,
    _raft_server: aspen_raft::server::RaftRpcServer,
}

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
        if let Err(error) = self.cmd_tx.send(NodeCommand::Shutdown { reply: tx }).await {
            debug!(%error, node_id = self.node_id.0, "patchbay node already stopped before shutdown signal");
            return Ok(());
        }
        if let Err(error) = rx.await {
            debug!(%error, node_id = self.node_id.0, "patchbay node dropped shutdown reply channel");
        }
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
    let endpoint = Endpoint::builder(iroh::endpoint::presets::N0)
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
    let log_store = InMemoryLogStore::new();
    let state_machine = InMemoryStateMachine::new();
    let sm_variant = StateMachineVariant::InMemory(Arc::clone(&state_machine));

    // Create test transport wrapper
    let raft_network_backend = Arc::new(TestTransport::new(endpoint.clone(), node_addr.clone(), secret_key));

    // Create Raft network factory with empty initial peers
    let network_factory = IrpcRaftNetworkFactory::new(raft_network_backend.clone(), HashMap::new(), false);

    // Raft config tuned for test speed
    let config = Arc::new(
        openraft::Config::build(&[
            "patchbay-test",
            "--cluster-name",
            "patchbay-test",
            "--heartbeat-interval",
            "1000",
            "--election-timeout-min",
            "3000",
            "--election-timeout-max",
            "5000",
        ])
        .context("invalid raft config")?,
    );

    // Create Raft instance
    let raft = Raft::new(
        node_id,
        config,
        network_factory.clone(),
        log_store,
        InMemoryStateMachine::store(state_machine.clone()),
    )
    .await
    .context("failed to create Raft instance")?;

    // Start Raft RPC server for incoming connections
    let raft_server = aspen_raft::server::RaftRpcServer::spawn(Arc::new(endpoint.clone()), raft.clone());
    debug_assert!(!node_addr.addrs.is_empty());
    debug_assert_eq!(handle_node_id_value(node_id), node_id.0);

    // RaftNode wrapper for KV operations
    let raft_node = RaftNode::new(node_id, Arc::new(raft.clone()), sm_variant);

    // Command channel
    let (cmd_tx, cmd_rx) = mpsc::channel(CMD_CHANNEL_CAPACITY);

    let handle = NodeHandle { node_id, cmd_tx };
    debug_assert_eq!(handle.node_id.0, node_id.0);

    let cmd_context = CommandLoopContext {
        node_id,
        raft,
        raft_node,
        network_factory,
        endpoint_addr: node_addr,
        _raft_server: raft_server,
    };
    let cmd_loop = run_command_loop(cmd_context, cmd_rx);

    Ok((handle, cmd_loop))
}

fn handle_node_id_value(node_id: NodeId) -> u64 {
    node_id.0
}

fn send_reply_or_log<T>(reply: oneshot::Sender<T>, value: T, node_id: NodeId, action: &str) {
    if reply.send(value).is_err() {
        debug!(node_id = node_id.0, action, "patchbay reply receiver dropped");
    }
}

async fn init_cluster_command(raft_node: &RaftNode, members: Vec<ClusterNode>) -> Result<()> {
    let request = InitRequest {
        initial_members: members,
        trust: trust_disabled_config(),
    };
    raft_node.init(request).await.map(|_| ()).map_err(|e| anyhow::anyhow!("init failed: {}", e))
}

async fn write_kv_command(raft_node: &RaftNode, key: String, value: String) -> Result<()> {
    let request = aspen_kv_types::WriteRequest::set(key, value);
    raft_node.write(request).await.map(|_| ()).map_err(|e| anyhow::anyhow!("write failed: {}", e))
}

async fn read_kv_command(raft_node: &RaftNode, key: String) -> Result<Option<String>> {
    let request = aspen_kv_types::ReadRequest::stale(key);
    raft_node
        .read(request)
        .await
        .map(|response| response.kv.map(|kv| kv.value))
        .map_err(|e| anyhow::anyhow!("read failed: {}", e))
}

/// Run the command loop for a spawned node.
async fn run_command_loop(cmd_context: CommandLoopContext, mut cmd_rx: mpsc::Receiver<NodeCommand>) {
    debug_assert_eq!(cmd_context.node_id.0, handle_node_id_value(cmd_context.node_id));
    debug_assert!(!cmd_context.endpoint_addr.addrs.is_empty());

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            NodeCommand::GetAddr { reply } => {
                send_reply_or_log(reply, cmd_context.endpoint_addr.clone(), cmd_context.node_id, "get-addr");
            }
            NodeCommand::AddPeers { peers, reply } => {
                cmd_context.network_factory.update_peers(peers).await;
                send_reply_or_log(reply, Ok(()), cmd_context.node_id, "add-peers");
            }
            NodeCommand::InitCluster { members, reply } => {
                let result = init_cluster_command(&cmd_context.raft_node, members).await;
                send_reply_or_log(reply, result, cmd_context.node_id, "init");
            }
            NodeCommand::WriteKv { key, value, reply } => {
                let result = write_kv_command(&cmd_context.raft_node, key, value).await;
                send_reply_or_log(reply, result, cmd_context.node_id, "write");
            }
            NodeCommand::ReadKv { key, reply } => {
                let result = read_kv_command(&cmd_context.raft_node, key).await;
                send_reply_or_log(reply, result, cmd_context.node_id, "read");
            }
            NodeCommand::GetLeader { reply } => {
                let metrics = cmd_context.raft.metrics().borrow().clone();
                send_reply_or_log(reply, metrics.current_leader, cmd_context.node_id, "leader");
            }
            NodeCommand::GetAppliedIndex { reply } => {
                let metrics = cmd_context.raft.metrics().borrow().clone();
                let index = metrics.last_applied.map(|li| li.index);
                send_reply_or_log(reply, index, cmd_context.node_id, "applied-index");
            }
            NodeCommand::Shutdown { reply } => {
                info!(node_id = cmd_context.node_id.0, "shutting down patchbay test node");
                if let Err(error) = cmd_context.raft.shutdown().await {
                    debug!(%error, node_id = cmd_context.node_id.0, "patchbay raft shutdown returned error");
                }
                send_reply_or_log(reply, (), cmd_context.node_id, "shutdown");
                break;
            }
        }
    }
}
