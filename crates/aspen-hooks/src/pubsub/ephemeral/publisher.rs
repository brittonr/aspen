//! Ephemeral publisher — implements the Publisher trait without Raft consensus.

use std::sync::Arc;

use async_trait::async_trait;

use super::super::cursor::Cursor;
use super::super::error::Result;
use super::super::event::Event;
use super::super::publisher::Publisher;
use super::super::topic::Topic;
use super::broker::EphemeralBroker;

/// Ephemeral publisher that delivers events directly to subscribers without Raft.
///
/// Events are not persisted and delivery is best-effort. Use for real-time
/// streaming where sub-millisecond latency matters more than durability.
pub struct EphemeralPublisher {
    broker: Arc<EphemeralBroker>,
}

impl EphemeralPublisher {
    /// Create a new ephemeral publisher wrapping a broker.
    pub fn new(broker: Arc<EphemeralBroker>) -> Self {
        Self { broker }
    }
}

#[async_trait]
impl Publisher for EphemeralPublisher {
    async fn publish(&self, topic: &Topic, payload: &[u8]) -> Result<Cursor> {
        // Create event with EPHEMERAL cursor
        let event = Event {
            topic: topic.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: current_timestamp_ms(),
            payload: payload.to_vec(),
            headers: Vec::new(),
        };

        event.validate()?;

        // Publish to broker (non-blocking)
        self.broker.publish(topic, event).await;

        // Always return EPHEMERAL cursor
        Ok(Cursor::EPHEMERAL)
    }

    async fn publish_batch(&self, events: Vec<(Topic, Vec<u8>)>) -> Result<Cursor> {
        if events.is_empty() {
            return Ok(Cursor::BEGINNING);
        }

        let timestamp = current_timestamp_ms();

        // Publish each event individually (no atomicity guarantee for ephemeral)
        for (topic, payload) in events {
            let event = Event {
                topic: topic.clone(),
                cursor: Cursor::EPHEMERAL,
                timestamp_ms: timestamp,
                payload,
                headers: Vec::new(),
            };

            event.validate()?;

            self.broker.publish(&topic, event).await;
        }

        // Return EPHEMERAL cursor
        Ok(Cursor::EPHEMERAL)
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
    use super::super::super::cursor::Cursor;
    use super::super::super::topic::TopicPattern;
    use super::*;

    #[tokio::test]
    async fn test_ephemeral_publish_returns_ephemeral_cursor() {
        let broker = Arc::new(EphemeralBroker::new());
        let publisher = EphemeralPublisher::new(broker.clone());

        let topic = Topic::new("orders.created").unwrap();
        let cursor = publisher.publish(&topic, b"test data").await.unwrap();

        assert_eq!(cursor, Cursor::EPHEMERAL);
    }

    #[tokio::test]
    async fn test_ephemeral_publish_batch_empty() {
        let broker = Arc::new(EphemeralBroker::new());
        let publisher = EphemeralPublisher::new(broker);

        let cursor = publisher.publish_batch(vec![]).await.unwrap();

        assert_eq!(cursor, Cursor::BEGINNING);
    }

    #[tokio::test]
    async fn test_ephemeral_publish_batch_returns_ephemeral_cursor() {
        let broker = Arc::new(EphemeralBroker::new());
        let publisher = EphemeralPublisher::new(broker);

        let events = vec![
            (Topic::new("orders.created").unwrap(), b"order1".to_vec()),
            (Topic::new("orders.updated").unwrap(), b"order2".to_vec()),
        ];

        let cursor = publisher.publish_batch(events).await.unwrap();

        assert_eq!(cursor, Cursor::EPHEMERAL);
    }

    #[tokio::test]
    async fn test_ephemeral_publish_delivers_to_subscriber() {
        let broker = Arc::new(EphemeralBroker::new());
        let publisher = EphemeralPublisher::new(broker.clone());

        // Subscribe
        let pattern = TopicPattern::new("orders.*").unwrap();
        let (_id, mut receiver) = broker.subscribe(pattern, 10).await.unwrap();

        // Publish
        let topic = Topic::new("orders.created").unwrap();
        let cursor = publisher.publish(&topic, b"order data").await.unwrap();

        assert_eq!(cursor, Cursor::EPHEMERAL);

        // Verify delivery
        let event = receiver.recv().await.unwrap();
        assert_eq!(event.payload, b"order data");
        assert_eq!(event.cursor, Cursor::EPHEMERAL);
    }

    #[tokio::test]
    async fn test_ephemeral_publish_batch_delivers_to_subscriber() {
        let broker = Arc::new(EphemeralBroker::new());
        let publisher = EphemeralPublisher::new(broker.clone());

        // Subscribe
        let pattern = TopicPattern::new("orders.>").unwrap();
        let (_id, mut receiver) = broker.subscribe(pattern, 10).await.unwrap();

        // Publish batch
        let events = vec![
            (Topic::new("orders.created").unwrap(), b"event1".to_vec()),
            (Topic::new("orders.updated").unwrap(), b"event2".to_vec()),
        ];

        let cursor = publisher.publish_batch(events).await.unwrap();
        assert_eq!(cursor, Cursor::EPHEMERAL);

        // Verify both events delivered
        let event1 = receiver.recv().await.unwrap();
        assert_eq!(event1.payload, b"event1");

        let event2 = receiver.recv().await.unwrap();
        assert_eq!(event2.payload, b"event2");
    }

    #[tokio::test]
    async fn test_ephemeral_publish_validates_payload() {
        use super::super::super::constants::MAX_PAYLOAD_SIZE;

        let broker = Arc::new(EphemeralBroker::new());
        let publisher = EphemeralPublisher::new(broker);

        let topic = Topic::new("orders.created").unwrap();
        let oversized_payload = vec![0u8; MAX_PAYLOAD_SIZE + 1];

        let result = publisher.publish(&topic, &oversized_payload).await;

        assert!(result.is_err());
    }
}
