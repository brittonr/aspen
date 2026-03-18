//! Verus specifications for DAG traversal pure functions.
//!
//! Proves correctness of the decision functions in `src/verified/traversal.rs`.

use vstd::prelude::*;

verus! {

    // ========================================================================
    // Spec Functions (mathematical definitions)
    // ========================================================================

    /// Spec: A node should be visited iff it is neither visited nor a known head.
    pub open spec fn should_visit_spec(is_visited: bool, is_known_head: bool) -> bool {
        !is_visited && !is_known_head
    }

    /// Spec: Depth is within bound iff strictly less than max.
    pub open spec fn is_within_depth_bound_spec(depth: u32, max_depth: u32) -> bool {
        depth < max_depth
    }

    /// Spec: Count is within bound iff strictly less than max.
    pub open spec fn is_within_count_bound_spec(count: u32, max_count: u32) -> bool {
        count < max_count
    }

    /// Spec: Visited set size after one insertion.
    pub open spec fn visited_set_size_after_insert(current: u32) -> int {
        if current as int + 1 > u32::MAX as int {
            u32::MAX as int
        } else {
            current as int + 1
        }
    }

    /// Spec: Child depth from parent depth.
    pub open spec fn child_depth_spec(parent_depth: u32) -> int {
        if parent_depth as int + 1 > u32::MAX as int {
            u32::MAX as int
        } else {
            parent_depth as int + 1
        }
    }

    // ========================================================================
    // Exec Functions (verified implementations)
    // ========================================================================

    /// TRAV-1, TRAV-5: Visit decision — excludes visited nodes and known heads.
    pub fn should_visit(is_visited: bool, is_known_head: bool) -> (result: bool)
        ensures result == should_visit_spec(is_visited, is_known_head)
    {
        !is_visited && !is_known_head
    }

    /// TRAV-2: Depth bound check.
    pub fn is_within_depth_bound(depth: u32, max_depth: u32) -> (result: bool)
        ensures result == is_within_depth_bound_spec(depth, max_depth)
    {
        depth < max_depth
    }

    /// TRAV-3: Count bound check.
    pub fn is_within_count_bound(count: u32, max_count: u32) -> (result: bool)
        ensures result == is_within_count_bound_spec(count, max_count)
    {
        count < max_count
    }

    /// TRAV-4: Visited set grows monotonically.
    pub fn compute_visited_set_size(current_size: u32) -> (result: u32)
        ensures
            result as int == visited_set_size_after_insert(current_size),
            result >= current_size,
    {
        if current_size == u32::MAX {
            u32::MAX
        } else {
            current_size + 1
        }
    }

    /// Child depth computation with overflow safety.
    pub fn compute_child_depth(parent_depth: u32) -> (result: u32)
        ensures
            result as int == child_depth_spec(parent_depth),
            result >= parent_depth,
    {
        if parent_depth == u32::MAX {
            u32::MAX
        } else {
            parent_depth + 1
        }
    }

    // ========================================================================
    // Invariants
    // ========================================================================

    /// TRAV-4: Visited set monotonicity — size only increases.
    pub open spec fn visited_set_monotonic(pre_size: u32, post_size: u32) -> bool {
        post_size >= pre_size
    }

    /// TRAV-2 + TRAV-3: Combined traversal boundedness.
    pub open spec fn traversal_bounded(depth: u32, count: u32, max_depth: u32, max_count: u32) -> bool {
        is_within_depth_bound_spec(depth, max_depth) &&
        is_within_count_bound_spec(count, max_count)
    }

    // ========================================================================
    // Proofs
    // ========================================================================

    /// Proof: Inserting into the visited set preserves monotonicity.
    proof fn visited_set_insert_monotonic(current_size: u32)
        ensures visited_set_monotonic(current_size, compute_visited_set_size(current_size) as u32)
    {
        // Follows from the ensures on compute_visited_set_size
    }

    /// Proof: A fresh traversal (depth=0, count=0) is always within bounds
    /// for any positive max values.
    proof fn initial_traversal_bounded(max_depth: u32, max_count: u32)
        requires
            max_depth > 0,
            max_count > 0,
        ensures
            traversal_bounded(0, 0, max_depth, max_count)
    {
        // depth=0 < max_depth > 0 and count=0 < max_count > 0
    }

    /// Proof: should_visit is false for any node in the visited set.
    proof fn visited_node_not_revisited(is_known_head: bool)
        ensures !should_visit_spec(true, is_known_head)
    {
        // is_visited=true means !is_visited=false, so conjunction is false
    }

    /// Proof: should_visit is false for any known head.
    proof fn known_head_not_descended(is_visited: bool)
        ensures !should_visit_spec(is_visited, true)
    {
        // is_known_head=true means !is_known_head=false, so conjunction is false
    }

} // verus!
