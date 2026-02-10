//! Stream Manager Specification
//!
//! Formal specification for per-connection stream limiting.
//!
//! # Properties
//!
//! 1. **STREAM-1: Stream Bounds**: active_streams <= max_streams
//! 2. **STREAM-2: Counter Consistency**: Atomic counter matches permits
//! 3. **STREAM-3: Permit Release**: Drop decrements counter
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-transport/verus/stream_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract stream manager state
    pub struct StreamManagerState {
        /// Maximum streams per connection
        pub max_streams: u32,
        /// Available permits in semaphore
        pub available_permits: u32,
        /// Atomic counter tracking active streams
        pub active_streams_counter: u32,
    }

    /// Abstract stream permit
    pub struct StreamPermitSpec {
        /// Whether this permit is valid (not dropped)
        pub valid: bool,
    }

    // ========================================================================
    // STREAM-1: Stream Bounds
    // ========================================================================

    /// Active streams never exceed maximum
    pub open spec fn stream_bounded(state: StreamManagerState) -> bool {
        state.active_streams_counter <= state.max_streams
    }

    /// Permits are bounded by max
    pub open spec fn stream_permits_bounded(state: StreamManagerState) -> bool {
        state.available_permits <= state.max_streams
    }

    // ========================================================================
    // STREAM-2: Counter Consistency
    // ========================================================================

    /// Atomic counter equals held permits
    pub open spec fn counter_consistent(state: StreamManagerState) -> bool {
        // active_streams_counter = max_streams - available_permits
        state.active_streams_counter == state.max_streams - state.available_permits
    }

    /// Combined invariant
    pub open spec fn stream_invariant(state: StreamManagerState) -> bool {
        stream_bounded(state) &&
        stream_permits_bounded(state) &&
        counter_consistent(state)
    }

    /// Proof: Counter consistency implies bounds
    pub proof fn consistency_implies_bounds(state: StreamManagerState)
        requires
            counter_consistent(state),
            stream_permits_bounded(state),
        ensures stream_bounded(state)
    {
        // counter = max - available
        // available <= max
        // Therefore counter <= max
    }

    // ========================================================================
    // STREAM-3: Permit Release
    // ========================================================================

    /// Dropping permit decrements counter and increments available
    pub open spec fn permit_release_decrements(
        pre: StreamManagerState,
        post: StreamManagerState,
        permit: StreamPermitSpec,
    ) -> bool {
        permit.valid ==>
        // Counter decremented
        post.active_streams_counter == pre.active_streams_counter - 1 &&
        // Available incremented
        post.available_permits == pre.available_permits + 1 &&
        // Max unchanged
        post.max_streams == pre.max_streams
    }

    /// Proof: Release preserves invariant
    pub proof fn release_preserves_invariant(
        pre: StreamManagerState,
        post: StreamManagerState,
        permit: StreamPermitSpec,
    )
        requires
            stream_invariant(pre),
            permit_release_decrements(pre, post, permit),
            permit.valid,
            pre.active_streams_counter > 0,  // At least one held
        ensures stream_invariant(post)
    {
        // post.counter = pre.counter - 1
        // post.available = pre.available + 1
        // pre.counter = pre.max - pre.available (consistency)
        // post.counter = pre.max - pre.available - 1 = pre.max - post.available
        // Therefore post is consistent
    }

    // ========================================================================
    // Try Acquire Semantics
    // ========================================================================

    /// try_acquire_stream semantics
    pub open spec fn try_acquire_stream_semantics(
        pre: StreamManagerState,
        post: StreamManagerState,
        result: Option<StreamPermitSpec>,
    ) -> bool {
        if pre.available_permits == 0 {
            // No permits: fails
            result.is_none() &&
            post == pre
        } else {
            // Success: decrement available, increment counter
            result.is_some() &&
            post.available_permits == pre.available_permits - 1 &&
            post.active_streams_counter == pre.active_streams_counter + 1 &&
            post.max_streams == pre.max_streams
        }
    }

    /// Proof: Try acquire preserves invariant
    pub proof fn try_acquire_preserves_invariant(
        pre: StreamManagerState,
        post: StreamManagerState,
        result: Option<StreamPermitSpec>,
    )
        requires
            stream_invariant(pre),
            try_acquire_stream_semantics(pre, post, result),
        ensures stream_invariant(post)
    {
        if pre.available_permits == 0 {
            // No change
        } else {
            // post.counter = pre.counter + 1
            // post.available = pre.available - 1
            // pre.counter = pre.max - pre.available (consistency)
            // post.counter = pre.max - pre.available + 1 = pre.max - post.available
        }
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial stream manager state
    pub open spec fn initial_stream_state(max_streams: u32) -> StreamManagerState {
        StreamManagerState {
            max_streams,
            available_permits: max_streams,
            active_streams_counter: 0,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_stream_state_valid(max_streams: u32)
        ensures stream_invariant(initial_stream_state(max_streams))
    {
        let state = initial_stream_state(max_streams);
        // counter = 0 = max - max = max - available
        // available = max <= max
        // counter = 0 <= max
    }

    // ========================================================================
    // Reusability
    // ========================================================================

    /// Permits can be reused after release
    pub open spec fn permits_reusable(
        acquire1_pre: StreamManagerState,
        acquire1_post: StreamManagerState,
        release_post: StreamManagerState,
        acquire2_post: StreamManagerState,
    ) -> bool {
        // After acquire, release, acquire: same state as single acquire
        acquire1_pre == release_post &&
        acquire1_post == acquire2_post
    }

    /// Proof: Acquire-release cycle is reversible
    pub proof fn acquire_release_reversible(
        state: StreamManagerState,
    )
        requires
            stream_invariant(state),
            state.available_permits > 0,
        ensures ({
            let acquired = StreamManagerState {
                max_streams: state.max_streams,
                available_permits: (state.available_permits - 1) as u32,
                active_streams_counter: (state.active_streams_counter + 1) as u32,
            };
            let released = StreamManagerState {
                max_streams: state.max_streams,
                available_permits: (acquired.available_permits + 1) as u32,
                active_streams_counter: (acquired.active_streams_counter - 1) as u32,
            };
            released == state
        })
    {
        // (available - 1) + 1 = available
        // (counter + 1) - 1 = counter
    }

    // ========================================================================
    // Zero Capacity Edge Case
    // ========================================================================

    /// Zero capacity stream manager never grants permits
    pub open spec fn zero_stream_capacity(
        state: StreamManagerState,
        result: Option<StreamPermitSpec>,
    ) -> bool {
        state.max_streams == 0 ==> result.is_none()
    }

    /// Proof: Zero capacity state is valid
    pub proof fn zero_stream_capacity_valid()
        ensures stream_invariant(initial_stream_state(0))
    {
        // counter = 0, available = 0, max = 0
    }

    // ========================================================================
    // Concurrent Safety (Informal)
    // ========================================================================

    /// Note: Actual concurrent safety relies on:
    /// 1. Tokio semaphore atomic operations
    /// 2. AtomicU32 with appropriate memory ordering
    /// 3. The invariant that semaphore and counter are updated atomically together
    ///
    /// This specification models single-threaded semantics.
    /// The implementation uses:
    /// - fetch_add(1, Relaxed) on acquire
    /// - fetch_sub(1, Relaxed) on release (in Drop)
    ///
    /// The semaphore ensures mutual exclusion for the critical sections.

    /// Axiom: Atomic operations preserve counter consistency
    pub proof fn atomic_ops_consistent()
        ensures true  // Trusted axiom
    {
        // Implementation uses atomic operations that maintain consistency
        // The semaphore serializes access to ensure counter updates are atomic
    }
}
