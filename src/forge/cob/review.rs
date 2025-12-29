//! Review state resolution.

use serde::{Deserialize, Serialize};

/// Resolved state of a review.
///
/// This is computed by walking the change DAG and applying all operations.
/// A review is attached to a specific patch and commit, and tracks the
/// reviewer's verdict (approved, changes requested, or pending).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    /// ID of the patch being reviewed.
    pub patch_id: [u8; 32],

    /// The commit hash that was reviewed.
    pub commit: [u8; 32],

    /// Current verdict of the review.
    pub verdict: ReviewVerdict,

    /// Inline comments (on specific lines of code).
    pub inline_comments: Vec<InlineComment>,

    /// General comments (not attached to specific code locations).
    pub general_comments: Vec<GeneralComment>,

    /// Author of the review (reviewer's public key).
    pub author: [u8; 32],

    /// Timestamp of creation.
    pub created_at_ms: u64,

    /// Timestamp of last activity.
    pub updated_at_ms: u64,
}

/// Verdict of a review.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewVerdict {
    /// Review is pending (no verdict yet).
    #[default]
    Pending,

    /// Review approved the changes.
    Approved {
        /// Optional approval message.
        message: Option<String>,
    },

    /// Review requested changes before approval.
    ChangesRequested,
}

impl ReviewVerdict {
    /// Check if the review is approved.
    pub fn is_approved(&self) -> bool {
        matches!(self, ReviewVerdict::Approved { .. })
    }

    /// Check if changes are requested.
    pub fn is_changes_requested(&self) -> bool {
        matches!(self, ReviewVerdict::ChangesRequested)
    }

    /// Check if the review is pending.
    pub fn is_pending(&self) -> bool {
        matches!(self, ReviewVerdict::Pending)
    }
}

/// An inline comment on a specific location in the code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineComment {
    /// Author's public key.
    pub author: [u8; 32],

    /// Path to the file.
    pub path: String,

    /// Line number (1-indexed).
    pub line: u32,

    /// Side of the diff (old or new).
    pub side: super::change::ReviewSide,

    /// Comment body.
    pub body: String,

    /// Timestamp.
    pub timestamp_ms: u64,

    /// Hash of the change that created this comment.
    pub change_hash: [u8; 32],
}

/// A general comment on the review (not attached to specific code).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralComment {
    /// Author's public key.
    pub author: [u8; 32],

    /// Comment body.
    pub body: String,

    /// Timestamp.
    pub timestamp_ms: u64,

    /// Hash of the change that created this comment.
    pub change_hash: [u8; 32],
}

impl Default for Review {
    fn default() -> Self {
        Self {
            patch_id: [0; 32],
            commit: [0; 32],
            verdict: ReviewVerdict::Pending,
            inline_comments: Vec::new(),
            general_comments: Vec::new(),
            author: [0; 32],
            created_at_ms: 0,
            updated_at_ms: 0,
        }
    }
}

impl Review {
    /// Create a new review for a patch.
    pub fn new(patch_id: [u8; 32], commit: [u8; 32], author: [u8; 32], created_at_ms: u64) -> Self {
        Self {
            patch_id,
            commit,
            verdict: ReviewVerdict::Pending,
            inline_comments: Vec::new(),
            general_comments: Vec::new(),
            author,
            created_at_ms,
            updated_at_ms: created_at_ms,
        }
    }

    /// Apply a change to update the review state.
    pub fn apply_change(
        &mut self,
        change_hash: blake3::Hash,
        author: &iroh::PublicKey,
        timestamp_ms: u64,
        op: &super::change::CobOperation,
    ) {
        use super::change::CobOperation;

        self.updated_at_ms = timestamp_ms.max(self.updated_at_ms);

        match op {
            CobOperation::Approve { commit, message } => {
                self.commit = *commit;
                self.verdict = ReviewVerdict::Approved {
                    message: message.clone(),
                };
            }

            CobOperation::RequestChanges { commit, comments } => {
                self.commit = *commit;
                self.verdict = ReviewVerdict::ChangesRequested;

                // Add inline comments from the review
                for comment in comments {
                    self.inline_comments.push(InlineComment {
                        author: *author.as_bytes(),
                        path: comment.path.clone(),
                        line: comment.line,
                        side: comment.side,
                        body: comment.body.clone(),
                        timestamp_ms,
                        change_hash: *change_hash.as_bytes(),
                    });
                }
            }

            CobOperation::Comment { body } => {
                self.general_comments.push(GeneralComment {
                    author: *author.as_bytes(),
                    body: body.clone(),
                    timestamp_ms,
                    change_hash: *change_hash.as_bytes(),
                });
            }

            // Issue and Patch operations are not applicable to reviews
            CobOperation::CreateIssue { .. }
            | CobOperation::AddLabel { .. }
            | CobOperation::RemoveLabel { .. }
            | CobOperation::Close { .. }
            | CobOperation::Reopen
            | CobOperation::EditTitle { .. }
            | CobOperation::EditBody { .. }
            | CobOperation::CreatePatch { .. }
            | CobOperation::UpdatePatch { .. }
            | CobOperation::Merge { .. }
            | CobOperation::React { .. }
            | CobOperation::Unreact { .. }
            | CobOperation::Assign { .. }
            | CobOperation::Unassign { .. } => {
                // Ignore non-review operations
            }

            // Merge operations are handled at the store level during resolution.
            // By the time we apply changes, the merge has already been processed
            // and the resolutions have been incorporated into the topological order.
            CobOperation::MergeHeads { .. } => {
                // Merge commits don't directly modify review state - they just
                // serve to unify divergent heads in the DAG.
            }
        }
    }

    /// Get the patch ID as a blake3::Hash.
    pub fn patch_id(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.patch_id)
    }

    /// Get the commit hash as a blake3::Hash.
    pub fn commit_hash(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.commit)
    }

    /// Get all comments (both inline and general) in chronological order.
    pub fn all_comments_chronological(&self) -> Vec<CommentRef<'_>> {
        let mut comments: Vec<CommentRef<'_>> = Vec::new();

        for inline in &self.inline_comments {
            comments.push(CommentRef::Inline(inline));
        }

        for general in &self.general_comments {
            comments.push(CommentRef::General(general));
        }

        comments.sort_by_key(|c| match c {
            CommentRef::Inline(i) => i.timestamp_ms,
            CommentRef::General(g) => g.timestamp_ms,
        });

        comments
    }

    /// Get inline comments for a specific file.
    pub fn comments_for_file(&self, path: &str) -> Vec<&InlineComment> {
        self.inline_comments
            .iter()
            .filter(|c| c.path == path)
            .collect()
    }
}

/// Reference to either an inline or general comment.
#[derive(Debug, Clone)]
pub enum CommentRef<'a> {
    /// Inline comment on specific code.
    Inline(&'a InlineComment),
    /// General comment on the review.
    General(&'a GeneralComment),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::cob::change::{CobOperation, ReviewComment, ReviewSide};

    fn test_key() -> iroh::PublicKey {
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        secret.public()
    }

    #[test]
    fn test_review_creation() {
        let patch_id = [1; 32];
        let commit = [2; 32];
        let author = [3; 32];
        let created_at_ms = 1000;

        let review = Review::new(patch_id, commit, author, created_at_ms);

        assert_eq!(review.patch_id, patch_id);
        assert_eq!(review.commit, commit);
        assert_eq!(review.author, author);
        assert!(review.verdict.is_pending());
        assert!(review.inline_comments.is_empty());
        assert!(review.general_comments.is_empty());
    }

    #[test]
    fn test_review_approve() {
        let author = test_key();
        let mut review = Review::default();

        let commit = [42; 32];
        let change_hash = blake3::hash(b"approve");

        review.apply_change(
            change_hash,
            &author,
            1000,
            &CobOperation::Approve {
                commit,
                message: Some("Looks good to me!".to_string()),
            },
        );

        assert!(review.verdict.is_approved());
        assert_eq!(review.commit, commit);

        if let ReviewVerdict::Approved { message } = &review.verdict {
            assert_eq!(message.as_deref(), Some("Looks good to me!"));
        } else {
            panic!("Expected Approved verdict");
        }
    }

    #[test]
    fn test_review_request_changes() {
        let author = test_key();
        let mut review = Review::default();

        let commit = [42; 32];
        let change_hash = blake3::hash(b"request-changes");

        let comments = vec![
            ReviewComment {
                path: "src/main.rs".to_string(),
                line: 10,
                side: ReviewSide::New,
                body: "This variable name is unclear".to_string(),
            },
            ReviewComment {
                path: "src/lib.rs".to_string(),
                line: 25,
                side: ReviewSide::Old,
                body: "Why was this removed?".to_string(),
            },
        ];

        review.apply_change(
            change_hash,
            &author,
            1000,
            &CobOperation::RequestChanges {
                commit,
                comments: comments.clone(),
            },
        );

        assert!(review.verdict.is_changes_requested());
        assert_eq!(review.commit, commit);
        assert_eq!(review.inline_comments.len(), 2);

        let first_comment = &review.inline_comments[0];
        assert_eq!(first_comment.path, "src/main.rs");
        assert_eq!(first_comment.line, 10);
        assert_eq!(first_comment.side, ReviewSide::New);
        assert_eq!(first_comment.body, "This variable name is unclear");

        let second_comment = &review.inline_comments[1];
        assert_eq!(second_comment.path, "src/lib.rs");
        assert_eq!(second_comment.line, 25);
        assert_eq!(second_comment.side, ReviewSide::Old);
    }

    #[test]
    fn test_review_general_comment() {
        let author = test_key();
        let mut review = Review::default();

        let change_hash = blake3::hash(b"comment");

        review.apply_change(
            change_hash,
            &author,
            1000,
            &CobOperation::Comment {
                body: "Overall the approach looks good".to_string(),
            },
        );

        assert_eq!(review.general_comments.len(), 1);
        assert_eq!(
            review.general_comments[0].body,
            "Overall the approach looks good"
        );
        assert_eq!(review.general_comments[0].author, *author.as_bytes());
    }

    #[test]
    fn test_review_lifecycle() {
        let reviewer = test_key();
        let patch_id = [1; 32];
        let commit_v1 = [2; 32];
        let commit_v2 = [3; 32];
        let author_bytes = [4; 32];

        let mut review = Review::new(patch_id, commit_v1, author_bytes, 1000);

        // First, request changes
        let change_hash_1 = blake3::hash(b"request-changes");
        review.apply_change(
            change_hash_1,
            &reviewer,
            2000,
            &CobOperation::RequestChanges {
                commit: commit_v1,
                comments: vec![ReviewComment {
                    path: "src/main.rs".to_string(),
                    line: 10,
                    side: ReviewSide::New,
                    body: "Please add error handling".to_string(),
                }],
            },
        );

        assert!(review.verdict.is_changes_requested());
        assert_eq!(review.inline_comments.len(), 1);

        // Add a general comment
        let change_hash_2 = blake3::hash(b"comment");
        review.apply_change(
            change_hash_2,
            &reviewer,
            3000,
            &CobOperation::Comment {
                body: "See my inline comment".to_string(),
            },
        );

        assert_eq!(review.general_comments.len(), 1);

        // After patch is updated, approve
        let change_hash_3 = blake3::hash(b"approve");
        review.apply_change(
            change_hash_3,
            &reviewer,
            4000,
            &CobOperation::Approve {
                commit: commit_v2,
                message: Some("LGTM now!".to_string()),
            },
        );

        assert!(review.verdict.is_approved());
        assert_eq!(review.commit, commit_v2);
        // Comments should still be preserved
        assert_eq!(review.inline_comments.len(), 1);
        assert_eq!(review.general_comments.len(), 1);
    }

    #[test]
    fn test_review_verdict_helpers() {
        let pending = ReviewVerdict::Pending;
        assert!(pending.is_pending());
        assert!(!pending.is_approved());
        assert!(!pending.is_changes_requested());

        let approved = ReviewVerdict::Approved { message: None };
        assert!(!approved.is_pending());
        assert!(approved.is_approved());
        assert!(!approved.is_changes_requested());

        let changes_requested = ReviewVerdict::ChangesRequested;
        assert!(!changes_requested.is_pending());
        assert!(!changes_requested.is_approved());
        assert!(changes_requested.is_changes_requested());
    }

    #[test]
    fn test_comments_for_file() {
        let author = test_key();
        let mut review = Review::default();

        let commit = [42; 32];
        let change_hash = blake3::hash(b"request-changes");

        let comments = vec![
            ReviewComment {
                path: "src/main.rs".to_string(),
                line: 10,
                side: ReviewSide::New,
                body: "Comment 1".to_string(),
            },
            ReviewComment {
                path: "src/lib.rs".to_string(),
                line: 20,
                side: ReviewSide::New,
                body: "Comment 2".to_string(),
            },
            ReviewComment {
                path: "src/main.rs".to_string(),
                line: 30,
                side: ReviewSide::New,
                body: "Comment 3".to_string(),
            },
        ];

        review.apply_change(
            change_hash,
            &author,
            1000,
            &CobOperation::RequestChanges { commit, comments },
        );

        let main_comments = review.comments_for_file("src/main.rs");
        assert_eq!(main_comments.len(), 2);
        assert_eq!(main_comments[0].body, "Comment 1");
        assert_eq!(main_comments[1].body, "Comment 3");

        let lib_comments = review.comments_for_file("src/lib.rs");
        assert_eq!(lib_comments.len(), 1);
        assert_eq!(lib_comments[0].body, "Comment 2");

        let no_comments = review.comments_for_file("nonexistent.rs");
        assert!(no_comments.is_empty());
    }

    #[test]
    fn test_all_comments_chronological() {
        let author = test_key();
        let mut review = Review::default();

        // Add a general comment at t=2000
        review.apply_change(
            blake3::hash(b"comment-1"),
            &author,
            2000,
            &CobOperation::Comment {
                body: "General comment".to_string(),
            },
        );

        // Add inline comments at t=1000 (earlier)
        review.apply_change(
            blake3::hash(b"request-changes"),
            &author,
            1000,
            &CobOperation::RequestChanges {
                commit: [1; 32],
                comments: vec![ReviewComment {
                    path: "src/main.rs".to_string(),
                    line: 10,
                    side: ReviewSide::New,
                    body: "Inline comment".to_string(),
                }],
            },
        );

        // Add another general comment at t=3000
        review.apply_change(
            blake3::hash(b"comment-2"),
            &author,
            3000,
            &CobOperation::Comment {
                body: "Another general comment".to_string(),
            },
        );

        let all_comments = review.all_comments_chronological();
        assert_eq!(all_comments.len(), 3);

        // Should be sorted by timestamp
        match &all_comments[0] {
            CommentRef::Inline(c) => assert_eq!(c.body, "Inline comment"),
            _ => panic!("Expected inline comment first"),
        }
        match &all_comments[1] {
            CommentRef::General(c) => assert_eq!(c.body, "General comment"),
            _ => panic!("Expected general comment second"),
        }
        match &all_comments[2] {
            CommentRef::General(c) => assert_eq!(c.body, "Another general comment"),
            _ => panic!("Expected general comment third"),
        }
    }

    #[test]
    fn test_ignore_non_review_operations() {
        let author = test_key();
        let mut review = Review::new([1; 32], [2; 32], [3; 32], 1000);

        // These operations should be ignored
        review.apply_change(
            blake3::hash(b"ignored-1"),
            &author,
            2000,
            &CobOperation::CreateIssue {
                title: "Test".to_string(),
                body: "Body".to_string(),
                labels: vec![],
            },
        );

        review.apply_change(
            blake3::hash(b"ignored-2"),
            &author,
            3000,
            &CobOperation::AddLabel {
                label: "bug".to_string(),
            },
        );

        review.apply_change(
            blake3::hash(b"ignored-3"),
            &author,
            4000,
            &CobOperation::Close { reason: None },
        );

        // State should be unchanged except for updated_at_ms
        assert!(review.verdict.is_pending());
        assert!(review.inline_comments.is_empty());
        assert!(review.general_comments.is_empty());
        assert_eq!(review.updated_at_ms, 4000); // Only timestamp updated
    }
}
