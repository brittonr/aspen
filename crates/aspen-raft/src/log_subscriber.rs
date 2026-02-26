//! Raft log subscription protocol for read-only clients.
//!
//! Provides a streaming interface for clients to receive committed Raft log
//! entries in real-time. This enables building reactive systems that respond
//! to state changes without polling.
//!
//! # Protocol Flow
//!
//! 1. Client connects via LOG_SUBSCRIBER_ALPN
//! 2. Server sends AuthChallenge (same as Raft auth)
//! 3. Client sends AuthResponse with valid HMAC
//! 4. Server sends AuthResult
//! 5. If authenticated, client sends SubscribeRequest
//! 6. Server streams LogEntryMessage for each committed entry
//!
//! # Design Notes
//!
//! - Read-only: Subscribers cannot modify state
//! - Authenticated: Uses same cookie-based auth as Raft RPC
//! - Bounded: Fixed buffer sizes and rate limits
//! - Resumable: Clients can specify starting log index
//!
//! # Tiger Style
//!
//! - Fixed message sizes with explicit bounds
//! - Explicit subscription limits (max subscribers, buffer size)
//! - Clear protocol versioning for forward compatibility

use std::time::Duration;

use aspen_core::utils;
use aspen_hlc::SerializableTimestamp;
// Re-export types from aspen-transport for backwards compatibility
pub use aspen_transport::log_subscriber::{
    EndOfStreamReason, HistoricalLogReader, KvOperation, LOG_SUBSCRIBE_PROTOCOL_VERSION, LOG_SUBSCRIBER_ALPN,
    LogEntryMessage, LogEntryPayload, MAX_HISTORICAL_BATCH_SIZE, SubscribeRejectReason, SubscribeRequest,
    SubscribeResponse,
};

/// Maximum number of concurrent log subscribers per node.
///
/// Tiger Style: Fixed upper bound on subscriber connections.
pub const MAX_LOG_SUBSCRIBERS: u32 = 100;

/// Size of the broadcast channel buffer for log entries.
///
/// Subscribers that fall behind by more than this many entries
/// will experience lag (receive lagged error).
pub const LOG_BROADCAST_BUFFER_SIZE: u32 = 1000;

/// Maximum size of a single log entry message (10 MB).
///
/// Matches MAX_RPC_MESSAGE_SIZE for consistency.
pub const MAX_LOG_ENTRY_MESSAGE_SIZE: u32 = 10 * 1024 * 1024;

/// Timeout for subscription handshake (5 seconds).
pub const SUBSCRIBE_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// Keepalive interval for idle connections (30 seconds).
pub const SUBSCRIBE_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);

// ============================================================================
// Subscriber State
// ============================================================================

/// State tracking for a connected subscriber.
#[derive(Debug)]
pub struct SubscriberState {
    /// Unique identifier for this subscriber connection.
    pub id: u64,
    /// Client's Iroh endpoint ID.
    pub client_endpoint_id: [u8; 32],
    /// Key prefix filter (empty = all keys).
    pub key_prefix: Vec<u8>,
    /// Last sent log index.
    pub last_sent_index: u64,
    /// Connection timestamp (milliseconds since UNIX epoch).
    pub connected_at_ms: u64,
    /// Number of entries sent.
    pub entries_sent: u64,
}

impl SubscriberState {
    /// Create new subscriber state.
    pub fn new(id: u64, client_endpoint_id: [u8; 32], key_prefix: Vec<u8>, start_index: u64) -> Self {
        Self {
            id,
            client_endpoint_id,
            key_prefix,
            last_sent_index: start_index.saturating_sub(1),
            connected_at_ms: current_time_ms(),
            entries_sent: 0,
        }
    }

    /// Check if an operation should be sent to this subscriber.
    pub fn should_send(&self, operation: &KvOperation) -> bool {
        operation.matches_prefix(&self.key_prefix)
    }

    /// Record that an entry was sent.
    pub fn record_sent(&mut self, index: u64) {
        self.last_sent_index = index;
        self.entries_sent += 1;
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get current time in milliseconds since UNIX epoch.
///
/// Delegates to `aspen_core::utils::current_time_ms()` for Tiger Style compliance.
#[inline]
fn current_time_ms() -> u64 {
    utils::current_time_ms()
}

// ============================================================================
// Protocol Handler
// ============================================================================

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use crate::auth::AUTH_HANDSHAKE_TIMEOUT;
use crate::auth::AuthContext;
use crate::auth::AuthResponse;
use crate::auth::AuthResult;
use crate::auth::MAX_AUTH_MESSAGE_SIZE;

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
    /// Hybrid Logical Clock for deterministic timestamp ordering.
    hlc: aspen_core::hlc::HLC,
}

impl std::fmt::Debug for LogSubscriberProtocolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogSubscriberProtocolHandler")
            .field("node_id", &self.node_id)
            .field("committed_index", &self.committed_index)
            .finish_non_exhaustive()
    }
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
    pub fn new(cluster_cookie: &str, node_id: u64) -> (Self, broadcast::Sender<LogEntryPayload>, Arc<AtomicU64>) {
        let (log_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE as usize);
        let committed_index = Arc::new(AtomicU64::new(0));
        let hlc = aspen_core::hlc::create_hlc(&node_id.to_string());
        let handler = Self {
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS as usize)),
            log_sender: log_sender.clone(),
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index: committed_index.clone(),
            historical_reader: None,
            hlc,
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
        let hlc = aspen_core::hlc::create_hlc(&node_id.to_string());
        Self {
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS as usize)),
            log_sender,
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index,
            historical_reader: None,
            hlc,
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
        let hlc = aspen_core::hlc::create_hlc(&node_id.to_string());
        Self {
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS as usize)),
            log_sender,
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index,
            historical_reader: Some(historical_reader),
            hlc,
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
                return Err(AcceptError::from_err(std::io::Error::other("subscriber limit reached")));
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
            &self.hlc,
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
///
/// Orchestrates the phases of subscriber connection handling:
/// 1. Authentication handshake
/// 2. Subscription request
/// 3. Acceptance
/// 4. Historical replay (if requested)
/// 5. Live streaming
#[allow(clippy::too_many_arguments)]
#[instrument(skip(connection, auth_context, log_receiver, committed_index, historical_reader, hlc))]
async fn handle_log_subscriber_connection(
    connection: Connection,
    auth_context: AuthContext,
    log_receiver: broadcast::Receiver<LogEntryPayload>,
    node_id: u64,
    subscriber_id: u64,
    committed_index: Arc<AtomicU64>,
    historical_reader: Option<Arc<dyn HistoricalLogReader>>,
    hlc: &aspen_core::hlc::HLC,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let remote_node_id = connection.remote_id();

    // Accept the initial stream for authentication and subscription setup
    let (mut send, mut recv) = connection.accept_bi().await.context("failed to accept subscriber stream")?;

    // Phase 1: Authenticate subscriber
    handle_log_subscriber_authenticate(&auth_context, subscriber_id, &mut send, &mut recv).await?;

    // Phase 2: Receive and validate subscription request
    let sub_request = handle_log_subscriber_receive_request(subscriber_id, &mut send, &mut recv).await?;

    debug!(
        subscriber_id = subscriber_id,
        start_index = sub_request.start_index,
        prefix = ?sub_request.key_prefix,
        "processing subscription request"
    );

    // Phase 3: Accept subscription
    let current_committed_index = committed_index.load(Ordering::Acquire);
    handle_log_subscriber_accept(node_id, subscriber_id, current_committed_index, &mut send).await?;

    info!(
        subscriber_id = subscriber_id,
        remote = %remote_node_id,
        start_index = sub_request.start_index,
        current_index = current_committed_index,
        "log subscription active"
    );

    // Phase 4: Historical replay if requested
    handle_log_subscriber_replay(subscriber_id, &sub_request, current_committed_index, &historical_reader, &mut send)
        .await?;

    // Phase 5: Stream live log entries
    handle_log_subscriber_stream(subscriber_id, sub_request.key_prefix, log_receiver, &committed_index, hlc, &mut send)
        .await;

    // Best-effort stream finish - log if it fails
    if let Err(finish_err) = send.finish() {
        debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish log subscription stream");
    }

    info!(subscriber_id = subscriber_id, "log subscription ended");

    Ok(())
}

/// Authenticate a log subscriber connection.
///
/// Performs the challenge-response authentication handshake.
async fn handle_log_subscriber_authenticate(
    auth_context: &AuthContext,
    subscriber_id: u64,
    send: &mut iroh::endpoint::SendStream,
    recv: &mut iroh::endpoint::RecvStream,
) -> anyhow::Result<()> {
    use anyhow::Context;

    // Step 1: Send challenge
    let challenge = auth_context.generate_challenge();
    let challenge_bytes = postcard::to_stdvec(&challenge).context("failed to serialize challenge")?;
    send.write_all(&challenge_bytes).await.context("failed to send challenge")?;

    // Step 2: Receive auth response
    let response_result = tokio::time::timeout(AUTH_HANDSHAKE_TIMEOUT, async {
        let buffer = recv.read_to_end(MAX_AUTH_MESSAGE_SIZE).await.context("failed to read auth response")?;
        let response: AuthResponse = postcard::from_bytes(&buffer).context("failed to deserialize auth response")?;
        Ok::<_, anyhow::Error>(response)
    })
    .await;

    let auth_response = match response_result {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => {
            warn!(error = %err, subscriber_id = subscriber_id, "subscriber auth failed");
            handle_log_subscriber_send_auth_failure(subscriber_id, send).await;
            return Err(err);
        }
        Err(_) => {
            warn!(subscriber_id = subscriber_id, "subscriber auth timed out");
            handle_log_subscriber_send_auth_failure(subscriber_id, send).await;
            return Err(anyhow::anyhow!("authentication timeout"));
        }
    };

    // Step 3: Verify
    let auth_result = auth_context.verify_response(&challenge, &auth_response);

    // Step 4: Send auth result
    let result_bytes = postcard::to_stdvec(&auth_result).context("failed to serialize auth result")?;
    send.write_all(&result_bytes).await.context("failed to send auth result")?;

    if !auth_result.is_ok() {
        warn!(subscriber_id = subscriber_id, result = ?auth_result, "subscriber auth failed");
        if let Err(finish_err) = send.finish() {
            debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth verification failure");
        }
        return Err(anyhow::anyhow!("authentication failed: {:?}", auth_result));
    }

    debug!(subscriber_id = subscriber_id, "subscriber authenticated");
    Ok(())
}

/// Send auth failure response (best-effort).
async fn handle_log_subscriber_send_auth_failure(subscriber_id: u64, send: &mut iroh::endpoint::SendStream) {
    if let Ok(result_bytes) = postcard::to_stdvec(&AuthResult::Failed)
        && let Err(write_err) = send.write_all(&result_bytes).await
    {
        debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send auth failure to subscriber");
    }
    if let Err(finish_err) = send.finish() {
        debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscriber auth failure");
    }
}

/// Receive subscription request from authenticated subscriber.
async fn handle_log_subscriber_receive_request(
    subscriber_id: u64,
    send: &mut iroh::endpoint::SendStream,
    recv: &mut iroh::endpoint::RecvStream,
) -> anyhow::Result<SubscribeRequest> {
    use anyhow::Context;

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

    match sub_request_result {
        Ok(Ok(request)) => Ok(request),
        Ok(Err(err)) => {
            handle_log_subscriber_send_rejection(subscriber_id, send).await;
            Err(err)
        }
        Err(_) => {
            handle_log_subscriber_send_rejection(subscriber_id, send).await;
            Err(anyhow::anyhow!("subscribe request timeout"))
        }
    }
}

/// Send subscription rejection response (best-effort).
async fn handle_log_subscriber_send_rejection(subscriber_id: u64, send: &mut iroh::endpoint::SendStream) {
    let response = SubscribeResponse::Rejected {
        reason: SubscribeRejectReason::InternalError,
    };
    if let Ok(response_bytes) = postcard::to_stdvec(&response)
        && let Err(write_err) = send.write_all(&response_bytes).await
    {
        debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send subscribe rejection");
    }
    if let Err(finish_err) = send.finish() {
        debug!(subscriber_id = subscriber_id, error = %finish_err, "failed to finish stream after subscribe error");
    }
}

/// Accept subscription and send acceptance response.
async fn handle_log_subscriber_accept(
    node_id: u64,
    subscriber_id: u64,
    current_committed_index: u64,
    send: &mut iroh::endpoint::SendStream,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let response = SubscribeResponse::Accepted {
        current_index: current_committed_index,
        node_id,
    };
    let response_bytes = postcard::to_stdvec(&response).context("failed to serialize subscribe response")?;
    send.write_all(&response_bytes).await.context("failed to send subscribe response")?;

    debug!(subscriber_id = subscriber_id, current_committed_index, "subscription accepted");
    Ok(())
}

/// Replay historical log entries if requested and available.
async fn handle_log_subscriber_replay(
    subscriber_id: u64,
    sub_request: &SubscribeRequest,
    current_committed_index: u64,
    historical_reader: &Option<Arc<dyn HistoricalLogReader>>,
    send: &mut iroh::endpoint::SendStream,
) -> anyhow::Result<()> {
    // Check if historical replay is needed
    let needs_replay = sub_request.start_index < current_committed_index;
    let is_not_latest_only = sub_request.start_index != u64::MAX;
    if !needs_replay || !is_not_latest_only {
        return Ok(());
    }

    let reader = match historical_reader {
        Some(r) => r,
        None => {
            debug!(subscriber_id = subscriber_id, "historical replay requested but no reader available");
            return Ok(());
        }
    };

    debug!(
        subscriber_id = subscriber_id,
        start_index = sub_request.start_index,
        end_index = current_committed_index,
        "starting historical replay"
    );

    let mut current_start = sub_request.start_index;
    let mut total_replayed = 0u64;

    while current_start <= current_committed_index {
        let batch_end =
            std::cmp::min(current_start.saturating_add(MAX_HISTORICAL_BATCH_SIZE as u64 - 1), current_committed_index);

        match reader.read_entries(current_start, batch_end).await {
            Ok(entries) => {
                if entries.is_empty() {
                    debug!(
                        subscriber_id = subscriber_id,
                        start = current_start,
                        "no historical entries available, may have been compacted"
                    );
                    break;
                }

                for entry in entries {
                    // Apply prefix filter
                    let has_prefix_filter = !sub_request.key_prefix.is_empty();
                    let is_not_matching = !entry.operation.matches_prefix(&sub_request.key_prefix);
                    if has_prefix_filter && is_not_matching {
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
                        return Err(anyhow::anyhow!("subscriber disconnected during replay"));
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

        if current_start > current_committed_index {
            break;
        }
    }

    info!(subscriber_id = subscriber_id, total_replayed = total_replayed, "historical replay complete");
    Ok(())
}

/// Stream live log entries to subscriber.
async fn handle_log_subscriber_stream(
    subscriber_id: u64,
    key_prefix: Vec<u8>,
    mut log_receiver: broadcast::Receiver<LogEntryPayload>,
    committed_index: &Arc<AtomicU64>,
    hlc: &aspen_core::hlc::HLC,
    send: &mut iroh::endpoint::SendStream,
) {
    let mut keepalive_interval = tokio::time::interval(SUBSCRIBE_KEEPALIVE_INTERVAL);
    keepalive_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            entry_result = log_receiver.recv() => {
                match entry_result {
                    Ok(entry) => {
                        // Apply prefix filter
                        // Decomposed: check if filter is active first, then check match
                        let has_prefix_filter = !key_prefix.is_empty();
                        let is_not_matching = !entry.operation.matches_prefix(&key_prefix);
                        if has_prefix_filter && is_not_matching {
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
                        handle_log_subscriber_send_end_of_stream(subscriber_id, EndOfStreamReason::Lagged, send).await;
                        break;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!(subscriber_id = subscriber_id, "log broadcast channel closed");
                        handle_log_subscriber_send_end_of_stream(subscriber_id, EndOfStreamReason::ServerShutdown, send).await;
                        break;
                    }
                }
            }

            _ = keepalive_interval.tick() => {
                let keepalive = LogEntryMessage::Keepalive {
                    committed_index: committed_index.load(Ordering::Acquire),
                    hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
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
}

/// Send end-of-stream message (best-effort).
async fn handle_log_subscriber_send_end_of_stream(
    subscriber_id: u64,
    reason: EndOfStreamReason,
    send: &mut iroh::endpoint::SendStream,
) {
    let end_message = LogEntryMessage::EndOfStream { reason };
    if let Ok(bytes) = postcard::to_stdvec(&end_message)
        && let Err(write_err) = send.write_all(&bytes).await
    {
        debug!(subscriber_id = subscriber_id, error = %write_err, "failed to send end-of-stream");
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_request_from_index() {
        let req = SubscribeRequest::from_index(100);
        assert_eq!(req.start_index, 100);
        assert!(req.key_prefix.is_empty());
        assert_eq!(req.protocol_version, LOG_SUBSCRIBE_PROTOCOL_VERSION);
    }

    #[test]
    fn test_subscribe_request_latest_only() {
        let req = SubscribeRequest::latest_only();
        assert_eq!(req.start_index, u64::MAX);
    }

    #[test]
    fn test_subscribe_request_with_prefix() {
        let req = SubscribeRequest::with_prefix(50, b"users/".to_vec());
        assert_eq!(req.start_index, 50);
        assert_eq!(req.key_prefix, b"users/");
    }

    #[test]
    fn test_kv_operation_matches_prefix() {
        let set_op = KvOperation::Set {
            key: b"users/123".to_vec(),
            value: b"data".to_vec(),
        };

        assert!(set_op.matches_prefix(b""));
        assert!(set_op.matches_prefix(b"users/"));
        assert!(set_op.matches_prefix(b"users/123"));
        assert!(!set_op.matches_prefix(b"posts/"));

        let multi_op = KvOperation::SetMulti {
            pairs: vec![
                (b"users/1".to_vec(), b"a".to_vec()),
                (b"posts/1".to_vec(), b"b".to_vec()),
            ],
        };

        assert!(multi_op.matches_prefix(b"users/"));
        assert!(multi_op.matches_prefix(b"posts/"));
        assert!(!multi_op.matches_prefix(b"comments/"));

        // Control operations always match
        assert!(KvOperation::Noop.matches_prefix(b"anything"));
    }

    #[test]
    fn test_kv_operation_primary_key() {
        let set_op = KvOperation::Set {
            key: b"key1".to_vec(),
            value: b"val".to_vec(),
        };
        assert_eq!(set_op.primary_key(), Some(b"key1".as_slice()));

        let delete_op = KvOperation::Delete { key: b"key2".to_vec() };
        assert_eq!(delete_op.primary_key(), Some(b"key2".as_slice()));

        assert_eq!(KvOperation::Noop.primary_key(), None);
    }

    #[test]
    fn test_kv_operation_key_count() {
        assert_eq!(
            KvOperation::Set {
                key: vec![],
                value: vec![]
            }
            .key_count(),
            1
        );

        assert_eq!(
            KvOperation::SetMulti {
                pairs: vec![(vec![], vec![]), (vec![], vec![]), (vec![], vec![])]
            }
            .key_count(),
            3
        );

        assert_eq!(KvOperation::Noop.key_count(), 0);
    }

    #[test]
    fn test_subscriber_state_should_send() {
        let mut state = SubscriberState::new(1, [0u8; 32], b"users/".to_vec(), 0);

        let matching = KvOperation::Set {
            key: b"users/123".to_vec(),
            value: vec![],
        };
        assert!(state.should_send(&matching));

        let non_matching = KvOperation::Set {
            key: b"posts/456".to_vec(),
            value: vec![],
        };
        assert!(!state.should_send(&non_matching));

        // With empty prefix, everything matches
        state.key_prefix = vec![];
        assert!(state.should_send(&non_matching));
    }

    #[test]
    fn test_subscriber_state_record_sent() {
        let mut state = SubscriberState::new(1, [0u8; 32], vec![], 10);
        assert_eq!(state.last_sent_index, 9); // start_index - 1
        assert_eq!(state.entries_sent, 0);

        state.record_sent(10);
        assert_eq!(state.last_sent_index, 10);
        assert_eq!(state.entries_sent, 1);

        state.record_sent(11);
        assert_eq!(state.last_sent_index, 11);
        assert_eq!(state.entries_sent, 2);
    }

    #[test]
    fn test_subscribe_reject_reason_display() {
        assert_eq!(SubscribeRejectReason::TooManySubscribers.to_string(), "too many subscribers");
        assert_eq!(SubscribeRejectReason::IndexNotAvailable.to_string(), "requested index not available");
    }

    #[test]
    fn test_end_of_stream_reason_display() {
        assert_eq!(EndOfStreamReason::ServerShutdown.to_string(), "server shutdown");
        assert_eq!(EndOfStreamReason::Lagged.to_string(), "subscriber lagged");
    }
}
