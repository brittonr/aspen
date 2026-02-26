//! Subscriber types and helpers for pub/sub.
//!
//! This module provides functionality for subscribing to topics and streaming
//! events. The actual streaming implementation is provided by the streaming module.

use super::cursor::Cursor;
use super::topic::TopicPattern;

/// A subscription to one or more topics.
///
/// Subscriptions track the pattern to match and the current cursor position.
#[derive(Debug, Clone)]
pub struct Subscription {
    /// The pattern to match topics against.
    pub pattern: TopicPattern,

    /// Starting cursor for this subscription.
    ///
    /// Events with cursors >= this value will be delivered.
    pub start_cursor: Cursor,

    /// Subscriber ID for tracking (optional).
    pub subscriber_id: Option<String>,
}

impl Subscription {
    /// Create a new subscription.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The topic pattern to subscribe to
    /// * `start_cursor` - Where to start reading from
    pub fn new(pattern: TopicPattern, start_cursor: Cursor) -> Self {
        Self {
            pattern,
            start_cursor,
            subscriber_id: None,
        }
    }

    /// Create a subscription from the beginning of all matching events.
    pub fn from_beginning(pattern: TopicPattern) -> Self {
        Self::new(pattern, Cursor::BEGINNING)
    }

    /// Create a subscription that only receives new events.
    pub fn new_events_only(pattern: TopicPattern) -> Self {
        Self::new(pattern, Cursor::LATEST)
    }

    /// Set a subscriber ID for tracking.
    pub fn with_subscriber_id(mut self, id: impl Into<String>) -> Self {
        self.subscriber_id = Some(id.into());
        self
    }
}

/// Options for controlling subscription behavior.
#[derive(Debug, Clone, Default)]
pub struct SubscriptionOptions {
    /// Maximum number of events to buffer before applying backpressure.
    ///
    /// Default is 1000.
    pub buffer_size: Option<usize>,

    /// How long to wait for new events before timing out (milliseconds).
    ///
    /// None means no timeout (wait indefinitely).
    pub timeout_ms: Option<u64>,

    /// Whether to deliver events in batches.
    ///
    /// When true, multiple events may be delivered together.
    pub batch_delivery: bool,

    /// Maximum events per batch (when batch_delivery is true).
    pub max_batch_size: Option<usize>,
}

impl SubscriptionOptions {
    /// Create options with a specific buffer size.
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = Some(size);
        self
    }

    /// Create options with a timeout.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Enable batch delivery.
    pub fn with_batch_delivery(mut self, max_batch_size: usize) -> Self {
        self.batch_delivery = true;
        self.max_batch_size = Some(max_batch_size);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::super::topic::Topic;
    use super::*;

    #[test]
    fn test_subscription_creation() {
        let pattern = TopicPattern::exact(&Topic::new("orders.created").unwrap());
        let sub = Subscription::new(pattern.clone(), Cursor::from_index(100));

        assert_eq!(sub.start_cursor.index(), 100);
        assert!(sub.subscriber_id.is_none());
    }

    #[test]
    fn test_subscription_from_beginning() {
        let pattern = TopicPattern::new("orders.>").unwrap();
        let sub = Subscription::from_beginning(pattern);

        assert!(sub.start_cursor.is_beginning());
    }

    #[test]
    fn test_subscription_new_events_only() {
        let pattern = TopicPattern::new("orders.*").unwrap();
        let sub = Subscription::new_events_only(pattern);

        assert!(sub.start_cursor.is_latest());
    }

    #[test]
    fn test_subscription_with_id() {
        let pattern = TopicPattern::new("orders.>").unwrap();
        let sub = Subscription::from_beginning(pattern).with_subscriber_id("my-subscriber");

        assert_eq!(sub.subscriber_id, Some("my-subscriber".to_string()));
    }

    #[test]
    fn test_subscription_options() {
        let opts = SubscriptionOptions::default().with_buffer_size(500).with_timeout_ms(5000).with_batch_delivery(100);

        assert_eq!(opts.buffer_size, Some(500));
        assert_eq!(opts.timeout_ms, Some(5000));
        assert!(opts.batch_delivery);
        assert_eq!(opts.max_batch_size, Some(100));
    }
}
