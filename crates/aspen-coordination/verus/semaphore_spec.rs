//! Semaphore State Machine Model and Operations
//!
//! Abstract state model for formal verification of distributed semaphore operations.
//!
//! # State Model
//!
//! The `SemaphoreState` captures:
//! - Maximum capacity (total permits)
//! - Current holders with their permit counts
//!
//! # Key Invariants
//!
//! 1. **Capacity Bound**: Total permits held <= capacity
//! 2. **Holder Limit**: Number of holders <= max_holders
//! 3. **Available Correctness**: available = capacity - used
//! 4. **Non-negative Permits**: Each holder has permits > 0
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/semaphore_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Semaphore holder entry
    pub struct SemaphoreHolderSpec {
        /// Unique holder identifier
        pub holder_id: Seq<u8>,
        /// Number of permits held
        pub permits: u32,
        /// Deadline for automatic release
        pub deadline_ms: u64,
    }

    /// Complete semaphore state
    pub struct SemaphoreStateSpec {
        /// Maximum permits (capacity)
        pub capacity: u32,
        /// Number of permits currently used (sum of all holders)
        pub used_permits: u32,
        /// Number of current holders
        pub holder_count: u32,
        /// Maximum allowed holders
        pub max_holders: u32,
    }

    // ========================================================================
    // Core Predicates
    // ========================================================================

    /// Calculate available permits
    pub open spec fn available_permits(state: SemaphoreStateSpec) -> u32 {
        if state.used_permits > state.capacity {
            0
        } else {
            (state.capacity - state.used_permits) as u32
        }
    }

    /// Check if permits can be acquired
    pub open spec fn can_acquire(state: SemaphoreStateSpec, permits: u32) -> bool {
        available_permits(state) >= permits &&
        state.holder_count < state.max_holders
    }

    // ========================================================================
    // Invariant 1: Capacity Bound (SEM-1)
    // ========================================================================

    /// SEM-1: Total permits held <= capacity
    pub open spec fn capacity_bound(state: SemaphoreStateSpec) -> bool {
        state.used_permits <= state.capacity
    }

    // ========================================================================
    // Invariant 2: Holder Limit (SEM-2)
    // ========================================================================

    /// SEM-2: Number of holders <= max_holders
    pub open spec fn holder_limit(state: SemaphoreStateSpec) -> bool {
        state.holder_count <= state.max_holders
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined invariant for semaphore state
    pub open spec fn semaphore_invariant(state: SemaphoreStateSpec) -> bool {
        capacity_bound(state) &&
        holder_limit(state)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial semaphore state (empty)
    pub open spec fn initial_semaphore_state(capacity: u32, max_holders: u32) -> SemaphoreStateSpec {
        SemaphoreStateSpec {
            capacity,
            used_permits: 0,
            holder_count: 0,
            max_holders,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant(capacity: u32, max_holders: u32)
        ensures semaphore_invariant(initial_semaphore_state(capacity, max_holders))
    {
        let state = initial_semaphore_state(capacity, max_holders);
        // used_permits = 0 <= capacity (capacity_bound)
        assert(state.used_permits == 0);
        assert(state.used_permits <= state.capacity);
        // holder_count = 0 <= max_holders (holder_limit)
        assert(state.holder_count == 0);
        assert(state.holder_count <= state.max_holders);
    }

    // ========================================================================
    // Acquire Operation
    // ========================================================================

    /// Precondition for acquiring permits
    ///
    /// Validates:
    /// - Permits available and holder capacity
    /// - Overflow protection for arithmetic operations
    pub open spec fn acquire_pre(state: SemaphoreStateSpec, permits: u32) -> bool {
        can_acquire(state, permits) &&
        permits > 0 &&
        // Overflow protection for used_permits + permits
        // Use consistent constant: u32::MAX - permits to ensure used_permits + permits <= u32::MAX
        state.used_permits <= 0xFFFF_FFFFu32 - permits &&
        // Overflow protection for holder_count + 1
        state.holder_count < 0xFFFF_FFFFu32
    }

    /// Result of acquiring permits
    pub open spec fn acquire_post(pre: SemaphoreStateSpec, permits: u32) -> SemaphoreStateSpec
        requires acquire_pre(pre, permits)
    {
        SemaphoreStateSpec {
            capacity: pre.capacity,
            used_permits: (pre.used_permits + permits) as u32,
            holder_count: (pre.holder_count + 1) as u32,
            max_holders: pre.max_holders,
        }
    }

    /// Proof: Acquire preserves capacity bound
    pub proof fn acquire_preserves_capacity_bound(
        pre: SemaphoreStateSpec,
        permits: u32,
    )
        requires
            acquire_pre(pre, permits),
            capacity_bound(pre),
        ensures
            capacity_bound(acquire_post(pre, permits))
    {
        // available_permits(pre) >= permits
        // available = capacity - used
        // So: used + permits <= capacity
    }

    /// Proof: Acquire preserves holder limit
    pub proof fn acquire_preserves_holder_limit(
        pre: SemaphoreStateSpec,
        permits: u32,
    )
        requires
            acquire_pre(pre, permits),
            holder_limit(pre),
        ensures
            holder_limit(acquire_post(pre, permits))
    {
        // holder_count < max_holders (from acquire_pre)
        // post.holder_count = pre.holder_count + 1 <= max_holders
    }

    /// Proof: Acquire preserves invariant
    pub proof fn acquire_preserves_invariant(
        pre: SemaphoreStateSpec,
        permits: u32,
    )
        requires
            acquire_pre(pre, permits),
            semaphore_invariant(pre),
        ensures
            semaphore_invariant(acquire_post(pre, permits))
    {
        acquire_preserves_capacity_bound(pre, permits);
        acquire_preserves_holder_limit(pre, permits);
    }

    /// Proof: Acquire decreases available permits
    pub proof fn acquire_decreases_available(
        pre: SemaphoreStateSpec,
        permits: u32,
    )
        requires
            acquire_pre(pre, permits),
            semaphore_invariant(pre),
        ensures
            available_permits(acquire_post(pre, permits)) == available_permits(pre) - permits
    {
        // post.used = pre.used + permits
        // post.available = capacity - post.used = capacity - (used + permits)
        //                = (capacity - used) - permits = pre.available - permits
    }

    // ========================================================================
    // Release Operation
    // ========================================================================

    /// Precondition for releasing permits
    pub open spec fn release_pre(state: SemaphoreStateSpec, permits: u32) -> bool {
        state.used_permits >= permits &&
        state.holder_count > 0
    }

    /// Result of releasing permits (removes holder entirely)
    pub open spec fn release_post(pre: SemaphoreStateSpec, permits: u32) -> SemaphoreStateSpec
        requires release_pre(pre, permits)
    {
        SemaphoreStateSpec {
            capacity: pre.capacity,
            used_permits: (pre.used_permits - permits) as u32,
            holder_count: (pre.holder_count - 1) as u32,
            max_holders: pre.max_holders,
        }
    }

    /// Proof: Release preserves capacity bound
    pub proof fn release_preserves_capacity_bound(
        pre: SemaphoreStateSpec,
        permits: u32,
    )
        requires
            release_pre(pre, permits),
            capacity_bound(pre),
        ensures
            capacity_bound(release_post(pre, permits))
    {
        // post.used = pre.used - permits <= pre.used <= capacity
    }

    /// Proof: Release preserves holder limit
    pub proof fn release_preserves_holder_limit(
        pre: SemaphoreStateSpec,
        permits: u32,
    )
        requires
            release_pre(pre, permits),
            holder_limit(pre),
        ensures
            holder_limit(release_post(pre, permits))
    {
        // post.holder_count = pre.holder_count - 1 < pre.holder_count <= max
    }

    /// Proof: Release preserves invariant
    pub proof fn release_preserves_invariant(
        pre: SemaphoreStateSpec,
        permits: u32,
    )
        requires
            release_pre(pre, permits),
            semaphore_invariant(pre),
        ensures
            semaphore_invariant(release_post(pre, permits))
    {
        release_preserves_capacity_bound(pre, permits);
        release_preserves_holder_limit(pre, permits);
    }

    /// Proof: Release increases available permits
    pub proof fn release_increases_available(
        pre: SemaphoreStateSpec,
        permits: u32,
    )
        requires
            release_pre(pre, permits),
            semaphore_invariant(pre),
        ensures
            available_permits(release_post(pre, permits)) == available_permits(pre) + permits
    {
        // post.used = pre.used - permits
        // post.available = capacity - post.used = capacity - (used - permits)
        //                = (capacity - used) + permits = pre.available + permits
    }

    // ========================================================================
    // Expiration Properties
    // ========================================================================

    /// Expired holders are cleaned up, increasing available permits
    pub open spec fn cleanup_expired_effect(
        pre: SemaphoreStateSpec,
        expired_permits: u32,
        expired_holders: u32,
    ) -> SemaphoreStateSpec
        requires
            pre.used_permits >= expired_permits,
            pre.holder_count >= expired_holders,
    {
        SemaphoreStateSpec {
            capacity: pre.capacity,
            used_permits: (pre.used_permits - expired_permits) as u32,
            holder_count: (pre.holder_count - expired_holders) as u32,
            max_holders: pre.max_holders,
        }
    }

    /// Proof: Cleanup preserves invariant
    pub proof fn cleanup_preserves_invariant(
        pre: SemaphoreStateSpec,
        expired_permits: u32,
        expired_holders: u32,
    )
        requires
            semaphore_invariant(pre),
            pre.used_permits >= expired_permits,
            pre.holder_count >= expired_holders,
        ensures
            semaphore_invariant(cleanup_expired_effect(pre, expired_permits, expired_holders))
    {
        // Removing permits and holders only decreases counts
        // Invariants are upper bounds, so they're preserved
    }
}
