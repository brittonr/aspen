//! Priority and Retry Policy Verification Specifications
//!
//! Formal specifications for job priority ordering and retry policy bounds.
//!
//! # Key Invariants
//!
//! 1. **PRIO-1: Total Order**: Critical > High > Normal > Low
//! 2. **PRIO-2: Queue Names**: Unique names per priority
//! 3. **RETRY-1: max_attempts >= 1**: At least initial attempt
//! 4. **RETRY-2: Exponential Bounds**: Bounded delay growth
//! 5. **RETRY-3: Custom Consistency**: Enough delays for attempts
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-jobs/verus/priority_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Priority Model
    // ========================================================================

    /// Priority levels (numeric for ordering)
    pub enum PrioritySpec {
        Low,       // 0
        Normal,    // 1
        High,      // 2
        Critical,  // 3
    }

    /// Priority to numeric value
    pub open spec fn priority_value(p: PrioritySpec) -> u64 {
        match p {
            PrioritySpec::Low => 0,
            PrioritySpec::Normal => 1,
            PrioritySpec::High => 2,
            PrioritySpec::Critical => 3,
        }
    }

    // ========================================================================
    // Invariant PRIO-1: Total Order
    // ========================================================================

    /// PRIO-1: Priorities form a total order
    pub open spec fn priority_less_than(a: PrioritySpec, b: PrioritySpec) -> bool {
        priority_value(a) < priority_value(b)
    }

    /// Critical is highest
    pub open spec fn critical_is_highest(p: PrioritySpec) -> bool {
        matches!(p, PrioritySpec::Critical) ||
        priority_less_than(p, PrioritySpec::Critical)
    }

    /// Low is lowest
    pub open spec fn low_is_lowest(p: PrioritySpec) -> bool {
        matches!(p, PrioritySpec::Low) ||
        priority_less_than(PrioritySpec::Low, p)
    }

    /// all_ordered() returns correct order
    pub open spec fn all_ordered_correct(
        first: PrioritySpec,
        second: PrioritySpec,
        third: PrioritySpec,
        fourth: PrioritySpec,
    ) -> bool {
        matches!(first, PrioritySpec::Critical) &&
        matches!(second, PrioritySpec::High) &&
        matches!(third, PrioritySpec::Normal) &&
        matches!(fourth, PrioritySpec::Low)
    }

    /// Proof: Order is correct
    pub proof fn order_is_total()
        ensures
            priority_less_than(PrioritySpec::Low, PrioritySpec::Normal),
            priority_less_than(PrioritySpec::Normal, PrioritySpec::High),
            priority_less_than(PrioritySpec::High, PrioritySpec::Critical),
    {
        // By definition of priority_value
    }

    /// Transitivity of priority ordering
    pub proof fn priority_transitivity(a: PrioritySpec, b: PrioritySpec, c: PrioritySpec)
        requires
            priority_less_than(a, b),
            priority_less_than(b, c),
        ensures priority_less_than(a, c)
    {
        // Follows from numeric comparison transitivity
    }

    // ========================================================================
    // Invariant PRIO-2: Queue Names
    // ========================================================================

    /// Queue name for priority
    pub enum QueueNameSpec {
        Low,
        Normal,
        High,
        Critical,
    }

    /// PRIO-2: Each priority maps to unique queue name
    pub open spec fn queue_name_for_priority(p: PrioritySpec) -> QueueNameSpec {
        match p {
            PrioritySpec::Low => QueueNameSpec::Low,
            PrioritySpec::Normal => QueueNameSpec::Normal,
            PrioritySpec::High => QueueNameSpec::High,
            PrioritySpec::Critical => QueueNameSpec::Critical,
        }
    }

    /// Queue names are unique
    pub open spec fn queue_names_unique(p1: PrioritySpec, p2: PrioritySpec) -> bool {
        // Different priorities map to different queues
        (priority_value(p1) != priority_value(p2)) ==>
        !matches!((queue_name_for_priority(p1), queue_name_for_priority(p2)),
                  (QueueNameSpec::Low, QueueNameSpec::Low) |
                  (QueueNameSpec::Normal, QueueNameSpec::Normal) |
                  (QueueNameSpec::High, QueueNameSpec::High) |
                  (QueueNameSpec::Critical, QueueNameSpec::Critical))
    }

    // ========================================================================
    // Retry Policy Model
    // ========================================================================

    /// Retry policy specification
    pub enum RetryPolicySpec {
        /// No retries
        None,
        /// Fixed delay
        Fixed { max_attempts: u64, delay_ms: u64 },
        /// Exponential backoff
        Exponential {
            max_attempts: u64,
            initial_delay_ms: u64,
            multiplier: u64, // scaled by 10 (20 = 2.0x)
            max_delay_ms: Option<u64>,
        },
        /// Custom delays
        Custom {
            delays_count: u64,
            max_attempts: Option<u64>,
        },
    }

    // ========================================================================
    // Invariant RETRY-1: max_attempts >= 1
    // ========================================================================

    /// RETRY-1: max_attempts is at least 1
    pub open spec fn max_attempts_at_least_one(policy: RetryPolicySpec) -> bool {
        match policy {
            RetryPolicySpec::None => true,  // 1 initial attempt
            RetryPolicySpec::Fixed { max_attempts, .. } => max_attempts >= 1,
            RetryPolicySpec::Exponential { max_attempts, .. } => max_attempts >= 1,
            RetryPolicySpec::Custom { delays_count, max_attempts } => {
                match max_attempts {
                    Some(n) => n >= 1,
                    None => delays_count >= 1,
                }
            }
        }
    }

    /// Get max attempts for policy
    pub open spec fn get_max_attempts(policy: RetryPolicySpec) -> u64 {
        match policy {
            RetryPolicySpec::None => 1,
            RetryPolicySpec::Fixed { max_attempts, .. } => max_attempts,
            RetryPolicySpec::Exponential { max_attempts, .. } => max_attempts,
            RetryPolicySpec::Custom { delays_count, max_attempts } => {
                match max_attempts {
                    Some(n) => n,
                    None => delays_count,
                }
            }
        }
    }

    /// Proof: All policies have at least 1 attempt
    pub proof fn all_policies_have_initial_attempt(policy: RetryPolicySpec)
        requires max_attempts_at_least_one(policy)
        ensures get_max_attempts(policy) >= 1
    {
        // By definition
    }

    // ========================================================================
    // Invariant RETRY-2: Exponential Bounds
    // ========================================================================

    /// Calculate exponential delay (with saturation)
    pub open spec fn exponential_delay_ms(
        initial_delay_ms: u64,
        multiplier_scaled: u64,  // multiplier * 10
        attempt: u64,
        max_delay_ms: Option<u64>,
    ) -> u64 {
        // delay = initial * (multiplier^attempt)
        // We cap at u64::MAX / 10 to prevent overflow
        let base_delay = initial_delay_ms;

        // Simple approximation: multiply by (multiplier/10) for each attempt
        // In practice, this would be computed iteratively
        let raw_delay = if attempt < 20 {
            // Safe range
            base_delay  // Simplified
        } else {
            u64::MAX / 2  // Very large
        };

        match max_delay_ms {
            Some(max) => if raw_delay > max { max } else { raw_delay },
            None => raw_delay,
        }
    }

    /// RETRY-2: Exponential delay is bounded by max_delay
    pub open spec fn exponential_bounded(
        initial_delay_ms: u64,
        multiplier_scaled: u64,
        attempt: u64,
        max_delay_ms: u64,
    ) -> bool {
        exponential_delay_ms(initial_delay_ms, multiplier_scaled, attempt, Some(max_delay_ms))
            <= max_delay_ms
    }

    /// Proof: Exponential delay never exceeds max
    pub proof fn exponential_respects_max(
        initial_delay_ms: u64,
        multiplier_scaled: u64,
        attempt: u64,
        max_delay_ms: u64,
    )
        ensures exponential_bounded(initial_delay_ms, multiplier_scaled, attempt, max_delay_ms)
    {
        // By definition of exponential_delay_ms with Some(max)
    }

    // ========================================================================
    // Invariant RETRY-3: Custom Consistency
    // ========================================================================

    /// RETRY-3: Custom policy has enough delays
    pub open spec fn custom_delays_sufficient(
        delays_count: u64,
        max_attempts: u64,
    ) -> bool {
        delays_count >= max_attempts
    }

    /// Custom policy construction ensures consistency
    pub open spec fn custom_construction_consistent(
        delays_count: u64,
        explicit_max: Option<u64>,
    ) -> bool {
        match explicit_max {
            Some(max) => delays_count >= max,
            None => true,  // max = delays_count, always sufficient
        }
    }

    // ========================================================================
    // Combined Policy Invariant
    // ========================================================================

    /// Complete invariant for retry policy
    pub open spec fn retry_policy_invariant(policy: RetryPolicySpec) -> bool {
        // At least 1 attempt
        max_attempts_at_least_one(policy) &&
        // Type-specific constraints
        match policy {
            RetryPolicySpec::None => true,
            RetryPolicySpec::Fixed { delay_ms, .. } => delay_ms > 0,
            RetryPolicySpec::Exponential { initial_delay_ms, multiplier, .. } => {
                initial_delay_ms > 0 && multiplier >= 10  // multiplier >= 1.0
            }
            RetryPolicySpec::Custom { delays_count, max_attempts } => {
                custom_construction_consistent(delays_count, max_attempts)
            }
        }
    }

    // ========================================================================
    // Default Policy
    // ========================================================================

    /// Default policy is exponential with 3 retries
    pub open spec fn default_policy() -> RetryPolicySpec {
        RetryPolicySpec::Exponential {
            max_attempts: 3,
            initial_delay_ms: 1000,
            multiplier: 20,  // 2.0x
            max_delay_ms: Some(300_000),  // 5 minutes
        }
    }

    /// Proof: Default policy satisfies invariant
    pub proof fn default_policy_valid()
        ensures retry_policy_invariant(default_policy())
    {
        // max_attempts = 3 >= 1
        // initial_delay_ms = 1000 > 0
        // multiplier = 20 >= 10
    }

    // ========================================================================
    // Schedule Specifications (Basic)
    // ========================================================================

    /// Schedule type enumeration
    pub enum ScheduleSpec {
        Once,
        Recurring,
        Interval { every_ms: u64 },
        RateLimit { max_per_hour: u64 },
        BusinessHours,
        Exponential { base_delay_ms: u64, max_delay_ms: u64 },
    }

    /// Schedule produces valid next time
    pub open spec fn schedule_next_valid(
        schedule: ScheduleSpec,
        current_time_ms: u64,
        next_time_ms: u64,
    ) -> bool {
        // Next time is in the future or equal to current
        next_time_ms >= current_time_ms
    }

    /// Rate limit respects capacity
    pub open spec fn rate_limit_respects_capacity(
        max_per_hour: u64,
        current_hour_count: u64,
        can_execute: bool,
    ) -> bool {
        can_execute ==> current_hour_count < max_per_hour
    }
}
