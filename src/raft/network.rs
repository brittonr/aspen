use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use anyhow::Context;
use openraft::error::{NetworkError, RPCError, ReplicationClosed, StreamingError, Unreachable};
use openraft::network::{RPCOption, RaftNetworkFactory, v2::RaftNetworkV2};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, SnapshotResponse, VoteRequest, VoteResponse,
};
use openraft::type_config::alias::VoteOf;
use openraft::{BasicNode, OptionalSend, Snapshot, StorageError};
use tokio::io::AsyncReadExt;
use tokio::select;
use tracing::warn;

use crate::cluster::IrohEndpointManager;
use crate::raft::node_failure_detection::{ConnectionStatus, NodeFailureDetector};
use crate::raft::rpc::{
    RaftAppendEntriesRequest, RaftRpcProtocol, RaftRpcResponse, RaftSnapshotRequest,
    RaftVoteRequest,
};
use crate::raft::types::{AppTypeConfig, NodeId};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum size for RPC messages (10 MB).
///
/// Tiger Style: Fixed limit to prevent unbounded memory use.
const MAX_RPC_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;

/// Timeout for Iroh connection establishment (5 seconds).
///
/// Tiger Style: Explicit timeout prevents indefinite hangs on unreachable peers.
const IROH_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for bidirectional stream open (2 seconds).
///
/// Tiger Style: Bounded wait for stream establishment after connection succeeds.
const IROH_STREAM_OPEN_TIMEOUT: Duration = Duration::from_secs(2);

/// Timeout for RPC response read (10 seconds).
///
/// Accounts for slow snapshot transfers and disk I/O.
/// Tiger Style: Prevents indefinite blocking on slow or stalled peers.
const IROH_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// IRPC-based Raft network factory for Iroh P2P transport.
///
/// Tiger Style: Fixed peer map, explicit endpoint management.
#[derive(Clone)]
pub struct IrpcRaftNetworkFactory {
    endpoint_manager: Arc<IrohEndpointManager>,
    /// Map of NodeId to Iroh EndpointAddr for peer discovery.
    ///
    /// Initially populated from:
    /// - CLI args/config manual peers (`--peers` flag)
    /// - Empty if using automatic discovery
    ///
    /// Dynamically updated via:
    /// - Gossip announcements (when `enable_gossip: true`)
    /// - `add_peer()`/`update_peers()` calls (manual/testing)
    ///
    /// Uses Arc<RwLock> to allow concurrent peer addition during runtime.
    peer_addrs: Arc<RwLock<HashMap<NodeId, iroh::EndpointAddr>>>,
    /// Failure detector for distinguishing actor crashes from node crashes.
    ///
    /// Updated automatically based on Raft RPC success/failure and Iroh connection status.
    failure_detector: Arc<RwLock<NodeFailureDetector>>,
}

impl IrpcRaftNetworkFactory {
    pub fn new(
        endpoint_manager: Arc<IrohEndpointManager>,
        peer_addrs: HashMap<NodeId, iroh::EndpointAddr>,
    ) -> Self {
        Self {
            endpoint_manager,
            peer_addrs: Arc::new(RwLock::new(peer_addrs)),
            failure_detector: Arc::new(RwLock::new(NodeFailureDetector::default_timeout())),
        }
    }

    /// Get a reference to the failure detector for metrics/monitoring.
    pub fn failure_detector(&self) -> Arc<RwLock<NodeFailureDetector>> {
        Arc::clone(&self.failure_detector)
    }

    /// Add a peer address for future connections.
    ///
    /// This allows dynamic peer addition after the network factory has been created.
    /// Useful for integration tests where nodes exchange addresses at runtime.
    pub async fn add_peer(&self, node_id: NodeId, addr: iroh::EndpointAddr) {
        let mut peers = self.peer_addrs.write().await;
        peers.insert(node_id, addr);
    }

    /// Update peer addresses in bulk.
    ///
    /// Extends the existing peer map with new entries. Existing entries are replaced.
    pub async fn update_peers(&self, new_peers: HashMap<NodeId, iroh::EndpointAddr>) {
        let mut peers = self.peer_addrs.write().await;
        peers.extend(new_peers);
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

    #[tracing::instrument(level = "debug", skip_all)]
    async fn new_client(&mut self, target: NodeId, _node: &BasicNode) -> Self::Network {
        // Look up peer's Iroh address
        let peer_addr = {
            let peers = self.peer_addrs.read().await;
            peers.get(&target).cloned()
        };

        IrpcRaftNetwork {
            endpoint_manager: Arc::clone(&self.endpoint_manager),
            peer_addr,
            target,
            failure_detector: Arc::clone(&self.failure_detector),
        }
    }
}

/// IRPC-based Raft network client for a single peer.
///
/// Tiger Style:
/// - Explicit error handling for connection failures
/// - Fail fast if peer address is missing
/// - Bounded retries handled by IRPC/Iroh layers
pub struct IrpcRaftNetwork {
    endpoint_manager: Arc<IrohEndpointManager>,
    peer_addr: Option<iroh::EndpointAddr>,
    target: NodeId,
    failure_detector: Arc<RwLock<NodeFailureDetector>>,
}

impl IrpcRaftNetwork {
    /// Send an RPC request to the peer and wait for response.
    ///
    /// Tiger Style: Fail fast if peer address is unknown.
    ///
    /// This implements a simple request-response pattern:
    /// 1. Serialize the protocol enum (request without channels)
    /// 2. Send it over Iroh bidirectional stream
    /// 3. Wait for response on the same stream
    /// 4. Deserialize and return the response
    ///
    /// Updates failure detector based on RPC success/failure and Iroh connection status.
    async fn send_rpc(&self, request: RaftRpcProtocol) -> anyhow::Result<RaftRpcResponse> {
        let peer_addr = self
            .peer_addr
            .as_ref()
            .context("peer address not found in peer map")?;

        let endpoint = self.endpoint_manager.endpoint();

        // Open a connection to the peer (with timeout)
        let connection_result = tokio::time::timeout(
            IROH_CONNECT_TIMEOUT,
            endpoint.connect(peer_addr.clone(), b"raft-rpc"),
        )
        .await
        .context("timeout connecting to peer")?;

        // Determine Iroh connection status based on connect attempt
        let iroh_status = if connection_result.is_ok() {
            ConnectionStatus::Connected
        } else {
            ConnectionStatus::Disconnected
        };

        let connection = connection_result.context("failed to connect to peer")?;

        // Open bidirectional stream (with timeout)
        let (mut send_stream, mut recv_stream) =
            tokio::time::timeout(IROH_STREAM_OPEN_TIMEOUT, connection.open_bi())
                .await
                .context("timeout opening bidirectional stream")?
                .context("failed to open bidirectional stream")?;

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

        // Deserialize response
        let response: RaftRpcResponse =
            postcard::from_bytes(&response_buf).context("failed to deserialize RPC response")?;

        // Update failure detector: RPC succeeded, Iroh connection succeeded
        self.failure_detector.write().await.update_node_status(
            self.target,
            ConnectionStatus::Connected,
            iroh_status,
        );

        Ok(response)
    }

    /// Update failure detector when RPC fails.
    ///
    /// Called by RPC methods when send_rpc returns an error.
    /// Assumes Iroh connection status based on error type.
    async fn update_failure_on_rpc_error(&self, err: &anyhow::Error) {
        // Determine Iroh status from error message
        // Connection/stream errors suggest Iroh is down
        // Other errors (serialization, etc.) suggest Iroh is up but Raft is down
        let iroh_status = if err.to_string().contains("connect")
            || err.to_string().contains("stream")
            || err.to_string().contains("peer address not found")
        {
            ConnectionStatus::Disconnected
        } else {
            ConnectionStatus::Connected
        };

        self.failure_detector.write().await.update_node_status(
            self.target,
            ConnectionStatus::Disconnected,
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
        // Read snapshot data into bytes
        let mut snapshot_data = Vec::new();
        let mut snapshot_reader = snapshot.snapshot;
        snapshot_reader
            .read_to_end(&mut snapshot_data)
            .await
            .map_err(|err| {
                StreamingError::StorageError(StorageError::read_snapshot(
                    Some(snapshot.meta.signature()),
                    &err,
                ))
            })?;

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
