//! Publisher for pub/sub events.
//!
//! Publishers write events to topics through Raft consensus,
//! providing linearizable ordering across all topics.

use std::collections::HashMap;
use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use async_trait::async_trait;

use crate::constants::MAX_PUBLISH_BATCH_SIZE;
use crate::cursor::Cursor;
use crate::encoding::encode_event;
use crate::error::BatchTooLargeSnafu;
use crate::error::Result;
use crate::event::validate_headers;
use crate::event::validate_payload;
use crate::keys::key_to_string;
use crate::keys::topic_to_key;
use crate::topic::Topic;

/// Trait for publishing events to topics.
///
/// Publishers provide linearizable event ordering by writing
/// events through Raft consensus.
#[async_trait]
pub trait Publisher: Send + Sync {
    /// Publish an event to a topic.
    ///
    /// Returns the cursor (Raft log index) of the committed event.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Payload exceeds size limits
    /// - Raft consensus fails
    async fn publish(&self, topic: &Topic, payload: &[u8]) -> Result<Cursor>;

    /// Publish an event with headers.
    ///
    /// Headers are key-value metadata attached to the event.
    /// Common uses include trace IDs, content types, etc.
    async fn publish_with_headers(
        &self,
        topic: &Topic,
        payload: &[u8],
        headers: HashMap<String, String>,
    ) -> Result<Cursor>;

    /// Publish multiple events atomically.
    ///
    /// All events are committed together in a single Raft operation,
    /// ensuring atomic delivery (all or none).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Batch exceeds `MAX_PUBLISH_BATCH_SIZE`
    /// - Any payload exceeds size limits
    /// - Raft consensus fails
    async fn publish_batch(&self, events: Vec<(Topic, Vec<u8>)>) -> Result<Cursor>;
}

/// Publisher implementation using Raft consensus.
///
/// Events are written to the KV store with topic-encoded keys,
/// going through Raft for linearizable ordering.
///
/// # Examples
///
/// ```ignore
/// use aspen_pubsub::{RaftPublisher, Topic};
///
/// let publisher = RaftPublisher::new(kv_store);
///
/// let topic = Topic::new("orders.created")?;
/// let cursor = publisher.publish(&topic, b"order data").await?;
/// println!("Published at cursor {}", cursor);
/// ```
pub struct RaftPublisher<K: KeyValueStore> {
    kv_store: Arc<K>,
}

impl<K: KeyValueStore> RaftPublisher<K> {
    /// Create a new publisher wrapping a KeyValueStore.
    pub fn new(kv_store: Arc<K>) -> Self {
        Self { kv_store }
    }
}

#[async_trait]
impl<K: KeyValueStore + 'static> Publisher for RaftPublisher<K> {
    async fn publish(&self, topic: &Topic, payload: &[u8]) -> Result<Cursor> {
        self.publish_with_headers(topic, payload, HashMap::new()).await
    }

    async fn publish_with_headers(
        &self,
        topic: &Topic,
        payload: &[u8],
        headers: HashMap<String, String>,
    ) -> Result<Cursor> {
        // Validate inputs
        validate_payload(payload)?;
        validate_headers(&headers)?;

        // Encode the event
        let key = topic_to_key(topic);
        let value = encode_event(topic, payload, &headers)?;

        // Write through Raft consensus
        let result = self
            .kv_store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: key_to_string(&key),
                    value: String::from_utf8_lossy(&value).to_string(),
                },
            })
            .await?;

        // Use header_revision as the cursor (Raft revision)
        Ok(Cursor::from_index(result.header_revision.unwrap_or(0)))
    }

    async fn publish_batch(&self, events: Vec<(Topic, Vec<u8>)>) -> Result<Cursor> {
        // Validate batch size
        if events.len() > MAX_PUBLISH_BATCH_SIZE {
            return BatchTooLargeSnafu { count: events.len() }.fail();
        }

        if events.is_empty() {
            // Empty batch - return current cursor
            // In practice, we could query the current committed index,
            // but for simplicity we return BEGINNING
            return Ok(Cursor::BEGINNING);
        }

        // Encode all events
        let mut pairs = Vec::with_capacity(events.len());
        for (topic, payload) in &events {
            validate_payload(payload)?;

            let key = topic_to_key(topic);
            let value = encode_event(topic, payload, &HashMap::new())?;

            pairs.push((key_to_string(&key), String::from_utf8_lossy(&value).to_string()));
        }

        // Write as atomic batch through Raft
        let result = self
            .kv_store
            .write(WriteRequest {
                command: WriteCommand::SetMulti { pairs },
            })
            .await?;

        // Use header_revision as the cursor (Raft revision)
        Ok(Cursor::from_index(result.header_revision.unwrap_or(0)))
    }
}

/// Convenience function for publishing without creating a Publisher instance.
///
/// Useful for one-off publishes or when you don't need the full Publisher API.
///
/// # Examples
///
/// ```ignore
/// use aspen_pubsub::{publish, Topic};
///
/// let topic = Topic::new("orders.created")?;
/// let cursor = publish(&kv_store, &topic, b"order data").await?;
/// ```
pub async fn publish<K: KeyValueStore>(kv_store: &K, topic: &Topic, payload: &[u8]) -> Result<Cursor> {
    validate_payload(payload)?;

    let key = topic_to_key(topic);
    let value = encode_event(topic, payload, &HashMap::new())?;

    let result = kv_store
        .write(WriteRequest {
            command: WriteCommand::Set {
                key: key_to_string(&key),
                value: String::from_utf8_lossy(&value).to_string(),
            },
        })
        .await?;

    Ok(Cursor::from_index(result.header_revision.unwrap_or(0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MAX_PAYLOAD_SIZE;
    use crate::error::PubSubError;

    // Note: Full integration tests require a running Raft cluster.
    // These tests verify validation logic only.

    #[test]
    fn test_validate_payload_success() {
        assert!(validate_payload(b"small payload").is_ok());
        assert!(validate_payload(&vec![0u8; 1024]).is_ok());
    }

    #[test]
    fn test_validate_payload_too_large() {
        let large = vec![0u8; MAX_PAYLOAD_SIZE + 1];
        assert!(matches!(validate_payload(&large), Err(PubSubError::PayloadTooLarge { .. })));
    }

    // Compile-time batch size bounds check
    const _: () = {
        assert!(MAX_PUBLISH_BATCH_SIZE >= 10);
        assert!(MAX_PUBLISH_BATCH_SIZE <= 1000);
    };
}
