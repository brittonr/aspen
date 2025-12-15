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

use anyhow::Context;
use openraft::error::{NetworkError, RPCError, ReplicationClosed, StreamingError, Unreachable};
use openraft::network::{RPCOption, RaftNetworkFactory, v2::RaftNetworkV2};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, SnapshotResponse, VoteRequest, VoteResponse,
};
use openraft::type_config::alias::VoteOf;
use openraft::{OptionalSend, Snapshot, StorageError};
use tokio::io::AsyncReadExt;
use tokio::select;
use tracing::{error, info, warn};

use crate::cluster::IrohEndpointManager;
use crate::raft::clock_drift_detection::{ClockDriftDetector, current_time_ms};
use crate::raft::connection_pool::RaftConnectionPool;
use crate::raft::constants::{
    IROH_READ_TIMEOUT, MAX_PEERS, MAX_RPC_MESSAGE_SIZE, MAX_SNAPSHOT_SIZE,
};
use crate::raft::node_failure_detection::{ConnectionStatus, NodeFailureDetector};
use crate::raft::rpc::{
    RaftAppendEntriesRequest, RaftRpcProtocol, RaftRpcResponse, RaftRpcResponseWithTimestamps,
    RaftSnapshotRequest, RaftVoteRequest,
};
use crate::raft::types::{AppTypeConfig, NodeId, RaftMemberInfo};
use std::sync::Arc;
use tokio::sync::RwLock;

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
/// Tiger Style: Fixed peer map, explicit endpoint management.
#[derive(Clone)]
pub struct IrpcRaftNetworkFactory {
    /// Connection pool for reusable QUIC connections.
    connection_pool: Arc<RaftConnectionPool>,
    /// Fallback map of NodeId to Iroh EndpointAddr.
    ///
    /// Used as a fallback when addresses aren't available from Raft membership.
    /// Initially populated from CLI args/config manual peers.
    ///
    /// With `RaftMemberInfo`, the primary source of peer addresses is now the
    /// Raft membership state, passed to `new_client()` via the `node` parameter.
    ///
    /// Uses Arc<RwLock> to allow concurrent peer addition during runtime.
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
}

impl IrpcRaftNetworkFactory {
    pub fn new(
        endpoint_manager: Arc<IrohEndpointManager>,
        peer_addrs: HashMap<NodeId, iroh::EndpointAddr>,
    ) -> Self {
        let failure_detector = Arc::new(RwLock::new(NodeFailureDetector::default_timeout()));
        let drift_detector = Arc::new(RwLock::new(ClockDriftDetector::new()));
        let connection_pool = Arc::new(RaftConnectionPool::new(
            Arc::clone(&endpoint_manager),
            Arc::clone(&failure_detector),
        ));

        // Start the background cleanup task for idle connections
        let pool_clone = Arc::clone(&connection_pool);
        tokio::spawn(async move {
            pool_clone.start_cleanup_task().await;
        });

        Self {
            connection_pool,
            peer_addrs: Arc::new(RwLock::new(peer_addrs)),
            failure_detector,
            drift_detector,
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
}

impl RaftNetworkFactory<AppTypeConfig> for IrpcRaftNetworkFactory {
    type Network = IrpcRaftNetwork;

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
        }
    }
}

/// IRPC-based Raft network client for a single peer.
///
/// Tiger Style:
/// - Explicit error handling for connection failures
/// - Fail fast if peer address is missing
/// - Connection pooling for efficient stream multiplexing
pub struct IrpcRaftNetwork {
    connection_pool: Arc<RaftConnectionPool>,
    peer_addr: Option<iroh::EndpointAddr>,
    target: NodeId,
    failure_detector: Arc<RwLock<NodeFailureDetector>>,
    drift_detector: Arc<RwLock<ClockDriftDetector>>,
}

impl IrpcRaftNetwork {
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
        let peer_addr = self
            .peer_addr
            .as_ref()
            .context("peer address not found in peer map")?;

        // Get or create connection from pool
        let peer_connection = self
            .connection_pool
            .get_or_connect(self.target, peer_addr)
            .await
            .inspect_err(|_err| {
                // Update failure detector on connection failure
                let failure_detector_clone = Arc::clone(&self.failure_detector);
                let target = self.target;
                tokio::spawn(async move {
                    failure_detector_clone.write().await.update_node_status(
                        target,
                        ConnectionStatus::Disconnected,
                        ConnectionStatus::Disconnected,
                    );
                });
            })
            .context("failed to get connection from pool")?;

        // Acquire a stream from the pooled connection
        let (mut send_stream, mut recv_stream) = peer_connection
            .acquire_stream()
            .await
            .inspect_err(|_err| {
                // Update failure detector on stream failure (connection exists but stream failed)
                let failure_detector_clone = Arc::clone(&self.failure_detector);
                let target = self.target;
                tokio::spawn(async move {
                    failure_detector_clone.write().await.update_node_status(
                        target,
                        ConnectionStatus::Disconnected,
                        ConnectionStatus::Connected, // Connection exists but Raft actor might be down
                    );
                });
            })
            .context("failed to acquire stream from connection")?;

        // Record client send time (t1) for clock drift detection
        let client_send_ms = current_time_ms();

        // Serialize and send the request
        let serialized =
            postcard::to_stdvec(&request).context("failed to serialize RPC request")?;
        send_stream
            .write_all(&serialized)
            .await
            .context("failed to write RPC request to stream")?;
        send_stream
            .finish()
            .context("failed to finish send stream")?;

        // Read response (with size and time limits)
        let response_buf = tokio::time::timeout(
            IROH_READ_TIMEOUT,
            recv_stream.read_to_end(MAX_RPC_MESSAGE_SIZE as usize),
        )
        .await
        .context("timeout reading RPC response")?
        .context("failed to read RPC response")?;

        // Record client receive time (t4) for clock drift detection
        let client_recv_ms = current_time_ms();

        info!(
            response_size = response_buf.len(),
            "received RPC response bytes"
        );

        // Try to deserialize as response with timestamps first (new format)
        // Fall back to legacy format for backward compatibility
        let response: RaftRpcResponse = if let Ok(response_with_ts) =
            postcard::from_bytes::<RaftRpcResponseWithTimestamps>(&response_buf)
        {
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
            postcard::from_bytes::<RaftRpcResponse>(&response_buf)
                .map_err(|e| {
                    error!(
                        error = %e,
                        bytes_len = response_buf.len(),
                        first_bytes = ?response_buf.get(..20.min(response_buf.len())),
                        "failed to deserialize RPC response"
                    );
                    e
                })
                .context("failed to deserialize RPC response")?
        };

        // Update failure detector: RPC succeeded with both connection and Raft working
        self.failure_detector.write().await.update_node_status(
            self.target,
            ConnectionStatus::Connected,
            ConnectionStatus::Connected,
        );

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
        let (raft_status, iroh_status) = if err.to_string().contains("connection pool")
            || err.to_string().contains("peer address not found")
        {
            (
                ConnectionStatus::Disconnected,
                ConnectionStatus::Disconnected,
            )
        } else if err.to_string().contains("stream") {
            (ConnectionStatus::Disconnected, ConnectionStatus::Connected)
        } else {
            // Timeout or other RPC-level error - assume connection is ok but Raft is having issues
            (ConnectionStatus::Disconnected, ConnectionStatus::Connected)
        };

        self.failure_detector.write().await.update_node_status(
            self.target,
            raft_status,
            iroh_status,
        );
    }
}

#[allow(clippy::blocks_in_conditions)]
impl RaftNetworkV2<AppTypeConfig> for IrpcRaftNetwork {
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
                return Err(RPCError::Unreachable(Unreachable::new(
                    &std::io::Error::other(err_str),
                )));
            }
        };

        // Extract result from response
        match response {
            RaftRpcResponse::AppendEntries(result) => Ok(result),
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
                return Err(RPCError::Unreachable(Unreachable::new(
                    &std::io::Error::other(err_str),
                )));
            }
        };

        // Extract result from response
        match response {
            RaftRpcResponse::Vote(result) => Ok(result),
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
                StreamingError::StorageError(StorageError::read_snapshot(
                    Some(snapshot.meta.signature()),
                    &err,
                ))
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
                        format!(
                            "snapshot exceeds maximum size of {} bytes",
                            MAX_SNAPSHOT_SIZE
                        ),
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

        // Extract result from response
        match response {
            RaftRpcResponse::InstallSnapshot(result) => {
                // Handle remote RaftError as StorageError since snapshot installation failed
                result.map_err(|raft_err| {
                    StreamingError::StorageError(StorageError::read_snapshot(None, &raft_err))
                })
            }
            _ => Err(StreamingError::Unreachable(Unreachable::new(
                &std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unexpected response type for install_snapshot",
                ),
            ))),
        }
    }
}
