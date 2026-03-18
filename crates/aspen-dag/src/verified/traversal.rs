//! Pure traversal state computation functions.
//!
//! These functions make all traversal decisions. The imperative shell
//! (traversal.rs) calls them but never makes policy decisions itself.
//!
//! Formally verified — see `verus/traversal_spec.rs` for proofs.

/// Determine whether a node should be visited during traversal.
///
/// A node should be visited if and only if:
/// - It has not been visited before (`is_visited` is false)
/// - It is not in the known heads set (`is_known_head` is false)
///
/// Known heads represent boundaries where the receiver already has
/// all data below. Visited nodes should never be re-traversed in a DAG.
#[inline]
pub fn should_visit(is_visited: bool, is_known_head: bool) -> bool {
    !is_visited && !is_known_head
}

/// Check whether a depth value is within the allowed bound.
///
/// Returns true if `depth < max_depth`. Uses strict less-than because
/// depth 0 is the root, so `max_depth = 10_000` allows depths 0..9_999.
#[inline]
pub fn is_within_depth_bound(depth: u32, max_depth: u32) -> bool {
    depth < max_depth
}

/// Check whether a node count is within the allowed bound.
///
/// Returns true if `count < max_count`. The count is checked before
/// yielding, so `max_count = 100` means at most 100 nodes are yielded.
#[inline]
pub fn is_within_count_bound(count: u32, max_count: u32) -> bool {
    count < max_count
}

/// Compute the size of a visited set after inserting one element.
///
/// Uses saturating arithmetic to prevent overflow.
#[inline]
pub fn compute_visited_set_size(current_size: u32) -> u32 {
    current_size.saturating_add(1)
}

/// Compute child depth from parent depth.
///
/// Uses saturating arithmetic. If parent is at `u32::MAX`, children
/// stay at `u32::MAX` rather than wrapping.
#[inline]
pub fn compute_child_depth(parent_depth: u32) -> u32 {
    parent_depth.saturating_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_visit_unvisited_unknown() {
        assert!(should_visit(false, false));
    }

    #[test]
    fn test_should_visit_already_visited() {
        assert!(!should_visit(true, false));
    }

    #[test]
    fn test_should_visit_known_head() {
        assert!(!should_visit(false, true));
    }

    #[test]
    fn test_should_visit_both() {
        assert!(!should_visit(true, true));
    }

    #[test]
    fn test_depth_bound_within() {
        assert!(is_within_depth_bound(0, 10_000));
        assert!(is_within_depth_bound(9_999, 10_000));
    }

    #[test]
    fn test_depth_bound_at_limit() {
        assert!(!is_within_depth_bound(10_000, 10_000));
    }

    #[test]
    fn test_depth_bound_over() {
        assert!(!is_within_depth_bound(10_001, 10_000));
    }

    #[test]
    fn test_count_bound() {
        assert!(is_within_count_bound(0, 100));
        assert!(is_within_count_bound(99, 100));
        assert!(!is_within_count_bound(100, 100));
    }

    #[test]
    fn test_compute_visited_set_size() {
        assert_eq!(compute_visited_set_size(0), 1);
        assert_eq!(compute_visited_set_size(u32::MAX), u32::MAX);
    }

    #[test]
    fn test_compute_child_depth() {
        assert_eq!(compute_child_depth(0), 1);
        assert_eq!(compute_child_depth(u32::MAX), u32::MAX);
    }
}
