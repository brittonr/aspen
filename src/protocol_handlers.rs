//! Protocol handlers for Iroh Router-based ALPN dispatching.
//!
//! This module provides `ProtocolHandler` implementations for different protocols:
//! - `RaftProtocolHandler`: Handles Raft RPC connections (ALPN: `raft-rpc`)
//! - `AuthenticatedRaftProtocolHandler`: Handles authenticated Raft RPC (ALPN: `raft-auth`)
//! - `LogSubscriberProtocolHandler`: Handles log subscription connections (ALPN: `aspen-logs`)
//! - `ClientProtocolHandler`: Handles client connections (ALPN: `aspen-tui`)
//!
//! These handlers are registered with an Iroh Router to properly dispatch
//! incoming connections based on their ALPN, eliminating the race condition
//! that occurred when both servers were accepting from the same endpoint.
//!
//! # Architecture
//!
//! ```text
//! Iroh Endpoint
//!       |
//!       v
//!   Router (ALPN dispatch)
//!       |
//!       +---> raft-auth ALPN --> AuthenticatedRaftProtocolHandler (recommended)
//!       |
//!       +---> raft-rpc ALPN ---> RaftProtocolHandler (legacy, no auth)
//!       |
//!       +---> aspen-logs ALPN -> LogSubscriberProtocolHandler (read-only)
//!       |
//!       +---> aspen-tui ALPN --> ClientProtocolHandler
//!       |
//!       +---> gossip ALPN -----> Gossip (via iroh-gossip)
//! ```
//!
//! # Authentication
//!
//! The `AuthenticatedRaftProtocolHandler` uses HMAC-SHA256 challenge-response
//! authentication based on the cluster cookie. This prevents unauthorized nodes
//! from participating in consensus.
//!
//! # Tiger Style
//!
//! - Bounded connection and stream limits per handler
//! - Explicit error handling with AcceptError
//! - Clean shutdown via ProtocolHandler::shutdown()
//!
//! # Test Coverage
//!
//! TODO: Add unit tests for RaftProtocolHandler:
//!       - accept() with valid Raft RPC messages
//!       - Connection semaphore limiting MAX_CONCURRENT_CONNECTIONS
//!       - Stream semaphore limiting MAX_STREAMS_PER_CONNECTION
//!       - Error handling for malformed RPC messages
//!       Coverage: 0% line coverage (tested via integration tests)
//!
//! TODO: Add unit tests for ClientProtocolHandler:
//!       - accept() with valid Client RPC messages
//!       - Connection handling for all Client commands
//!       - Graceful shutdown behavior

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use openraft::Raft;
use tokio::sync::{Semaphore, broadcast};
use tracing::{debug, error, info, instrument, warn};

use crate::api::{
    AddLearnerRequest, ChangeMembershipRequest, ClusterController, InitRequest, KeyValueStore,
    ReadRequest, WriteRequest,
};
use crate::blob::IrohBlobStore;
use crate::client_rpc::{
    AddBlobResultResponse, AddLearnerResultResponse, AddPeerResultResponse,
    BlobListEntry as RpcBlobListEntry, ChangeMembershipResultResponse, CheckpointWalResultResponse,
    ClientRpcRequest, ClientRpcResponse, ClientTicketResponse as ClientTicketRpcResponse,
    ClusterStateResponse, ClusterTicketResponse, DeleteResultResponse,
    DocsTicketResponse as DocsTicketRpcResponse, GetBlobResultResponse,
    GetBlobTicketResultResponse, HasBlobResultResponse, HealthResponse, InitResultResponse,
    ListBlobsResultResponse, MAX_CLIENT_MESSAGE_SIZE, MAX_CLUSTER_NODES, MetricsResponse,
    NodeDescriptor, NodeInfoResponse, PromoteLearnerResultResponse, ProtectBlobResultResponse,
    RaftMetricsResponse, ReadResultResponse, ScanEntry, ScanResultResponse, SnapshotResultResponse,
    UnprotectBlobResultResponse, VaultKeysResponse, VaultListResponse, WriteResultResponse,
};
use crate::cluster::IrohEndpointManager;
use crate::raft::auth::{
    AUTH_HANDSHAKE_TIMEOUT, AuthContext, AuthResponse, AuthResult, MAX_AUTH_MESSAGE_SIZE,
};
use crate::raft::constants::{
    MAX_CONCURRENT_CONNECTIONS, MAX_RPC_MESSAGE_SIZE, MAX_STREAMS_PER_CONNECTION,
};
use crate::raft::log_subscriber::{
    EndOfStreamReason, LOG_BROADCAST_BUFFER_SIZE, LogEntryMessage, LogEntryPayload,
    MAX_LOG_SUBSCRIBERS, SUBSCRIBE_HANDSHAKE_TIMEOUT, SUBSCRIBE_KEEPALIVE_INTERVAL,
    SubscribeRejectReason, SubscribeRequest, SubscribeResponse,
};
use crate::raft::rpc::{RaftRpcProtocol, RaftRpcResponse};
use crate::raft::types::AppTypeConfig;

/// ALPN protocol identifier for Raft RPC (legacy, no authentication).
pub const RAFT_ALPN: &[u8] = b"raft-rpc";

/// ALPN protocol identifier for authenticated Raft RPC.
///
/// Uses HMAC-SHA256 challenge-response authentication based on the cluster cookie.
/// This is the recommended ALPN for production deployments.
pub const RAFT_AUTH_ALPN: &[u8] = b"raft-auth";

/// Re-export LOG_SUBSCRIBER_ALPN for convenience.
pub use crate::raft::log_subscriber::LOG_SUBSCRIBER_ALPN;

/// ALPN protocol identifier for Client RPC.
pub const CLIENT_ALPN: &[u8] = b"aspen-client";

/// Maximum concurrent Client connections.
///
/// Tiger Style: Lower limit than Raft since client connections are less critical.
const MAX_CLIENT_CONNECTIONS: u32 = 50;

/// Maximum concurrent streams per Client connection.
const MAX_CLIENT_STREAMS_PER_CONNECTION: u32 = 10;

/// Protocol handler for Raft RPC over Iroh.
///
/// Implements `ProtocolHandler` to receive connections routed by the Iroh Router
/// based on the `raft-rpc` ALPN. Processes Raft RPC messages (Vote, AppendEntries,
/// InstallSnapshot) and forwards them to the Raft core.
///
/// # Tiger Style
///
/// - Bounded connection count via semaphore
/// - Bounded stream count per connection
/// - Explicit size limits on RPC messages
#[derive(Debug)]
pub struct RaftProtocolHandler {
    raft_core: Raft<AppTypeConfig>,
    connection_semaphore: Arc<Semaphore>,
}

impl RaftProtocolHandler {
    /// Create a new Raft protocol handler.
    ///
    /// # Arguments
    /// * `raft_core` - Raft instance to forward RPCs to
    pub fn new(raft_core: Raft<AppTypeConfig>) -> Self {
        Self {
            raft_core,
            connection_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS as usize)),
        }
    }
}

impl ProtocolHandler for RaftProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_node_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Raft connection limit reached ({}), rejecting connection from {}",
                    MAX_CONCURRENT_CONNECTIONS, remote_node_id
                );
                return Err(AcceptError::from_err(std::io::Error::other(
                    "connection limit reached",
                )));
            }
        };

        debug!(remote_node = %remote_node_id, "accepted Raft RPC connection");

        // Handle the connection with bounded resources
        let result = handle_raft_connection(connection, self.raft_core.clone()).await;

        // Release permit when done
        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Raft protocol handler shutting down");
        // Close the semaphore to prevent new connections
        self.connection_semaphore.close();
    }
}

/// Handle a single Raft RPC connection.
///
/// Tiger Style: Bounded stream count per connection.
#[instrument(skip(connection, raft_core))]
async fn handle_raft_connection(
    connection: Connection,
    raft_core: Raft<AppTypeConfig>,
) -> anyhow::Result<()> {
    let remote_node_id = connection.remote_id();

    // Tiger Style: Fixed limit on concurrent streams per connection
    let stream_semaphore = Arc::new(Semaphore::new(MAX_STREAMS_PER_CONNECTION as usize));
    let active_streams = Arc::new(AtomicU32::new(0));

    // Accept bidirectional streams from this connection
    loop {
        let stream = match connection.accept_bi().await {
            Ok(stream) => stream,
            Err(err) => {
                // Connection closed or error
                debug!(remote_node = %remote_node_id, error = %err, "Raft connection closed");
                break;
            }
        };

        // Try to acquire a stream permit
        let permit = match stream_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    remote_node = %remote_node_id,
                    max_streams = MAX_STREAMS_PER_CONNECTION,
                    "Raft stream limit reached, dropping stream"
                );
                continue;
            }
        };

        active_streams.fetch_add(1, Ordering::Relaxed);
        let active_streams_clone = active_streams.clone();

        let raft_core_clone = raft_core.clone();
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = handle_raft_rpc_stream((recv, send), raft_core_clone).await {
                error!(error = %err, "failed to handle Raft RPC stream");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single Raft RPC message on a bidirectional stream.
#[instrument(skip(recv, send, raft_core))]
async fn handle_raft_rpc_stream(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    raft_core: Raft<AppTypeConfig>,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Read the RPC message with size limit
    let buffer = recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
        .await
        .context("failed to read RPC message")?;

    // Deserialize the RPC request
    let request: RaftRpcProtocol =
        postcard::from_bytes(&buffer).context("failed to deserialize RPC request")?;

    debug!(request_type = ?request, "received Raft RPC request");

    // Process the RPC and create response
    let response = match request {
        RaftRpcProtocol::Vote(vote_req) => {
            let result = match raft_core.vote(vote_req.request).await {
                Ok(result) => result,
                Err(err) => {
                    error!(error = ?err, "vote RPC failed with fatal error");
                    return Err(anyhow::anyhow!("vote failed: {:?}", err));
                }
            };
            RaftRpcResponse::Vote(result)
        }
        RaftRpcProtocol::AppendEntries(append_req) => {
            let result = match raft_core.append_entries(append_req.request).await {
                Ok(result) => result,
                Err(err) => {
                    error!(error = ?err, "append_entries RPC failed with fatal error");
                    return Err(anyhow::anyhow!("append_entries failed: {:?}", err));
                }
            };
            RaftRpcResponse::AppendEntries(result)
        }
        RaftRpcProtocol::InstallSnapshot(snapshot_req) => {
            let snapshot_cursor = Cursor::new(snapshot_req.snapshot_data);
            let snapshot = openraft::Snapshot {
                meta: snapshot_req.snapshot_meta,
                snapshot: snapshot_cursor,
            };
            let result = raft_core
                .install_full_snapshot(snapshot_req.vote, snapshot)
                .await
                .map_err(openraft::error::RaftError::Fatal);
            RaftRpcResponse::InstallSnapshot(result)
        }
    };

    // Serialize and send response
    let response_bytes =
        postcard::to_stdvec(&response).context("failed to serialize RPC response")?;

    debug!(
        response_size = response_bytes.len(),
        "sending Raft RPC response"
    );

    send.write_all(&response_bytes)
        .await
        .context("failed to write RPC response")?;
    send.finish().context("failed to finish send stream")?;

    debug!("Raft RPC response sent successfully");

    Ok(())
}

// ============================================================================
// Authenticated Raft Protocol Handler
// ============================================================================

/// Protocol handler for authenticated Raft RPC over Iroh.
///
/// Uses HMAC-SHA256 challenge-response authentication to verify that connecting
/// nodes share the same cluster cookie before processing Raft RPCs.
///
/// # Tiger Style
///
/// - Bounded connection count via semaphore
/// - Explicit authentication timeout
/// - Fixed nonce and HMAC sizes
#[derive(Debug)]
pub struct AuthenticatedRaftProtocolHandler {
    raft_core: Raft<AppTypeConfig>,
    auth_context: AuthContext,
    connection_semaphore: Arc<Semaphore>,
    /// Our endpoint ID for verification.
    #[allow(dead_code)] // TODO: Use for enhanced security verification
    our_endpoint_id: [u8; 32],
}

impl AuthenticatedRaftProtocolHandler {
    /// Create a new authenticated Raft protocol handler.
    ///
    /// # Arguments
    /// * `raft_core` - Raft instance to forward RPCs to
    /// * `cluster_cookie` - Shared secret for authentication
    /// * `our_endpoint_id` - This node's Iroh endpoint ID
    pub fn new(
        raft_core: Raft<AppTypeConfig>,
        cluster_cookie: &str,
        our_endpoint_id: [u8; 32],
    ) -> Self {
        Self {
            raft_core,
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS as usize)),
            our_endpoint_id,
        }
    }
}

impl ProtocolHandler for AuthenticatedRaftProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_node_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Authenticated Raft connection limit reached ({}), rejecting connection from {}",
                    MAX_CONCURRENT_CONNECTIONS, remote_node_id
                );
                return Err(AcceptError::from_err(std::io::Error::other(
                    "connection limit reached",
                )));
            }
        };

        debug!(remote_node = %remote_node_id, "accepted authenticated Raft RPC connection");

        // Handle the connection with authentication
        let result = handle_authenticated_raft_connection(
            connection,
            self.raft_core.clone(),
            self.auth_context.clone(),
        )
        .await;

        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Authenticated Raft protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Handle an authenticated Raft RPC connection.
///
/// Performs challenge-response authentication before processing RPCs.
#[instrument(skip(connection, raft_core, auth_context))]
async fn handle_authenticated_raft_connection(
    connection: Connection,
    raft_core: Raft<AppTypeConfig>,
    auth_context: AuthContext,
) -> anyhow::Result<()> {
    let remote_node_id = connection.remote_id();

    // Tiger Style: Fixed limit on concurrent streams per connection
    let stream_semaphore = Arc::new(Semaphore::new(MAX_STREAMS_PER_CONNECTION as usize));
    let active_streams = Arc::new(AtomicU32::new(0));

    // Accept bidirectional streams from this connection
    loop {
        let stream = match connection.accept_bi().await {
            Ok(stream) => stream,
            Err(err) => {
                debug!(remote_node = %remote_node_id, error = %err, "Authenticated Raft connection closed");
                break;
            }
        };

        // Try to acquire a stream permit
        let permit = match stream_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    remote_node = %remote_node_id,
                    max_streams = MAX_STREAMS_PER_CONNECTION,
                    "Authenticated Raft stream limit reached, dropping stream"
                );
                continue;
            }
        };

        active_streams.fetch_add(1, Ordering::Relaxed);
        let active_streams_clone = active_streams.clone();

        let raft_core_clone = raft_core.clone();
        let auth_context_clone = auth_context.clone();
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) =
                handle_authenticated_raft_stream((recv, send), raft_core_clone, auth_context_clone)
                    .await
            {
                error!(error = %err, "failed to handle authenticated Raft RPC stream");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single authenticated Raft RPC stream.
///
/// 1. Send AuthChallenge
/// 2. Receive AuthResponse
/// 3. Verify HMAC
/// 4. Send AuthResult
/// 5. If authenticated, process Raft RPC
#[instrument(skip(recv, send, raft_core, auth_context))]
async fn handle_authenticated_raft_stream(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    raft_core: Raft<AppTypeConfig>,
    auth_context: AuthContext,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Step 1: Generate and send challenge
    let challenge = auth_context.generate_challenge();
    let challenge_bytes =
        postcard::to_stdvec(&challenge).context("failed to serialize challenge")?;
    send.write_all(&challenge_bytes)
        .await
        .context("failed to send challenge")?;

    // Step 2: Receive response with timeout
    let response_result = tokio::time::timeout(AUTH_HANDSHAKE_TIMEOUT, async {
        let buffer = recv
            .read_to_end(MAX_AUTH_MESSAGE_SIZE)
            .await
            .context("failed to read auth response")?;
        let response: AuthResponse =
            postcard::from_bytes(&buffer).context("failed to deserialize auth response")?;
        Ok::<_, anyhow::Error>(response)
    })
    .await;

    let response = match response_result {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => {
            warn!(error = %err, "authentication failed: bad response");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)
                .context("failed to serialize auth result")?;
            send.write_all(&result_bytes).await.ok();
            send.finish().ok();
            return Err(err);
        }
        Err(_) => {
            warn!("authentication timed out");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)
                .context("failed to serialize auth result")?;
            send.write_all(&result_bytes).await.ok();
            send.finish().ok();
            return Err(anyhow::anyhow!("authentication timeout"));
        }
    };

    // Step 3: Verify the response
    let auth_result = auth_context.verify_response(&challenge, &response);

    // Step 4: Send result
    let result_bytes =
        postcard::to_stdvec(&auth_result).context("failed to serialize auth result")?;
    send.write_all(&result_bytes)
        .await
        .context("failed to send auth result")?;

    if !auth_result.is_ok() {
        warn!(result = ?auth_result, "authentication failed");
        send.finish().ok();
        return Err(anyhow::anyhow!("authentication failed: {:?}", auth_result));
    }

    debug!("authentication successful, processing Raft RPC");

    // Step 5: Read and process the Raft RPC message
    let buffer = recv
        .read_to_end(MAX_RPC_MESSAGE_SIZE as usize)
        .await
        .context("failed to read RPC message")?;

    let request: RaftRpcProtocol =
        postcard::from_bytes(&buffer).context("failed to deserialize RPC request")?;

    debug!(request_type = ?request, "received authenticated Raft RPC request");

    // Process the RPC (same as unauthenticated handler)
    let response = match request {
        RaftRpcProtocol::Vote(vote_req) => {
            let result = match raft_core.vote(vote_req.request).await {
                Ok(result) => result,
                Err(err) => {
                    error!(error = ?err, "vote RPC failed with fatal error");
                    return Err(anyhow::anyhow!("vote failed: {:?}", err));
                }
            };
            RaftRpcResponse::Vote(result)
        }
        RaftRpcProtocol::AppendEntries(append_req) => {
            let result = match raft_core.append_entries(append_req.request).await {
                Ok(result) => result,
                Err(err) => {
                    error!(error = ?err, "append_entries RPC failed with fatal error");
                    return Err(anyhow::anyhow!("append_entries failed: {:?}", err));
                }
            };
            RaftRpcResponse::AppendEntries(result)
        }
        RaftRpcProtocol::InstallSnapshot(snapshot_req) => {
            let snapshot_cursor = Cursor::new(snapshot_req.snapshot_data);
            let snapshot = openraft::Snapshot {
                meta: snapshot_req.snapshot_meta,
                snapshot: snapshot_cursor,
            };
            let result = raft_core
                .install_full_snapshot(snapshot_req.vote, snapshot)
                .await
                .map_err(openraft::error::RaftError::Fatal);
            RaftRpcResponse::InstallSnapshot(result)
        }
    };

    let response_bytes =
        postcard::to_stdvec(&response).context("failed to serialize RPC response")?;

    send.write_all(&response_bytes)
        .await
        .context("failed to write RPC response")?;
    send.finish().context("failed to finish send stream")?;

    debug!("authenticated Raft RPC response sent successfully");

    Ok(())
}

// ============================================================================
// Log Subscriber Protocol Handler
// ============================================================================

/// Protocol handler for log subscription over Iroh.
///
/// Provides a read-only interface for clients to stream committed Raft log entries.
/// Uses the same HMAC-SHA256 authentication as the authenticated Raft handler.
///
/// # Tiger Style
///
/// - Bounded subscriber count
/// - Keepalive for idle connections
/// - Explicit subscription limits
#[derive(Debug)]
pub struct LogSubscriberProtocolHandler {
    auth_context: AuthContext,
    connection_semaphore: Arc<Semaphore>,
    /// Broadcast channel for log entries.
    log_sender: broadcast::Sender<LogEntryPayload>,
    /// Node ID for response messages.
    node_id: u64,
    /// Subscriber ID counter.
    next_subscriber_id: AtomicU64,
}

impl LogSubscriberProtocolHandler {
    /// Create a new log subscriber protocol handler.
    ///
    /// # Arguments
    /// * `cluster_cookie` - Shared secret for authentication
    /// * `node_id` - This node's ID
    ///
    /// # Returns
    /// The handler and a sender that should be used to broadcast log entries.
    pub fn new(cluster_cookie: &str, node_id: u64) -> (Self, broadcast::Sender<LogEntryPayload>) {
        let (log_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        let handler = Self {
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender: log_sender.clone(),
            node_id,
            next_subscriber_id: AtomicU64::new(1),
        };
        (handler, log_sender)
    }

    /// Create a handler with an existing broadcast sender.
    ///
    /// Use this when you need multiple handlers to share the same broadcast channel.
    pub fn with_sender(
        cluster_cookie: &str,
        node_id: u64,
        log_sender: broadcast::Sender<LogEntryPayload>,
    ) -> Self {
        Self {
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender,
            node_id,
            next_subscriber_id: AtomicU64::new(1),
        }
    }
}

impl ProtocolHandler for LogSubscriberProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_node_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Log subscriber limit reached ({}), rejecting connection from {}",
                    MAX_LOG_SUBSCRIBERS, remote_node_id
                );
                return Err(AcceptError::from_err(std::io::Error::other(
                    "subscriber limit reached",
                )));
            }
        };

        let subscriber_id = self.next_subscriber_id.fetch_add(1, Ordering::Relaxed);
        debug!(
            remote_node = %remote_node_id,
            subscriber_id = subscriber_id,
            "accepted log subscriber connection"
        );

        // Handle the subscriber connection
        let result = handle_log_subscriber_connection(
            connection,
            self.auth_context.clone(),
            self.log_sender.subscribe(),
            self.node_id,
            subscriber_id,
        )
        .await;

        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Log subscriber protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Handle a log subscriber connection.
#[instrument(skip(connection, auth_context, log_receiver))]
async fn handle_log_subscriber_connection(
    connection: Connection,
    auth_context: AuthContext,
    mut log_receiver: broadcast::Receiver<LogEntryPayload>,
    node_id: u64,
    subscriber_id: u64,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let remote_node_id = connection.remote_id();

    // Accept the initial stream for authentication and subscription setup
    let (mut send, mut recv) = connection
        .accept_bi()
        .await
        .context("failed to accept subscriber stream")?;

    // Step 1: Send challenge
    let challenge = auth_context.generate_challenge();
    let challenge_bytes =
        postcard::to_stdvec(&challenge).context("failed to serialize challenge")?;
    send.write_all(&challenge_bytes)
        .await
        .context("failed to send challenge")?;

    // Step 2: Receive auth response
    let response_result = tokio::time::timeout(AUTH_HANDSHAKE_TIMEOUT, async {
        let buffer = recv
            .read_to_end(MAX_AUTH_MESSAGE_SIZE)
            .await
            .context("failed to read auth response")?;
        let response: AuthResponse =
            postcard::from_bytes(&buffer).context("failed to deserialize auth response")?;
        Ok::<_, anyhow::Error>(response)
    })
    .await;

    let auth_response = match response_result {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => {
            warn!(error = %err, subscriber_id = subscriber_id, "subscriber auth failed");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)?;
            send.write_all(&result_bytes).await.ok();
            send.finish().ok();
            return Err(err);
        }
        Err(_) => {
            warn!(subscriber_id = subscriber_id, "subscriber auth timed out");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)?;
            send.write_all(&result_bytes).await.ok();
            send.finish().ok();
            return Err(anyhow::anyhow!("authentication timeout"));
        }
    };

    // Step 3: Verify
    let auth_result = auth_context.verify_response(&challenge, &auth_response);

    // Step 4: Send auth result
    let result_bytes = postcard::to_stdvec(&auth_result)?;
    send.write_all(&result_bytes)
        .await
        .context("failed to send auth result")?;

    if !auth_result.is_ok() {
        warn!(subscriber_id = subscriber_id, result = ?auth_result, "subscriber auth failed");
        send.finish().ok();
        return Err(anyhow::anyhow!("authentication failed: {:?}", auth_result));
    }

    debug!(subscriber_id = subscriber_id, "subscriber authenticated");

    // Step 5: Receive subscription request
    let sub_request_result = tokio::time::timeout(SUBSCRIBE_HANDSHAKE_TIMEOUT, async {
        let buffer = recv
            .read_to_end(1024) // Subscription requests are small
            .await
            .context("failed to read subscribe request")?;
        let request: SubscribeRequest =
            postcard::from_bytes(&buffer).context("failed to deserialize subscribe request")?;
        Ok::<_, anyhow::Error>(request)
    })
    .await;

    let sub_request = match sub_request_result {
        Ok(Ok(request)) => request,
        Ok(Err(err)) => {
            let response = SubscribeResponse::Rejected {
                reason: SubscribeRejectReason::InternalError,
            };
            let response_bytes = postcard::to_stdvec(&response)?;
            send.write_all(&response_bytes).await.ok();
            send.finish().ok();
            return Err(err);
        }
        Err(_) => {
            let response = SubscribeResponse::Rejected {
                reason: SubscribeRejectReason::InternalError,
            };
            let response_bytes = postcard::to_stdvec(&response)?;
            send.write_all(&response_bytes).await.ok();
            send.finish().ok();
            return Err(anyhow::anyhow!("subscribe request timeout"));
        }
    };

    debug!(
        subscriber_id = subscriber_id,
        start_index = sub_request.start_index,
        prefix = ?sub_request.key_prefix,
        "processing subscription request"
    );

    // Step 6: Accept subscription
    // TODO: Implement historical replay from start_index if available
    let response = SubscribeResponse::Accepted {
        current_index: 0, // TODO: Get actual committed index
        node_id,
    };
    let response_bytes = postcard::to_stdvec(&response)?;
    send.write_all(&response_bytes)
        .await
        .context("failed to send subscribe response")?;

    info!(
        subscriber_id = subscriber_id,
        remote = %remote_node_id,
        "log subscription active"
    );

    // Step 7: Stream log entries
    let key_prefix = sub_request.key_prefix;
    let mut keepalive_interval = tokio::time::interval(SUBSCRIBE_KEEPALIVE_INTERVAL);
    keepalive_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            // Receive log entry from broadcast channel
            entry_result = log_receiver.recv() => {
                match entry_result {
                    Ok(entry) => {
                        // Apply prefix filter
                        if !key_prefix.is_empty() && !entry.operation.matches_prefix(&key_prefix) {
                            continue;
                        }

                        let message = LogEntryMessage::Entry(entry);
                        let message_bytes = match postcard::to_stdvec(&message) {
                            Ok(bytes) => bytes,
                            Err(err) => {
                                error!(error = %err, "failed to serialize log entry");
                                continue;
                            }
                        };

                        if let Err(err) = send.write_all(&message_bytes).await {
                            debug!(subscriber_id = subscriber_id, error = %err, "subscriber disconnected");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        warn!(
                            subscriber_id = subscriber_id,
                            lagged_count = count,
                            "subscriber lagged, disconnecting"
                        );
                        let end_message = LogEntryMessage::EndOfStream {
                            reason: EndOfStreamReason::Lagged,
                        };
                        if let Ok(bytes) = postcard::to_stdvec(&end_message) {
                            send.write_all(&bytes).await.ok();
                        }
                        break;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!(subscriber_id = subscriber_id, "log broadcast channel closed");
                        let end_message = LogEntryMessage::EndOfStream {
                            reason: EndOfStreamReason::ServerShutdown,
                        };
                        if let Ok(bytes) = postcard::to_stdvec(&end_message) {
                            send.write_all(&bytes).await.ok();
                        }
                        break;
                    }
                }
            }

            // Send keepalive on idle
            _ = keepalive_interval.tick() => {
                let keepalive = LogEntryMessage::Keepalive {
                    committed_index: 0, // TODO: Get actual committed index
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                };
                let message_bytes = match postcard::to_stdvec(&keepalive) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };
                if send.write_all(&message_bytes).await.is_err() {
                    debug!(subscriber_id = subscriber_id, "subscriber disconnected during keepalive");
                    break;
                }
            }
        }
    }

    send.finish().ok();

    info!(subscriber_id = subscriber_id, "log subscription ended");

    Ok(())
}

// ============================================================================
// Client Protocol Handler
// ============================================================================

/// Context for Client protocol handler with all dependencies.
#[derive(Clone)]
pub struct ClientProtocolContext {
    /// Node identifier.
    pub node_id: u64,
    /// Cluster controller for Raft operations.
    pub controller: Arc<dyn ClusterController>,
    /// Key-value store interface.
    pub kv_store: Arc<dyn KeyValueStore>,
    /// Iroh endpoint manager for peer info.
    pub endpoint_manager: Arc<IrohEndpointManager>,
    /// Blob store for content-addressed storage (optional).
    pub blob_store: Option<Arc<IrohBlobStore>>,
    /// Peer manager for cluster-to-cluster sync (optional).
    pub peer_manager: Option<Arc<crate::docs::PeerManager>>,
    /// Cluster cookie for ticket generation.
    pub cluster_cookie: String,
    /// Node start time for uptime calculation.
    pub start_time: Instant,
    /// Network factory for dynamic peer addition (optional).
    ///
    /// When present, enables AddPeer RPC to register peers in the network factory.
    pub network_factory: Option<Arc<crate::raft::network::IrpcRaftNetworkFactory>>,
}

impl std::fmt::Debug for ClientProtocolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientProtocolContext")
            .field("node_id", &self.node_id)
            .field("cluster_cookie", &self.cluster_cookie)
            .finish_non_exhaustive()
    }
}

/// Deprecated: Use `ClientProtocolContext` instead.
#[deprecated(since = "0.1.0", note = "Use ClientProtocolContext instead")]
pub type TuiProtocolContext = ClientProtocolContext;

/// Protocol handler for Client RPC over Iroh.
///
/// Implements `ProtocolHandler` to receive connections routed by the Iroh Router
/// based on the `aspen-tui` ALPN. Processes Client requests for monitoring and
/// control operations.
///
/// # Tiger Style
///
/// - Lower connection limit than Raft (Client is less critical)
/// - Bounded stream count per connection
/// - Explicit size limits on messages
#[derive(Debug)]
pub struct ClientProtocolHandler {
    ctx: Arc<ClientProtocolContext>,
    connection_semaphore: Arc<Semaphore>,
}

impl ClientProtocolHandler {
    /// Create a new Client protocol handler.
    ///
    /// # Arguments
    /// * `ctx` - Client context with dependencies
    pub fn new(ctx: ClientProtocolContext) -> Self {
        Self {
            ctx: Arc::new(ctx),
            connection_semaphore: Arc::new(Semaphore::new(MAX_CLIENT_CONNECTIONS as usize)),
        }
    }
}

impl ProtocolHandler for ClientProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote_node_id = connection.remote_id();

        // Try to acquire a connection permit
        let permit = match self.connection_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Client connection limit reached ({}), rejecting connection from {}",
                    MAX_CLIENT_CONNECTIONS, remote_node_id
                );
                return Err(AcceptError::from_err(std::io::Error::other(
                    "connection limit reached",
                )));
            }
        };

        debug!(remote_node = %remote_node_id, "accepted Client connection");

        // Handle the connection with bounded resources
        let result = handle_client_connection(connection, Arc::clone(&self.ctx)).await;

        // Release permit when done
        drop(permit);

        result.map_err(|err| AcceptError::from_err(std::io::Error::other(err.to_string())))
    }

    async fn shutdown(&self) {
        info!("Client protocol handler shutting down");
        self.connection_semaphore.close();
    }
}

/// Deprecated: Use `ClientProtocolHandler` instead.
#[deprecated(since = "0.1.0", note = "Use ClientProtocolHandler instead")]
pub type TuiProtocolHandler = ClientProtocolHandler;

/// Handle a single Client connection.
#[instrument(skip(connection, ctx))]
async fn handle_client_connection(
    connection: Connection,
    ctx: Arc<ClientProtocolContext>,
) -> anyhow::Result<()> {
    let remote_node_id = connection.remote_id();

    let stream_semaphore = Arc::new(Semaphore::new(MAX_CLIENT_STREAMS_PER_CONNECTION as usize));
    let active_streams = Arc::new(AtomicU32::new(0));

    // Accept bidirectional streams from this connection
    loop {
        let stream = match connection.accept_bi().await {
            Ok(stream) => stream,
            Err(err) => {
                debug!(remote_node = %remote_node_id, error = %err, "Client connection closed");
                break;
            }
        };

        let permit = match stream_semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    remote_node = %remote_node_id,
                    max_streams = MAX_CLIENT_STREAMS_PER_CONNECTION,
                    "Client stream limit reached, dropping stream"
                );
                continue;
            }
        };

        active_streams.fetch_add(1, Ordering::Relaxed);
        let active_streams_clone = active_streams.clone();

        let ctx_clone = Arc::clone(&ctx);
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = handle_client_request((recv, send), ctx_clone).await {
                error!(error = %err, "failed to handle Client request");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single Client RPC request on a stream.
#[instrument(skip(recv, send, ctx))]
async fn handle_client_request(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    ctx: Arc<ClientProtocolContext>,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Read the request with size limit
    let buffer = recv
        .read_to_end(MAX_CLIENT_MESSAGE_SIZE)
        .await
        .context("failed to read Client request")?;

    // Deserialize the request
    let request: ClientRpcRequest =
        postcard::from_bytes(&buffer).context("failed to deserialize Client request")?;

    debug!(request_type = ?request, "received Client request");

    // Process the request and create response
    let response = match process_client_request(request, &ctx).await {
        Ok(resp) => resp,
        Err(err) => {
            warn!(error = %err, "Client request processing failed");
            ClientRpcResponse::error("INTERNAL_ERROR", err.to_string())
        }
    };

    // Serialize and send response
    let response_bytes =
        postcard::to_stdvec(&response).context("failed to serialize Client response")?;
    send.write_all(&response_bytes)
        .await
        .context("failed to write Client response")?;
    send.finish().context("failed to finish send stream")?;

    Ok(())
}

/// Process a Client RPC request and generate response.
async fn process_client_request(
    request: ClientRpcRequest,
    ctx: &ClientProtocolContext,
) -> anyhow::Result<ClientRpcResponse> {
    match request {
        ClientRpcRequest::GetHealth => {
            let uptime_seconds = ctx.start_time.elapsed().as_secs();

            let status = match ctx.controller.get_metrics().await {
                Ok(metrics) => {
                    if metrics.current_leader.is_some() {
                        "healthy"
                    } else {
                        "degraded"
                    }
                }
                Err(_) => "unhealthy",
            };

            Ok(ClientRpcResponse::Health(HealthResponse {
                status: status.to_string(),
                node_id: ctx.node_id,
                raft_node_id: Some(ctx.node_id),
                uptime_seconds,
            }))
        }

        ClientRpcRequest::GetRaftMetrics => {
            let metrics = ctx
                .controller
                .get_metrics()
                .await
                .map_err(|e| anyhow::anyhow!("failed to get Raft metrics: {}", e))?;

            Ok(ClientRpcResponse::RaftMetrics(RaftMetricsResponse {
                node_id: ctx.node_id,
                state: format!("{:?}", metrics.state),
                current_leader: metrics.current_leader.map(|id| id.0),
                current_term: metrics.current_term,
                last_log_index: metrics.last_log_index,
                last_applied_index: metrics.last_applied.as_ref().map(|la| la.index),
                snapshot_index: metrics.snapshot.as_ref().map(|s| s.index),
            }))
        }

        ClientRpcRequest::GetLeader => {
            let leader = ctx
                .controller
                .get_leader()
                .await
                .map_err(|e| anyhow::anyhow!("failed to get leader: {}", e))?;
            Ok(ClientRpcResponse::Leader(leader))
        }

        ClientRpcRequest::GetNodeInfo => {
            let endpoint_addr = ctx.endpoint_manager.node_addr();
            Ok(ClientRpcResponse::NodeInfo(NodeInfoResponse {
                node_id: ctx.node_id,
                endpoint_addr: format!("{:?}", endpoint_addr),
            }))
        }

        ClientRpcRequest::GetClusterTicket => {
            use crate::cluster::ticket::AspenClusterTicket;
            use iroh_gossip::proto::TopicId;

            let hash = blake3::hash(ctx.cluster_cookie.as_bytes());
            let topic_id = TopicId::from_bytes(*hash.as_bytes());

            let ticket = AspenClusterTicket::with_bootstrap(
                topic_id,
                ctx.cluster_cookie.clone(),
                ctx.endpoint_manager.endpoint().id(),
            );

            let ticket_str = ticket.serialize();

            Ok(ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
                ticket: ticket_str,
                topic_id: format!("{:?}", topic_id),
                cluster_id: ctx.cluster_cookie.clone(),
                endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
                bootstrap_peers: Some(1),
            }))
        }

        ClientRpcRequest::InitCluster => {
            let result = ctx
                .controller
                .init(InitRequest {
                    initial_members: vec![],
                })
                .await;

            Ok(ClientRpcResponse::InitResult(InitResultResponse {
                success: result.is_ok(),
                error: result.err().map(|e| e.to_string()),
            }))
        }

        ClientRpcRequest::ReadKey { key } => {
            let result = ctx.kv_store.read(ReadRequest { key: key.clone() }).await;

            match result {
                Ok(resp) => Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
                    value: Some(resp.value.into_bytes()),
                    found: true,
                    error: None,
                })),
                Err(e) => {
                    let (found, error) = match e {
                        crate::api::KeyValueStoreError::NotFound { .. } => (false, None),
                        crate::api::KeyValueStoreError::NotLeader { leader, reason } => (
                            false,
                            Some(format!("not leader: {}; leader={:?}", reason, leader)),
                        ),
                        other => (false, Some(other.to_string())),
                    };
                    Ok(ClientRpcResponse::ReadResult(ReadResultResponse {
                        value: None,
                        found,
                        error,
                    }))
                }
            }
        }

        ClientRpcRequest::WriteKey { key, value } => {
            use crate::api::WriteCommand;

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key,
                        value: String::from_utf8_lossy(&value).to_string(),
                    },
                })
                .await;

            Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
                success: result.is_ok(),
                error: result.err().map(|e| e.to_string()),
            }))
        }

        ClientRpcRequest::TriggerSnapshot => {
            let result = ctx.controller.trigger_snapshot().await;

            match result {
                Ok(snapshot) => Ok(ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
                    success: true,
                    snapshot_index: snapshot.as_ref().map(|log_id| log_id.index),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SnapshotResult(SnapshotResultResponse {
                    success: false,
                    snapshot_index: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::AddLearner { node_id, addr } => {
            use crate::api::ClusterNode;
            use iroh::EndpointAddr;
            use std::str::FromStr;

            // Parse the address as either JSON EndpointAddr or bare EndpointId
            let iroh_addr = if addr.starts_with('{') {
                serde_json::from_str::<EndpointAddr>(&addr)
                    .map_err(|e| format!("invalid JSON EndpointAddr: {e}"))
            } else {
                iroh::EndpointId::from_str(&addr)
                    .map(EndpointAddr::new)
                    .map_err(|e| format!("invalid EndpointId: {e}"))
            };

            let result = match iroh_addr {
                Ok(iroh_addr) => {
                    ctx.controller
                        .add_learner(AddLearnerRequest {
                            learner: ClusterNode::with_iroh_addr(node_id, iroh_addr),
                        })
                        .await
                }
                Err(parse_err) => {
                    Err(crate::api::ControlPlaneError::InvalidRequest { reason: parse_err })
                }
            };

            Ok(ClientRpcResponse::AddLearnerResult(
                AddLearnerResultResponse {
                    success: result.is_ok(),
                    error: result.err().map(|e| e.to_string()),
                },
            ))
        }

        ClientRpcRequest::ChangeMembership { members } => {
            let result = ctx
                .controller
                .change_membership(ChangeMembershipRequest { members })
                .await;

            Ok(ClientRpcResponse::ChangeMembershipResult(
                ChangeMembershipResultResponse {
                    success: result.is_ok(),
                    error: result.err().map(|e| e.to_string()),
                },
            ))
        }

        ClientRpcRequest::Ping => Ok(ClientRpcResponse::Pong),

        ClientRpcRequest::GetClusterState => {
            // Get current cluster state from the Raft controller
            let cluster_state = ctx
                .controller
                .current_state()
                .await
                .map_err(|e| anyhow::anyhow!("failed to get cluster state: {}", e))?;

            // Get current leader from metrics
            let leader_id = ctx.controller.get_leader().await.unwrap_or(None);

            // Convert ClusterNode to NodeDescriptor with membership info
            // Tiger Style: Bounded to MAX_CLUSTER_NODES
            let mut nodes: Vec<NodeDescriptor> = Vec::with_capacity(
                (cluster_state.nodes.len() + cluster_state.learners.len()).min(MAX_CLUSTER_NODES),
            );

            // Track which nodes are voters (members)
            let member_ids: std::collections::HashSet<u64> =
                cluster_state.members.iter().copied().collect();

            // Add all nodes from the cluster state
            for node in cluster_state.nodes.iter().take(MAX_CLUSTER_NODES) {
                let is_voter = member_ids.contains(&node.id);

                // For our own node, use our actual endpoint address; for others use Raft-stored address
                let endpoint_addr = if node.id == ctx.node_id {
                    let our_addr = format!("{:?}", ctx.endpoint_manager.node_addr());
                    debug!(
                        node_id = node.id,
                        addr = %our_addr,
                        "using our own Iroh endpoint address"
                    );
                    our_addr
                } else {
                    debug!(
                        node_id = node.id,
                        addr = %node.addr,
                        "using Raft-stored address"
                    );
                    node.addr.clone()
                };

                nodes.push(NodeDescriptor {
                    node_id: node.id,
                    endpoint_addr,
                    is_voter,
                    is_learner: false,
                    is_leader: leader_id == Some(node.id),
                });
            }

            // Add learners
            for learner in cluster_state.learners.iter() {
                if nodes.len() >= MAX_CLUSTER_NODES {
                    break;
                }

                nodes.push(NodeDescriptor {
                    node_id: learner.id,
                    endpoint_addr: learner.addr.clone(),
                    is_voter: false,
                    is_learner: true,
                    is_leader: false, // Learners can't be leaders
                });
            }

            Ok(ClientRpcResponse::ClusterState(ClusterStateResponse {
                nodes,
                leader_id,
                this_node_id: ctx.node_id,
            }))
        }

        // =========================================================================
        // New operations (migrated from HTTP API)
        // =========================================================================
        ClientRpcRequest::DeleteKey { key } => {
            use crate::api::WriteCommand;

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::Delete { key: key.clone() },
                })
                .await;

            Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
                key,
                deleted: result.is_ok(),
                error: result.err().map(|e| e.to_string()),
            }))
        }

        ClientRpcRequest::ScanKeys {
            prefix,
            limit,
            continuation_token,
        } => {
            use crate::api::ScanRequest;

            let result = ctx
                .kv_store
                .scan(ScanRequest {
                    prefix: prefix.clone(),
                    limit,
                    continuation_token,
                })
                .await;

            match result {
                Ok(scan_resp) => {
                    // Convert from api::ScanEntry to client_rpc::ScanEntry
                    let entries: Vec<ScanEntry> = scan_resp
                        .entries
                        .into_iter()
                        .map(|e| ScanEntry {
                            key: e.key,
                            value: e.value,
                        })
                        .collect();

                    Ok(ClientRpcResponse::ScanResult(ScanResultResponse {
                        entries,
                        count: scan_resp.count,
                        is_truncated: scan_resp.is_truncated,
                        continuation_token: scan_resp.continuation_token,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::ScanResult(ScanResultResponse {
                    entries: vec![],
                    count: 0,
                    is_truncated: false,
                    continuation_token: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::GetMetrics => {
            // TODO: Phase 3 - Implement full Prometheus metrics collection
            // For now, return basic metrics from Raft
            let metrics_text = match ctx.controller.get_metrics().await {
                Ok(metrics) => {
                    format!(
                        "# HELP aspen_raft_term Current Raft term\n\
                         # TYPE aspen_raft_term gauge\n\
                         aspen_raft_term{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_last_log_index Last log index\n\
                         # TYPE aspen_raft_last_log_index gauge\n\
                         aspen_raft_last_log_index{{node_id=\"{}\"}} {}\n",
                        ctx.node_id,
                        metrics.current_term,
                        ctx.node_id,
                        metrics.last_log_index.unwrap_or(0)
                    )
                }
                Err(_) => String::new(),
            };

            Ok(ClientRpcResponse::Metrics(MetricsResponse {
                prometheus_text: metrics_text,
            }))
        }

        ClientRpcRequest::PromoteLearner {
            learner_id,
            replace_node,
            force: _force,
        } => {
            // Get current membership to build new voter set
            let cluster_state = ctx
                .controller
                .current_state()
                .await
                .map_err(|e| anyhow::anyhow!("failed to get cluster state: {}", e))?;

            let previous_voters: Vec<u64> = cluster_state.members.clone();

            // Build new membership: add learner, optionally remove replaced node
            let mut new_members: Vec<u64> = previous_voters.clone();
            if !new_members.contains(&learner_id) {
                new_members.push(learner_id);
            }
            if let Some(replace_id) = replace_node {
                new_members.retain(|&id| id != replace_id);
            }

            let result = ctx
                .controller
                .change_membership(ChangeMembershipRequest {
                    members: new_members.clone(),
                })
                .await;

            Ok(ClientRpcResponse::PromoteLearnerResult(
                PromoteLearnerResultResponse {
                    success: result.is_ok(),
                    learner_id,
                    previous_voters,
                    new_voters: new_members,
                    message: if result.is_ok() {
                        format!("Learner {} promoted to voter", learner_id)
                    } else {
                        "Promotion failed".to_string()
                    },
                    error: result.err().map(|e| e.to_string()),
                },
            ))
        }

        ClientRpcRequest::CheckpointWal => {
            // TODO: Phase 3 - Implement WAL checkpoint via state machine
            // For now, return a placeholder response
            Ok(ClientRpcResponse::CheckpointWalResult(
                CheckpointWalResultResponse {
                    success: false,
                    pages_checkpointed: None,
                    wal_size_before_bytes: None,
                    wal_size_after_bytes: None,
                    error: Some("WAL checkpoint not yet implemented via Client RPC".to_string()),
                },
            ))
        }

        ClientRpcRequest::ListVaults => {
            // TODO: Phase 3 - Implement vault listing via state machine
            // Vaults are key prefixes ending in ':'
            // For now, return empty list
            Ok(ClientRpcResponse::VaultList(VaultListResponse {
                vaults: vec![],
                error: Some("Vault listing not yet implemented via Client RPC".to_string()),
            }))
        }

        ClientRpcRequest::GetVaultKeys { vault_name } => {
            // TODO: Phase 3 - Implement vault key listing via state machine
            // For now, return empty response
            Ok(ClientRpcResponse::VaultKeys(VaultKeysResponse {
                vault: vault_name,
                keys: vec![],
                error: Some("Vault keys not yet implemented via Client RPC".to_string()),
            }))
        }

        ClientRpcRequest::AddPeer {
            node_id,
            endpoint_addr,
        } => {
            // Parse the endpoint address (JSON-serialized EndpointAddr)
            let parsed_addr: iroh::EndpointAddr = match serde_json::from_str(&endpoint_addr) {
                Ok(addr) => addr,
                Err(e) => {
                    debug!(
                        node_id = node_id,
                        endpoint_addr = %endpoint_addr,
                        error = %e,
                        "AddPeer: failed to parse endpoint_addr as JSON EndpointAddr"
                    );
                    return Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                        success: false,
                        error: Some(format!(
                            "invalid endpoint_addr: expected JSON-serialized EndpointAddr: {}",
                            e
                        )),
                    }));
                }
            };

            // Check if network factory is available
            let network_factory = match &ctx.network_factory {
                Some(nf) => nf,
                None => {
                    warn!(
                        node_id = node_id,
                        "AddPeer: network_factory not available in context"
                    );
                    return Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                        success: false,
                        error: Some("network_factory not configured for this node".to_string()),
                    }));
                }
            };

            // Add peer to the network factory
            // Tiger Style: add_peer is bounded by MAX_PEERS (1000)
            network_factory
                .add_peer(crate::raft::types::NodeId(node_id), parsed_addr.clone())
                .await;

            info!(
                node_id = node_id,
                endpoint_id = %parsed_addr.id,
                "AddPeer: successfully registered peer in network factory"
            );

            Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                success: true,
                error: None,
            }))
        }

        ClientRpcRequest::GetClusterTicketCombined { endpoint_ids } => {
            use crate::cluster::ticket::AspenClusterTicket;
            use iroh_gossip::proto::TopicId;

            let hash = blake3::hash(ctx.cluster_cookie.as_bytes());
            let topic_id = TopicId::from_bytes(*hash.as_bytes());

            // Start with this node as the first bootstrap peer
            let mut ticket = AspenClusterTicket::with_bootstrap(
                topic_id,
                ctx.cluster_cookie.clone(),
                ctx.endpoint_manager.endpoint().id(),
            );

            // Collect additional peers from:
            // 1. Explicit endpoint_ids parameter (comma-separated EndpointId strings)
            // 2. Cluster state (iroh_addr from known cluster nodes)
            let mut added_peers = 1u32; // Already added this node

            // Parse explicit endpoint_ids if provided
            if let Some(ids_str) = &endpoint_ids {
                for id_str in ids_str
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
                    // Skip if we've hit the limit (Tiger Style: MAX_BOOTSTRAP_PEERS = 16)
                    if added_peers >= AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
                        debug!(
                            max_peers = AspenClusterTicket::MAX_BOOTSTRAP_PEERS,
                            "GetClusterTicketCombined: reached max bootstrap peers, skipping remaining"
                        );
                        break;
                    }

                    // Try to parse as EndpointId (public key)
                    match id_str.parse::<iroh::EndpointId>() {
                        Ok(endpoint_id) => {
                            // Skip if it's our own endpoint
                            if endpoint_id == ctx.endpoint_manager.endpoint().id() {
                                continue;
                            }
                            if ticket.add_bootstrap(endpoint_id).is_ok() {
                                added_peers += 1;
                            }
                        }
                        Err(e) => {
                            debug!(
                                endpoint_id = id_str,
                                error = %e,
                                "GetClusterTicketCombined: failed to parse endpoint_id, skipping"
                            );
                        }
                    }
                }
            }

            // If we haven't hit the limit, also add peers from cluster state
            if added_peers < AspenClusterTicket::MAX_BOOTSTRAP_PEERS
                && let Ok(cluster_state) = ctx.controller.current_state().await
            {
                // Add endpoint IDs from nodes with known iroh_addr
                for node in cluster_state
                    .nodes
                    .iter()
                    .chain(cluster_state.learners.iter())
                {
                    if added_peers >= AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
                        break;
                    }

                    if let Some(iroh_addr) = &node.iroh_addr {
                        // Skip our own endpoint
                        if iroh_addr.id == ctx.endpoint_manager.endpoint().id() {
                            continue;
                        }
                        if ticket.add_bootstrap(iroh_addr.id).is_ok() {
                            added_peers += 1;
                        }
                    }
                }
            }

            let ticket_str = ticket.serialize();

            debug!(
                bootstrap_peers = added_peers,
                "GetClusterTicketCombined: generated ticket with multiple bootstrap peers"
            );

            Ok(ClientRpcResponse::ClusterTicket(ClusterTicketResponse {
                ticket: ticket_str,
                topic_id: format!("{:?}", topic_id),
                cluster_id: ctx.cluster_cookie.clone(),
                endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
                bootstrap_peers: Some(added_peers as usize),
            }))
        }

        ClientRpcRequest::GetClientTicket { access, priority } => {
            use crate::client::AccessLevel;
            use crate::client::ticket::AspenClientTicket;

            let endpoint_addr = ctx.endpoint_manager.node_addr().clone();
            let access_level = match access.to_lowercase().as_str() {
                "write" | "readwrite" | "read_write" | "rw" => AccessLevel::ReadWrite,
                _ => AccessLevel::ReadOnly,
            };

            // Tiger Style: saturate priority to u8 range
            let priority_u8 = priority.min(255) as u8;

            let ticket = AspenClientTicket::new(&ctx.cluster_cookie, vec![endpoint_addr])
                .with_access(access_level)
                .with_priority(priority_u8);

            let ticket_str = ticket.serialize();

            Ok(ClientRpcResponse::ClientTicket(ClientTicketRpcResponse {
                ticket: ticket_str,
                cluster_id: ctx.cluster_cookie.clone(),
                access: match access_level {
                    AccessLevel::ReadOnly => "read".to_string(),
                    AccessLevel::ReadWrite => "write".to_string(),
                },
                priority,
                endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
                error: None,
            }))
        }

        ClientRpcRequest::GetDocsTicket {
            read_write,
            priority,
        } => {
            use crate::docs::ticket::AspenDocsTicket;

            let endpoint_addr = ctx.endpoint_manager.node_addr().clone();

            // Derive namespace ID from cluster cookie
            let namespace_hash =
                blake3::hash(format!("aspen-docs-{}", ctx.cluster_cookie).as_bytes());
            let namespace_id_str = format!("{}", namespace_hash);

            let ticket = AspenDocsTicket::new(
                ctx.cluster_cookie.clone(),
                priority,
                namespace_id_str.clone(),
                vec![endpoint_addr],
                read_write,
            );

            let ticket_str = ticket.serialize();

            Ok(ClientRpcResponse::DocsTicket(DocsTicketRpcResponse {
                ticket: ticket_str,
                cluster_id: ctx.cluster_cookie.clone(),
                namespace_id: namespace_id_str,
                read_write,
                priority,
                endpoint_id: ctx.endpoint_manager.endpoint().id().to_string(),
                error: None,
            }))
        }

        // =========================================================================
        // Blob operations
        // =========================================================================
        ClientRpcRequest::AddBlob { data, tag } => {
            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
                    success: false,
                    hash: None,
                    size: None,
                    was_new: None,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            match blob_store.add_bytes(&data).await {
                Ok(result) => {
                    // Apply tag if provided
                    if let Some(tag_name) = tag {
                        let tag_name = IrohBlobStore::user_tag(&tag_name);
                        if let Err(e) = blob_store.protect(&result.blob_ref.hash, &tag_name).await {
                            warn!(error = %e, "failed to apply tag to blob");
                        }
                    }

                    Ok(ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
                        success: true,
                        hash: Some(result.blob_ref.hash.to_string()),
                        size: Some(result.blob_ref.size),
                        was_new: Some(result.was_new),
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
                    success: false,
                    hash: None,
                    size: None,
                    was_new: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::GetBlob { hash } => {
            use crate::blob::BlobStore;
            use iroh_blobs::Hash;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                    found: false,
                    data: None,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(e) => {
                    return Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                        found: false,
                        data: None,
                        error: Some(format!("invalid hash: {}", e)),
                    }));
                }
            };

            match blob_store.get_bytes(&hash).await {
                Ok(Some(data)) => Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                    found: true,
                    data: Some(data.to_vec()),
                    error: None,
                })),
                Ok(None) => Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                    found: false,
                    data: None,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                    found: false,
                    data: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::HasBlob { hash } => {
            use crate::blob::BlobStore;
            use iroh_blobs::Hash;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                    exists: false,
                    error: Some("blob store not enabled".to_string()),
                }));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(e) => {
                    return Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                        exists: false,
                        error: Some(format!("invalid hash: {}", e)),
                    }));
                }
            };

            match blob_store.has(&hash).await {
                Ok(exists) => Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                    exists,
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                    exists: false,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::GetBlobTicket { hash } => {
            use crate::blob::BlobStore;
            use iroh_blobs::Hash;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::GetBlobTicketResult(
                    GetBlobTicketResultResponse {
                        success: false,
                        ticket: None,
                        error: Some("blob store not enabled".to_string()),
                    },
                ));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(e) => {
                    return Ok(ClientRpcResponse::GetBlobTicketResult(
                        GetBlobTicketResultResponse {
                            success: false,
                            ticket: None,
                            error: Some(format!("invalid hash: {}", e)),
                        },
                    ));
                }
            };

            match blob_store.ticket(&hash).await {
                Ok(ticket) => Ok(ClientRpcResponse::GetBlobTicketResult(
                    GetBlobTicketResultResponse {
                        success: true,
                        ticket: Some(ticket.to_string()),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::GetBlobTicketResult(
                    GetBlobTicketResultResponse {
                        success: false,
                        ticket: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::ListBlobs {
            limit,
            continuation_token,
        } => {
            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::ListBlobsResult(
                    ListBlobsResultResponse {
                        blobs: vec![],
                        count: 0,
                        has_more: false,
                        continuation_token: None,
                        error: Some("blob store not enabled".to_string()),
                    },
                ));
            };

            // Tiger Style: Cap limit to prevent unbounded responses
            let limit = limit.min(1000);

            match blob_store.list(limit, continuation_token.as_deref()).await {
                Ok(result) => {
                    let count = result.blobs.len() as u32;
                    let blobs = result
                        .blobs
                        .into_iter()
                        .map(|entry| RpcBlobListEntry {
                            hash: entry.hash.to_string(),
                            size: entry.size,
                        })
                        .collect();

                    Ok(ClientRpcResponse::ListBlobsResult(
                        ListBlobsResultResponse {
                            blobs,
                            count,
                            has_more: result.continuation_token.is_some(),
                            continuation_token: result.continuation_token,
                            error: None,
                        },
                    ))
                }
                Err(e) => Ok(ClientRpcResponse::ListBlobsResult(
                    ListBlobsResultResponse {
                        blobs: vec![],
                        count: 0,
                        has_more: false,
                        continuation_token: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::ProtectBlob { hash, tag } => {
            use crate::blob::BlobStore;
            use iroh_blobs::Hash;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::ProtectBlobResult(
                    ProtectBlobResultResponse {
                        success: false,
                        error: Some("blob store not enabled".to_string()),
                    },
                ));
            };

            // Parse hash from string
            let hash = match hash.parse::<Hash>() {
                Ok(h) => h,
                Err(e) => {
                    return Ok(ClientRpcResponse::ProtectBlobResult(
                        ProtectBlobResultResponse {
                            success: false,
                            error: Some(format!("invalid hash: {}", e)),
                        },
                    ));
                }
            };

            let tag_name = IrohBlobStore::user_tag(&tag);
            match blob_store.protect(&hash, &tag_name).await {
                Ok(()) => Ok(ClientRpcResponse::ProtectBlobResult(
                    ProtectBlobResultResponse {
                        success: true,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::ProtectBlobResult(
                    ProtectBlobResultResponse {
                        success: false,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::UnprotectBlob { tag } => {
            use crate::blob::BlobStore;

            let Some(ref blob_store) = ctx.blob_store else {
                return Ok(ClientRpcResponse::UnprotectBlobResult(
                    UnprotectBlobResultResponse {
                        success: false,
                        error: Some("blob store not enabled".to_string()),
                    },
                ));
            };

            let tag_name = IrohBlobStore::user_tag(&tag);
            match blob_store.unprotect(&tag_name).await {
                Ok(()) => Ok(ClientRpcResponse::UnprotectBlobResult(
                    UnprotectBlobResultResponse {
                        success: true,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::UnprotectBlobResult(
                    UnprotectBlobResultResponse {
                        success: false,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        // =========================================================================
        // Peer cluster operations (cluster-to-cluster sync)
        // =========================================================================
        ClientRpcRequest::AddPeerCluster { ticket } => {
            use crate::client_rpc::AddPeerClusterResultResponse;
            use crate::docs::ticket::AspenDocsTicket;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::AddPeerClusterResult(
                    AddPeerClusterResultResponse {
                        success: false,
                        cluster_id: None,
                        priority: None,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            // Parse the ticket
            let docs_ticket = match AspenDocsTicket::deserialize(&ticket) {
                Ok(t) => t,
                Err(e) => {
                    return Ok(ClientRpcResponse::AddPeerClusterResult(
                        AddPeerClusterResultResponse {
                            success: false,
                            cluster_id: None,
                            priority: None,
                            error: Some(format!("invalid ticket: {}", e)),
                        },
                    ));
                }
            };

            let cluster_id = docs_ticket.cluster_id.clone();
            let priority = docs_ticket.priority as u32;

            match peer_manager.add_peer(docs_ticket).await {
                Ok(()) => Ok(ClientRpcResponse::AddPeerClusterResult(
                    AddPeerClusterResultResponse {
                        success: true,
                        cluster_id: Some(cluster_id),
                        priority: Some(priority),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::AddPeerClusterResult(
                    AddPeerClusterResultResponse {
                        success: false,
                        cluster_id: Some(cluster_id),
                        priority: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::RemovePeerCluster { cluster_id } => {
            use crate::client_rpc::RemovePeerClusterResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::RemovePeerClusterResult(
                    RemovePeerClusterResultResponse {
                        success: false,
                        cluster_id: cluster_id.clone(),
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            match peer_manager.remove_peer(&cluster_id).await {
                Ok(()) => Ok(ClientRpcResponse::RemovePeerClusterResult(
                    RemovePeerClusterResultResponse {
                        success: true,
                        cluster_id,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RemovePeerClusterResult(
                    RemovePeerClusterResultResponse {
                        success: false,
                        cluster_id,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::ListPeerClusters => {
            use crate::client_rpc::{ListPeerClustersResultResponse, PeerClusterInfo};

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::ListPeerClustersResult(
                    ListPeerClustersResultResponse {
                        peers: vec![],
                        count: 0,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            let peers = peer_manager.list_peers().await;
            let count = peers.len() as u32;
            let peer_infos: Vec<PeerClusterInfo> = peers
                .into_iter()
                .map(|p| PeerClusterInfo {
                    cluster_id: p.cluster_id,
                    name: p.name,
                    state: format!("{:?}", p.state),
                    priority: p.priority,
                    enabled: p.enabled,
                    sync_count: p.sync_count,
                    failure_count: p.failure_count,
                })
                .collect();

            Ok(ClientRpcResponse::ListPeerClustersResult(
                ListPeerClustersResultResponse {
                    peers: peer_infos,
                    count,
                    error: None,
                },
            ))
        }

        ClientRpcRequest::GetPeerClusterStatus { cluster_id } => {
            use crate::client_rpc::PeerClusterStatusResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::PeerClusterStatus(
                    PeerClusterStatusResponse {
                        found: false,
                        cluster_id: cluster_id.clone(),
                        state: "unknown".to_string(),
                        syncing: false,
                        entries_received: 0,
                        entries_imported: 0,
                        entries_skipped: 0,
                        entries_filtered: 0,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            match peer_manager.sync_status(&cluster_id).await {
                Some(status) => Ok(ClientRpcResponse::PeerClusterStatus(
                    PeerClusterStatusResponse {
                        found: true,
                        cluster_id: status.cluster_id,
                        state: format!("{:?}", status.state),
                        syncing: status.syncing,
                        entries_received: status.entries_received,
                        entries_imported: status.entries_imported,
                        entries_skipped: status.entries_skipped,
                        entries_filtered: status.entries_filtered,
                        error: None,
                    },
                )),
                None => Ok(ClientRpcResponse::PeerClusterStatus(
                    PeerClusterStatusResponse {
                        found: false,
                        cluster_id,
                        state: "unknown".to_string(),
                        syncing: false,
                        entries_received: 0,
                        entries_imported: 0,
                        entries_skipped: 0,
                        entries_filtered: 0,
                        error: None,
                    },
                )),
            }
        }

        ClientRpcRequest::UpdatePeerClusterFilter {
            cluster_id,
            filter_type,
            prefixes,
        } => {
            use crate::client::SubscriptionFilter;
            use crate::client_rpc::UpdatePeerClusterFilterResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(
                    UpdatePeerClusterFilterResultResponse {
                        success: false,
                        cluster_id: cluster_id.clone(),
                        filter_type: None,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            // Parse filter type and prefixes
            let filter = match filter_type.to_lowercase().as_str() {
                "full" | "fullreplication" => SubscriptionFilter::FullReplication,
                "include" | "prefixfilter" => {
                    let prefix_list: Vec<String> = prefixes
                        .as_ref()
                        .map(|p| serde_json::from_str(p).unwrap_or_default())
                        .unwrap_or_default();
                    SubscriptionFilter::PrefixFilter(prefix_list)
                }
                "exclude" | "prefixexclude" => {
                    let prefix_list: Vec<String> = prefixes
                        .as_ref()
                        .map(|p| serde_json::from_str(p).unwrap_or_default())
                        .unwrap_or_default();
                    SubscriptionFilter::PrefixExclude(prefix_list)
                }
                other => {
                    return Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(
                        UpdatePeerClusterFilterResultResponse {
                            success: false,
                            cluster_id,
                            filter_type: None,
                            error: Some(format!("invalid filter type: {}", other)),
                        },
                    ));
                }
            };

            match peer_manager
                .importer()
                .update_filter(&cluster_id, filter.clone())
                .await
            {
                Ok(()) => Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(
                    UpdatePeerClusterFilterResultResponse {
                        success: true,
                        cluster_id,
                        filter_type: Some(filter_type),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(
                    UpdatePeerClusterFilterResultResponse {
                        success: false,
                        cluster_id,
                        filter_type: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::UpdatePeerClusterPriority {
            cluster_id,
            priority,
        } => {
            use crate::client_rpc::UpdatePeerClusterPriorityResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(
                    UpdatePeerClusterPriorityResultResponse {
                        success: false,
                        cluster_id: cluster_id.clone(),
                        previous_priority: None,
                        new_priority: None,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            // Get current priority before update
            let previous_priority = peer_manager
                .list_peers()
                .await
                .into_iter()
                .find(|p| p.cluster_id == cluster_id)
                .map(|p| p.priority);

            match peer_manager
                .importer()
                .update_priority(&cluster_id, priority)
                .await
            {
                Ok(()) => Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(
                    UpdatePeerClusterPriorityResultResponse {
                        success: true,
                        cluster_id,
                        previous_priority,
                        new_priority: Some(priority),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(
                    UpdatePeerClusterPriorityResultResponse {
                        success: false,
                        cluster_id,
                        previous_priority,
                        new_priority: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::SetPeerClusterEnabled {
            cluster_id,
            enabled,
        } => {
            use crate::client_rpc::SetPeerClusterEnabledResultResponse;

            let Some(ref peer_manager) = ctx.peer_manager else {
                return Ok(ClientRpcResponse::SetPeerClusterEnabledResult(
                    SetPeerClusterEnabledResultResponse {
                        success: false,
                        cluster_id: cluster_id.clone(),
                        enabled: None,
                        error: Some("peer sync not enabled".to_string()),
                    },
                ));
            };

            match peer_manager
                .importer()
                .set_enabled(&cluster_id, enabled)
                .await
            {
                Ok(()) => Ok(ClientRpcResponse::SetPeerClusterEnabledResult(
                    SetPeerClusterEnabledResultResponse {
                        success: true,
                        cluster_id,
                        enabled: Some(enabled),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::SetPeerClusterEnabledResult(
                    SetPeerClusterEnabledResultResponse {
                        success: false,
                        cluster_id,
                        enabled: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }
    }
}
