//! Pure functions for credential chain validation.
//!
//! Formally verified — see `verus/credential_spec.rs` for proofs.
//! These functions are deterministic, no I/O, no async.
//! Time is passed as an explicit parameter.

use alloc::vec::Vec;

use crate::capability::Capability;
use crate::constants::MAX_DELEGATION_DEPTH;

/// Fraction denominator for the refresh threshold (20% remaining lifetime).
const REFRESH_THRESHOLD_DIVISOR: u64 = 5;

/// Inputs for token-expiration checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokenExpiryInput {
    /// Token expiration time in seconds since epoch.
    pub expires_at_secs: u64,
    /// Current time in seconds since epoch.
    pub now_secs: u64,
    /// Allowed clock skew tolerance in seconds.
    pub clock_skew_tolerance_secs: u64,
}

/// Inputs for token-refresh checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RefreshCheckInput {
    /// Token issue time in seconds since epoch.
    pub issued_at_secs: u64,
    /// Token expiration time in seconds since epoch.
    pub expires_at_secs: u64,
    /// Current time in seconds since epoch.
    pub now_secs: u64,
}

/// Check if a delegation chain is valid structurally.
///
/// Verifies:
/// 1. Chain length does not exceed MAX_DELEGATION_DEPTH
/// 2. Each level attenuates capabilities (child is subset of parent)
/// 3. Each parent has the Delegate capability
///
/// Does NOT verify signatures or expiry (those require crypto/time).
///
/// # Arguments
///
/// * `leaf_caps` - Capabilities of the leaf (presented) token
/// * `leaf_depth` - Delegation depth of the leaf token
/// * `proof_caps` - Capabilities of each proof token, ordered parent-to-root
/// * `proof_has_delegate` - Whether each proof token has Delegate capability
#[inline]
pub fn is_credential_chain_valid(
    leaf_caps: &[Capability],
    leaf_depth: u8,
    proof_caps: &[Vec<Capability>],
    proof_has_delegate: &[bool],
) -> bool {
    if leaf_depth > MAX_DELEGATION_DEPTH {
        return false;
    }
    if proof_caps.len() != proof_has_delegate.len() {
        return false;
    }

    debug_assert!(leaf_depth <= MAX_DELEGATION_DEPTH);
    debug_assert_eq!(proof_caps.len(), proof_has_delegate.len());

    let mut child_caps = leaf_caps;
    for (parent_caps, has_delegate) in proof_caps.iter().zip(proof_has_delegate.iter().copied()) {
        if !has_delegate {
            return false;
        }
        debug_assert!(has_delegate);

        let is_parent_capability_superset = child_caps
            .iter()
            .all(|child_cap| parent_caps.iter().any(|parent_cap| parent_cap.contains(child_cap)));
        if !is_parent_capability_superset {
            return false;
        }
        debug_assert!(is_parent_capability_superset);

        child_caps = parent_caps;
    }

    true
}

/// Check if a credential's capabilities authorize access to a given prefix.
///
/// Returns true if any capability in the token authorizes Read for the prefix.
#[inline]
pub fn credential_authorized_for_prefix(caps: &[Capability], prefix: &str) -> bool {
    caps.iter().any(|cap| match cap {
        Capability::Read { prefix: cap_prefix } => prefix.starts_with(cap_prefix),
        Capability::Full { prefix: cap_prefix } => prefix.starts_with(cap_prefix),
        _ => false,
    })
}

/// Check if a token has expired given the current time.
///
/// Accounts for clock skew tolerance.
#[inline]
pub fn is_token_expired(input: TokenExpiryInput) -> bool {
    input
        .expires_at_secs
        .saturating_add(input.clock_skew_tolerance_secs)
        < input.now_secs
}

/// Check if a token needs refresh (within 20% of remaining lifetime).
#[inline]
pub fn needs_refresh(input: RefreshCheckInput) -> bool {
    let total_lifetime_secs = input.expires_at_secs.saturating_sub(input.issued_at_secs);
    let threshold_secs = total_lifetime_secs / REFRESH_THRESHOLD_DIVISOR;
    let remaining_secs = input.expires_at_secs.saturating_sub(input.now_secs);
    remaining_secs <= threshold_secs
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINGLE_LEVEL_DEPTH: u8 = 1;
    const ONE_LEVEL_PAST_MAX_DEPTH: u8 = MAX_DELEGATION_DEPTH.saturating_add(1);
    const BASE_EXPIRES_AT_SECS: u64 = 100;
    const EXPIRED_NOW_SECS: u64 = 200;
    const NON_EXPIRED_NOW_SECS: u64 = 150;
    const ZERO_SKEW_TOLERANCE_SECS: u64 = 0;
    const DEFAULT_CLOCK_SKEW_TOLERANCE_SECS: u64 = 60;
    const REFRESH_ISSUED_AT_SECS: u64 = 0;
    const REFRESH_EXPIRES_AT_SECS: u64 = 100;
    const REFRESH_NOT_NEEDED_NOW_SECS: u64 = 70;
    const REFRESH_THRESHOLD_NOW_SECS: u64 = 80;
    const REFRESH_NEEDED_NOW_SECS: u64 = 85;

    #[test]
    fn test_valid_chain_single_level() {
        let parent_caps = vec![Capability::Read { prefix: "data:".into() }, Capability::Delegate];
        let leaf_caps = vec![Capability::Read {
            prefix: "data:sub:".into(),
        }];

        assert!(is_credential_chain_valid(
            &leaf_caps,
            SINGLE_LEVEL_DEPTH,
            &[parent_caps],
            &[true],
        ));
    }

    #[test]
    fn test_invalid_chain_no_delegate() {
        let parent_caps = vec![Capability::Read { prefix: "data:".into() }];
        let leaf_caps = vec![Capability::Read {
            prefix: "data:sub:".into(),
        }];

        assert!(!is_credential_chain_valid(
            &leaf_caps,
            SINGLE_LEVEL_DEPTH,
            &[parent_caps],
            &[false],
        ));
    }

    #[test]
    fn test_invalid_chain_escalation() {
        let parent_caps = vec![Capability::Read { prefix: "data:".into() }, Capability::Delegate];
        let leaf_caps = vec![Capability::Write { prefix: "data:".into() }];

        assert!(!is_credential_chain_valid(
            &leaf_caps,
            SINGLE_LEVEL_DEPTH,
            &[parent_caps],
            &[true],
        ));
    }

    #[test]
    fn test_invalid_chain_too_deep() {
        assert!(!is_credential_chain_valid(
            &[],
            ONE_LEVEL_PAST_MAX_DEPTH,
            &[],
            &[],
        ));
    }

    #[test]
    fn test_authorized_for_prefix() {
        let caps = vec![Capability::Read {
            prefix: "_sys:nix-cache:".into(),
        }];

        assert!(credential_authorized_for_prefix(&caps, "_sys:nix-cache:narinfo:abc"));
        assert!(!credential_authorized_for_prefix(&caps, "_forge:repos:"));
    }

    #[test]
    fn test_authorized_for_prefix_full() {
        let caps = vec![Capability::Full { prefix: "data:".into() }];

        assert!(credential_authorized_for_prefix(&caps, "data:sub:key"));
        assert!(!credential_authorized_for_prefix(&caps, "other:key"));
    }

    #[test]
    fn test_is_token_expired() {
        assert!(is_token_expired(TokenExpiryInput {
            expires_at_secs: BASE_EXPIRES_AT_SECS,
            now_secs: EXPIRED_NOW_SECS,
            clock_skew_tolerance_secs: DEFAULT_CLOCK_SKEW_TOLERANCE_SECS,
        }));
        assert!(!is_token_expired(TokenExpiryInput {
            expires_at_secs: BASE_EXPIRES_AT_SECS,
            now_secs: NON_EXPIRED_NOW_SECS,
            clock_skew_tolerance_secs: DEFAULT_CLOCK_SKEW_TOLERANCE_SECS,
        }));
        assert!(!is_token_expired(TokenExpiryInput {
            expires_at_secs: BASE_EXPIRES_AT_SECS,
            now_secs: BASE_EXPIRES_AT_SECS,
            clock_skew_tolerance_secs: ZERO_SKEW_TOLERANCE_SECS,
        }));
    }

    #[test]
    fn test_needs_refresh() {
        assert!(needs_refresh(RefreshCheckInput {
            issued_at_secs: REFRESH_ISSUED_AT_SECS,
            expires_at_secs: REFRESH_EXPIRES_AT_SECS,
            now_secs: REFRESH_NEEDED_NOW_SECS,
        }));
        assert!(!needs_refresh(RefreshCheckInput {
            issued_at_secs: REFRESH_ISSUED_AT_SECS,
            expires_at_secs: REFRESH_EXPIRES_AT_SECS,
            now_secs: REFRESH_NOT_NEEDED_NOW_SECS,
        }));
        assert!(needs_refresh(RefreshCheckInput {
            issued_at_secs: REFRESH_ISSUED_AT_SECS,
            expires_at_secs: REFRESH_EXPIRES_AT_SECS,
            now_secs: REFRESH_THRESHOLD_NOW_SECS,
        }));
    }
}
