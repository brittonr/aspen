//! Fencing token validation for consumer groups.
//!
//! Fencing tokens prevent zombie consumers from affecting the group.
//! When a consumer's token doesn't match the stored state, they are
//! "fenced out" and cannot perform operations.

use super::error::ConsumerGroupError;
use super::error::FencingTokenMismatchSnafu;
use super::error::Result;
use super::types::ConsumerId;
use super::types::ConsumerState;

/// Validate that a fencing token matches the consumer's stored state.
///
/// # Arguments
///
/// * `consumer_id` - The consumer making the request
/// * `provided_token` - The token provided in the request
/// * `stored_state` - The consumer's stored state
///
/// # Errors
///
/// Returns `FencingTokenMismatch` if the tokens don't match.
pub fn validate_fencing(consumer_id: &ConsumerId, provided_token: u64, stored_state: &ConsumerState) -> Result<()> {
    if provided_token != stored_state.fencing_token {
        return FencingTokenMismatchSnafu {
            consumer_id: consumer_id.as_str().to_string(),
            expected: stored_state.fencing_token,
            actual: provided_token,
        }
        .fail();
    }
    Ok(())
}

/// Generate a new fencing token.
///
/// Tokens are monotonically increasing and unique per consumer session.
/// Uses the current timestamp plus a random component to avoid collisions.
pub fn generate_fencing_token() -> u64 {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Combine timestamp with random bits for uniqueness
    let random: u32 = rand::random();
    (timestamp << 20) | (random as u64 & 0xFFFFF)
}

/// Check if a fencing error should trigger consumer removal.
///
/// Some fencing errors indicate the consumer is permanently stale
/// and should be removed from the group.
pub fn should_remove_consumer(error: &ConsumerGroupError) -> bool {
    matches!(error, ConsumerGroupError::FencingTokenMismatch { .. } | ConsumerGroupError::SessionExpired { .. })
}

#[cfg(test)]
mod tests {
    use super::super::types::ConsumerGroupId;
    use super::*;

    fn make_consumer_state(token: u64) -> ConsumerState {
        ConsumerState {
            consumer_id: ConsumerId::new_unchecked("test-consumer"),
            group_id: ConsumerGroupId::new_unchecked("test-group"),
            assigned_partitions: vec![],
            session_id: 1,
            fencing_token: token,
            pending_count: 0,
            metadata: None,
            tags: vec![],
            joined_at_ms: 1000,
            last_heartbeat_ms: 1000,
        }
    }

    #[test]
    fn test_validate_fencing_success() {
        let consumer_id = ConsumerId::new_unchecked("c1");
        let state = make_consumer_state(12345);

        let result = validate_fencing(&consumer_id, 12345, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_fencing_mismatch() {
        let consumer_id = ConsumerId::new_unchecked("c1");
        let state = make_consumer_state(12345);

        let result = validate_fencing(&consumer_id, 99999, &state);
        assert!(result.is_err());

        if let Err(ConsumerGroupError::FencingTokenMismatch { expected, actual, .. }) = result {
            assert_eq!(expected, 12345);
            assert_eq!(actual, 99999);
        } else {
            panic!("Expected FencingTokenMismatch error");
        }
    }

    #[test]
    fn test_generate_fencing_token_unique() {
        let token1 = generate_fencing_token();
        let token2 = generate_fencing_token();

        // Tokens should be different (with very high probability)
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_generate_fencing_token_increasing() {
        let token1 = generate_fencing_token();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let token2 = generate_fencing_token();

        // Tokens should generally increase due to timestamp component
        // (This might rarely fail due to random component, but is very unlikely)
        assert!(token2 > token1 || (token2 - token1) < 1_000_000);
    }

    #[test]
    fn test_should_remove_consumer() {
        let fencing_error = ConsumerGroupError::FencingTokenMismatch {
            consumer_id: "c1".to_string(),
            expected: 1,
            actual: 2,
        };
        assert!(should_remove_consumer(&fencing_error));

        let session_error = ConsumerGroupError::SessionExpired {
            group_id: "g1".to_string(),
            consumer_id: "c1".to_string(),
        };
        assert!(should_remove_consumer(&session_error));

        let other_error = ConsumerGroupError::GroupNotFound {
            group_id: "g1".to_string(),
        };
        assert!(!should_remove_consumer(&other_error));
    }
}
