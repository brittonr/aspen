//! Hook event types.
//!
//! Defines the event types that can trigger hooks and the payload structure.

use aspen_hlc::SerializableTimestamp;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::HOOK_TOPIC_PREFIX;
use crate::error::HookTypeError;
use crate::error::Result;

/// Types of events that can trigger hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEventType {
    /// KV write operation committed.
    WriteCommitted,
    /// KV delete operation committed.
    DeleteCommitted,
    /// New leader elected in Raft consensus.
    LeaderElected,
    /// Cluster membership changed (voters/learners added or removed).
    MembershipChanged,
    /// Node added to cluster.
    NodeAdded,
    /// Node removed from cluster.
    NodeRemoved,
    /// Snapshot created.
    SnapshotCreated,
    /// Snapshot installed (restored).
    SnapshotInstalled,
    /// Node health status changed.
    HealthChanged,
    /// TTL expiration on a key.
    TtlExpired,

    // Blob events
    /// Blob added to store (via add_bytes, add_path, or download).
    BlobAdded,
    /// Blob deleted (GC protection removed, eligible for cleanup).
    BlobDeleted,
    /// Blob downloaded from remote peer.
    BlobDownloaded,
    /// Blob protected from garbage collection.
    BlobProtected,
    /// Blob unprotected (GC protection tag removed).
    BlobUnprotected,

    // Docs events
    /// Docs sync session started with peer cluster.
    DocsSyncStarted,
    /// Docs sync session completed with peer cluster.
    DocsSyncCompleted,
    /// Remote entry imported from peer cluster to local KV.
    DocsEntryImported,
    /// Batch of entries exported to docs namespace.
    DocsEntryExported,
}

impl HookEventType {
    /// Get the topic suffix for this event type.
    ///
    /// Used to construct full topic names like "hooks.kv.write_committed".
    pub fn topic_suffix(&self) -> &'static str {
        match self {
            HookEventType::WriteCommitted => "kv.write_committed",
            HookEventType::DeleteCommitted => "kv.delete_committed",
            HookEventType::LeaderElected => "cluster.leader_elected",
            HookEventType::MembershipChanged => "cluster.membership_changed",
            HookEventType::NodeAdded => "cluster.node_added",
            HookEventType::NodeRemoved => "cluster.node_removed",
            HookEventType::SnapshotCreated => "system.snapshot_created",
            HookEventType::SnapshotInstalled => "system.snapshot_installed",
            HookEventType::HealthChanged => "system.health_changed",
            HookEventType::TtlExpired => "kv.ttl_expired",
            // Blob events
            HookEventType::BlobAdded => "blob.blob_added",
            HookEventType::BlobDeleted => "blob.blob_deleted",
            HookEventType::BlobDownloaded => "blob.blob_downloaded",
            HookEventType::BlobProtected => "blob.blob_protected",
            HookEventType::BlobUnprotected => "blob.blob_unprotected",
            // Docs events
            HookEventType::DocsSyncStarted => "docs.sync_started",
            HookEventType::DocsSyncCompleted => "docs.sync_completed",
            HookEventType::DocsEntryImported => "docs.entry_imported",
            HookEventType::DocsEntryExported => "docs.entry_exported",
        }
    }

    /// Get the full topic name with the hooks prefix.
    pub fn topic(&self) -> String {
        format!("{}.{}", HOOK_TOPIC_PREFIX, self.topic_suffix())
    }

    /// Get the event category (kv, cluster, system, blob, docs).
    pub fn category(&self) -> &'static str {
        match self {
            HookEventType::WriteCommitted | HookEventType::DeleteCommitted | HookEventType::TtlExpired => "kv",
            HookEventType::LeaderElected
            | HookEventType::MembershipChanged
            | HookEventType::NodeAdded
            | HookEventType::NodeRemoved => "cluster",
            HookEventType::SnapshotCreated | HookEventType::SnapshotInstalled | HookEventType::HealthChanged => {
                "system"
            }
            HookEventType::BlobAdded
            | HookEventType::BlobDeleted
            | HookEventType::BlobDownloaded
            | HookEventType::BlobProtected
            | HookEventType::BlobUnprotected => "blob",
            HookEventType::DocsSyncStarted
            | HookEventType::DocsSyncCompleted
            | HookEventType::DocsEntryImported
            | HookEventType::DocsEntryExported => "docs",
        }
    }
}

impl std::fmt::Display for HookEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.topic_suffix())
    }
}

/// A hook event that can trigger handler execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    /// Type of the event.
    pub event_type: HookEventType,

    /// Raft log index where this event was committed (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_index: Option<u64>,

    /// Timestamp when the event occurred.
    pub timestamp: SerializableTimestamp,

    /// Node ID where the event originated.
    pub node_id: u64,

    /// Event-specific payload data.
    pub payload: serde_json::Value,
}

impl HookEvent {
    /// Create a new hook event.
    pub fn new(event_type: HookEventType, node_id: u64, payload: serde_json::Value) -> Self {
        let hlc = aspen_hlc::create_hlc("hook-event");
        Self {
            event_type,
            log_index: None,
            timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
            node_id,
            payload,
        }
    }

    /// Create a hook event with a specific log index.
    pub fn with_log_index(mut self, log_index: u64) -> Self {
        self.log_index = Some(log_index);
        self
    }

    /// Create a hook event with a specific timestamp.
    pub fn with_timestamp(mut self, timestamp: SerializableTimestamp) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Get the full topic for this event.
    pub fn topic(&self) -> String {
        self.event_type.topic()
    }

    /// Serialize the event to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|source| HookTypeError::SerializationFailed { source })
    }

    /// Deserialize an event from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|source| HookTypeError::DeserializationFailed { source })
    }
}

/// Payload for KV write events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvWritePayload {
    /// The key that was written.
    pub key: String,
    /// The value that was written (may be truncated for large values).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Size of the value in bytes.
    pub value_size_bytes: u64,
    /// Whether this was a create or update operation.
    pub is_create: bool,
}

/// Payload for KV delete events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvDeletePayload {
    /// The key that was deleted.
    pub key: String,
    /// Whether the key existed before deletion.
    pub existed: bool,
}

/// Payload for leader election events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElectedPayload {
    /// ID of the new leader.
    pub new_leader_id: u64,
    /// IDs of the previous leader (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_leader_id: Option<u64>,
    /// Current term.
    pub term: u64,
    /// Current voter node IDs.
    pub voters: Vec<u64>,
    /// Current learner node IDs.
    pub learners: Vec<u64>,
}

/// Payload for membership change events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipChangedPayload {
    /// Current voter node IDs.
    pub voters: Vec<u64>,
    /// Current learner node IDs.
    pub learners: Vec<u64>,
    /// Node ID that was affected (added, removed, or promoted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affected_node: Option<u64>,
    /// Type of change (added_voter, added_learner, removed, promoted).
    pub change_type: String,
}

/// Payload for snapshot events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotPayload {
    /// Log index of the snapshot.
    pub snapshot_index: u64,
    /// Term of the snapshot.
    pub term: u64,
    /// Number of entries in the snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_count: Option<u64>,
    /// Size of the snapshot in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

/// Payload for health change events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthChangedPayload {
    /// Previous health state.
    pub previous_state: String,
    /// Current health state.
    pub current_state: String,
    /// Reason for the health change.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Payload for TTL expiration events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtlExpiredPayload {
    /// The key that expired.
    pub key: String,
    /// When the TTL was set (if known).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_set_at: Option<SerializableTimestamp>,
}

// ============================================================================
// Blob Event Payloads
// ============================================================================

/// Payload for blob added events.
///
/// Includes the blob content inline if under the threshold (64KB),
/// otherwise provides a ticket for the handler to fetch the blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobAddedPayload {
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: String,
    /// Size of the blob in bytes.
    #[serde(rename = "size")]
    pub size_bytes: u64,
    /// Source of the blob (add_bytes, add_path, download).
    pub source: String,
    /// Base64-encoded blob content (for small blobs under threshold).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_base64: Option<String>,
    /// Ticket to fetch the blob (for large blobs over threshold).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_ticket: Option<String>,
}

/// Payload for blob deleted events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobDeletedPayload {
    /// BLAKE3 hash of the deleted blob (hex-encoded).
    pub hash: String,
    /// GC tag name that was removed.
    pub tag_name: String,
    /// Reason for deletion (gc, explicit, kv_overwrite, kv_delete).
    pub reason: String,
}

/// Payload for blob downloaded events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobDownloadedPayload {
    /// BLAKE3 hash of the blob (hex-encoded).
    pub hash: String,
    /// Size of the blob in bytes.
    #[serde(rename = "size")]
    pub size_bytes: u64,
    /// Public key of the provider peer (hex-encoded).
    pub provider_id: String,
    /// Download duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Base64-encoded blob content (for small blobs under threshold).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_base64: Option<String>,
    /// Ticket to fetch the blob (for large blobs over threshold).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_ticket: Option<String>,
}

/// Payload for blob protected events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobProtectedPayload {
    /// BLAKE3 hash of the protected blob (hex-encoded).
    pub hash: String,
    /// GC tag name that was created.
    pub tag_name: String,
}

/// Payload for blob unprotected events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobUnprotectedPayload {
    /// BLAKE3 hash of the unprotected blob (hex-encoded), if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// GC tag name that was removed.
    pub tag_name: String,
    /// Reason for unprotection (kv_delete, kv_overwrite, user_action).
    pub reason: String,
    /// Grace period in seconds before blob may be garbage collected.
    pub grace_period_secs: u64,
}

// ============================================================================
// Docs Event Payloads
// ============================================================================

/// Payload for docs sync started events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsSyncStartedPayload {
    /// ID of the peer cluster being synced.
    pub cluster_id: String,
    /// Number of peers connected for this cluster.
    pub peer_count: u32,
}

/// Payload for docs sync completed events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsSyncCompletedPayload {
    /// ID of the peer cluster that was synced.
    pub cluster_id: String,
    /// Number of entries synced in this session.
    pub entries_synced: u64,
    /// Duration of the sync session in milliseconds.
    pub duration_ms: u64,
    /// Whether the sync completed successfully.
    pub is_success: bool,
    /// Error message if sync failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Payload for docs entry imported events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsEntryImportedPayload {
    /// Key that was imported.
    pub key: String,
    /// ID of the source cluster.
    pub source_cluster: String,
    /// Priority of the import (lower wins in conflicts).
    pub priority: u32,
    /// Size of the imported value in bytes.
    #[serde(rename = "value_size")]
    pub value_size_bytes: u64,
    /// Whether this was a new key or an update.
    pub is_update: bool,
}

/// Payload for docs entry exported events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsEntryExportedPayload {
    /// Number of entries in the exported batch.
    #[serde(rename = "batch_size")]
    pub batch_entry_count: u32,
    /// Total entries exported so far.
    pub total_exported: u64,
    /// Duration to flush the batch in milliseconds.
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_topics() {
        assert_eq!(HookEventType::WriteCommitted.topic(), "hooks.kv.write_committed");
        assert_eq!(HookEventType::LeaderElected.topic(), "hooks.cluster.leader_elected");
        assert_eq!(HookEventType::SnapshotCreated.topic(), "hooks.system.snapshot_created");
        // Blob events
        assert_eq!(HookEventType::BlobAdded.topic(), "hooks.blob.blob_added");
        assert_eq!(HookEventType::BlobDownloaded.topic(), "hooks.blob.blob_downloaded");
        // Docs events
        assert_eq!(HookEventType::DocsSyncStarted.topic(), "hooks.docs.sync_started");
        assert_eq!(HookEventType::DocsEntryImported.topic(), "hooks.docs.entry_imported");
    }

    #[test]
    fn test_event_type_categories() {
        assert_eq!(HookEventType::WriteCommitted.category(), "kv");
        assert_eq!(HookEventType::DeleteCommitted.category(), "kv");
        assert_eq!(HookEventType::LeaderElected.category(), "cluster");
        assert_eq!(HookEventType::MembershipChanged.category(), "cluster");
        assert_eq!(HookEventType::SnapshotCreated.category(), "system");
        assert_eq!(HookEventType::HealthChanged.category(), "system");
        // Blob events
        assert_eq!(HookEventType::BlobAdded.category(), "blob");
        assert_eq!(HookEventType::BlobDeleted.category(), "blob");
        assert_eq!(HookEventType::BlobDownloaded.category(), "blob");
        assert_eq!(HookEventType::BlobProtected.category(), "blob");
        assert_eq!(HookEventType::BlobUnprotected.category(), "blob");
        // Docs events
        assert_eq!(HookEventType::DocsSyncStarted.category(), "docs");
        assert_eq!(HookEventType::DocsSyncCompleted.category(), "docs");
        assert_eq!(HookEventType::DocsEntryImported.category(), "docs");
        assert_eq!(HookEventType::DocsEntryExported.category(), "docs");
    }

    #[test]
    fn test_hook_event_creation() {
        let payload = serde_json::json!({"key": "test", "value": "data"});
        let event = HookEvent::new(HookEventType::WriteCommitted, 1, payload.clone());

        assert_eq!(event.event_type, HookEventType::WriteCommitted);
        assert_eq!(event.node_id, 1);
        assert_eq!(event.payload, payload);
        assert!(event.log_index.is_none());
    }

    #[test]
    fn test_hook_event_with_log_index() {
        let event = HookEvent::new(HookEventType::WriteCommitted, 1, serde_json::json!({})).with_log_index(100);

        assert_eq!(event.log_index, Some(100));
    }

    #[test]
    fn test_hook_event_serialization() {
        let event = HookEvent::new(HookEventType::LeaderElected, 1, serde_json::json!({"leader": 2}));

        let bytes = event.to_bytes().unwrap();
        let decoded = HookEvent::from_bytes(&bytes).unwrap();

        assert_eq!(event.event_type, decoded.event_type);
        assert_eq!(event.node_id, decoded.node_id);
        assert_eq!(event.payload, decoded.payload);
    }

    #[test]
    fn test_payload_types() {
        let write_payload = KvWritePayload {
            key: "test/key".to_string(),
            value: Some("value".to_string()),
            value_size_bytes: 5,
            is_create: true,
        };
        let json = serde_json::to_value(&write_payload).unwrap();
        assert_eq!(json["key"], "test/key");
        assert_eq!(json["is_create"], true);

        let leader_payload = LeaderElectedPayload {
            new_leader_id: 2,
            previous_leader_id: Some(1),
            term: 5,
            voters: vec![1, 2, 3],
            learners: vec![4],
        };
        let json = serde_json::to_value(&leader_payload).unwrap();
        assert_eq!(json["new_leader_id"], 2);
        assert_eq!(json["term"], 5);
    }

    #[test]
    fn test_blob_payload_types() {
        // BlobAdded with inline content
        let blob_added = BlobAddedPayload {
            hash: "abc123".to_string(),
            size_bytes: 1024,
            source: "add_bytes".to_string(),
            content_base64: Some("SGVsbG8gV29ybGQ=".to_string()),
            blob_ticket: None,
        };
        let json = serde_json::to_value(&blob_added).unwrap();
        assert_eq!(json["hash"], "abc123");
        assert_eq!(json["size"], 1024);
        assert_eq!(json["source"], "add_bytes");
        assert!(json.get("content_base64").is_some());
        assert!(json.get("blob_ticket").is_none());

        // BlobAdded with ticket (large blob)
        let blob_added_large = BlobAddedPayload {
            hash: "def456".to_string(),
            size_bytes: 1_000_000,
            source: "add_path".to_string(),
            content_base64: None,
            blob_ticket: Some("aspenblob...".to_string()),
        };
        let json = serde_json::to_value(&blob_added_large).unwrap();
        assert!(json.get("content_base64").is_none());
        assert!(json.get("blob_ticket").is_some());

        // BlobDownloaded
        let blob_downloaded = BlobDownloadedPayload {
            hash: "ghi789".to_string(),
            size_bytes: 2048,
            provider_id: "peer123".to_string(),
            duration_ms: Some(150),
            content_base64: Some("dGVzdA==".to_string()),
            blob_ticket: None,
        };
        let json = serde_json::to_value(&blob_downloaded).unwrap();
        assert_eq!(json["provider_id"], "peer123");
        assert_eq!(json["duration_ms"], 150);
    }

    #[test]
    fn test_docs_payload_types() {
        // DocsSyncCompleted
        let sync_completed = DocsSyncCompletedPayload {
            cluster_id: "cluster-1".to_string(),
            entries_synced: 100,
            duration_ms: 500,
            is_success: true,
            error: None,
        };
        let json = serde_json::to_value(&sync_completed).unwrap();
        assert_eq!(json["cluster_id"], "cluster-1");
        assert_eq!(json["entries_synced"], 100);
        assert_eq!(json["is_success"], true);
        assert!(json.get("error").is_none());

        // DocsEntryImported
        let entry_imported = DocsEntryImportedPayload {
            key: "user/123".to_string(),
            source_cluster: "remote-cluster".to_string(),
            priority: 1,
            value_size_bytes: 256,
            is_update: false,
        };
        let json = serde_json::to_value(&entry_imported).unwrap();
        assert_eq!(json["key"], "user/123");
        assert_eq!(json["priority"], 1);
        assert_eq!(json["is_update"], false);
    }
}
