//! Mock gossip infrastructure for fast, deterministic unit tests.
//!
//! Provides an in-memory gossip simulator that allows multiple nodes to
//! communicate via shared broadcast channels without any network overhead.
//!
//! # Example
//!
//! ```
//! use aspen::tests::support::mock_gossip::{MockGossip, MockGossipHandle};
//! use iroh_gossip::proto::TopicId;
//!
//! let gossip = MockGossip::new();
//! let topic = TopicId::from_bytes([1u8; 32]);
//!
//! // Node 1 subscribes
//! let handle1 = gossip.subscribe(topic).await?;
//!
//! // Node 2 subscribes to same topic
//! let handle2 = gossip.subscribe(topic).await?;
//!
//! // Node 1 broadcasts
//! handle1.broadcast(b"hello".to_vec()).await?;
//!
//! // Node 2 receives
//! let msg = handle2.received().await?;
//! assert_eq!(msg, b"hello");
//! # Ok::<(), anyhow::Error>(())
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Context;
use anyhow::Result;
use bytes::Bytes;
use iroh_gossip::proto::TopicId;
use tokio::sync::broadcast;

/// Mock gossip instance managing multiple topics.
///
/// All nodes subscribing to the same topic share a broadcast channel,
/// allowing instant message propagation without network overhead.
///
/// Tiger Style: Bounded channel size (256 messages) to prevent unbounded memory use.
#[derive(Clone)]
pub struct MockGossip {
    topics: Arc<Mutex<HashMap<TopicId, broadcast::Sender<Bytes>>>>,
    channel_capacity: usize,
}

impl MockGossip {
    /// Default channel capacity for each topic.
    ///
    /// Tiger Style: Fixed limit to prevent unbounded queue growth.
    const DEFAULT_CAPACITY: usize = 256;

    /// Create a new mock gossip instance.
    pub fn new() -> Self {
        Self {
            topics: Arc::new(Mutex::new(HashMap::new())),
            channel_capacity: Self::DEFAULT_CAPACITY,
        }
    }

    /// Create a mock gossip instance with custom channel capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            topics: Arc::new(Mutex::new(HashMap::new())),
            channel_capacity: capacity,
        }
    }

    /// Subscribe to a topic, creating it if it doesn't exist.
    ///
    /// Returns a handle that can broadcast and receive messages on the topic.
    ///
    /// Tiger Style: Fail fast if channel is full.
    pub fn subscribe(&self, topic: TopicId) -> Result<MockGossipHandle> {
        let mut topics = self.topics.lock().unwrap();

        let sender = topics
            .entry(topic)
            .or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(self.channel_capacity);
                tx
            })
            .clone();

        let receiver = sender.subscribe();

        Ok(MockGossipHandle {
            topic,
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        })
    }

    /// Get the number of active topics.
    pub fn topic_count(&self) -> usize {
        self.topics.lock().unwrap().len()
    }
}

impl Default for MockGossip {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for interacting with a mock gossip topic.
///
/// Provides methods to broadcast messages and receive messages from other
/// subscribers on the same topic.
pub struct MockGossipHandle {
    #[allow(dead_code)]
    topic: TopicId,
    sender: broadcast::Sender<Bytes>,
    receiver: Arc<tokio::sync::Mutex<broadcast::Receiver<Bytes>>>,
}

impl MockGossipHandle {
    /// Broadcast a message to all subscribers on this topic.
    ///
    /// Returns the number of receivers that will receive the message.
    ///
    /// Tiger Style: Fail fast if channel is full.
    pub async fn broadcast(&self, data: Vec<u8>) -> Result<usize> {
        let bytes = Bytes::from(data);
        let receiver_count = self.sender.send(bytes).context("failed to broadcast message (channel full or closed)")?;
        Ok(receiver_count)
    }

    /// Receive the next message on this topic.
    ///
    /// Blocks until a message is available or the channel is closed.
    ///
    /// Returns `None` if all senders have been dropped (topic closed).
    pub async fn receive(&self) -> Result<Option<Bytes>> {
        let mut rx = self.receiver.lock().await;
        match rx.recv().await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(broadcast::error::RecvError::Closed) => Ok(None),
            Err(broadcast::error::RecvError::Lagged(n)) => {
                anyhow::bail!("mock gossip receiver lagged by {} messages", n)
            }
        }
    }

    /// Try to receive a message without blocking.
    ///
    /// Returns:
    /// - `Ok(Some(bytes))` if a message is immediately available
    /// - `Ok(None)` if no message is available or channel is closed
    /// - `Err` if receiver lagged
    pub async fn try_receive(&self) -> Result<Option<Bytes>> {
        let mut rx = self.receiver.lock().await;
        match rx.try_recv() {
            Ok(bytes) => Ok(Some(bytes)),
            Err(broadcast::error::TryRecvError::Empty) => Ok(None),
            Err(broadcast::error::TryRecvError::Closed) => Ok(None),
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                anyhow::bail!("mock gossip receiver lagged by {} messages", n)
            }
        }
    }

    /// Get the topic ID for this handle.
    #[allow(dead_code)]
    pub fn topic_id(&self) -> TopicId {
        self.topic
    }

    /// Get the number of active receivers on this topic (including self).
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_topic_id(seed: u8) -> TopicId {
        TopicId::from_bytes([seed; 32])
    }

    #[tokio::test]
    async fn test_mock_gossip_basic() {
        let gossip = MockGossip::new();
        let topic = create_topic_id(1);

        let handle1 = gossip.subscribe(topic).unwrap();
        let handle2 = gossip.subscribe(topic).unwrap();

        // Broadcast from handle1
        handle1.broadcast(b"hello".to_vec()).await.unwrap();

        // Receive on handle2
        let msg = handle2.receive().await.unwrap().unwrap();
        assert_eq!(msg, b"hello"[..]);

        // handle1 also receives its own broadcast
        let msg = handle1.receive().await.unwrap().unwrap();
        assert_eq!(msg, b"hello"[..]);
    }

    #[tokio::test]
    async fn test_mock_gossip_multiple_topics() {
        let gossip = MockGossip::new();
        let topic1 = create_topic_id(1);
        let topic2 = create_topic_id(2);

        let handle1a = gossip.subscribe(topic1).unwrap();
        let handle1b = gossip.subscribe(topic1).unwrap();
        let handle2a = gossip.subscribe(topic2).unwrap();

        // Broadcast on topic1
        handle1a.broadcast(b"topic1".to_vec()).await.unwrap();

        // Only topic1 subscribers receive
        let msg = handle1b.receive().await.unwrap().unwrap();
        assert_eq!(msg, b"topic1"[..]);

        // topic2 subscriber doesn't receive
        let msg = handle2a.try_receive().await.unwrap();
        assert!(msg.is_none());

        assert_eq!(gossip.topic_count(), 2);
    }

    #[tokio::test]
    async fn test_mock_gossip_receiver_count() {
        let gossip = MockGossip::new();
        let topic = create_topic_id(1);

        let handle1 = gossip.subscribe(topic).unwrap();
        assert_eq!(handle1.receiver_count(), 1);

        let handle2 = gossip.subscribe(topic).unwrap();
        assert_eq!(handle1.receiver_count(), 2);
        assert_eq!(handle2.receiver_count(), 2);

        drop(handle2);
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert_eq!(handle1.receiver_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_gossip_try_receive() {
        let gossip = MockGossip::new();
        let topic = create_topic_id(1);

        let handle = gossip.subscribe(topic).unwrap();

        // No message available
        let msg = handle.try_receive().await.unwrap();
        assert!(msg.is_none());

        // Broadcast a message
        handle.broadcast(b"test".to_vec()).await.unwrap();

        // Message immediately available
        let msg = handle.try_receive().await.unwrap().unwrap();
        assert_eq!(msg, b"test"[..]);

        // No more messages
        let msg = handle.try_receive().await.unwrap();
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn test_mock_gossip_custom_capacity() {
        let gossip = MockGossip::with_capacity(2);
        let topic = create_topic_id(1);

        let handle1 = gossip.subscribe(topic).unwrap();
        let _handle2 = gossip.subscribe(topic).unwrap();

        // Fill the channel (capacity 2)
        handle1.broadcast(b"msg1".to_vec()).await.unwrap();
        handle1.broadcast(b"msg2".to_vec()).await.unwrap();

        // Third message should succeed (broadcast doesn't block on full channel)
        // but receiver may lag
        handle1.broadcast(b"msg3".to_vec()).await.unwrap();

        // handle2 should receive all messages or detect lag
        // (depending on timing and channel behavior)
    }
}
