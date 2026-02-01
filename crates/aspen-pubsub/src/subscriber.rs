//! Subscriber for pub/sub events.
//!
//! Subscribers receive events from topics by streaming committed
//! Raft log entries. Supports wildcard patterns and historical replay.

use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use aspen_transport::log_subscriber::LogEntryPayload;
use futures::Stream;

use crate::cursor::Cursor;
use crate::encoding::decode_event;
use crate::encoding::is_pubsub_operation;
use crate::encoding::try_decode_event;
use crate::error::Result;
use crate::event::Event;
use crate::keys::pattern_to_prefix;
use crate::topic::TopicPattern;

/// Stream of events from a subscription.
///
/// Implements async iteration over events matching a topic pattern.
/// Events are delivered in commit order (Raft log index order).
///
/// # Examples
///
/// ```ignore
/// let mut stream = subscriber.subscribe(pattern, Cursor::BEGINNING).await?;
///
/// while let Some(result) = stream.next().await {
///     let event = result?;
///     println!("Received: {} at cursor {}", event.topic, event.cursor);
///
///     // Checkpoint cursor for resume
///     save_cursor(event.cursor)?;
/// }
/// ```
pub struct EventStream<S> {
    /// Inner stream of LogEntryPayload.
    inner: S,
    /// Pattern for filtering events.
    pattern: TopicPattern,
    /// Key prefix for fast rejection (server-side filtered, but we double-check).
    /// Reserved for future server-side filtering optimization.
    #[allow(dead_code)]
    prefix: Vec<u8>,
}

impl<S> EventStream<S>
where
    S: Stream<Item = Result<LogEntryPayload>> + Unpin,
{
    /// Create a new event stream with pattern filtering.
    pub fn new(inner: S, pattern: TopicPattern) -> Self {
        let prefix = pattern_to_prefix(&pattern);
        Self { inner, pattern, prefix }
    }

    /// Get the pattern this stream is filtering on.
    pub fn pattern(&self) -> &TopicPattern {
        &self.pattern
    }

    /// Get the next event from the stream.
    ///
    /// Returns `None` when the stream ends.
    pub async fn next(&mut self) -> Option<Result<Event>> {
        use futures::StreamExt;

        loop {
            // Get next log entry from inner stream
            let payload = match self.inner.next().await {
                Some(Ok(p)) => p,
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            };

            // Skip non-pub/sub operations
            if !is_pubsub_operation(&payload.operation) {
                continue;
            }

            // Decode the event
            let event = match decode_event(payload) {
                Ok(e) => e,
                Err(e) => {
                    // Log decoding errors but continue (defensive)
                    tracing::warn!("failed to decode pub/sub event: {}", e);
                    continue;
                }
            };

            // Apply client-side wildcard filtering
            if self.pattern.matches(&event.topic) {
                return Some(Ok(event));
            }
            // Event doesn't match pattern, continue to next
        }
    }
}

impl<S> Stream for EventStream<S>
where
    S: Stream<Item = Result<LogEntryPayload>> + Unpin,
{
    type Item = Result<Event>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // Poll inner stream
            let payload = match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(p))) => p,
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            };

            // Skip non-pub/sub operations
            if !is_pubsub_operation(&payload.operation) {
                continue;
            }

            // Decode the event
            let event = match decode_event(payload) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("failed to decode pub/sub event: {}", e);
                    continue;
                }
            };

            // Apply client-side wildcard filtering
            if self.pattern.matches(&event.topic) {
                return Poll::Ready(Some(Ok(event)));
            }
            // Event doesn't match pattern, continue loop
        }
    }
}

/// Filter function for use with raw log streams.
///
/// Returns `Some(event)` if the payload is a pub/sub event matching the pattern,
/// `None` otherwise.
///
/// # Examples
///
/// ```ignore
/// let pattern = TopicPattern::new("orders.*")?;
///
/// log_stream
///     .filter_map(|payload| async { filter_event(payload, &pattern) })
///     .for_each(|event| async { process(event) })
///     .await;
/// ```
pub fn filter_event(payload: LogEntryPayload, pattern: &TopicPattern) -> Option<Result<Event>> {
    // Try to decode as pub/sub event
    let event = match try_decode_event(payload) {
        Some(Ok(e)) => e,
        Some(Err(e)) => return Some(Err(e)),
        None => return None, // Not a pub/sub event
    };

    // Check pattern match
    if pattern.matches(&event.topic) {
        Some(Ok(event))
    } else {
        None
    }
}

/// Builder for creating subscriptions.
///
/// Provides a fluent API for configuring subscription parameters.
///
/// # Examples
///
/// ```ignore
/// let stream = SubscriptionBuilder::new(log_stream)
///     .pattern(TopicPattern::new("orders.*")?)
///     .from_cursor(Cursor::BEGINNING)
///     .build();
/// ```
pub struct SubscriptionBuilder<S> {
    inner: S,
    pattern: Option<TopicPattern>,
}

impl<S> SubscriptionBuilder<S>
where
    S: Stream<Item = Result<LogEntryPayload>> + Unpin,
{
    /// Create a new subscription builder.
    pub fn new(inner: S) -> Self {
        Self { inner, pattern: None }
    }

    /// Set the topic pattern to filter on.
    pub fn pattern(mut self, pattern: TopicPattern) -> Self {
        self.pattern = Some(pattern);
        self
    }

    /// Build the event stream.
    ///
    /// Uses the all-matching pattern if none was specified.
    pub fn build(self) -> EventStream<S> {
        let pattern = self.pattern.unwrap_or_else(TopicPattern::all);
        EventStream::new(self.inner, pattern)
    }
}

/// Configuration for subscriptions.
#[derive(Debug, Clone)]
pub struct SubscriptionConfig {
    /// Pattern to filter topics.
    pub pattern: TopicPattern,
    /// Starting cursor position.
    pub cursor: Cursor,
}

impl SubscriptionConfig {
    /// Create a new subscription config.
    pub fn new(pattern: TopicPattern, cursor: Cursor) -> Self {
        Self { pattern, cursor }
    }

    /// Create a config for subscribing to all topics from the beginning.
    pub fn all_from_beginning() -> Self {
        Self {
            pattern: TopicPattern::all(),
            cursor: Cursor::BEGINNING,
        }
    }

    /// Create a config for subscribing to all topics, new events only.
    pub fn all_latest() -> Self {
        Self {
            pattern: TopicPattern::all(),
            cursor: Cursor::LATEST,
        }
    }

    /// Get the key prefix for server-side filtering.
    pub fn key_prefix(&self) -> Vec<u8> {
        pattern_to_prefix(&self.pattern)
    }
}

#[cfg(test)]
mod tests {
    use aspen_core::SerializableTimestamp;
    use aspen_transport::log_subscriber::KvOperation;

    use super::*;
    use crate::topic::Topic;

    fn test_timestamp() -> SerializableTimestamp {
        SerializableTimestamp::from(aspen_core::create_hlc("test").new_timestamp())
    }

    fn make_pubsub_payload(topic: &str, index: u64) -> LogEntryPayload {
        use crate::encoding::encode_event;
        use crate::keys::topic_to_key;

        let topic = Topic::new(topic).unwrap();
        let key = topic_to_key(&topic);
        let value = encode_event(&topic, b"payload", &std::collections::HashMap::new()).unwrap();

        LogEntryPayload {
            index,
            term: 1,
            hlc_timestamp: test_timestamp(),
            operation: KvOperation::Set { key, value },
        }
    }

    fn make_non_pubsub_payload(index: u64) -> LogEntryPayload {
        LogEntryPayload {
            index,
            term: 1,
            hlc_timestamp: test_timestamp(),
            operation: KvOperation::Set {
                key: b"user/data".to_vec(),
                value: b"value".to_vec(),
            },
        }
    }

    #[test]
    fn test_filter_event_matching() {
        let pattern = TopicPattern::new("orders.*").unwrap();
        let payload = make_pubsub_payload("orders.created", 1);

        let result = filter_event(payload, &pattern);
        assert!(result.is_some());
        let event = result.unwrap().unwrap();
        assert_eq!(event.topic, Topic::new("orders.created").unwrap());
    }

    #[test]
    fn test_filter_event_not_matching() {
        let pattern = TopicPattern::new("orders.*").unwrap();
        let payload = make_pubsub_payload("users.created", 1);

        let result = filter_event(payload, &pattern);
        assert!(result.is_none());
    }

    #[test]
    fn test_filter_event_non_pubsub() {
        let pattern = TopicPattern::new("orders.*").unwrap();
        let payload = make_non_pubsub_payload(1);

        let result = filter_event(payload, &pattern);
        assert!(result.is_none());
    }

    #[test]
    fn test_subscription_config() {
        let config = SubscriptionConfig::new(TopicPattern::new("orders.*").unwrap(), Cursor::from_index(100));

        assert!(config.pattern.matches(&Topic::new("orders.created").unwrap()));
        assert_eq!(config.cursor.index(), 100);
    }

    #[test]
    fn test_subscription_config_all() {
        let config = SubscriptionConfig::all_from_beginning();
        assert!(config.pattern.matches(&Topic::new("anything").unwrap()));
        assert!(config.cursor.is_beginning());

        let config = SubscriptionConfig::all_latest();
        assert!(config.cursor.is_latest());
    }

    #[tokio::test]
    async fn test_event_stream_filtering() {
        use futures::stream;

        let payloads = vec![
            Ok(make_pubsub_payload("orders.created", 1)),
            Ok(make_non_pubsub_payload(2)),              // Should be skipped
            Ok(make_pubsub_payload("users.created", 3)), // Doesn't match pattern
            Ok(make_pubsub_payload("orders.updated", 4)),
        ];

        let inner = stream::iter(payloads);
        let pattern = TopicPattern::new("orders.*").unwrap();
        let mut stream = EventStream::new(inner, pattern);

        // Should get orders.created
        let event = stream.next().await.unwrap().unwrap();
        assert_eq!(event.topic, Topic::new("orders.created").unwrap());
        assert_eq!(event.cursor.index(), 1);

        // Should get orders.updated (skipped non-matching)
        let event = stream.next().await.unwrap().unwrap();
        assert_eq!(event.topic, Topic::new("orders.updated").unwrap());
        assert_eq!(event.cursor.index(), 4);

        // Stream should be exhausted
        assert!(stream.next().await.is_none());
    }
}
