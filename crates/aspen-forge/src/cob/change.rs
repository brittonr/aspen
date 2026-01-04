//! COB change types and operations.

use serde::Deserialize;
use serde::Serialize;

/// Type of collaborative object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CobType {
    /// Issue (bug report, feature request).
    Issue,
    /// Patch (code change proposal).
    Patch,
    /// Review (code review on a patch).
    Review,
    /// Discussion (threaded conversation).
    Discussion,
}

impl CobType {
    /// Get the type name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            CobType::Issue => "issue",
            CobType::Patch => "patch",
            CobType::Review => "review",
            CobType::Discussion => "discussion",
        }
    }
}

impl std::fmt::Display for CobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A change to a collaborative object.
///
/// Each change is an immutable record that references its parent changes.
/// The full state of a COB is computed by walking the DAG from the heads
/// back to the root and applying all operations in causal order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CobChange {
    /// Type of COB this change belongs to.
    pub cob_type: CobType,

    /// ID of the COB instance (hash of the first change).
    pub cob_id: [u8; 32],

    /// Parent change hashes (empty for root change).
    pub parents: Vec<[u8; 32]>,

    /// The operation to apply.
    pub op: CobOperation,
}

impl CobChange {
    /// Create a new root change (no parents).
    pub fn root(cob_type: CobType, cob_id: blake3::Hash, op: CobOperation) -> Self {
        Self {
            cob_type,
            cob_id: *cob_id.as_bytes(),
            parents: vec![],
            op,
        }
    }

    /// Create a new change with parents.
    pub fn new(cob_type: CobType, cob_id: blake3::Hash, parents: Vec<blake3::Hash>, op: CobOperation) -> Self {
        Self {
            cob_type,
            cob_id: *cob_id.as_bytes(),
            parents: parents.iter().map(|h| *h.as_bytes()).collect(),
            op,
        }
    }

    /// Get the COB ID as a blake3::Hash.
    pub fn cob_id(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.cob_id)
    }

    /// Get the parent hashes.
    pub fn parents(&self) -> Vec<blake3::Hash> {
        self.parents.iter().map(|h| blake3::Hash::from_bytes(*h)).collect()
    }

    /// Check if this is a root change (no parents).
    pub fn is_root(&self) -> bool {
        self.parents.is_empty()
    }
}

/// Strategy for resolving merge conflicts.
///
/// When a COB has multiple heads (divergent histories), this determines
/// how conflicting changes are resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Automatic merge for non-conflicting changes.
    ///
    /// Used when changes don't conflict (e.g., adding different labels,
    /// different users commenting). Collections are unioned, lists are
    /// appended in causal order.
    Auto,

    /// Last-write-wins based on timestamp.
    ///
    /// For conflicting scalar fields, the change with the higher timestamp
    /// wins. If timestamps are equal, the lexicographically lower hash wins.
    LastWriteWins,

    /// Explicit user-specified resolutions.
    ///
    /// The user explicitly chose which value to keep for each conflicting field.
    Explicit,
}

/// Resolution for a specific field in a merge conflict.
///
/// When multiple heads have different values for a scalar field (title, body, state),
/// this specifies which change's value should be used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldResolution {
    /// The field being resolved (e.g., "title", "body", "state").
    pub field: String,

    /// Hash of the change whose value is selected.
    pub selected: [u8; 32],
}

impl FieldResolution {
    /// Create a new field resolution.
    pub fn new(field: impl Into<String>, selected: blake3::Hash) -> Self {
        Self {
            field: field.into(),
            selected: *selected.as_bytes(),
        }
    }

    /// Get the selected change hash.
    pub fn selected(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.selected)
    }
}

/// An operation on a collaborative object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CobOperation {
    // ========================================================================
    // Issue Operations
    // ========================================================================
    /// Create a new issue.
    CreateIssue {
        title: String,
        body: String,
        labels: Vec<String>,
    },

    /// Add a comment to an issue or patch.
    Comment { body: String },

    /// Add a label.
    AddLabel { label: String },

    /// Remove a label.
    RemoveLabel { label: String },

    /// Close an issue or patch.
    Close { reason: Option<String> },

    /// Reopen an issue or patch.
    Reopen,

    /// Edit the title.
    EditTitle { title: String },

    /// Edit the body/description.
    EditBody { body: String },

    // ========================================================================
    // Patch Operations
    // ========================================================================
    /// Create a new patch (code change proposal).
    CreatePatch {
        title: String,
        description: String,
        /// Base commit hash.
        base: [u8; 32],
        /// Head commit hash (the proposed change).
        head: [u8; 32],
    },

    /// Update the patch to a new head commit.
    UpdatePatch {
        /// New head commit hash.
        head: [u8; 32],
        /// Optional message explaining the update.
        message: Option<String>,
    },

    /// Merge the patch.
    Merge {
        /// The commit hash that was merged.
        commit: [u8; 32],
    },

    // ========================================================================
    // Review Operations
    // ========================================================================
    /// Approve a patch.
    Approve {
        /// The commit hash being approved.
        commit: [u8; 32],
        /// Optional approval message.
        message: Option<String>,
    },

    /// Request changes on a patch.
    RequestChanges {
        /// The commit hash being reviewed.
        commit: [u8; 32],
        /// Review comments.
        comments: Vec<ReviewComment>,
    },

    // ========================================================================
    // Generic Operations
    // ========================================================================
    /// Add a reaction (emoji).
    React { emoji: String },

    /// Remove a reaction.
    Unreact { emoji: String },

    /// Assign to a user (by public key).
    Assign { assignee: [u8; 32] },

    /// Unassign a user.
    Unassign { assignee: [u8; 32] },

    // ========================================================================
    // Merge Operations
    // ========================================================================
    /// Merge multiple concurrent heads.
    ///
    /// This operation is used to resolve conflicts when a COB has divergent
    /// histories. The change that contains this operation should have all
    /// the conflicting heads as its parents.
    ///
    /// Git-like behavior:
    /// - Non-conflicting changes are merged automatically (labels, reactions, comments)
    /// - Conflicting scalar fields (title, body, state) require explicit resolution
    MergeHeads {
        /// Strategy used for resolving conflicts.
        strategy: MergeStrategy,
        /// Explicit resolutions for conflicting fields (when strategy is Explicit).
        resolutions: Vec<FieldResolution>,
    },
}

/// A review comment on a specific location in the code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewComment {
    /// Path to the file being commented on.
    pub path: String,

    /// Line number (1-indexed).
    pub line: u32,

    /// Whether this is on the old or new version.
    pub side: ReviewSide,

    /// The comment body.
    pub body: String,
}

/// Which side of a diff a review comment refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewSide {
    /// The old (base) version.
    Old,
    /// The new (head) version.
    New,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cob_change_creation() {
        let cob_id = blake3::hash(b"issue-1");

        let root = CobChange::root(
            CobType::Issue,
            cob_id,
            CobOperation::CreateIssue {
                title: "Bug report".to_string(),
                body: "Something is broken".to_string(),
                labels: vec!["bug".to_string()],
            },
        );

        assert!(root.is_root());
        assert_eq!(root.cob_type, CobType::Issue);
        assert_eq!(root.cob_id(), cob_id);
    }

    #[test]
    fn test_cob_change_with_parents() {
        let cob_id = blake3::hash(b"issue-1");
        let parent = blake3::hash(b"parent");

        let change = CobChange::new(
            CobType::Issue,
            cob_id,
            vec![parent],
            CobOperation::Comment {
                body: "I can reproduce this".to_string(),
            },
        );

        assert!(!change.is_root());
        assert_eq!(change.parents().len(), 1);
        assert_eq!(change.parents()[0], parent);
    }
}
