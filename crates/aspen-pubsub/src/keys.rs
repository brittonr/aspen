//! Key encoding for pub/sub topics.
//!
//! Uses FoundationDB-style Tuple and Subspace primitives from aspen-layer
//! to create order-preserving keys that enable efficient prefix scans.

use aspen_core::layer::Element;
use aspen_core::layer::Subspace;
use aspen_core::layer::Tuple;

use crate::constants::PUBSUB_KEY_PREFIX;
use crate::error::KeyParseFailedSnafu;
use crate::error::PubSubError;
use crate::error::Result;
use crate::topic::Topic;
use crate::topic::TopicPattern;

/// Convert a topic to a KV key.
///
/// The key format is: `__pubsub/events/{segment1}/{segment2}/...`
///
/// Using Tuple encoding ensures lexicographic ordering that matches
/// topic hierarchy, enabling efficient prefix scans.
///
/// # Examples
///
/// ```
/// use aspen_pubsub::{Topic, topic_to_key};
///
/// let topic = Topic::new("orders.created").unwrap();
/// let key = topic_to_key(&topic);
/// // Key encodes as: __pubsub/events/orders/created
/// ```
pub fn topic_to_key(topic: &Topic) -> Vec<u8> {
    let subspace = pubsub_subspace();

    let mut tuple = Tuple::new();
    for segment in topic.segments() {
        tuple = tuple.push(segment);
    }

    subspace.pack(&tuple)
}

/// Convert a KV key back to a topic.
///
/// # Errors
///
/// Returns an error if the key is not a valid pub/sub key.
pub fn key_to_topic(key: &[u8]) -> Result<Topic> {
    let subspace = pubsub_subspace();

    // Unpack the key to get the tuple (this removes the subspace prefix)
    let tuple = subspace.unpack(key)?;

    // Extract string segments from the tuple
    let mut segments = Vec::new();
    for i in 0..tuple.len() {
        let element = tuple.get(i).unwrap();
        match element {
            Element::String(s) => segments.push(s.clone()),
            Element::Bytes(b) => {
                // Try to interpret bytes as UTF-8 string
                let s = std::str::from_utf8(b).map_err(|_| PubSubError::KeyParseFailed {
                    reason: "topic segment is not valid UTF-8".to_string(),
                })?;
                segments.push(s.to_string());
            }
            _ => {
                return KeyParseFailedSnafu {
                    reason: format!("unexpected element type in key: {element:?}"),
                }
                .fail();
            }
        }
    }

    if segments.is_empty() {
        return KeyParseFailedSnafu {
            reason: "key contains no topic segments".to_string(),
        }
        .fail();
    }

    let topic_str = segments.join(".");
    Topic::new(topic_str).map_err(|e| PubSubError::KeyParseFailed {
        reason: format!("invalid topic from key: {e}"),
    })
}

/// Convert a topic pattern to a key prefix for server-side filtering.
///
/// Returns the encoded prefix up to the first wildcard. The server
/// uses this for efficient prefix filtering, then the client does
/// final wildcard matching.
///
/// # Examples
///
/// ```
/// use aspen_pubsub::{TopicPattern, pattern_to_prefix};
///
/// // Literal pattern - full key prefix
/// let pattern = TopicPattern::new("orders.us.created").unwrap();
/// let prefix = pattern_to_prefix(&pattern);
///
/// // Pattern with wildcard - prefix up to wildcard
/// let pattern = TopicPattern::new("orders.*").unwrap();
/// let prefix = pattern_to_prefix(&pattern);
/// // Prefix encodes as: __pubsub/events/orders/
///
/// // Pattern starting with wildcard - just the subspace prefix
/// let pattern = TopicPattern::new("*.created").unwrap();
/// let prefix = pattern_to_prefix(&pattern);
/// // Prefix encodes as: __pubsub/events/
/// ```
pub fn pattern_to_prefix(pattern: &TopicPattern) -> Vec<u8> {
    let subspace = pubsub_subspace();
    let literal_prefix = pattern.literal_prefix();

    if literal_prefix.is_empty() {
        // No literal prefix, return just the subspace prefix
        return subspace.raw_prefix().to_vec();
    }

    // Build tuple from literal segments
    let mut tuple = Tuple::new();
    for segment in literal_prefix {
        tuple = tuple.push(segment);
    }

    // If the pattern has wildcards after the literal prefix, we want
    // the prefix that includes all keys starting with these segments.
    // We use the subspace.pack() which gives us the prefix bytes.
    if pattern.has_wildcards() {
        // Return the packed prefix (all keys starting with this will match)
        subspace.pack(&tuple)
    } else {
        // Exact match - return full key
        subspace.pack(&tuple)
    }
}

/// Check if a key belongs to the pub/sub events namespace.
pub fn is_pubsub_key(key: &[u8]) -> bool {
    let subspace = pubsub_subspace();
    subspace.contains(key)
}

/// Get the pub/sub events subspace.
fn pubsub_subspace() -> Subspace {
    // Build subspace from prefix
    // Note: We use push_bytes to handle the raw prefix bytes
    Subspace::new(Tuple::new().push(PUBSUB_KEY_PREFIX))
}

/// Convert a key to a string for the KV store.
///
/// The KV store uses String keys, so we need to convert
/// the binary key to a string (using lossy UTF-8 conversion
/// since our keys are actually valid ASCII/UTF-8 from tuple encoding).
pub fn key_to_string(key: &[u8]) -> String {
    // Tuple encoding produces ASCII-safe bytes, but we use
    // lossy conversion for safety
    String::from_utf8_lossy(key).to_string()
}

/// Convert a string key back to bytes.
pub fn string_to_key(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_to_key_roundtrip() {
        let topic = Topic::new("orders.created").unwrap();
        let key = topic_to_key(&topic);
        let decoded = key_to_topic(&key).unwrap();
        assert_eq!(topic, decoded);
    }

    #[test]
    fn test_topic_to_key_single_segment() {
        let topic = Topic::new("orders").unwrap();
        let key = topic_to_key(&topic);
        let decoded = key_to_topic(&key).unwrap();
        assert_eq!(topic, decoded);
    }

    #[test]
    fn test_topic_to_key_multiple_segments() {
        let topic = Topic::new("orders.us.east.created").unwrap();
        let key = topic_to_key(&topic);
        let decoded = key_to_topic(&key).unwrap();
        assert_eq!(topic, decoded);
    }

    #[test]
    fn test_topic_key_ordering() {
        // Keys should be ordered lexicographically in a way that
        // groups topics by prefix
        let key1 = topic_to_key(&Topic::new("orders.a").unwrap());
        let key2 = topic_to_key(&Topic::new("orders.b").unwrap());
        let key3 = topic_to_key(&Topic::new("orders.b.extra").unwrap());
        let key4 = topic_to_key(&Topic::new("users.a").unwrap());

        assert!(key1 < key2);
        assert!(key2 < key3);
        assert!(key3 < key4);
    }

    #[test]
    fn test_pattern_to_prefix_literal() {
        let pattern = TopicPattern::new("orders.us.created").unwrap();
        let prefix = pattern_to_prefix(&pattern);

        // The prefix should match the exact topic key
        let topic_key = topic_to_key(&Topic::new("orders.us.created").unwrap());
        assert_eq!(prefix, topic_key);
    }

    #[test]
    fn test_pattern_to_prefix_with_wildcard() {
        let pattern = TopicPattern::new("orders.*").unwrap();
        let prefix = pattern_to_prefix(&pattern);

        // For prefix filtering, we want keys that START with prefix
        // so prefix of "orders.*" should match "orders.created", "orders.updated"

        // Check that keys starting with "orders" have our prefix
        let key1 = topic_to_key(&Topic::new("orders.created").unwrap());
        let key2 = topic_to_key(&Topic::new("orders.updated").unwrap());

        assert!(key1.starts_with(&prefix));
        assert!(key2.starts_with(&prefix));

        // But a different topic shouldn't match
        let key3 = topic_to_key(&Topic::new("users.created").unwrap());
        assert!(!key3.starts_with(&prefix));
    }

    #[test]
    fn test_pattern_to_prefix_starts_with_wildcard() {
        let pattern = TopicPattern::new("*.created").unwrap();
        let prefix = pattern_to_prefix(&pattern);

        // Prefix should be just the subspace prefix (matches all pub/sub keys)
        let subspace = pubsub_subspace();
        assert_eq!(prefix, subspace.raw_prefix());
    }

    #[test]
    fn test_is_pubsub_key() {
        let topic = Topic::new("orders.created").unwrap();
        let key = topic_to_key(&topic);
        assert!(is_pubsub_key(&key));

        // Non-pub/sub key should not match
        assert!(!is_pubsub_key(b"some/other/key"));
    }

    #[test]
    fn test_key_string_conversion() {
        let topic = Topic::new("orders.created").unwrap();
        let key = topic_to_key(&topic);
        let key_str = key_to_string(&key);
        let key_back = string_to_key(&key_str);
        assert_eq!(key, key_back);
    }
}
