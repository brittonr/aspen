//! Event type for pub/sub messages.
//!
//! Events are the core message type in the pub/sub system. Each event
//! has a topic, cursor (Raft log position), timestamp, payload, and
//! optional headers.

use serde::Deserialize;
use serde::Serialize;

use super::constants::MAX_HEADER_KEY_SIZE;
use super::constants::MAX_HEADER_VALUE_SIZE;
use super::constants::MAX_HEADERS;
use super::constants::MAX_PAYLOAD_SIZE;
use super::cursor::Cursor;
use super::error::HeaderKeyTooLongSnafu;
use super::error::HeaderValueTooLargeSnafu;
use super::error::PayloadTooLargeSnafu;
use super::error::Result;
use super::error::TooManyHeadersSnafu;
use super::topic::Topic;

/// A pub/sub event.
///
/// Events are published to topics and can be subscribed to by consumers.
/// Each event includes:
///
/// - **Topic**: The hierarchical topic name (e.g., "orders.created")
/// - **Cursor**: The Raft log index, providing global ordering
/// - **Timestamp**: Unix milliseconds when the event was published
/// - **Payload**: Binary payload (up to 1 MB)
/// - **Headers**: Optional key-value metadata (e.g., trace IDs)
///
/// # Example
///
/// ```
/// use aspen_hooks::pubsub::{Event, Topic, Cursor};
///
/// let event = Event {
///     topic: Topic::new("orders.created").unwrap(),
///     cursor: Cursor::from_index(12345),
///     timestamp_ms: 1704067200000,
///     payload: b"order data".to_vec(),
///     headers: vec![
///         ("trace-id".to_string(), b"abc123".to_vec()),
///     ],
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// The topic this event was published to.
    pub topic: Topic,

    /// The cursor (Raft log index) for this event.
    ///
    /// This provides global ordering across all topics and enables
    /// resumable subscriptions.
    pub cursor: Cursor,

    /// Unix timestamp in milliseconds when the event was published.
    pub timestamp_ms: u64,

    /// The event payload.
    ///
    /// Can contain any binary data up to `MAX_PAYLOAD_SIZE` bytes.
    pub payload: Vec<u8>,

    /// Optional headers (key-value metadata).
    ///
    /// Useful for trace IDs, content types, or other metadata.
    pub headers: Vec<(String, Vec<u8>)>,
}

impl Event {
    /// Validate the event against Tiger Style limits.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Payload exceeds `MAX_PAYLOAD_SIZE`
    /// - More than `MAX_HEADERS` headers
    /// - Header key exceeds `MAX_HEADER_KEY_SIZE`
    /// - Header value exceeds `MAX_HEADER_VALUE_SIZE`
    pub fn validate(&self) -> Result<()> {
        if self.payload.len() > MAX_PAYLOAD_SIZE {
            return PayloadTooLargeSnafu {
                size: self.payload.len(),
            }
            .fail();
        }

        if self.headers.len() > MAX_HEADERS {
            return TooManyHeadersSnafu {
                count: self.headers.len(),
            }
            .fail();
        }

        for (key, value) in &self.headers {
            if key.len() > MAX_HEADER_KEY_SIZE {
                return HeaderKeyTooLongSnafu {
                    key: key.clone(),
                    length: key.len(),
                }
                .fail();
            }
            if value.len() > MAX_HEADER_VALUE_SIZE {
                return HeaderValueTooLargeSnafu {
                    key: key.clone(),
                    size: value.len(),
                }
                .fail();
            }
        }

        Ok(())
    }

    /// Get a header value by key.
    pub fn header(&self, key: &str) -> Option<&[u8]> {
        self.headers.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_slice())
    }

    /// Get a header value as a UTF-8 string.
    pub fn header_str(&self, key: &str) -> Option<&str> {
        self.header(key).and_then(|v| std::str::from_utf8(v).ok())
    }
}

/// Builder for creating events with validation.
///
/// # Example
///
/// ```
/// use aspen_hooks::pubsub::{EventBuilder, Topic};
///
/// let event = EventBuilder::new(Topic::new("orders.created").unwrap())
///     .payload(b"order data".to_vec())
///     .header("trace-id", b"abc123".to_vec())
///     .build()
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct EventBuilder {
    topic: Topic,
    payload: Vec<u8>,
    headers: Vec<(String, Vec<u8>)>,
}

impl EventBuilder {
    /// Create a new event builder for the given topic.
    pub fn new(topic: Topic) -> Self {
        Self {
            topic,
            payload: Vec::new(),
            headers: Vec::new(),
        }
    }

    /// Set the event payload.
    pub fn payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self
    }

    /// Add a header.
    pub fn header(mut self, key: impl Into<String>, value: Vec<u8>) -> Self {
        self.headers.push((key.into(), value));
        self
    }

    /// Build the event with validation.
    ///
    /// The cursor and timestamp will be set to defaults (0).
    /// These are typically filled in by the publisher.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn build(self) -> Result<Event> {
        let event = Event {
            topic: self.topic,
            cursor: Cursor::BEGINNING,
            timestamp_ms: 0,
            payload: self.payload,
            headers: self.headers,
        };
        event.validate()?;
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_validation_valid() {
        let event = Event {
            topic: Topic::new("orders").unwrap(),
            cursor: Cursor::from_index(1),
            timestamp_ms: 1000,
            payload: vec![1, 2, 3],
            headers: vec![("key".to_string(), vec![4, 5, 6])],
        };

        assert!(event.validate().is_ok());
    }

    #[test]
    fn test_event_validation_payload_too_large() {
        let event = Event {
            topic: Topic::new("orders").unwrap(),
            cursor: Cursor::from_index(1),
            timestamp_ms: 1000,
            payload: vec![0; MAX_PAYLOAD_SIZE + 1],
            headers: vec![],
        };

        assert!(event.validate().is_err());
    }

    #[test]
    fn test_event_validation_too_many_headers() {
        let event = Event {
            topic: Topic::new("orders").unwrap(),
            cursor: Cursor::from_index(1),
            timestamp_ms: 1000,
            payload: vec![],
            headers: (0..MAX_HEADERS + 1).map(|i| (format!("key{}", i), vec![])).collect(),
        };

        assert!(event.validate().is_err());
    }

    #[test]
    fn test_event_header_access() {
        let event = Event {
            topic: Topic::new("orders").unwrap(),
            cursor: Cursor::from_index(1),
            timestamp_ms: 1000,
            payload: vec![],
            headers: vec![
                ("trace-id".to_string(), b"abc123".to_vec()),
                ("content-type".to_string(), b"application/json".to_vec()),
            ],
        };

        assert_eq!(event.header("trace-id"), Some(b"abc123".as_slice()));
        assert_eq!(event.header_str("content-type"), Some("application/json"));
        assert_eq!(event.header("missing"), None);
    }

    #[test]
    fn test_event_builder() {
        let event = EventBuilder::new(Topic::new("orders.created").unwrap())
            .payload(b"test".to_vec())
            .header("key1", b"value1".to_vec())
            .header("key2", b"value2".to_vec())
            .build()
            .unwrap();

        assert_eq!(event.topic.as_str(), "orders.created");
        assert_eq!(event.payload, b"test");
        assert_eq!(event.headers.len(), 2);
    }

    #[test]
    fn test_event_builder_validation() {
        let result = EventBuilder::new(Topic::new("orders").unwrap()).payload(vec![0; MAX_PAYLOAD_SIZE + 1]).build();

        assert!(result.is_err());
    }
}
