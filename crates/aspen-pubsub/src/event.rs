//! Event type for pub/sub messages.
//!
//! Events contain a topic, payload, cursor, timestamp, and optional headers.

use std::collections::HashMap;

use aspen_core::SerializableTimestamp;
use serde::Deserialize;
use serde::Serialize;

use crate::constants::MAX_HEADER_KEY_SIZE;
use crate::constants::MAX_HEADER_VALUE_SIZE;
use crate::constants::MAX_HEADERS;
use crate::constants::MAX_PAYLOAD_SIZE;
use crate::cursor::Cursor;
use crate::error::HeaderKeyTooLongSnafu;
use crate::error::HeaderValueTooLargeSnafu;
use crate::error::PayloadTooLargeSnafu;
use crate::error::Result;
use crate::error::TooManyHeadersSnafu;
use crate::topic::Topic;

/// An event published to a topic.
///
/// Events are the fundamental unit of data in pub/sub. Each event contains:
/// - **topic**: The hierarchical topic name
/// - **payload**: The event data (application-defined)
/// - **cursor**: Position in the global event stream (Raft log index)
/// - **timestamp**: HLC timestamp for ordering
/// - **headers**: Optional metadata (trace IDs, content type, etc.)
///
/// # Examples
///
/// ```
/// use aspen_pubsub::{Event, Topic, Cursor};
/// use std::collections::HashMap;
///
/// let event = Event::new(
///     Topic::new("orders.created").unwrap(),
///     b"order data".to_vec(),
///     Cursor::from_index(12345),
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// The topic this event was published to.
    pub topic: Topic,

    /// The event payload (application data).
    pub payload: Vec<u8>,

    /// Position in the global event stream.
    pub cursor: Cursor,

    /// HLC timestamp when the event was committed.
    pub timestamp: SerializableTimestamp,

    /// Optional headers (metadata).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
}

impl Event {
    /// Create a new event without headers.
    ///
    /// Creates an event with a placeholder timestamp. The actual timestamp
    /// is set when the event is committed through Raft.
    pub fn new(topic: Topic, payload: Vec<u8>, cursor: Cursor) -> Self {
        // Create a placeholder timestamp (will be overwritten by Raft)
        let hlc = aspen_core::create_hlc("placeholder");
        let timestamp = SerializableTimestamp::from(hlc.new_timestamp());
        Self {
            topic,
            payload,
            cursor,
            timestamp,
            headers: HashMap::new(),
        }
    }

    /// Create a new event with headers.
    pub fn with_headers(
        topic: Topic,
        payload: Vec<u8>,
        cursor: Cursor,
        timestamp: SerializableTimestamp,
        headers: HashMap<String, String>,
    ) -> Self {
        Self {
            topic,
            payload,
            cursor,
            timestamp,
            headers,
        }
    }

    /// Get the size of the payload in bytes.
    pub fn payload_size(&self) -> usize {
        self.payload.len()
    }

    /// Get the number of headers.
    pub fn header_count(&self) -> usize {
        self.headers.len()
    }

    /// Get a header value by key.
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(String::as_str)
    }

    /// Validate the event against Tiger Style limits.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Payload exceeds `MAX_PAYLOAD_SIZE`
    /// - Too many headers (> `MAX_HEADERS`)
    /// - Header key exceeds `MAX_HEADER_KEY_SIZE`
    /// - Header value exceeds `MAX_HEADER_VALUE_SIZE`
    pub fn validate(&self) -> Result<()> {
        validate_payload(&self.payload)?;
        validate_headers(&self.headers)?;
        Ok(())
    }
}

/// Validate a payload against size limits.
pub fn validate_payload(payload: &[u8]) -> Result<()> {
    if payload.len() > MAX_PAYLOAD_SIZE {
        return PayloadTooLargeSnafu { size: payload.len() }.fail();
    }
    Ok(())
}

/// Validate headers against limits.
pub fn validate_headers(headers: &HashMap<String, String>) -> Result<()> {
    if headers.len() > MAX_HEADERS {
        return TooManyHeadersSnafu { count: headers.len() }.fail();
    }

    for (key, value) in headers {
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

/// Builder for creating events with fluent API.
///
/// # Examples
///
/// ```
/// use aspen_pubsub::{EventBuilder, Topic};
///
/// let event = EventBuilder::new(Topic::new("orders.created").unwrap())
///     .payload(b"order data")
///     .header("trace-id", "abc123")
///     .header("content-type", "application/json")
///     .build()
///     .unwrap();
/// ```
pub struct EventBuilder {
    topic: Topic,
    payload: Vec<u8>,
    headers: HashMap<String, String>,
}

impl EventBuilder {
    /// Create a new event builder.
    pub fn new(topic: Topic) -> Self {
        Self {
            topic,
            payload: Vec::new(),
            headers: HashMap::new(),
        }
    }

    /// Set the payload.
    pub fn payload(mut self, payload: impl Into<Vec<u8>>) -> Self {
        self.payload = payload.into();
        self
    }

    /// Add a header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add multiple headers.
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Build the event, validating against limits.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn build(self) -> Result<Event> {
        validate_payload(&self.payload)?;
        validate_headers(&self.headers)?;

        Ok(Event {
            topic: self.topic,
            payload: self.payload,
            cursor: Cursor::BEGINNING, // Set by publisher
            timestamp: SerializableTimestamp::from(aspen_core::create_hlc("builder").new_timestamp()),
            headers: self.headers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PubSubError;

    fn test_timestamp() -> SerializableTimestamp {
        SerializableTimestamp::from(aspen_core::create_hlc("test").new_timestamp())
    }

    #[test]
    fn test_event_new() {
        let topic = Topic::new("orders.created").unwrap();
        let event = Event::new(topic.clone(), b"test".to_vec(), Cursor::from_index(100));

        assert_eq!(event.topic, topic);
        assert_eq!(event.payload, b"test");
        assert_eq!(event.cursor.index(), 100);
        assert!(event.headers.is_empty());
    }

    #[test]
    fn test_event_with_headers() {
        let topic = Topic::new("orders.created").unwrap();
        let mut headers = HashMap::new();
        headers.insert("trace-id".to_string(), "abc123".to_string());

        let event = Event::with_headers(topic, b"test".to_vec(), Cursor::from_index(100), test_timestamp(), headers);

        assert_eq!(event.header("trace-id"), Some("abc123"));
        assert_eq!(event.header_count(), 1);
    }

    #[test]
    fn test_event_validation_payload_size() {
        let topic = Topic::new("test").unwrap();
        let large_payload = vec![0u8; MAX_PAYLOAD_SIZE + 1];
        let event = Event::new(topic, large_payload, Cursor::BEGINNING);

        assert!(matches!(event.validate(), Err(PubSubError::PayloadTooLarge { .. })));
    }

    #[test]
    fn test_event_validation_too_many_headers() {
        let topic = Topic::new("test").unwrap();
        let mut headers = HashMap::new();
        for i in 0..MAX_HEADERS + 1 {
            headers.insert(format!("key{i}"), "value".to_string());
        }

        let event = Event::with_headers(topic, Vec::new(), Cursor::BEGINNING, test_timestamp(), headers);

        assert!(matches!(event.validate(), Err(PubSubError::TooManyHeaders { .. })));
    }

    #[test]
    fn test_event_validation_header_key_too_long() {
        let topic = Topic::new("test").unwrap();
        let mut headers = HashMap::new();
        let long_key = "x".repeat(MAX_HEADER_KEY_SIZE + 1);
        headers.insert(long_key, "value".to_string());

        let event = Event::with_headers(topic, Vec::new(), Cursor::BEGINNING, test_timestamp(), headers);

        assert!(matches!(event.validate(), Err(PubSubError::HeaderKeyTooLong { .. })));
    }

    #[test]
    fn test_event_validation_header_value_too_large() {
        let topic = Topic::new("test").unwrap();
        let mut headers = HashMap::new();
        let large_value = "x".repeat(MAX_HEADER_VALUE_SIZE + 1);
        headers.insert("key".to_string(), large_value);

        let event = Event::with_headers(topic, Vec::new(), Cursor::BEGINNING, test_timestamp(), headers);

        assert!(matches!(event.validate(), Err(PubSubError::HeaderValueTooLarge { .. })));
    }

    #[test]
    fn test_event_builder() {
        let topic = Topic::new("orders.created").unwrap();
        let event = EventBuilder::new(topic)
            .payload(b"test data".to_vec())
            .header("trace-id", "abc123")
            .header("content-type", "application/json")
            .build()
            .unwrap();

        assert_eq!(event.payload, b"test data");
        assert_eq!(event.header_count(), 2);
        assert_eq!(event.header("trace-id"), Some("abc123"));
    }

    #[test]
    fn test_event_serialization() {
        let topic = Topic::new("test").unwrap();
        let event = Event::new(topic, b"payload".to_vec(), Cursor::from_index(100));

        let json = serde_json::to_string(&event).unwrap();
        let decoded: Event = serde_json::from_str(&json).unwrap();

        assert_eq!(event.topic, decoded.topic);
        assert_eq!(event.payload, decoded.payload);
        assert_eq!(event.cursor, decoded.cursor);
    }
}
