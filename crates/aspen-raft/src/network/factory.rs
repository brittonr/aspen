//! Network factory for creating per-peer Raft network clients.

use std::collections::HashMap;
use std::sync::Arc;

use aspen_core::NetworkFactory as CoreNetworkFactory;
use aspen_core::NetworkTransport;
use aspen_sharding::ShardId;
use openraft::network::RaftNetworkFactory;
use tokio::sync::RwLock;
use tracing::info;
use tracing::warn;

use super::FailureDetectorUpdate;
use crate::clock_drift_detection::ClockDriftDetector;
use crate::connection_pool::RaftConnectionPool;
use crate::constants::FAILURE_DETECTOR_CHANNEL_CAPACITY;
use crate::constants::MAX_PEERS;
use crate::network::client::IrpcRaftNetwork;
use crate::node_failure_detection::NodeFailureDetector;
use crate::types::AppTypeConfig;
use crate::types::NodeId;
use crate::types::RaftMemberInfo;

/// IRPC-based Raft network factory for Iroh P2P transport.
///
/// With the introduction of `RaftMemberInfo`, peer addresses are now stored directly
/// in the Raft membership state. The network factory receives addresses via
/// the `new_client()` method's `node` parameter, which contains the `RaftMemberInfo`
/// with the Iroh `EndpointAddr`.
///
/// The `peer_addrs` map is retained as a fallback/cache for:
/// - Initial bootstrap before Raft membership is populated
/// - Manual peer additions via CLI/config
/// - Testing scenarios
///
/// # Type Parameters
///
/// * `T` - Transport implementation that provides Iroh endpoint access. Must implement
///   `NetworkTransport` with Iroh-specific associated types.
///
/// Tiger Style: Fixed peer map, explicit endpoint management.
pub struct IrpcRaftNetworkFactory<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr>
{
    /// Transport providing endpoint access for creating connections.
    #[allow(dead_code)]
    transport: Arc<T>,
    /// Connection pool for reusable QUIC connections.
    connection_pool: Arc<RaftConnectionPool<T>>,
    /// Fallback map of NodeId to Iroh EndpointAddr.
    ///
    /// Used as a fallback when addresses aren't available from Raft membership.
    /// Initially populated from CLI args/config manual peers.
    ///
    /// With `RaftMemberInfo`, the primary source of peer addresses is now the
    /// Raft membership state, passed to `new_client()` via the `node` parameter.
    ///
    /// Uses `Arc<RwLock<T>>` to allow concurrent peer addition during runtime.
    peer_addrs: Arc<RwLock<HashMap<NodeId, iroh::EndpointAddr>>>,
    /// Failure detector for distinguishing actor crashes from node crashes.
    ///
    /// Updated automatically based on Raft RPC success/failure and Iroh connection status.
    failure_detector: Arc<RwLock<NodeFailureDetector>>,
    /// Clock drift detector for monitoring peer clock synchronization.
    ///
    /// Purely observational - does NOT affect Raft consensus.
    /// Used to detect NTP misconfiguration for operational health.
    drift_detector: Arc<RwLock<ClockDriftDetector>>,
    /// Bounded channel for failure detector updates.
    ///
    /// Tiger Style: Prevents unbounded task spawning by batching updates
    /// through a single consumer task instead of spawning per-failure tasks.
    failure_update_tx: tokio::sync::mpsc::Sender<FailureDetectorUpdate>,
}

// Manual Clone implementation that doesn't require T: Clone.
// All fields are Arc<...> which are always Clone regardless of T.
impl<T> Clone for IrpcRaftNetworkFactory<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr>
{
    fn clone(&self) -> Self {
        Self {
            transport: Arc::clone(&self.transport),
            connection_pool: Arc::clone(&self.connection_pool),
            peer_addrs: Arc::clone(&self.peer_addrs),
            failure_detector: Arc::clone(&self.failure_detector),
            drift_detector: Arc::clone(&self.drift_detector),
            failure_update_tx: self.failure_update_tx.clone(),
        }
    }
}

impl<T> IrpcRaftNetworkFactory<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    /// Create a new Raft network factory.
    ///
    /// # Arguments
    ///
    /// * `transport` - Network transport providing endpoint access for P2P connections
    /// * `peer_addrs` - Initial peer addresses for fallback lookup
    /// * `use_auth_alpn` - When true, use `RAFT_AUTH_ALPN` for connections (requires auth server).
    ///   When false, use legacy `RAFT_ALPN` for backward compatibility.
    ///
    /// # Iroh-Native Authentication
    ///
    /// Authentication is handled at connection accept time by the server using
    /// Iroh's native NodeId verification. The client side (this factory) does
    /// not need to perform any authentication - it simply connects and the
    /// server validates the NodeId against the TrustedPeersRegistry.
    ///
    /// When `use_auth_alpn` is true, outgoing connections use `RAFT_AUTH_ALPN` which
    /// signals to the server that the client expects authenticated operation.
    pub fn new(transport: Arc<T>, peer_addrs: HashMap<NodeId, iroh::EndpointAddr>, use_auth_alpn: bool) -> Self {
        let failure_detector = Arc::new(RwLock::new(NodeFailureDetector::default_timeout()));
        let drift_detector = Arc::new(RwLock::new(ClockDriftDetector::new()));
        let connection_pool =
            Arc::new(RaftConnectionPool::new(Arc::clone(&transport), Arc::clone(&failure_detector), use_auth_alpn));

        // Start the background cleanup task for idle connections.
        // Tiger Style: This task runs for the lifetime of the process and terminates
        // when the connection pool is dropped. The JoinHandle is not tracked because
        // the task is intentionally long-lived and shutdown is handled via process exit.
        let pool_clone = Arc::clone(&connection_pool);
        tokio::spawn(async move {
            pool_clone.start_cleanup_task().await;
        });

        // Tiger Style: Bounded channel prevents unbounded task spawning
        // Instead of spawning a new task for each failure update, we use a
        // bounded channel with a single consumer task.
        let (failure_update_tx, mut failure_update_rx) =
            tokio::sync::mpsc::channel::<FailureDetectorUpdate>(FAILURE_DETECTOR_CHANNEL_CAPACITY);

        // Spawn single consumer task for failure detector updates.
        // Tiger Style: This task terminates automatically when failure_update_tx is dropped
        // (channel closes, recv() returns None). No explicit shutdown needed because the
        // task lifetime is bounded by the channel sender's lifetime.
        let failure_detector_clone = Arc::clone(&failure_detector);
        tokio::spawn(async move {
            while let Some(update) = failure_update_rx.recv().await {
                failure_detector_clone.write().await.update_node_status(
                    update.node_id,
                    update.raft_status,
                    update.iroh_status,
                );
            }
        });

        Self {
            transport,
            connection_pool,
            peer_addrs: Arc::new(RwLock::new(peer_addrs)),
            failure_detector,
            drift_detector,
            failure_update_tx,
        }
    }

    /// Get a reference to the failure detector for metrics/monitoring.
    pub fn failure_detector(&self) -> Arc<RwLock<NodeFailureDetector>> {
        Arc::clone(&self.failure_detector)
    }

    /// Get a reference to the clock drift detector for metrics/monitoring.
    pub fn drift_detector(&self) -> Arc<RwLock<ClockDriftDetector>> {
        Arc::clone(&self.drift_detector)
    }

    /// Add a peer address for future connections.
    ///
    /// This allows dynamic peer addition after the network factory has been created.
    /// Useful for integration tests where nodes exchange addresses at runtime.
    ///
    /// Tiger Style: Bounded peer map to prevent Sybil attacks and memory exhaustion.
    pub async fn add_peer(&self, node_id: NodeId, addr: iroh::EndpointAddr) {
        let mut peers = self.peer_addrs.write().await;
        // Decomposed: check capacity first, then check if key is new
        let is_at_capacity = peers.len() >= MAX_PEERS as usize;
        let is_new_key = !peers.contains_key(&node_id);
        if is_at_capacity && is_new_key {
            warn!(
                current_peers = peers.len(),
                max_peers = MAX_PEERS,
                "peer map full, dropping add_peer request for node {}",
                node_id
            );
            return;
        }
        peers.insert(node_id, addr);
    }

    /// Update peer addresses in bulk.
    ///
    /// Extends the existing peer map with new entries. Existing entries are replaced.
    /// Tiger Style: Bounded peer map to prevent Sybil attacks and memory exhaustion.
    pub async fn update_peers(&self, new_peers: HashMap<NodeId, iroh::EndpointAddr>) {
        let mut peers = self.peer_addrs.write().await;
        for (node_id, addr) in new_peers {
            // Decomposed: check capacity first, then check if key is new
            let is_at_capacity = peers.len() >= MAX_PEERS as usize;
            let is_new_key = !peers.contains_key(&node_id);
            if is_at_capacity && is_new_key {
                warn!(
                    current_peers = peers.len(),
                    max_peers = MAX_PEERS,
                    "peer map full, dropping peer {} from bulk update",
                    node_id
                );
                continue;
            }
            peers.insert(node_id, addr);
        }
    }

    /// Get a clone of the current peer addresses map.
    ///
    /// Useful for debugging or inspection.
    pub async fn peer_addrs(&self) -> HashMap<NodeId, iroh::EndpointAddr> {
        self.peer_addrs.read().await.clone()
    }

    /// Create a network client for a specific shard.
    ///
    /// This creates an `IrpcRaftNetwork` that will prepend the shard ID to all
    /// RPC messages, enabling routing to the correct Raft core on the remote node.
    ///
    /// # Arguments
    ///
    /// * `target` - Target node ID
    /// * `node` - Target node's Raft member info containing Iroh address
    /// * `shard_id` - Shard ID to route RPCs to
    ///
    /// # Example
    ///
    /// ```ignore
    /// let client = factory.new_client_for_shard(target_id, &member_info, 5).await;
    /// // All RPCs from this client will be routed to shard 5 on the target node
    /// ```
    pub async fn new_client_for_shard(
        &mut self,
        target: NodeId,
        node: &RaftMemberInfo,
        shard_id: ShardId,
    ) -> IrpcRaftNetwork<T> {
        // Update the fallback cache with this address
        {
            let mut peers = self.peer_addrs.write().await;
            if peers.len() < MAX_PEERS as usize || peers.contains_key(&target) {
                peers.insert(target, node.iroh_addr.clone());
            }
        }

        info!(
            target_node = %target,
            shard_id,
            endpoint_id = %node.iroh_addr.id,
            "creating sharded network client"
        );

        IrpcRaftNetwork::new(
            Arc::clone(&self.connection_pool),
            Some(node.iroh_addr.clone()),
            target,
            Arc::clone(&self.failure_detector),
            Arc::clone(&self.drift_detector),
            Some(shard_id),
            self.failure_update_tx.clone(),
        )
    }
}

impl<T> RaftNetworkFactory<AppTypeConfig> for IrpcRaftNetworkFactory<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    type Network = IrpcRaftNetwork<T>;

    #[tracing::instrument(level = "debug", skip_all, fields(target = %target))]
    async fn new_client(&mut self, target: NodeId, node: &RaftMemberInfo) -> Self::Network {
        // Primary source: Get address from RaftMemberInfo (stored in Raft membership)
        let peer_addr = Some(node.iroh_addr.clone());

        // Update the fallback cache with this address for future reference
        {
            let mut peers = self.peer_addrs.write().await;
            if peers.len() < MAX_PEERS as usize || peers.contains_key(&target) {
                peers.insert(target, node.iroh_addr.clone());
            }
        }

        info!(
            target_node = %target,
            endpoint_id = %node.iroh_addr.id,
            "creating network client with address from Raft membership"
        );

        IrpcRaftNetwork::new(
            Arc::clone(&self.connection_pool),
            peer_addr,
            target,
            Arc::clone(&self.failure_detector),
            Arc::clone(&self.drift_detector),
            None, // Non-sharded mode for backward compatibility
            self.failure_update_tx.clone(),
        )
    }
}

/// Implementation of aspen-core's NetworkFactory trait for RPC handler integration.
///
/// This allows the IrpcRaftNetworkFactory to be used directly in ClientProtocolContext
/// for dynamic peer registration via the AddPeer RPC.
#[async_trait::async_trait]
impl<T> CoreNetworkFactory for IrpcRaftNetworkFactory<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    async fn add_peer(&self, node_id: u64, address: String) -> Result<(), String> {
        // Parse the JSON-serialized EndpointAddr
        let addr: iroh::EndpointAddr =
            serde_json::from_str(&address).map_err(|e| format!("invalid EndpointAddr JSON: {e}"))?;

        // Delegate to the internal add_peer method (NodeId is a newtype around u64)
        IrpcRaftNetworkFactory::add_peer(self, node_id.into(), addr).await;
        Ok(())
    }

    async fn remove_peer(&self, node_id: u64) -> Result<(), String> {
        let mut peers = self.peer_addrs.write().await;
        peers.remove(&node_id.into());
        Ok(())
    }
}
