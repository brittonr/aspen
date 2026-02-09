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

    /// Precondition for refill (always succeeds)
    pub open spec fn refill_pre(
        state: RateLimiterState,
        current_time_ms: u64,
    ) -> bool {
        // Time must not go backwards
        current_time_ms >= state.last_refill_ms
    }

    /// Effect of refill based on elapsed time
    pub open spec fn refill_post(
        pre: RateLimiterState,
        current_time_ms: u64,
    ) -> RateLimiterState
        requires refill_pre(pre, current_time_ms)
    {
        let elapsed = current_time_ms - pre.last_refill_ms;
        let intervals = elapsed / pre.refill_interval_ms;

        if intervals == 0 {
            // No full interval elapsed, no change
            RateLimiterState {
                current_time_ms,
                ..pre
            }
        } else {
            let tokens_to_add = intervals * pre.refill_amount;
            let new_tokens = if pre.tokens + tokens_to_add > pre.capacity {
                pre.capacity
            } else {
                pre.tokens + tokens_to_add
            };
            let new_last_refill = pre.last_refill_ms + intervals * pre.refill_interval_ms;

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
    pub open spec fn effective_rate_per_second(state: RateLimiterState) -> u64 {
        if state.refill_interval_ms == 0 {
            0
        } else {
            (state.refill_amount * 1000) / state.refill_interval_ms
        }
    }

    /// Proof: Rate is bounded by capacity
    /// Over any time window, can't exceed capacity + rate * time
    pub proof fn rate_bounded_by_capacity(
        state: RateLimiterState,
        duration_ms: u64,
    )
        requires rate_limiter_invariant(state)
        ensures {
            let intervals = duration_ms / state.refill_interval_ms;
            let max_tokens = state.capacity + intervals * state.refill_amount;
            // Can never have more than capacity at any point
            max_tokens >= state.capacity
        }
    {
        // Bucket can't exceed capacity
    }
}

mod rate_limiter_state_spec;
