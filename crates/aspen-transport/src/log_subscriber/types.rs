//! Protocol types for the log subscriber protocol.

use aspen_core::hlc::SerializableTimestamp;
use serde::Deserialize;
use serde::Serialize;

use super::constants::LOG_SUBSCRIBE_PROTOCOL_VERSION;
use super::kv_operation::KvOperation;

/// Trait for reading historical log entries for replay.
///
/// Implementations should return log entries in the given range, converting
/// from the internal Raft log format to `LogEntryPayload`.
#[async_trait::async_trait]
pub trait HistoricalLogReader: Send + Sync + std::fmt::Debug {
    /// Fetch log entries in the given range [start, end].
    ///
    /// Returns entries ordered by index. If the start index has been
    /// purged (compacted), returns entries starting from the earliest
    /// available index.
    ///
    /// # Arguments
    /// * `start_index` - First log index to fetch (inclusive)
    /// * `end_index` - Last log index to fetch (inclusive)
    ///
    /// # Returns
    /// * `Ok(entries)` - Vector of log entries in the range
    /// * `Err(error)` - If reading fails
    async fn read_entries(&self, start_index: u64, end_index: u64) -> Result<Vec<LogEntryPayload>, std::io::Error>;

    /// Get the earliest available log index (after compaction).
    ///
    /// Returns `None` if no logs exist yet.
    async fn earliest_available_index(&self) -> Result<Option<u64>, std::io::Error>;
}

/// Subscription request from client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    /// Starting log index (0 = from beginning, u64::MAX = latest only).
    pub start_index: u64,
    /// Optional key prefix filter (empty = all keys).
    pub key_prefix: Vec<u8>,
    /// Protocol version for compatibility checking.
    pub protocol_version: u8,
}

impl SubscribeRequest {
    /// Create a subscription starting from a specific log index.
    pub fn from_index(index: u64) -> Self {
        Self {
            start_index: index,
            key_prefix: Vec::new(),
            protocol_version: LOG_SUBSCRIBE_PROTOCOL_VERSION,
        }
    }

    /// Create a subscription for only the latest entries.
    pub fn latest_only() -> Self {
        Self {
            start_index: u64::MAX,
            key_prefix: Vec::new(),
            protocol_version: LOG_SUBSCRIBE_PROTOCOL_VERSION,
        }
    }

    /// Create a subscription with a key prefix filter.
    pub fn with_prefix(start_index: u64, prefix: impl Into<Vec<u8>>) -> Self {
        Self {
            start_index,
            key_prefix: prefix.into(),
            protocol_version: LOG_SUBSCRIBE_PROTOCOL_VERSION,
        }
    }
}

/// Response to subscription request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubscribeResponse {
    /// Subscription accepted, streaming will begin.
    Accepted {
        /// Current committed index at time of subscription.
        current_index: u64,
        /// Node ID of the server.
        node_id: u64,
    },
    /// Subscription rejected.
    Rejected {
        /// Reason for rejection.
        reason: SubscribeRejectReason,
    },
}

/// Reasons why a subscription might be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscribeRejectReason {
    /// Too many subscribers connected.
    TooManySubscribers,
    /// Requested start index is not available (compacted).
    IndexNotAvailable,
    /// Protocol version not supported.
    UnsupportedVersion,
    /// Server is not ready to accept subscriptions.
    NotReady,
    /// Generic internal error.
    InternalError,
}

impl std::fmt::Display for SubscribeRejectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManySubscribers => write!(f, "too many subscribers"),
            Self::IndexNotAvailable => write!(f, "requested index not available"),
            Self::UnsupportedVersion => write!(f, "protocol version not supported"),
            Self::NotReady => write!(f, "server not ready"),
            Self::InternalError => write!(f, "internal error"),
        }
    }
}

/// A streamed log entry message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEntryMessage {
    /// A committed log entry.
    Entry(LogEntryPayload),
    /// Keepalive message (sent periodically on idle connections).
    Keepalive {
        /// Current committed index.
        committed_index: u64,
        /// HLC timestamp of the keepalive.
        hlc_timestamp: SerializableTimestamp,
    },
    /// Stream is ending (server shutting down or error).
    EndOfStream {
        /// Reason for stream termination.
        reason: EndOfStreamReason,
    },
}

/// Reasons why a log stream might end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndOfStreamReason {
    /// Server is shutting down gracefully.
    ServerShutdown,
    /// Client requested disconnect.
    ClientDisconnect,
    /// Subscriber fell too far behind.
    Lagged,
    /// Internal error occurred.
    InternalError,
}

impl std::fmt::Display for EndOfStreamReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServerShutdown => write!(f, "server shutdown"),
            Self::ClientDisconnect => write!(f, "client disconnect"),
            Self::Lagged => write!(f, "subscriber lagged"),
            Self::InternalError => write!(f, "internal error"),
        }
    }
}

/// Payload of a committed log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntryPayload {
    /// Log index of this entry.
    pub index: u64,
    /// Raft term when entry was created.
    pub term: u64,
    /// HLC timestamp when entry was committed.
    pub hlc_timestamp: SerializableTimestamp,
    /// The operation that was committed.
    pub operation: KvOperation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_request_from_index() {
        let req = SubscribeRequest::from_index(42);
        assert_eq!(req.start_index, 42);
        assert!(req.key_prefix.is_empty());
        assert_eq!(req.protocol_version, LOG_SUBSCRIBE_PROTOCOL_VERSION);
    }

    #[test]
    fn test_subscribe_request_latest_only() {
        let req = SubscribeRequest::latest_only();
        assert_eq!(req.start_index, u64::MAX);
        assert!(req.key_prefix.is_empty());
    }

    #[test]
    fn test_subscribe_request_with_prefix() {
        let req = SubscribeRequest::with_prefix(10, b"user:".to_vec());
        assert_eq!(req.start_index, 10);
        assert_eq!(req.key_prefix, b"user:");
    }

    #[test]
    fn test_subscribe_request_serde_roundtrip() {
        let req = SubscribeRequest::with_prefix(100, b"kv:".to_vec());
        let bytes = postcard::to_allocvec(&req).unwrap();
        let decoded: SubscribeRequest = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.start_index, 100);
        assert_eq!(decoded.key_prefix, b"kv:");
    }

    #[test]
    fn test_subscribe_response_accepted_serde() {
        let resp = SubscribeResponse::Accepted {
            current_index: 999,
            node_id: 7,
        };
        let bytes = postcard::to_allocvec(&resp).unwrap();
        let decoded: SubscribeResponse = postcard::from_bytes(&bytes).unwrap();
        match decoded {
            SubscribeResponse::Accepted { current_index, node_id } => {
                assert_eq!(current_index, 999);
                assert_eq!(node_id, 7);
            }
            _ => panic!("expected Accepted"),
        }
    }

    #[test]
    fn test_subscribe_response_rejected_serde() {
        let resp = SubscribeResponse::Rejected {
            reason: SubscribeRejectReason::TooManySubscribers,
        };
        let bytes = postcard::to_allocvec(&resp).unwrap();
        let decoded: SubscribeResponse = postcard::from_bytes(&bytes).unwrap();
        match decoded {
            SubscribeResponse::Rejected { reason } => {
                assert_eq!(reason, SubscribeRejectReason::TooManySubscribers);
            }
            _ => panic!("expected Rejected"),
        }
    }

    #[test]
    fn test_subscribe_reject_reason_display() {
        assert_eq!(SubscribeRejectReason::TooManySubscribers.to_string(), "too many subscribers");
        assert_eq!(SubscribeRejectReason::IndexNotAvailable.to_string(), "requested index not available");
        assert_eq!(SubscribeRejectReason::UnsupportedVersion.to_string(), "protocol version not supported");
        assert_eq!(SubscribeRejectReason::NotReady.to_string(), "server not ready");
        assert_eq!(SubscribeRejectReason::InternalError.to_string(), "internal error");
    }

    #[test]
    fn test_end_of_stream_reason_display() {
        assert_eq!(EndOfStreamReason::ServerShutdown.to_string(), "server shutdown");
        assert_eq!(EndOfStreamReason::ClientDisconnect.to_string(), "client disconnect");
        assert_eq!(EndOfStreamReason::Lagged.to_string(), "subscriber lagged");
        assert_eq!(EndOfStreamReason::InternalError.to_string(), "internal error");
    }

    #[test]
    fn test_log_entry_message_keepalive_serde() {
        let ts = SerializableTimestamp::from_millis(0);
        let msg = LogEntryMessage::Keepalive {
            committed_index: 42,
            hlc_timestamp: ts,
        };
        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: LogEntryMessage = postcard::from_bytes(&bytes).unwrap();
        match decoded {
            LogEntryMessage::Keepalive { committed_index, .. } => {
                assert_eq!(committed_index, 42);
            }
            _ => panic!("expected Keepalive"),
        }
    }

    #[test]
    fn test_log_entry_message_end_of_stream_serde() {
        let msg = LogEntryMessage::EndOfStream {
            reason: EndOfStreamReason::Lagged,
        };
        let bytes = postcard::to_allocvec(&msg).unwrap();
        let decoded: LogEntryMessage = postcard::from_bytes(&bytes).unwrap();
        match decoded {
            LogEntryMessage::EndOfStream { reason } => {
                assert_eq!(reason, EndOfStreamReason::Lagged);
            }
            _ => panic!("expected EndOfStream"),
        }
    }

    #[test]
    fn test_subscribe_reject_reason_equality() {
        assert_eq!(SubscribeRejectReason::TooManySubscribers, SubscribeRejectReason::TooManySubscribers);
        assert_ne!(SubscribeRejectReason::TooManySubscribers, SubscribeRejectReason::NotReady);
    }

    #[test]
    fn test_end_of_stream_reason_equality() {
        assert_eq!(EndOfStreamReason::Lagged, EndOfStreamReason::Lagged);
        assert_ne!(EndOfStreamReason::Lagged, EndOfStreamReason::ServerShutdown);
    }

    #[test]
    fn test_subscribe_request_clone() {
        let req = SubscribeRequest::with_prefix(5, b"test:".to_vec());
        let cloned = req.clone();
        assert_eq!(cloned.start_index, 5);
        assert_eq!(cloned.key_prefix, b"test:");
    }

    #[test]
    fn test_subscribe_reject_reason_copy() {
        let reason = SubscribeRejectReason::NotReady;
        let copied = reason;
        assert_eq!(reason, copied);
    }

    #[test]
    fn test_end_of_stream_reason_copy() {
        let reason = EndOfStreamReason::ServerShutdown;
        let copied = reason;
        assert_eq!(reason, copied);
    }

    #[test]
    fn test_subscribe_request_debug() {
        let req = SubscribeRequest::from_index(0);
        let debug = format!("{:?}", req);
        assert!(debug.contains("SubscribeRequest"));
        assert!(debug.contains("start_index"));
    }
}
