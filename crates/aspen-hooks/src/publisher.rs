//! Hook event publisher.
//!
//! Wraps the pub/sub RaftPublisher to publish hook events to the appropriate topics.

use std::sync::Arc;

use aspen_core::KeyValueStore;

use crate::constants::HOOK_TOPIC_PREFIX;
use crate::error::InvalidPatternSnafu;
use crate::error::Result;
use crate::event::HookEvent;
use crate::event::HookEventType;
use crate::pubsub::Cursor;
use crate::pubsub::Publisher;
use crate::pubsub::RaftPublisher;
use crate::pubsub::Topic;

/// Publisher for hook events.
///
/// Wraps the underlying pub/sub publisher to provide a convenient API
/// for publishing hook events to the appropriate topics.
///
/// # Example
///
/// ```ignore
/// let publisher = HookPublisher::new(kv_store);
///
/// let event = HookEvent::new(
///     HookEventType::WriteCommitted,
///     1,
///     serde_json::json!({"key": "test"}),
/// );
///
/// let cursor = publisher.publish_event(&event).await?;
/// ```
pub struct HookPublisher<K: KeyValueStore + 'static> {
    inner: RaftPublisher<K>,
}

impl<K: KeyValueStore + 'static> HookPublisher<K> {
    /// Create a new hook publisher wrapping a KeyValueStore.
    pub fn new(kv_store: Arc<K>) -> Self {
        Self {
            inner: RaftPublisher::new(kv_store),
        }
    }

    /// Publish a hook event.
    ///
    /// The event is published to the topic determined by its event type
    /// (e.g., `hooks.kv.write_committed` for WriteCommitted events).
    ///
    /// Returns the cursor (Raft log index) of the committed event.
    pub async fn publish_event(&self, event: &HookEvent) -> Result<Cursor> {
        let topic = self.topic_for_event(event)?;
        let payload = event.to_bytes()?;
        let cursor = self.inner.publish(&topic, &payload).await?;
        Ok(cursor)
    }

    /// Publish an event to a specific topic.
    ///
    /// This allows publishing to custom topics within the hooks namespace.
    /// The topic must start with the hooks prefix.
    pub async fn publish_to_topic(&self, topic_name: &str, event: &HookEvent) -> Result<Cursor> {
        if !topic_name.starts_with(HOOK_TOPIC_PREFIX) {
            return InvalidPatternSnafu {
                reason: format!("topic must start with '{HOOK_TOPIC_PREFIX}': {topic_name}"),
            }
            .fail();
        }

        let topic = Topic::new(topic_name)?;
        let payload = event.to_bytes()?;
        let cursor = self.inner.publish(&topic, &payload).await?;
        Ok(cursor)
    }

    /// Publish multiple events atomically.
    ///
    /// All events are committed together in a single Raft operation.
    /// Events can be of different types and will be published to their
    /// respective topics.
    pub async fn publish_batch(&self, events: &[HookEvent]) -> Result<Cursor> {
        if events.is_empty() {
            return Ok(Cursor::BEGINNING);
        }

        let mut batch = Vec::with_capacity(events.len());
        for event in events {
            let topic = self.topic_for_event(event)?;
            let payload = event.to_bytes()?;
            batch.push((topic, payload));
        }

        let cursor = self.inner.publish_batch(batch).await?;
        Ok(cursor)
    }

    /// Get the topic for a given event.
    fn topic_for_event(&self, event: &HookEvent) -> Result<Topic> {
        let topic_name = event.topic();
        Topic::new(topic_name).map_err(|e| crate::error::HookError::InvalidPattern {
            reason: format!("invalid topic for event type {:?}: {}", event.event_type, e),
        })
    }
}

/// Convenience function for creating a topic name for an event type.
pub fn topic_for_event_type(event_type: HookEventType) -> String {
    event_type.topic()
}

/// Create a topic pattern for subscribing to all events of a category.
///
/// Categories: "kv", "cluster", "system"
pub fn topic_pattern_for_category(category: &str) -> String {
    format!("{}.{}.*", HOOK_TOPIC_PREFIX, category)
}

/// Create a topic pattern for subscribing to all hook events.
pub fn topic_pattern_all() -> String {
    format!("{}.>", HOOK_TOPIC_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_for_event_type() {
        assert_eq!(topic_for_event_type(HookEventType::WriteCommitted), "hooks.kv.write_committed");
        assert_eq!(topic_for_event_type(HookEventType::LeaderElected), "hooks.cluster.leader_elected");
        assert_eq!(topic_for_event_type(HookEventType::SnapshotCreated), "hooks.system.snapshot_created");
    }

    #[test]
    fn test_topic_pattern_for_category() {
        assert_eq!(topic_pattern_for_category("kv"), "hooks.kv.*");
        assert_eq!(topic_pattern_for_category("cluster"), "hooks.cluster.*");
        assert_eq!(topic_pattern_for_category("system"), "hooks.system.*");
    }

    #[test]
    fn test_topic_pattern_all() {
        assert_eq!(topic_pattern_all(), "hooks.>");
    }
}
