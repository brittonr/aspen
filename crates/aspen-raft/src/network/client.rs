//! Single-peer Raft network client for sending RPCs.

use std::future::Future;
use std::sync::Arc;

use anyhow::Context;
use aspen_core::NetworkTransport;
use aspen_raft_types::StreamPriority;
use aspen_sharding::ShardId;
use openraft::OptionalSend;
use openraft::Snapshot;
use openraft::StorageError;
use openraft::error::NetworkError;
use openraft::error::RPCError;
use openraft::error::ReplicationClosed;
use openraft::error::StreamingError;
use openraft::error::Unreachable;
use openraft::network::RPCOption;
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
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::FailureDetectorUpdate;
use crate::clock_drift_detection::ClockDriftDetector;
use crate::clock_drift_detection::current_time_ms;
use crate::connection_pool::RaftConnectionPool;
use crate::constants::IROH_READ_TIMEOUT;
use crate::constants::MAX_RPC_MESSAGE_SIZE;
use crate::constants::MAX_SNAPSHOT_SIZE;
use crate::node_failure_detection::ConnectionStatus;
use crate::node_failure_detection::NodeFailureDetector;
use crate::rpc::RaftAppendEntriesRequest;
use crate::rpc::RaftRpcProtocol;
use crate::rpc::RaftRpcResponse;
use crate::rpc::RaftSnapshotRequest;
use crate::rpc::RaftVoteRequest;
use crate::types::AppTypeConfig;
use crate::types::NodeId;
use crate::verified::classify_response_health;
use crate::verified::classify_rpc_error;
use crate::verified::deserialize_rpc_response;
use crate::verified::extract_sharded_response;
use crate::verified::maybe_prefix_shard_id;

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
    /// Shared gossip-populated peer address cache for address refresh.
    ///
    /// After a peer restarts with a new port, gossip announcements update
    /// this cache. On RPC failure, we check for a fresher address here
    /// before retrying with the stale Raft membership address.
    gossip_addrs: Option<Arc<RwLock<std::collections::HashMap<NodeId, iroh::EndpointAddr>>>>,
    /// Optional snapshot transfer history for observability.
    snapshot_history: Option<Arc<aspen_transport::snapshot_history::SnapshotTransferHistory>>,
}

impl<T> IrpcRaftNetwork<T>
where T: NetworkTransport<Endpoint = iroh::Endpoint, Address = iroh::EndpointAddr> + 'static
{
    /// Create a new network client for a single peer.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        connection_pool: Arc<RaftConnectionPool<T>>,
        peer_addr: Option<iroh::EndpointAddr>,
        target: NodeId,
        failure_detector: Arc<RwLock<NodeFailureDetector>>,
        drift_detector: Arc<RwLock<ClockDriftDetector>>,
        shard_id: Option<ShardId>,
        failure_update_tx: tokio::sync::mpsc::Sender<FailureDetectorUpdate>,
    ) -> Self {
        Self {
            connection_pool,
            peer_addr,
            target,
            failure_detector,
            drift_detector,
            shard_id,
            failure_update_tx,
            gossip_addrs: None,
            snapshot_history: None,
        }
    }

    /// Attach a snapshot transfer history buffer for observability.
    #[expect(dead_code, reason = "wired in task 5.4 when connection pool is exposed")]
    pub(crate) fn with_snapshot_history(
        mut self,
        history: Arc<aspen_transport::snapshot_history::SnapshotTransferHistory>,
    ) -> Self {
        self.snapshot_history = Some(history);
        self
    }

    /// Attach a shared gossip address cache for dynamic address refresh.
    ///
    /// When set, `send_rpc` will check this cache for fresher addresses
    /// after connection failures — enabling reconnection after peer restarts.
    pub(crate) fn with_gossip_addrs(
        mut self,
        addrs: Arc<RwLock<std::collections::HashMap<NodeId, iroh::EndpointAddr>>>,
    ) -> Self {
        self.gossip_addrs = Some(addrs);
        self
    }

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
    async fn send_rpc(&self, request: RaftRpcProtocol, priority: StreamPriority) -> anyhow::Result<RaftRpcResponse> {
        let peer_addr = self.peer_addr.as_ref().context("peer address not found in peer map")?;

        // Acquire connection and stream. On failure, check gossip cache for
        // a fresher address (peer may have restarted with a new port).
        let mut stream_handle = match self.send_rpc_acquire_stream(peer_addr, priority).await {
            Ok(sh) => sh,
            Err(first_err) => {
                if let Some(refreshed) = self.try_refresh_addr_from_gossip(peer_addr).await {
                    info!(
                        target_node = %self.target,
                        "retrying RPC with gossip-refreshed address"
                    );
                    self.send_rpc_acquire_stream(&refreshed, priority).await?
                } else {
                    return Err(first_err);
                }
            }
        };

        // Record client send time (t1) for clock drift detection
        let client_send_ms = current_time_ms();

        // Serialize and send the request
        self.send_rpc_write_request(&mut stream_handle, &request).await?;

        // Read response (with size and time limits)
        let response_buf = self.send_rpc_read_response(&mut stream_handle).await?;

        // Record client receive time (t4) for clock drift detection
        let client_recv_ms = current_time_ms();

        // Process response and update failure/drift detectors
        self.send_rpc_process_response(&response_buf, client_send_ms, client_recv_ms).await
    }

    /// Check gossip cache for a fresher address after connection failure.
    ///
    /// Returns `Some(addr)` if the gossip cache has an address with the same
    /// endpoint ID but different socket addresses — indicating the peer restarted.
    async fn try_refresh_addr_from_gossip(&self, current_addr: &iroh::EndpointAddr) -> Option<iroh::EndpointAddr> {
        let cache = self.gossip_addrs.as_ref()?;
        let peers = cache.read().await;
        let gossip_addr = peers.get(&self.target)?;
        if gossip_addr.id == current_addr.id && gossip_addr.addrs != current_addr.addrs {
            Some(gossip_addr.clone())
        } else {
            None
        }
    }

    /// Acquire connection and stream for RPC.
    async fn send_rpc_acquire_stream(
        &self,
        peer_addr: &iroh::EndpointAddr,
        priority: StreamPriority,
    ) -> anyhow::Result<crate::connection_pool::StreamHandle> {
        // Get or create connection from pool
        //
        // Error classification: Connection failure (timeout, refused, unreachable) is
        // classified as NodeCrash (both Raft and Iroh disconnected).
        let peer_connection = self
            .connection_pool
            .get_or_connect(self.target, peer_addr)
            .await
            .inspect_err(|err| {
                tracing::debug!(
                    target = %self.target,
                    error = %err,
                    "connection failure, classifying as NodeCrash"
                );
                // Tiger Style: Use bounded channel instead of spawning unbounded tasks
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
        // as ActorCrash (Iroh connected but Raft disconnected).
        peer_connection
            .acquire_stream(priority)
            .await
            .inspect_err(|err| {
                tracing::debug!(
                    target = %self.target,
                    error = %err,
                    "stream failure with connection up, classifying as ActorCrash"
                );
                let _ = self.failure_update_tx.try_send(FailureDetectorUpdate {
                    node_id: self.target,
                    raft_status: ConnectionStatus::Disconnected,
                    iroh_status: ConnectionStatus::Connected,
                });
            })
            .context("failed to acquire stream from connection")
    }

    /// Serialize and send the RPC request.
    async fn send_rpc_write_request(
        &self,
        stream_handle: &mut crate::connection_pool::StreamHandle,
        request: &RaftRpcProtocol,
    ) -> anyhow::Result<()> {
        let serialized = postcard::to_stdvec(request).context("failed to serialize RPC request")?;
        let message = maybe_prefix_shard_id(serialized, self.shard_id);
        stream_handle.send.write_all(&message).await.context("failed to write RPC request to stream")?;
        stream_handle.send.finish().context("failed to finish send stream")?;
        Ok(())
    }

    /// Read the RPC response with timeout.
    async fn send_rpc_read_response(
        &self,
        stream_handle: &mut crate::connection_pool::StreamHandle,
    ) -> anyhow::Result<Vec<u8>> {
        tokio::time::timeout(IROH_READ_TIMEOUT, stream_handle.recv.read_to_end(MAX_RPC_MESSAGE_SIZE as usize))
            .await
            .context("timeout reading RPC response")?
            .context("failed to read RPC response")
    }

    /// Process RPC response: validate, deserialize, and update detectors.
    async fn send_rpc_process_response(
        &self,
        response_buf: &[u8],
        client_send_ms: u64,
        client_recv_ms: u64,
    ) -> anyhow::Result<RaftRpcResponse> {
        // CRITICAL: Detect empty response buffer
        if response_buf.is_empty() {
            error!(
                target_node = %self.target,
                shard_id = ?self.shard_id,
                "received empty response buffer from peer - peer may have panicked or closed stream prematurely"
            );
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
        let response_bytes =
            extract_sharded_response(response_buf, self.shard_id).map_err(|e| anyhow::anyhow!("{}", e))?;

        debug!(
            response_size = response_bytes.len(),
            shard_id = ?self.shard_id,
            "received RPC response bytes"
        );

        // Deserialize response with backward compatibility
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

        // Classify response health and update failure detector
        let (raft_status, iroh_status) = classify_response_health(&response);

        if let RaftRpcResponse::FatalError(error_kind) = &response {
            warn!(
                target_node = %self.target,
                error_kind = %error_kind,
                "peer reported fatal RaftCore error"
            );
        }

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

        // Send the RPC and get response (Critical: heartbeats must beat bulk data)
        let response = match self.send_rpc(protocol, StreamPriority::Critical).await {
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

        // Send the RPC and get response (Critical: votes must beat bulk data)
        let response = match self.send_rpc(protocol, StreamPriority::Critical).await {
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

        let snapshot_size = snapshot_data.len() as f64;
        let snapshot_send_start = std::time::Instant::now();
        let peer_label = self.target.to_string();

        let request = RaftSnapshotRequest {
            vote,
            snapshot_meta: snapshot.meta,
            snapshot_data,
        };
        let protocol = RaftRpcProtocol::InstallSnapshot(request);

        // Send the RPC with cancellation support (Bulk: snapshots are large)
        let response = select! {
            send_result = self.send_rpc(protocol, StreamPriority::Bulk) => {
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
        let result = match response {
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
        };

        // Record snapshot transfer metrics
        let elapsed_ms = snapshot_send_start.elapsed().as_secs_f64() * 1000.0;
        let outcome = if result.is_ok() { "success" } else { "error" };
        metrics::histogram!("aspen.snapshot.transfer_size_bytes", "direction" => "send", "peer" => peer_label.clone())
            .record(snapshot_size);
        metrics::histogram!("aspen.snapshot.transfer_duration_ms", "direction" => "send", "peer" => peer_label)
            .record(elapsed_ms);
        metrics::counter!("aspen.snapshot.transfers_total", "direction" => "send", "outcome" => outcome).increment(1);
        if let Some(ref history) = self.snapshot_history {
            let now_us =
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_micros()
                    as u64;
            history.push(aspen_transport::snapshot_history::SnapshotTransferEntry {
                peer_id: self.target.0,
                direction: "send",
                size_bytes: snapshot_size as u64,
                duration_ms: elapsed_ms as u64,
                outcome,
                timestamp_us: now_us,
            });
        }

        result
    }
}
