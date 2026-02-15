//! Log subscriber protocol handler.

use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use iroh::protocol::ProtocolHandler;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::connection::handle_log_subscriber_connection;
use super::constants::LOG_BROADCAST_BUFFER_SIZE;
use super::constants::MAX_LOG_SUBSCRIBERS;
use super::types::HistoricalLogReader;
use super::types::LogEntryPayload;
// TODO: SECURITY - Migrate to proper aspen_raft::auth::AuthContext implementation
// The current AuthContext in crate::rpc is INSECURE (only checks for non-empty fields)
// See https://github.com/aspen/aspen/issues/XXX for tracking
#[allow(deprecated)]
use crate::rpc::AuthContext;

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
#[allow(deprecated)] // TODO: Migrate to proper auth implementation
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
    /// Optional watch registry for tracking active subscriptions.
    watch_registry: Option<Arc<dyn aspen_core::WatchRegistry>>,
}

impl std::fmt::Debug for LogSubscriberProtocolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogSubscriberProtocolHandler")
            .field("node_id", &self.node_id)
            .field("committed_index", &self.committed_index)
            .finish_non_exhaustive()
    }
}

#[allow(deprecated)] // TODO: Migrate to proper auth implementation
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
        let (log_sender, _) = broadcast::channel(LOG_BROADCAST_BUFFER_SIZE);
        let committed_index = Arc::new(AtomicU64::new(0));
        let hlc = aspen_core::hlc::create_hlc(&node_id.to_string());
        let handler = Self {
            auth_context: AuthContext::new(cluster_cookie.to_string()),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender: log_sender.clone(),
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index: committed_index.clone(),
            historical_reader: None,
            hlc,
            watch_registry: None,
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
            auth_context: AuthContext::new(cluster_cookie.to_string()),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender,
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index,
            historical_reader: None,
            hlc,
            watch_registry: None,
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
            auth_context: AuthContext::new(cluster_cookie.to_string()),
            connection_semaphore: Arc::new(Semaphore::new(MAX_LOG_SUBSCRIBERS)),
            log_sender,
            node_id,
            next_subscriber_id: AtomicU64::new(1),
            committed_index,
            historical_reader: Some(historical_reader),
            hlc,
            watch_registry: None,
        }
    }

    /// Set the watch registry for tracking active subscriptions.
    ///
    /// When set, active subscriptions are registered with the watch registry,
    /// enabling `WatchStatus` queries through the RPC interface.
    ///
    /// # Arguments
    /// * `registry` - The watch registry to use for tracking
    pub fn with_watch_registry(mut self, registry: Arc<dyn aspen_core::WatchRegistry>) -> Self {
        self.watch_registry = Some(registry);
        self
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
            self.watch_registry.clone(),
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
