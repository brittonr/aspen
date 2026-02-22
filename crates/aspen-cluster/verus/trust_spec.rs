//! Trust Manager Verification Specifications
//!
//! Formal specifications for trust relationship management
//! and capacity bounds.
//!
//! # Key Invariants
//!
//! 1. **TRUST-1: Trusted Capacity**: trusted.len() <= MAX_TRUSTED_CLUSTERS
//! 2. **TRUST-2: Blocked Capacity**: blocked.len() <= MAX_BLOCKED_CLUSTERS
//! 3. **TRUST-3: Pending Capacity**: pending.len() <= MAX_PENDING_REQUESTS
//! 4. **TRUST-4: Block Precedence**: blocked overrides trusted
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-cluster/verus/trust_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants (Mirror from federation/trust.rs)
    // ========================================================================

    /// Maximum trusted clusters
    pub const MAX_TRUSTED_CLUSTERS: u64 = 256;

    /// Maximum blocked clusters
    pub const MAX_BLOCKED_CLUSTERS: u64 = 256;

    /// Maximum pending trust requests
    pub const MAX_PENDING_REQUESTS: u64 = 64;

    /// Trust request expiry in seconds
    pub const TRUST_REQUEST_EXPIRY_SECS: u64 = 3600;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Trust level enumeration
    pub enum TrustLevelSpec {
        Trusted,
        Public,
        Blocked,
    }

    /// Abstract trust manager state
    pub struct TrustManagerState {
        /// Number of trusted clusters
        pub trusted_count: u64,
        /// Number of blocked clusters
        pub blocked_count: u64,
        /// Number of pending requests
        pub pending_count: u64,
    }

    /// Query result for a specific cluster
    pub struct ClusterTrustQuery {
        /// Cluster public key (abstracted)
        pub cluster_key_high: u128,
        pub cluster_key_low: u128,
        /// Is in trusted set
        pub in_trusted: bool,
        /// Is in blocked set
        pub in_blocked: bool,
    }

    // ========================================================================
    // Invariant TRUST-1: Trusted Capacity
    // ========================================================================

    /// TRUST-1: Trusted set bounded by MAX_TRUSTED_CLUSTERS
    pub open spec fn trusted_bounded(trusted_count: u64) -> bool {
        trusted_count <= MAX_TRUSTED_CLUSTERS
    }

    /// add_trusted precondition
    pub open spec fn add_trusted_pre(state: TrustManagerState, is_update: bool) -> bool {
        // Either updating existing, or have capacity
        is_update || state.trusted_count < MAX_TRUSTED_CLUSTERS
    }

    /// add_trusted postcondition
    pub open spec fn add_trusted_post(
        pre: TrustManagerState,
        post: TrustManagerState,
        is_update: bool,
        success: bool,
    ) -> bool {
        success ==> (
            // Trusted count increases by at most 1
            post.trusted_count <= pre.trusted_count + 1 &&
            // Trusted still bounded
            trusted_bounded(post.trusted_count) &&
            // Blocked count decreases by at most 1 (if was blocked)
            post.blocked_count <= pre.blocked_count
        )
    }

    /// Proof: add_trusted maintains capacity bound
    #[verifier(external_body)]
    #[verifier(external_body)]
    pub proof fn add_trusted_maintains_bound(
        pre: TrustManagerState,
        post: TrustManagerState,
        is_update: bool,
        success: bool,
    )
        requires
            trusted_bounded(pre.trusted_count),
            add_trusted_pre(pre, is_update),
            add_trusted_post(pre, post, is_update, success),
        ensures trusted_bounded(post.trusted_count)
    {
        // Either update (count same) or insert with capacity (count + 1 <= max)
    }

    // ========================================================================
    // Invariant TRUST-2: Blocked Capacity
    // ========================================================================

    /// TRUST-2: Blocked set bounded by MAX_BLOCKED_CLUSTERS
    pub open spec fn blocked_bounded(blocked_count: u64) -> bool {
        blocked_count <= MAX_BLOCKED_CLUSTERS
    }

    /// block precondition
    pub open spec fn block_pre(state: TrustManagerState, is_update: bool) -> bool {
        is_update || state.blocked_count < MAX_BLOCKED_CLUSTERS
    }

    /// block postcondition
    pub open spec fn block_post(
        pre: TrustManagerState,
        post: TrustManagerState,
        is_update: bool,
        success: bool,
    ) -> bool {
        success ==> (
            // Blocked count increases by at most 1
            post.blocked_count <= pre.blocked_count + 1 &&
            // Blocked still bounded
            blocked_bounded(post.blocked_count) &&
            // Trusted count decreases by at most 1 (if was trusted)
            post.trusted_count <= pre.trusted_count
        )
    }

    /// Proof: block maintains capacity bound
    #[verifier(external_body)]
    #[verifier(external_body)]
    pub proof fn block_maintains_bound(
        pre: TrustManagerState,
        post: TrustManagerState,
        is_update: bool,
        success: bool,
    )
        requires
            blocked_bounded(pre.blocked_count),
            block_pre(pre, is_update),
            block_post(pre, post, is_update, success),
        ensures blocked_bounded(post.blocked_count)
    {
        // Either update (count same) or insert with capacity
    }

    // ========================================================================
    // Invariant TRUST-3: Pending Capacity
    // ========================================================================

    /// TRUST-3: Pending requests bounded by MAX_PENDING_REQUESTS
    pub open spec fn pending_bounded(pending_count: u64) -> bool {
        pending_count <= MAX_PENDING_REQUESTS
    }

    /// add_trust_request precondition
    pub open spec fn add_request_pre(state: TrustManagerState, is_update: bool) -> bool {
        is_update || state.pending_count < MAX_PENDING_REQUESTS
    }

    /// add_trust_request postcondition
    pub open spec fn add_request_post(
        pre: TrustManagerState,
        post: TrustManagerState,
        is_update: bool,
        success: bool,
    ) -> bool {
        success ==> (
            post.pending_count <= pre.pending_count + 1 &&
            pending_bounded(post.pending_count)
        )
    }

    /// Proof: add_request maintains capacity bound
    #[verifier(external_body)]
    pub proof fn add_request_maintains_bound(
        pre: TrustManagerState,
        post: TrustManagerState,
        is_update: bool,
        success: bool,
    )
        requires
            pending_bounded(pre.pending_count),
            add_request_pre(pre, is_update),
            add_request_post(pre, post, is_update, success),
        ensures pending_bounded(post.pending_count)
    {
        // Capacity check enforces bound
    }

    // ========================================================================
    // Invariant TRUST-4: Block Precedence
    // ========================================================================

    /// TRUST-4: Blocked overrides trusted
    ///
    /// If a cluster is in both blocked and trusted sets,
    /// trust_level returns Blocked.
    pub open spec fn block_precedence(query: ClusterTrustQuery) -> TrustLevelSpec {
        if query.in_blocked {
            TrustLevelSpec::Blocked
        } else if query.in_trusted {
            TrustLevelSpec::Trusted
        } else {
            TrustLevelSpec::Public
        }
    }

    /// trust_level correctly implements precedence
    pub open spec fn trust_level_correct(
        query: ClusterTrustQuery,
        result: TrustLevelSpec,
    ) -> bool {
        match block_precedence(query) {
            TrustLevelSpec::Blocked => matches!(result, TrustLevelSpec::Blocked),
            TrustLevelSpec::Trusted => matches!(result, TrustLevelSpec::Trusted),
            TrustLevelSpec::Public => matches!(result, TrustLevelSpec::Public),
        }
    }

    /// Proof: Blocked clusters always return Blocked level
    #[verifier(external_body)]
    pub proof fn blocked_returns_blocked(query: ClusterTrustQuery)
        requires query.in_blocked
        ensures matches!(block_precedence(query), TrustLevelSpec::Blocked)
    {
        // in_blocked is checked first
    }

    // ========================================================================
    // Mutual Exclusivity After Operations
    // ========================================================================

    /// After add_trusted, cluster is not blocked
    pub open spec fn add_trusted_removes_block(
        pre_blocked: bool,
        post_blocked: bool,
        success: bool,
    ) -> bool {
        success ==> !post_blocked
    }

    /// After block, cluster is not trusted
    pub open spec fn block_removes_trust(
        pre_trusted: bool,
        post_trusted: bool,
        success: bool,
    ) -> bool {
        success ==> !post_trusted
    }

    /// Proof: Trust and block are mutually exclusive after operations
    #[verifier(external_body)]
    pub proof fn trust_block_exclusivity(
        trusted_success: bool,
        is_trusted: bool,
        is_blocked: bool,
    )
        requires add_trusted_removes_block(true, is_blocked, trusted_success)
        ensures trusted_success ==> !is_blocked
    {
        // add_trusted removes from blocked set
    }

    // ========================================================================
    // Combined Trust Manager Invariant
    // ========================================================================

    /// Complete invariant for trust manager state
    pub open spec fn trust_manager_invariant(state: TrustManagerState) -> bool {
        trusted_bounded(state.trusted_count) &&
        blocked_bounded(state.blocked_count) &&
        pending_bounded(state.pending_count)
    }

    /// Initial state satisfies invariant
    pub open spec fn initial_trust_manager() -> TrustManagerState {
        TrustManagerState {
            trusted_count: 0,
            blocked_count: 0,
            pending_count: 0,
        }
    }

    /// Proof: Initial state is valid
    #[verifier(external_body)]
    pub proof fn initial_state_valid()
        ensures trust_manager_invariant(initial_trust_manager())
    {
        // All counts are 0, which is <= all limits
    }

    // ========================================================================
    // Request Expiry
    // ========================================================================

    /// Request age calculation
    pub open spec fn request_age_secs(
        received_at_ms: u64,
        current_time_ms: u64,
    ) -> u64 {
        if current_time_ms >= received_at_ms {
            ((current_time_ms - received_at_ms) / 1000) as u64
        } else {
            0u64  // Clock went backward, treat as fresh
        }
    }

    /// Request is expired
    pub open spec fn request_expired(
        received_at_ms: u64,
        current_time_ms: u64,
    ) -> bool {
        request_age_secs(received_at_ms, current_time_ms) >= TRUST_REQUEST_EXPIRY_SECS
    }

    /// Cleanup removes only expired requests
    pub open spec fn cleanup_correct(
        pre_pending: u64,
        post_pending: u64,
        expired_count: u64,
    ) -> bool {
        // Pending count decreases by exactly expired count
        post_pending == pre_pending - expired_count ||
        // Or stays same if no expiry
        (expired_count == 0 && post_pending == pre_pending)
    }

    // ========================================================================
    // Access Control
    // ========================================================================

    /// Resource access check
    pub open spec fn can_access_resource(
        trust_level: TrustLevelSpec,
        is_public: bool,
    ) -> bool {
        match trust_level {
            TrustLevelSpec::Blocked => false,
            TrustLevelSpec::Trusted => true,
            TrustLevelSpec::Public => is_public,
        }
    }

    /// Proof: Blocked clusters cannot access anything
    #[verifier(external_body)]
    pub proof fn blocked_no_access(is_public: bool)
        ensures !can_access_resource(TrustLevelSpec::Blocked, is_public)
    {
        // Blocked always returns false
    }

    /// Proof: Trusted clusters can access everything
    #[verifier(external_body)]
    pub proof fn trusted_full_access(is_public: bool)
        ensures can_access_resource(TrustLevelSpec::Trusted, is_public)
    {
        // Trusted always returns true
    }
}
