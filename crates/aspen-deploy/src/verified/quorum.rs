//! Quorum safety computations for rolling deployments.
//!
//! Pure functions that determine whether it's safe to upgrade a node
//! without losing Raft quorum. All functions are deterministic with
//! no I/O dependencies.
//!
//! Formally verified — see `verus/quorum_spec.rs` for proofs.

/// Compute the quorum size (minimum voters for majority).
///
/// Returns `ceil((voter_count + 1) / 2)`.
///
/// # Examples
///
/// - 1 voter → quorum 1
/// - 3 voters → quorum 2
/// - 5 voters → quorum 3
/// - 7 voters → quorum 4
#[inline]
pub fn quorum_size(voter_count: u32) -> u32 {
    // ceil((voter_count + 1) / 2) via integer arithmetic
    // Equivalent: (voter_count / 2) + 1
    voter_count.saturating_add(1).saturating_add(1) / 2
}

/// Check if it's safe to upgrade one more node.
///
/// Returns true if after upgrading one additional node, the cluster
/// still has enough healthy voters for quorum.
///
/// The invariant is:
/// `(healthy_voters - currently_upgrading - 1) >= quorum_size(voter_count)`
///
/// # Parameters
///
/// - `healthy_voters`: Number of voters currently healthy and serving
/// - `currently_upgrading`: Number of voters currently mid-upgrade (unavailable)
/// - `voter_count`: Total number of voters in the cluster
#[inline]
pub fn can_upgrade_node(healthy_voters: u32, currently_upgrading: u32, voter_count: u32) -> bool {
    let quorum = quorum_size(voter_count);
    let remaining = healthy_voters.saturating_sub(currently_upgrading).saturating_sub(1);
    remaining >= quorum
}

/// Compute the maximum number of nodes that can be upgraded simultaneously.
///
/// Returns `(voter_count - 1) / 2`, minimum 1.
///
/// This is the theoretical upper bound. For 3 nodes: max 1. For 5 nodes: max 2.
/// For 7 nodes: max 3.
///
/// Note: 1-node clusters return 1 (the single node must be upgraded directly).
#[inline]
pub fn max_concurrent_upgrades(voter_count: u32) -> u32 {
    if voter_count <= 1 {
        return 1;
    }
    let max = voter_count.saturating_sub(1) / 2;
    max.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quorum_size() {
        assert_eq!(quorum_size(1), 1);
        assert_eq!(quorum_size(2), 2); // 2-node cluster needs both for quorum
        assert_eq!(quorum_size(3), 2);
        assert_eq!(quorum_size(4), 3);
        assert_eq!(quorum_size(5), 3);
        assert_eq!(quorum_size(6), 4);
        assert_eq!(quorum_size(7), 4);
    }

    #[test]
    fn test_quorum_size_zero() {
        // Edge case: 0 voters
        assert_eq!(quorum_size(0), 1);
    }

    #[test]
    fn test_can_upgrade_node_3_node_cluster() {
        // 3 voters, all healthy, none upgrading
        // Can upgrade 1: 3 - 0 - 1 = 2 >= quorum(3)=2 → true
        assert!(can_upgrade_node(3, 0, 3));

        // 3 voters, all healthy, 1 already upgrading
        // Can upgrade another: 3 - 1 - 1 = 1 < quorum(3)=2 → false
        assert!(!can_upgrade_node(3, 1, 3));
    }

    #[test]
    fn test_can_upgrade_node_5_node_cluster() {
        // 5 voters, all healthy, none upgrading
        // Can upgrade 1: 5 - 0 - 1 = 4 >= quorum(5)=3 → true
        assert!(can_upgrade_node(5, 0, 5));

        // 5 voters, all healthy, 1 upgrading
        // Can upgrade another: 5 - 1 - 1 = 3 >= quorum(5)=3 → true
        assert!(can_upgrade_node(5, 1, 5));

        // 5 voters, all healthy, 2 upgrading
        // Can upgrade another: 5 - 2 - 1 = 2 < quorum(5)=3 → false
        assert!(!can_upgrade_node(5, 2, 5));
    }

    #[test]
    fn test_can_upgrade_node_7_node_cluster() {
        // 7 voters, all healthy
        assert!(can_upgrade_node(7, 0, 7)); // 7-0-1=6 >= 4
        assert!(can_upgrade_node(7, 1, 7)); // 7-1-1=5 >= 4
        assert!(can_upgrade_node(7, 2, 7)); // 7-2-1=4 >= 4
        assert!(!can_upgrade_node(7, 3, 7)); // 7-3-1=3 < 4
    }

    #[test]
    fn test_can_upgrade_node_1_node_cluster() {
        // Single-node cluster: quorum is 1, can always upgrade
        // 1 - 0 - 1 = 0 < quorum(1)=1 → false
        // But the design says single-node must accept brief unavailability
        assert!(!can_upgrade_node(1, 0, 1));
    }

    #[test]
    fn test_can_upgrade_node_unhealthy_voter() {
        // 3 voters, only 2 healthy, none upgrading
        // 2 - 0 - 1 = 1 < quorum(3)=2 → false
        assert!(!can_upgrade_node(2, 0, 3));
    }

    #[test]
    fn test_max_concurrent_upgrades() {
        assert_eq!(max_concurrent_upgrades(1), 1);
        assert_eq!(max_concurrent_upgrades(2), 1);
        assert_eq!(max_concurrent_upgrades(3), 1);
        assert_eq!(max_concurrent_upgrades(4), 1);
        assert_eq!(max_concurrent_upgrades(5), 2);
        assert_eq!(max_concurrent_upgrades(6), 2);
        assert_eq!(max_concurrent_upgrades(7), 3);
    }

    #[test]
    fn test_max_concurrent_upgrades_zero() {
        assert_eq!(max_concurrent_upgrades(0), 1);
    }

    // Invariant: max_concurrent_upgrades(n) should always leave quorum intact
    // for clusters with odd voter counts >= 3. Even voter counts and n<=2
    // cannot maintain quorum during any upgrade.
    #[test]
    fn test_max_concurrent_preserves_quorum() {
        for n in 1..=20 {
            let max = max_concurrent_upgrades(n);
            let remaining = n.saturating_sub(max);
            let quorum = quorum_size(n);
            // Odd clusters >= 3 can always maintain quorum with max_concurrent
            // Even clusters and n<=2 accept brief unavailability
            if n >= 3 && n % 2 == 1 {
                assert!(
                    remaining >= quorum,
                    "max_concurrent_upgrades({n}) = {max} leaves {remaining} \
                     which is less than quorum {quorum}"
                );
            }
        }
    }

    // Invariant: can_upgrade_node(n, max-1, n) should be true for valid clusters
    #[test]
    fn test_can_upgrade_at_max_minus_one() {
        for n in 3..=20 {
            let max = max_concurrent_upgrades(n);
            if max > 1 {
                assert!(can_upgrade_node(n, max - 1, n), "Should be able to upgrade at max-1 for {n}-node cluster");
            }
        }
    }

    // Invariant: can_upgrade_node(n, max, n) should be false (at max, can't add more)
    #[test]
    fn test_cannot_exceed_max_concurrent() {
        for n in 3..=20 {
            let max = max_concurrent_upgrades(n);
            assert!(!can_upgrade_node(n, max, n), "Should NOT be able to upgrade beyond max for {n}-node cluster");
        }
    }
}
