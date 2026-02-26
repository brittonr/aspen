//! Error types for pub/sub operations.
//!
//! Uses snafu for structured error handling with context.

use snafu::Snafu;

use super::constants::MAX_HEADER_KEY_SIZE;
use super::constants::MAX_HEADER_VALUE_SIZE;
use super::constants::MAX_HEADERS;
use super::constants::MAX_PAYLOAD_SIZE;
use super::constants::MAX_PUBLISH_BATCH_SIZE;
use super::constants::MAX_SEGMENT_LENGTH;
use super::constants::MAX_TOPIC_SEGMENTS;

/// Errors that can occur during pub/sub operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum PubSubError {
    /// Topic has too many segments.
    #[snafu(display("topic has {segments} segments, maximum allowed is {MAX_TOPIC_SEGMENTS}"))]
    TopicTooManySegments {
        /// Number of segments in the topic.
        segments: usize,
    },

    /// Topic segment is too long.
    #[snafu(display("topic segment '{segment}' has {length} bytes, maximum allowed is {MAX_SEGMENT_LENGTH}"))]
    TopicSegmentTooLong {
        /// The segment that exceeded the limit.
        segment: String,
        /// Length of the segment in bytes.
        length: usize,
    },

    /// Topic segment is empty.
    #[snafu(display("topic segment cannot be empty (found empty segment at position {position})"))]
    TopicSegmentEmpty {
        /// Position of the empty segment (0-indexed).
        position: usize,
    },

    /// Topic name is empty.
    #[snafu(display("topic name cannot be empty"))]
    TopicEmpty,

    /// Topic contains invalid characters.
    #[snafu(display("topic segment '{segment}' contains invalid character '{character}'"))]
    TopicInvalidCharacter {
        /// The segment containing the invalid character.
        segment: String,
        /// The invalid character.
        character: char,
    },

    /// Pattern has invalid wildcard placement.
    #[snafu(display("invalid pattern: {reason}"))]
    PatternInvalid {
        /// Description of why the pattern is invalid.
        reason: String,
    },

    /// Event payload is too large.
    #[snafu(display("event payload has {size} bytes, maximum allowed is {MAX_PAYLOAD_SIZE}"))]
    PayloadTooLarge {
        /// Size of the payload in bytes.
        size: usize,
    },

    /// Too many headers on event.
    #[snafu(display("event has {count} headers, maximum allowed is {MAX_HEADERS}"))]
    TooManyHeaders {
        /// Number of headers on the event.
        count: usize,
    },

    /// Header key is too long.
    #[snafu(display("header key '{key}' has {length} bytes, maximum allowed is {MAX_HEADER_KEY_SIZE}"))]
    HeaderKeyTooLong {
        /// The header key that exceeded the limit.
        key: String,
        /// Length of the key in bytes.
        length: usize,
    },

    /// Header value is too large.
    #[snafu(display("header value for key '{key}' has {size} bytes, maximum allowed is {MAX_HEADER_VALUE_SIZE}"))]
    HeaderValueTooLarge {
        /// The header key.
        key: String,
        /// Size of the value in bytes.
        size: usize,
    },

    /// Batch publish has too many events.
    #[snafu(display("batch has {count} events, maximum allowed is {MAX_PUBLISH_BATCH_SIZE}"))]
    BatchTooLarge {
        /// Number of events in the batch.
        count: usize,
    },

    /// Failed to encode event.
    #[snafu(display("failed to encode event: {source}"))]
    EncodingFailed {
        /// The underlying encoding error.
        source: rmp_serde::encode::Error,
    },

    /// Failed to decode event.
    #[snafu(display("failed to decode event: {source}"))]
    DecodingFailed {
        /// The underlying decoding error.
        source: rmp_serde::decode::Error,
    },

    /// Key-value store operation failed.
    #[snafu(display("key-value store operation failed: {source}"))]
    KvStoreFailed {
        /// The underlying KV store error.
        source: aspen_core::KeyValueStoreError,
    },

    /// Failed to parse key as topic.
    #[snafu(display("failed to parse key as topic: {reason}"))]
    KeyParseFailed {
        /// Description of why parsing failed.
        reason: String,
    },

    /// Subspace operation failed.
    #[snafu(display("subspace operation failed: {source}"))]
    SubspaceFailed {
        /// The underlying subspace error.
        source: aspen_core::layer::SubspaceError,
    },

    /// Tuple operation failed.
    #[snafu(display("tuple operation failed: {source}"))]
    TupleFailed {
        /// The underlying tuple error.
        source: aspen_core::layer::TupleError,
    },

    /// Received unexpected operation type from log stream.
    #[snafu(display("received unexpected operation type: expected Set, got {operation}"))]
    UnexpectedOperation {
        /// Description of the unexpected operation.
        operation: String,
    },

    /// Connection to node failed.
    #[snafu(display("connection failed: {reason}"))]
    ConnectionFailed {
        /// Description of why connection failed.
        reason: String,
    },

    /// Authentication failed.
    #[snafu(display("authentication failed: {reason}"))]
    AuthenticationFailed {
        /// Description of why authentication failed.
        reason: String,
    },

    /// Subscription was rejected.
    #[snafu(display("subscription rejected: {reason}"))]
    SubscriptionRejected {
        /// Reason for rejection.
        reason: String,
    },

    /// Stream ended unexpectedly.
    #[snafu(display("stream ended: {reason}"))]
    StreamEnded {
        /// Reason for stream ending.
        reason: String,
    },

    /// IO error during stream operations.
    #[snafu(display("IO error: {source}"))]
    IoError {
        /// The underlying IO error.
        source: std::io::Error,
    },
}

impl From<aspen_core::KeyValueStoreError> for PubSubError {
    fn from(source: aspen_core::KeyValueStoreError) -> Self {
        PubSubError::KvStoreFailed { source }
    }
}

impl From<aspen_core::layer::SubspaceError> for PubSubError {
    fn from(source: aspen_core::layer::SubspaceError) -> Self {
        PubSubError::SubspaceFailed { source }
    }
}

impl From<aspen_core::layer::TupleError> for PubSubError {
    fn from(source: aspen_core::layer::TupleError) -> Self {
        PubSubError::TupleFailed { source }
    }
}

impl From<rmp_serde::encode::Error> for PubSubError {
    fn from(source: rmp_serde::encode::Error) -> Self {
        PubSubError::EncodingFailed { source }
    }
}

impl From<rmp_serde::decode::Error> for PubSubError {
    fn from(source: rmp_serde::decode::Error) -> Self {
        PubSubError::DecodingFailed { source }
    }
}

impl From<std::io::Error> for PubSubError {
    fn from(source: std::io::Error) -> Self {
        PubSubError::IoError { source }
    }
}

/// Result type for pub/sub operations.
pub type Result<T> = std::result::Result<T, PubSubError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PubSubError::TopicTooManySegments { segments: 20 };
        assert!(err.to_string().contains("20"));
        assert!(err.to_string().contains("16")); // MAX_TOPIC_SEGMENTS

        let err = PubSubError::TopicSegmentTooLong {
            segment: "test".to_string(),
            length: 300,
        };
        assert!(err.to_string().contains("test"));
        assert!(err.to_string().contains("300"));

        let err = PubSubError::PayloadTooLarge { size: 2_000_000 };
        assert!(err.to_string().contains("2000000"));
    }

    #[test]
    fn test_error_from_conversions() {
        // Test that From conversions compile (actual testing requires real errors)
        fn accepts_pubsub_error(_: PubSubError) {}

        // These should compile, showing From is implemented
        let _ = |e: aspen_core::KeyValueStoreError| accepts_pubsub_error(e.into());
        let _ = |e: aspen_core::layer::SubspaceError| accepts_pubsub_error(e.into());
        let _ = |e: aspen_core::layer::TupleError| accepts_pubsub_error(e.into());
    }
}
