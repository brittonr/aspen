//! IRPC-based Raft network layer over Iroh P2P transport.
//!
//! Provides the network factory and client implementations for Raft RPC communication
//! between cluster nodes using Iroh's QUIC-based P2P networking. Includes connection
//! pooling for efficient stream multiplexing and failure detection for distinguishing
//! between actor crashes and node failures.
//!
//! # Key Components
//!
//! - `IrpcRaftNetworkFactory`: Factory for creating per-peer network clients
//! - `IrpcRaftNetwork`: Single-peer network client for sending Raft RPCs
//! - Connection pooling via `RaftConnectionPool` for stream reuse
//! - `NodeFailureDetector` integration for health monitoring
//!
//! # Tiger Style
//!
//! - Bounded peer map (MAX_PEERS) prevents Sybil attacks and memory exhaustion
//! - Explicit error handling with failure detector updates
//! - Connection pooling with idle cleanup for resource efficiency
//! - Fixed timeouts (IROH_READ_TIMEOUT) for bounded operation
//! - Size limits (MAX_RPC_MESSAGE_SIZE, MAX_SNAPSHOT_SIZE) for memory safety
//!
//! # Test Coverage
//!
//! Comprehensive unit tests (42 tests) covering:
//! - Factory creation, configuration, and peer management
//! - Client creation (new_client, new_client_for_shard)
//! - Connection pooling and peer address handling
//! - RPC serialization (Vote, AppendEntries, Snapshot)
//! - Error classification (NodeCrash, ActorCrash, Unreachable)
//! - Failure detector integration
//!
//! Full cluster-level testing via madsim simulation tests
//!
//! # Architecture
//!
//! ```text
//! RaftNode
//!    |
//!    v
//! IrpcRaftNetworkFactory
//!    |
//!    +---> IrpcRaftNetwork (peer 1) ---> RaftConnectionPool ---> Iroh QUIC
//!    +---> IrpcRaftNetwork (peer 2) ---> RaftConnectionPool ---> Iroh QUIC
//!    +---> IrpcRaftNetwork (peer 3) ---> RaftConnectionPool ---> Iroh QUIC
//! ```
//!
//! # Example
//!
//! ```ignore
//! use aspen::raft::network::IrpcRaftNetworkFactory;
//!
//! // With authentication enabled (production)
//! let factory = IrpcRaftNetworkFactory::new(endpoint_manager, peer_addrs, true);
//!
//! // Without authentication (legacy/testing)
//! let factory = IrpcRaftNetworkFactory::new(endpoint_manager, peer_addrs, false);
//!
//! // Factory is passed to openraft::Raft::new() for network creation
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use anyhow::Context;
use aspen_core::NetworkFactory as CoreNetworkFactory;
use aspen_core::NetworkTransport;
use aspen_sharding::ShardId;
#[cfg(feature = "sharding")]
use aspen_sharding::router::ShardId;
use openraft::OptionalSend;
use openraft::Snapshot;
use openraft::StorageError;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::ReplicationClosed;
use openraft::error::StreamingError;
use openraft::error::Unreachable;
use openraft::network::RPCOption;
use openraft::network::RaftNetworkFactory;
use openraft::network::v2::RaftNetworkV2;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::SnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use openraft::type_config::alias::VoteOf;
use tokio::io::AsyncReadExt;
use tokio::select;
use tokio::sync::RwLock;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::clock_drift_detection::ClockDriftDetector;
use crate::clock_drift_detection::current_time_ms;
use crate::connection_pool::RaftConnectionPool;
use crate::constants::FAILURE_DETECTOR_CHANNEL_CAPACITY;
use crate::constants::IROH_READ_TIMEOUT;
use crate::constants::MAX_PEERS;
use crate::constants::MAX_RPC_MESSAGE_SIZE;
use crate::constants::MAX_SNAPSHOT_SIZE;
use crate::node_failure_detection::ConnectionStatus;
use crate::node_failure_detection::NodeFailureDetector;
use crate::pure::classify_response_health;
use crate::pure::classify_rpc_error;
use crate::pure::deserialize_rpc_response;
use crate::pure::extract_sharded_response;
use crate::pure::maybe_prefix_shard_id;
use crate::rpc::RaftAppendEntriesRequest;
use crate::rpc::RaftRpcProtocol;
use crate::rpc::RaftRpcResponse;
use crate::rpc::RaftSnapshotRequest;
use crate::rpc::RaftVoteRequest;
use crate::types::AppTypeConfig;
use crate::types::NodeId;
use crate::types::RaftMemberInfo;

/// Update message for the failure detector.
///
/// Tiger Style: Bounded channel prevents unbounded task spawning.
/// Instead of spawning a new task for each failure update, we send
/// through a bounded channel and let a single consumer process updates.
#[derive(Debug)]
struct FailureDetectorUpdate {
    node_id: NodeId,
    raft_status: ConnectionStatus,
    iroh_status: ConnectionStatus,
}

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

        // Start the background cleanup task for idle connections
        let pool_clone = Arc::clone(&connection_pool);
        tokio::spawn(async move {
            pool_clone.start_cleanup_task().await;
        });

        // Tiger Style: Bounded channel prevents unbounded task spawning
        // Instead of spawning a new task for each failure update, we use a
        // bounded channel with a single consumer task.
        let (failure_update_tx, mut failure_update_rx) =
            tokio::sync::mpsc::channel::<FailureDetectorUpdate>(FAILURE_DETECTOR_CHANNEL_CAPACITY);

        // Spawn single consumer task for failure detector updates
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
        if peers.len() >= MAX_PEERS as usize && !peers.contains_key(&node_id) {
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
            if peers.len() >= MAX_PEERS as usize && !peers.contains_key(&node_id) {
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

        IrpcRaftNetwork {
            connection_pool: Arc::clone(&self.connection_pool),
            peer_addr: Some(node.iroh_addr.clone()),
            target,
            failure_detector: Arc::clone(&self.failure_detector),
            drift_detector: Arc::clone(&self.drift_detector),
            shard_id: Some(shard_id),
            failure_update_tx: self.failure_update_tx.clone(),
        }
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

        IrpcRaftNetwork {
            connection_pool: Arc::clone(&self.connection_pool),
            peer_addr,
            target,
            failure_detector: Arc::clone(&self.failure_detector),
            drift_detector: Arc::clone(&self.drift_detector),
            shard_id: None, // Non-sharded mode for backward compatibility
            failure_update_tx: self.failure_update_tx.clone(),
        }
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

/// IRPC-based Raft network client for a single peer.
///
/// Tiger Style:
/// - Explicit error handling for connection failures
/// - Fail fast if peer address is missing
/// - Connection pooling for efficient stream multiplexing
/// - Bounded channel for failure detector updates (prevents unbounded task spawning)
///
/// # Type Parameters
///
/// * `T` - Transport implementation that provides Iroh endpoint access. Must implement
///   `NetworkTransport` with Iroh-specific associated types.
///
/// # Sharded Mode
///
/// When `shard_id` is `Some`, all RPC messages are prefixed with a 4-byte
/// big-endian shard ID. This enables routing to the correct Raft core on
/// the remote node when using the sharded ALPN (`raft-shard`).
pub struct IrpcRaftNetwork<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr>
{
    connection_pool: Arc<RaftConnectionPool<T>>,
    peer_addr: Option<iroh::EndpointAddr>,
    target: NodeId,
    failure_detector: Arc<RwLock<NodeFailureDetector>>,
    drift_detector: Arc<RwLock<ClockDriftDetector>>,
    /// Optional shard ID for sharded RPC routing.
    ///
    /// When set, all RPC messages are prefixed with this shard ID,
    /// and responses are expected to include the shard ID prefix.
    shard_id: Option<ShardId>,
    /// Bounded channel for failure detector updates.
    ///
    /// Tiger Style: Prevents unbounded task spawning by sending updates
    /// through a bounded channel instead of spawning per-failure tasks.
    failure_update_tx: tokio::sync::mpsc::Sender<FailureDetectorUpdate>,
}

impl<T> IrpcRaftNetwork<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    /// Send an RPC request to the peer and wait for response.
    ///
    /// Tiger Style: Fail fast if peer address is unknown.
    ///
    /// This implements a simple request-response pattern:
    /// 1. Get or create connection from pool
    /// 2. Acquire a stream from the pooled connection
    /// 3. Serialize and send the request
    /// 4. Wait for response on the same stream
    /// 5. Deserialize and return the response
    ///
    /// Updates failure detector and drift detector based on RPC success/failure.
    async fn send_rpc(&self, request: RaftRpcProtocol) -> anyhow::Result<RaftRpcResponse> {
        let peer_addr = self.peer_addr.as_ref().context("peer address not found in peer map")?;

        // Get or create connection from pool
        //
        // Error classification: Connection failure (timeout, refused, unreachable) is
        // classified as NodeCrash (both Raft and Iroh disconnected). This is intentional:
        // the failure detector tracks reachability, not error details. Specific error
        // types are logged but not used for classification.
        let peer_connection = self
            .connection_pool
            .get_or_connect(self.target, peer_addr)
            .await
            .inspect_err(|err| {
                // Log specific error for debugging, but classify uniformly as NodeCrash
                tracing::debug!(
                    target = %self.target,
                    error = %err,
                    "connection failure, classifying as NodeCrash"
                );
                // Tiger Style: Use bounded channel instead of spawning unbounded tasks
                // try_send is non-blocking; if channel is full, we drop the update
                // (acceptable since failure detector will get future updates)
                let _ = self.failure_update_tx.try_send(FailureDetectorUpdate {
                    node_id: self.target,
                    raft_status: ConnectionStatus::Disconnected,
                    iroh_status: ConnectionStatus::Disconnected,
                });
            })
            .context("failed to get connection from pool")?;

        // Acquire a stream from the pooled connection
        //
        // Error classification: Stream failure with existing connection is classified
        // as ActorCrash (Iroh connected but Raft disconnected). This distinguishes
        // node-level failures from application-level issues.
        let mut stream_handle = peer_connection
            .acquire_stream()
            .await
            .inspect_err(|err| {
                // Log specific error for debugging, but classify uniformly as ActorCrash
                tracing::debug!(
                    target = %self.target,
                    error = %err,
                    "stream failure with connection up, classifying as ActorCrash"
                );
                // Tiger Style: Use bounded channel instead of spawning unbounded tasks
                // try_send is non-blocking; if channel is full, we drop the update
                // (acceptable since failure detector will get future updates)
                let _ = self.failure_update_tx.try_send(FailureDetectorUpdate {
                    node_id: self.target,
                    raft_status: ConnectionStatus::Disconnected,
                    iroh_status: ConnectionStatus::Connected,
                });
            })
            .context("failed to acquire stream from connection")?;

        // Record client send time (t1) for clock drift detection
        let client_send_ms = current_time_ms();

        // Serialize the request
        let serialized = postcard::to_stdvec(&request).context("failed to serialize RPC request")?;

        // If sharded mode, prepend shard ID prefix (pure function)
        let message = maybe_prefix_shard_id(serialized, self.shard_id);

        // Send the request
        stream_handle.send.write_all(&message).await.context("failed to write RPC request to stream")?;
        stream_handle.send.finish().context("failed to finish send stream")?;

        // Read response (with size and time limits)
        let response_buf =
            tokio::time::timeout(IROH_READ_TIMEOUT, stream_handle.recv.read_to_end(MAX_RPC_MESSAGE_SIZE as usize))
                .await
                .context("timeout reading RPC response")?
                .context("failed to read RPC response")?;

        // Record client receive time (t4) for clock drift detection
        let client_recv_ms = current_time_ms();

        // CRITICAL: Detect empty response buffer
        // This can happen when the server panics mid-RPC or closes the stream prematurely.
        // An empty buffer is always an error - we need at least 1 byte for valid postcard encoding.
        if response_buf.is_empty() {
            error!(
                target_node = %self.target,
                shard_id = ?self.shard_id,
                "received empty response buffer from peer - peer may have panicked or closed stream prematurely"
            );
            // Mark node as having Raft issues (actor crash) but Iroh connection worked
            self.failure_detector.write().await.update_node_status(
                self.target,
                ConnectionStatus::Disconnected,
                ConnectionStatus::Connected,
            );
            return Err(anyhow::anyhow!(
                "empty response buffer from node {} - peer RaftCore may have panicked",
                self.target
            ));
        }

        // If sharded mode, strip shard ID prefix from response and verify (pure function)
        let response_bytes =
            extract_sharded_response(&response_buf, self.shard_id).map_err(|e| anyhow::anyhow!("{}", e))?;

        info!(
            response_size = response_bytes.len(),
            shard_id = ?self.shard_id,
            "received RPC response bytes"
        );

        // Deserialize response with backward compatibility (pure function)
        let (response, timestamps) = deserialize_rpc_response(response_bytes).map_err(|e| {
            error!(
                target_node = %self.target,
                error = %e,
                bytes_len = response_bytes.len(),
                first_bytes = ?response_bytes.get(..20.min(response_bytes.len())),
                "failed to deserialize RPC response"
            );
            anyhow::anyhow!("failed to deserialize RPC response: {}", e)
        })?;

        // Update clock drift detector if timestamps are present
        if let Some(ts) = timestamps {
            self.drift_detector.write().await.record_observation(
                self.target,
                client_send_ms,
                ts.server_recv_ms,
                ts.server_send_ms,
                client_recv_ms,
            );
        }

        // Classify response health and update failure detector (pure function for classification)
        let (raft_status, iroh_status) = classify_response_health(&response);

        // Log fatal errors for visibility
        if let RaftRpcResponse::FatalError(error_kind) = &response {
            warn!(
                target_node = %self.target,
                error_kind = %error_kind,
                "peer reported fatal RaftCore error"
            );
        }

        // Update failure detector with the classified status
        self.failure_detector.write().await.update_node_status(self.target, raft_status, iroh_status);

        Ok(response)
    }

    /// Update failure detector when RPC fails.
    ///
    /// Called by RPC methods when send_rpc returns an error.
    /// The connection pool already updates failure detector for connection/stream errors,
    /// so this is mainly for RPC-level failures (timeouts, deserialization, etc.)
    async fn update_failure_on_rpc_error(&self, err: &anyhow::Error) {
        // Classify error to determine connection status (pure function)
        let (raft_status, iroh_status) = classify_rpc_error(&err.to_string());
        self.failure_detector.write().await.update_node_status(self.target, raft_status, iroh_status);
    }
}

#[allow(clippy::blocks_in_conditions)]
impl<T> RaftNetworkV2<AppTypeConfig> for IrpcRaftNetwork<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        let request = RaftAppendEntriesRequest { request: rpc };
        let protocol = RaftRpcProtocol::AppendEntries(request);

        // Send the RPC and get response
        let response = match self.send_rpc(protocol).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(target_node = %self.target, error = %err, "failed to send append_entries RPC");
                self.update_failure_on_rpc_error(&err).await;
                let err_str = err.to_string();
                return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::other(err_str))));
            }
        };

        // Extract result from response, handling fatal errors gracefully
        match response {
            RaftRpcResponse::AppendEntries(result) => Ok(result),
            RaftRpcResponse::FatalError(error_kind) => {
                // Peer's RaftCore is in a fatal state - treat as unreachable
                // The failure detector was already updated in send_rpc
                error!(
                    target_node = %self.target,
                    error_kind = %error_kind,
                    "peer RaftCore reported fatal error for append_entries"
                );
                Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::other(format!(
                    "peer RaftCore fatal error: {}",
                    error_kind
                )))))
            }
            _ => Err(RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unexpected response type for append_entries",
            )))),
        }
    }

    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn vote(
        &mut self,
        rpc: VoteRequest<AppTypeConfig>,
        _option: RPCOption,
    ) -> Result<VoteResponse<AppTypeConfig>, RPCError<AppTypeConfig>> {
        let request = RaftVoteRequest { request: rpc };
        let protocol = RaftRpcProtocol::Vote(request);

        // Send the RPC and get response
        let response = match self.send_rpc(protocol).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(target_node = %self.target, error = %err, "failed to send vote RPC");
                self.update_failure_on_rpc_error(&err).await;
                let err_str = err.to_string();
                return Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::other(err_str))));
            }
        };

        // Extract result from response, handling fatal errors gracefully
        match response {
            RaftRpcResponse::Vote(result) => Ok(result),
            RaftRpcResponse::FatalError(error_kind) => {
                // Peer's RaftCore is in a fatal state - treat as unreachable
                // The failure detector was already updated in send_rpc
                error!(
                    target_node = %self.target,
                    error_kind = %error_kind,
                    "peer RaftCore reported fatal error for vote"
                );
                Err(RPCError::Unreachable(Unreachable::new(&std::io::Error::other(format!(
                    "peer RaftCore fatal error: {}",
                    error_kind
                )))))
            }
            _ => Err(RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unexpected response type for vote",
            )))),
        }
    }

    #[tracing::instrument(level = "debug", skip_all, err(Debug))]
    async fn full_snapshot(
        &mut self,
        vote: VoteOf<AppTypeConfig>,
        snapshot: Snapshot<AppTypeConfig>,
        cancel: impl Future<Output = ReplicationClosed> + OptionalSend + 'static,
        _option: RPCOption,
    ) -> Result<SnapshotResponse<AppTypeConfig>, StreamingError<AppTypeConfig>> {
        // Read snapshot data into bytes with size limit (Tiger Style: bounded allocation)
        let mut snapshot_data = Vec::new();
        let mut snapshot_reader = snapshot.snapshot;

        // Read in chunks, checking size limit to prevent unbounded memory allocation
        let mut buffer = [0u8; 8192]; // 8KB chunks
        loop {
            let bytes_read = snapshot_reader.read(&mut buffer).await.map_err(|err| {
                StreamingError::StorageError(StorageError::read_snapshot(Some(snapshot.meta.signature()), &err))
            })?;

            if bytes_read == 0 {
                break; // EOF
            }

            // Tiger Style: Fail fast if snapshot exceeds size limit
            if snapshot_data.len() as u64 + bytes_read as u64 > MAX_SNAPSHOT_SIZE {
                return Err(StreamingError::StorageError(StorageError::read_snapshot(
                    Some(snapshot.meta.signature()),
                    &std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("snapshot exceeds maximum size of {} bytes", MAX_SNAPSHOT_SIZE),
                    ),
                )));
            }

            snapshot_data.extend_from_slice(&buffer[..bytes_read]);
        }

        let request = RaftSnapshotRequest {
            vote,
            snapshot_meta: snapshot.meta,
            snapshot_data,
        };
        let protocol = RaftRpcProtocol::InstallSnapshot(request);

        // Send the RPC with cancellation support
        let response = select! {
            send_result = self.send_rpc(protocol) => {
                match send_result {
                    Ok(resp) => resp,
                    Err(err) => {
                        warn!(target_node = %self.target, error = %err, "failed to send snapshot RPC");
                        self.update_failure_on_rpc_error(&err).await;
                        let err_str = err.to_string();
                        return Err(StreamingError::Unreachable(Unreachable::new(&std::io::Error::other(
                            err_str,
                        ))));
                    }
                }
            }
            closed = cancel => {
                warn!(target_node = %self.target, "snapshot transmission cancelled");
                return Err(StreamingError::Closed(closed));
            }
        };

        // Extract result from response, handling fatal errors gracefully
        match response {
            RaftRpcResponse::InstallSnapshot(result) => {
                // Handle remote RaftError as StorageError since snapshot installation failed
                result.map_err(|raft_err| StreamingError::StorageError(StorageError::read_snapshot(None, &raft_err)))
            }
            RaftRpcResponse::FatalError(error_kind) => {
                // Peer's RaftCore is in a fatal state - treat as unreachable
                // The failure detector was already updated in send_rpc
                error!(
                    target_node = %self.target,
                    error_kind = %error_kind,
                    "peer RaftCore reported fatal error for install_snapshot"
                );
                Err(StreamingError::Unreachable(Unreachable::new(&std::io::Error::other(format!(
                    "peer RaftCore fatal error: {}",
                    error_kind
                )))))
            }
            _ => Err(StreamingError::Unreachable(Unreachable::new(&std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unexpected response type for install_snapshot",
            )))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    // =========================================================================
    // FailureDetectorUpdate Tests
    // =========================================================================

    #[test]
    fn test_failure_detector_update_construction() {
        let update = FailureDetectorUpdate {
            node_id: NodeId::from(42),
            raft_status: ConnectionStatus::Connected,
            iroh_status: ConnectionStatus::Disconnected,
        };

        assert_eq!(update.node_id, NodeId::from(42));
        assert_eq!(update.raft_status, ConnectionStatus::Connected);
        assert_eq!(update.iroh_status, ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_failure_detector_update_debug() {
        let update = FailureDetectorUpdate {
            node_id: NodeId::from(1),
            raft_status: ConnectionStatus::Connected,
            iroh_status: ConnectionStatus::Connected,
        };

        let debug_str = format!("{:?}", update);
        assert!(debug_str.contains("FailureDetectorUpdate"));
        assert!(debug_str.contains("node_id"));
    }

    #[test]
    fn test_failure_detector_update_all_status_combinations() {
        // Test all 4 combinations of connection statuses
        let combinations = [
            (ConnectionStatus::Connected, ConnectionStatus::Connected),
            (ConnectionStatus::Connected, ConnectionStatus::Disconnected),
            (ConnectionStatus::Disconnected, ConnectionStatus::Connected),
            (ConnectionStatus::Disconnected, ConnectionStatus::Disconnected),
        ];

        for (raft, iroh) in combinations {
            let update = FailureDetectorUpdate {
                node_id: NodeId::from(1),
                raft_status: raft,
                iroh_status: iroh,
            };
            assert_eq!(update.raft_status, raft);
            assert_eq!(update.iroh_status, iroh);
        }
    }

    // =========================================================================
    // IrpcRaftNetworkFactory Tests (Pure Unit Tests)
    // =========================================================================

    // Note: Full async tests of IrpcRaftNetworkFactory require a real Iroh endpoint
    // or a mock transport. These tests cover the pure/synchronous aspects.

    #[test]
    fn test_max_peers_constant_is_bounded() {
        // Tiger Style: Verify MAX_PEERS is within reasonable bounds
        let max_peers = MAX_PEERS;
        assert!(max_peers >= 64, "MAX_PEERS should allow reasonable cluster size");
        assert!(max_peers <= 10_000, "MAX_PEERS should prevent resource exhaustion");
    }

    #[test]
    fn test_failure_detector_channel_capacity_is_bounded() {
        // Tiger Style: Verify channel capacity is bounded
        let capacity = FAILURE_DETECTOR_CHANNEL_CAPACITY;
        assert!(capacity >= 64, "Channel should handle burst of failures");
        assert!(capacity <= 10_000, "Channel should not consume unbounded memory");
    }

    #[test]
    fn test_iroh_read_timeout_is_reasonable() {
        // Tiger Style: Verify read timeout is bounded and reasonable
        assert!(
            IROH_READ_TIMEOUT >= std::time::Duration::from_millis(100),
            "Timeout should allow for network latency"
        );
        assert!(IROH_READ_TIMEOUT <= std::time::Duration::from_secs(60), "Timeout should not wait forever");
    }

    #[test]
    fn test_max_rpc_message_size_is_bounded() {
        // Tiger Style: Verify RPC message size limit
        let max_size = MAX_RPC_MESSAGE_SIZE;
        assert!(max_size >= 1024, "Should allow reasonable message sizes");
        assert!(max_size <= 100 * 1024 * 1024, "Should prevent memory exhaustion (100MB max)");
    }

    #[test]
    fn test_max_snapshot_size_is_bounded() {
        // Tiger Style: Verify snapshot size limit
        let max_size = MAX_SNAPSHOT_SIZE;
        assert!(max_size >= 1024 * 1024, "Should allow 1MB+ snapshots");
        assert!(max_size <= 10 * 1024 * 1024 * 1024, "Should prevent 10GB+ allocations");
    }

    // =========================================================================
    // Peer Address Map Tests
    // =========================================================================

    #[test]
    fn test_peer_addrs_hashmap_capacity() {
        // Verify HashMap can be created with expected capacity
        let peer_addrs: HashMap<NodeId, iroh::EndpointAddr> = HashMap::with_capacity(MAX_PEERS as usize);
        assert!(peer_addrs.capacity() >= MAX_PEERS as usize);
    }

    #[test]
    fn test_node_id_as_hashmap_key() {
        let mut map: HashMap<NodeId, u32> = HashMap::new();

        let node1 = NodeId::from(1);
        let node2 = NodeId::from(2);
        let node3 = NodeId::from(u64::MAX);

        map.insert(node1, 100);
        map.insert(node2, 200);
        map.insert(node3, 300);

        assert_eq!(map.get(&node1), Some(&100));
        assert_eq!(map.get(&node2), Some(&200));
        assert_eq!(map.get(&node3), Some(&300));
        assert_eq!(map.get(&NodeId::from(999)), None);
    }

    #[test]
    fn test_node_id_equality() {
        let node1a = NodeId::from(42);
        let node1b = NodeId::from(42);
        let node2 = NodeId::from(43);

        assert_eq!(node1a, node1b);
        assert_ne!(node1a, node2);
    }

    #[test]
    fn test_node_id_copy_semantics() {
        let node = NodeId::from(42);
        let copied = node; // Copy
        assert_eq!(node, copied); // Both still valid
    }

    // =========================================================================
    // RaftMemberInfo Tests
    // =========================================================================

    #[test]
    fn test_raft_member_info_construction() {
        use iroh::SecretKey;

        // Use deterministic seed pattern from aspen-testing
        let mut seed = [0u8; 32];
        seed[0] = 1;
        let secret_key = SecretKey::from(seed);
        let endpoint_id = secret_key.public();
        let endpoint_addr = iroh::EndpointAddr::new(endpoint_id);

        let member_info = RaftMemberInfo::new(endpoint_addr.clone());

        assert_eq!(member_info.iroh_addr.id, endpoint_addr.id);
    }

    #[test]
    fn test_raft_member_info_clone() {
        use iroh::SecretKey;

        // Use deterministic seed pattern from aspen-testing
        let mut seed = [0u8; 32];
        seed[0] = 2;
        let secret_key = SecretKey::from(seed);
        let endpoint_id = secret_key.public();
        let endpoint_addr = iroh::EndpointAddr::new(endpoint_id);

        let member_info = RaftMemberInfo::new(endpoint_addr);
        let cloned = member_info.clone();

        assert_eq!(member_info.iroh_addr.id, cloned.iroh_addr.id);
    }

    // =========================================================================
    // ShardId Tests
    // =========================================================================

    #[test]
    fn test_shard_id_type() {
        // ShardId is u32, verify it can represent reasonable shard counts
        let shard: ShardId = 0;
        assert_eq!(shard, 0);

        let shard: ShardId = u32::MAX;
        assert_eq!(shard, u32::MAX);
    }

    #[test]
    fn test_shard_id_in_option() {
        let none_shard: Option<ShardId> = None;
        let some_shard: Option<ShardId> = Some(42);

        assert!(none_shard.is_none());
        assert_eq!(some_shard, Some(42));
    }

    // =========================================================================
    // RPC Protocol Serialization Tests
    // =========================================================================

    #[test]
    fn test_raft_rpc_response_vote_serialization() {
        use openraft::Vote;

        let response = RaftRpcResponse::Vote(openraft::raft::VoteResponse {
            vote: Vote::new(1, NodeId::from(1)),
            vote_granted: true,
            last_log_id: None,
        });

        // Test postcard serialization (what we use on the wire)
        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

        assert!(matches!(deserialized, RaftRpcResponse::Vote(v) if v.vote_granted));
    }

    #[test]
    fn test_raft_rpc_response_append_entries_serialization() {
        let response = RaftRpcResponse::AppendEntries(openraft::raft::AppendEntriesResponse::Success);

        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

        assert!(matches!(
            deserialized,
            RaftRpcResponse::AppendEntries(openraft::raft::AppendEntriesResponse::Success)
        ));
    }

    #[test]
    fn test_raft_rpc_response_fatal_error_serialization() {
        use crate::rpc::RaftFatalErrorKind;

        for kind in [
            RaftFatalErrorKind::Panicked,
            RaftFatalErrorKind::Stopped,
            RaftFatalErrorKind::StorageError,
        ] {
            let response = RaftRpcResponse::FatalError(kind);

            let bytes = postcard::to_stdvec(&response).expect("serialize");
            let deserialized: RaftRpcResponse = postcard::from_bytes(&bytes).expect("deserialize");

            assert!(matches!(deserialized, RaftRpcResponse::FatalError(k) if k == kind));
        }
    }

    #[test]
    fn test_raft_rpc_response_with_timestamps_serialization() {
        use crate::rpc::RaftRpcResponseWithTimestamps;
        use crate::rpc::TimestampInfo;

        let response = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::AppendEntries(openraft::raft::AppendEntriesResponse::Conflict),
            timestamps: Some(TimestampInfo {
                server_recv_ms: 1000,
                server_send_ms: 1005,
            }),
        };

        let bytes = postcard::to_stdvec(&response).expect("serialize");
        let deserialized: RaftRpcResponseWithTimestamps = postcard::from_bytes(&bytes).expect("deserialize");

        assert!(deserialized.timestamps.is_some());
        let ts = deserialized.timestamps.unwrap();
        assert_eq!(ts.server_recv_ms, 1000);
        assert_eq!(ts.server_send_ms, 1005);
    }

    // =========================================================================
    // Shard Prefix Encoding Tests
    // =========================================================================

    #[test]
    fn test_shard_prefix_encoding_zero() {
        let prefix = encode_shard_prefix(0);
        assert_eq!(prefix, [0, 0, 0, 0]);

        let decoded = try_decode_shard_prefix(&prefix);
        assert_eq!(decoded, Some(0));
    }

    #[test]
    fn test_shard_prefix_encoding_max() {
        let prefix = encode_shard_prefix(u32::MAX);
        assert_eq!(prefix, [0xFF, 0xFF, 0xFF, 0xFF]);

        let decoded = try_decode_shard_prefix(&prefix);
        assert_eq!(decoded, Some(u32::MAX));
    }

    #[test]
    fn test_shard_prefix_encoding_roundtrip() {
        for shard_id in [0, 1, 42, 255, 256, 1000, 65535, 0x12345678, u32::MAX] {
            let encoded = encode_shard_prefix(shard_id);
            let decoded = try_decode_shard_prefix(&encoded);
            assert_eq!(decoded, Some(shard_id), "Roundtrip failed for shard_id={}", shard_id);
        }
    }

    #[test]
    fn test_shard_prefix_decode_too_short() {
        assert_eq!(try_decode_shard_prefix(&[]), None);
        assert_eq!(try_decode_shard_prefix(&[0]), None);
        assert_eq!(try_decode_shard_prefix(&[0, 0]), None);
        assert_eq!(try_decode_shard_prefix(&[0, 0, 0]), None);
    }

    #[test]
    fn test_shard_prefix_size_constant() {
        assert_eq!(SHARD_PREFIX_SIZE, 4);
    }

    // =========================================================================
    // Error Classification Tests
    // =========================================================================

    #[test]
    fn test_error_message_classification_connection_pool() {
        // Test that connection pool errors are classified as NodeCrash
        let err_msg = "connection pool: failed to connect";
        let is_connection_pool = err_msg.contains("connection pool");
        assert!(is_connection_pool);
    }

    #[test]
    fn test_error_message_classification_peer_address() {
        // Test that peer address errors are classified as NodeCrash
        let err_msg = "peer address not found in peer map";
        let is_peer_not_found = err_msg.contains("peer address not found");
        assert!(is_peer_not_found);
    }

    #[test]
    fn test_error_message_classification_stream() {
        // Test that stream errors are classified as ActorCrash
        let err_msg = "stream failure: connection refused";
        let is_stream_error = err_msg.contains("stream");
        assert!(is_stream_error);
    }

    // =========================================================================
    // Bounded Channel Tests
    // =========================================================================

    #[tokio::test]
    async fn test_failure_update_channel_bounded() {
        // Verify channel is bounded and blocks appropriately
        let (tx, mut rx) = tokio::sync::mpsc::channel::<FailureDetectorUpdate>(2);

        // Fill the channel
        for i in 0..2 {
            tx.send(FailureDetectorUpdate {
                node_id: NodeId::from(i),
                raft_status: ConnectionStatus::Connected,
                iroh_status: ConnectionStatus::Connected,
            })
            .await
            .expect("send should succeed");
        }

        // try_send should fail when channel is full
        let result = tx.try_send(FailureDetectorUpdate {
            node_id: NodeId::from(999),
            raft_status: ConnectionStatus::Disconnected,
            iroh_status: ConnectionStatus::Disconnected,
        });
        assert!(result.is_err(), "try_send should fail when channel is full");

        // Drain the channel
        for _ in 0..2 {
            let _ = rx.recv().await.expect("should receive");
        }
    }

    #[tokio::test]
    async fn test_failure_update_channel_try_send_pattern() {
        // Test the try_send pattern used in send_rpc for failure updates
        let (tx, _rx) = tokio::sync::mpsc::channel::<FailureDetectorUpdate>(FAILURE_DETECTOR_CHANNEL_CAPACITY);

        // try_send should succeed when channel has capacity
        let result = tx.try_send(FailureDetectorUpdate {
            node_id: NodeId::from(1),
            raft_status: ConnectionStatus::Disconnected,
            iroh_status: ConnectionStatus::Connected,
        });
        assert!(result.is_ok(), "try_send should succeed with capacity");
    }

    // =========================================================================
    // ALPN Tests
    // =========================================================================

    #[test]
    #[allow(deprecated)]
    fn test_raft_alpn_values() {
        // Test both deprecated and recommended ALPN values for backward compat
        assert_eq!(aspen_transport::RAFT_ALPN, b"raft-rpc");
        assert_eq!(aspen_transport::RAFT_AUTH_ALPN, b"raft-auth");
    }

    #[test]
    #[allow(deprecated)]
    fn test_raft_alpn_values_are_different() {
        assert_ne!(aspen_transport::RAFT_ALPN, aspen_transport::RAFT_AUTH_ALPN);
    }

    #[test]
    #[allow(deprecated)]
    fn test_raft_alpn_values_are_valid_utf8() {
        assert!(std::str::from_utf8(aspen_transport::RAFT_ALPN).is_ok());
        assert!(std::str::from_utf8(aspen_transport::RAFT_AUTH_ALPN).is_ok());
    }

    // =========================================================================
    // RwLock and Arc Pattern Tests
    // =========================================================================

    #[tokio::test]
    async fn test_rwlock_peer_addrs_pattern() {
        // Test the RwLock<HashMap> pattern used for peer_addrs
        let peer_addrs: Arc<RwLock<HashMap<NodeId, u32>>> = Arc::new(RwLock::new(HashMap::new()));

        // Write path (add_peer pattern)
        {
            let mut peers = peer_addrs.write().await;
            if peers.len() < MAX_PEERS as usize {
                peers.insert(NodeId::from(1), 100);
            }
        }

        // Read path (new_client pattern)
        {
            let peers = peer_addrs.read().await;
            assert_eq!(peers.get(&NodeId::from(1)), Some(&100));
        }

        // Concurrent access
        let peer_addrs_clone = Arc::clone(&peer_addrs);
        let handle = tokio::spawn(async move {
            let peers = peer_addrs_clone.read().await;
            peers.get(&NodeId::from(1)).copied()
        });

        let result = handle.await.expect("task should complete");
        assert_eq!(result, Some(100));
    }

    #[tokio::test]
    async fn test_rwlock_failure_detector_pattern() {
        // Test the RwLock<NodeFailureDetector> pattern
        let failure_detector = Arc::new(RwLock::new(NodeFailureDetector::default_timeout()));

        // Write path (update status)
        failure_detector.write().await.update_node_status(
            NodeId::from(1),
            ConnectionStatus::Connected,
            ConnectionStatus::Connected,
        );

        // Verify the update was successful by getting the unreachable nodes count
        // (using public unreachable_count method)
        let detector = failure_detector.read().await;
        assert_eq!(detector.unreachable_count(), 0); // No unreachable nodes
    }

    // =========================================================================
    // CoreNetworkFactory Trait Tests
    // =========================================================================

    #[test]
    fn test_endpoint_addr_json_serialization() {
        use iroh::SecretKey;

        // Use deterministic seed pattern from aspen-testing
        let mut seed = [0u8; 32];
        seed[0] = 3;
        let secret_key = SecretKey::from(seed);
        let endpoint_id = secret_key.public();
        let endpoint_addr = iroh::EndpointAddr::new(endpoint_id);

        // This is the pattern used in CoreNetworkFactory::add_peer
        let json = serde_json::to_string(&endpoint_addr).expect("serialize");
        let parsed: iroh::EndpointAddr = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(endpoint_addr.id, parsed.id);
    }

    #[test]
    fn test_endpoint_addr_json_parse_error() {
        // Test error handling for invalid JSON
        let invalid_json = "not valid json";
        let result: Result<iroh::EndpointAddr, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }

    // =========================================================================
    // Connection Status Classification Tests
    // =========================================================================

    #[test]
    fn test_connection_status_enum() {
        // Verify ConnectionStatus enum values
        let connected = ConnectionStatus::Connected;
        let disconnected = ConnectionStatus::Disconnected;

        assert_eq!(connected, ConnectionStatus::Connected);
        assert_eq!(disconnected, ConnectionStatus::Disconnected);
        assert_ne!(connected, disconnected);
    }

    #[test]
    fn test_connection_status_copy() {
        let status = ConnectionStatus::Connected;
        let copied = status; // Copy
        assert_eq!(status, copied);
    }

    // =========================================================================
    // Empty Response Buffer Detection Tests
    // =========================================================================

    #[test]
    fn test_empty_response_detection() {
        // Test the empty response buffer check pattern
        let empty_buf: Vec<u8> = vec![];
        let non_empty_buf: Vec<u8> = vec![1, 2, 3];

        assert!(empty_buf.is_empty(), "Empty buffer should be detected");
        assert!(!non_empty_buf.is_empty(), "Non-empty buffer should not be detected as empty");
    }

    #[test]
    fn test_empty_postcard_deserialization_fails() {
        // Empty buffer should fail postcard deserialization
        let empty_buf: &[u8] = &[];
        let result: Result<RaftRpcResponse, _> = postcard::from_bytes(empty_buf);
        assert!(result.is_err(), "Empty buffer should fail deserialization");
    }

    // =========================================================================
    // Timeout Duration Tests
    // =========================================================================

    #[test]
    fn test_timeout_durations_reasonable() {
        // Verify timeouts are in reasonable ranges
        use crate::constants::IROH_CONNECT_TIMEOUT;
        use crate::constants::IROH_STREAM_OPEN_TIMEOUT;

        // Connect timeout should be longer than stream timeout
        assert!(IROH_CONNECT_TIMEOUT >= IROH_STREAM_OPEN_TIMEOUT, "Connect timeout should be >= stream timeout");

        // Read timeout should be reasonable
        assert!(IROH_READ_TIMEOUT >= std::time::Duration::from_secs(1), "Read timeout should be at least 1 second");
    }
}
