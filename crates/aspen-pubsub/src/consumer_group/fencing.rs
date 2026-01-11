//! Fencing token validation for consumer groups.
//!
//! Fencing prevents stale consumers (those that have timed out but don't know it yet)
//! from interfering with active consumers. Each consumer operation validates
//! the fencing token against stored state.
//!
//! # Fencing Components
//!
//! - **Session ID**: Monotonically increasing per consumer rejoin
//! - **Fencing Token**: Random value generated per session for tamper detection
//! - **Generation ID**: Increments on each group rebalance

use crate::consumer_group::error::ConsumerGroupError;
use crate::consumer_group::error::Result;
use crate::consumer_group::types::ConsumerId;
use crate::consumer_group::types::ConsumerState;

/// Validate that a consumer operation is not from a stale session.
///
/// This prevents zombie consumers (those that have timed out but don't
/// know it yet) from interfering with active consumers.
///
/// # Validation Order
///
/// 1. Fencing token must not be zero (invalid)
/// 2. Fencing token must match stored state
/// 3. Session ID is implicitly validated via fencing token
///
/// # Arguments
///
/// * `consumer_id` - Consumer being validated
/// * `provided_fencing_token` - Token from the consumer's request
/// * `stored_state` - Current stored state for the consumer
///
/// # Errors
///
/// - `InvalidReceiptHandle` if fencing token is zero
/// - `FencingTokenMismatch` if tokens don't match
pub fn validate_fencing(
    consumer_id: &ConsumerId,
    provided_fencing_token: u64,
    stored_state: &ConsumerState,
) -> Result<()> {
    // Check fencing token is not zero (invalid/uninitialized)
    if provided_fencing_token == 0 {
        return Err(ConsumerGroupError::InvalidReceiptHandle {
            reason: "fencing token cannot be zero".to_string(),
        });
    }

    // Validate fencing token matches stored state
    if stored_state.fencing_token != provided_fencing_token {
        return Err(ConsumerGroupError::FencingTokenMismatch {
            consumer_id: consumer_id.as_str().to_string(),
            expected: stored_state.fencing_token,
            actual: provided_fencing_token,
        });
    }

    Ok(())
}

/// Generate a new fencing token.
///
/// Uses a combination of timestamp and randomness to ensure uniqueness
/// and prevent prediction attacks.
///
/// # Returns
///
/// A non-zero u64 suitable for use as a fencing token.
pub fn generate_fencing_token() -> u64 {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    // Mix with random bits for unpredictability
    let random: u64 = rand::random();

    // Combine: high bits from time, low bits from random
    // Ensure non-zero by ORing with 1
    ((timestamp << 20) | (random & 0xFFFFF)) | 1
}

/// Generate a monotonically increasing session ID.
///
/// Session IDs must increase on each join to detect stale sessions.
/// Uses the previous session ID + 1, or current timestamp if no previous.
///
/// # Arguments
///
/// * `previous_session_id` - The previous session ID, if any
///
/// # Returns
///
/// A new session ID guaranteed to be greater than the previous one.
pub fn next_session_id(previous_session_id: Option<u64>) -> u64 {
    match previous_session_id {
        Some(prev) => prev.saturating_add(1),
        None => {
            // First session - use timestamp for reasonable starting point
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consumer_group::types::ConsumerGroupId;
    use crate::consumer_group::types::PartitionId;

    fn make_consumer_state(fencing_token: u64, session_id: u64) -> ConsumerState {
        ConsumerState {
            consumer_id: ConsumerId::new_unchecked("consumer-1"),
            group_id: ConsumerGroupId::new_unchecked("group-1"),
            assigned_partitions: vec![PartitionId::new(0)],
            session_id,
            fencing_token,
            pending_count: 0,
            metadata: None,
            tags: vec![],
            joined_at_ms: 1000,
            last_heartbeat_ms: 1000,
        }
    }

    #[test]
    fn test_validate_fencing_success() {
        let consumer_id = ConsumerId::new_unchecked("consumer-1");
        let state = make_consumer_state(12345, 1);

        let result = validate_fencing(&consumer_id, 12345, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_fencing_zero_token_rejected() {
        let consumer_id = ConsumerId::new_unchecked("consumer-1");
        let state = make_consumer_state(12345, 1);

        let result = validate_fencing(&consumer_id, 0, &state);
        assert!(matches!(
            result,
            Err(ConsumerGroupError::InvalidReceiptHandle { .. })
        ));
    }

    #[test]
    fn test_validate_fencing_mismatch() {
        let consumer_id = ConsumerId::new_unchecked("consumer-1");
        let state = make_consumer_state(12345, 1);

        let result = validate_fencing(&consumer_id, 99999, &state);
        assert!(matches!(
            result,
            Err(ConsumerGroupError::FencingTokenMismatch { expected: 12345, actual: 99999, .. })
        ));
    }

    #[test]
    fn test_generate_fencing_token_non_zero() {
        for _ in 0..100 {
            let token = generate_fencing_token();
            assert_ne!(token, 0);
        }
    }

    #[test]
    fn test_generate_fencing_token_unique() {
        let tokens: Vec<u64> = (0..100).map(|_| generate_fencing_token()).collect();
        let unique: std::collections::HashSet<_> = tokens.iter().collect();
        // Should be highly unique (allow for rare collision)
        assert!(unique.len() >= 98);
    }

    #[test]
    fn test_next_session_id_increments() {
        assert_eq!(next_session_id(Some(1)), 2);
        assert_eq!(next_session_id(Some(100)), 101);
        assert_eq!(next_session_id(Some(u64::MAX - 1)), u64::MAX);
        // Saturating add prevents overflow
        assert_eq!(next_session_id(Some(u64::MAX)), u64::MAX);
    }

    #[test]
    fn test_next_session_id_initial() {
        let session = next_session_id(None);
        // Should be a reasonable timestamp (after 2020)
        assert!(session > 1577836800); // 2020-01-01
    }
}
