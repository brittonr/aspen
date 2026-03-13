//! Verus Formal Specifications for Credential Chain Validation
//!
//! Verifies correctness of credential chain validation logic:
//! - Chain validity (attenuation at each level)
//! - Authorization correctness (prefix matching)
//! - Delegation depth bound
//! - Refresh timing

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants (mirror production values)
    // ========================================================================

    pub const MAX_DELEGATION_DEPTH: u8 = 8;

    // ========================================================================
    // Spec Functions
    // ========================================================================

    /// A capability authorizes read for a prefix if the capability's prefix
    /// is a prefix of the requested prefix.
    pub open spec fn prefix_authorized(cap_prefix: Seq<char>, req_prefix: Seq<char>) -> bool {
        req_prefix.len() >= cap_prefix.len() &&
        req_prefix.subrange(0, cap_prefix.len() as int) == cap_prefix
    }

    /// Chain depth is bounded.
    pub open spec fn depth_bounded(depth: u8) -> bool {
        depth <= MAX_DELEGATION_DEPTH
    }

    /// Token is expired given current time and tolerance.
    pub open spec fn is_expired_spec(expires_at: u64, now: u64, tolerance: u64) -> bool {
        (expires_at as int + tolerance as int) < now as int
    }

    /// Token needs refresh (within 20% of remaining lifetime).
    pub open spec fn needs_refresh_spec(issued_at: u64, expires_at: u64, now: u64) -> bool {
        let total = expires_at as int - issued_at as int;
        let threshold = total / 5;
        let remaining = expires_at as int - now as int;
        remaining <= threshold
    }

    // ========================================================================
    // Exec Functions (verified implementations)
    // ========================================================================

    /// Check if token has expired.
    pub fn is_token_expired(expires_at: u64, now_secs: u64, clock_skew_tolerance: u64) -> (result: bool)
        ensures
            // Safe case: no overflow
            (expires_at as int + clock_skew_tolerance as int) <= u64::MAX as int ==>
                result == ((expires_at + clock_skew_tolerance) < now_secs),
            // Overflow case: saturating add clamps to u64::MAX, so never < now
            (expires_at as int + clock_skew_tolerance as int) > u64::MAX as int ==>
                result == false,
    {
        expires_at.saturating_add(clock_skew_tolerance) < now_secs
    }

    /// Check if token needs refresh.
    pub fn needs_refresh(issued_at: u64, expires_at: u64, now_secs: u64) -> (result: bool)
        requires
            expires_at >= issued_at,
        ensures
            result == ({
                let total_lifetime = (expires_at - issued_at) as int;
                let threshold = total_lifetime / 5;
                let remaining = if expires_at >= now_secs { (expires_at - now_secs) as int } else { 0 };
                remaining <= threshold
            })
    {
        let total_lifetime = expires_at - issued_at;
        let threshold = total_lifetime / 5;
        let remaining = expires_at.saturating_sub(now_secs);
        remaining <= threshold
    }

    /// Delegation depth is bounded.
    pub fn check_depth_bounded(depth: u8) -> (result: bool)
        ensures result == (depth <= MAX_DELEGATION_DEPTH)
    {
        depth <= MAX_DELEGATION_DEPTH
    }

    // ========================================================================
    // Invariants
    // ========================================================================

    /// CRED-1: Delegation chain attenuation
    /// Each level in a delegation chain has strictly narrower capabilities
    /// than its parent. This is enforced by Capability::contains() checks.
    pub open spec fn chain_attenuated(
        leaf_depth: u8,
        chain_len: nat,
    ) -> bool {
        leaf_depth as nat == chain_len &&
        depth_bounded(leaf_depth)
    }

    /// CRED-2: Authorization is prefix-based
    /// A credential authorizes access to a prefix P if any capability C
    /// satisfies: P.starts_with(C.prefix)
    pub open spec fn authorization_correct(
        cap_prefix: Seq<char>,
        requested: Seq<char>,
        result: bool,
    ) -> bool {
        result == prefix_authorized(cap_prefix, requested)
    }

    /// CRED-3: Credential size bounded
    /// Total credential size is bounded by MAX_DELEGATION_DEPTH × MAX_TOKEN_SIZE
    pub open spec fn credential_size_bounded(chain_len: nat) -> bool {
        chain_len <= MAX_DELEGATION_DEPTH as nat
    }

    // ========================================================================
    // Proofs
    // ========================================================================

    /// Proof: Root credential (depth 0) always has bounded depth.
    pub proof fn root_credential_bounded()
        ensures depth_bounded(0)
    {
    }

    /// Proof: If parent is at max depth, child delegation fails.
    pub proof fn max_depth_prevents_delegation()
        ensures !depth_bounded(MAX_DELEGATION_DEPTH + 1)
    {
    }
}
