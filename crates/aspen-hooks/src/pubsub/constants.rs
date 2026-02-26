//! Tiger Style resource limits for pub/sub operations.
//!
//! These constants enforce bounded resource usage following Tiger Style principles.
//! All limits are chosen to prevent unbounded memory allocation while supporting
//! practical pub/sub workloads.

/// Maximum number of segments in a topic name.
///
/// Topics like "orders.us.east.created" have 4 segments.
/// This limit prevents pathologically deep topic hierarchies.
pub const MAX_TOPIC_SEGMENTS: usize = 16;

/// Maximum length of a single topic segment in bytes.
///
/// Individual segments like "orders" or "created" must be under this limit.
pub const MAX_SEGMENT_LENGTH: usize = 256;

/// Maximum event payload size in bytes (1 MB).
///
/// Events larger than this should use blob offloading (Phase 2).
/// Matches `MAX_VALUE_SIZE` from aspen-core for consistency.
pub const MAX_PAYLOAD_SIZE: usize = 1_048_576;

/// Maximum number of headers per event.
///
/// Headers are key-value metadata attached to events (e.g., trace IDs).
pub const MAX_HEADERS: usize = 32;

/// Maximum size of a single header value in bytes (4 KB).
pub const MAX_HEADER_VALUE_SIZE: usize = 4_096;

/// Maximum size of a header key in bytes.
pub const MAX_HEADER_KEY_SIZE: usize = 256;

/// Maximum events in a batch publish operation.
///
/// Batch publishes go through Raft as a single atomic operation.
pub const MAX_PUBLISH_BATCH_SIZE: usize = 100;

/// Reserved key prefix for pub/sub events in the KV store.
///
/// All pub/sub keys start with this prefix to avoid collisions
/// with application data.
pub const PUBSUB_KEY_PREFIX: &str = "__pubsub/events/";

/// Reserved key prefix for pub/sub consumer groups (Phase 2).
pub const PUBSUB_GROUPS_PREFIX: &str = "__pubsub/groups/";

/// Separator character for topic segments.
pub const TOPIC_SEGMENT_SEPARATOR: char = '.';

/// Single-segment wildcard character (matches exactly one segment).
pub const WILDCARD_SINGLE: &str = "*";

/// Multi-segment wildcard character (matches zero or more segments).
pub const WILDCARD_MULTI: &str = ">";

// ============================================================================
// Compile-Time Constant Assertions
// ============================================================================

// Topic limits must be positive and reasonable
const _: () = assert!(MAX_TOPIC_SEGMENTS > 0);
const _: () = assert!(MAX_TOPIC_SEGMENTS >= 4); // allow typical hierarchies
const _: () = assert!(MAX_SEGMENT_LENGTH > 0);
const _: () = assert!(MAX_SEGMENT_LENGTH >= 64); // allow descriptive names

// Payload limits must be positive
const _: () = assert!(MAX_PAYLOAD_SIZE > 0);
const _: () = assert!(MAX_PAYLOAD_SIZE >= 1024); // allow typical payloads

// Header limits must be positive
const _: () = assert!(MAX_HEADERS > 0);
const _: () = assert!(MAX_HEADERS >= 8); // allow common headers
const _: () = assert!(MAX_HEADER_KEY_SIZE > 0);
const _: () = assert!(MAX_HEADER_VALUE_SIZE > 0);
const _: () = assert!(MAX_HEADER_KEY_SIZE < MAX_HEADER_VALUE_SIZE); // keys smaller than values

// Batch size must be positive
const _: () = assert!(MAX_PUBLISH_BATCH_SIZE > 0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefixes_are_valid() {
        // Prefixes should end with separator for proper namespacing
        assert!(PUBSUB_KEY_PREFIX.ends_with('/'));
        assert!(PUBSUB_GROUPS_PREFIX.ends_with('/'));

        // Prefixes should start with system prefix
        assert!(PUBSUB_KEY_PREFIX.starts_with("__"));
        assert!(PUBSUB_GROUPS_PREFIX.starts_with("__"));
    }
}
