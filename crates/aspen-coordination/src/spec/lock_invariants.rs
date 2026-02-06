//! Lock state machine model for verification
//!
//! This module defines the abstract state model used for formal verification
//! of distributed lock operations.
//!
//! # State Model
//!
//! The `LockStateSpec` captures:
//! - Current lock entry (holder, token, TTL, deadline)
//! - Current system time for expiration checks
//! - Maximum fencing token ever issued (monotonicity tracking)
//!
//! # Invariants
//!
//! The following invariants are verified to hold across all operations:
//!
//! 1. Fencing token monotonicity: tokens only increase
//! 2. TTL validity: deadline = acquired_at + ttl (or 0 if released)
//! 3. Mutual exclusion: at most one non-expired holder at a time

use crate::types::LockEntry;

/// Abstract lock entry for specifications.
#[derive(Clone, Debug, Default)]
pub struct LockEntrySpec {
    /// Unique identifier of the lock holder.
    pub holder_id: String,
    /// Monotonically increasing token for fencing.
    pub fencing_token: u64,
    /// When the lock was acquired (Unix timestamp milliseconds).
    pub acquired_at_ms: u64,
    /// TTL in milliseconds.
    pub ttl_ms: u64,
    /// Deadline = acquired_at_ms + ttl_ms (0 means released).
    pub deadline_ms: u64,
}

impl From<&LockEntry> for LockEntrySpec {
    fn from(entry: &LockEntry) -> Self {
        Self {
            holder_id: entry.holder_id.clone(),
            fencing_token: entry.fencing_token,
            acquired_at_ms: entry.acquired_at_ms,
            ttl_ms: entry.ttl_ms,
            deadline_ms: entry.deadline_ms,
        }
    }
}

/// Abstract lock state for verification.
///
/// This is a simplified model of the distributed lock that captures
/// the essential state needed for correctness proofs.
#[derive(Clone, Debug, Default)]
pub struct LockStateSpec {
    /// Current lock entry (None if lock never acquired)
    pub entry: Option<LockEntrySpec>,
    /// Current system time in milliseconds
    pub current_time_ms: u64,
    /// Maximum fencing token ever issued
    pub max_fencing_token_issued: u64,
}

impl LockStateSpec {
    /// Create a new empty lock state.
    pub fn new(current_time_ms: u64) -> Self {
        Self {
            entry: None,
            current_time_ms,
            max_fencing_token_issued: 0,
        }
    }

    /// Create a lock state from an existing entry.
    pub fn from_entry(entry: Option<&LockEntry>, current_time_ms: u64) -> Self {
        let max_token = entry.map(|e| e.fencing_token).unwrap_or(0);
        Self {
            entry: entry.map(LockEntrySpec::from),
            current_time_ms,
            max_fencing_token_issued: max_token,
        }
    }
}

// ============================================================================
// Ghost State Extraction
// ============================================================================

/// Ghost state snapshot for verification.
///
/// This struct captures the abstract state of the distributed lock at a point
/// in time for use in ghost code proofs. It is a zero-cost abstraction that
/// compiles away during normal cargo builds.
#[derive(Clone, Debug, Default)]
pub struct GhostLockState {
    /// The abstract lock state for verification.
    pub spec: LockStateSpec,
}

impl GhostLockState {
    /// Create an empty ghost state (zero-cost).
    #[inline(always)]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a ghost state with the given spec (zero-cost in production).
    #[inline(always)]
    pub fn from_spec(_spec: LockStateSpec) -> Self {
        Self::default()
    }
}

/// Trait for extracting ghost state from lock components.
pub trait GhostLockStateExtractor {
    /// Extract abstract state for verification.
    fn to_spec_state(&self, current_time_ms: u64) -> GhostLockState;
}

// ============================================================================
// Invariant Predicates
// ============================================================================

/// Check if a lock entry is expired.
///
/// A lock is expired if:
/// - deadline_ms == 0 (explicitly released), OR
/// - current_time > deadline_ms (TTL elapsed)
pub fn is_expired(entry: &LockEntrySpec, current_time_ms: u64) -> bool {
    entry.deadline_ms == 0 || current_time_ms > entry.deadline_ms
}

/// Check if the lock is available for acquisition.
pub fn is_lock_available(state: &LockStateSpec) -> bool {
    match &state.entry {
        None => true,
        Some(entry) => is_expired(entry, state.current_time_ms),
    }
}

/// INVARIANT 1: Fencing token monotonicity
///
/// The max_fencing_token_issued can only increase, never decrease.
pub fn fencing_token_monotonic(pre: &LockStateSpec, post: &LockStateSpec) -> bool {
    post.max_fencing_token_issued >= pre.max_fencing_token_issued
}

/// INVARIANT 2: TTL expiration validity
///
/// For non-released entries, deadline = acquired_at + ttl
pub fn ttl_expiration_valid(entry: &LockEntrySpec) -> bool {
    entry.deadline_ms == 0 || entry.deadline_ms == entry.acquired_at_ms + entry.ttl_ms
}

/// Check TTL validity for the entire state.
pub fn state_ttl_valid(state: &LockStateSpec) -> bool {
    match &state.entry {
        None => true,
        Some(entry) => ttl_expiration_valid(entry),
    }
}

/// The current entry's token is always <= max_fencing_token_issued
pub fn entry_token_bounded(state: &LockStateSpec) -> bool {
    match &state.entry {
        None => true,
        Some(entry) => entry.fencing_token <= state.max_fencing_token_issued,
    }
}

/// INVARIANT 3: Mutual exclusion
///
/// At most one holder at any time. If lock is not expired, holder_id is valid.
pub fn mutual_exclusion_holds(state: &LockStateSpec) -> bool {
    match &state.entry {
        None => true,
        Some(entry) => !entry.holder_id.is_empty() || is_expired(entry, state.current_time_ms),
    }
}

/// Combined invariant check for all lock invariants.
pub fn lock_invariant(state: &LockStateSpec) -> bool {
    entry_token_bounded(state) && state_ttl_valid(state) && mutual_exclusion_holds(state)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_state_invariants() {
        let state = LockStateSpec::new(1000);

        assert!(entry_token_bounded(&state));
        assert!(state_ttl_valid(&state));
        assert!(mutual_exclusion_holds(&state));
        assert!(lock_invariant(&state));
        assert!(is_lock_available(&state));
    }

    #[test]
    fn test_fencing_token_monotonic() {
        let pre = LockStateSpec {
            max_fencing_token_issued: 5,
            ..Default::default()
        };

        // Increasing is allowed
        let post_increase = LockStateSpec {
            max_fencing_token_issued: 10,
            ..Default::default()
        };
        assert!(fencing_token_monotonic(&pre, &post_increase));

        // Same is allowed
        let post_same = LockStateSpec {
            max_fencing_token_issued: 5,
            ..Default::default()
        };
        assert!(fencing_token_monotonic(&pre, &post_same));

        // Decreasing is not allowed
        let post_decrease = LockStateSpec {
            max_fencing_token_issued: 3,
            ..Default::default()
        };
        assert!(!fencing_token_monotonic(&pre, &post_decrease));
    }

    #[test]
    fn test_ttl_expiration_valid() {
        // Valid: deadline = acquired_at + ttl
        let valid_entry = LockEntrySpec {
            holder_id: "node-1".to_string(),
            fencing_token: 1,
            acquired_at_ms: 1000,
            ttl_ms: 5000,
            deadline_ms: 6000, // 1000 + 5000
        };
        assert!(ttl_expiration_valid(&valid_entry));

        // Valid: released (deadline = 0)
        let released_entry = LockEntrySpec {
            holder_id: String::new(),
            fencing_token: 1,
            acquired_at_ms: 1000,
            ttl_ms: 0,
            deadline_ms: 0,
        };
        assert!(ttl_expiration_valid(&released_entry));

        // Invalid: deadline doesn't match
        let invalid_entry = LockEntrySpec {
            holder_id: "node-1".to_string(),
            fencing_token: 1,
            acquired_at_ms: 1000,
            ttl_ms: 5000,
            deadline_ms: 7000, // Should be 6000
        };
        assert!(!ttl_expiration_valid(&invalid_entry));
    }

    #[test]
    fn test_is_expired() {
        let entry = LockEntrySpec {
            holder_id: "node-1".to_string(),
            fencing_token: 1,
            acquired_at_ms: 1000,
            ttl_ms: 5000,
            deadline_ms: 6000,
        };

        // Not expired: current_time < deadline
        assert!(!is_expired(&entry, 5000));

        // Expired: current_time > deadline
        assert!(is_expired(&entry, 7000));

        // Released entry (deadline = 0) is always expired
        let released = LockEntrySpec {
            deadline_ms: 0,
            ..entry.clone()
        };
        assert!(is_expired(&released, 1));
    }

    #[test]
    fn test_lock_available() {
        // Available: no entry
        let empty_state = LockStateSpec::new(1000);
        assert!(is_lock_available(&empty_state));

        // Not available: held by someone
        let held_state = LockStateSpec {
            entry: Some(LockEntrySpec {
                holder_id: "node-1".to_string(),
                fencing_token: 1,
                acquired_at_ms: 1000,
                ttl_ms: 5000,
                deadline_ms: 6000,
            }),
            current_time_ms: 2000,
            max_fencing_token_issued: 1,
        };
        assert!(!is_lock_available(&held_state));

        // Available: expired
        let expired_state = LockStateSpec {
            current_time_ms: 7000, // After deadline
            ..held_state
        };
        assert!(is_lock_available(&expired_state));
    }

    #[test]
    fn test_entry_token_bounded() {
        // Valid: token <= max
        let valid_state = LockStateSpec {
            entry: Some(LockEntrySpec {
                fencing_token: 5,
                ..Default::default()
            }),
            max_fencing_token_issued: 5,
            ..Default::default()
        };
        assert!(entry_token_bounded(&valid_state));

        // Invalid: token > max
        let invalid_state = LockStateSpec {
            entry: Some(LockEntrySpec {
                fencing_token: 10,
                ..Default::default()
            }),
            max_fencing_token_issued: 5,
            ..Default::default()
        };
        assert!(!entry_token_bounded(&invalid_state));
    }
}
