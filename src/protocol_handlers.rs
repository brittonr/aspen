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
    ReadRequest, WriteRequest, validate_client_key,
};
use crate::blob::IrohBlobStore;
use crate::client_rpc::{
    AddBlobResultResponse, AddLearnerResultResponse, AddPeerResultResponse,
    BatchReadResultResponse, BatchWriteResultResponse, BlobListEntry as RpcBlobListEntry,
    ChangeMembershipResultResponse, CheckpointWalResultResponse, ClientRpcRequest,
    ClientRpcResponse, ClientTicketResponse as ClientTicketRpcResponse, ClusterStateResponse,
    ClusterTicketResponse, ConditionalBatchWriteResultResponse, CounterResultResponse,
    DeleteResultResponse, DocsTicketResponse as DocsTicketRpcResponse, GetBlobResultResponse,
    GetBlobTicketResultResponse, HasBlobResultResponse, HealthResponse, InitResultResponse,
    ListBlobsResultResponse, LockResultResponse, MAX_CLIENT_MESSAGE_SIZE, MAX_CLUSTER_NODES,
    MetricsResponse, NodeDescriptor, NodeInfoResponse, PromoteLearnerResultResponse,
    ProtectBlobResultResponse, RaftMetricsResponse, RateLimiterResultResponse, ReadResultResponse,
    ScanEntry, ScanResultResponse, SequenceResultResponse, SignedCounterResultResponse,
    SnapshotResultResponse, SqlResultResponse, UnprotectBlobResultResponse, VaultKeysResponse,
    VaultListResponse, WriteResultResponse,
};
use crate::cluster::IrohEndpointManager;
use crate::coordination::{
    AtomicCounter, CounterConfig, DistributedLock, DistributedRateLimiter, LockConfig,
    RateLimiterConfig, SequenceConfig, SequenceGenerator, SignedAtomicCounter,
};
use crate::raft::auth::{
    AUTH_HANDSHAKE_TIMEOUT, AuthContext, AuthResponse, AuthResult, MAX_AUTH_MESSAGE_SIZE,
};
use crate::raft::constants::{
    CLIENT_RPC_BURST, CLIENT_RPC_RATE_LIMIT_PREFIX, CLIENT_RPC_RATE_PER_SECOND,
    CLIENT_RPC_REQUEST_COUNTER, CLIENT_RPC_REQUEST_ID_SEQUENCE, MAX_CONCURRENT_CONNECTIONS,
    MAX_RPC_MESSAGE_SIZE, MAX_STREAMS_PER_CONNECTION,
};
use crate::raft::log_subscriber::{
    EndOfStreamReason, HistoricalLogReader, LOG_BROADCAST_BUFFER_SIZE, LogEntryMessage,
    LogEntryPayload, MAX_HISTORICAL_BATCH_SIZE, MAX_LOG_SUBSCRIBERS, SUBSCRIBE_HANDSHAKE_TIMEOUT,
    SUBSCRIBE_KEEPALIVE_INTERVAL, SubscribeRejectReason, SubscribeRequest, SubscribeResponse,
};
use crate::raft::rpc::{RaftRpcProtocol, RaftRpcResponse};
use crate::raft::types::AppTypeConfig;

// ============================================================================
// Error Sanitization (HIGH-4 Security Enhancement)
// ============================================================================

/// Sanitize an error message for client consumption.
///
/// This function converts internal error messages to user-safe messages that
/// don't leak implementation details, file paths, or internal state.
///
/// # Security Rationale
///
/// Internal errors can reveal:
/// - File paths and system structure
/// - Database schema and query patterns
/// - Network topology and peer information
/// - Internal component names and versions
///
/// By categorizing errors and returning generic messages, we prevent
/// information leakage while still providing useful feedback to clients.
///
/// # Tiger Style
///
/// - Exhaustive pattern matching for known error types
/// - Generic fallback for unknown errors
/// - Full error logged internally at appropriate level
fn sanitize_error_for_client(err: &anyhow::Error) -> String {
    // Check for specific error types we can provide better messages for
    let err_string = err.to_string().to_lowercase();

    // Categorize errors by their root cause
    if err_string.contains("not leader") || err_string.contains("forward to leader") {
        return "operation must be performed on leader node".to_string();
    }

    if err_string.contains("not found") || err_string.contains("key not found") {
        return "resource not found".to_string();
    }

    if err_string.contains("not initialized")
        || err_string.contains("cluster not initialized")
        || err_string.contains("uninitialized")
    {
        return "cluster not initialized".to_string();
    }

    if err_string.contains("timeout") || err_string.contains("timed out") {
        return "operation timed out".to_string();
    }

    if err_string.contains("connection")
        || err_string.contains("network")
        || err_string.contains("unreachable")
    {
        return "network error".to_string();
    }

    if err_string.contains("permission") || err_string.contains("unauthorized") {
        return "permission denied".to_string();
    }

    if err_string.contains("invalid") || err_string.contains("malformed") {
        return "invalid request".to_string();
    }

    if err_string.contains("quorum") || err_string.contains("membership") {
        return "cluster membership error".to_string();
    }

    if err_string.contains("snapshot") {
        return "snapshot operation failed".to_string();
    }

    if err_string.contains("storage")
        || err_string.contains("database")
        || err_string.contains("sqlite")
        || err_string.contains("redb")
    {
        return "storage error".to_string();
    }

    // Generic fallback - never expose raw error messages
    "internal error".to_string()
}

/// Sanitize an error string for client consumption.
///
/// Variant of `sanitize_error_for_client` that works with string errors.
#[allow(dead_code)]
fn sanitize_error_string_for_client(err: &str) -> String {
    let err_lower = err.to_lowercase();

    if err_lower.contains("not leader") || err_lower.contains("forward to leader") {
        return "operation must be performed on leader node".to_string();
    }

    if err_lower.contains("not found") || err_lower.contains("key not found") {
        return "resource not found".to_string();
    }

    if err_lower.contains("not initialized") || err_lower.contains("cluster not initialized") {
        return "cluster not initialized".to_string();
    }

    if err_lower.contains("timeout") || err_lower.contains("timed out") {
        return "operation timed out".to_string();
    }

    if err_lower.contains("connection") || err_lower.contains("network") {
        return "network error".to_string();
    }

    if err_lower.contains("invalid") || err_lower.contains("malformed") {
        return "invalid request".to_string();
    }

    // Generic fallback
    "internal error".to_string()
}

/// Sanitize a ControlPlaneError for client consumption.
///
/// These errors are part of our API and can be returned directly since they
/// are already designed to be user-facing. However, we still sanitize the
/// inner reason strings to be safe.
fn sanitize_control_error(err: &crate::api::ControlPlaneError) -> String {
    use crate::api::ControlPlaneError;
    match err {
        ControlPlaneError::InvalidRequest { .. } => "invalid request".to_string(),
        ControlPlaneError::NotInitialized => "cluster not initialized".to_string(),
        ControlPlaneError::Failed { .. } => "operation failed".to_string(),
        ControlPlaneError::Unsupported { operation, .. } => {
            format!("operation not supported: {}", operation)
        }
    }
}

/// Sanitize a KeyValueStoreError for client consumption.
///
/// Key-value errors are often user-actionable, so we preserve the error category
/// but remove potentially sensitive implementation details.
fn sanitize_kv_error(err: &crate::api::KeyValueStoreError) -> String {
    use crate::api::KeyValueStoreError;
    match err {
        KeyValueStoreError::NotFound { .. } => "key not found".to_string(),
        KeyValueStoreError::Failed { .. } => "operation failed".to_string(),
        KeyValueStoreError::NotLeader { leader, .. } => {
            if let Some(leader_id) = leader {
                format!("not leader; leader is node {}", leader_id)
            } else {
                "not leader; leader unknown".to_string()
            }
        }
        KeyValueStoreError::KeyTooLarge { max, .. } => {
            format!("key too large; max {} bytes", max)
        }
        KeyValueStoreError::ValueTooLarge { max, .. } => {
            format!("value too large; max {} bytes", max)
        }
        KeyValueStoreError::BatchTooLarge { max, .. } => {
            format!("batch too large; max {} keys", max)
        }
        KeyValueStoreError::Timeout { duration_ms } => {
            format!("operation timed out after {}ms", duration_ms)
        }
        KeyValueStoreError::CompareAndSwapFailed { key, .. } => {
            format!("compare-and-swap failed for key '{}'", key)
        }
    }
}

/// Sanitize a blob store error for client consumption.
///
/// Blob store errors can contain file paths, IO errors, and other internal details.
/// We categorize them into user-safe messages.
fn sanitize_blob_error(err: &crate::blob::BlobStoreError) -> String {
    use crate::blob::BlobStoreError;
    match err {
        BlobStoreError::NotFound { .. } => "blob not found".to_string(),
        BlobStoreError::TooLarge { max, .. } => format!("blob too large; max {} bytes", max),
        BlobStoreError::Storage { .. } => "storage error".to_string(),
        BlobStoreError::Download { .. } => "download failed".to_string(),
        BlobStoreError::InvalidTicket { .. } => "invalid ticket".to_string(),
    }
}

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
}

impl AuthenticatedRaftProtocolHandler {
    /// Create a new authenticated Raft protocol handler.
    ///
    /// # Arguments
    /// * `raft_core` - Raft instance to forward RPCs to
    /// * `cluster_cookie` - Shared secret for authentication
    ///
    /// # Security Note
    /// Authentication is handled by HMAC-SHA256 challenge-response using the cluster cookie.
    /// Iroh already provides cryptographic identity verification at the transport layer.
    pub fn new(raft_core: Raft<AppTypeConfig>, cluster_cookie: &str) -> Self {
        Self {
            raft_core,
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS as usize)),
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
/// Uses length-prefixed framing for auth messages so they can share a stream with RPC:
/// - Each auth message is prefixed with a 4-byte big-endian length
/// - RPC message uses read_to_end after auth completes (client finishes send)
///
/// Protocol:
/// 1. Send AuthChallenge (length-prefixed)
/// 2. Receive AuthResponse (length-prefixed)
/// 3. Verify HMAC
/// 4. Send AuthResult (length-prefixed)
/// 5. If authenticated, read RPC (read_to_end) and process
#[instrument(skip(recv, send, raft_core, auth_context))]
async fn handle_authenticated_raft_stream(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    raft_core: Raft<AppTypeConfig>,
    auth_context: AuthContext,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Step 1: Generate and send challenge (length-prefixed)
    let challenge = auth_context.generate_challenge();
    let challenge_bytes =
        postcard::to_stdvec(&challenge).context("failed to serialize challenge")?;
    write_length_prefixed(&mut send, &challenge_bytes)
        .await
        .context("failed to send challenge")?;

    // Step 2: Receive response with timeout (length-prefixed)
    let response_result = tokio::time::timeout(AUTH_HANDSHAKE_TIMEOUT, async {
        let buffer = read_length_prefixed(&mut recv, MAX_AUTH_MESSAGE_SIZE)
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
            // Best-effort send of failure response - log if it fails but don't block error return
            if let Err(write_err) = write_length_prefixed(&mut send, &result_bytes).await {
                debug!(error = %write_err, "failed to send auth failure response to client");
            }
            if let Err(finish_err) = send.finish() {
                debug!(error = %finish_err, "failed to finish stream after auth failure");
            }
            return Err(err);
        }
        Err(_) => {
            warn!("authentication timed out");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)
                .context("failed to serialize auth result")?;
            // Best-effort send of failure response - log if it fails but don't block error return
            if let Err(write_err) = write_length_prefixed(&mut send, &result_bytes).await {
                debug!(error = %write_err, "failed to send auth timeout response to client");
            }
            if let Err(finish_err) = send.finish() {
                debug!(error = %finish_err, "failed to finish stream after auth timeout");
            }
            return Err(anyhow::anyhow!("authentication timeout"));
        }
    };

    // Step 3: Verify the response
    let auth_result = auth_context.verify_response(&challenge, &response);

    // Step 4: Send result (length-prefixed)
    let result_bytes =
        postcard::to_stdvec(&auth_result).context("failed to serialize auth result")?;
    write_length_prefixed(&mut send, &result_bytes)
        .await
        .context("failed to send auth result")?;

    if !auth_result.is_ok() {
        warn!(result = ?auth_result, "authentication failed");
        if let Err(finish_err) = send.finish() {
            debug!(error = %finish_err, "failed to finish stream after auth verification failure");
        }
        return Err(anyhow::anyhow!("authentication failed: {:?}", auth_result));
    }

    debug!("authentication successful, processing Raft RPC");

    // Step 5: Read and process the Raft RPC message (read_to_end - client finishes send)
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
// Length-Prefixed Framing Helpers
// ============================================================================

/// Read a length-prefixed message from a stream.
///
/// Format: 4-byte big-endian length + payload
/// Tiger Style: Bounded by max_size to prevent memory exhaustion.
async fn read_length_prefixed(
    recv: &mut iroh::endpoint::RecvStream,
    max_size: usize,
) -> anyhow::Result<Vec<u8>> {
    use anyhow::Context;

    // Read 4-byte length prefix
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .context("failed to read length prefix")?;
    let len = u32::from_be_bytes(len_buf) as usize;

    // Validate length
    if len > max_size {
        return Err(anyhow::anyhow!("message too large: {} > {}", len, max_size));
    }

    // Read payload
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf)
        .await
        .context("failed to read message payload")?;

    Ok(buf)
}

/// Write a length-prefixed message to a stream.
///
/// Format: 4-byte big-endian length + payload
async fn write_length_prefixed(
    send: &mut iroh::endpoint::SendStream,
    data: &[u8],
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Write 4-byte length prefix
    let len = data.len() as u32;
    send.write_all(&len.to_be_bytes())
        .await
        .context("failed to write length prefix")?;

    // Write payload
    send.write_all(data)
        .await
        .context("failed to write message payload")?;

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
    /// Current committed log index (updated externally).
    committed_index: Arc<AtomicU64>,
    /// Optional historical log reader for replay from start_index.
    historical_reader: Option<Arc<dyn HistoricalLogReader>>,
}

impl LogSubscriberProtocolHandler {
    /// Create a new log subscriber protocol handler.
    ///
    /// # Arguments
    /// * `cluster_cookie` - Shared secret for authentication
    /// * `node_id` - This node's ID
    ///
    /// # Returns
    /// A tuple of (handler, log_sender, committed_index_handle).
    /// - `log_sender`: Use to broadcast log entries to subscribers
    /// - `committed_index_handle`: Update this atomic to reflect current Raft committed index
    ///
    /// # Note
    /// Historical replay is disabled by default. Use `with_historical_reader()` to enable it.
    pub fn new(
        cluster_cookie: &str,
        node_id: u64,
    ) -> (Self, broadcast::Sender<LogEntryPayload>, Arc<AtomicU64>) {
        let (log_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        let committed_index = Arc::new(AtomicU64::new(0));
        let handler = Self {
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender: log_sender.clone(),
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index: committed_index.clone(),
            historical_reader: None,
        };
        (handler, log_sender, committed_index)
    }

    /// Create a handler with an existing broadcast sender and committed index tracker.
    ///
    /// Use this when you need multiple handlers to share the same broadcast channel
    /// and committed index state.
    pub fn with_sender(
        cluster_cookie: &str,
        node_id: u64,
        log_sender: broadcast::Sender<LogEntryPayload>,
        committed_index: Arc<AtomicU64>,
    ) -> Self {
        Self {
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender,
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index,
            historical_reader: None,
        }
    }

    /// Create a handler with historical log replay support.
    ///
    /// When a subscriber connects with a `start_index`, historical entries
    /// from `start_index` to the current committed index will be replayed
    /// before streaming new entries.
    ///
    /// # Arguments
    /// * `cluster_cookie` - Shared secret for authentication
    /// * `node_id` - This node's ID
    /// * `log_sender` - Broadcast channel for new entries
    /// * `committed_index` - Atomic counter for current committed index
    /// * `historical_reader` - Reader for fetching historical log entries
    pub fn with_historical_reader(
        cluster_cookie: &str,
        node_id: u64,
        log_sender: broadcast::Sender<LogEntryPayload>,
        committed_index: Arc<AtomicU64>,
        historical_reader: Arc<dyn HistoricalLogReader>,
    ) -> Self {
        Self {
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender,
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index,
            historical_reader: Some(historical_reader),
        }
    }

    /// Get a handle to the committed index for external updates.
    ///
    /// Call `committed_index_handle.store(new_index, Ordering::Release)` when
    /// the Raft committed index changes.
    pub fn committed_index_handle(&self) -> Arc<AtomicU64> {
        self.committed_index.clone()
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
            self.committed_index.clone(),
            self.historical_reader.clone(),
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
#[instrument(skip(
    connection,
    auth_context,
    log_receiver,
    committed_index,
    historical_reader
))]
async fn handle_log_subscriber_connection(
    connection: Connection,
    auth_context: AuthContext,
    mut log_receiver: broadcast::Receiver<LogEntryPayload>,
    node_id: u64,
    subscriber_id: u64,
    committed_index: Arc<AtomicU64>,
    historical_reader: Option<Arc<dyn HistoricalLogReader>>,
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
            // Best-effort send of failure response - log if it fails
            if let Err(write_err) = send.write_all(&result_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send auth failure to subscriber");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth failure");
            }
            return Err(err);
        }
        Err(_) => {
            warn!(subscriber_id = subscriber_id, "subscriber auth timed out");
            let result_bytes = postcard::to_stdvec(&AuthResult::Failed)?;
            // Best-effort send of failure response - log if it fails
            if let Err(write_err) = send.write_all(&result_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send auth timeout to subscriber");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth timeout");
            }
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
        if let Err(finish_err) = send.finish() {
            debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth verification failure");
        }
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
            // Best-effort send of rejection - log if it fails
            if let Err(write_err) = send.write_all(&response_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send subscribe rejection");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscribe error");
            }
            return Err(err);
        }
        Err(_) => {
            let response = SubscribeResponse::Rejected {
                reason: SubscribeRejectReason::InternalError,
            };
            let response_bytes = postcard::to_stdvec(&response)?;
            // Best-effort send of rejection - log if it fails
            if let Err(write_err) = send.write_all(&response_bytes).await {
                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send subscribe timeout rejection");
            }
            if let Err(finish_err) = send.finish() {
                debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscribe timeout");
            }
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
    let current_committed_index = committed_index.load(Ordering::Acquire);
    let response = SubscribeResponse::Accepted {
        current_index: current_committed_index,
        node_id,
    };
    let response_bytes = postcard::to_stdvec(&response)?;
    send.write_all(&response_bytes)
        .await
        .context("failed to send subscribe response")?;

    info!(
        subscriber_id = subscriber_id,
        remote = %remote_node_id,
        start_index = sub_request.start_index,
        current_index = current_committed_index,
        "log subscription active"
    );

    // Step 6b: Historical replay if requested and available
    let replay_end_index = if sub_request.start_index < current_committed_index
        && sub_request.start_index != u64::MAX
    {
        if let Some(ref reader) = historical_reader {
            debug!(
                subscriber_id = subscriber_id,
                start_index = sub_request.start_index,
                end_index = current_committed_index,
                "starting historical replay"
            );

            // Replay in batches to avoid memory exhaustion
            let mut current_start = sub_request.start_index;
            let mut total_replayed = 0u64;

            while current_start <= current_committed_index {
                let batch_end = std::cmp::min(
                    current_start.saturating_add(MAX_HISTORICAL_BATCH_SIZE as u64 - 1),
                    current_committed_index,
                );

                match reader.read_entries(current_start, batch_end).await {
                    Ok(entries) => {
                        if entries.is_empty() {
                            // No more entries available (may have been compacted)
                            debug!(
                                subscriber_id = subscriber_id,
                                start = current_start,
                                "no historical entries available, may have been compacted"
                            );
                            break;
                        }

                        for entry in entries {
                            // Apply prefix filter
                            if !sub_request.key_prefix.is_empty()
                                && !entry.operation.matches_prefix(&sub_request.key_prefix)
                            {
                                continue;
                            }

                            let message = LogEntryMessage::Entry(entry.clone());
                            let message_bytes = match postcard::to_stdvec(&message) {
                                Ok(bytes) => bytes,
                                Err(err) => {
                                    error!(error = %err, "failed to serialize historical entry");
                                    continue;
                                }
                            };

                            if let Err(err) = send.write_all(&message_bytes).await {
                                debug!(
                                    subscriber_id = subscriber_id,
                                    error = %err,
                                    "subscriber disconnected during replay"
                                );
                                return Err(anyhow::anyhow!(
                                    "subscriber disconnected during replay"
                                ));
                            }

                            total_replayed += 1;
                            current_start = entry.index + 1;
                        }
                    }
                    Err(err) => {
                        warn!(
                            subscriber_id = subscriber_id,
                            error = %err,
                            "failed to read historical entries, continuing with live stream"
                        );
                        break;
                    }
                }

                // Check if we've finished
                if current_start > current_committed_index {
                    break;
                }
            }

            info!(
                subscriber_id = subscriber_id,
                total_replayed = total_replayed,
                "historical replay complete"
            );
            current_start
        } else {
            debug!(
                subscriber_id = subscriber_id,
                "historical replay requested but no reader available"
            );
            current_committed_index
        }
    } else {
        current_committed_index
    };

    // Update the log receiver to skip entries we've already sent
    // (entries between replay_end_index and any new entries that arrived during replay)
    let _ = replay_end_index; // Used for logging context

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
                            // Best-effort send of end-of-stream message
                            if let Err(write_err) = send.write_all(&bytes).await {
                                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send lagged end-of-stream");
                            }
                        }
                        break;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!(subscriber_id = subscriber_id, "log broadcast channel closed");
                        let end_message = LogEntryMessage::EndOfStream {
                            reason: EndOfStreamReason::ServerShutdown,
                        };
                        if let Ok(bytes) = postcard::to_stdvec(&end_message) {
                            // Best-effort send of end-of-stream message
                            if let Err(write_err) = send.write_all(&bytes).await {
                                debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send shutdown end-of-stream");
                            }
                        }
                        break;
                    }
                }
            }

            // Send keepalive on idle
            _ = keepalive_interval.tick() => {
                let keepalive = LogEntryMessage::Keepalive {
                    committed_index: committed_index.load(Ordering::Acquire),
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

    // Best-effort stream finish - log if it fails
    if let Err(finish_err) = send.finish() {
        debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish log subscription stream");
    }

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
    /// SQL query executor for read-only SQL queries.
    pub sql_executor: Arc<dyn crate::api::SqlQueryExecutor>,
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
        let remote_id = remote_node_id.to_string();
        let (send, recv) = stream;
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = handle_client_request((recv, send), ctx_clone, &remote_id).await {
                error!(error = %err, "failed to handle Client request");
            }
            active_streams_clone.fetch_sub(1, Ordering::Relaxed);
        });
    }

    Ok(())
}

/// Handle a single Client RPC request on a stream.
#[instrument(skip(recv, send, ctx, client_id), fields(request_id))]
async fn handle_client_request(
    (mut recv, mut send): (iroh::endpoint::RecvStream, iroh::endpoint::SendStream),
    ctx: Arc<ClientProtocolContext>,
    client_id: &str,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Generate unique request ID using distributed sequence
    // Tiger Style: Monotonic IDs enable cluster-wide request tracing
    let request_id = {
        let seq = SequenceGenerator::new(
            ctx.kv_store.clone(),
            CLIENT_RPC_REQUEST_ID_SEQUENCE,
            SequenceConfig::default(),
        );
        seq.next().await.unwrap_or(0)
    };

    // Record request_id in current span for distributed tracing
    tracing::Span::current().record("request_id", request_id);

    // Read the request with size limit
    let buffer = recv
        .read_to_end(MAX_CLIENT_MESSAGE_SIZE)
        .await
        .context("failed to read Client request")?;

    // Deserialize the request
    let request: ClientRpcRequest =
        postcard::from_bytes(&buffer).context("failed to deserialize Client request")?;

    debug!(request_type = ?request, client_id = %client_id, request_id = %request_id, "received Client request");

    // Rate limit check using distributed rate limiter
    // Tiger Style: Per-client rate limiting prevents DoS from individual clients
    let rate_limit_key = format!("{}{}", CLIENT_RPC_RATE_LIMIT_PREFIX, client_id);
    let limiter = DistributedRateLimiter::new(
        ctx.kv_store.clone(),
        &rate_limit_key,
        RateLimiterConfig::new(CLIENT_RPC_RATE_PER_SECOND, CLIENT_RPC_BURST),
    );

    if let Err(e) = limiter.try_acquire().await {
        warn!(
            client_id = %client_id,
            retry_after_ms = e.retry_after_ms,
            "Client rate limited"
        );
        let response = ClientRpcResponse::error(
            "RATE_LIMITED",
            format!("Too many requests. Retry after {}ms", e.retry_after_ms),
        );
        let response_bytes =
            postcard::to_stdvec(&response).context("failed to serialize rate limit response")?;
        send.write_all(&response_bytes)
            .await
            .context("failed to write rate limit response")?;
        send.finish().context("failed to finish send stream")?;
        return Ok(());
    }

    // Process the request and create response
    let response = match process_client_request(request, &ctx).await {
        Ok(resp) => resp,
        Err(err) => {
            // Log full error internally for debugging, but return sanitized message to client
            // HIGH-4: Prevent information leakage through error messages
            warn!(error = %err, "Client request processing failed");
            ClientRpcResponse::error("INTERNAL_ERROR", sanitize_error_for_client(&err))
        }
    };

    // Serialize and send response
    let response_bytes =
        postcard::to_stdvec(&response).context("failed to serialize Client response")?;
    send.write_all(&response_bytes)
        .await
        .context("failed to write Client response")?;
    send.finish().context("failed to finish send stream")?;

    // Increment cluster-wide request counter (best-effort, non-blocking)
    // Tiger Style: Fire-and-forget counter increment doesn't block request path
    let kv_store = ctx.kv_store.clone();
    tokio::spawn(async move {
        let counter = AtomicCounter::new(
            kv_store,
            CLIENT_RPC_REQUEST_COUNTER,
            CounterConfig::default(),
        );
        if let Err(e) = counter.increment().await {
            debug!(error = %e, "Failed to increment request counter");
        }
    });

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
                // HIGH-4: Sanitize error messages to prevent information leakage
                error: result.err().map(|e| sanitize_control_error(&e)),
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
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    let (found, error) = match &e {
                        crate::api::KeyValueStoreError::NotFound { .. } => (false, None),
                        other => (false, Some(sanitize_kv_error(other))),
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

            // Validate key against reserved _system: prefix
            if let Err(vault_err) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
                    success: false,
                    error: Some(vault_err.to_string()),
                }));
            }

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: key.clone(),
                        value: String::from_utf8_lossy(&value).to_string(),
                    },
                })
                .await;

            Ok(ClientRpcResponse::WriteResult(WriteResultResponse {
                success: result.is_ok(),
                // HIGH-4: Sanitize error messages to prevent information leakage
                error: result.err().map(|e| sanitize_kv_error(&e)),
            }))
        }

        ClientRpcRequest::CompareAndSwapKey {
            key,
            expected,
            new_value,
        } => {
            use crate::api::WriteCommand;
            use crate::client_rpc::CompareAndSwapResultResponse;

            // Validate key against reserved _system: prefix
            if let Err(vault_err) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: false,
                        actual_value: None,
                        error: Some(vault_err.to_string()),
                    },
                ));
            }

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: key.clone(),
                        expected: expected.map(|v| String::from_utf8_lossy(&v).to_string()),
                        new_value: String::from_utf8_lossy(&new_value).to_string(),
                    },
                })
                .await;

            match result {
                Ok(_) => Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: true,
                        actual_value: None,
                        error: None,
                    },
                )),
                Err(crate::api::KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => Ok(
                    ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                        success: false,
                        actual_value: actual.map(|v| v.into_bytes()),
                        error: None,
                    }),
                ),
                Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: false,
                        actual_value: None,
                        // HIGH-4: Sanitize error messages to prevent information leakage
                        error: Some(sanitize_kv_error(&e)),
                    },
                )),
            }
        }

        ClientRpcRequest::CompareAndDeleteKey { key, expected } => {
            use crate::api::WriteCommand;
            use crate::client_rpc::CompareAndSwapResultResponse;

            // Validate key against reserved _system: prefix
            if let Err(vault_err) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: false,
                        actual_value: None,
                        error: Some(vault_err.to_string()),
                    },
                ));
            }

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndDelete {
                        key: key.clone(),
                        expected: String::from_utf8_lossy(&expected).to_string(),
                    },
                })
                .await;

            match result {
                Ok(_) => Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: true,
                        actual_value: None,
                        error: None,
                    },
                )),
                Err(crate::api::KeyValueStoreError::CompareAndSwapFailed { actual, .. }) => Ok(
                    ClientRpcResponse::CompareAndSwapResult(CompareAndSwapResultResponse {
                        success: false,
                        actual_value: actual.map(|v| v.into_bytes()),
                        error: None,
                    }),
                ),
                Err(e) => Ok(ClientRpcResponse::CompareAndSwapResult(
                    CompareAndSwapResultResponse {
                        success: false,
                        actual_value: None,
                        // HIGH-4: Sanitize error messages to prevent information leakage
                        error: Some(sanitize_kv_error(&e)),
                    },
                )),
            }
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
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: Some(sanitize_control_error(&e)),
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
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: result.err().map(|e| sanitize_control_error(&e)),
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
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: result.err().map(|e| sanitize_control_error(&e)),
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

            // Validate key against reserved _system: prefix
            if let Err(vault_err) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
                    key,
                    deleted: false,
                    error: Some(vault_err.to_string()),
                }));
            }

            let result = ctx
                .kv_store
                .write(WriteRequest {
                    command: WriteCommand::Delete { key: key.clone() },
                })
                .await;

            Ok(ClientRpcResponse::DeleteResult(DeleteResultResponse {
                key,
                deleted: result.is_ok(),
                // HIGH-4: Sanitize error messages to prevent information leakage
                error: result.err().map(|e| sanitize_kv_error(&e)),
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
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: Some(sanitize_kv_error(&e)),
                })),
            }
        }

        ClientRpcRequest::GetMetrics => {
            // Return Prometheus-format metrics from Raft
            let metrics_text = match ctx.controller.get_metrics().await {
                Ok(metrics) => {
                    let state_value = match metrics.state {
                        openraft::ServerState::Learner => 0,
                        openraft::ServerState::Follower => 1,
                        openraft::ServerState::Candidate => 2,
                        openraft::ServerState::Leader => 3,
                        openraft::ServerState::Shutdown => 4,
                    };
                    let is_leader: u8 = if metrics.state == openraft::ServerState::Leader {
                        1
                    } else {
                        0
                    };
                    let last_applied = metrics
                        .last_applied
                        .as_ref()
                        .map(|la| la.index)
                        .unwrap_or(0);
                    let snapshot_index = metrics.snapshot.as_ref().map(|s| s.index).unwrap_or(0);

                    // Get cluster-wide request counter
                    let request_counter = {
                        let counter = AtomicCounter::new(
                            ctx.kv_store.clone(),
                            CLIENT_RPC_REQUEST_COUNTER,
                            CounterConfig::default(),
                        );
                        counter.get().await.unwrap_or(0)
                    };

                    format!(
                        "# HELP aspen_raft_term Current Raft term\n\
                         # TYPE aspen_raft_term gauge\n\
                         aspen_raft_term{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_state Raft state (0=Learner, 1=Follower, 2=Candidate, 3=Leader, 4=Shutdown)\n\
                         # TYPE aspen_raft_state gauge\n\
                         aspen_raft_state{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_is_leader Whether this node is the leader\n\
                         # TYPE aspen_raft_is_leader gauge\n\
                         aspen_raft_is_leader{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_last_log_index Last log index\n\
                         # TYPE aspen_raft_last_log_index gauge\n\
                         aspen_raft_last_log_index{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_last_applied_index Last applied log index\n\
                         # TYPE aspen_raft_last_applied_index gauge\n\
                         aspen_raft_last_applied_index{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_raft_snapshot_index Snapshot index\n\
                         # TYPE aspen_raft_snapshot_index gauge\n\
                         aspen_raft_snapshot_index{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_node_uptime_seconds Node uptime in seconds\n\
                         # TYPE aspen_node_uptime_seconds counter\n\
                         aspen_node_uptime_seconds{{node_id=\"{}\"}} {}\n\
                         # HELP aspen_client_rpc_requests_total Total client RPC requests processed cluster-wide\n\
                         # TYPE aspen_client_rpc_requests_total counter\n\
                         aspen_client_rpc_requests_total{{node_id=\"{}\"}} {}\n",
                        ctx.node_id,
                        metrics.current_term,
                        ctx.node_id,
                        state_value,
                        ctx.node_id,
                        is_leader,
                        ctx.node_id,
                        metrics.last_log_index.unwrap_or(0),
                        ctx.node_id,
                        last_applied,
                        ctx.node_id,
                        snapshot_index,
                        ctx.node_id,
                        ctx.start_time.elapsed().as_secs(),
                        ctx.node_id,
                        request_counter
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
                    // HIGH-4: Sanitize error messages to prevent information leakage
                    error: result.err().map(|e| sanitize_control_error(&e)),
                },
            ))
        }

        ClientRpcRequest::CheckpointWal => {
            // WAL checkpoint requires direct access to the SQLite state machine,
            // which is not exposed through the ClusterController/KeyValueStore traits.
            // This would need to be implemented as a new trait method or by extending
            // the ClientProtocolContext to include the state machine reference.
            Ok(ClientRpcResponse::CheckpointWalResult(
                CheckpointWalResultResponse {
                    success: false,
                    pages_checkpointed: None,
                    wal_size_before_bytes: None,
                    wal_size_after_bytes: None,
                    error: Some("WAL checkpoint requires state machine access - use trigger_snapshot for log compaction".to_string()),
                },
            ))
        }

        ClientRpcRequest::ListVaults => {
            // Deprecated: Vault-specific operations removed in favor of flat keyspace.
            // Use ScanKeys with an empty prefix to list all keys.
            Ok(ClientRpcResponse::VaultList(VaultListResponse {
                vaults: vec![],
                error: Some(
                    "ListVaults is deprecated. Use ScanKeys with a prefix instead.".to_string(),
                ),
            }))
        }

        ClientRpcRequest::GetVaultKeys { vault_name } => {
            // Deprecated: Vault-specific operations removed in favor of flat keyspace.
            // Use ScanKeys with the desired prefix instead.
            Ok(ClientRpcResponse::VaultKeys(VaultKeysResponse {
                vault: vault_name,
                keys: vec![],
                error: Some(
                    "GetVaultKeys is deprecated. Use ScanKeys with a prefix instead.".to_string(),
                ),
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
                    // HIGH-4: Don't expose parse error details to clients
                    return Ok(ClientRpcResponse::AddPeerResult(AddPeerResultResponse {
                        success: false,
                        error: Some(
                            "invalid endpoint_addr: expected JSON-serialized EndpointAddr"
                                .to_string(),
                        ),
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob add failed");
                    Ok(ClientRpcResponse::AddBlobResult(AddBlobResultResponse {
                        success: false,
                        hash: None,
                        size: None,
                        was_new: None,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
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
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                        found: false,
                        data: None,
                        error: Some("invalid hash".to_string()),
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob get failed");
                    Ok(ClientRpcResponse::GetBlobResult(GetBlobResultResponse {
                        found: false,
                        data: None,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob has check failed");
                    Ok(ClientRpcResponse::HasBlobResult(HasBlobResultResponse {
                        exists: false,
                        error: Some(sanitize_blob_error(&e)),
                    }))
                }
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
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::GetBlobTicketResult(
                        GetBlobTicketResultResponse {
                            success: false,
                            ticket: None,
                            error: Some("invalid hash".to_string()),
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob ticket generation failed");
                    Ok(ClientRpcResponse::GetBlobTicketResult(
                        GetBlobTicketResultResponse {
                            success: false,
                            ticket: None,
                            error: Some(sanitize_blob_error(&e)),
                        },
                    ))
                }
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob list failed");
                    Ok(ClientRpcResponse::ListBlobsResult(
                        ListBlobsResultResponse {
                            blobs: vec![],
                            count: 0,
                            has_more: false,
                            continuation_token: None,
                            error: Some(sanitize_blob_error(&e)),
                        },
                    ))
                }
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob protect failed");
                    Ok(ClientRpcResponse::ProtectBlobResult(
                        ProtectBlobResultResponse {
                            success: false,
                            error: Some(sanitize_blob_error(&e)),
                        },
                    ))
                }
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "blob unprotect failed");
                    Ok(ClientRpcResponse::UnprotectBlobResult(
                        UnprotectBlobResultResponse {
                            success: false,
                            error: Some(sanitize_blob_error(&e)),
                        },
                    ))
                }
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
                Err(_) => {
                    // HIGH-4: Don't expose parse error details
                    return Ok(ClientRpcResponse::AddPeerClusterResult(
                        AddPeerClusterResultResponse {
                            success: false,
                            cluster_id: None,
                            priority: None,
                            error: Some("invalid ticket".to_string()),
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "add peer cluster failed");
                    Ok(ClientRpcResponse::AddPeerClusterResult(
                        AddPeerClusterResultResponse {
                            success: false,
                            cluster_id: Some(cluster_id),
                            priority: None,
                            error: Some("peer cluster operation failed".to_string()),
                        },
                    ))
                }
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "remove peer cluster failed");
                    Ok(ClientRpcResponse::RemovePeerClusterResult(
                        RemovePeerClusterResultResponse {
                            success: false,
                            cluster_id,
                            error: Some("peer cluster operation failed".to_string()),
                        },
                    ))
                }
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "update peer cluster filter failed");
                    Ok(ClientRpcResponse::UpdatePeerClusterFilterResult(
                        UpdatePeerClusterFilterResultResponse {
                            success: false,
                            cluster_id,
                            filter_type: None,
                            error: Some("peer cluster operation failed".to_string()),
                        },
                    ))
                }
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "update peer cluster priority failed");
                    Ok(ClientRpcResponse::UpdatePeerClusterPriorityResult(
                        UpdatePeerClusterPriorityResultResponse {
                            success: false,
                            cluster_id,
                            previous_priority,
                            new_priority: None,
                            error: Some("peer cluster operation failed".to_string()),
                        },
                    ))
                }
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
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message to client
                    warn!(error = %e, "set peer cluster enabled failed");
                    Ok(ClientRpcResponse::SetPeerClusterEnabledResult(
                        SetPeerClusterEnabledResultResponse {
                            success: false,
                            cluster_id,
                            enabled: None,
                            error: Some("peer cluster operation failed".to_string()),
                        },
                    ))
                }
            }
        }

        ClientRpcRequest::ExecuteSql {
            query,
            params,
            consistency,
            limit,
            timeout_ms,
        } => {
            use crate::api::{SqlConsistency, SqlQueryExecutor, SqlQueryRequest, SqlValue};

            // Parse consistency level
            let consistency = match consistency.to_lowercase().as_str() {
                "stale" => SqlConsistency::Stale,
                _ => SqlConsistency::Linearizable, // Default to linearizable
            };

            // Parse parameters from JSON
            let params: Vec<SqlValue> = if params.is_empty() {
                Vec::new()
            } else {
                match serde_json::from_str::<Vec<serde_json::Value>>(&params) {
                    Ok(values) => values
                        .into_iter()
                        .map(|v| match v {
                            serde_json::Value::Null => SqlValue::Null,
                            serde_json::Value::Bool(b) => SqlValue::Integer(if b { 1 } else { 0 }),
                            serde_json::Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    SqlValue::Integer(i)
                                } else if let Some(f) = n.as_f64() {
                                    SqlValue::Real(f)
                                } else {
                                    SqlValue::Text(n.to_string())
                                }
                            }
                            serde_json::Value::String(s) => SqlValue::Text(s),
                            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                                SqlValue::Text(v.to_string())
                            }
                        })
                        .collect(),
                    Err(e) => {
                        return Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
                            success: false,
                            columns: None,
                            rows: None,
                            row_count: None,
                            is_truncated: None,
                            execution_time_ms: None,
                            error: Some(format!("invalid params JSON: {}", e)),
                        }));
                    }
                }
            };

            // Build request
            let request = SqlQueryRequest {
                query,
                params,
                consistency,
                limit,
                timeout_ms,
            };

            // Execute SQL query via SqlQueryExecutor trait
            // The RaftNode implements this trait
            match ctx.sql_executor.execute_sql(request).await {
                Ok(result) => {
                    // Convert SqlValue to serde_json::Value for response
                    let rows: Vec<Vec<serde_json::Value>> = result
                        .rows
                        .into_iter()
                        .map(|row| {
                            row.into_iter()
                                .map(|v| match v {
                                    SqlValue::Null => serde_json::Value::Null,
                                    SqlValue::Integer(i) => serde_json::json!(i),
                                    SqlValue::Real(f) => serde_json::json!(f),
                                    SqlValue::Text(s) => serde_json::Value::String(s),
                                    SqlValue::Blob(b) => {
                                        // Encode blob as base64
                                        use base64::Engine;
                                        serde_json::Value::String(
                                            base64::engine::general_purpose::STANDARD.encode(&b),
                                        )
                                    }
                                })
                                .collect()
                        })
                        .collect();

                    let columns: Vec<String> = result.columns.into_iter().map(|c| c.name).collect();

                    Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
                        success: true,
                        columns: Some(columns),
                        rows: Some(rows),
                        row_count: Some(result.row_count),
                        is_truncated: Some(result.is_truncated),
                        execution_time_ms: Some(result.execution_time_ms),
                        error: None,
                    }))
                }
                Err(e) => {
                    // HIGH-4: Log full error internally, return sanitized message
                    warn!(error = %e, "SQL query execution failed");
                    Ok(ClientRpcResponse::SqlResult(SqlResultResponse {
                        success: false,
                        columns: None,
                        rows: None,
                        row_count: None,
                        is_truncated: None,
                        execution_time_ms: None,
                        // Return the error message (SQL errors are safe to show)
                        error: Some(e.to_string()),
                    }))
                }
            }
        }

        // =====================================================================
        // Coordination Primitives - Distributed Lock
        // =====================================================================
        ClientRpcRequest::LockAcquire {
            key,
            holder_id,
            ttl_ms,
            timeout_ms,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            let config = LockConfig {
                ttl_ms,
                acquire_timeout_ms: timeout_ms,
                ..Default::default()
            };
            let lock = DistributedLock::new(ctx.kv_store.clone(), &key, &holder_id, config);

            match lock.acquire().await {
                Ok(guard) => {
                    let token = guard.fencing_token().value();
                    let deadline = guard.deadline_ms();
                    // Don't drop the guard here - let it release on its own
                    // The client is responsible for calling release explicitly
                    std::mem::forget(guard);
                    Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: true,
                        fencing_token: Some(token),
                        holder_id: Some(holder_id),
                        deadline_ms: Some(deadline),
                        error: None,
                    }))
                }
                Err(e) => {
                    use crate::coordination::CoordinationError;
                    let (holder, deadline) = match &e {
                        CoordinationError::LockHeld {
                            holder,
                            deadline_ms,
                        } => (Some(holder.clone()), Some(*deadline_ms)),
                        _ => (None, None),
                    };
                    Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: false,
                        fencing_token: None,
                        holder_id: holder,
                        deadline_ms: deadline,
                        error: Some(e.to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::LockTryAcquire {
            key,
            holder_id,
            ttl_ms,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            let config = LockConfig {
                ttl_ms,
                ..Default::default()
            };
            let lock = DistributedLock::new(ctx.kv_store.clone(), &key, &holder_id, config);

            match lock.try_acquire().await {
                Ok(guard) => {
                    let token = guard.fencing_token().value();
                    let deadline = guard.deadline_ms();
                    std::mem::forget(guard);
                    Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: true,
                        fencing_token: Some(token),
                        holder_id: Some(holder_id),
                        deadline_ms: Some(deadline),
                        error: None,
                    }))
                }
                Err(e) => {
                    use crate::coordination::CoordinationError;
                    let (holder, deadline) = match &e {
                        CoordinationError::LockHeld {
                            holder,
                            deadline_ms,
                        } => (Some(holder.clone()), Some(*deadline_ms)),
                        _ => (None, None),
                    };
                    Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: false,
                        fencing_token: None,
                        holder_id: holder,
                        deadline_ms: deadline,
                        error: Some(e.to_string()),
                    }))
                }
            }
        }

        ClientRpcRequest::LockRelease {
            key,
            holder_id,
            fencing_token,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            // Read current lock state and validate holder
            use crate::api::WriteCommand;
            use crate::coordination::LockEntry;

            let read_result = ctx.kv_store.read(ReadRequest { key: key.clone() }).await;

            match read_result {
                Ok(result) => {
                    match serde_json::from_str::<LockEntry>(&result.value) {
                        Ok(entry) => {
                            if entry.holder_id != holder_id || entry.fencing_token != fencing_token
                            {
                                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                    success: false,
                                    fencing_token: Some(entry.fencing_token),
                                    holder_id: Some(entry.holder_id),
                                    deadline_ms: Some(entry.deadline_ms),
                                    error: Some("lock not held by this holder".to_string()),
                                }));
                            }

                            // Release the lock via CAS
                            let released = entry.released();
                            let released_json = serde_json::to_string(&released).unwrap();

                            match ctx
                                .kv_store
                                .write(WriteRequest {
                                    command: WriteCommand::CompareAndSwap {
                                        key: key.clone(),
                                        expected: Some(result.value),
                                        new_value: released_json,
                                    },
                                })
                                .await
                            {
                                Ok(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                    success: true,
                                    fencing_token: Some(fencing_token),
                                    holder_id: Some(holder_id),
                                    deadline_ms: None,
                                    error: None,
                                })),
                                Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                    success: false,
                                    fencing_token: None,
                                    holder_id: None,
                                    deadline_ms: None,
                                    error: Some(format!("release failed: {}", e)),
                                })),
                            }
                        }
                        Err(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                            success: false,
                            fencing_token: None,
                            holder_id: None,
                            deadline_ms: None,
                            error: Some("invalid lock entry format".to_string()),
                        })),
                    }
                }
                Err(crate::api::KeyValueStoreError::NotFound { .. }) => {
                    Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: true, // Lock doesn't exist, consider it released
                        fencing_token: Some(fencing_token),
                        holder_id: Some(holder_id),
                        deadline_ms: None,
                        error: None,
                    }))
                }
                Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(format!("read failed: {}", e)),
                })),
            }
        }

        ClientRpcRequest::LockRenew {
            key,
            holder_id,
            fencing_token,
            ttl_ms,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(e.to_string()),
                }));
            }

            use crate::api::WriteCommand;
            use crate::coordination::LockEntry;

            let read_result = ctx.kv_store.read(ReadRequest { key: key.clone() }).await;

            match read_result {
                Ok(result) => match serde_json::from_str::<LockEntry>(&result.value) {
                    Ok(entry) => {
                        if entry.holder_id != holder_id || entry.fencing_token != fencing_token {
                            return Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                success: false,
                                fencing_token: Some(entry.fencing_token),
                                holder_id: Some(entry.holder_id),
                                deadline_ms: Some(entry.deadline_ms),
                                error: Some("lock not held by this holder".to_string()),
                            }));
                        }

                        // Create renewed entry
                        let renewed = LockEntry::new(holder_id.clone(), fencing_token, ttl_ms);
                        let renewed_json = serde_json::to_string(&renewed).unwrap();

                        match ctx
                            .kv_store
                            .write(WriteRequest {
                                command: WriteCommand::CompareAndSwap {
                                    key: key.clone(),
                                    expected: Some(result.value),
                                    new_value: renewed_json,
                                },
                            })
                            .await
                        {
                            Ok(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                success: true,
                                fencing_token: Some(fencing_token),
                                holder_id: Some(holder_id),
                                deadline_ms: Some(renewed.deadline_ms),
                                error: None,
                            })),
                            Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                                success: false,
                                fencing_token: None,
                                holder_id: None,
                                deadline_ms: None,
                                error: Some(format!("renew failed: {}", e)),
                            })),
                        }
                    }
                    Err(_) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                        success: false,
                        fencing_token: None,
                        holder_id: None,
                        deadline_ms: None,
                        error: Some("invalid lock entry format".to_string()),
                    })),
                },
                Err(e) => Ok(ClientRpcResponse::LockResult(LockResultResponse {
                    success: false,
                    fencing_token: None,
                    holder_id: None,
                    deadline_ms: None,
                    error: Some(format!("read failed: {}", e)),
                })),
            }
        }

        // =====================================================================
        // Coordination Primitives - Atomic Counter
        // =====================================================================
        ClientRpcRequest::CounterGet { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.get().await {
                Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterIncrement { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.increment().await {
                Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterDecrement { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.decrement().await {
                Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterAdd { key, amount } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.add(amount).await {
                Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterSubtract { key, amount } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.subtract(amount).await {
                Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterSet { key, value } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.set(value).await {
                Ok(()) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::CounterCompareAndSet {
            key,
            expected,
            new_value,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.compare_and_set(expected, new_value).await {
                Ok(true) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: true,
                    value: Some(new_value),
                    error: None,
                })),
                Ok(false) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some("compare-and-set condition not met".to_string()),
                })),
                Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        // =====================================================================
        // Coordination Primitives - Signed Counter
        // =====================================================================
        ClientRpcRequest::SignedCounterGet { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: false,
                        value: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let counter =
                SignedAtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.get().await {
                Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: true,
                        value: Some(value),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: false,
                        value: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::SignedCounterAdd { key, amount } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: false,
                        value: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let counter =
                SignedAtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
            match counter.add(amount).await {
                Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: true,
                        value: Some(value),
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::SignedCounterResult(
                    SignedCounterResultResponse {
                        success: false,
                        value: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        // =====================================================================
        // Coordination Primitives - Sequence Generator
        // =====================================================================
        ClientRpcRequest::SequenceNext { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
            match seq.next().await {
                Ok(value) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::SequenceReserve { key, count } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
            match seq.reserve(count).await {
                Ok(start) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: true,
                    value: Some(start),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        ClientRpcRequest::SequenceCurrent { key } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                }));
            }

            let seq = SequenceGenerator::new(ctx.kv_store.clone(), &key, SequenceConfig::default());
            match seq.current().await {
                Ok(value) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: true,
                    value: Some(value),
                    error: None,
                })),
                Err(e) => Ok(ClientRpcResponse::SequenceResult(SequenceResultResponse {
                    success: false,
                    value: None,
                    error: Some(e.to_string()),
                })),
            }
        }

        // =====================================================================
        // Coordination Primitives - Rate Limiter
        // =====================================================================
        ClientRpcRequest::RateLimiterTryAcquire {
            key,
            tokens,
            capacity,
            refill_rate,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter.try_acquire_n(tokens).await {
                Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: true,
                        tokens_remaining: Some(remaining),
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: Some(e.available),
                        retry_after_ms: Some(e.retry_after_ms),
                        error: None,
                    },
                )),
            }
        }

        ClientRpcRequest::RateLimiterAcquire {
            key,
            tokens,
            capacity,
            refill_rate,
            timeout_ms,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter
                .acquire_n(tokens, std::time::Duration::from_millis(timeout_ms))
                .await
            {
                Ok(remaining) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: true,
                        tokens_remaining: Some(remaining),
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: Some(e.available),
                        retry_after_ms: Some(e.retry_after_ms),
                        error: Some("timeout waiting for tokens".to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::RateLimiterAvailable {
            key,
            capacity,
            refill_rate,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter.available().await {
                Ok(tokens) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: true,
                        tokens_remaining: Some(tokens),
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::RateLimiterReset {
            key,
            capacity,
            refill_rate,
        } => {
            if let Err(e) = validate_client_key(&key) {
                return Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                ));
            }

            let config = RateLimiterConfig {
                capacity,
                refill_rate,
                initial_tokens: None,
            };
            let limiter = DistributedRateLimiter::new(ctx.kv_store.clone(), &key, config);
            match limiter.reset().await {
                Ok(()) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: true,
                        tokens_remaining: Some(capacity),
                        retry_after_ms: None,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::RateLimiterResult(
                    RateLimiterResultResponse {
                        success: false,
                        tokens_remaining: None,
                        retry_after_ms: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        // ==========================================================================
        // Batch Operations
        // ==========================================================================
        ClientRpcRequest::BatchRead { keys } => {
            use crate::api::KeyValueStoreError;
            // Validate all keys
            for key in &keys {
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::BatchReadResult(
                        BatchReadResultResponse {
                            success: false,
                            values: None,
                            error: Some(e.to_string()),
                        },
                    ));
                }
            }

            // Read all keys atomically
            let mut values = Vec::with_capacity(keys.len());
            for key in &keys {
                let request = ReadRequest { key: key.clone() };
                match ctx.kv_store.read(request).await {
                    Ok(result) => {
                        // Key exists - return the value
                        values.push(Some(result.value.into_bytes()));
                    }
                    Err(KeyValueStoreError::NotFound { .. }) => {
                        // Key doesn't exist - return None for this position
                        values.push(None);
                    }
                    Err(e) => {
                        // Real error - fail the entire batch
                        return Ok(ClientRpcResponse::BatchReadResult(
                            BatchReadResultResponse {
                                success: false,
                                values: None,
                                error: Some(e.to_string()),
                            },
                        ));
                    }
                }
            }

            Ok(ClientRpcResponse::BatchReadResult(
                BatchReadResultResponse {
                    success: true,
                    values: Some(values),
                    error: None,
                },
            ))
        }

        ClientRpcRequest::BatchWrite { operations } => {
            use crate::api::{BatchOperation, WriteCommand};
            use crate::client_rpc::BatchWriteOperation;

            // Validate all keys
            for op in &operations {
                let key = match op {
                    BatchWriteOperation::Set { key, .. } => key,
                    BatchWriteOperation::Delete { key } => key,
                };
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::BatchWriteResult(
                        BatchWriteResultResponse {
                            success: false,
                            operations_applied: None,
                            error: Some(e.to_string()),
                        },
                    ));
                }
            }

            // Convert to internal batch operations
            let batch_ops: Vec<BatchOperation> = operations
                .iter()
                .map(|op| match op {
                    BatchWriteOperation::Set { key, value } => BatchOperation::Set {
                        key: key.clone(),
                        value: String::from_utf8_lossy(value).to_string(),
                    },
                    BatchWriteOperation::Delete { key } => {
                        BatchOperation::Delete { key: key.clone() }
                    }
                })
                .collect();

            let request = WriteRequest {
                command: WriteCommand::Batch {
                    operations: batch_ops,
                },
            };

            match ctx.kv_store.write(request).await {
                Ok(result) => Ok(ClientRpcResponse::BatchWriteResult(
                    BatchWriteResultResponse {
                        success: true,
                        operations_applied: result.batch_applied,
                        error: None,
                    },
                )),
                Err(e) => Ok(ClientRpcResponse::BatchWriteResult(
                    BatchWriteResultResponse {
                        success: false,
                        operations_applied: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }

        ClientRpcRequest::ConditionalBatchWrite {
            conditions,
            operations,
        } => {
            use crate::api::{BatchCondition as ApiBatchCondition, BatchOperation, WriteCommand};
            use crate::client_rpc::{BatchCondition, BatchWriteOperation};

            // Validate all keys in conditions
            for cond in &conditions {
                let key = match cond {
                    BatchCondition::ValueEquals { key, .. } => key,
                    BatchCondition::KeyExists { key } => key,
                    BatchCondition::KeyNotExists { key } => key,
                };
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::ConditionalBatchWriteResult(
                        ConditionalBatchWriteResultResponse {
                            success: false,
                            conditions_met: false,
                            operations_applied: None,
                            failed_condition_index: None,
                            failed_condition_reason: Some(e.to_string()),
                            error: None,
                        },
                    ));
                }
            }

            // Validate all keys in operations
            for op in &operations {
                let key = match op {
                    BatchWriteOperation::Set { key, .. } => key,
                    BatchWriteOperation::Delete { key } => key,
                };
                if let Err(e) = validate_client_key(key) {
                    return Ok(ClientRpcResponse::ConditionalBatchWriteResult(
                        ConditionalBatchWriteResultResponse {
                            success: false,
                            conditions_met: false,
                            operations_applied: None,
                            failed_condition_index: None,
                            failed_condition_reason: Some(e.to_string()),
                            error: None,
                        },
                    ));
                }
            }

            // Convert conditions
            let api_conditions: Vec<ApiBatchCondition> = conditions
                .iter()
                .map(|c| match c {
                    BatchCondition::ValueEquals { key, expected } => {
                        ApiBatchCondition::ValueEquals {
                            key: key.clone(),
                            expected: String::from_utf8_lossy(expected).to_string(),
                        }
                    }
                    BatchCondition::KeyExists { key } => {
                        ApiBatchCondition::KeyExists { key: key.clone() }
                    }
                    BatchCondition::KeyNotExists { key } => {
                        ApiBatchCondition::KeyNotExists { key: key.clone() }
                    }
                })
                .collect();

            // Convert operations
            let batch_ops: Vec<BatchOperation> = operations
                .iter()
                .map(|op| match op {
                    BatchWriteOperation::Set { key, value } => BatchOperation::Set {
                        key: key.clone(),
                        value: String::from_utf8_lossy(value).to_string(),
                    },
                    BatchWriteOperation::Delete { key } => {
                        BatchOperation::Delete { key: key.clone() }
                    }
                })
                .collect();

            let request = WriteRequest {
                command: WriteCommand::ConditionalBatch {
                    conditions: api_conditions,
                    operations: batch_ops,
                },
            };

            match ctx.kv_store.write(request).await {
                Ok(result) => {
                    let conditions_met = result.conditions_met.unwrap_or(false);
                    Ok(ClientRpcResponse::ConditionalBatchWriteResult(
                        ConditionalBatchWriteResultResponse {
                            success: conditions_met,
                            conditions_met,
                            operations_applied: result.batch_applied,
                            failed_condition_index: result.failed_condition_index,
                            failed_condition_reason: None,
                            error: None,
                        },
                    ))
                }
                Err(e) => Ok(ClientRpcResponse::ConditionalBatchWriteResult(
                    ConditionalBatchWriteResultResponse {
                        success: false,
                        conditions_met: false,
                        operations_applied: None,
                        failed_condition_index: None,
                        failed_condition_reason: None,
                        error: Some(e.to_string()),
                    },
                )),
            }
        }
    }
}
