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
pub const MAX_LOG_ENTRY_MESSAGE_SIZE: u32 = 10_u32.saturating_mul(1024).saturating_mul(1024);

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
        self.entries_sent = self.entries_sent.saturating_add(1);
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
use tracing::Instrument;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::auth::AUTH_HANDSHAKE_TIMEOUT;
use crate::auth::AuthContext;
use crate::auth::AuthResponse;
use crate::auth::AuthResult;
use crate::auth::MAX_AUTH_MESSAGE_SIZE;

#[derive(Clone)]
struct SubscriberConnectionContext<'a> {
    node_id: u64,
    subscriber_id: u64,
    committed_index: Arc<AtomicU64>,
    historical_reader: Option<Arc<dyn HistoricalLogReader>>,
    hlc: &'a aspen_core::hlc::HLC,
}

#[derive(Clone, Copy)]
struct SubscriptionAcceptance {
    node_id: u64,
    subscriber_id: u64,
    current_committed_index: u64,
}

#[derive(Clone, Copy)]
struct KeepaliveContext<'a> {
    committed_index: &'a Arc<AtomicU64>,
    hlc: &'a aspen_core::hlc::HLC,
}

#[derive(Clone, Copy)]
struct ReplayRange {
    start_index: u64,
    committed_index: u64,
}

#[inline]
fn usize_from_u32(value_u32: u32) -> usize {
    usize::try_from(value_u32).unwrap_or(usize::MAX)
}

#[inline]
fn should_replay_historical_entries(replay_range: ReplayRange) -> bool {
    replay_range.start_index < replay_range.committed_index && replay_range.start_index != u64::MAX
}

#[inline]
fn historical_batch_end(replay_range: ReplayRange) -> u64 {
    let max_batch_span = u64::try_from(MAX_HISTORICAL_BATCH_SIZE).map(|v| v.saturating_sub(1)).unwrap_or(u64::MAX);
    replay_range.start_index.saturating_add(max_batch_span).min(replay_range.committed_index)
}

#[inline]
fn matches_subscriber_filter(key_prefix: &[u8], operation: &KvOperation) -> bool {
    key_prefix.is_empty() || operation.matches_prefix(key_prefix)
}

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
        let (log_sender, _) = broadcast::channel(usize_from_u32(LOG_BROADCAST_BUFFER_SIZE));
        let committed_index = Arc::new(AtomicU64::new(0));
        let hlc = aspen_core::hlc::create_hlc(&node_id.to_string());
        let handler = Self {
            auth_context: AuthContext::new(cluster_cookie),
            connection_semaphore: Arc::new(Semaphore::new(usize_from_u32(MAX_LOG_SUBSCRIBERS))),
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
            connection_semaphore: Arc::new(Semaphore::new(usize_from_u32(MAX_LOG_SUBSCRIBERS))),
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
            connection_semaphore: Arc::new(Semaphore::new(usize_from_u32(MAX_LOG_SUBSCRIBERS))),
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
            SubscriberConnectionContext {
                node_id: self.node_id,
                subscriber_id,
                committed_index: self.committed_index.clone(),
                historical_reader: self.historical_reader.clone(),
                hlc: &self.hlc,
            },
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
async fn handle_log_subscriber_connection(
    connection: Connection,
    auth_context: AuthContext,
    log_receiver: broadcast::Receiver<LogEntryPayload>,
    context: SubscriberConnectionContext<'_>,
) -> anyhow::Result<()> {
    let subscriber_id = context.subscriber_id;
    async move {
        use anyhow::Context;

        let remote_node_id = connection.remote_id();
        debug_assert!(context.node_id > 0, "node_id must be positive");
        debug_assert!(context.subscriber_id > 0, "subscriber_id must be positive");

        // Accept the initial stream for authentication and subscription setup
        let (mut send, mut recv) = connection.accept_bi().await.context("failed to accept subscriber stream")?;

        // Phase 1: Authenticate subscriber
        handle_log_subscriber_authenticate(&auth_context, context.subscriber_id, &mut send, &mut recv).await?;

        // Phase 2: Receive and validate subscription request
        let sub_request = handle_log_subscriber_receive_request(context.subscriber_id, &mut send, &mut recv).await?;

        debug!(
            subscriber_id = context.subscriber_id,
            start_index = sub_request.start_index,
            prefix = ?sub_request.key_prefix,
            "processing subscription request"
        );

        // Phase 3: Accept subscription
        let current_committed_index = context.committed_index.load(Ordering::Acquire);
        handle_log_subscriber_accept(
            SubscriptionAcceptance {
                node_id: context.node_id,
                subscriber_id: context.subscriber_id,
                current_committed_index,
            },
            &mut send,
        )
        .await?;

        info!(
            subscriber_id = context.subscriber_id,
            remote = %remote_node_id,
            start_index = sub_request.start_index,
            current_index = current_committed_index,
            "log subscription active"
        );

        // Phase 4: Historical replay if requested
        handle_log_subscriber_replay(
            context.subscriber_id,
            &sub_request,
            current_committed_index,
            &context.historical_reader,
            &mut send,
        )
        .await?;

        // Phase 5: Stream live log entries
        handle_log_subscriber_stream(
            context.subscriber_id,
            sub_request.key_prefix,
            log_receiver,
            KeepaliveContext {
                committed_index: &context.committed_index,
                hlc: context.hlc,
            },
            &mut send,
        )
        .await;

        // Best-effort stream finish - log if it fails
        if let Err(finish_err) = send.finish() {
            debug!(subscriber_id = context.subscriber_id, error = %finish_err, "failed to finish log subscription stream");
        }

        info!(subscriber_id = context.subscriber_id, "log subscription ended");

        Ok(())
    }
    .instrument(tracing::info_span!("handle_log_subscriber_connection", subscriber_id = subscriber_id))
    .await
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

    debug_assert!(subscriber_id > 0, "subscriber_id must be positive during auth");
    const { assert!(MAX_AUTH_MESSAGE_SIZE > 0, "auth message limit must be positive") };

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

    debug_assert!(subscriber_id > 0, "subscriber_id must be positive during subscribe request");
    debug_assert!(SUBSCRIBE_HANDSHAKE_TIMEOUT > Duration::ZERO, "subscribe handshake timeout must be positive");

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
    acceptance: SubscriptionAcceptance,
    send: &mut iroh::endpoint::SendStream,
) -> anyhow::Result<()> {
    use anyhow::Context;

    debug_assert!(acceptance.node_id > 0, "node_id must be positive when accepting subscription");
    debug_assert!(acceptance.subscriber_id > 0, "subscriber_id must be positive when accepting subscription");
    let response = SubscribeResponse::Accepted {
        current_index: acceptance.current_committed_index,
        node_id: acceptance.node_id,
    };
    let response_bytes = postcard::to_stdvec(&response).context("failed to serialize subscribe response")?;
    send.write_all(&response_bytes).await.context("failed to send subscribe response")?;

    debug!(
        subscriber_id = acceptance.subscriber_id,
        current_committed_index = acceptance.current_committed_index,
        "subscription accepted"
    );
    Ok(())
}

struct ReplayBatchInput<'a> {
    subscriber_id: u64,
    key_prefix: &'a [u8],
    entries: Vec<LogEntryPayload>,
    current_start: &'a mut u64,
    total_entries_replayed: &'a mut u64,
    send: &'a mut iroh::endpoint::SendStream,
}

async fn handle_log_subscriber_replay_batch(input: ReplayBatchInput<'_>) -> anyhow::Result<()> {
    let ReplayBatchInput {
        subscriber_id,
        key_prefix,
        entries,
        current_start,
        total_entries_replayed,
        send,
    } = input;
    debug_assert!(subscriber_id > 0, "subscriber_id must be positive during replay batch");
    debug_assert!(entries.len() <= MAX_HISTORICAL_BATCH_SIZE as usize, "historical replay batch must stay bounded");

    for entry in entries {
        *current_start = entry.index.saturating_add(1);
        if !matches_subscriber_filter(key_prefix, &entry.operation) {
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
            debug!(subscriber_id = subscriber_id, error = %err, "subscriber disconnected during replay");
            return Err(anyhow::anyhow!("subscriber disconnected during replay"));
        }
        *total_entries_replayed = total_entries_replayed.saturating_add(1);
    }

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
    debug_assert!(subscriber_id > 0, "subscriber_id must be positive during replay");
    const { assert!(MAX_HISTORICAL_BATCH_SIZE > 0, "historical batch size must be positive") };
    if !should_replay_historical_entries(ReplayRange {
        start_index: sub_request.start_index,
        committed_index: current_committed_index,
    }) {
        return Ok(());
    }

    let reader = match historical_reader {
        Some(reader) => reader,
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
    let mut total_entries_replayed = 0u64;
    while current_start <= current_committed_index {
        let batch_end = historical_batch_end(ReplayRange {
            start_index: current_start,
            committed_index: current_committed_index,
        });
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
                handle_log_subscriber_replay_batch(ReplayBatchInput {
                    subscriber_id,
                    key_prefix: &sub_request.key_prefix,
                    entries,
                    current_start: &mut current_start,
                    total_entries_replayed: &mut total_entries_replayed,
                    send,
                })
                .await?;
            }
            Err(err) => {
                warn!(subscriber_id = subscriber_id, error = %err, "failed to read historical entries, continuing with live stream");
                break;
            }
        }
    }

    info!(
        subscriber_id = subscriber_id,
        total_entries_replayed = total_entries_replayed,
        "historical replay complete"
    );
    Ok(())
}

/// Stream live log entries to subscriber.
async fn handle_log_subscriber_stream(
    subscriber_id: u64,
    key_prefix: Vec<u8>,
    mut log_receiver: broadcast::Receiver<LogEntryPayload>,
    keepalive_context: KeepaliveContext<'_>,
    send: &mut iroh::endpoint::SendStream,
) {
    debug_assert!(subscriber_id > 0, "subscriber_id must be positive during live stream");
    debug_assert!(SUBSCRIBE_KEEPALIVE_INTERVAL > Duration::ZERO, "keepalive interval must be positive");
    let mut keepalive_timer = tokio::time::interval(SUBSCRIBE_KEEPALIVE_INTERVAL);
    keepalive_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            entry_result = log_receiver.recv() => {
                match entry_result {
                    Ok(entry) => {
                        if !matches_subscriber_filter(&key_prefix, &entry.operation) {
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

            _ = keepalive_timer.tick() => {
                let keepalive = LogEntryMessage::Keepalive {
                    committed_index: keepalive_context.committed_index.load(Ordering::Acquire),
                    hlc_timestamp: SerializableTimestamp::from(keepalive_context.hlc.new_timestamp()),
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
