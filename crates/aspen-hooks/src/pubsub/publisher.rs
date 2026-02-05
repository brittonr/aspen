//! Publisher trait and Raft-backed implementation.
//!
//! Publishers write events to topics through the Raft consensus layer,
//! ensuring durability and strong ordering guarantees.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::kv::WriteCommand;
use aspen_core::kv::WriteRequest;
use async_trait::async_trait;

use super::constants::MAX_PUBLISH_BATCH_SIZE;
use super::cursor::Cursor;
use super::encoding::encode_event;
use super::error::BatchTooLargeSnafu;
use super::error::Result;
use super::event::Event;
use super::keys::build_event_key;
use super::topic::Topic;

/// Trait for publishing events to topics.
///
/// Publishers write events to the Raft log, which ensures:
/// - Durability across cluster failures
/// - Strong ordering (events are totally ordered by log index)
/// - Atomic batch writes
#[async_trait]
pub trait Publisher: Send + Sync {
    /// Publish a single event to a topic.
    ///
    /// Returns the cursor (Raft log index) where the event was committed.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to publish to
    /// * `payload` - The event payload (binary data)
    ///
    /// # Returns
    ///
    /// The cursor (log index) of the committed event.
    async fn publish(&self, topic: &Topic, payload: &[u8]) -> Result<Cursor>;

    /// Publish multiple events atomically.
    ///
    /// All events are committed together in a single Raft operation.
    /// If any event fails validation, the entire batch is rejected.
    ///
    /// # Arguments
    ///
    /// * `events` - Vec of (topic, payload) pairs
    ///
    /// # Returns
    ///
    /// The cursor of the batch (all events share this cursor).
    async fn publish_batch(&self, events: Vec<(Topic, Vec<u8>)>) -> Result<Cursor>;
}

/// Raft-backed publisher implementation.
///
/// Writes events to the KV store through Raft consensus.
pub struct RaftPublisher<K: KeyValueStore + 'static> {
    store: Arc<K>,
}

impl<K: KeyValueStore + 'static> RaftPublisher<K> {
    /// Create a new Raft publisher.
    ///
    /// # Arguments
    ///
    /// * `store` - The KV store to publish through
    pub fn new(store: Arc<K>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<K: KeyValueStore + 'static> Publisher for RaftPublisher<K> {
    async fn publish(&self, topic: &Topic, payload: &[u8]) -> Result<Cursor> {
        // Create event with placeholder cursor (will be set by Raft)
        // We use 0 as placeholder; the actual cursor is returned by the write
        let event = Event {
            topic: topic.clone(),
            cursor: Cursor::BEGINNING, // Placeholder
            timestamp_ms: current_timestamp_ms(),
            payload: payload.to_vec(),
            headers: Vec::new(),
        };

        event.validate()?;

        // Encode event
        let event_bytes = encode_event(&event)?;

        // Build key using placeholder cursor (we'll get actual index from response)
        // Note: In production, we'd get the actual log index from Raft.
        // For now, we use timestamp-based ordering within the prefix.
        let key = build_event_key(topic, Cursor::from_index(current_timestamp_ms()));

        // Write to KV store through Raft
        let response = self
            .store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key,
                    value: String::from_utf8_lossy(&event_bytes).into_owned(),
                },
            })
            .await?;

        // The header_revision from the response is the Raft log index (our cursor)
        let log_index = response.header_revision.unwrap_or(0);
        Ok(Cursor::from_index(log_index))
    }

    async fn publish_batch(&self, events: Vec<(Topic, Vec<u8>)>) -> Result<Cursor> {
        if events.is_empty() {
            return Ok(Cursor::BEGINNING);
        }

        if events.len() > MAX_PUBLISH_BATCH_SIZE {
            return BatchTooLargeSnafu { count: events.len() }.fail();
        }

        let timestamp = current_timestamp_ms();
        let mut operations = Vec::with_capacity(events.len());

        for (i, (topic, payload)) in events.iter().enumerate() {
            let event = Event {
                topic: topic.clone(),
                cursor: Cursor::BEGINNING, // Placeholder
                timestamp_ms: timestamp,
                payload: payload.clone(),
                headers: Vec::new(),
            };

            event.validate()?;

            let event_bytes = encode_event(&event)?;
            // Use timestamp + index for unique ordering within batch
            let key = build_event_key(topic, Cursor::from_index(timestamp + i as u64));

            operations.push(aspen_core::kv::BatchOperation::Set {
                key,
                value: String::from_utf8_lossy(&event_bytes).into_owned(),
            });
        }

        let response = self
            .store
            .write(WriteRequest {
                command: WriteCommand::Batch { operations },
            })
            .await?;

        // The header_revision from the response is the Raft log index (our cursor)
        let log_index = response.header_revision.unwrap_or(0);
        Ok(Cursor::from_index(log_index))
    }
}

/// Get current timestamp in milliseconds.
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp_ms();
        // Should be after 2020-01-01
        assert!(ts > 1577836800000);
    }
}
