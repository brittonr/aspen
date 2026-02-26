//! Pure collaborative object (COB) state transition functions.
//!
//! These functions determine valid state transitions for issues and patches
//! without side effects. The async shell layer handles persistence and
//! event emission.
//!
//! # Tiger Style
//!
//! - Pure validation functions
//! - Explicit state transition rules
//! - No I/O or system calls

/// Issue state for transition validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueStateKind {
    /// Issue is open.
    Open,
    /// Issue is closed.
    Closed,
}

/// Patch state for transition validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchStateKind {
    /// Patch is open and awaiting review.
    Open,
    /// Patch has been merged.
    Merged,
    /// Patch has been closed without merging.
    Closed,
}

/// Check if an issue can be closed.
///
/// An issue can be closed if it is currently open.
///
/// # Arguments
///
/// * `state` - Current issue state
///
/// # Returns
///
/// `true` if the issue can be closed.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::{IssueStateKind, can_close_issue};
///
/// assert!(can_close_issue(IssueStateKind::Open));
/// assert!(!can_close_issue(IssueStateKind::Closed));
/// ```
#[inline]
pub const fn can_close_issue(state: IssueStateKind) -> bool {
    matches!(state, IssueStateKind::Open)
}

/// Check if an issue can be reopened.
///
/// An issue can be reopened if it is currently closed.
///
/// # Arguments
///
/// * `state` - Current issue state
///
/// # Returns
///
/// `true` if the issue can be reopened.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::{IssueStateKind, can_reopen_issue};
///
/// assert!(can_reopen_issue(IssueStateKind::Closed));
/// assert!(!can_reopen_issue(IssueStateKind::Open));
/// ```
#[inline]
pub const fn can_reopen_issue(state: IssueStateKind) -> bool {
    matches!(state, IssueStateKind::Closed)
}

/// Check if a patch can be merged.
///
/// A patch can be merged if it is currently open.
///
/// # Arguments
///
/// * `state` - Current patch state
///
/// # Returns
///
/// `true` if the patch can be merged.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::{PatchStateKind, can_merge_patch};
///
/// assert!(can_merge_patch(PatchStateKind::Open));
/// assert!(!can_merge_patch(PatchStateKind::Merged));
/// assert!(!can_merge_patch(PatchStateKind::Closed));
/// ```
#[inline]
pub const fn can_merge_patch(state: PatchStateKind) -> bool {
    matches!(state, PatchStateKind::Open)
}

/// Check if a patch can be closed.
///
/// A patch can be closed if it is currently open (not merged).
///
/// # Arguments
///
/// * `state` - Current patch state
///
/// # Returns
///
/// `true` if the patch can be closed.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::{PatchStateKind, can_close_patch};
///
/// assert!(can_close_patch(PatchStateKind::Open));
/// assert!(!can_close_patch(PatchStateKind::Merged));
/// assert!(!can_close_patch(PatchStateKind::Closed));
/// ```
#[inline]
pub const fn can_close_patch(state: PatchStateKind) -> bool {
    matches!(state, PatchStateKind::Open)
}

/// Check if a patch can be reopened.
///
/// A patch can be reopened if it is closed but not merged.
/// Merged patches cannot be reopened.
///
/// # Arguments
///
/// * `state` - Current patch state
///
/// # Returns
///
/// `true` if the patch can be reopened.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::{PatchStateKind, can_reopen_patch};
///
/// assert!(can_reopen_patch(PatchStateKind::Closed));
/// assert!(!can_reopen_patch(PatchStateKind::Open));
/// assert!(!can_reopen_patch(PatchStateKind::Merged));
/// ```
#[inline]
pub const fn can_reopen_patch(state: PatchStateKind) -> bool {
    matches!(state, PatchStateKind::Closed)
}

/// Check if a patch is in a terminal state.
///
/// A patch is terminal if it has been merged. Closed patches can be
/// reopened, but merged patches cannot.
///
/// # Arguments
///
/// * `state` - Current patch state
///
/// # Returns
///
/// `true` if the patch is in a terminal state.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::{PatchStateKind, is_patch_terminal};
///
/// assert!(is_patch_terminal(PatchStateKind::Merged));
/// assert!(!is_patch_terminal(PatchStateKind::Open));
/// assert!(!is_patch_terminal(PatchStateKind::Closed));
/// ```
#[inline]
pub const fn is_patch_terminal(state: PatchStateKind) -> bool {
    matches!(state, PatchStateKind::Merged)
}

/// Last-Write-Wins (LWW) conflict resolution.
///
/// Determines the winner between two concurrent updates based on timestamps.
/// Uses author key as tiebreaker for identical timestamps.
///
/// # Arguments
///
/// * `timestamp_a_ms` - Timestamp of first update (Unix ms)
/// * `author_a` - Author key of first update (for tiebreaking)
/// * `timestamp_b_ms` - Timestamp of second update (Unix ms)
/// * `author_b` - Author key of second update (for tiebreaking)
///
/// # Returns
///
/// `true` if update A wins, `false` if update B wins.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::lww_winner;
///
/// // Later timestamp wins
/// assert!(lww_winner(2000, &[1; 32], 1000, &[2; 32]));
/// assert!(!lww_winner(1000, &[1; 32], 2000, &[2; 32]));
///
/// // Same timestamp: higher author key wins (deterministic tiebreaker)
/// assert!(lww_winner(1000, &[2; 32], 1000, &[1; 32]));
/// assert!(!lww_winner(1000, &[1; 32], 1000, &[2; 32]));
/// ```
#[inline]
pub fn lww_winner(timestamp_a_ms: u64, author_a: &[u8; 32], timestamp_b_ms: u64, author_b: &[u8; 32]) -> bool {
    if timestamp_a_ms != timestamp_b_ms {
        timestamp_a_ms > timestamp_b_ms
    } else {
        // Tiebreaker: compare author keys lexicographically
        author_a > author_b
    }
}

/// Compute updated timestamp for COB activity.
///
/// Returns the maximum of the current updated_at and the new activity timestamp.
///
/// # Arguments
///
/// * `current_updated_at_ms` - Current updated_at timestamp
/// * `new_activity_ms` - Timestamp of new activity
///
/// # Returns
///
/// The new updated_at timestamp.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::compute_updated_at;
///
/// assert_eq!(compute_updated_at(1000, 2000), 2000);
/// assert_eq!(compute_updated_at(2000, 1000), 2000);
/// assert_eq!(compute_updated_at(1000, 1000), 1000);
/// ```
#[inline]
pub const fn compute_updated_at(current_updated_at_ms: u64, new_activity_ms: u64) -> u64 {
    if new_activity_ms > current_updated_at_ms {
        new_activity_ms
    } else {
        current_updated_at_ms
    }
}

/// Check if a COB operation is allowed for the given author.
///
/// Only the author can edit their own comments/reactions.
/// Delegates can close/reopen/merge.
///
/// # Arguments
///
/// * `operation_author` - Author attempting the operation
/// * `target_author` - Author of the target (comment, reaction, etc.)
///
/// # Returns
///
/// `true` if the operation author matches the target author.
///
/// # Example
///
/// ```
/// use aspen_forge::verified::is_same_author;
///
/// let author = [1u8; 32];
/// let other = [2u8; 32];
///
/// assert!(is_same_author(&author, &author));
/// assert!(!is_same_author(&author, &other));
/// ```
#[inline]
pub fn is_same_author(operation_author: &[u8; 32], target_author: &[u8; 32]) -> bool {
    operation_author == target_author
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Issue state transition tests
    // ========================================================================

    #[test]
    fn test_can_close_open_issue() {
        assert!(can_close_issue(IssueStateKind::Open));
    }

    #[test]
    fn test_cannot_close_closed_issue() {
        assert!(!can_close_issue(IssueStateKind::Closed));
    }

    #[test]
    fn test_can_reopen_closed_issue() {
        assert!(can_reopen_issue(IssueStateKind::Closed));
    }

    #[test]
    fn test_cannot_reopen_open_issue() {
        assert!(!can_reopen_issue(IssueStateKind::Open));
    }

    // ========================================================================
    // Patch state transition tests
    // ========================================================================

    #[test]
    fn test_can_merge_open_patch() {
        assert!(can_merge_patch(PatchStateKind::Open));
    }

    #[test]
    fn test_cannot_merge_closed_patch() {
        assert!(!can_merge_patch(PatchStateKind::Closed));
    }

    #[test]
    fn test_cannot_merge_merged_patch() {
        assert!(!can_merge_patch(PatchStateKind::Merged));
    }

    #[test]
    fn test_can_close_open_patch() {
        assert!(can_close_patch(PatchStateKind::Open));
    }

    #[test]
    fn test_cannot_close_merged_patch() {
        assert!(!can_close_patch(PatchStateKind::Merged));
    }

    #[test]
    fn test_can_reopen_closed_patch() {
        assert!(can_reopen_patch(PatchStateKind::Closed));
    }

    #[test]
    fn test_cannot_reopen_merged_patch() {
        assert!(!can_reopen_patch(PatchStateKind::Merged));
    }

    #[test]
    fn test_merged_is_terminal() {
        assert!(is_patch_terminal(PatchStateKind::Merged));
    }

    #[test]
    fn test_open_not_terminal() {
        assert!(!is_patch_terminal(PatchStateKind::Open));
    }

    #[test]
    fn test_closed_not_terminal() {
        assert!(!is_patch_terminal(PatchStateKind::Closed));
    }

    // ========================================================================
    // LWW tests
    // ========================================================================

    #[test]
    fn test_lww_later_wins() {
        assert!(lww_winner(2000, &[1; 32], 1000, &[2; 32]));
        assert!(!lww_winner(1000, &[1; 32], 2000, &[2; 32]));
    }

    #[test]
    fn test_lww_tiebreaker() {
        // Higher author key wins on tie
        assert!(lww_winner(1000, &[2; 32], 1000, &[1; 32]));
        assert!(!lww_winner(1000, &[1; 32], 1000, &[2; 32]));
    }

    #[test]
    fn test_lww_identical() {
        // Same timestamp and author: first wins (a > b is false when equal)
        assert!(!lww_winner(1000, &[1; 32], 1000, &[1; 32]));
    }

    // ========================================================================
    // compute_updated_at tests
    // ========================================================================

    #[test]
    fn test_updated_at_newer() {
        assert_eq!(compute_updated_at(1000, 2000), 2000);
    }

    #[test]
    fn test_updated_at_older() {
        assert_eq!(compute_updated_at(2000, 1000), 2000);
    }

    #[test]
    fn test_updated_at_same() {
        assert_eq!(compute_updated_at(1000, 1000), 1000);
    }

    // ========================================================================
    // is_same_author tests
    // ========================================================================

    #[test]
    fn test_same_author() {
        let author = [1u8; 32];
        assert!(is_same_author(&author, &author));
    }

    #[test]
    fn test_different_author() {
        let author1 = [1u8; 32];
        let author2 = [2u8; 32];
        assert!(!is_same_author(&author1, &author2));
    }
}
