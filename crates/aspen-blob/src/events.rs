//! Blob event types for the hook system.
//!
//! This module provides event types and a broadcaster for emitting blob lifecycle
//! events that can be consumed by the hook system. Events are emitted for:
//!
//! - Blob added (via add_bytes, add_path, or download)
//! - Blob downloaded from remote peer
//! - Blob protected (GC tag created)
//! - Blob unprotected (GC tag removed)
//!
//! ## Architecture
//!
//! ```text
//! IrohBlobStore operations
//!        |
//!        v
//! BlobEventBroadcaster
//!        |
//!        v
//! broadcast::Sender<BlobEvent>
//!        |
//!        v
//! blob_bridge (in aspen-cluster)
//!        |
//!        v
//! HookService
//! ```
//!
//! ## Tiger Style Compliance
//!
//! - `BLOB_EVENT_BUFFER_SIZE`: 500 events (bounded backpressure)
//! - `INLINE_BLOB_THRESHOLD`: 64KB (content inlined vs ticket)
//! - Non-blocking sends (lagged receivers drop events)

use std::time::Instant;

use iroh_blobs::Hash;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;
use tracing::debug;

use crate::types::BlobRef;

/// Buffer size for blob event broadcast channel.
/// Tiger Style: Bounded to prevent unbounded memory growth.
pub const BLOB_EVENT_BUFFER_SIZE: u32 = 500;

/// Threshold for inlining blob content in events (64 KB).
/// Blobs smaller than this have their content included in the event payload.
/// Larger blobs include a ticket for the handler to fetch.
pub const INLINE_BLOB_THRESHOLD: u32 = 65_536;

/// Types of blob events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlobEventType {
    /// Blob added to store.
    Added,
    /// Blob downloaded from remote peer.
    Downloaded,
    /// Blob protected from garbage collection.
    Protected,
    /// Blob unprotected (GC tag removed).
    Unprotected,
}

/// Source of a blob addition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlobSource {
    /// Added via add_bytes().
    AddBytes,
    /// Added via add_path().
    AddPath,
    /// Added via download from remote peer.
    Download,
    /// Added via KV large value offloading.
    KvOffload,
}

impl std::fmt::Display for BlobSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlobSource::AddBytes => write!(f, "add_bytes"),
            BlobSource::AddPath => write!(f, "add_path"),
            BlobSource::Download => write!(f, "download"),
            BlobSource::KvOffload => write!(f, "kv_offload"),
        }
    }
}

/// Reason for blob unprotection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnprotectReason {
    /// KV entry was deleted.
    KvDelete,
    /// KV entry was overwritten with new value.
    KvOverwrite,
    /// User explicitly unprotected the blob.
    UserAction,
    /// Other reason (with description).
    Other(String),
}

impl std::fmt::Display for UnprotectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnprotectReason::KvDelete => write!(f, "kv_delete"),
            UnprotectReason::KvOverwrite => write!(f, "kv_overwrite"),
            UnprotectReason::UserAction => write!(f, "user_action"),
            UnprotectReason::Other(s) => write!(f, "{}", s),
        }
    }
}

/// A blob lifecycle event.
#[derive(Debug, Clone)]
pub struct BlobEvent {
    /// Type of event.
    pub event_type: BlobEventType,
    /// BLAKE3 hash of the blob.
    pub hash: Hash,
    /// Size of the blob in bytes.
    pub size_bytes: u64,
    /// Source of the blob (for Added events).
    pub source: Option<BlobSource>,
    /// Provider peer ID (for Downloaded events).
    pub provider_id: Option<String>,
    /// Download duration in milliseconds (for Downloaded events).
    pub duration_ms: Option<u64>,
    /// Tag name (for Protected/Unprotected events).
    pub tag_name: Option<String>,
    /// Reason for unprotection (for Unprotected events).
    pub unprotect_reason: Option<UnprotectReason>,
    /// Blob content (for small blobs under threshold).
    pub content: Option<Vec<u8>>,
    /// Whether the blob was new (not deduplicated).
    pub was_new: bool,
    /// Timestamp when event was created.
    pub timestamp: Instant,
}

impl BlobEvent {
    /// Create a new blob added event.
    pub fn added(hash: Hash, size_bytes: u64, source: BlobSource, content: Option<Vec<u8>>, was_new: bool) -> Self {
        Self {
            event_type: BlobEventType::Added,
            hash,
            size_bytes,
            source: Some(source),
            provider_id: None,
            duration_ms: None,
            tag_name: None,
            unprotect_reason: None,
            content,
            was_new,
            timestamp: Instant::now(),
        }
    }

    /// Create a new blob downloaded event.
    pub fn downloaded(
        hash: Hash,
        size_bytes: u64,
        provider_id: String,
        duration_ms: u64,
        content: Option<Vec<u8>>,
    ) -> Self {
        Self {
            event_type: BlobEventType::Downloaded,
            hash,
            size_bytes,
            source: Some(BlobSource::Download),
            provider_id: Some(provider_id),
            duration_ms: Some(duration_ms),
            tag_name: None,
            unprotect_reason: None,
            content,
            was_new: true,
            timestamp: Instant::now(),
        }
    }

    /// Create a new blob protected event.
    pub fn protected(hash: Hash, tag_name: String) -> Self {
        Self {
            event_type: BlobEventType::Protected,
            hash,
            size_bytes: 0,
            source: None,
            provider_id: None,
            duration_ms: None,
            tag_name: Some(tag_name),
            unprotect_reason: None,
            content: None,
            was_new: false,
            timestamp: Instant::now(),
        }
    }

    /// Create a new blob unprotected event.
    pub fn unprotected(hash: Option<Hash>, tag_name: String, reason: UnprotectReason) -> Self {
        Self {
            event_type: BlobEventType::Unprotected,
            hash: hash.unwrap_or_else(|| Hash::from_bytes([0u8; 32])),
            size_bytes: 0,
            source: None,
            provider_id: None,
            duration_ms: None,
            tag_name: Some(tag_name),
            unprotect_reason: Some(reason),
            content: None,
            was_new: false,
            timestamp: Instant::now(),
        }
    }
}

/// Broadcaster for blob events.
///
/// Wraps a broadcast channel and provides convenience methods for emitting
/// blob lifecycle events. The broadcaster decides whether to inline blob
/// content or generate a ticket based on size threshold.
pub struct BlobEventBroadcaster {
    sender: broadcast::Sender<BlobEvent>,
    inline_threshold: u32,
}

impl BlobEventBroadcaster {
    /// Create a new broadcaster with the given sender.
    pub fn new(sender: broadcast::Sender<BlobEvent>) -> Self {
        Self {
            sender,
            inline_threshold: INLINE_BLOB_THRESHOLD,
        }
    }

    /// Create a new broadcaster with a custom inline threshold.
    pub fn with_threshold(sender: broadcast::Sender<BlobEvent>, threshold: u32) -> Self {
        Self {
            sender,
            inline_threshold: threshold,
        }
    }

    /// Subscribe to blob events.
    pub fn subscribe(&self) -> broadcast::Receiver<BlobEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active receivers.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Emit a blob added event.
    ///
    /// Content is included if the blob is under the inline threshold.
    pub fn emit_added(&self, blob_ref: &BlobRef, source: BlobSource, data: &[u8], was_new: bool) {
        let content = if data.len() <= self.inline_threshold as usize {
            Some(data.to_vec())
        } else {
            None
        };

        let event = BlobEvent::added(blob_ref.hash, blob_ref.size_bytes, source, content, was_new);

        self.send(event);
    }

    /// Emit a blob downloaded event.
    ///
    /// Content is included if the blob is under the inline threshold.
    pub fn emit_downloaded(&self, blob_ref: &BlobRef, provider_id: &str, duration_ms: u64, data: Option<&[u8]>) {
        let content = data.and_then(|d| {
            if d.len() <= self.inline_threshold as usize {
                Some(d.to_vec())
            } else {
                None
            }
        });

        let event =
            BlobEvent::downloaded(blob_ref.hash, blob_ref.size_bytes, provider_id.to_string(), duration_ms, content);

        self.send(event);
    }

    /// Emit a blob protected event.
    pub fn emit_protected(&self, hash: &Hash, tag_name: &str) {
        let event = BlobEvent::protected(*hash, tag_name.to_string());
        self.send(event);
    }

    /// Emit a blob unprotected event.
    pub fn emit_unprotected(&self, hash: Option<&Hash>, tag_name: &str, reason: UnprotectReason) {
        let event = BlobEvent::unprotected(hash.copied(), tag_name.to_string(), reason);
        self.send(event);
    }

    /// Send an event to all subscribers.
    fn send(&self, event: BlobEvent) {
        // Non-blocking send - if no receivers, that's OK
        match self.sender.send(event) {
            Ok(count) => {
                debug!(receivers = count, "blob event sent");
            }
            Err(_) => {
                // No receivers - this is not an error, just means no one is listening
                debug!("blob event dropped (no receivers)");
            }
        }
    }
}

/// Create a blob event broadcast channel.
///
/// Returns the sender (for the broadcaster) and a receiver (for testing).
pub fn create_blob_event_channel() -> (broadcast::Sender<BlobEvent>, broadcast::Receiver<BlobEvent>) {
    broadcast::channel(BLOB_EVENT_BUFFER_SIZE as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_event_added() {
        let hash = Hash::from_bytes([1u8; 32]);
        let event = BlobEvent::added(hash, 1024, BlobSource::AddBytes, Some(vec![1, 2, 3]), true);

        assert_eq!(event.event_type, BlobEventType::Added);
        assert_eq!(event.hash, hash);
        assert_eq!(event.size_bytes, 1024);
        assert_eq!(event.source, Some(BlobSource::AddBytes));
        assert!(event.content.is_some());
        assert!(event.was_new);
    }

    #[test]
    fn test_blob_event_downloaded() {
        let hash = Hash::from_bytes([2u8; 32]);
        let event = BlobEvent::downloaded(hash, 2048, "peer123".to_string(), 150, None);

        assert_eq!(event.event_type, BlobEventType::Downloaded);
        assert_eq!(event.provider_id, Some("peer123".to_string()));
        assert_eq!(event.duration_ms, Some(150));
    }

    #[test]
    fn test_blob_event_protected() {
        let hash = Hash::from_bytes([3u8; 32]);
        let event = BlobEvent::protected(hash, "kv:test/key".to_string());

        assert_eq!(event.event_type, BlobEventType::Protected);
        assert_eq!(event.tag_name, Some("kv:test/key".to_string()));
    }

    #[test]
    fn test_blob_event_unprotected() {
        let hash = Hash::from_bytes([4u8; 32]);
        let event = BlobEvent::unprotected(Some(hash), "kv:test/key".to_string(), UnprotectReason::KvDelete);

        assert_eq!(event.event_type, BlobEventType::Unprotected);
        assert_eq!(event.unprotect_reason, Some(UnprotectReason::KvDelete));
    }

    #[test]
    fn test_broadcaster_inline_threshold() {
        let (sender, mut receiver) = create_blob_event_channel();
        let broadcaster = BlobEventBroadcaster::with_threshold(sender, 100);

        let hash = Hash::from_bytes([5u8; 32]);
        let blob_ref = BlobRef {
            hash,
            size_bytes: 50,
            format: iroh_blobs::BlobFormat::Raw,
        };

        // Small blob - should be inlined
        let small_data = vec![0u8; 50];
        broadcaster.emit_added(&blob_ref, BlobSource::AddBytes, &small_data, true);

        let event = receiver.try_recv().unwrap();
        assert!(event.content.is_some());
        assert_eq!(event.content.unwrap().len(), 50);

        // Large blob - should NOT be inlined
        let blob_ref_large = BlobRef {
            hash,
            size_bytes: 200,
            format: iroh_blobs::BlobFormat::Raw,
        };
        let large_data = vec![0u8; 200];
        broadcaster.emit_added(&blob_ref_large, BlobSource::AddBytes, &large_data, true);

        let event = receiver.try_recv().unwrap();
        assert!(event.content.is_none());
    }

    #[test]
    fn test_blob_source_display() {
        assert_eq!(BlobSource::AddBytes.to_string(), "add_bytes");
        assert_eq!(BlobSource::AddPath.to_string(), "add_path");
        assert_eq!(BlobSource::Download.to_string(), "download");
        assert_eq!(BlobSource::KvOffload.to_string(), "kv_offload");
    }

    #[test]
    fn test_unprotect_reason_display() {
        assert_eq!(UnprotectReason::KvDelete.to_string(), "kv_delete");
        assert_eq!(UnprotectReason::KvOverwrite.to_string(), "kv_overwrite");
        assert_eq!(UnprotectReason::UserAction.to_string(), "user_action");
        assert_eq!(UnprotectReason::Other("custom".to_string()).to_string(), "custom");
    }
}
