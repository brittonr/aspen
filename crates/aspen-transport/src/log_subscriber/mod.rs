//! Raft log subscription protocol for read-only clients.
//!
//! Provides a streaming interface for clients to receive committed Raft log
//! entries in real-time. This enables building reactive systems that respond
//! to state changes without polling.
//!
//! # Protocol Flow
//!
//! 1. Client connects via LOG_SUBSCRIBER_ALPN
//! 2. Server sends AuthChallenge (same as Raft auth)
//! 3. Client sends AuthResponse with valid HMAC
//! 4. Server sends AuthResult
//! 5. If authenticated, client sends SubscribeRequest
//! 6. Server streams LogEntryMessage for each committed entry
//!
//! # Design Notes
//!
//! - Read-only: Subscribers cannot modify state
//! - Authenticated: Uses same cookie-based auth as Raft RPC
//! - Bounded: Fixed buffer sizes and rate limits
//! - Resumable: Clients can specify starting log index
//!
//! # Tiger Style
//!
//! - Fixed message sizes with explicit bounds
//! - Explicit subscription limits (max subscribers, buffer size)
//! - Clear protocol versioning for forward compatibility

mod connection;
pub mod constants;
mod handler;
pub mod kv_operation;
mod state;
pub mod types;
pub mod wire;

// Re-export all public items for API compatibility
pub use constants::*;
pub use handler::LogSubscriberProtocolHandler;
pub use kv_operation::KvOperation;
pub use state::SubscriberState;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_request_from_index() {
        let req = SubscribeRequest::from_index(100);
        assert_eq!(req.start_index, 100);
        assert!(req.key_prefix.is_empty());
        assert_eq!(req.protocol_version, LOG_SUBSCRIBE_PROTOCOL_VERSION);
    }

    #[test]
    fn test_subscribe_request_latest_only() {
        let req = SubscribeRequest::latest_only();
        assert_eq!(req.start_index, u64::MAX);
    }

    #[test]
    fn test_subscribe_request_with_prefix() {
        let req = SubscribeRequest::with_prefix(50, b"users/".to_vec());
        assert_eq!(req.start_index, 50);
        assert_eq!(req.key_prefix, b"users/");
    }

    #[test]
    fn test_kv_operation_matches_prefix() {
        let set_op = KvOperation::Set {
            key: b"users/123".to_vec(),
            value: b"data".to_vec(),
        };

        assert!(set_op.matches_prefix(b""));
        assert!(set_op.matches_prefix(b"users/"));
        assert!(set_op.matches_prefix(b"users/123"));
        assert!(!set_op.matches_prefix(b"posts/"));

        let multi_op = KvOperation::SetMulti {
            pairs: vec![
                (b"users/1".to_vec(), b"a".to_vec()),
                (b"posts/1".to_vec(), b"b".to_vec()),
            ],
        };

        assert!(multi_op.matches_prefix(b"users/"));
        assert!(multi_op.matches_prefix(b"posts/"));
        assert!(!multi_op.matches_prefix(b"comments/"));

        // Control operations always match
        assert!(KvOperation::Noop.matches_prefix(b"anything"));
    }

    #[test]
    fn test_kv_operation_primary_key() {
        let set_op = KvOperation::Set {
            key: b"key1".to_vec(),
            value: b"val".to_vec(),
        };
        assert_eq!(set_op.primary_key(), Some(b"key1".as_slice()));

        let delete_op = KvOperation::Delete { key: b"key2".to_vec() };
        assert_eq!(delete_op.primary_key(), Some(b"key2".as_slice()));

        assert_eq!(KvOperation::Noop.primary_key(), None);
    }

    #[test]
    fn test_kv_operation_key_count() {
        assert_eq!(
            KvOperation::Set {
                key: vec![],
                value: vec![]
            }
            .key_count(),
            1
        );

        assert_eq!(
            KvOperation::SetMulti {
                pairs: vec![(vec![], vec![]), (vec![], vec![]), (vec![], vec![])]
            }
            .key_count(),
            3
        );

        assert_eq!(KvOperation::Noop.key_count(), 0);
    }

    #[test]
    fn test_subscriber_state_should_send() {
        let mut state = SubscriberState::new(1, [0u8; 32], b"users/".to_vec(), 0);

        let matching = KvOperation::Set {
            key: b"users/123".to_vec(),
            value: vec![],
        };
        assert!(state.should_send(&matching));

        let non_matching = KvOperation::Set {
            key: b"posts/456".to_vec(),
            value: vec![],
        };
        assert!(!state.should_send(&non_matching));

        // With empty prefix, everything matches
        state.key_prefix = vec![];
        assert!(state.should_send(&non_matching));
    }

    #[test]
    fn test_subscriber_state_record_sent() {
        let mut state = SubscriberState::new(1, [0u8; 32], vec![], 10);
        assert_eq!(state.last_sent_index, 9); // start_index - 1
        assert_eq!(state.entries_sent, 0);

        state.record_sent(10);
        assert_eq!(state.last_sent_index, 10);
        assert_eq!(state.entries_sent, 1);

        state.record_sent(11);
        assert_eq!(state.last_sent_index, 11);
        assert_eq!(state.entries_sent, 2);
    }

    #[test]
    fn test_subscribe_reject_reason_display() {
        assert_eq!(SubscribeRejectReason::TooManySubscribers.to_string(), "too many subscribers");
        assert_eq!(SubscribeRejectReason::IndexNotAvailable.to_string(), "requested index not available");
    }

    #[test]
    fn test_end_of_stream_reason_display() {
        assert_eq!(EndOfStreamReason::ServerShutdown.to_string(), "server shutdown");
        assert_eq!(EndOfStreamReason::Lagged.to_string(), "subscriber lagged");
    }
}
