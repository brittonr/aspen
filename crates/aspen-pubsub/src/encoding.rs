//! MessagePack encoding for pub/sub events.
//!
//! Events are stored in the KV store as MessagePack-encoded data.
//! This provides compact binary encoding while remaining schema-flexible.

use std::collections::HashMap;

use aspen_core::SerializableTimestamp;
use aspen_transport::log_subscriber::KvOperation;
use aspen_transport::log_subscriber::LogEntryPayload;
use serde::Deserialize;
use serde::Serialize;

use crate::cursor::Cursor;
use crate::error::PubSubError;
use crate::error::Result;
use crate::error::UnexpectedOperationSnafu;
use crate::event::Event;
use crate::keys::key_to_topic;
use crate::topic::Topic;

/// Internal representation of event data stored in the KV value.
///
/// This is the wire format stored in the Raft log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    /// Topic name (stored redundantly for easier debugging).
    pub topic: String,
    /// Event payload.
    pub payload: Vec<u8>,
    /// Optional headers.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
}

/// Encode an event for storage in the KV store.
///
/// Returns MessagePack-encoded bytes.
pub fn encode_event(topic: &Topic, payload: &[u8], headers: &HashMap<String, String>) -> Result<Vec<u8>> {
    let data = EventData {
        topic: topic.as_str().to_string(),
        payload: payload.to_vec(),
        headers: headers.clone(),
    };

    rmp_serde::to_vec(&data).map_err(Into::into)
}

/// Decode an event from a LogEntryPayload.
///
/// This is used by the subscriber to convert committed log entries
/// back into Event objects.
pub fn decode_event(payload: LogEntryPayload) -> Result<Event> {
    match payload.operation {
        KvOperation::Set { key, value } => {
            decode_from_set_operation(&key, &value, payload.index, payload.hlc_timestamp)
        }
        KvOperation::SetWithTTL { key, value, .. } => {
            // Treat TTL events the same as regular Set
            decode_from_set_operation(&key, &value, payload.index, payload.hlc_timestamp)
        }
        other => UnexpectedOperationSnafu {
            operation: format!("{other:?}"),
        }
        .fail(),
    }
}

/// Decode event from a Set operation's key and value.
fn decode_from_set_operation(key: &[u8], value: &[u8], index: u64, timestamp: SerializableTimestamp) -> Result<Event> {
    // Decode the MessagePack value
    let data: EventData = rmp_serde::from_slice(value)?;

    // Parse topic from stored data (more reliable than key parsing)
    let topic = Topic::new(&data.topic).map_err(|e| PubSubError::KeyParseFailed {
        reason: format!("invalid topic in event data: {e}"),
    })?;

    // Verify topic matches key (defensive check)
    if let Ok(key_topic) = key_to_topic(key)
        && key_topic != topic
    {
        tracing::warn!("topic mismatch: key={}, data={}", key_topic.as_str(), topic.as_str());
    }

    Ok(Event {
        topic,
        payload: data.payload,
        cursor: Cursor::from_index(index),
        timestamp,
        headers: data.headers,
    })
}

/// Check if a KvOperation is a pub/sub event.
///
/// This is used by the subscriber to filter out non-pub/sub operations.
pub fn is_pubsub_operation(operation: &KvOperation) -> bool {
    match operation {
        KvOperation::Set { key, .. } | KvOperation::SetWithTTL { key, .. } => crate::keys::is_pubsub_key(key),
        _ => false,
    }
}

/// Try to decode an event from a KvOperation, returning None if not a pub/sub event.
pub fn try_decode_event(payload: LogEntryPayload) -> Option<Result<Event>> {
    if is_pubsub_operation(&payload.operation) {
        Some(decode_event(payload))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_timestamp() -> SerializableTimestamp {
        SerializableTimestamp::from(aspen_core::create_hlc("test").new_timestamp())
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let topic = Topic::new("orders.created").unwrap();
        let payload = b"test payload";
        let mut headers = HashMap::new();
        headers.insert("trace-id".to_string(), "abc123".to_string());

        let encoded = encode_event(&topic, payload, &headers).unwrap();

        // Decode back
        let data: EventData = rmp_serde::from_slice(&encoded).unwrap();
        assert_eq!(data.topic, "orders.created");
        assert_eq!(data.payload, payload);
        assert_eq!(data.headers.get("trace-id"), Some(&"abc123".to_string()));
    }

    #[test]
    fn test_encode_empty_headers() {
        let topic = Topic::new("test").unwrap();
        let encoded = encode_event(&topic, b"data", &HashMap::new()).unwrap();

        let data: EventData = rmp_serde::from_slice(&encoded).unwrap();
        assert!(data.headers.is_empty());
    }

    #[test]
    fn test_is_pubsub_operation() {
        use crate::keys::topic_to_key;

        let topic = Topic::new("orders.created").unwrap();
        let key = topic_to_key(&topic);

        // Pub/sub operation
        let op = KvOperation::Set {
            key: key.clone(),
            value: vec![],
        };
        assert!(is_pubsub_operation(&op));

        // Non-pub/sub operation
        let op = KvOperation::Set {
            key: b"some/other/key".to_vec(),
            value: vec![],
        };
        assert!(!is_pubsub_operation(&op));

        // Delete is not a pub/sub operation (we only care about Set)
        let op = KvOperation::Delete { key };
        assert!(!is_pubsub_operation(&op));
    }

    #[test]
    fn test_decode_event_from_log_entry() {
        use crate::keys::topic_to_key;

        let topic = Topic::new("orders.created").unwrap();
        let key = topic_to_key(&topic);
        let value = encode_event(&topic, b"payload", &HashMap::new()).unwrap();

        let log_entry = LogEntryPayload {
            index: 12345,
            term: 1,
            hlc_timestamp: test_timestamp(),
            operation: KvOperation::Set { key, value },
        };

        let event = decode_event(log_entry).unwrap();
        assert_eq!(event.topic, topic);
        assert_eq!(event.payload, b"payload");
        assert_eq!(event.cursor.index(), 12345);
    }

    #[test]
    fn test_decode_unexpected_operation() {
        let log_entry = LogEntryPayload {
            index: 1,
            term: 1,
            hlc_timestamp: test_timestamp(),
            operation: KvOperation::Delete { key: b"test".to_vec() },
        };

        let result = decode_event(log_entry);
        assert!(matches!(result, Err(PubSubError::UnexpectedOperation { .. })));
    }

    #[test]
    fn test_try_decode_event() {
        use crate::keys::topic_to_key;

        let topic = Topic::new("orders.created").unwrap();
        let key = topic_to_key(&topic);
        let value = encode_event(&topic, b"payload", &HashMap::new()).unwrap();

        // Pub/sub event
        let log_entry = LogEntryPayload {
            index: 1,
            term: 1,
            hlc_timestamp: test_timestamp(),
            operation: KvOperation::Set {
                key: key.clone(),
                value,
            },
        };
        assert!(try_decode_event(log_entry).is_some());

        // Non-pub/sub event
        let log_entry = LogEntryPayload {
            index: 1,
            term: 1,
            hlc_timestamp: test_timestamp(),
            operation: KvOperation::Set {
                key: b"user/data".to_vec(),
                value: vec![],
            },
        };
        assert!(try_decode_event(log_entry).is_none());
    }
}
