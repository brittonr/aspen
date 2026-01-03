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
//! TODO: Add unit tests for IrpcRaftNetworkFactory:
//!       - Factory creation with peer address registration
//!       - Dynamic peer addition via add_peer()
//!       - Network client creation via new_client()
//!       Coverage: 0% line coverage (tested via madsim simulation tests)
//!
//! TODO: Add unit tests for IrpcRaftNetwork RPC methods:
//!       - vote() request/response serialization
//!       - append_entries() with various payload sizes
//!       - full_snapshot() streaming with MAX_SNAPSHOT_SIZE limit
//!       - Error propagation (Unreachable, NetworkError, Timeout)
//!       Coverage: Tested via integration tests only
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
//! let factory = IrpcRaftNetworkFactory::new(endpoint_manager, peer_addrs);
//! // Factory is passed to openraft::Raft::new() for network creation
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use anyhow::Context;
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
use crate::rpc::RaftAppendEntriesRequest;
use crate::rpc::RaftRpcProtocol;
use crate::rpc::RaftRpcResponse;
use crate::rpc::RaftRpcResponseWithTimestamps;
use crate::rpc::RaftSnapshotRequest;
use crate::rpc::RaftVoteRequest;
use crate::rpc::SHARD_PREFIX_SIZE;
use crate::rpc::encode_shard_prefix;
use crate::rpc::try_decode_shard_prefix;
use crate::types::AppTypeConfig;
use crate::types::NodeId;
use crate::types::RaftMemberInfo;
use aspen_core::NetworkTransport;
use aspen_sharding::ShardId;
#[cfg(feature = "sharding")]
use aspen_sharding::router::ShardId;

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
/// * `T` - Transport implementation that provides Iroh endpoint access.
///   Must implement `NetworkTransport` with Iroh-specific associated types.
///
/// Tiger Style: Fixed peer map, explicit endpoint management.
pub struct IrpcRaftNetworkFactory<T>
where
    T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr>,
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
where
    T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr>,
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
where
    T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static,
{
    /// Create a new Raft network factory.
    ///
    /// # Arguments
    ///
    /// * `transport` - Network transport providing endpoint access for P2P connections
    /// * `peer_addrs` - Initial peer addresses for fallback lookup
    ///
    /// # Iroh-Native Authentication
    ///
    /// Authentication is handled at connection accept time by the server using
    /// Iroh's native NodeId verification. The client side (this factory) does
    /// not need to perform any authentication - it simply connects and the
    /// server validates the NodeId against the TrustedPeersRegistry.
    pub fn new(transport: Arc<T>, peer_addrs: HashMap<NodeId, iroh::EndpointAddr>) -> Self {
        let failure_detector = Arc::new(RwLock::new(NodeFailureDetector::default_timeout()));
        let drift_detector = Arc::new(RwLock::new(ClockDriftDetector::new()));
        let connection_pool = Arc::new(RaftConnectionPool::new(Arc::clone(&transport), Arc::clone(&failure_detector)));

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
where
    T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static,
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
/// * `T` - Transport implementation that provides Iroh endpoint access.
///   Must implement `NetworkTransport` with Iroh-specific associated types.
///
/// # Sharded Mode
///
/// When `shard_id` is `Some`, all RPC messages are prefixed with a 4-byte
/// big-endian shard ID. This enables routing to the correct Raft core on
/// the remote node when using the sharded ALPN (`raft-shard`).
pub struct IrpcRaftNetwork<T>
where
    T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr>,
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
where
    T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static,
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

        // If sharded mode, prepend shard ID prefix
        let message = if let Some(shard_id) = self.shard_id {
            let mut prefixed = Vec::with_capacity(SHARD_PREFIX_SIZE + serialized.len());
            prefixed.extend_from_slice(&encode_shard_prefix(shard_id));
            prefixed.extend_from_slice(&serialized);
            prefixed
        } else {
            serialized
        };

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

        // If sharded mode, strip shard ID prefix from response and verify
        let response_bytes = if let Some(expected_shard_id) = self.shard_id {
            let response_shard_id = try_decode_shard_prefix(&response_buf).ok_or_else(|| {
                anyhow::anyhow!(
                    "sharded response too short: expected at least {} bytes, got {}",
                    SHARD_PREFIX_SIZE,
                    response_buf.len()
                )
            })?;

            if response_shard_id != expected_shard_id {
                return Err(anyhow::anyhow!(
                    "shard ID mismatch: expected {}, got {}",
                    expected_shard_id,
                    response_shard_id
                ));
            }

            &response_buf[SHARD_PREFIX_SIZE..]
        } else {
            &response_buf[..]
        };

        info!(
            response_size = response_bytes.len(),
            shard_id = ?self.shard_id,
            "received RPC response bytes"
        );

        // Try to deserialize as response with timestamps first (new format)
        // Fall back to legacy format for backward compatibility
        let response: RaftRpcResponse =
            if let Ok(response_with_ts) = postcard::from_bytes::<RaftRpcResponseWithTimestamps>(response_bytes) {
                // Update clock drift detector if timestamps are present
                if let Some(timestamps) = response_with_ts.timestamps {
                    self.drift_detector.write().await.record_observation(
                        self.target,
                        client_send_ms,
                        timestamps.server_recv_ms,
                        timestamps.server_send_ms,
                        client_recv_ms,
                    );
                }
                response_with_ts.inner
            } else {
                // Fall back to legacy format (no timestamps)
                postcard::from_bytes::<RaftRpcResponse>(response_bytes)
                    .map_err(|e| {
                        error!(
                            target_node = %self.target,
                            error = %e,
                            bytes_len = response_bytes.len(),
                            first_bytes = ?response_bytes.get(..20.min(response_bytes.len())),
                            "failed to deserialize RPC response"
                        );
                        e
                    })
                    .context("failed to deserialize RPC response")?
            };

        // Check if we received a fatal error response - this means the peer's RaftCore is down
        if let RaftRpcResponse::FatalError(error_kind) = &response {
            warn!(
                target_node = %self.target,
                error_kind = %error_kind,
                "peer reported fatal RaftCore error"
            );
            // Mark Raft as disconnected but Iroh as connected (we got a response)
            self.failure_detector.write().await.update_node_status(
                self.target,
                ConnectionStatus::Disconnected,
                ConnectionStatus::Connected,
            );
        } else {
            // Update failure detector: RPC succeeded with both connection and Raft working
            self.failure_detector.write().await.update_node_status(
                self.target,
                ConnectionStatus::Connected,
                ConnectionStatus::Connected,
            );
        }

        Ok(response)
    }

    /// Update failure detector when RPC fails.
    ///
    /// Called by RPC methods when send_rpc returns an error.
    /// The connection pool already updates failure detector for connection/stream errors,
    /// so this is mainly for RPC-level failures (timeouts, deserialization, etc.)
    async fn update_failure_on_rpc_error(&self, err: &anyhow::Error) {
        // Determine connection status from error message
        // Connection pool errors suggest connection is down
        // Stream errors suggest connection is up but Raft is down
        // Other errors (serialization, timeout) suggest both might be up but RPC failed
        let (raft_status, iroh_status) =
            if err.to_string().contains("connection pool") || err.to_string().contains("peer address not found") {
                (ConnectionStatus::Disconnected, ConnectionStatus::Disconnected)
            } else if err.to_string().contains("stream") {
                (ConnectionStatus::Disconnected, ConnectionStatus::Connected)
            } else {
                // Timeout or other RPC-level error - assume connection is ok but Raft is having issues
                (ConnectionStatus::Disconnected, ConnectionStatus::Connected)
            };

        self.failure_detector.write().await.update_node_status(self.target, raft_status, iroh_status);
    }
}

#[allow(clippy::blocks_in_conditions)]
impl<T> RaftNetworkV2<AppTypeConfig> for IrpcRaftNetwork<T>
where
    T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static,
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
