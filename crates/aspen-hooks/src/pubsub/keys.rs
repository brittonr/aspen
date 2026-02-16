//! Key encoding for pub/sub events in the KV store.
//!
//! Events are stored with keys that enable efficient topic-based scanning.
//! The key format is:
//!
//! `__pubsub/events/{topic_segment}/{topic_segment}/.../{cursor_be}`
//!
//! Where:
//! - `__pubsub/events/` is the reserved prefix
//! - Topic segments are nested to enable prefix scanning
//! - `cursor_be` is the cursor (log index) as big-endian u64 for ordering

use super::constants::PUBSUB_KEY_PREFIX;
use super::cursor::Cursor;
use super::error::Result;
use super::topic::Topic;

/// Build the KV store key for an event.
///
/// The key format enables efficient range scans for a topic:
/// - Scan all events for "orders": prefix scan on `__pubsub/events/orders/`
/// - Scan all events for "orders.created": prefix scan on `__pubsub/events/orders/created/`
///
/// Within a topic prefix, events are ordered by cursor (which equals Raft log index).
pub fn build_event_key(topic: &Topic, cursor: Cursor) -> String {
    let mut key = PUBSUB_KEY_PREFIX.to_string();

    // Add topic segments
    for segment in topic.segments() {
        key.push_str(segment);
        key.push('/');
    }

    // Add cursor as big-endian hex for lexicographic ordering
    key.push_str(&format!("{:016x}", cursor.index()));

    key
}

/// Build a key prefix for scanning all events in a topic (and subtopics).
///
/// Returns a prefix like `__pubsub/events/orders/` that will match all events
/// whose topic starts with "orders".
pub fn build_topic_prefix(topic: &Topic) -> String {
    let mut prefix = PUBSUB_KEY_PREFIX.to_string();

    for segment in topic.segments() {
        prefix.push_str(segment);
        prefix.push('/');
    }

    prefix
}

/// Build a key prefix for a cursor range within a topic.
///
/// Returns a (start, end) range for scanning events in a topic
/// starting from the given cursor.
pub fn build_cursor_range(topic: &Topic, start_cursor: Cursor) -> (String, String) {
    let prefix = build_topic_prefix(topic);

    let start = format!("{}{:016x}", prefix, start_cursor.index());
    let end = format!("{}{:016x}", prefix, u64::MAX);

    (start, end)
}

/// Parse a KV key back into topic and cursor.
///
/// # Errors
///
/// Returns an error if the key format is invalid.
pub fn parse_event_key(key: &str) -> Result<(Topic, Cursor)> {
    // Remove prefix
    let remainder = key.strip_prefix(PUBSUB_KEY_PREFIX).ok_or_else(|| super::error::PubSubError::KeyParseFailed {
        reason: format!("key '{}' does not start with prefix '{}'", key, PUBSUB_KEY_PREFIX),
    })?;

    // Split into segments
    let segments: Vec<&str> = remainder.split('/').collect();

    if segments.len() < 2 {
        return Err(super::error::PubSubError::KeyParseFailed {
            reason: format!("key '{}' has too few segments", key),
        });
    }

    // SAFETY: We verified segments.len() >= 2 above, so .last() is always Some.
    let cursor_hex = segments.last().unwrap();
    let cursor_index = u64::from_str_radix(cursor_hex, 16).map_err(|_| super::error::PubSubError::KeyParseFailed {
        reason: format!("invalid cursor hex '{}' in key '{}'", cursor_hex, key),
    })?;

    // Remaining segments are the topic
    let topic_segments = &segments[..segments.len() - 1];
    let topic_str = topic_segments.join(".");

    let topic = Topic::new(&topic_str).map_err(|e| super::error::PubSubError::KeyParseFailed {
        reason: format!("invalid topic in key '{}': {}", key, e),
    })?;

    Ok((topic, Cursor::from_index(cursor_index)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_event_key() {
        let topic = Topic::new("orders.created").unwrap();
        let cursor = Cursor::from_index(12345);

        let key = build_event_key(&topic, cursor);
        assert!(key.starts_with("__pubsub/events/"));
        assert!(key.contains("orders/created/"));
        assert!(key.ends_with("0000000000003039")); // 12345 in hex
    }

    #[test]
    fn test_build_topic_prefix() {
        let topic = Topic::new("orders.created").unwrap();
        let prefix = build_topic_prefix(&topic);
        assert_eq!(prefix, "__pubsub/events/orders/created/");
    }

    #[test]
    fn test_key_roundtrip() {
        let topic = Topic::new("orders.us.created").unwrap();
        let cursor = Cursor::from_index(99999);

        let key = build_event_key(&topic, cursor);
        let (parsed_topic, parsed_cursor) = parse_event_key(&key).unwrap();

        assert_eq!(parsed_topic, topic);
        assert_eq!(parsed_cursor, cursor);
    }

    #[test]
    fn test_key_ordering() {
        let topic = Topic::new("orders").unwrap();

        let key1 = build_event_key(&topic, Cursor::from_index(100));
        let key2 = build_event_key(&topic, Cursor::from_index(200));
        let key3 = build_event_key(&topic, Cursor::from_index(300));

        // Keys should be lexicographically ordered by cursor
        assert!(key1 < key2);
        assert!(key2 < key3);
    }

    #[test]
    fn test_cursor_range() {
        let topic = Topic::new("orders").unwrap();
        let (start, end) = build_cursor_range(&topic, Cursor::from_index(100));

        assert!(start.contains("0000000000000064")); // 100 in hex
        assert!(start < end);
    }
}
