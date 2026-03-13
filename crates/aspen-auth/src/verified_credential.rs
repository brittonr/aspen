//! Pure functions for credential chain validation.
//!
//! Formally verified — see `verus/credential_spec.rs` for proofs.
//! These functions are deterministic, no I/O, no async.
//! Time is passed as an explicit parameter.

use crate::capability::Capability;
use crate::constants::MAX_DELEGATION_DEPTH;

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
    // Chain length check
    if leaf_depth > MAX_DELEGATION_DEPTH {
        return false;
    }
    if proof_caps.len() != proof_has_delegate.len() {
        return false;
    }

    // Check each level attenuates
    // The leaf's capabilities must be a subset of proof[0]'s capabilities
    // proof[0]'s capabilities must be a subset of proof[1]'s, etc.
    let mut child_caps = leaf_caps;

    for (i, parent_caps) in proof_caps.iter().enumerate() {
        // Parent must have Delegate capability
        if !proof_has_delegate[i] {
            return false;
        }

        // Each child capability must be contained by some parent capability
        for child_cap in child_caps {
            if !parent_caps.iter().any(|pc| pc.contains(child_cap)) {
                return false;
            }
        }

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
pub fn is_token_expired(expires_at: u64, now_secs: u64, clock_skew_tolerance: u64) -> bool {
    expires_at.saturating_add(clock_skew_tolerance) < now_secs
}

/// Check if a token needs refresh (within 20% of remaining lifetime).
#[inline]
pub fn needs_refresh(issued_at: u64, expires_at: u64, now_secs: u64) -> bool {
    let total_lifetime = expires_at.saturating_sub(issued_at);
    let threshold = total_lifetime / 5; // 20%
    let remaining = expires_at.saturating_sub(now_secs);
    remaining <= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_chain_single_level() {
        let parent_caps = vec![Capability::Read { prefix: "data:".into() }, Capability::Delegate];
        let leaf_caps = vec![Capability::Read {
            prefix: "data:sub:".into(),
        }];

        assert!(is_credential_chain_valid(&leaf_caps, 1, &[parent_caps], &[true],));
    }

    #[test]
    fn test_invalid_chain_no_delegate() {
        let parent_caps = vec![Capability::Read { prefix: "data:".into() }];
        let leaf_caps = vec![Capability::Read {
            prefix: "data:sub:".into(),
        }];

        assert!(!is_credential_chain_valid(
            &leaf_caps,
            1,
            &[parent_caps],
            &[false], // No Delegate
        ));
    }

    #[test]
    fn test_invalid_chain_escalation() {
        let parent_caps = vec![Capability::Read { prefix: "data:".into() }, Capability::Delegate];
        let leaf_caps = vec![Capability::Write { prefix: "data:".into() }];

        assert!(!is_credential_chain_valid(&leaf_caps, 1, &[parent_caps], &[true],));
    }

    #[test]
    fn test_invalid_chain_too_deep() {
        assert!(!is_credential_chain_valid(&[], MAX_DELEGATION_DEPTH + 1, &[], &[],));
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
        assert!(is_token_expired(100, 200, 60)); // 100 + 60 = 160 < 200
        assert!(!is_token_expired(100, 150, 60)); // 100 + 60 = 160 >= 150
        assert!(!is_token_expired(100, 100, 0)); // 100 >= 100
    }

    #[test]
    fn test_needs_refresh() {
        // Token issued at 0, expires at 100, now at 85 → remaining 15, threshold 20
        assert!(needs_refresh(0, 100, 85));
        // Token issued at 0, expires at 100, now at 70 → remaining 30, threshold 20
        assert!(!needs_refresh(0, 100, 70));
        // Token issued at 0, expires at 100, now at 80 → remaining 20, threshold 20
        assert!(needs_refresh(0, 100, 80));
    }
}
