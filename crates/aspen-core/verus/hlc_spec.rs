//! HLC (Hybrid Logical Clock) State Machine Model
//!
//! Abstract state model for formal verification of HLC operations.
//!
//! # State Model
//!
//! The `HlcState` captures:
//! - Current timestamp (wall_time, logical_counter, node_id)
//! - Last issued timestamp for monotonicity tracking
//!
//! # Key Invariants
//!
//! 1. **HLC-1: Monotonicity**: Timestamps strictly increase within a node
//! 2. **HLC-2: Causality**: Received timestamps advance the local clock
//! 3. **HLC-3: Total Order**: Any two timestamps are comparable
//! 4. **HLC-4: Bounded Drift**: Logical counter is bounded
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-core/verus/hlc_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract HLC timestamp structure
    ///
    /// Models the HlcTimestamp from hlc.rs
    pub struct HlcTimestampSpec {
        /// Wall clock time in nanoseconds (NTP64 format)
        pub wall_time_ns: u64,
        /// Logical counter for sub-nanosecond ordering
        pub logical_counter: u32,
        /// Node identifier (16 bytes, represented as two u64s)
        pub node_id_high: u64,
        pub node_id_low: u64,
    }

    /// Complete HLC state for verification
    pub struct HlcState {
        /// Last issued timestamp
        pub last_timestamp: HlcTimestampSpec,
        /// Node identifier (derived from node_id string via blake3)
        pub node_id_high: u64,
        pub node_id_low: u64,
    }

    // ========================================================================
    // Timestamp Comparison
    // ========================================================================

    /// Compare two timestamps for ordering
    ///
    /// Order is: wall_time_ns, then logical_counter, then node_id
    pub open spec fn timestamp_less_than(
        a: HlcTimestampSpec,
        b: HlcTimestampSpec,
    ) -> bool {
        if a.wall_time_ns < b.wall_time_ns {
            true
        } else if a.wall_time_ns > b.wall_time_ns {
            false
        } else if a.logical_counter < b.logical_counter {
            true
        } else if a.logical_counter > b.logical_counter {
            false
        } else if a.node_id_high < b.node_id_high {
            true
        } else if a.node_id_high > b.node_id_high {
            false
        } else {
            a.node_id_low < b.node_id_low
        }
    }

    /// Check if two timestamps are equal
    pub open spec fn timestamp_equal(
        a: HlcTimestampSpec,
        b: HlcTimestampSpec,
    ) -> bool {
        a.wall_time_ns == b.wall_time_ns &&
        a.logical_counter == b.logical_counter &&
        a.node_id_high == b.node_id_high &&
        a.node_id_low == b.node_id_low
    }

    /// Check if a timestamp is strictly less than or equal
    pub open spec fn timestamp_leq(
        a: HlcTimestampSpec,
        b: HlcTimestampSpec,
    ) -> bool {
        timestamp_less_than(a, b) || timestamp_equal(a, b)
    }

    // ========================================================================
    // Invariant 1: HLC Monotonicity
    // ========================================================================

    /// HLC-1: Timestamps strictly increase with each new_timestamp() call
    ///
    /// Given a pre-state and post-state after new_timestamp(),
    /// the new timestamp must be strictly greater than the old one.
    pub open spec fn hlc_monotonicity(
        pre: HlcState,
        post: HlcState,
    ) -> bool {
        timestamp_less_than(pre.last_timestamp, post.last_timestamp)
    }

    /// Weaker form: monotonicity is non-decreasing
    pub open spec fn hlc_monotonicity_weak(
        pre: HlcState,
        post: HlcState,
    ) -> bool {
        timestamp_leq(pre.last_timestamp, post.last_timestamp)
    }

    // ========================================================================
    // Invariant 2: HLC Causality
    // ========================================================================

    /// HLC-2: Receiving a timestamp advances local clock
    ///
    /// After update_from_timestamp(received), the next issued timestamp
    /// must be >= received timestamp.
    pub open spec fn hlc_causality(
        received: HlcTimestampSpec,
        post_update: HlcState,
    ) -> bool {
        // After update, last_timestamp >= received
        // Note: timestamp_leq already includes equality case, so no need for separate || timestamp_equal
        timestamp_leq(received, post_update.last_timestamp)
    }

    /// Precondition for update_timestamp to prevent counter overflow
    ///
    /// The logical counters must have room to increment. If a counter is at
    /// MAX_LOGICAL_COUNTER - 1, incrementing it would produce MAX_LOGICAL_COUNTER,
    /// which is the maximum representable value. To maintain the invariant that
    /// we can always issue another timestamp, we require counters be strictly
    /// less than MAX_LOGICAL_COUNTER - 1.
    pub open spec fn update_timestamp_pre(
        pre: HlcState,
        received: HlcTimestampSpec,
    ) -> bool {
        // Both local and received counters must have room to increment
        pre.last_timestamp.logical_counter < MAX_LOGICAL_COUNTER - 1 &&
        received.logical_counter < MAX_LOGICAL_COUNTER - 1
    }

    /// Result of update_from_timestamp operation
    ///
    /// SAFETY: Caller must ensure update_timestamp_pre holds to prevent overflow.
    /// In practice, the wall clock should advance before counters reach this limit.
    pub open spec fn update_timestamp_effect(
        pre: HlcState,
        received: HlcTimestampSpec,
        current_wall_time_ns: u64,
    ) -> HlcState {
        // max(local_wall, received_wall, current_wall)
        let max_wall = if pre.last_timestamp.wall_time_ns > received.wall_time_ns {
            if pre.last_timestamp.wall_time_ns > current_wall_time_ns {
                pre.last_timestamp.wall_time_ns
            } else {
                current_wall_time_ns
            }
        } else {
            if received.wall_time_ns > current_wall_time_ns {
                received.wall_time_ns
            } else {
                current_wall_time_ns
            }
        };

        // Compute logical counter based on which wall time won
        // When wall time advances, counter resets to 0 (no overflow risk)
        // When wall time stays same, we increment the relevant counter
        let new_counter = if max_wall == pre.last_timestamp.wall_time_ns &&
                            max_wall == received.wall_time_ns {
            // Both match max, take max of counters + 1
            let max_counter = if pre.last_timestamp.logical_counter > received.logical_counter {
                pre.last_timestamp.logical_counter
            } else {
                received.logical_counter
            };
            // SAFETY: update_timestamp_pre ensures max_counter < MAX_LOGICAL_COUNTER - 1,
            // so max_counter + 1 < MAX_LOGICAL_COUNTER and fits in u32
            (max_counter + 1) as u32
        } else if max_wall == pre.last_timestamp.wall_time_ns {
            // Local wins, increment its counter
            // SAFETY: update_timestamp_pre ensures logical_counter < MAX_LOGICAL_COUNTER - 1
            (pre.last_timestamp.logical_counter + 1) as u32
        } else if max_wall == received.wall_time_ns {
            // Received wins, increment its counter
            // SAFETY: update_timestamp_pre ensures logical_counter < MAX_LOGICAL_COUNTER - 1
            (received.logical_counter + 1) as u32
        } else {
            // Current wall time wins, reset counter
            0u32
        };

        HlcState {
            last_timestamp: HlcTimestampSpec {
                wall_time_ns: max_wall,
                logical_counter: new_counter,
                node_id_high: pre.node_id_high,
                node_id_low: pre.node_id_low,
            },
            node_id_high: pre.node_id_high,
            node_id_low: pre.node_id_low,
        }
    }

    // ========================================================================
    // Invariant 3: Total Order
    // ========================================================================

    /// HLC-3: Any two different timestamps are comparable
    ///
    /// For any two timestamps a and b where a != b,
    /// either a < b or b < a.
    pub open spec fn hlc_total_order(
        a: HlcTimestampSpec,
        b: HlcTimestampSpec,
    ) -> bool {
        timestamp_equal(a, b) ||
        timestamp_less_than(a, b) ||
        timestamp_less_than(b, a)
    }

    /// Proof that total order holds for all timestamps
    pub proof fn total_order_holds(a: HlcTimestampSpec, b: HlcTimestampSpec)
        ensures hlc_total_order(a, b)
    {
        // Case analysis on wall_time_ns
        if a.wall_time_ns < b.wall_time_ns {
            assert(timestamp_less_than(a, b));
        } else if a.wall_time_ns > b.wall_time_ns {
            assert(timestamp_less_than(b, a));
        } else {
            // wall_time_ns equal, compare logical_counter
            if a.logical_counter < b.logical_counter {
                assert(timestamp_less_than(a, b));
            } else if a.logical_counter > b.logical_counter {
                assert(timestamp_less_than(b, a));
            } else {
                // logical_counter equal, compare node_id
                if a.node_id_high < b.node_id_high {
                    assert(timestamp_less_than(a, b));
                } else if a.node_id_high > b.node_id_high {
                    assert(timestamp_less_than(b, a));
                } else {
                    // node_id_high equal
                    if a.node_id_low < b.node_id_low {
                        assert(timestamp_less_than(a, b));
                    } else if a.node_id_low > b.node_id_low {
                        assert(timestamp_less_than(b, a));
                    } else {
                        // All fields equal
                        assert(timestamp_equal(a, b));
                    }
                }
            }
        }
    }

    // ========================================================================
    // Invariant 4: Bounded Drift
    // ========================================================================

    /// Maximum allowed logical counter value before overflow concern
    pub const MAX_LOGICAL_COUNTER: u32 = 0xFFFF_FFFFu32;

    /// HLC-4: Logical counter is bounded
    ///
    /// The logical counter should reset when wall clock advances,
    /// preventing unbounded growth.
    pub open spec fn hlc_bounded_drift(ts: HlcTimestampSpec) -> bool {
        ts.logical_counter <= MAX_LOGICAL_COUNTER
    }

    /// Precondition for new_timestamp to prevent counter overflow
    ///
    /// When the wall clock hasn't advanced, we need room to increment the counter.
    /// The counter must be strictly less than MAX_LOGICAL_COUNTER - 1 to ensure
    /// the result fits and we can still issue another timestamp after this one.
    pub open spec fn new_timestamp_pre(pre: HlcState, current_wall_time_ns: u64) -> bool {
        // If wall clock advances, counter resets - no overflow possible
        // If wall clock doesn't advance, we need room to increment
        current_wall_time_ns > pre.last_timestamp.wall_time_ns ||
        pre.last_timestamp.logical_counter < MAX_LOGICAL_COUNTER - 1
    }

    /// New timestamp generation resets counter when wall clock advances
    ///
    /// SAFETY: Caller must ensure new_timestamp_pre holds. In practice, the wall
    /// clock should advance before the counter reaches this limit. If the wall
    /// clock is stuck and counter exhausted, the operation should return an error
    /// rather than overflow.
    pub open spec fn new_timestamp_effect(
        pre: HlcState,
        current_wall_time_ns: u64,
    ) -> HlcState {
        if current_wall_time_ns > pre.last_timestamp.wall_time_ns {
            // Wall clock advanced, reset counter
            HlcState {
                last_timestamp: HlcTimestampSpec {
                    wall_time_ns: current_wall_time_ns,
                    logical_counter: 0,
                    node_id_high: pre.node_id_high,
                    node_id_low: pre.node_id_low,
                },
                node_id_high: pre.node_id_high,
                node_id_low: pre.node_id_low,
            }
        } else {
            // Wall clock hasn't advanced, increment counter
            // SAFETY: new_timestamp_pre ensures logical_counter < MAX_LOGICAL_COUNTER - 1
            HlcState {
                last_timestamp: HlcTimestampSpec {
                    wall_time_ns: pre.last_timestamp.wall_time_ns,
                    logical_counter: (pre.last_timestamp.logical_counter + 1) as u32,
                    node_id_high: pre.node_id_high,
                    node_id_low: pre.node_id_low,
                },
                node_id_high: pre.node_id_high,
                node_id_low: pre.node_id_low,
            }
        }
    }

    /// Proof: new_timestamp maintains bounded drift if wall clock advances
    pub proof fn new_timestamp_resets_counter(
        pre: HlcState,
        current_wall_time_ns: u64,
    )
        requires current_wall_time_ns > pre.last_timestamp.wall_time_ns
        ensures new_timestamp_effect(pre, current_wall_time_ns).last_timestamp.logical_counter == 0
    {
        // When wall clock advances, counter resets to 0
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Combined invariant predicate for HLC state
    pub open spec fn hlc_invariant(state: HlcState) -> bool {
        // Node ID consistency
        state.last_timestamp.node_id_high == state.node_id_high &&
        state.last_timestamp.node_id_low == state.node_id_low &&
        // Bounded drift
        hlc_bounded_drift(state.last_timestamp)
    }

    // ========================================================================
    // Initial State
    // ========================================================================

    /// Initial HLC state (never used)
    pub open spec fn initial_hlc_state(
        node_id_high: u64,
        node_id_low: u64,
        current_wall_time_ns: u64,
    ) -> HlcState {
        HlcState {
            last_timestamp: HlcTimestampSpec {
                wall_time_ns: current_wall_time_ns,
                logical_counter: 0,
                node_id_high,
                node_id_low,
            },
            node_id_high,
            node_id_low,
        }
    }

    /// Proof: Initial state satisfies invariant
    pub proof fn initial_state_invariant(
        node_id_high: u64,
        node_id_low: u64,
        current_wall_time_ns: u64,
    )
        ensures hlc_invariant(initial_hlc_state(node_id_high, node_id_low, current_wall_time_ns))
    {
        // Initial state has:
        // - Matching node IDs
        // - logical_counter = 0 <= MAX_LOGICAL_COUNTER
    }

    // ========================================================================
    // Operation Proofs
    // ========================================================================

    /// Proof: new_timestamp preserves monotonicity
    pub proof fn new_timestamp_preserves_monotonicity(
        pre: HlcState,
        current_wall_time_ns: u64,
    )
        requires
            hlc_invariant(pre),
            new_timestamp_pre(pre, current_wall_time_ns),
        ensures ({
            let post = new_timestamp_effect(pre, current_wall_time_ns);
            hlc_monotonicity(pre, post)
        })
    {
        let post = new_timestamp_effect(pre, current_wall_time_ns);
        if current_wall_time_ns > pre.last_timestamp.wall_time_ns {
            // Wall time advanced, post.wall_time_ns > pre.wall_time_ns
            assert(timestamp_less_than(pre.last_timestamp, post.last_timestamp));
        } else {
            // Wall time same, counter incremented
            assert(post.last_timestamp.wall_time_ns == pre.last_timestamp.wall_time_ns);
            assert(post.last_timestamp.logical_counter == pre.last_timestamp.logical_counter + 1);
            assert(timestamp_less_than(pre.last_timestamp, post.last_timestamp));
        }
    }

    /// Proof: new_timestamp preserves invariant
    ///
    /// The precondition ensures either:
    /// 1. Wall clock advances (counter resets to 0), or
    /// 2. Counter has room to increment (< MAX_LOGICAL_COUNTER - 1)
    ///
    /// In case 2, counter + 1 < MAX_LOGICAL_COUNTER, satisfying hlc_bounded_drift.
    pub proof fn new_timestamp_preserves_invariant(
        pre: HlcState,
        current_wall_time_ns: u64,
    )
        requires
            hlc_invariant(pre),
            // Strengthened precondition: counter must have room for increment AND subsequent use
            new_timestamp_pre(pre, current_wall_time_ns),
        ensures hlc_invariant(new_timestamp_effect(pre, current_wall_time_ns))
    {
        let post = new_timestamp_effect(pre, current_wall_time_ns);
        // Node IDs are preserved
        assert(post.last_timestamp.node_id_high == post.node_id_high);
        assert(post.last_timestamp.node_id_low == post.node_id_low);
        // Counter is bounded
        if current_wall_time_ns > pre.last_timestamp.wall_time_ns {
            assert(post.last_timestamp.logical_counter == 0);
        } else {
            // new_timestamp_pre ensures logical_counter < MAX_LOGICAL_COUNTER - 1
            // So logical_counter + 1 < MAX_LOGICAL_COUNTER <= MAX_LOGICAL_COUNTER
            assert(post.last_timestamp.logical_counter == pre.last_timestamp.logical_counter + 1);
            assert(post.last_timestamp.logical_counter < MAX_LOGICAL_COUNTER);
            assert(post.last_timestamp.logical_counter <= MAX_LOGICAL_COUNTER);
        }
    }

    // ========================================================================
    // Additional Properties
    // ========================================================================

    /// Transitivity of timestamp ordering
    pub proof fn timestamp_transitivity(
        a: HlcTimestampSpec,
        b: HlcTimestampSpec,
        c: HlcTimestampSpec,
    )
        requires
            timestamp_less_than(a, b),
            timestamp_less_than(b, c),
        ensures timestamp_less_than(a, c)
    {
        // Lexicographic ordering is transitive
        // Case analysis on which field determines a < b
        if a.wall_time_ns < b.wall_time_ns {
            // a.wall_time_ns < b.wall_time_ns
            // From b < c: either b.wall_time_ns < c.wall_time_ns or b.wall_time_ns == c.wall_time_ns
            if b.wall_time_ns < c.wall_time_ns {
                assert(a.wall_time_ns < c.wall_time_ns);
            } else if b.wall_time_ns == c.wall_time_ns {
                // a.wall_time_ns < b.wall_time_ns == c.wall_time_ns
                assert(a.wall_time_ns < c.wall_time_ns);
            } else {
                // b.wall_time_ns > c.wall_time_ns contradicts b < c
                assert(false);
            }
        } else if a.wall_time_ns == b.wall_time_ns {
            // wall_time_ns equal, a.logical_counter determines or further fields
            if a.logical_counter < b.logical_counter {
                if b.wall_time_ns < c.wall_time_ns {
                    assert(a.wall_time_ns < c.wall_time_ns);
                } else if b.wall_time_ns == c.wall_time_ns {
                    if b.logical_counter < c.logical_counter {
                        assert(a.logical_counter < c.logical_counter);
                    } else if b.logical_counter == c.logical_counter {
                        // Continue to node_id comparison
                        assert(a.logical_counter < c.logical_counter);
                    }
                }
            } else if a.logical_counter == b.logical_counter {
                // Compare node_id_high
                if a.node_id_high < b.node_id_high {
                    if b.wall_time_ns < c.wall_time_ns {
                        assert(a.wall_time_ns < c.wall_time_ns);
                    } else if b.wall_time_ns == c.wall_time_ns {
                        if b.logical_counter < c.logical_counter {
                            assert(a.logical_counter < c.logical_counter);
                        } else if b.logical_counter == c.logical_counter {
                            if b.node_id_high < c.node_id_high {
                                assert(a.node_id_high < c.node_id_high);
                            } else if b.node_id_high == c.node_id_high {
                                assert(a.node_id_high < c.node_id_high);
                            }
                        }
                    }
                } else if a.node_id_high == b.node_id_high {
                    // Compare node_id_low
                    assert(a.node_id_low < b.node_id_low);
                    if b.wall_time_ns < c.wall_time_ns {
                        assert(a.wall_time_ns < c.wall_time_ns);
                    } else if b.wall_time_ns == c.wall_time_ns {
                        if b.logical_counter < c.logical_counter {
                            assert(a.logical_counter < c.logical_counter);
                        } else if b.logical_counter == c.logical_counter {
                            if b.node_id_high < c.node_id_high {
                                assert(a.node_id_high < c.node_id_high);
                            } else if b.node_id_high == c.node_id_high {
                                // b.node_id_low < c.node_id_low
                                assert(a.node_id_low < c.node_id_low);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Anti-symmetry: if a < b then not b < a
    pub proof fn timestamp_antisymmetry(
        a: HlcTimestampSpec,
        b: HlcTimestampSpec,
    )
        requires timestamp_less_than(a, b)
        ensures !timestamp_less_than(b, a)
    {
        // Case analysis on which field determines a < b
        if a.wall_time_ns < b.wall_time_ns {
            // a.wall_time_ns < b.wall_time_ns means b.wall_time_ns > a.wall_time_ns
            // So timestamp_less_than(b, a) requires b.wall_time_ns < a.wall_time_ns, which is false
            assert(b.wall_time_ns > a.wall_time_ns);
            assert(!timestamp_less_than(b, a));
        } else if a.wall_time_ns == b.wall_time_ns {
            if a.logical_counter < b.logical_counter {
                // b.logical_counter > a.logical_counter
                assert(b.logical_counter > a.logical_counter);
                assert(!timestamp_less_than(b, a));
            } else if a.logical_counter == b.logical_counter {
                if a.node_id_high < b.node_id_high {
                    assert(b.node_id_high > a.node_id_high);
                    assert(!timestamp_less_than(b, a));
                } else if a.node_id_high == b.node_id_high {
                    // a.node_id_low < b.node_id_low
                    assert(a.node_id_low < b.node_id_low);
                    assert(b.node_id_low > a.node_id_low);
                    assert(!timestamp_less_than(b, a));
                }
            }
        }
    }

    /// Irreflexivity: not a < a
    pub proof fn timestamp_irreflexivity(a: HlcTimestampSpec)
        ensures !timestamp_less_than(a, a)
    {
        // For a < a to hold, we would need one of:
        // - a.wall_time_ns < a.wall_time_ns (false)
        // - wall times equal AND a.logical_counter < a.logical_counter (false)
        // - wall times and counters equal AND a.node_id_high < a.node_id_high (false)
        // - all above equal AND a.node_id_low < a.node_id_low (false)
        // None of these can hold, so a < a is false
        assert(!(a.wall_time_ns < a.wall_time_ns));
        assert(!(a.logical_counter < a.logical_counter));
        assert(!(a.node_id_high < a.node_id_high));
        assert(!(a.node_id_low < a.node_id_low));
    }
}
