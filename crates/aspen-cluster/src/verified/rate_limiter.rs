//! Pure rate limiter computation functions.
//!
//! This module contains pure functions for token bucket rate limiting.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Uses saturating arithmetic for all calculations
//! - Time is passed explicitly (no calls to system time)
//! - Deterministic behavior for testing and verification

// ============================================================================
// Token Bucket Types
// ============================================================================

/// Immutable token bucket configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenBucketConfig {
    /// Maximum tokens available in the bucket.
    pub capacity: u64,
    /// Tokens replenished per second.
    pub rate_per_sec: f64,
}

impl TokenBucketConfig {
    /// Build a token bucket configuration from per-minute rate and burst.
    #[inline]
    pub fn new(rate_per_minute: u32, burst: u32) -> Self {
        Self {
            capacity: u64::from(burst),
            rate_per_sec: f64::from(rate_per_minute) / 60.0,
        }
    }
}

/// Pure token bucket state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenBucketState {
    /// Current number of available tokens.
    pub tokens: f64,
    /// Last timestamp used to update the bucket.
    pub last_update_ms: u64,
}

impl TokenBucketState {
    /// Create a full bucket at the given timestamp.
    #[inline]
    pub fn full(config: TokenBucketConfig, now_ms: u64) -> Self {
        Self {
            tokens: config.capacity as f64,
            last_update_ms: now_ms,
        }
    }
}

/// Per-peer limiter state used by the two-tier gossip limiter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeerRateState {
    /// Token bucket for this peer.
    pub bucket: TokenBucketState,
    /// Last access time used for bounded-LRU eviction.
    pub last_access_ms: u64,
}

/// Reason a two-tier rate limit decision denied traffic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoTierRateLimitReason {
    /// Shared global budget was exhausted.
    Global,
    /// Per-peer budget was exhausted.
    PerPeer,
}

/// Result of a pure bucket transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BucketTransition {
    /// State after applying time advancement and optional consumption.
    pub next_state: TokenBucketState,
    /// Outcome of the consume attempt.
    pub result: TokenConsumptionResult,
}

/// Result of a pure gossip admission decision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GossipAdmissionTransition {
    /// Overall allow/deny outcome.
    pub outcome: Result<(), TwoTierRateLimitReason>,
    /// Global bucket state to commit.
    pub next_global: TokenBucketState,
    /// Per-peer state to commit when the shell should update or insert it.
    pub next_peer: Option<PeerRateState>,
}

// ============================================================================
// Token Replenishment
// ============================================================================

/// Calculate available tokens after replenishment.
///
/// Implements the token bucket algorithm where tokens are added
/// at a constant rate up to a maximum capacity.
///
/// # Arguments
///
/// * `current_tokens` - Current token count (can be fractional)
/// * `elapsed_ms` - Milliseconds elapsed since last update
/// * `rate_per_sec` - Tokens added per second
/// * `capacity` - Maximum token capacity
///
/// # Returns
///
/// Available tokens after replenishment (capped at capacity).
#[inline]
pub fn calculate_replenished_tokens(current_tokens: f64, elapsed_ms: u64, rate_per_sec: f64, capacity: u64) -> f64 {
    let elapsed_secs = elapsed_ms as f64 / 1000.0;
    let replenished = elapsed_secs * rate_per_sec;
    (current_tokens + replenished).min(capacity as f64)
}

/// Clamp a timestamp so time never goes backward for a state machine.
#[inline]
pub fn monotonic_timestamp_ms(previous_ms: u64, requested_ms: u64) -> u64 {
    previous_ms.max(requested_ms)
}

/// Advance a token bucket to the provided timestamp without consuming a token.
#[inline]
pub fn advance_token_bucket(state: TokenBucketState, config: TokenBucketConfig, now_ms: u64) -> TokenBucketState {
    let effective_now_ms = monotonic_timestamp_ms(state.last_update_ms, now_ms);
    let elapsed_ms = effective_now_ms.saturating_sub(state.last_update_ms);
    let tokens = calculate_replenished_tokens(state.tokens, elapsed_ms, config.rate_per_sec, config.capacity);

    TokenBucketState {
        tokens,
        last_update_ms: effective_now_ms,
    }
}

// ============================================================================
// Token Consumption
// ============================================================================

/// Result of attempting to consume tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenConsumptionResult {
    /// Token was consumed successfully.
    /// Contains remaining tokens after consumption.
    Consumed { remaining: f64 },
    /// Not enough tokens available.
    /// Contains current available tokens.
    Denied { available: f64 },
}

impl TokenConsumptionResult {
    /// Check if the token was consumed.
    #[inline]
    pub fn is_consumed(&self) -> bool {
        matches!(self, TokenConsumptionResult::Consumed { .. })
    }
}

/// Check if a token can be consumed from the bucket.
#[inline]
pub fn can_consume_token(available_tokens: f64) -> TokenConsumptionResult {
    if available_tokens >= 1.0 {
        TokenConsumptionResult::Consumed {
            remaining: available_tokens - 1.0,
        }
    } else {
        TokenConsumptionResult::Denied {
            available: available_tokens,
        }
    }
}

/// Advance a bucket to `now_ms` and try to consume one token.
#[inline]
pub fn try_consume_token_at(state: TokenBucketState, config: TokenBucketConfig, now_ms: u64) -> BucketTransition {
    let advanced_state = advance_token_bucket(state, config, now_ms);
    let result = can_consume_token(advanced_state.tokens);
    let tokens = match result {
        TokenConsumptionResult::Consumed { remaining } => remaining,
        TokenConsumptionResult::Denied { available } => available,
    };

    BucketTransition {
        next_state: TokenBucketState {
            tokens,
            last_update_ms: advanced_state.last_update_ms,
        },
        result,
    }
}

/// Update LRU access time without allowing backward timestamps.
#[inline]
pub fn update_last_access_ms(previous_ms: u64, requested_ms: u64) -> u64 {
    monotonic_timestamp_ms(previous_ms, requested_ms)
}

/// Evaluate a two-tier gossip limiter admission decision atomically.
///
/// The returned state advances time for the dimensions that should move forward
/// for the final outcome. A per-peer denial does not consume shared global
/// budget, while accepted traffic consumes both per-peer and global tokens.
#[inline]
pub fn evaluate_gossip_admission(
    global_state: TokenBucketState,
    global_config: TokenBucketConfig,
    peer_state: Option<PeerRateState>,
    peer_config: TokenBucketConfig,
    now_ms: u64,
) -> GossipAdmissionTransition {
    match peer_state {
        Some(existing_peer) => {
            evaluate_existing_peer_admission(global_state, global_config, existing_peer, peer_config, now_ms)
        }
        None => evaluate_new_peer_admission(global_state, global_config, peer_config, now_ms),
    }
}

#[inline]
fn evaluate_existing_peer_admission(
    global_state: TokenBucketState,
    global_config: TokenBucketConfig,
    peer_state: PeerRateState,
    peer_config: TokenBucketConfig,
    now_ms: u64,
) -> GossipAdmissionTransition {
    let peer_transition = try_consume_token_at(peer_state.bucket, peer_config, now_ms);
    let next_peer = PeerRateState {
        bucket: peer_transition.next_state,
        last_access_ms: update_last_access_ms(peer_state.last_access_ms, now_ms),
    };

    if !peer_transition.result.is_consumed() {
        return GossipAdmissionTransition {
            outcome: Err(TwoTierRateLimitReason::PerPeer),
            next_global: advance_token_bucket(global_state, global_config, now_ms),
            next_peer: Some(next_peer),
        };
    }

    let global_transition = try_consume_token_at(global_state, global_config, now_ms);
    if !global_transition.result.is_consumed() {
        return GossipAdmissionTransition {
            outcome: Err(TwoTierRateLimitReason::Global),
            next_global: global_transition.next_state,
            next_peer: None,
        };
    }

    GossipAdmissionTransition {
        outcome: Ok(()),
        next_global: global_transition.next_state,
        next_peer: Some(next_peer),
    }
}

#[inline]
fn evaluate_new_peer_admission(
    global_state: TokenBucketState,
    global_config: TokenBucketConfig,
    peer_config: TokenBucketConfig,
    now_ms: u64,
) -> GossipAdmissionTransition {
    let initial_peer = PeerRateState {
        bucket: TokenBucketState::full(peer_config, now_ms),
        last_access_ms: now_ms,
    };
    let peer_transition = try_consume_token_at(initial_peer.bucket, peer_config, now_ms);

    if !peer_transition.result.is_consumed() {
        return GossipAdmissionTransition {
            outcome: Err(TwoTierRateLimitReason::PerPeer),
            next_global: advance_token_bucket(global_state, global_config, now_ms),
            next_peer: None,
        };
    }

    let global_transition = try_consume_token_at(global_state, global_config, now_ms);
    if !global_transition.result.is_consumed() {
        return GossipAdmissionTransition {
            outcome: Err(TwoTierRateLimitReason::Global),
            next_global: global_transition.next_state,
            next_peer: None,
        };
    }

    GossipAdmissionTransition {
        outcome: Ok(()),
        next_global: global_transition.next_state,
        next_peer: Some(PeerRateState {
            bucket: peer_transition.next_state,
            last_access_ms: now_ms,
        }),
    }
}

// ============================================================================
// LRU Eviction
// ============================================================================

/// Determine if the oldest entry should be evicted from the map.
#[inline]
pub fn should_evict_oldest(current_len: u32, max_capacity: u32) -> bool {
    current_len >= max_capacity
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 0.001, "actual={actual}, expected={expected}");
    }

    #[test]
    fn test_replenish_one_second() {
        let available = calculate_replenished_tokens(0.0, 1000, 10.0, 100);
        assert_close(available, 10.0);
    }

    #[test]
    fn test_replenish_partial_second() {
        let available = calculate_replenished_tokens(0.0, 500, 10.0, 100);
        assert_close(available, 5.0);
    }

    #[test]
    fn test_replenish_caps_at_capacity() {
        let available = calculate_replenished_tokens(90.0, 1000, 100.0, 100);
        assert_close(available, 100.0);
    }

    #[test]
    fn test_replenish_no_time_elapsed() {
        let available = calculate_replenished_tokens(50.0, 0, 10.0, 100);
        assert_close(available, 50.0);
    }

    #[test]
    fn test_replenish_zero_rate() {
        let available = calculate_replenished_tokens(50.0, 1000, 0.0, 100);
        assert_close(available, 50.0);
    }

    #[test]
    fn test_replenish_large_elapsed_time() {
        let available = calculate_replenished_tokens(0.0, u64::MAX / 2, 1000.0, 100);
        assert_close(available, 100.0);
    }

    #[test]
    fn test_replenish_fractional_tokens() {
        let available = calculate_replenished_tokens(5.5, 100, 10.0, 100);
        assert_close(available, 6.5);
    }

    #[test]
    fn test_monotonic_timestamp_clamps_backward_time() {
        assert_eq!(monotonic_timestamp_ms(2000, 1500), 2000);
        assert_eq!(monotonic_timestamp_ms(2000, 2500), 2500);
    }

    #[test]
    fn test_advance_token_bucket_ignores_backward_time() {
        let config = TokenBucketConfig::new(60, 2);
        let state = TokenBucketState {
            tokens: 0.0,
            last_update_ms: 1000,
        };

        let advanced = advance_token_bucket(state, config, 500);
        assert_close(advanced.tokens, 0.0);
        assert_eq!(advanced.last_update_ms, 1000);
    }

    #[test]
    fn test_consume_sufficient() {
        let result = can_consume_token(5.0);
        match result {
            TokenConsumptionResult::Consumed { remaining } => assert_close(remaining, 4.0),
            _ => panic!("Expected Consumed"),
        }
    }

    #[test]
    fn test_consume_exact() {
        let result = can_consume_token(1.0);
        match result {
            TokenConsumptionResult::Consumed { remaining } => assert_close(remaining, 0.0),
            _ => panic!("Expected Consumed"),
        }
    }

    #[test]
    fn test_consume_insufficient() {
        let result = can_consume_token(0.5);
        match result {
            TokenConsumptionResult::Denied { available } => assert_close(available, 0.5),
            _ => panic!("Expected Denied"),
        }
    }

    #[test]
    fn test_consume_zero() {
        let result = can_consume_token(0.0);
        assert!(!result.is_consumed());
    }

    #[test]
    fn test_consume_negative() {
        let result = can_consume_token(-1.0);
        assert!(!result.is_consumed());
    }

    #[test]
    fn test_is_consumed_helper() {
        assert!(can_consume_token(5.0).is_consumed());
        assert!(!can_consume_token(0.5).is_consumed());
    }

    #[test]
    fn test_try_consume_token_at_updates_state() {
        let config = TokenBucketConfig::new(60, 2);
        let state = TokenBucketState {
            tokens: 0.0,
            last_update_ms: 1000,
        };

        let transition = try_consume_token_at(state, config, 2000);
        assert!(transition.result.is_consumed());
        assert_close(transition.next_state.tokens, 0.0);
        assert_eq!(transition.next_state.last_update_ms, 2000);
    }

    #[test]
    fn test_evaluate_gossip_admission_preserves_global_budget_on_per_peer_denial() {
        let global_config = TokenBucketConfig::new(0, 2);
        let peer_config = TokenBucketConfig::new(0, 1);
        let global_state = TokenBucketState::full(global_config, 0);
        let peer_state = PeerRateState {
            bucket: TokenBucketState {
                tokens: 0.0,
                last_update_ms: 0,
            },
            last_access_ms: 0,
        };

        let transition = evaluate_gossip_admission(global_state, global_config, Some(peer_state), peer_config, 0);

        assert_eq!(transition.outcome, Err(TwoTierRateLimitReason::PerPeer));
        assert_close(transition.next_global.tokens, 2.0);
        assert_eq!(transition.next_peer.expect("peer state").last_access_ms, 0);
    }

    #[test]
    fn test_evaluate_gossip_admission_consumes_both_budgets_on_success() {
        let global_config = TokenBucketConfig::new(0, 2);
        let peer_config = TokenBucketConfig::new(0, 2);
        let global_state = TokenBucketState::full(global_config, 0);
        let peer_state = PeerRateState::new(TokenBucketState::full(peer_config, 0), 0);

        let transition = evaluate_gossip_admission(global_state, global_config, Some(peer_state), peer_config, 0);

        assert_eq!(transition.outcome, Ok(()));
        assert_close(transition.next_global.tokens, 1.0);
        assert_close(transition.next_peer.expect("peer state").bucket.tokens, 1.0);
    }

    #[test]
    fn test_evaluate_gossip_admission_global_denial_keeps_peer_state_unmodified() {
        let global_config = TokenBucketConfig::new(0, 0);
        let peer_config = TokenBucketConfig::new(0, 2);
        let global_state = TokenBucketState::full(global_config, 0);
        let peer_state = PeerRateState::new(TokenBucketState::full(peer_config, 0), 100);

        let transition = evaluate_gossip_admission(global_state, global_config, Some(peer_state), peer_config, 200);

        assert_eq!(transition.outcome, Err(TwoTierRateLimitReason::Global));
        assert!(transition.next_peer.is_none());
        assert_eq!(transition.next_global.last_update_ms, 200);
    }

    #[test]
    fn test_update_last_access_ms_is_monotonic() {
        assert_eq!(update_last_access_ms(1000, 900), 1000);
        assert_eq!(update_last_access_ms(1000, 1100), 1100);
    }

    #[test]
    fn test_evict_at_capacity() {
        assert!(should_evict_oldest(100, 100));
    }

    #[test]
    fn test_evict_over_capacity() {
        assert!(should_evict_oldest(101, 100));
    }

    #[test]
    fn test_no_evict_under_capacity() {
        assert!(!should_evict_oldest(99, 100));
    }

    #[test]
    fn test_no_evict_empty() {
        assert!(!should_evict_oldest(0, 100));
    }

    #[test]
    fn test_evict_zero_capacity() {
        assert!(should_evict_oldest(0, 0));
        assert!(should_evict_oldest(1, 0));
    }

    #[test]
    fn test_evict_max_values() {
        assert!(should_evict_oldest(u32::MAX, u32::MAX));
        assert!(!should_evict_oldest(u32::MAX - 1, u32::MAX));
    }

    impl PeerRateState {
        fn new(bucket: TokenBucketState, last_access_ms: u64) -> Self {
            Self { bucket, last_access_ms }
        }
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_replenish_never_exceeds_capacity() {
        check!().with_type::<(u64, u64)>().for_each(|(elapsed_ms, capacity)| {
            if *capacity > 0 {
                let available = calculate_replenished_tokens(0.0, *elapsed_ms, 1000.0, *capacity);
                assert!(available <= *capacity as f64, "Replenished tokens must not exceed capacity");
            }
        });
    }

    #[test]
    fn prop_replenish_monotonic_with_time() {
        check!().with_type::<(u64, u64)>().for_each(|(elapsed1, delta)| {
            let elapsed2 = elapsed1.saturating_add(*delta);
            let available1 = calculate_replenished_tokens(0.0, *elapsed1, 10.0, 100);
            let available2 = calculate_replenished_tokens(0.0, elapsed2, 10.0, 100);
            assert!(available2 >= available1, "More time should mean more or equal tokens");
        });
    }

    #[test]
    fn prop_consume_consistency() {
        check!().with_type::<u32>().for_each(|available_int| {
            let available = *available_int as f64 / 100.0;
            let result = can_consume_token(available);
            if available >= 1.0 {
                assert!(result.is_consumed());
            } else {
                assert!(!result.is_consumed());
            }
        });
    }

    #[test]
    fn prop_backward_time_never_increases_tokens() {
        check!().with_type::<(u32, u64, u64)>().for_each(|(burst, now_ms, delta_ms)| {
            let burst = (*burst).max(1);
            let config = TokenBucketConfig::new(60, burst);
            let state = TokenBucketState {
                tokens: 0.0,
                last_update_ms: now_ms.saturating_add(*delta_ms),
            };
            let advanced = advance_token_bucket(state, config, *now_ms);
            assert!(advanced.tokens <= state.tokens + 0.001);
            assert!(advanced.last_update_ms >= state.last_update_ms);
        });
    }

    #[test]
    fn prop_evict_decision_consistent() {
        check!().with_type::<(u32, u32)>().for_each(|(current, max)| {
            let should = should_evict_oldest(*current, *max);
            if *current >= *max {
                assert!(should, "Should evict when at or over capacity");
            } else {
                assert!(!should, "Should not evict when under capacity");
            }
        });
    }
}
