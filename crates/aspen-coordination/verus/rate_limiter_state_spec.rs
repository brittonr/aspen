//! Rate Limiter State Machine Model
//!
//! Abstract state model for formal verification of token bucket rate limiting.
//!
//! # State Model
//!
//! The `RateLimiterState` captures:
//! - Token bucket capacity and current tokens
//! - Refill rate and last refill timestamp
//!
//! # Key Invariants
//!
//! 1. **RATE-1: Capacity Bound**: Tokens never exceed capacity
//! 2. **RATE-2: Token Conservation**: Tokens only change via acquire/refill
//! 3. **RATE-3: Refill Monotonicity**: last_refill_ms only increases
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/rate_limiter_state_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Token bucket configuration
    pub struct RateLimiterConfigSpec {
        /// Maximum tokens in bucket
        pub capacity: u64,
        /// Tokens added per refill interval
        pub refill_amount: u64,
        /// Refill interval in milliseconds
        pub refill_interval_ms: u64,
    }

    /// Complete rate limiter state
    pub struct RateLimiterState {
        /// Limiter identifier
        pub limiter_id: Seq<u8>,
        /// Current number of tokens
        pub tokens: u64,
        /// Maximum capacity
        pub capacity: u64,
        /// Tokens added per refill
        pub refill_amount: u64,
        /// Refill interval in milliseconds
        pub refill_interval_ms: u64,
        /// Last refill timestamp (Unix ms)
        pub last_refill_ms: u64,
        /// Current time (for reasoning)
        pub current_time_ms: u64,
    }

    // ========================================================================
    // Invariant 1: Capacity Bound
    // ========================================================================

    /// RATE-1: Tokens never exceed capacity
    pub open spec fn capacity_bound(state: RateLimiterState) -> bool {
        state.tokens <= state.capacity
    }

    // ========================================================================
    // Invariant 2: Token Conservation
    // ========================================================================

    /// RATE-2: Tokens only change through defined operations
    ///
    /// This is implicit in the operation definitions - tokens can only:
    /// - Decrease via acquire
    /// - Increase via refill (up to capacity)
    pub open spec fn token_conservation_acquire(
        pre: RateLimiterState,
        post: RateLimiterState,
        acquired: u64,
    ) -> bool {
        post.tokens == pre.tokens - acquired
    }

    pub open spec fn token_conservation_refill(
        pre: RateLimiterState,
        post: RateLimiterState,
        added: u64,
    ) -> bool {
        // Saturating add at capacity
        if pre.tokens + added > pre.capacity {
            post.tokens == pre.capacity
        } else {
            post.tokens == pre.tokens + added
        }
    }

    // ========================================================================
    // Invariant 3: Refill Monotonicity
    // ========================================================================

    /// RATE-3: last_refill_ms only increases
    pub open spec fn refill_monotonicity(pre: RateLimiterState, post: RateLimiterState) -> bool {
        post.last_refill_ms >= pre.last_refill_ms
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined rate limiter invariant
    pub open spec fn rate_limiter_invariant(state: RateLimiterState) -> bool {
        // Capacity bound
        capacity_bound(state) &&
        // Configuration is valid
        state.capacity > 0 &&
        state.refill_amount > 0 &&
        state.refill_interval_ms > 0 &&
        // Refill amount doesn't exceed capacity
        state.refill_amount <= state.capacity
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial rate limiter state (full bucket)
    pub open spec fn initial_rate_limiter_state(
        limiter_id: Seq<u8>,
        config: RateLimiterConfigSpec,
        current_time_ms: u64,
    ) -> RateLimiterState {
        RateLimiterState {
            limiter_id,
            tokens: config.capacity, // Start full
            capacity: config.capacity,
            refill_amount: config.refill_amount,
            refill_interval_ms: config.refill_interval_ms,
            last_refill_ms: current_time_ms,
            current_time_ms,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant(
        limiter_id: Seq<u8>,
        config: RateLimiterConfigSpec,
        current_time_ms: u64,
    )
        requires
            config.capacity > 0,
            config.refill_amount > 0,
            config.refill_amount <= config.capacity,
            config.refill_interval_ms > 0,
        ensures rate_limiter_invariant(initial_rate_limiter_state(limiter_id, config, current_time_ms))
    {
        // tokens = capacity, so capacity_bound holds
    }

    // ========================================================================
    // Helper Predicates
    // ========================================================================

    /// Check if tokens are available
    pub open spec fn has_tokens(state: RateLimiterState, amount: u64) -> bool {
        state.tokens >= amount
    }

    /// Calculate tokens to add based on elapsed time
    pub open spec fn calculate_refill(state: RateLimiterState) -> u64 {
        let elapsed = if state.current_time_ms > state.last_refill_ms {
            state.current_time_ms - state.last_refill_ms
        } else {
            0
        };

        let intervals = elapsed / state.refill_interval_ms;
        let tokens_to_add = intervals * state.refill_amount;

        // Cap at capacity
        if state.tokens + tokens_to_add > state.capacity {
            state.capacity - state.tokens
        } else {
            tokens_to_add
        }
    }

    /// Check if refill is needed
    pub open spec fn needs_refill(state: RateLimiterState) -> bool {
        state.current_time_ms >= state.last_refill_ms + state.refill_interval_ms
    }
}
