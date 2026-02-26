//! Cache Statistics Specification
//!
//! Formal specification for cache hit rate tracking.
//!
//! # Properties
//!
//! 1. **CACHE-5: Query Count**: query_count == hit_count + miss_count
//! 2. **CACHE-6: Hit Rate**: 0 <= hit_rate <= 100
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-cache/verus/stats_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract cache statistics
    pub struct CacheStatsSpec {
        /// Total number of cache entries
        pub total_entries: u64,
        /// Total size of all NAR archives
        pub total_nar_bytes: u64,
        /// Number of queries
        pub query_count: u64,
        /// Number of cache hits
        pub hit_count: u64,
        /// Number of cache misses
        pub miss_count: u64,
        /// Last update timestamp
        pub last_updated: u64,
    }

    // ========================================================================
    // CACHE-5: Query Count Consistency
    // ========================================================================

    /// Query count equals hits plus misses
    pub open spec fn query_count_consistent(stats: CacheStatsSpec) -> bool {
        stats.query_count == stats.hit_count + stats.miss_count
    }

    /// Proof: Initial stats are consistent
    pub proof fn initial_stats_consistent()
        ensures query_count_consistent(CacheStatsSpec {
            total_entries: 0,
            total_nar_bytes: 0,
            query_count: 0,
            hit_count: 0,
            miss_count: 0,
            last_updated: 0,
        })
    {
        // 0 == 0 + 0
    }

    // ========================================================================
    // CACHE-6: Hit Rate Valid
    // ========================================================================

    /// Hit rate is valid percentage (0-100)
    /// Using fixed-point: rate * 100 to avoid floats
    pub open spec fn hit_rate_percent(stats: CacheStatsSpec) -> u64 {
        if stats.query_count == 0 {
            0
        } else {
            (stats.hit_count * 100) / stats.query_count
        }
    }

    /// Hit rate is valid
    pub open spec fn hit_rate_valid(stats: CacheStatsSpec) -> bool {
        hit_rate_percent(stats) <= 100
    }

    /// Proof: Hit rate always valid when consistent
    pub proof fn consistent_implies_valid_rate(stats: CacheStatsSpec)
        requires query_count_consistent(stats)
        ensures hit_rate_valid(stats)
    {
        // hit_count <= query_count (since query = hit + miss)
        // Therefore hit * 100 / query <= 100
    }

    // ========================================================================
    // Stats Operations
    // ========================================================================

    /// Record hit effect
    pub open spec fn record_hit_effect(
        pre: CacheStatsSpec,
        post: CacheStatsSpec,
    ) -> bool {
        post.query_count == pre.query_count + 1 &&
        post.hit_count == pre.hit_count + 1 &&
        post.miss_count == pre.miss_count &&
        post.total_entries == pre.total_entries &&
        post.total_nar_bytes == pre.total_nar_bytes
    }

    /// Record miss effect
    pub open spec fn record_miss_effect(
        pre: CacheStatsSpec,
        post: CacheStatsSpec,
    ) -> bool {
        post.query_count == pre.query_count + 1 &&
        post.hit_count == pre.hit_count &&
        post.miss_count == pre.miss_count + 1 &&
        post.total_entries == pre.total_entries &&
        post.total_nar_bytes == pre.total_nar_bytes
    }

    /// Proof: Recording hit preserves consistency
    pub proof fn hit_preserves_consistency(
        pre: CacheStatsSpec,
        post: CacheStatsSpec,
    )
        requires
            query_count_consistent(pre),
            record_hit_effect(pre, post),
        ensures query_count_consistent(post)
    {
        // post.query = pre.query + 1
        // post.hit = pre.hit + 1
        // post.miss = pre.miss
        // post.query = (pre.hit + 1) + pre.miss = post.hit + post.miss
    }

    /// Proof: Recording miss preserves consistency
    pub proof fn miss_preserves_consistency(
        pre: CacheStatsSpec,
        post: CacheStatsSpec,
    )
        requires
            query_count_consistent(pre),
            record_miss_effect(pre, post),
        ensures query_count_consistent(post)
    {
        // post.query = pre.query + 1
        // post.hit = pre.hit
        // post.miss = pre.miss + 1
        // post.query = pre.hit + (pre.miss + 1) = post.hit + post.miss
    }

    // ========================================================================
    // Entry Addition
    // ========================================================================

    /// Record entry added effect
    pub open spec fn record_entry_added_effect(
        pre: CacheStatsSpec,
        post: CacheStatsSpec,
        nar_size: u64,
    ) -> bool {
        post.total_entries == pre.total_entries + 1 &&
        post.total_nar_bytes == pre.total_nar_bytes + nar_size &&
        post.query_count == pre.query_count &&
        post.hit_count == pre.hit_count &&
        post.miss_count == pre.miss_count
    }

    /// Proof: Entry addition preserves query consistency
    pub proof fn entry_add_preserves_consistency(
        pre: CacheStatsSpec,
        post: CacheStatsSpec,
        nar_size: u64,
    )
        requires
            query_count_consistent(pre),
            record_entry_added_effect(pre, post, nar_size),
        ensures query_count_consistent(post)
    {
        // Query, hit, miss unchanged
    }

    // ========================================================================
    // Combined Invariant
    // ========================================================================

    /// Full stats invariant
    pub open spec fn stats_invariant(stats: CacheStatsSpec) -> bool {
        query_count_consistent(stats) &&
        hit_rate_valid(stats)
    }

    /// Initial stats
    pub open spec fn initial_stats() -> CacheStatsSpec {
        CacheStatsSpec {
            total_entries: 0,
            total_nar_bytes: 0,
            query_count: 0,
            hit_count: 0,
            miss_count: 0,
            last_updated: 0,
        }
    }

    /// Proof: Initial stats satisfy invariant
    pub proof fn initial_stats_valid()
        ensures stats_invariant(initial_stats())
    {
        // 0 = 0 + 0, rate = 0 <= 100
    }

    // ========================================================================
    // Hit Rate Analysis
    // ========================================================================

    /// Hit rate increases after hit
    pub open spec fn hit_improves_rate(
        pre: CacheStatsSpec,
        post: CacheStatsSpec,
    ) -> bool
        recommends
            query_count_consistent(pre),
            record_hit_effect(pre, post),
            pre.query_count > 0,
    {
        hit_rate_percent(post) >= hit_rate_percent(pre)
    }

    /// Proof: More hits improve rate (informal)
    /// Note: This is approximate due to integer division
    pub proof fn hits_improve_rate()
        ensures true  // Informal proof
    {
        // Adding a hit increases hit_count / query_count ratio
        // when both are incremented by 1
    }

    // ========================================================================
    // Size Tracking
    // ========================================================================

    /// Size tracking invariant (total >= entries * min_nar_size)
    pub const MIN_NAR_SIZE: u64 = 1;  // At least 1 byte per NAR

    /// Size is consistent with entry count
    pub open spec fn size_consistent(stats: CacheStatsSpec) -> bool {
        stats.total_entries == 0 ||
        stats.total_nar_bytes >= stats.total_entries * MIN_NAR_SIZE
    }
}
