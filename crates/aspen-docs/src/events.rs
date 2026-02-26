//! Docs event types for the hook system.
//!
//! This module provides event types and a broadcaster for emitting docs lifecycle
//! events that can be consumed by the hook system. Events are emitted for:
//!
//! - Sync started with a peer cluster
//! - Sync completed with a peer cluster
//! - Entry imported from a remote cluster
//! - Batch exported to docs namespace
//!
//! ## Architecture
//!
//! ```text
//! PeerManager / DocsExporter operations
//!        |
//!        v
//! DocsEventBroadcaster
//!        |
//!        v
//! broadcast::Sender<DocsEvent>
//!        |
//!        v
//! docs_bridge (in aspen-cluster)
//!        |
//!        v
//! HookService
//! ```
//!
//! ## Tiger Style Compliance
//!
//! - `DOCS_EVENT_BUFFER_SIZE`: 500 events (bounded backpressure)
//! - Non-blocking sends (lagged receivers drop events)

use std::time::Instant;

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;
use tracing::debug;

/// Buffer size for docs event broadcast channel.
/// Tiger Style: Bounded to prevent unbounded memory growth.
pub const DOCS_EVENT_BUFFER_SIZE: usize = 500;

/// Types of docs events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocsEventType {
    /// Sync session started with a peer cluster.
    SyncStarted,
    /// Sync session completed with a peer cluster.
    SyncCompleted,
    /// Entry imported from a remote cluster to local KV.
    EntryImported,
    /// Batch exported to docs namespace.
    EntryExported,
}

/// Result of an import operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportResultType {
    /// Entry was imported successfully.
    Imported,
    /// Entry was skipped due to priority.
    PrioritySkipped,
    /// Entry was filtered out.
    Filtered,
    /// Entry was skipped for other reasons.
    Skipped,
}

impl std::fmt::Display for ImportResultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportResultType::Imported => write!(f, "imported"),
            ImportResultType::PrioritySkipped => write!(f, "priority_skipped"),
            ImportResultType::Filtered => write!(f, "filtered"),
            ImportResultType::Skipped => write!(f, "skipped"),
        }
    }
}

/// A docs lifecycle event.
#[derive(Debug, Clone)]
pub struct DocsEvent {
    /// Type of event.
    pub event_type: DocsEventType,
    /// Cluster ID involved (peer cluster for sync, source cluster for import).
    pub cluster_id: String,
    /// Number of entries synced/imported/exported (for completed events).
    pub entry_count: Option<u64>,
    /// Duration of the operation in milliseconds (for completed events).
    pub duration_ms: Option<u64>,
    /// Key involved (for import events).
    pub key: Option<Vec<u8>>,
    /// Value size in bytes (for import events).
    pub value_size: Option<u64>,
    /// Priority of the source cluster (for import events).
    pub priority: Option<u32>,
    /// Import result (for import events).
    pub import_result: Option<ImportResultType>,
    /// Batch size (for export events).
    pub batch_size: Option<u32>,
    /// Number of connected peers (for sync started events).
    pub peer_count: Option<u32>,
    /// Timestamp when event was created.
    pub timestamp: Instant,
}

impl DocsEvent {
    /// Create a new sync started event.
    pub fn sync_started(cluster_id: impl Into<String>, peer_count: u32) -> Self {
        Self {
            event_type: DocsEventType::SyncStarted,
            cluster_id: cluster_id.into(),
            entry_count: None,
            duration_ms: None,
            key: None,
            value_size: None,
            priority: None,
            import_result: None,
            batch_size: None,
            peer_count: Some(peer_count),
            timestamp: Instant::now(),
        }
    }

    /// Create a new sync completed event.
    pub fn sync_completed(cluster_id: impl Into<String>, entries_synced: u64, duration_ms: u64) -> Self {
        Self {
            event_type: DocsEventType::SyncCompleted,
            cluster_id: cluster_id.into(),
            entry_count: Some(entries_synced),
            duration_ms: Some(duration_ms),
            key: None,
            value_size: None,
            priority: None,
            import_result: None,
            batch_size: None,
            peer_count: None,
            timestamp: Instant::now(),
        }
    }

    /// Create a new entry imported event.
    pub fn entry_imported(
        source_cluster: impl Into<String>,
        key: Vec<u8>,
        value_size: u64,
        priority: u32,
        result: ImportResultType,
    ) -> Self {
        Self {
            event_type: DocsEventType::EntryImported,
            cluster_id: source_cluster.into(),
            entry_count: None,
            duration_ms: None,
            key: Some(key),
            value_size: Some(value_size),
            priority: Some(priority),
            import_result: Some(result),
            batch_size: None,
            peer_count: None,
            timestamp: Instant::now(),
        }
    }

    /// Create a new entry exported event.
    pub fn entry_exported(batch_size: u32, entry_count: u64) -> Self {
        Self {
            event_type: DocsEventType::EntryExported,
            cluster_id: String::new(), // Not applicable for exports
            entry_count: Some(entry_count),
            duration_ms: None,
            key: None,
            value_size: None,
            priority: None,
            import_result: None,
            batch_size: Some(batch_size),
            peer_count: None,
            timestamp: Instant::now(),
        }
    }
}

/// Broadcaster for docs events.
///
/// Wraps a broadcast channel and provides convenience methods for emitting
/// docs lifecycle events.
pub struct DocsEventBroadcaster {
    sender: broadcast::Sender<DocsEvent>,
}

impl DocsEventBroadcaster {
    /// Create a new broadcaster with the given sender.
    pub fn new(sender: broadcast::Sender<DocsEvent>) -> Self {
        Self { sender }
    }

    /// Subscribe to docs events.
    pub fn subscribe(&self) -> broadcast::Receiver<DocsEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active receivers.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Emit a sync started event.
    pub fn emit_sync_started(&self, cluster_id: &str, peer_count: u32) {
        let event = DocsEvent::sync_started(cluster_id, peer_count);
        self.send(event);
    }

    /// Emit a sync completed event.
    pub fn emit_sync_completed(&self, cluster_id: &str, entries_synced: u64, duration_ms: u64) {
        let event = DocsEvent::sync_completed(cluster_id, entries_synced, duration_ms);
        self.send(event);
    }

    /// Emit an entry imported event.
    pub fn emit_entry_imported(
        &self,
        source_cluster: &str,
        key: &[u8],
        value_size: u64,
        priority: u32,
        result: ImportResultType,
    ) {
        let event = DocsEvent::entry_imported(source_cluster, key.to_vec(), value_size, priority, result);
        self.send(event);
    }

    /// Emit an entry exported event.
    pub fn emit_entry_exported(&self, batch_size: u32, entry_count: u64) {
        let event = DocsEvent::entry_exported(batch_size, entry_count);
        self.send(event);
    }

    /// Send an event to all subscribers.
    fn send(&self, event: DocsEvent) {
        // Non-blocking send - if no receivers, that's OK
        match self.sender.send(event) {
            Ok(count) => {
                debug!(receivers = count, "docs event sent");
            }
            Err(_) => {
                // No receivers - this is not an error, just means no one is listening
                debug!("docs event dropped (no receivers)");
            }
        }
    }
}

/// Create a docs event broadcast channel.
///
/// Returns the sender (for the broadcaster) and a receiver (for testing).
pub fn create_docs_event_channel() -> (broadcast::Sender<DocsEvent>, broadcast::Receiver<DocsEvent>) {
    broadcast::channel(DOCS_EVENT_BUFFER_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docs_event_sync_started() {
        let event = DocsEvent::sync_started("cluster-a", 3);

        assert_eq!(event.event_type, DocsEventType::SyncStarted);
        assert_eq!(event.cluster_id, "cluster-a");
        assert_eq!(event.peer_count, Some(3));
        assert!(event.entry_count.is_none());
    }

    #[test]
    fn test_docs_event_sync_completed() {
        let event = DocsEvent::sync_completed("cluster-a", 100, 5000);

        assert_eq!(event.event_type, DocsEventType::SyncCompleted);
        assert_eq!(event.cluster_id, "cluster-a");
        assert_eq!(event.entry_count, Some(100));
        assert_eq!(event.duration_ms, Some(5000));
    }

    #[test]
    fn test_docs_event_entry_imported() {
        let event = DocsEvent::entry_imported("cluster-b", b"test/key".to_vec(), 1024, 5, ImportResultType::Imported);

        assert_eq!(event.event_type, DocsEventType::EntryImported);
        assert_eq!(event.cluster_id, "cluster-b");
        assert_eq!(event.key, Some(b"test/key".to_vec()));
        assert_eq!(event.value_size, Some(1024));
        assert_eq!(event.priority, Some(5));
        assert_eq!(event.import_result, Some(ImportResultType::Imported));
    }

    #[test]
    fn test_docs_event_entry_exported() {
        let event = DocsEvent::entry_exported(50, 45);

        assert_eq!(event.event_type, DocsEventType::EntryExported);
        assert_eq!(event.batch_size, Some(50));
        assert_eq!(event.entry_count, Some(45));
        assert!(event.cluster_id.is_empty());
    }

    #[test]
    fn test_broadcaster_basic() {
        let (sender, mut receiver) = create_docs_event_channel();
        let broadcaster = DocsEventBroadcaster::new(sender);

        broadcaster.emit_sync_started("cluster-a", 2);

        let event = receiver.try_recv().unwrap();
        assert_eq!(event.event_type, DocsEventType::SyncStarted);
        assert_eq!(event.cluster_id, "cluster-a");
    }

    #[test]
    fn test_broadcaster_multiple_events() {
        let (sender, mut receiver) = create_docs_event_channel();
        let broadcaster = DocsEventBroadcaster::new(sender);

        broadcaster.emit_sync_started("cluster-a", 2);
        broadcaster.emit_entry_imported("cluster-b", b"key", 100, 5, ImportResultType::Imported);
        broadcaster.emit_sync_completed("cluster-a", 50, 1000);

        let e1 = receiver.try_recv().unwrap();
        assert_eq!(e1.event_type, DocsEventType::SyncStarted);

        let e2 = receiver.try_recv().unwrap();
        assert_eq!(e2.event_type, DocsEventType::EntryImported);

        let e3 = receiver.try_recv().unwrap();
        assert_eq!(e3.event_type, DocsEventType::SyncCompleted);
    }

    #[test]
    fn test_import_result_type_display() {
        assert_eq!(ImportResultType::Imported.to_string(), "imported");
        assert_eq!(ImportResultType::PrioritySkipped.to_string(), "priority_skipped");
        assert_eq!(ImportResultType::Filtered.to_string(), "filtered");
        assert_eq!(ImportResultType::Skipped.to_string(), "skipped");
    }
}
