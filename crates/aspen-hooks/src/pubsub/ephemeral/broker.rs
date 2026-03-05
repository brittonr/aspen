//! In-memory ephemeral event broker.
//!
//! Routes published events to matching subscribers via tokio mpsc channels.
//! No Raft, no KV, no persistence.

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use super::super::error::Result;
use super::super::error::SubscriptionRejectedSnafu;
use super::super::event::Event;
use super::super::topic::Topic;
use super::super::topic::TopicPattern;
use super::MAX_EPHEMERAL_SUBSCRIPTIONS;

/// In-memory ephemeral event broker.
///
/// Routes published events to matching subscribers via tokio mpsc channels.
/// No Raft, no KV, no persistence.
pub struct EphemeralBroker {
    subscriptions: tokio::sync::RwLock<Vec<ActiveSubscription>>,
    next_id: AtomicU64,
}

impl std::fmt::Debug for EphemeralBroker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EphemeralBroker")
            .field("next_id", &self.next_id.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

struct ActiveSubscription {
    id: u64,
    pattern: TopicPattern,
    sender: tokio::sync::mpsc::Sender<Event>,
}

impl EphemeralBroker {
    /// Create a new ephemeral broker.
    pub fn new() -> Self {
        Self {
            subscriptions: tokio::sync::RwLock::new(Vec::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Subscribe to events matching a topic pattern.
    ///
    /// Returns a subscription ID and receiver channel for matched events.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Topic pattern to match
    /// * `buffer_size` - Channel buffer size (events are dropped if full)
    ///
    /// # Returns
    ///
    /// Tuple of (subscription_id, event_receiver)
    ///
    /// # Errors
    ///
    /// Returns an error if the broker is at capacity (`MAX_EPHEMERAL_SUBSCRIPTIONS`).
    pub async fn subscribe(
        &self,
        pattern: TopicPattern,
        buffer_size: usize,
    ) -> Result<(u64, tokio::sync::mpsc::Receiver<Event>)> {
        let mut subs = self.subscriptions.write().await;

        if subs.len() >= MAX_EPHEMERAL_SUBSCRIPTIONS {
            return SubscriptionRejectedSnafu {
                reason: format!(
                    "broker at capacity: {} subscriptions (max: {})",
                    subs.len(),
                    MAX_EPHEMERAL_SUBSCRIPTIONS
                ),
            }
            .fail();
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = tokio::sync::mpsc::channel(buffer_size);

        subs.push(ActiveSubscription { id, pattern, sender });

        Ok((id, receiver))
    }

    /// Unsubscribe by subscription ID.
    ///
    /// Removes the subscription from the broker. Future publishes will not
    /// be sent to this subscriber.
    pub async fn unsubscribe(&self, id: u64) {
        let mut subs = self.subscriptions.write().await;
        subs.retain(|sub| sub.id != id);
    }

    /// Publish an event to all matching subscribers.
    ///
    /// Events are sent via `try_send` (non-blocking). If a subscriber's buffer
    /// is full, the event is dropped for that subscriber. Closed channels are
    /// ignored (subscribers are responsible for calling unsubscribe).
    ///
    /// # Arguments
    ///
    /// * `topic` - The event's topic
    /// * `event` - The event to publish
    pub async fn publish(&self, topic: &Topic, event: Event) {
        let subs = self.subscriptions.read().await;

        for sub in subs.iter() {
            if sub.pattern.matches(topic) {
                // try_send is non-blocking
                // Ignore errors: Full means drop, Closed means subscriber will clean up
                let _ = sub.sender.try_send(event.clone());
            }
        }
    }

    /// Get the current number of active subscriptions.
    ///
    /// Useful for monitoring and testing.
    pub async fn subscription_count(&self) -> usize {
        let subs = self.subscriptions.read().await;
        subs.len()
    }
}

impl Default for EphemeralBroker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::cursor::Cursor;
    use super::super::super::error::PubSubError;
    use super::*;

    #[tokio::test]
    async fn test_publish_no_subscribers() {
        let broker = EphemeralBroker::new();
        let topic = Topic::new("orders.created").unwrap();
        let event = Event {
            topic: topic.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1000,
            payload: b"test".to_vec(),
            headers: vec![],
        };

        // Should not panic
        broker.publish(&topic, event).await;
        assert_eq!(broker.subscription_count().await, 0);
    }

    #[tokio::test]
    async fn test_subscribe_and_publish_matching() {
        let broker = EphemeralBroker::new();
        let pattern = TopicPattern::new("orders.created").unwrap();
        let topic = Topic::new("orders.created").unwrap();

        let (_id, mut receiver) = broker.subscribe(pattern, 10).await.unwrap();

        let event = Event {
            topic: topic.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1000,
            payload: b"order data".to_vec(),
            headers: vec![],
        };

        broker.publish(&topic, event.clone()).await;

        // Should receive the event
        let received = receiver.recv().await.unwrap();
        assert_eq!(received.payload, b"order data");
        assert_eq!(received.cursor, Cursor::EPHEMERAL);
    }

    #[tokio::test]
    async fn test_subscribe_and_publish_non_matching() {
        let broker = EphemeralBroker::new();
        let pattern = TopicPattern::new("orders.created").unwrap();
        let topic = Topic::new("users.created").unwrap();

        let (_id, mut receiver) = broker.subscribe(pattern, 10).await.unwrap();

        let event = Event {
            topic: topic.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1000,
            payload: b"user data".to_vec(),
            headers: vec![],
        };

        broker.publish(&topic, event).await;

        // Should NOT receive the event (timeout to avoid hanging)
        let result = tokio::time::timeout(std::time::Duration::from_millis(50), receiver.recv()).await;

        assert!(result.is_err(), "Expected timeout, got event");
    }

    #[tokio::test]
    async fn test_wildcard_single_segment() {
        let broker = EphemeralBroker::new();
        let pattern = TopicPattern::new("orders.*").unwrap();

        let (_id, mut receiver) = broker.subscribe(pattern, 10).await.unwrap();

        // Should match "orders.created"
        let topic1 = Topic::new("orders.created").unwrap();
        let event1 = Event {
            topic: topic1.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1000,
            payload: b"match".to_vec(),
            headers: vec![],
        };
        broker.publish(&topic1, event1).await;

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.payload, b"match");

        // Should NOT match "orders.us.created" (too many segments)
        let topic2 = Topic::new("orders.us.created").unwrap();
        let event2 = Event {
            topic: topic2.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1001,
            payload: b"no match".to_vec(),
            headers: vec![],
        };
        broker.publish(&topic2, event2).await;

        let result = tokio::time::timeout(std::time::Duration::from_millis(50), receiver.recv()).await;

        assert!(result.is_err(), "Expected timeout, got event");
    }

    #[tokio::test]
    async fn test_wildcard_multi_segment() {
        let broker = EphemeralBroker::new();
        let pattern = TopicPattern::new("orders.>").unwrap();

        let (_id, mut receiver) = broker.subscribe(pattern, 10).await.unwrap();

        // Should match "orders.created"
        let topic1 = Topic::new("orders.created").unwrap();
        let event1 = Event {
            topic: topic1.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1000,
            payload: b"match1".to_vec(),
            headers: vec![],
        };
        broker.publish(&topic1, event1).await;

        let received1 = receiver.recv().await.unwrap();
        assert_eq!(received1.payload, b"match1");

        // Should ALSO match "orders.us.east.created"
        let topic2 = Topic::new("orders.us.east.created").unwrap();
        let event2 = Event {
            topic: topic2.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1001,
            payload: b"match2".to_vec(),
            headers: vec![],
        };
        broker.publish(&topic2, event2).await;

        let received2 = receiver.recv().await.unwrap();
        assert_eq!(received2.payload, b"match2");
    }

    #[tokio::test]
    async fn test_full_buffer_drops() {
        let broker = EphemeralBroker::new();
        let pattern = TopicPattern::new("orders.*").unwrap();
        let topic = Topic::new("orders.created").unwrap();

        // Small buffer
        let (_id, mut receiver) = broker.subscribe(pattern, 1).await.unwrap();

        // Publish 10 events rapidly
        for i in 0..10 {
            let event = Event {
                topic: topic.clone(),
                cursor: Cursor::EPHEMERAL,
                timestamp_ms: 1000 + i,
                payload: format!("event{}", i).into_bytes(),
                headers: vec![],
            };
            broker.publish(&topic, event).await;
        }

        // Should receive at least one
        let first = receiver.recv().await.unwrap();
        assert!(first.payload.starts_with(b"event"));

        // But not all 10 (some should be dropped)
        let mut count = 1;
        while let Ok(result) = tokio::time::timeout(std::time::Duration::from_millis(50), receiver.recv()).await {
            if result.is_some() {
                count += 1;
            } else {
                break;
            }
        }

        assert!(count < 10, "Expected some events to be dropped, received all {}", count);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let broker = EphemeralBroker::new();
        let pattern = TopicPattern::new("orders.created").unwrap();
        let topic = Topic::new("orders.created").unwrap();

        let (id, mut receiver) = broker.subscribe(pattern, 10).await.unwrap();
        assert_eq!(broker.subscription_count().await, 1);

        // Unsubscribe
        broker.unsubscribe(id).await;
        assert_eq!(broker.subscription_count().await, 0);

        // Publish after unsubscribe
        let event = Event {
            topic: topic.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1000,
            payload: b"should not receive".to_vec(),
            headers: vec![],
        };
        broker.publish(&topic, event).await;

        // Should NOT receive (either timeout or channel closed)
        let result = tokio::time::timeout(std::time::Duration::from_millis(50), receiver.recv()).await;

        match result {
            Err(_) => {}   // Timeout - expected
            Ok(None) => {} // Channel closed - also expected
            Ok(Some(_)) => panic!("Expected no event after unsubscribe, but received one"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broker = EphemeralBroker::new();
        let pattern = TopicPattern::new("orders.created").unwrap();
        let topic = Topic::new("orders.created").unwrap();

        // Two subscribers with same pattern
        let (_id1, mut receiver1) = broker.subscribe(pattern.clone(), 10).await.unwrap();
        let (_id2, mut receiver2) = broker.subscribe(pattern, 10).await.unwrap();

        assert_eq!(broker.subscription_count().await, 2);

        let event = Event {
            topic: topic.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1000,
            payload: b"shared event".to_vec(),
            headers: vec![],
        };

        // Publish once
        broker.publish(&topic, event).await;

        // Both should receive
        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();

        assert_eq!(received1.payload, b"shared event");
        assert_eq!(received2.payload, b"shared event");
    }

    #[tokio::test]
    async fn test_subscription_capacity_limit() {
        let broker = EphemeralBroker::new();
        let pattern = TopicPattern::new("test.*").unwrap();

        // Subscribe up to capacity
        let mut subs = Vec::new();
        for _ in 0..MAX_EPHEMERAL_SUBSCRIPTIONS {
            let (id, _rx) = broker.subscribe(pattern.clone(), 10).await.unwrap();
            subs.push(id);
        }

        assert_eq!(broker.subscription_count().await, MAX_EPHEMERAL_SUBSCRIPTIONS);

        // Next subscription should fail
        let result = broker.subscribe(pattern, 10).await;
        assert!(matches!(result, Err(PubSubError::SubscriptionRejected { .. })));
    }

    #[tokio::test]
    async fn test_multi_topic_isolation() {
        let broker = EphemeralBroker::new();

        // Subscribe to different patterns
        let pat_orders = TopicPattern::new("orders.*").unwrap();
        let pat_users = TopicPattern::new("users.*").unwrap();

        let (_id1, mut rx_orders) = broker.subscribe(pat_orders, 10).await.unwrap();
        let (_id2, mut rx_users) = broker.subscribe(pat_users, 10).await.unwrap();

        // Publish to orders
        let topic_orders = Topic::new("orders.created").unwrap();
        let order_event = Event {
            topic: topic_orders.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1000,
            payload: b"order-1".to_vec(),
            headers: vec![],
        };
        broker.publish(&topic_orders, order_event).await;

        // Publish to users
        let topic_users = Topic::new("users.signup").unwrap();
        let user_event = Event {
            topic: topic_users.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1001,
            payload: b"user-1".to_vec(),
            headers: vec![],
        };
        broker.publish(&topic_users, user_event).await;

        // Orders subscriber should only get order events
        let received = rx_orders.recv().await.unwrap();
        assert_eq!(received.payload, b"order-1");
        let timeout = tokio::time::timeout(std::time::Duration::from_millis(50), rx_orders.recv()).await;
        assert!(timeout.is_err(), "orders subscriber should not receive user events");

        // Users subscriber should only get user events
        let received = rx_users.recv().await.unwrap();
        assert_eq!(received.payload, b"user-1");
        let timeout = tokio::time::timeout(std::time::Duration::from_millis(50), rx_users.recv()).await;
        assert!(timeout.is_err(), "users subscriber should not receive order events");
    }

    #[tokio::test]
    async fn test_disconnect_cleanup_no_leak() {
        let broker = EphemeralBroker::new();
        let pattern = TopicPattern::new("test.*").unwrap();

        // Subscribe and immediately drop the receiver (simulating disconnect)
        let (id, rx) = broker.subscribe(pattern.clone(), 10).await.unwrap();
        assert_eq!(broker.subscription_count().await, 1);

        // Drop the receiver
        drop(rx);

        // Unsubscribe explicitly (as the handler would on disconnect)
        broker.unsubscribe(id).await;
        assert_eq!(broker.subscription_count().await, 0);

        // Publish should not panic/hang with no subscribers
        let topic = Topic::new("test.thing").unwrap();
        let event = Event {
            topic: topic.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 999,
            payload: b"orphan".to_vec(),
            headers: vec![],
        };
        broker.publish(&topic, event).await;

        // Second subscriber should still work fine
        let (_id2, mut rx2) = broker.subscribe(pattern, 10).await.unwrap();
        assert_eq!(broker.subscription_count().await, 1);

        let event2 = Event {
            topic: topic.clone(),
            cursor: Cursor::EPHEMERAL,
            timestamp_ms: 1000,
            payload: b"new".to_vec(),
            headers: vec![],
        };
        broker.publish(&topic, event2).await;

        let received = rx2.recv().await.unwrap();
        assert_eq!(received.payload, b"new");
    }
}
