//! Timestamp Validation Specification
//!
//! Formal specification for timestamp-based validation of capability tokens.
//!
//! # Security Properties
//!
//! 1. **AUTH-9: Expiration Check**: Token expires at expires_at + tolerance
//! 2. **AUTH-10: Future Token Rejection**: issued_at <= now + tolerance
//!
//! # Verify with:
//! ```bash
//! verus --crate-type=lib crates/aspen-auth/verus/timestamp_spec.rs
//! ```

use vstd::prelude::*;

verus! {
    // ========================================================================
    // Constants
    // ========================================================================

    /// Clock skew tolerance in seconds (60 seconds)
    pub const TOKEN_CLOCK_SKEW_SECS: u64 = 60;

    // ========================================================================
    // State Model
    // ========================================================================

    /// Abstract timestamp state for token validation
    pub struct TokenTimestampSpec {
        /// Unix timestamp when token was issued
        pub issued_at: u64,
        /// Unix timestamp when token expires
        pub expires_at: u64,
    }

    // ========================================================================
    // Invariant 9: Expiration Check
    // ========================================================================

    /// AUTH-9: Token is not expired
    ///
    /// Token is considered expired when:
    ///   current_time > expires_at + clock_skew_tolerance
    ///
    /// The tolerance allows for minor clock differences between nodes.
    pub open spec fn not_expired(
        token: TokenTimestampSpec,
        current_time: u64,
        tolerance: u64,
    ) -> bool {
        // Token valid if expires_at + tolerance >= current_time
        token.expires_at + tolerance >= current_time
    }

    /// Default expiration check with standard tolerance
    pub open spec fn not_expired_default(
        token: TokenTimestampSpec,
        current_time: u64,
    ) -> bool {
        not_expired(token, current_time, TOKEN_CLOCK_SKEW_SECS)
    }

    // ========================================================================
    // Invariant 10: Future Token Rejection
    // ========================================================================

    /// AUTH-10: Token was not issued in the future
    ///
    /// Rejects tokens where:
    ///   issued_at > current_time + clock_skew_tolerance
    pub open spec fn not_from_future(
        token: TokenTimestampSpec,
        current_time: u64,
        tolerance: u64,
    ) -> bool {
        // Token valid if issued_at <= current_time + tolerance
        token.issued_at <= current_time + tolerance
    }

    /// Default future check with standard tolerance
    pub open spec fn not_from_future_default(
        token: TokenTimestampSpec,
        current_time: u64,
    ) -> bool {
        not_from_future(token, current_time, TOKEN_CLOCK_SKEW_SECS)
    }

    // ========================================================================
    // Combined Timestamp Validity
    // ========================================================================

    /// Combined timestamp validity
    pub open spec fn timestamp_valid(
        token: TokenTimestampSpec,
        current_time: u64,
        tolerance: u64,
    ) -> bool {
        not_expired(token, current_time, tolerance) &&
        not_from_future(token, current_time, tolerance) &&
        // Additional: issued_at <= expires_at
        token.issued_at <= token.expires_at
    }

    /// Default timestamp validity
    pub open spec fn timestamp_valid_default(
        token: TokenTimestampSpec,
        current_time: u64,
    ) -> bool {
        timestamp_valid(token, current_time, TOKEN_CLOCK_SKEW_SECS)
    }

    // ========================================================================
    // Validity Window
    // ========================================================================

    /// Earliest time token can be used (considering clock skew)
    pub open spec fn validity_window_start(
        token: TokenTimestampSpec,
        tolerance: u64,
    ) -> u64 {
        // Can be used as early as issued_at - tolerance (if issuer clock ahead)
        if token.issued_at >= tolerance {
            token.issued_at - tolerance
        } else {
            0
        }
    }

    /// Latest time token can be used (considering clock skew)
    pub open spec fn validity_window_end(
        token: TokenTimestampSpec,
        tolerance: u64,
    ) -> u64 {
        // Can be used as late as expires_at + tolerance
        token.expires_at + tolerance
    }

    /// Time is within validity window
    pub open spec fn in_validity_window(
        token: TokenTimestampSpec,
        current_time: u64,
        tolerance: u64,
    ) -> bool {
        current_time <= validity_window_end(token, tolerance) &&
        current_time >= validity_window_start(token, tolerance)
    }

    // ========================================================================
    // Proofs
    // ========================================================================

    /// Proof: Fresh token is immediately valid
    pub proof fn fresh_token_valid(
        issued_at: u64,
        expires_at: u64,
    )
        requires
            issued_at <= expires_at,
        ensures {
            let token = TokenTimestampSpec { issued_at, expires_at };
            timestamp_valid(token, issued_at, TOKEN_CLOCK_SKEW_SECS)
        }
    {
        let token = TokenTimestampSpec { issued_at, expires_at };
        // not_expired: expires_at + tolerance >= issued_at (since expires_at >= issued_at)
        assert(token.expires_at + TOKEN_CLOCK_SKEW_SECS >= issued_at);
        // not_from_future: issued_at <= issued_at + tolerance
        assert(token.issued_at <= issued_at + TOKEN_CLOCK_SKEW_SECS);
    }

    /// Proof: Expired token is invalid
    pub proof fn expired_token_invalid(
        token: TokenTimestampSpec,
        current_time: u64,
    )
        requires current_time > token.expires_at + TOKEN_CLOCK_SKEW_SECS
        ensures !not_expired_default(token, current_time)
    {
        // expires_at + tolerance < current_time
    }

    /// Proof: Future token is invalid
    pub proof fn future_token_invalid(
        token: TokenTimestampSpec,
        current_time: u64,
    )
        requires token.issued_at > current_time + TOKEN_CLOCK_SKEW_SECS
        ensures !not_from_future_default(token, current_time)
    {
        // issued_at > current_time + tolerance
    }

    /// Proof: Token valid during its window
    pub proof fn token_valid_during_window(
        token: TokenTimestampSpec,
        check_time: u64,
    )
        requires
            token.issued_at <= token.expires_at,
            check_time >= token.issued_at,
            check_time <= token.expires_at,
        ensures timestamp_valid(token, check_time, TOKEN_CLOCK_SKEW_SECS)
    {
        // not_expired: expires_at + tolerance >= check_time (since check_time <= expires_at)
        // not_from_future: issued_at <= check_time + tolerance (since check_time >= issued_at)
    }

    /// Proof: Expiration is monotonic
    pub proof fn expiration_monotonic(
        token: TokenTimestampSpec,
        time1: u64,
        time2: u64,
    )
        requires
            time1 <= time2,
            !not_expired(token, time1, TOKEN_CLOCK_SKEW_SECS),
        ensures !not_expired(token, time2, TOKEN_CLOCK_SKEW_SECS)
    {
        // If time1 > expires_at + tolerance, then time2 > expires_at + tolerance
    }

    // ========================================================================
    // Clock Skew Handling
    // ========================================================================

    /// Two nodes with clock skew can agree on validity
    pub open spec fn skew_compatible(
        token: TokenTimestampSpec,
        node1_time: u64,
        node2_time: u64,
        max_skew: u64,
        tolerance: u64,
    ) -> bool {
        // If skew <= tolerance, both nodes agree on validity
        (node1_time >= node2_time - max_skew &&
         node1_time <= node2_time + max_skew &&
         max_skew <= tolerance)
        ==>
        (timestamp_valid(token, node1_time, tolerance) ==
         timestamp_valid(token, node2_time, tolerance))
    }

    /// Proof: Clock skew within tolerance is acceptable
    pub proof fn skew_within_tolerance_ok(
        token: TokenTimestampSpec,
        issuer_time: u64,
        verifier_time: u64,
        skew: u64,
    )
        requires
            token.issued_at == issuer_time,
            token.expires_at == issuer_time + 3600,  // 1 hour validity
            verifier_time == issuer_time + skew,      // Verifier slightly behind
            skew <= TOKEN_CLOCK_SKEW_SECS,
        ensures timestamp_valid(token, verifier_time, TOKEN_CLOCK_SKEW_SECS)
    {
        // issued_at <= verifier_time + tolerance
        // expires_at + tolerance >= verifier_time
    }
}
