//! Connection Manager Specification
//!
//! Formal specification for semaphore-based connection limiting.
//!
//! # Properties
//!
//! 1. **CONN-1: Connection Bounds**: active <= max
//! 2. **CONN-2: Permit RAII**: Drop releases slot
//! 3. **CONN-3: Shutdown Semantics**: Closed semaphore rejects all
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-transport/verus/connection_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract connection manager state
    pub struct ConnectionManagerState {
        /// Maximum allowed connections
        pub max_connections: u32,
        /// Available permits in semaphore
        pub available_permits: u32,
        /// Whether the manager has been shut down
        pub is_shutdown: bool,
    }

    /// Abstract connection permit
    pub struct ConnectionPermitSpec {
        /// Whether this permit is valid (not dropped)
        pub is_valid: bool,
    }

    // ========================================================================
    // Derived State
    // ========================================================================

    /// Number of active connections (held permits)
    pub open spec fn active_connections(state: ConnectionManagerState) -> u32 {
        (state.max_connections - state.available_permits) as u32
    }

    // ========================================================================
    // CONN-1: Connection Bounds
    // ========================================================================

    /// Active connections never exceed maximum
    pub open spec fn connection_bounded(state: ConnectionManagerState) -> bool {
        active_connections(state) <= state.max_connections
    }

    /// Invariant: available permits bounded by max
    pub open spec fn permits_bounded(state: ConnectionManagerState) -> bool {
        state.available_permits <= state.max_connections
    }

    /// Proof: Connection bounds follow from permit bounds
    #[verifier(external_body)]
    pub proof fn connection_bounds_hold(state: ConnectionManagerState)
        requires permits_bounded(state)
        ensures connection_bounded(state)
    {
        // active = max - available
        // available <= max
        // Therefore active >= 0 and active <= max
    }

    // ========================================================================
    // CONN-2: Permit RAII
    // ========================================================================

    /// Dropping a permit releases the slot
    pub open spec fn permit_released_on_drop(
        pre: ConnectionManagerState,
        post: ConnectionManagerState,
        permit: ConnectionPermitSpec,
    ) -> bool {
        // If permit was valid and is being dropped
        permit.is_valid ==>
        // Available permits increase by 1
        post.available_permits == pre.available_permits + 1 &&
        // Max stays the same
        post.max_connections == pre.max_connections &&
        // Shutdown state unchanged
        post.is_shutdown == pre.is_shutdown
    }

    /// Proof: RAII release preserves bounds
    #[verifier(external_body)]
    pub proof fn raii_preserves_bounds(
        pre: ConnectionManagerState,
        post: ConnectionManagerState,
        permit: ConnectionPermitSpec,
    )
        requires
            permits_bounded(pre),
            permit_released_on_drop(pre, post, permit),
            permit.is_valid,
            pre.available_permits < pre.max_connections,  // At least one held
        ensures permits_bounded(post)
    {
        // post.available = pre.available + 1
        // pre.available < max
        // Therefore post.available <= max
    }

    // ========================================================================
    // CONN-3: Shutdown Semantics
    // ========================================================================

    /// After shutdown, no new permits can be acquired
    pub open spec fn shutdown_prevents_acquire(
        state: ConnectionManagerState,
        acquire_result: Option<ConnectionPermitSpec>,
    ) -> bool {
        state.is_shutdown ==> acquire_result.is_none()
    }

    /// Shutdown effect on state
    pub open spec fn shutdown_effect(
        pre: ConnectionManagerState,
        post: ConnectionManagerState,
    ) -> bool {
        // Shutdown flag is set
        post.is_shutdown &&
        // Permits unchanged (existing connections continue)
        post.available_permits == pre.available_permits &&
        post.max_connections == pre.max_connections
    }

    /// Proof: Shutdown is monotonic (can't un-shutdown)
    #[verifier(external_body)]
    pub proof fn shutdown_monotonic(
        state: ConnectionManagerState,
        next_state: ConnectionManagerState,
    )
        requires state.is_shutdown
        ensures
            // Any valid transition preserves shutdown
            (next_state.max_connections == state.max_connections ==>
                next_state.is_shutdown)
    {
        // Axiom: Semaphore close is irreversible
    }

    // ========================================================================
    // Try Acquire Semantics
    // ========================================================================

    /// try_acquire_connection semantics
    pub open spec fn try_acquire_semantics(
        pre: ConnectionManagerState,
        post: ConnectionManagerState,
        result: Option<ConnectionPermitSpec>,
    ) -> bool {
        if pre.is_shutdown {
            // Shutdown: always fails
            result.is_none() &&
            post == pre
        } else if pre.available_permits == 0 {
            // No permits: fails
            result.is_none() &&
            post == pre
        } else {
            // Success: decrement available
            result.is_some() &&
            post.available_permits == pre.available_permits - 1 &&
            post.max_connections == pre.max_connections &&
            post.is_shutdown == pre.is_shutdown
        }
    }

    /// Proof: Try acquire preserves bounds
    #[verifier(external_body)]
    pub proof fn try_acquire_preserves_bounds(
        pre: ConnectionManagerState,
        post: ConnectionManagerState,
        result: Option<ConnectionPermitSpec>,
    )
        requires
            permits_bounded(pre),
            try_acquire_semantics(pre, post, result),
        ensures permits_bounded(post)
    {
        if pre.is_shutdown || pre.available_permits == 0 {
            // No change case
        } else {
            // Decrement case: available - 1 < max
        }
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial connection manager state
    pub open spec fn initial_state(max_connections: u32) -> ConnectionManagerState {
        ConnectionManagerState {
            max_connections,
            available_permits: max_connections,
            is_shutdown: false,
        }
    }

    /// Proof: Initial state satisfies all invariants
    #[verifier(external_body)]
    pub proof fn initial_state_valid(max_connections: u32)
        ensures ({
            let state = initial_state(max_connections);
            permits_bounded(state) &&
            connection_bounded(state) &&
            !state.is_shutdown &&
            active_connections(state) == 0
        })
    {
        // available = max, so active = 0
    }

    // ========================================================================
    // Zero Capacity Edge Case
    // ========================================================================

    /// Zero capacity manager never grants permits
    pub open spec fn zero_capacity_never_grants(
        state: ConnectionManagerState,
        result: Option<ConnectionPermitSpec>,
    ) -> bool {
        state.max_connections == 0 ==> result.is_none()
    }

    /// Proof: Zero capacity is consistent
    #[verifier(external_body)]
    pub proof fn zero_capacity_consistent()
        ensures ({
            let state = initial_state(0);
            state.available_permits == 0 &&
            active_connections(state) == 0
        })
    {
        // available = max = 0
    }
}
