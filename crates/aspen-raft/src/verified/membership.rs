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
/// use aspen_raft::verified::compute_learner_lag;
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
/// use aspen_raft::verified::is_learner_caught_up;
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
/// use aspen_raft::verified::calculate_quorum_size;
///
/// assert_eq!(calculate_quorum_size(1), 1); // 1/2 + 1 = 1
/// assert_eq!(calculate_quorum_size(2), 2); // 2/2 + 1 = 2
/// assert_eq!(calculate_quorum_size(3), 2); // 3/2 + 1 = 2
/// assert_eq!(calculate_quorum_size(4), 3); // 4/2 + 1 = 3
/// assert_eq!(calculate_quorum_size(5), 3); // 5/2 + 1 = 3
/// ```
#[inline]
pub const fn calculate_quorum_size(voter_count: u32) -> u32 {
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
/// use aspen_raft::verified::can_remove_voter_safely;
///
/// assert!(can_remove_voter_safely(5));  // 4 remaining >= quorum(5)=3
/// assert!(can_remove_voter_safely(4));  // 3 remaining >= quorum(4)=3
/// assert!(can_remove_voter_safely(3));  // 2 remaining >= quorum(3)=2
/// assert!(!can_remove_voter_safely(2)); // 1 remaining < quorum(2)=2
/// assert!(!can_remove_voter_safely(1)); // 0 remaining < quorum(1)=1
/// ```
#[inline]
pub const fn can_remove_voter_safely(current_voter_count: u32) -> bool {
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
/// use aspen_raft::verified::would_exceed_max_voters;
///
/// assert!(!would_exceed_max_voters(99, 100));  // 100 <= 100
/// assert!(!would_exceed_max_voters(100, 100)); // Edge: 101 > 100, but we check after add
/// assert!(would_exceed_max_voters(100, 100));  // Actually: 100 >= 100, so adding would make 101
/// ```
#[inline]
pub const fn would_exceed_max_voters(current_voter_count: u32, max_voters: u32) -> bool {
    current_voter_count >= max_voters
}

/// Check if a cluster has quorum with given healthy node count.
///
/// # Arguments
///
/// * `total_voters` - Total number of voters in the cluster
/// * `healthy_count` - Number of healthy/responsive voters
///
/// # Returns
///
/// `true` if the healthy count meets quorum requirements.
///
/// # Example
///
/// ```rust
/// use aspen_raft::verified::has_quorum;
///
/// assert!(has_quorum(5, 3));  // 3 >= (5/2 + 1) = 3
/// assert!(!has_quorum(5, 2)); // 2 < 3
/// ```
#[inline]
#[allow(dead_code)]
pub const fn has_quorum(total_voters: u32, healthy_count: u32) -> bool {
    healthy_count >= calculate_quorum_size(total_voters)
}

/// Check if a learner is eligible for promotion.
///
/// Combines lag check with additional health/stability criteria.
///
/// # Arguments
///
/// * `leader_last_log` - The leader's last log index
/// * `learner_matched` - The learner's matched log index
/// * `threshold` - Maximum acceptable lag for promotion
/// * `is_healthy` - Whether the learner is currently healthy
///
/// # Returns
///
/// `true` if the learner is eligible for promotion.
///
/// # Example
///
/// ```rust
/// use aspen_raft::verified::is_promotion_eligible;
///
/// assert!(is_promotion_eligible(100, 95, 10, true));  // lag=5 <= 10, healthy
/// assert!(!is_promotion_eligible(100, 95, 10, false)); // not healthy
/// assert!(!is_promotion_eligible(100, 80, 10, true));  // lag=20 > 10
/// ```
#[inline]
#[allow(dead_code)]
pub const fn is_promotion_eligible(
    leader_last_log: u64,
    learner_matched: u64,
    threshold: u64,
    is_healthy: bool,
) -> bool {
    is_healthy && compute_learner_lag(leader_last_log, learner_matched) <= threshold
}

/// Calculate required lag threshold for learner promotion.
///
/// # Arguments
///
/// * `base_threshold` - Base threshold in log entries
/// * `safety_margin` - Additional safety margin
///
/// # Returns
///
/// Effective threshold for learner promotion eligibility.
///
/// # Example
///
/// ```rust
/// use aspen_raft::verified::calculate_promotion_threshold;
///
/// assert_eq!(calculate_promotion_threshold(100, 50), 150);
/// assert_eq!(calculate_promotion_threshold(u64::MAX, 1), u64::MAX); // Saturates
/// ```
#[inline]
#[allow(dead_code)]
pub const fn calculate_promotion_threshold(base_threshold: u64, safety_margin: u64) -> u64 {
    base_threshold.saturating_add(safety_margin)
}

/// Calculate minimum cluster size for fault tolerance.
///
/// Returns the minimum number of voters needed to tolerate `f` failures.
/// Formula: 2f + 1 (to maintain majority after f failures)
///
/// # Arguments
///
/// * `fault_tolerance` - Number of failures to tolerate
///
/// # Returns
///
/// Minimum cluster size, saturating at u32::MAX.
///
/// # Example
///
/// ```rust
/// use aspen_raft::verified::min_cluster_size_for_tolerance;
///
/// assert_eq!(min_cluster_size_for_tolerance(0), 1); // 2*0 + 1
/// assert_eq!(min_cluster_size_for_tolerance(1), 3); // 2*1 + 1
/// assert_eq!(min_cluster_size_for_tolerance(2), 5); // 2*2 + 1
/// ```
#[inline]
#[allow(dead_code)]
pub fn min_cluster_size_for_tolerance(fault_tolerance: u32) -> u32 {
    let doubled = if fault_tolerance > u32::MAX / 2 {
        u32::MAX
    } else {
        fault_tolerance * 2
    };
    if doubled > u32::MAX - 1 { u32::MAX } else { doubled + 1 }
}

/// Calculate fault tolerance from cluster size.
///
/// Returns the number of failures that can be tolerated with given cluster size.
/// Formula: (voters - 1) / 2
///
/// # Arguments
///
/// * `voter_count` - Number of voters in the cluster
///
/// # Returns
///
/// Number of failures that can be tolerated.
///
/// # Example
///
/// ```rust
/// use aspen_raft::verified::fault_tolerance_for_size;
///
/// assert_eq!(fault_tolerance_for_size(1), 0); // (1-1)/2 = 0
/// assert_eq!(fault_tolerance_for_size(3), 1); // (3-1)/2 = 1
/// assert_eq!(fault_tolerance_for_size(5), 2); // (5-1)/2 = 2
/// assert_eq!(fault_tolerance_for_size(7), 3); // (7-1)/2 = 3
/// ```
#[inline]
#[allow(dead_code)]
pub const fn fault_tolerance_for_size(voter_count: u32) -> u32 {
    if voter_count == 0 { 0 } else { (voter_count - 1) / 2 }
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
/// use aspen_raft::verified::build_new_membership;
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
    let mut new_members: Vec<u64> = current_voters.iter().filter(|&&id| Some(id) != replace_node).copied().collect();

    // Add promoted learner if not already in voter set
    if !new_members.contains(&promote_node) {
        new_members.push(promote_node);
    }

    // Enforce maximum voters limit
    if new_members.len() as u32 > max_voters {
        return Err(MembershipError::MaxVotersExceeded {
            current: new_members.len() as u32,
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
        current: u32,
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

// ============================================================================
// Cluster State Building
// ============================================================================

/// Classification of nodes by their membership role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedNodes<T> {
    /// Nodes that are voters (full participants in consensus).
    pub voters: Vec<T>,
    /// Node IDs of voters (for quick lookup).
    pub voter_ids: Vec<u64>,
    /// Nodes that are learners (replicating but not voting).
    pub learners: Vec<T>,
}

impl<T> Default for ClassifiedNodes<T> {
    fn default() -> Self {
        Self {
            voters: Vec::new(),
            voter_ids: Vec::new(),
            learners: Vec::new(),
        }
    }
}

/// Classify nodes into voters and learners based on voter ID set.
///
/// This pure function takes a list of nodes and a set of voter IDs,
/// and separates them into voters and learners.
///
/// # Arguments
///
/// * `nodes` - Iterator of (node_id, node_data) pairs
/// * `is_voter` - Function to check if a node ID is a voter
///
/// # Returns
///
/// A [`ClassifiedNodes`] struct with separated voters and learners.
///
/// # Example
///
/// ```rust
/// use aspen_raft::verified::membership::classify_nodes_by_role;
/// use std::collections::HashSet;
///
/// let voters: HashSet<u64> = [1, 2, 3].into_iter().collect();
/// let nodes = vec![
///     (1, "node1"),
///     (2, "node2"),
///     (4, "learner"),
/// ];
///
/// let classified = classify_nodes_by_role(nodes, |id| voters.contains(&id));
///
/// assert_eq!(classified.voters.len(), 2);
/// assert_eq!(classified.learners.len(), 1);
/// assert_eq!(classified.voter_ids, vec![1, 2]);
/// ```
pub fn classify_nodes_by_role<T, I, F>(nodes: I, is_voter: F) -> ClassifiedNodes<T>
where
    I: IntoIterator<Item = (u64, T)>,
    F: Fn(u64) -> bool,
{
    let mut result = ClassifiedNodes::default();

    for (node_id, node_data) in nodes {
        if is_voter(node_id) {
            result.voter_ids.push(node_id);
            result.voters.push(node_data);
        } else {
            result.learners.push(node_data);
        }
    }

    result
}

/// Extract voter IDs from a membership configuration.
///
/// Creates a HashSet of voter IDs for efficient lookups.
/// This is useful when you need to check multiple nodes against
/// the voter set.
///
/// # Arguments
///
/// * `voter_ids` - Iterator of voter node IDs
///
/// # Returns
///
/// A HashSet containing all voter IDs.
///
/// # Example
///
/// ```rust
/// use aspen_raft::verified::membership::collect_voter_ids;
///
/// let voters = vec![1u64, 2, 3];
/// let voter_set = collect_voter_ids(voters);
///
/// assert!(voter_set.contains(&1));
/// assert!(voter_set.contains(&2));
/// assert!(!voter_set.contains(&4));
/// ```
#[inline]
pub fn collect_voter_ids<I>(voter_ids: I) -> std::collections::HashSet<u64>
where I: IntoIterator<Item = u64> {
    voter_ids.into_iter().collect()
}

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

    // ========================================================================
    // classify_nodes_by_role tests
    // ========================================================================

    #[test]
    fn test_classify_all_voters() {
        use std::collections::HashSet;

        let voters: HashSet<u64> = [1, 2, 3].into_iter().collect();
        let nodes = vec![(1, "a"), (2, "b"), (3, "c")];

        let classified = classify_nodes_by_role(nodes, |id| voters.contains(&id));

        assert_eq!(classified.voters.len(), 3);
        assert!(classified.learners.is_empty());
        assert_eq!(classified.voter_ids, vec![1, 2, 3]);
    }

    #[test]
    fn test_classify_all_learners() {
        use std::collections::HashSet;

        let voters: HashSet<u64> = HashSet::new();
        let nodes = vec![(1, "a"), (2, "b")];

        let classified = classify_nodes_by_role(nodes, |id| voters.contains(&id));

        assert!(classified.voters.is_empty());
        assert_eq!(classified.learners.len(), 2);
        assert!(classified.voter_ids.is_empty());
    }

    #[test]
    fn test_classify_mixed() {
        use std::collections::HashSet;

        let voters: HashSet<u64> = [1, 3].into_iter().collect();
        let nodes = vec![(1, "voter1"), (2, "learner1"), (3, "voter2"), (4, "learner2")];

        let classified = classify_nodes_by_role(nodes, |id| voters.contains(&id));

        assert_eq!(classified.voters, vec!["voter1", "voter2"]);
        assert_eq!(classified.learners, vec!["learner1", "learner2"]);
        assert_eq!(classified.voter_ids, vec![1, 3]);
    }

    #[test]
    fn test_classify_empty() {
        use std::collections::HashSet;

        let voters: HashSet<u64> = HashSet::new();
        let nodes: Vec<(u64, &str)> = vec![];

        let classified = classify_nodes_by_role(nodes, |id| voters.contains(&id));

        assert!(classified.voters.is_empty());
        assert!(classified.learners.is_empty());
        assert!(classified.voter_ids.is_empty());
    }

    // ========================================================================
    // collect_voter_ids tests
    // ========================================================================

    #[test]
    fn test_collect_voter_ids() {
        let ids = vec![1u64, 2, 3];
        let set = collect_voter_ids(ids);

        assert!(set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));
        assert!(!set.contains(&4));
    }

    #[test]
    fn test_collect_voter_ids_empty() {
        let ids: Vec<u64> = vec![];
        let set = collect_voter_ids(ids);

        assert!(set.is_empty());
    }

    #[test]
    fn test_collect_voter_ids_duplicates() {
        let ids = vec![1u64, 1, 2, 2, 3];
        let set = collect_voter_ids(ids);

        assert_eq!(set.len(), 3);
    }
}
