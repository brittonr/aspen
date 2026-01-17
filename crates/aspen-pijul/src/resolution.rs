//! Conflict resolution module for Pijul.
//!
//! Provides types and utilities for resolving conflicts in Pijul repositories.
//! Supports both manual resolution (via working directory edits) and
//! automated resolution strategies.
//!
//! # Resolution Strategies
//!
//! - **Ours**: Keep our version, discard theirs
//! - **Theirs**: Keep their version, discard ours
//! - **Newest**: Keep the most recently authored change
//! - **Oldest**: Keep the oldest change (first committed)
//! - **Manual**: User manually resolves in working directory
//!
//! # Example
//!
//! ```ignore
//! use aspen_pijul::resolution::{ResolutionStrategy, ResolutionAction};
//!
//! // Automatic resolution using "theirs" strategy
//! let action = ResolutionAction {
//!     path: "config.toml".to_string(),
//!     strategy: ResolutionStrategy::Theirs,
//!     keep_change: Some(their_change_hash),
//!     discard_changes: vec![our_change_hash],
//! };
//! ```

use serde::Deserialize;
use serde::Serialize;

use crate::types::ChangeHash;
use crate::types::ConflictKind;
use crate::types::DetailedFileConflict;
use crate::types::ResolutionStatus;

// ============================================================================
// Resolution Strategy
// ============================================================================

/// Strategy for automatically resolving conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionStrategy {
    /// Keep our version, discard theirs.
    Ours,
    /// Keep their version, discard ours.
    Theirs,
    /// Keep the most recently authored change.
    Newest,
    /// Keep the oldest change.
    Oldest,
    /// User will manually resolve in the working directory.
    Manual,
}

impl std::fmt::Display for ResolutionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolutionStrategy::Ours => write!(f, "ours"),
            ResolutionStrategy::Theirs => write!(f, "theirs"),
            ResolutionStrategy::Newest => write!(f, "newest"),
            ResolutionStrategy::Oldest => write!(f, "oldest"),
            ResolutionStrategy::Manual => write!(f, "manual"),
        }
    }
}

impl std::str::FromStr for ResolutionStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ours" => Ok(ResolutionStrategy::Ours),
            "theirs" => Ok(ResolutionStrategy::Theirs),
            "newest" => Ok(ResolutionStrategy::Newest),
            "oldest" => Ok(ResolutionStrategy::Oldest),
            "manual" => Ok(ResolutionStrategy::Manual),
            _ => Err(format!("unknown resolution strategy: {}", s)),
        }
    }
}

// ============================================================================
// Resolution Action
// ============================================================================

/// A specific action to resolve a conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionAction {
    /// Path to the conflicting file.
    pub path: String,

    /// Strategy used for this resolution.
    pub strategy: ResolutionStrategy,

    /// Change to keep (if applicable).
    pub keep_change: Option<ChangeHash>,

    /// Changes to discard.
    pub discard_changes: Vec<ChangeHash>,
}

// ============================================================================
// Channel Resolution State
// ============================================================================

/// Tracks resolution state for a channel.
///
/// This is stored in Raft KV to persist resolution progress across sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChannelResolutionState {
    /// List of conflicts detected.
    pub conflicts: Vec<DetailedFileConflict>,

    /// Pending resolution actions (prepared but not committed).
    pub pending_actions: Vec<ResolutionAction>,

    /// Channel head when conflicts were detected.
    pub head_at_detection: Option<ChangeHash>,

    /// When this state was last updated.
    pub updated_at_ms: u64,
}

impl ChannelResolutionState {
    /// Create a new empty resolution state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if there are any unresolved conflicts.
    pub fn has_unresolved(&self) -> bool {
        self.conflicts.iter().any(|c| c.resolution_status == ResolutionStatus::Unresolved)
    }

    /// Count unresolved conflicts.
    pub fn unresolved_count(&self) -> usize {
        self.conflicts.iter().filter(|c| c.resolution_status == ResolutionStatus::Unresolved).count()
    }

    /// Count pending resolutions.
    pub fn pending_count(&self) -> usize {
        self.pending_actions.len()
    }

    /// Get conflicts by kind.
    pub fn conflicts_by_kind(&self, kind: ConflictKind) -> Vec<&DetailedFileConflict> {
        self.conflicts.iter().filter(|c| c.kind == kind).collect()
    }

    /// Get a conflict by path.
    pub fn get_conflict(&self, path: &str) -> Option<&DetailedFileConflict> {
        self.conflicts.iter().find(|c| c.path == path)
    }

    /// Get a mutable conflict by path.
    pub fn get_conflict_mut(&mut self, path: &str) -> Option<&mut DetailedFileConflict> {
        self.conflicts.iter_mut().find(|c| c.path == path)
    }

    /// Add a pending resolution action.
    ///
    /// Updates the conflict's status to Pending.
    pub fn add_pending_action(&mut self, action: ResolutionAction) {
        // Update conflict status
        if let Some(conflict) = self.get_conflict_mut(&action.path) {
            conflict.resolution_status = ResolutionStatus::Pending;
        }

        // Remove any existing action for this path
        self.pending_actions.retain(|a| a.path != action.path);

        // Add the new action
        self.pending_actions.push(action);

        self.updated_at_ms = chrono::Utc::now().timestamp_millis() as u64;
    }

    /// Mark a conflict as resolved.
    pub fn mark_resolved(&mut self, path: &str) {
        if let Some(conflict) = self.get_conflict_mut(path) {
            conflict.resolution_status = ResolutionStatus::Resolved;
        }
        self.pending_actions.retain(|a| a.path != path);
        self.updated_at_ms = chrono::Utc::now().timestamp_millis() as u64;
    }

    /// Clear all resolved conflicts.
    pub fn clear_resolved(&mut self) {
        self.conflicts.retain(|c| c.resolution_status != ResolutionStatus::Resolved);
        self.updated_at_ms = chrono::Utc::now().timestamp_millis() as u64;
    }
}

// ============================================================================
// Resolution Helpers
// ============================================================================

/// Check if a strategy is applicable to a conflict kind.
///
/// Not all strategies work for all conflict types:
/// - Name conflicts: All strategies work
/// - Order conflicts: Manual recommended, ours/theirs work
/// - Cyclic conflicts: Manual only
/// - Zombie conflicts: Ours/theirs work
/// - Multiple names: Ours/theirs work
pub fn is_strategy_applicable(strategy: ResolutionStrategy, kind: ConflictKind) -> bool {
    match (strategy, kind) {
        // Manual works for everything
        (ResolutionStrategy::Manual, _) => true,

        // Cyclic conflicts typically need manual intervention
        (_, ConflictKind::Cyclic) => strategy == ResolutionStrategy::Manual,

        // All other strategies work for other conflict types
        _ => true,
    }
}

/// Get recommended strategy for a conflict kind.
pub fn recommended_strategy(kind: ConflictKind) -> ResolutionStrategy {
    match kind {
        ConflictKind::Name => ResolutionStrategy::Theirs,
        ConflictKind::Order => ResolutionStrategy::Manual,
        ConflictKind::Cyclic => ResolutionStrategy::Manual,
        ConflictKind::Zombie => ResolutionStrategy::Theirs,
        ConflictKind::MultipleNames => ResolutionStrategy::Theirs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_strategy_display_parse() {
        for strategy in [
            ResolutionStrategy::Ours,
            ResolutionStrategy::Theirs,
            ResolutionStrategy::Newest,
            ResolutionStrategy::Oldest,
            ResolutionStrategy::Manual,
        ] {
            let s = strategy.to_string();
            let parsed: ResolutionStrategy = s.parse().unwrap();
            assert_eq!(strategy, parsed);
        }
    }

    #[test]
    fn test_resolution_state_unresolved_count() {
        let mut state = ChannelResolutionState::new();

        state.conflicts.push(DetailedFileConflict {
            path: "file1.txt".to_string(),
            kind: ConflictKind::Order,
            involved_changes: vec![],
            resolution_status: ResolutionStatus::Unresolved,
            detected_at_ms: 0,
            markers: None,
        });

        state.conflicts.push(DetailedFileConflict {
            path: "file2.txt".to_string(),
            kind: ConflictKind::Name,
            involved_changes: vec![],
            resolution_status: ResolutionStatus::Resolved,
            detected_at_ms: 0,
            markers: None,
        });

        assert_eq!(state.unresolved_count(), 1);
        assert!(state.has_unresolved());
    }

    #[test]
    fn test_add_pending_action() {
        let mut state = ChannelResolutionState::new();

        state.conflicts.push(DetailedFileConflict {
            path: "config.toml".to_string(),
            kind: ConflictKind::Order,
            involved_changes: vec![],
            resolution_status: ResolutionStatus::Unresolved,
            detected_at_ms: 0,
            markers: None,
        });

        let action = ResolutionAction {
            path: "config.toml".to_string(),
            strategy: ResolutionStrategy::Theirs,
            keep_change: None,
            discard_changes: vec![],
        };

        state.add_pending_action(action);

        assert_eq!(state.pending_count(), 1);
        assert_eq!(state.get_conflict("config.toml").unwrap().resolution_status, ResolutionStatus::Pending);
    }

    #[test]
    fn test_strategy_applicability() {
        // Manual always works
        assert!(is_strategy_applicable(ResolutionStrategy::Manual, ConflictKind::Cyclic));
        assert!(is_strategy_applicable(ResolutionStrategy::Manual, ConflictKind::Order));

        // Cyclic only supports manual
        assert!(!is_strategy_applicable(ResolutionStrategy::Ours, ConflictKind::Cyclic));
        assert!(!is_strategy_applicable(ResolutionStrategy::Theirs, ConflictKind::Cyclic));

        // Other conflicts support more strategies
        assert!(is_strategy_applicable(ResolutionStrategy::Ours, ConflictKind::Name));
        assert!(is_strategy_applicable(ResolutionStrategy::Theirs, ConflictKind::Order));
    }
}
