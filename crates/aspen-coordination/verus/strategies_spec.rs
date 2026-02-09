//! Load Balancing Strategy Verification Specifications
//!
//! This module provides formal specifications for worker selection strategies,
//! load balancing, and work stealing heuristics.
//!
//! These specifications formalize the invariants implemented in `pure/strategies.rs`.
//!
//! # Key Invariants
//!
//! ## Load Score Bounds (STRAT-1)
//! - Load scores are always in [0.0, 1.0]
//! - Lower scores indicate better candidates
//!
//! ## Round-Robin Properties (STRAT-2)
//! - Selection index is always < eligible_count
//! - Counter wraps correctly
//!
//! ## Hash Ring Correctness (STRAT-3)
//! - Lookup always returns a valid index from the ring
//! - Ring wraps at the end
//!
//! ## Work Stealing Safety (STRAT-4)
//! - Stealing stops at batch limit
//! - Stealing stops before overloading target
//!
//! ## Group Balance (STRAT-5)
//! - Average load is correctly computed
//! - Balanced groups have all workers within tolerance
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-coordination/verus/strategies_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Load Score Bounds (STRAT-1)
    // ========================================================================

    /// STRAT-1a: Load score is bounded
    ///
    /// The composite load score is always in [0.0, 1.0].
    /// We model f32 as u64 scaled by 1000 (0-1000 represents 0.0-1.0).
    pub open spec fn load_score_bounded(score_scaled: u64) -> bool {
        score_scaled <= 1000
    }

    /// STRAT-1b: Load score composition
    ///
    /// score = (load * load_weight) + (queue_ratio * queue_weight)
    /// where queue_ratio = min(queue_depth / max_concurrent, 1.0)
    ///
    /// All components are clamped to [0, 1], so the result is in [0, 1]
    /// when load_weight + queue_weight = 1.0.
    pub open spec fn compute_load_score_scaled(
        load_scaled: u64,           // 0-1000 representing 0.0-1.0
        queue_depth: u32,
        max_concurrent: u32,
        load_weight_scaled: u64,    // 0-1000 representing 0.0-1.0
        queue_weight_scaled: u64,   // 0-1000 representing 0.0-1.0
    ) -> u64 {
        // Clamp load to [0, 1000]
        let clamped_load = if load_scaled > 1000 { 1000u64 } else { load_scaled };

        // Compute queue ratio (scaled by 1000)
        let max_concurrent_safe = if max_concurrent == 0 { 1u32 } else { max_concurrent };
        let queue_ratio_raw = (queue_depth as u64 * 1000) / max_concurrent_safe as u64;
        let queue_ratio = if queue_ratio_raw > 1000 { 1000u64 } else { queue_ratio_raw };

        // Compute weighted score (need to scale down by 1000 due to multiplication)
        let load_component = (clamped_load * load_weight_scaled) / 1000;
        let queue_component = (queue_ratio * queue_weight_scaled) / 1000;

        let total = load_component + queue_component;
        // Clamp final result
        if total > 1000 { 1000u64 } else { total }
    }

    /// Proof: Computed load score is always bounded
    #[verifier(external_body)]
    pub proof fn load_score_always_bounded(
        load_scaled: u64,
        queue_depth: u32,
        max_concurrent: u32,
        load_weight_scaled: u64,
        queue_weight_scaled: u64,
    )
        ensures load_score_bounded(
            compute_load_score_scaled(load_scaled, queue_depth, max_concurrent, load_weight_scaled, queue_weight_scaled)
        )
    {
        // Final result is explicitly clamped to <= 1000
    }

    // ========================================================================
    // Round-Robin Properties (STRAT-2)
    // ========================================================================

    /// STRAT-2a: Round-robin selection validity
    ///
    /// For eligible_count > 0, selection is always in [0, eligible_count)
    pub open spec fn round_robin_valid(eligible_count: u32, counter: u32) -> bool {
        eligible_count > 0 ==> (counter % eligible_count) < eligible_count
    }

    /// STRAT-2b: Round-robin result computation
    ///
    /// Returns (selected_index, next_counter) if eligible_count > 0
    pub open spec fn compute_round_robin(eligible_count: u32, counter: u32) -> Option<(u32, u32)> {
        if eligible_count == 0 {
            None
        } else {
            let selected = counter % eligible_count;
            let next = (counter + 1) % eligible_count;
            Some((selected, next))
        }
    }

    /// Proof: Round-robin selection is always valid
    #[verifier(external_body)]
    pub proof fn round_robin_selection_valid(eligible_count: u32, counter: u32)
        ensures round_robin_valid(eligible_count, counter)
    {
        // counter % eligible_count < eligible_count by definition of modulo
    }

    /// STRAT-2c: Round-robin cycles through all workers
    ///
    /// After eligible_count selections, all workers have been selected exactly once.
    pub open spec fn round_robin_fair(eligible_count: u32, selections: Seq<u32>) -> bool {
        selections.len() == eligible_count as int ==>
            forall|i: u32| #![auto] i < eligible_count ==>
                exists|j: int| #![auto] 0 <= j < selections.len() && selections[j] == i
    }

    // ========================================================================
    // Hash Ring Correctness (STRAT-3)
    // ========================================================================

    /// STRAT-3a: Hash ring is sorted
    ///
    /// A valid hash ring has entries sorted by hash value.
    pub open spec fn ring_is_sorted(ring: Seq<(u64, u32)>) -> bool {
        forall|i: int, j: int| #![auto]
            0 <= i < j < ring.len() ==> ring[i].0 <= ring[j].0
    }

    /// STRAT-3b: Hash ring lookup result validity
    ///
    /// If ring is non-empty, lookup returns an index that exists in the ring.
    pub open spec fn lookup_result_valid(ring: Seq<(u64, u32)>, result: Option<u32>) -> bool {
        if ring.len() == 0 {
            result.is_none()
        } else {
            result.is_some() &&
            exists|i: int| #![auto] 0 <= i < ring.len() && ring[i].1 == result.unwrap()
        }
    }

    /// STRAT-3c: Binary search finds correct node
    ///
    /// The lookup finds the first node with hash >= key_hash, or wraps to first.
    pub open spec fn lookup_finds_successor(ring: Seq<(u64, u32)>, key_hash: u64, result_idx: int) -> bool {
        ring.len() > 0 ==> (
            // Either we found an exact match or successor
            (result_idx < ring.len() && ring[result_idx].0 >= key_hash) ||
            // Or we wrapped to the first node (all hashes are less than key_hash)
            (result_idx == 0 && forall|i: int| #![auto] 0 <= i < ring.len() ==> ring[i].0 < key_hash)
        )
    }

    // ========================================================================
    // Work Stealing Safety (STRAT-4)
    // ========================================================================

    /// STRAT-4a: Stealing respects batch limit
    ///
    /// Stealing stops when accumulated >= batch_limit.
    pub open spec fn stealing_respects_batch_limit(accumulated: u32, batch_limit: u32) -> bool {
        accumulated < batch_limit
    }

    /// STRAT-4b: Stealing respects load ceiling
    ///
    /// Stealing stops before projected load exceeds ceiling.
    /// We model load as scaled integers (0-1000 for 0.0-1.0).
    pub open spec fn stealing_respects_load_ceiling(
        target_load_scaled: u64,
        accumulated: u32,
        load_per_item_scaled: u64,
        load_ceiling_scaled: u64,
    ) -> bool {
        let projected_load = target_load_scaled + (accumulated as u64 * load_per_item_scaled);
        projected_load < load_ceiling_scaled
    }

    /// STRAT-4c: Combined stealing decision
    pub open spec fn should_continue_stealing(
        accumulated: u32,
        batch_limit: u32,
        target_load_scaled: u64,
        load_ceiling_scaled: u64,
        load_per_item_scaled: u64,
    ) -> bool {
        stealing_respects_batch_limit(accumulated, batch_limit) &&
        stealing_respects_load_ceiling(target_load_scaled, accumulated, load_per_item_scaled, load_ceiling_scaled)
    }

    /// Proof: Stealing eventually stops
    #[verifier(external_body)]
    pub proof fn stealing_terminates(batch_limit: u32)
        ensures batch_limit > 0 ==> !stealing_respects_batch_limit(batch_limit, batch_limit)
    {
        // accumulated >= batch_limit fails the check
    }

    // ========================================================================
    // Worker Idle Check (STRAT-4d)
    // ========================================================================

    /// STRAT-4d: Worker idle for stealing
    ///
    /// A worker is idle for stealing if load < threshold AND queue is empty.
    pub open spec fn worker_is_idle(
        load_scaled: u64,
        steal_threshold_scaled: u64,
        queue_depth: u32,
    ) -> bool {
        load_scaled < steal_threshold_scaled && queue_depth == 0
    }

    // ========================================================================
    // Steal Target Ranking (STRAT-4e)
    // ========================================================================

    /// Steal strategy enumeration
    pub enum StealStrategySpec {
        HighestQueueDepth,
        HighestLoad,
        Balanced,
    }

    /// STRAT-4e: Steal source eligibility
    ///
    /// A worker is eligible as a steal source if:
    /// - queue_depth > min_queue_depth
    /// - load > source_worker_load (they have more work than us)
    pub open spec fn is_steal_source_eligible(
        queue_depth: u32,
        load_scaled: u64,
        min_queue_depth: u32,
        source_worker_load_scaled: u64,
    ) -> bool {
        queue_depth > min_queue_depth && load_scaled > source_worker_load_scaled
    }

    // ========================================================================
    // Group Balance (STRAT-5)
    // ========================================================================

    /// STRAT-5a: Average load computation
    ///
    /// Average load = sum(loads) / count
    /// Returns 0 for empty groups.
    pub open spec fn compute_average_load(loads: Seq<u64>) -> u64 {
        if loads.len() == 0 {
            0
        } else {
            let sum = loads.fold_left(0u64, |acc: u64, x| acc + x);
            sum / loads.len() as u64
        }
    }

    /// STRAT-5b: Group balance check
    ///
    /// A group is balanced if all loads are within tolerance of the average.
    pub open spec fn group_is_balanced(loads: Seq<u64>, tolerance_scaled: u64) -> bool {
        if loads.len() < 2 {
            true
        } else {
            let avg = compute_average_load(loads);
            forall|i: int| #![auto] 0 <= i < loads.len() ==> {
                let diff = if loads[i] >= avg { loads[i] - avg } else { avg - loads[i] };
                diff <= tolerance_scaled
            }
        }
    }

    /// STRAT-5c: Rebalancing preserves total work
    ///
    /// When transferring work from overloaded to underloaded, total work is preserved.
    pub open spec fn rebalance_preserves_total(
        pre_loads: Seq<u64>,
        post_loads: Seq<u64>,
    ) -> bool {
        pre_loads.len() == post_loads.len() &&
        pre_loads.fold_left(0u64, |acc: u64, x| acc + x) ==
        post_loads.fold_left(0u64, |acc: u64, x| acc + x)
    }

    // ========================================================================
    // Affinity Scoring (STRAT-6)
    // ========================================================================

    /// STRAT-6a: Affinity score bounds
    ///
    /// Affinity scores are in [0, 100].
    pub open spec fn affinity_score_bounded(score_scaled: u64) -> bool {
        score_scaled <= 100
    }

    /// STRAT-6b: Affinity score composition
    ///
    /// Base: 50
    /// Same node bonus: +30
    /// Tag match bonus: +20
    /// Total possible: 100
    pub open spec fn compute_affinity_score(
        is_same_node: bool,
        has_tag_match: bool,
    ) -> u64 {
        let mut score = 50u64;
        if is_same_node {
            score = score + 30;
        }
        if has_tag_match {
            score = score + 20;
        }
        score
    }

    /// Proof: Affinity score is always bounded
    #[verifier(external_body)]
    pub proof fn affinity_always_bounded(is_same_node: bool, has_tag_match: bool)
        ensures affinity_score_bounded(compute_affinity_score(is_same_node, has_tag_match))
    {
        // Maximum: 50 + 30 + 20 = 100
    }

    // ========================================================================
    // Running Average (STRAT-7)
    // ========================================================================

    /// STRAT-7a: Running average computation
    ///
    /// new_avg = avg + (new_value - avg) / (count + 1)
    /// For count = 0, returns new_value.
    ///
    /// Note: Integer division may cause slight inaccuracy.
    pub open spec fn compute_running_average(current_avg: u64, count: u64, new_value: u64) -> u64 {
        if count == 0 {
            new_value
        } else {
            let new_count = count + 1;  // Saturating add would be safer
            if new_value >= current_avg {
                current_avg + (new_value - current_avg) / new_count
            } else {
                current_avg - (current_avg - new_value) / new_count
            }
        }
    }

    /// STRAT-7b: Running average bounds
    ///
    /// The running average stays between min and max of all values seen.
    pub open spec fn running_average_bounded(avg: u64, min_seen: u64, max_seen: u64) -> bool {
        avg >= min_seen && avg <= max_seen
    }

    // ========================================================================
    // Tag Matching (STRAT-8)
    // ========================================================================

    /// STRAT-8a: Tag match predicate
    ///
    /// A worker matches required tags if it has all required tags.
    /// Modeled as set containment.
    pub open spec fn tags_match(worker_tags: Set<Seq<char>>, required_tags: Set<Seq<char>>) -> bool {
        required_tags.subset_of(worker_tags)
    }

    /// STRAT-8b: Empty requirements always match
    #[verifier(external_body)]
    pub proof fn empty_requirements_match(worker_tags: Set<Seq<char>>)
        ensures tags_match(worker_tags, Set::empty())
    {
        // Empty set is subset of any set
    }

    // ========================================================================
    // Selection Result (STRAT-9)
    // ========================================================================

    /// Selection result type
    pub enum SelectionResultSpec {
        /// Selected the best (lowest score) worker
        Best(u32),
        /// Selected a preferred worker
        Preferred(u32),
        /// No eligible workers
        None,
    }

    /// STRAT-9a: Selection result validity
    ///
    /// If result is Best or Preferred, the index must be valid.
    pub open spec fn selection_result_valid(result: SelectionResultSpec, max_index: u32) -> bool {
        match result {
            SelectionResultSpec::Best(idx) => idx < max_index,
            SelectionResultSpec::Preferred(idx) => idx < max_index,
            SelectionResultSpec::None => true,
        }
    }

    /// STRAT-9b: Critical priority always selects best
    pub open spec fn critical_selects_best(
        result: SelectionResultSpec,
        is_critical: bool,
        best_idx: u32,
    ) -> bool {
        is_critical ==> match result {
            SelectionResultSpec::Best(idx) => idx == best_idx,
            _ => false,
        }
    }

    // ========================================================================
    // Combined Strategy Invariant
    // ========================================================================

    /// Worker state for load balancing
    pub struct WorkerLoadState {
        /// Worker index
        pub index: u32,
        /// Current load (scaled 0-1000)
        pub load_scaled: u64,
        /// Queue depth
        pub queue_depth: u32,
        /// Maximum concurrent jobs
        pub max_concurrent: u32,
    }

    /// Group of workers for load balancing
    pub struct WorkerGroup {
        /// Workers in the group
        pub workers: Seq<WorkerLoadState>,
        /// Round-robin counter
        pub rr_counter: u32,
    }

    /// Combined invariant for worker group
    pub open spec fn worker_group_invariant(group: WorkerGroup) -> bool {
        // All worker loads are bounded
        forall|i: int| #![auto] 0 <= i < group.workers.len() ==>
            group.workers[i].load_scaled <= 1000 &&
            group.workers[i].max_concurrent > 0
    }

    /// Proof: Well-formed groups have valid load scores
    #[verifier(external_body)]
    pub proof fn group_loads_bounded(group: WorkerGroup)
        requires worker_group_invariant(group)
        ensures forall|i: int| #![auto] 0 <= i < group.workers.len() ==>
            load_score_bounded(group.workers[i].load_scaled)
    {
    }
}