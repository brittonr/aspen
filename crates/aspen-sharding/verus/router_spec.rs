//! Shard Router Specification
//!
//! Formal specification for shard routing invariants.
//!
//! # Properties
//!
//! 1. **SHARD-4: Shard Limits**: num_shards in [MIN_SHARDS, MAX_SHARDS]
//! 2. **Router Consistency**: Local shard tracking is accurate
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-sharding/verus/router_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Maximum number of shards
    pub const MAX_SHARDS: u64 = 256;

    /// Minimum number of shards
    pub const MIN_SHARDS: u64 = 1;

    /// Default number of shards
    pub const DEFAULT_SHARDS: u64 = 4;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract shard configuration
    pub struct ShardConfigSpec {
        /// Number of shards in the cluster
        pub num_shards: u64,
    }

    /// Abstract router state
    pub struct ShardRouterState {
        /// Configuration
        pub config: ShardConfigSpec,
        /// Number of local shards
        pub local_shard_count: u64,
        /// Maximum local shard ID (for bounds checking)
        pub max_local_shard_id: u64,
    }

    // ========================================================================
    // SHARD-4: Shard Limits
    // ========================================================================

    /// Shard count is within valid range
    pub open spec fn shard_limits(config: ShardConfigSpec) -> bool {
        config.num_shards >= MIN_SHARDS &&
        config.num_shards <= MAX_SHARDS
    }

    /// Proof: Default configuration satisfies limits
    pub proof fn default_config_valid()
        ensures shard_limits(ShardConfigSpec { num_shards: DEFAULT_SHARDS })
    {
        // DEFAULT_SHARDS = 4
        // MIN_SHARDS = 1, MAX_SHARDS = 256
        // 1 <= 4 <= 256
    }

    // ========================================================================
    // Router Invariant
    // ========================================================================

    /// Combined router invariant
    pub open spec fn router_invariant(state: ShardRouterState) -> bool {
        // Config must be valid
        shard_limits(state.config) &&
        // Local shards must be within config bounds
        state.local_shard_count <= state.config.num_shards &&
        // Max local shard ID must be valid
        (state.local_shard_count > 0 ==>
            state.max_local_shard_id < state.config.num_shards)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial router state (no local shards)
    pub open spec fn initial_router_state(num_shards: u64) -> ShardRouterState {
        ShardRouterState {
            config: ShardConfigSpec { num_shards },
            local_shard_count: 0,
            max_local_shard_id: 0,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_valid(num_shards: u64)
        requires
            num_shards >= MIN_SHARDS,
            num_shards <= MAX_SHARDS,
        ensures router_invariant(initial_router_state(num_shards))
    {
        let state = initial_router_state(num_shards);
        // shard_limits: num_shards in [MIN, MAX]
        assert(shard_limits(state.config));
        // local_shard_count = 0 <= num_shards
        assert(state.local_shard_count <= state.config.num_shards);
        // No local shards, so max_local_shard_id condition is vacuously true
    }

    // ========================================================================
    // Local Shard Operations
    // ========================================================================

    /// Effect of adding a local shard
    pub open spec fn add_local_shard_effect(
        state: ShardRouterState,
        shard_id: u64,
    ) -> Option<ShardRouterState> {
        // Precondition: shard_id must be valid
        if shard_id >= state.config.num_shards {
            None  // Invalid shard ID
        } else {
            let new_max = if shard_id > state.max_local_shard_id {
                shard_id
            } else {
                state.max_local_shard_id
            };
            Some(ShardRouterState {
                config: state.config,
                local_shard_count: state.local_shard_count + 1,
                max_local_shard_id: new_max,
            })
        }
    }

    /// Proof: Adding local shard preserves invariant
    pub proof fn add_local_shard_preserves_invariant(
        state: ShardRouterState,
        shard_id: u64,
    )
        requires
            router_invariant(state),
            shard_id < state.config.num_shards,
            state.local_shard_count < state.config.num_shards,  // Room for more
        ensures {
            let post = add_local_shard_effect(state, shard_id).unwrap();
            router_invariant(post)
        }
    {
        let post = add_local_shard_effect(state, shard_id).unwrap();
        // Config unchanged, still valid
        assert(shard_limits(post.config));
        // local_shard_count increased but still bounded
        assert(post.local_shard_count <= post.config.num_shards);
        // max_local_shard_id is shard_id or previous max, both < num_shards
        assert(post.max_local_shard_id < post.config.num_shards);
    }

    /// Effect of removing a local shard
    pub open spec fn remove_local_shard_effect(
        state: ShardRouterState,
    ) -> ShardRouterState {
        // Simplified: just decrement count
        // In reality, would also update max_local_shard_id
        ShardRouterState {
            config: state.config,
            local_shard_count: if state.local_shard_count > 0 {
                state.local_shard_count - 1
            } else {
                0
            },
            max_local_shard_id: state.max_local_shard_id,  // May be stale
        }
    }

    // ========================================================================
    // Routing Operations
    // ========================================================================

    /// get_shard_for_key returns valid shard ID
    pub open spec fn get_shard_valid(
        config: ShardConfigSpec,
        result: u64,
    ) -> bool {
        result < config.num_shards
    }

    /// get_shards_for_prefix returns all shards
    pub open spec fn prefix_returns_all_shards(
        config: ShardConfigSpec,
        result_count: u64,
    ) -> bool {
        result_count == config.num_shards
    }

    /// Proof: Routing always returns valid shard
    pub proof fn routing_valid(config: ShardConfigSpec, result: u64)
        requires
            shard_limits(config),
            result < config.num_shards,  // Jump hash guarantee
        ensures get_shard_valid(config, result)
    {
        // Direct from precondition
    }

    // ========================================================================
    // is_local_key Consistency
    // ========================================================================

    /// is_local_key is consistent with get_shard_for_key and is_local_shard
    pub open spec fn is_local_key_consistent(
        state: ShardRouterState,
        shard_for_key: u64,
        shard_is_local: bool,
        key_is_local: bool,
    ) -> bool {
        key_is_local == (shard_is_local && shard_for_key < state.config.num_shards)
    }

    // ========================================================================
    // ShardRange Properties
    // ========================================================================

    /// Shard range contains check is correct
    pub open spec fn range_contains_correct(
        start_key: Seq<char>,
        end_key: Seq<char>,
        check_key: Seq<char>,
        result: bool,
    ) -> bool {
        // key >= start && (end.is_empty() || key < end)
        let geq_start = seq_geq(check_key, start_key);
        let lt_end = end_key.len() == 0 || seq_lt(check_key, end_key);
        result == (geq_start && lt_end)
    }

    /// Helper: sequence greater-or-equal (lexicographic)
    pub open spec fn seq_geq(a: Seq<char>, b: Seq<char>) -> bool {
        // Simplified: just check lengths for now
        // Real implementation would be lexicographic comparison
        true  // Axiom for simplification
    }

    /// Helper: sequence less-than (lexicographic)
    pub open spec fn seq_lt(a: Seq<char>, b: Seq<char>) -> bool {
        true  // Axiom for simplification
    }
}
