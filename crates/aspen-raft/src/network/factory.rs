//! Network factory for creating per-peer Raft network clients.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_core::NetworkFactory as CoreNetworkFactory;
use aspen_core::NetworkTransport;
use aspen_sharding::ShardId;
use openraft::network::RaftNetworkFactory;
use tokio::sync::RwLock;
use tracing::Instrument;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::FailureDetectorUpdate;
use crate::clock_drift_detection::ClockDriftDetector;
use crate::connection_pool::RaftConnectionPool;
use crate::constants::FAILURE_DETECTOR_CHANNEL_CAPACITY;
use crate::constants::MAX_PEERS;
use crate::network::client::IrpcRaftNetwork;
use crate::network::client::IrpcRaftNetworkConfig;
use crate::node_failure_detection::NodeFailureDetector;
use crate::types::AppTypeConfig;
use crate::types::NodeId;
use crate::types::RaftMemberInfo;
use crate::types::member_endpoint_addr as convert_member_endpoint_addr;

#[inline]
fn max_peers_usize() -> usize {
    usize::try_from(MAX_PEERS).unwrap_or(usize::MAX)
}

/// IRPC-based Raft network factory for Iroh P2P transport.
///
/// With the introduction of `RaftMemberInfo`, peer addresses are now stored directly
/// in the Raft membership state as transport-neutral `NodeAddress` values. The
/// network factory receives those values via `new_client()` and converts them to
/// concrete iroh `EndpointAddr` values only at the runtime shell boundary.
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
    ///
    /// Lock ordering: when multiple factory locks are needed, acquire
    /// `peer_addrs` before `failure_detector`, then `drift_detector`.
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
    /// Path to persist peer addresses across restarts.
    ///
    /// When set, gossip-discovered addresses are saved to this file so
    /// restarted nodes can reach peers at their current addresses without
    /// waiting for gossip rediscovery.
    peer_cache_path: Option<PathBuf>,
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
            peer_cache_path: self.peer_cache_path.clone(),
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
            peer_cache_path: None,
            failure_update_tx,
        }
    }

    /// Enable persistent peer address cache.
    ///
    /// When set, peer addresses are saved to `<data_dir>/peer_addrs.json` on
    /// every gossip update, and loaded on startup. This lets restarted nodes
    /// reach peers at their current addresses immediately.
    pub fn with_peer_cache_dir(mut self, data_dir: &std::path::Path) -> Self {
        let path = data_dir.join("peer_addrs.json");
        // Load existing cache
        if let Ok(contents) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<HashMap<u64, iroh::EndpointAddr>>(&contents) {
                Ok(cached) => {
                    // Use synchronous access since we're in the builder phase
                    // and no other references to peer_addrs exist yet.
                    if let Some(lock) = Arc::get_mut(&mut self.peer_addrs) {
                        let peers = lock.get_mut();
                        for (node_id, addr) in cached {
                            if peers.len() < max_peers_usize() || peers.contains_key(&node_id.into()) {
                                peers.insert(node_id.into(), addr);
                            }
                        }
                        info!(path = %path.display(), "loaded peer address cache");
                    } else {
                        warn!(path = %path.display(), "peer_addrs Arc already shared, cannot load cache");
                    }
                }
                Err(e) => {
                    debug!(path = %path.display(), error = %e, "failed to parse peer address cache, starting fresh");
                }
            }
        }
        self.peer_cache_path = Some(path);
        self
    }

    /// Get a reference to the failure detector for metrics/monitoring.
    pub fn failure_detector(&self) -> Arc<RwLock<NodeFailureDetector>> {
        Arc::clone(&self.failure_detector)
    }

    /// Get a reference to the clock drift detector for metrics/monitoring.
    pub fn drift_detector(&self) -> Arc<RwLock<ClockDriftDetector>> {
        Arc::clone(&self.drift_detector)
    }

    /// Get endpoint IDs from all known peers.
    ///
    /// Used to seed gossip bootstrap so restarted nodes can reconnect.
    pub async fn get_peer_endpoint_ids(&self) -> Vec<iroh::EndpointId> {
        let peers = self.peer_addrs.read().await;
        peers.values().map(|addr| addr.id).collect()
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
        let is_peer_map_full = peers.len() >= max_peers_usize();
        let is_new_key = !peers.contains_key(&node_id);
        if is_peer_map_full && is_new_key {
            warn!(
                current_peers = peers.len(),
                max_peers = MAX_PEERS,
                "peer map full, dropping add_peer request for node {}",
                node_id
            );
            return;
        }

        // Check if address changed — if so, evict the stale connection from the pool.
        // This is critical for post-restart recovery: a restarted node gets a new iroh port,
        // but the connection pool still holds a connection to the old port. Without eviction,
        // heartbeats/RPCs route through the dead connection until it times out (~60s),
        // causing election storms that break all inter-node connectivity.
        let is_address_changed = peers.get(&node_id).map(|old| old.addrs != addr.addrs).unwrap_or(false);

        peers.insert(node_id, addr);
        // Drop the lock before async eviction
        drop(peers);

        if is_address_changed {
            info!(
                %node_id,
                "peer address changed, evicting stale connection from pool"
            );
            self.connection_pool.evict(node_id).await;
        }

        // Persist to disk so restarted nodes have fresh addresses.
        if let Some(ref path) = self.peer_cache_path {
            let peers = self.peer_addrs.read().await;
            let serializable: HashMap<u64, &iroh::EndpointAddr> = peers.iter().map(|(k, v)| (k.0, v)).collect();
            if let Ok(json) = serde_json::to_string(&serializable)
                && let Err(write_err) = std::fs::write(path, json)
            {
                debug!(path = %path.display(), error = %write_err, "failed to persist peer address cache");
            }
        }
    }

    /// Update peer addresses in bulk.
    ///
    /// Extends the existing peer map with new entries. Existing entries are replaced.
    /// Tiger Style: Bounded peer map to prevent Sybil attacks and memory exhaustion.
    pub async fn update_peers(&self, new_peers: HashMap<NodeId, iroh::EndpointAddr>) {
        let mut peers = self.peer_addrs.write().await;
        for (node_id, addr) in new_peers {
            // Decomposed: check capacity first, then check if key is new
            let is_peer_map_full = peers.len() >= max_peers_usize();
            let is_new_key = !peers.contains_key(&node_id);
            if is_peer_map_full && is_new_key {
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
        let peer_addr = match convert_member_endpoint_addr(node) {
            Ok(peer_addr) => Some(peer_addr),
            Err(error) => {
                warn!(
                    target_node = %target,
                    endpoint_id = %node.endpoint_id(),
                    error = %error,
                    "skipping sharded peer cache seed because membership address is not a valid iroh endpoint"
                );
                None
            }
        };

        if let Some(peer_addr) = &peer_addr {
            let mut peers = self.peer_addrs.write().await;
            if peers.len() < max_peers_usize() || peers.contains_key(&target) {
                peers.insert(target, peer_addr.clone());
            }
        }

        info!(
            target_node = %target,
            shard_id,
            endpoint_id = %node.endpoint_id(),
            "creating sharded network client"
        );

        IrpcRaftNetwork::new(Arc::clone(&self.connection_pool), IrpcRaftNetworkConfig {
            peer_addr,
            target,
            failure_detector: Arc::clone(&self.failure_detector),
            drift_detector: Arc::clone(&self.drift_detector),
            shard_id: Some(shard_id),
            failure_update_tx: self.failure_update_tx.clone(),
        })
        .with_gossip_addrs(Arc::clone(&self.peer_addrs))
    }
}

impl<T> RaftNetworkFactory<AppTypeConfig> for IrpcRaftNetworkFactory<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    type Network = IrpcRaftNetwork<T>;

    async fn new_client(&mut self, target: NodeId, node: &RaftMemberInfo) -> Self::Network {
        let target_node = target;
        async move {
            let membership_addr = match convert_member_endpoint_addr(node) {
                Ok(addr) => Some(addr),
                Err(error) => {
                    warn!(
                        target_node = %target,
                        endpoint_id = %node.endpoint_id(),
                        error = %error,
                        "stored membership address is not a valid iroh endpoint"
                    );
                    None
                }
            };

            // Resolve the best address: gossip-discovered addresses may be fresher
            // than Raft membership after a node restart (new port, same endpoint ID).
            let peer_addr = {
                let peers = self.peer_addrs.read().await;
                if let Some(gossip_addr) = peers.get(&target) {
                    if gossip_addr.id.to_string() == node.endpoint_id() {
                        match &membership_addr {
                            Some(raft_addr) if gossip_addr != raft_addr => {
                                info!(
                                    target_node = %target,
                                    endpoint_id = %gossip_addr.id,
                                    gossip_addrs = gossip_addr.addrs.len(),
                                    raft_addrs = node.transport_addr_count(),
                                    "using gossip-discovered address (fresher than Raft membership)"
                                );
                                Some(gossip_addr.clone())
                            }
                            Some(raft_addr) => Some(raft_addr.clone()),
                            None => Some(gossip_addr.clone()),
                        }
                    } else {
                        // Endpoint ID mismatch — gossip cache has a different node's
                        // address under this node ID. Fall back to Raft membership.
                        warn!(
                            target_node = %target,
                            raft_endpoint_id = %node.endpoint_id(),
                            gossip_endpoint_id = %gossip_addr.id,
                            "gossip cache endpoint ID mismatch, using Raft membership address"
                        );
                        membership_addr.clone()
                    }
                } else {
                    // No gossip entry — use Raft membership address.
                    membership_addr.clone()
                }
            };

            // Update the fallback cache with Raft membership address (only if no
            // gossip entry exists — don't overwrite fresher gossip data).
            if let Some(membership_addr) = &membership_addr {
                let mut peers = self.peer_addrs.write().await;
                if !peers.contains_key(&target) && (peers.len() < max_peers_usize()) {
                    peers.insert(target, membership_addr.clone());
                }
            }

            debug!(
                target_node = %target,
                endpoint_id = %node.endpoint_id(),
                "creating network client with address from Raft membership"
            );

            IrpcRaftNetwork::new(Arc::clone(&self.connection_pool), IrpcRaftNetworkConfig {
                peer_addr,
                target,
                failure_detector: Arc::clone(&self.failure_detector),
                drift_detector: Arc::clone(&self.drift_detector),
                shard_id: None, // Non-sharded mode for backward compatibility
                failure_update_tx: self.failure_update_tx.clone(),
            })
            .with_gossip_addrs(Arc::clone(&self.peer_addrs))
        }
        .instrument(tracing::debug_span!("new_client", target = %target_node))
        .await
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
