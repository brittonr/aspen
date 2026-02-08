//! Pure functions for cluster membership and learner promotion.
//!
//! These functions encapsulate the core membership decision logic without
//! any async/I/O dependencies, enabling:
//!
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Deterministic behavior verification
//!
//! # Tiger Style
//!
//! - All calculations use explicit types (u64 for node IDs, usize for counts)
//! - Saturating arithmetic prevents overflow
//! - Bounded membership (max voters limit)
//! - No side effects or I/O

/// Calculate the replication lag for a learner node.
///
/// The lag is the number of log entries the learner is behind the leader.
/// Uses saturating subtraction to handle the case where the learner might
/// temporarily appear ahead (due to timing or metrics collection).
///
/// # Arguments
///
/// * `leader_last_log` - The leader's last log index
/// * `learner_matched` - The learner's matched log index
///
/// # Returns
///
/// Number of log entries the learner is behind (0 if caught up or ahead).
///
/// # Tiger Style
///
/// - Uses saturating_sub to prevent underflow
/// - Returns u64 for explicit type safety
///
/// # Example
///
/// ```rust
/// use aspen_raft::pure::compute_learner_lag;
///
/// assert_eq!(compute_learner_lag(100, 95), 5);
/// assert_eq!(compute_learner_lag(100, 100), 0);
/// assert_eq!(compute_learner_lag(100, 105), 0); // Learner ahead (edge case)
/// ```
#[inline]
pub const fn compute_learner_lag(leader_last_log: u64, learner_matched: u64) -> u64 {
    leader_last_log.saturating_sub(learner_matched)
}

/// Check if a learner's lag is within acceptable threshold.
///
/// # Arguments
///
/// * `lag` - The learner's replication lag (entries behind leader)
/// * `threshold` - Maximum acceptable lag
///
/// # Returns
///
/// `true` if the learner is caught up enough (lag <= threshold).
///
/// # Example
///
/// ```rust
/// use aspen_raft::pure::is_learner_caught_up;
///
/// assert!(is_learner_caught_up(50, 100));  // 50 <= 100
/// assert!(is_learner_caught_up(100, 100)); // 100 <= 100
/// assert!(!is_learner_caught_up(101, 100)); // 101 > 100
/// ```
#[inline]
pub const fn is_learner_caught_up(lag: u64, threshold: u64) -> bool {
    lag <= threshold
}

/// Calculate the quorum size for a cluster.
///
/// Quorum is the minimum number of nodes required for consensus,
/// calculated as (voters / 2) + 1 (majority).
///
/// # Arguments
///
/// * `voter_count` - Total number of voters in the cluster
///
/// # Returns
///
/// Minimum number of nodes required for quorum.
///
/// # Tiger Style
///
/// - Uses integer division (rounds down before adding 1)
/// - Works correctly for any cluster size
///
/// # Example
///
/// ```rust
/// use aspen_raft::pure::calculate_quorum_size;
///
/// assert_eq!(calculate_quorum_size(1), 1); // 1/2 + 1 = 1
/// assert_eq!(calculate_quorum_size(2), 2); // 2/2 + 1 = 2
/// assert_eq!(calculate_quorum_size(3), 2); // 3/2 + 1 = 2
/// assert_eq!(calculate_quorum_size(4), 3); // 4/2 + 1 = 3
/// assert_eq!(calculate_quorum_size(5), 3); // 5/2 + 1 = 3
/// ```
#[inline]
pub const fn calculate_quorum_size(voter_count: usize) -> usize {
    (voter_count / 2) + 1
}

/// Check if removing a node would maintain quorum.
///
/// When replacing a failed voter, we need to verify the cluster
/// will still have quorum after the removal.
///
/// # Arguments
///
/// * `current_voter_count` - Current number of voters
///
/// # Returns
///
/// `true` if removing one voter would still leave quorum.
///
/// # Tiger Style
///
/// - Pure function with explicit boolean return
/// - Quorum calculated from original cluster size
///
/// # Example
///
/// ```rust
/// use aspen_raft::pure::can_remove_voter_safely;
///
/// assert!(can_remove_voter_safely(5));  // 4 remaining >= quorum(5)=3
/// assert!(can_remove_voter_safely(4));  // 3 remaining >= quorum(4)=3
/// assert!(can_remove_voter_safely(3));  // 2 remaining >= quorum(3)=2
/// assert!(!can_remove_voter_safely(2)); // 1 remaining < quorum(2)=2
/// assert!(!can_remove_voter_safely(1)); // 0 remaining < quorum(1)=1
/// ```
#[inline]
pub const fn can_remove_voter_safely(current_voter_count: usize) -> bool {
    if current_voter_count == 0 {
        return false;
    }
    let remaining = current_voter_count - 1;
    let quorum = calculate_quorum_size(current_voter_count);
    remaining >= quorum
}

/// Check if adding a voter would exceed the maximum limit.
///
/// # Arguments
///
/// * `current_voter_count` - Current number of voters
/// * `max_voters` - Maximum allowed voters
///
/// # Returns
///
/// `true` if adding one more voter would exceed the limit.
///
/// # Example
///
/// ```rust
/// use aspen_raft::pure::would_exceed_max_voters;
///
/// assert!(!would_exceed_max_voters(99, 100));  // 100 <= 100
/// assert!(!would_exceed_max_voters(100, 100)); // Edge: 101 > 100, but we check after add
/// assert!(would_exceed_max_voters(100, 100));  // Actually: 100 >= 100, so adding would make 101
/// ```
#[inline]
pub const fn would_exceed_max_voters(current_voter_count: usize, max_voters: u32) -> bool {
    current_voter_count >= max_voters as usize
}

/// Build a new membership set by promoting a learner and optionally replacing a voter.
///
/// This function handles the pure logic of membership set construction:
/// 1. Optionally removes a voter being replaced
/// 2. Adds the promoted learner if not already a voter
/// 3. Sorts for deterministic ordering
///
/// # Arguments
///
/// * `current_voters` - Current set of voter node IDs
/// * `promote_node` - Node ID of learner to promote
/// * `replace_node` - Optional node ID of voter to remove
/// * `max_voters` - Maximum allowed voters
///
/// # Returns
///
/// `Ok(Vec<u64>)` with the new sorted voter set, or `Err(MembershipError)` if limits exceeded.
///
/// # Tiger Style
///
/// - Bounded output (max_voters limit)
/// - Deterministic ordering (sorted)
/// - Explicit error type
///
/// # Example
///
/// ```rust
/// use aspen_raft::pure::build_new_membership;
///
/// // Simple promotion
/// let new = build_new_membership(&[1, 2, 3], 4, None, 100).unwrap();
/// assert_eq!(new, vec![1, 2, 3, 4]);
///
/// // Promotion with replacement
/// let new = build_new_membership(&[1, 2, 3], 4, Some(2), 100).unwrap();
/// assert_eq!(new, vec![1, 3, 4]);
/// ```
pub fn build_new_membership(
    current_voters: &[u64],
    promote_node: u64,
    replace_node: Option<u64>,
    max_voters: u32,
) -> Result<Vec<u64>, MembershipError> {
    let mut new_members: Vec<u64> = current_voters
        .iter()
        .filter(|&&id| Some(id) != replace_node)
        .copied()
        .collect();

    // Add promoted learner if not already in voter set
    if !new_members.contains(&promote_node) {
        new_members.push(promote_node);
    }

    // Enforce maximum voters limit
    if new_members.len() > max_voters as usize {
        return Err(MembershipError::MaxVotersExceeded {
            current: new_members.len(),
            max: max_voters,
        });
    }

    // Deterministic ordering for reproducibility
    new_members.sort();

    Ok(new_members)
}

/// Error type for membership operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MembershipError {
    /// Adding a voter would exceed the maximum allowed.
    MaxVotersExceeded {
        /// Current count after proposed change.
        current: usize,
        /// Maximum allowed voters.
        max: u32,
    },
}

impl std::fmt::Display for MembershipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MembershipError::MaxVotersExceeded { current, max } => {
                write!(f, "maximum voters ({}) would be exceeded (have {})", max, current)
            }
        }
    }
}

impl std::error::Error for MembershipError {}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // compute_learner_lag tests
    // ========================================================================

    #[test]
    fn test_lag_normal_case() {
        assert_eq!(compute_learner_lag(100, 95), 5);
        assert_eq!(compute_learner_lag(1000, 900), 100);
    }

    #[test]
    fn test_lag_caught_up() {
        assert_eq!(compute_learner_lag(100, 100), 0);
    }

    #[test]
    fn test_lag_learner_ahead() {
        // Edge case: learner might appear ahead due to timing
        assert_eq!(compute_learner_lag(100, 105), 0);
    }

    #[test]
    fn test_lag_zero_indices() {
        assert_eq!(compute_learner_lag(0, 0), 0);
    }

    #[test]
    fn test_lag_large_values() {
        assert_eq!(compute_learner_lag(u64::MAX, u64::MAX - 100), 100);
        assert_eq!(compute_learner_lag(u64::MAX, 0), u64::MAX);
    }

    // ========================================================================
    // is_learner_caught_up tests
    // ========================================================================

    #[test]
    fn test_caught_up_within_threshold() {
        assert!(is_learner_caught_up(0, 100));
        assert!(is_learner_caught_up(50, 100));
        assert!(is_learner_caught_up(99, 100));
    }

    #[test]
    fn test_caught_up_at_threshold() {
        assert!(is_learner_caught_up(100, 100));
    }

    #[test]
    fn test_not_caught_up() {
        assert!(!is_learner_caught_up(101, 100));
        assert!(!is_learner_caught_up(1000, 100));
    }

    // ========================================================================
    // calculate_quorum_size tests
    // ========================================================================

    #[test]
    fn test_quorum_odd_clusters() {
        assert_eq!(calculate_quorum_size(1), 1);
        assert_eq!(calculate_quorum_size(3), 2);
        assert_eq!(calculate_quorum_size(5), 3);
        assert_eq!(calculate_quorum_size(7), 4);
    }

    #[test]
    fn test_quorum_even_clusters() {
        assert_eq!(calculate_quorum_size(2), 2);
        assert_eq!(calculate_quorum_size(4), 3);
        assert_eq!(calculate_quorum_size(6), 4);
    }

    #[test]
    fn test_quorum_zero() {
        assert_eq!(calculate_quorum_size(0), 1);
    }

    // ========================================================================
    // can_remove_voter_safely tests
    // ========================================================================

    #[test]
    fn test_remove_from_large_cluster() {
        assert!(can_remove_voter_safely(5)); // 4 >= 3
        assert!(can_remove_voter_safely(7)); // 6 >= 4
    }

    #[test]
    fn test_remove_from_three_node_cluster() {
        assert!(can_remove_voter_safely(3)); // 2 >= 2
    }

    #[test]
    fn test_cannot_remove_from_two_node_cluster() {
        assert!(!can_remove_voter_safely(2)); // 1 < 2
    }

    #[test]
    fn test_cannot_remove_from_single_node() {
        assert!(!can_remove_voter_safely(1)); // 0 < 1
    }

    #[test]
    fn test_cannot_remove_from_empty() {
        assert!(!can_remove_voter_safely(0));
    }

    // ========================================================================
    // would_exceed_max_voters tests
    // ========================================================================

    #[test]
    fn test_below_max() {
        assert!(!would_exceed_max_voters(99, 100));
        assert!(!would_exceed_max_voters(0, 100));
    }

    #[test]
    fn test_at_max() {
        assert!(would_exceed_max_voters(100, 100));
    }

    #[test]
    fn test_above_max() {
        assert!(would_exceed_max_voters(101, 100));
    }

    // ========================================================================
    // build_new_membership tests
    // ========================================================================

    #[test]
    fn test_simple_promotion() {
        let new = build_new_membership(&[1, 2, 3], 4, None, 100).unwrap();
        assert_eq!(new, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_promotion_with_replacement() {
        let new = build_new_membership(&[1, 2, 3], 4, Some(2), 100).unwrap();
        assert_eq!(new, vec![1, 3, 4]);
    }

    #[test]
    fn test_promotion_no_duplicate() {
        // Learner already in voter set (edge case)
        let new = build_new_membership(&[1, 2, 3, 4], 4, None, 100).unwrap();
        assert_eq!(new, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_promotion_sorts_result() {
        let new = build_new_membership(&[3, 1, 2], 4, None, 100).unwrap();
        assert_eq!(new, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_max_voters_exceeded() {
        let voters: Vec<u64> = (1..=100).collect();
        let result = build_new_membership(&voters, 999, None, 100);
        assert!(matches!(result, Err(MembershipError::MaxVotersExceeded { .. })));
    }

    #[test]
    fn test_replacement_at_max_voters() {
        // At max voters, but replacing one should succeed
        let voters: Vec<u64> = (1..=100).collect();
        let result = build_new_membership(&voters, 999, Some(50), 100);
        assert!(result.is_ok());
        let new = result.unwrap();
        assert_eq!(new.len(), 100);
        assert!(new.contains(&999));
        assert!(!new.contains(&50));
    }
}
