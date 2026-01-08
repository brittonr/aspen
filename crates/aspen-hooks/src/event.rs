//! Hook event types.
//!
//! Defines the event types that can trigger hooks and the payload structure.

use aspen_core::SerializableTimestamp;
use serde::Deserialize;
use serde::Serialize;

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
        }
    }

    /// Get the full topic name with the hooks prefix.
    pub fn topic(&self) -> String {
        format!("{}.{}", crate::constants::HOOK_TOPIC_PREFIX, self.topic_suffix())
    }

    /// Get the event category (kv, cluster, system).
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
        let hlc = aspen_core::create_hlc("hook-event");
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
    pub fn to_bytes(&self) -> crate::error::Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|source| crate::error::HookError::SerializationFailed { source })
    }

    /// Deserialize an event from JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> crate::error::Result<Self> {
        serde_json::from_slice(bytes).map_err(|source| crate::error::HookError::DeserializationFailed { source })
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
    pub value_size: usize,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_topics() {
        assert_eq!(HookEventType::WriteCommitted.topic(), "hooks.kv.write_committed");
        assert_eq!(HookEventType::LeaderElected.topic(), "hooks.cluster.leader_elected");
        assert_eq!(HookEventType::SnapshotCreated.topic(), "hooks.system.snapshot_created");
    }

    #[test]
    fn test_event_type_categories() {
        assert_eq!(HookEventType::WriteCommitted.category(), "kv");
        assert_eq!(HookEventType::DeleteCommitted.category(), "kv");
        assert_eq!(HookEventType::LeaderElected.category(), "cluster");
        assert_eq!(HookEventType::MembershipChanged.category(), "cluster");
        assert_eq!(HookEventType::SnapshotCreated.category(), "system");
        assert_eq!(HookEventType::HealthChanged.category(), "system");
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
            value_size: 5,
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
}
