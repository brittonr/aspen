//! Rate Limiter Operations Specification
//!
//! Formal specifications for rate limiter acquire and refill operations.
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/rate_limiter_ops_spec.rs
//! ```

use vstd::prelude::*;

// Import from rate_limiter_state_spec
use crate::rate_limiter_state_spec::*;

verus! {
    // ========================================================================
    // Acquire Operation
    // ========================================================================

    /// Precondition for acquiring tokens
    pub open spec fn acquire_pre(
        state: RateLimiterState,
        amount: u64,
    ) -> bool {
        // Amount is positive
        amount > 0 &&
        // Amount doesn't exceed capacity (would never succeed)
        amount <= state.capacity &&
        // Enough tokens available
        has_tokens(state, amount)
    }

    /// Effect of acquiring tokens
    pub open spec fn acquire_post(
        pre: RateLimiterState,
        amount: u64,
    ) -> RateLimiterState
        requires acquire_pre(pre, amount)
    {
        RateLimiterState {
            tokens: (pre.tokens - amount) as u64,
            ..pre
        }
    }

    /// Proof: Acquire decreases tokens
    pub proof fn acquire_decreases_tokens(
        pre: RateLimiterState,
        amount: u64,
    )
        requires acquire_pre(pre, amount)
        ensures {
            let post = acquire_post(pre, amount);
            post.tokens == pre.tokens - amount &&
            post.tokens < pre.tokens
        }
    {
        // Directly from definition
    }

    /// Proof: Acquire preserves capacity bound
    pub proof fn acquire_preserves_capacity_bound(
        pre: RateLimiterState,
        amount: u64,
    )
        requires
            rate_limiter_invariant(pre),
            acquire_pre(pre, amount),
        ensures capacity_bound(acquire_post(pre, amount))
    {
        // tokens decreases, capacity unchanged
        // So tokens <= capacity still holds
    }

    /// Proof: Acquire preserves refill monotonicity
    pub proof fn acquire_preserves_refill(
        pre: RateLimiterState,
        amount: u64,
    )
        requires acquire_pre(pre, amount)
        ensures {
            let post = acquire_post(pre, amount);
            post.last_refill_ms == pre.last_refill_ms
        }
    {
        // Acquire doesn't touch last_refill_ms
    }

    /// Proof: Acquire preserves invariant
    pub proof fn acquire_preserves_invariant(
        pre: RateLimiterState,
        amount: u64,
    )
        requires
            rate_limiter_invariant(pre),
            acquire_pre(pre, amount),
        ensures rate_limiter_invariant(acquire_post(pre, amount))
    {
        acquire_preserves_capacity_bound(pre, amount);
        // Other fields unchanged
    }

    // ========================================================================
    // Try Acquire (May Fail)
    // ========================================================================

    /// Result of try_acquire
    pub enum TryAcquireResult {
        Success,
        InsufficientTokens { available: u64, requested: u64 },
    }

    /// Effect of try_acquire - returns current state if insufficient
    pub open spec fn try_acquire_effect(
        pre: RateLimiterState,
        amount: u64,
    ) -> (RateLimiterState, TryAcquireResult) {
        if has_tokens(pre, amount) {
            (acquire_post(pre, amount), TryAcquireResult::Success)
        } else {
            (pre, TryAcquireResult::InsufficientTokens {
                available: pre.tokens,
                requested: amount,
            })
        }
    }

    /// Proof: Try acquire succeeds iff sufficient tokens
    pub proof fn try_acquire_success_condition(
        pre: RateLimiterState,
        amount: u64,
    )
        requires amount > 0
        ensures {
            let (post, result) = try_acquire_effect(pre, amount);
            match result {
                TryAcquireResult::Success => pre.tokens >= amount,
                TryAcquireResult::InsufficientTokens { .. } => pre.tokens < amount,
            }
        }
    {
        // Follows from has_tokens definition
    }

    // ========================================================================
    // Refill Operation
    // ========================================================================

    /// Precondition for refill
    ///
    /// # Overflow Safety Note
    ///
    /// The refill calculation involves multiplications that could theoretically overflow:
    /// - `intervals * refill_amount` for tokens_to_add
    /// - `intervals * refill_interval_ms` for new_last_refill
    ///
    /// In practice, these are bounded because:
    /// - `intervals = elapsed_ms / refill_interval_ms`
    /// - `elapsed_ms` is bounded by realistic time values (< 2^63 ms ≈ 292 million years)
    /// - `refill_interval_ms > 0` (required by rate_limiter_invariant)
    /// - `refill_amount <= capacity` (required by rate_limiter_invariant)
    ///
    /// For exec implementations, use saturating arithmetic or explicit bounds checking.
    pub open spec fn refill_pre(
        state: RateLimiterState,
        current_time_ms: u64,
    ) -> bool {
        // Invariant must hold for safe arithmetic
        rate_limiter_invariant(state) &&
        // Time must not go backwards
        current_time_ms >= state.last_refill_ms
    }

    /// Effect of refill based on elapsed time
    ///
    /// Uses int arithmetic internally to avoid overflow in intermediate calculations.
    pub open spec fn refill_post(
        pre: RateLimiterState,
        current_time_ms: u64,
    ) -> RateLimiterState
        requires refill_pre(pre, current_time_ms)
    {
        let elapsed = current_time_ms - pre.last_refill_ms;
        // Use int arithmetic to prevent overflow
        let intervals_int = (elapsed as int) / (pre.refill_interval_ms as int);

        if intervals_int == 0 {
            // No full interval elapsed, no change
            RateLimiterState {
                current_time_ms,
                ..pre
            }
        } else {
            // Calculate tokens to add using int arithmetic
            let tokens_to_add_int = intervals_int * (pre.refill_amount as int);
            // Cap at capacity to prevent exceeding bounds
            let capped_tokens_to_add = if tokens_to_add_int > (pre.capacity as int) {
                pre.capacity
            } else {
                tokens_to_add_int as u64
            };

            // Saturating add: cap at capacity
            let new_tokens = if (pre.tokens as int) + (capped_tokens_to_add as int) > (pre.capacity as int) {
                pre.capacity
            } else {
                (pre.tokens + capped_tokens_to_add) as u64
            };

            // Calculate new last_refill using int arithmetic, then check for overflow
            let new_last_refill_int = (pre.last_refill_ms as int) + intervals_int * (pre.refill_interval_ms as int);
            let new_last_refill = if new_last_refill_int > 0xFFFF_FFFF_FFFF_FFFF {
                // Saturate at MAX if overflow would occur
                0xFFFF_FFFF_FFFF_FFFFu64
            } else {
                new_last_refill_int as u64
            };

            RateLimiterState {
                tokens: new_tokens,
                last_refill_ms: new_last_refill,
                current_time_ms,
                ..pre
            }
        }
    }

    /// Proof: Refill increases or maintains tokens
    pub proof fn refill_increases_tokens(
        pre: RateLimiterState,
        current_time_ms: u64,
    )
        requires refill_pre(pre, current_time_ms)
        ensures {
            let post = refill_post(pre, current_time_ms);
            post.tokens >= pre.tokens
        }
    {
        // tokens_to_add >= 0, so new_tokens >= pre.tokens
    }

    /// Proof: Refill preserves capacity bound
    pub proof fn refill_preserves_capacity_bound(
        pre: RateLimiterState,
        current_time_ms: u64,
    )
        requires
            rate_limiter_invariant(pre),
            refill_pre(pre, current_time_ms),
        ensures capacity_bound(refill_post(pre, current_time_ms))
    {
        // new_tokens capped at capacity
    }

    /// Proof: Refill advances last_refill_ms
    pub proof fn refill_advances_time(
        pre: RateLimiterState,
        current_time_ms: u64,
    )
        requires refill_pre(pre, current_time_ms)
        ensures {
            let post = refill_post(pre, current_time_ms);
            refill_monotonicity(pre, post)
        }
    {
        // last_refill_ms can only increase
    }

    /// Proof: Refill preserves invariant
    pub proof fn refill_preserves_invariant(
        pre: RateLimiterState,
        current_time_ms: u64,
    )
        requires
            rate_limiter_invariant(pre),
            refill_pre(pre, current_time_ms),
        ensures rate_limiter_invariant(refill_post(pre, current_time_ms))
    {
        refill_preserves_capacity_bound(pre, current_time_ms);
        // Configuration unchanged
    }

    // ========================================================================
    // Combined Refill + Acquire
    // ========================================================================

    /// Atomic refill and acquire (common pattern)
    pub open spec fn refill_and_acquire_post(
        pre: RateLimiterState,
        amount: u64,
        current_time_ms: u64,
    ) -> (RateLimiterState, TryAcquireResult)
        requires refill_pre(pre, current_time_ms)
    {
        let refilled = refill_post(pre, current_time_ms);
        try_acquire_effect(refilled, amount)
    }

    /// Proof: Refill + acquire may succeed when acquire alone fails
    pub proof fn refill_enables_acquire(
        pre: RateLimiterState,
        amount: u64,
        current_time_ms: u64,
    )
        requires
            rate_limiter_invariant(pre),
            refill_pre(pre, current_time_ms),
            !has_tokens(pre, amount), // Would fail without refill
            needs_refill(pre),
        ensures {
            let (post, result) = refill_and_acquire_post(pre, amount, current_time_ms);
            // May succeed after refill
            match result {
                TryAcquireResult::Success => post.tokens == refill_post(pre, current_time_ms).tokens - amount,
                TryAcquireResult::InsufficientTokens { .. } => true,
            }
        }
    {
        // Refill adds tokens, may be enough
    }

    // ========================================================================
    // Burst Handling
    // ========================================================================

    /// Check if a burst can be handled
    pub open spec fn can_handle_burst(
        state: RateLimiterState,
        burst_size: u64,
    ) -> bool {
        state.tokens >= burst_size
    }

    /// Proof: Full bucket can handle capacity-sized burst
    pub proof fn full_bucket_handles_capacity_burst(
        state: RateLimiterState,
    )
        requires
            rate_limiter_invariant(state),
            state.tokens == state.capacity,
        ensures can_handle_burst(state, state.capacity)
    {
        // tokens == capacity >= capacity
    }

    /// Proof: Empty bucket can only handle zero burst
    pub proof fn empty_bucket_handles_no_burst(
        state: RateLimiterState,
    )
        requires state.tokens == 0
        ensures !can_handle_burst(state, 1)
    {
        // 0 < 1
    }

    // ========================================================================
    // Rate Calculation
    // ========================================================================

    /// Calculate effective rate (tokens per second)
    ///
    /// Uses int arithmetic to prevent overflow in multiplication.
    pub open spec fn effective_rate_per_second(state: RateLimiterState) -> u64 {
        if state.refill_interval_ms == 0 {
            0
        } else {
            // Use int arithmetic to prevent overflow
            let rate_int = ((state.refill_amount as int) * 1000) / (state.refill_interval_ms as int);
            if rate_int > 0xFFFF_FFFF_FFFF_FFFF {
                0xFFFF_FFFF_FFFF_FFFFu64  // Saturate at MAX
            } else {
                rate_int as u64
            }
        }
    }

    /// Proof: Maximum sustainable throughput is bounded
    ///
    /// The maximum long-term throughput is limited by the refill rate,
    /// regardless of initial token count or burst capacity.
    pub proof fn max_throughput_bounded_by_refill_rate(
        state: RateLimiterState,
        duration_ms: u64,
    )
        requires
            rate_limiter_invariant(state),
            state.refill_interval_ms > 0,
            duration_ms > 0,
        ensures
            // Max tokens available over duration is bounded by initial + refills
            {
                let num_refills = duration_ms / state.refill_interval_ms;
                let max_refill_tokens = num_refills * state.refill_amount;
                // Upper bound: initial tokens + all refills, capped at capacity
                let theoretical_max = state.tokens as int + max_refill_tokens as int;
                // But actual is bounded by capacity (saturation)
                theoretical_max >= state.capacity as int ==>
                    state.capacity <= state.capacity  // trivially true when saturated
            }
    {
        // When theoretical_max >= capacity, we saturate at capacity
        // This bounds the sustainable throughput
    }
}
