//! Patch state resolution.

use std::collections::HashSet;

use iroh::PublicKey;
use serde::{Deserialize, Serialize};

/// Resolved state of a patch (code change proposal).
///
/// This is computed by walking the change DAG and applying all operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Patch {
    /// Patch title.
    pub title: String,

    /// Patch description.
    pub description: String,

    /// Base commit hash (the target branch point).
    pub base: [u8; 32],

    /// Current head commit hash (the proposed change).
    pub head: [u8; 32],

    /// Current state (open/merged/closed).
    pub state: PatchState,

    /// Labels.
    pub labels: HashSet<String>,

    /// Comments in chronological order.
    pub comments: Vec<Comment>,

    /// Revision history (updates to head).
    pub revisions: Vec<PatchRevision>,

    /// Reactions (emoji -> set of reactors).
    pub reactions: std::collections::HashMap<String, HashSet<[u8; 32]>>,

    /// Assignees (reviewers).
    pub assignees: HashSet<[u8; 32]>,

    /// Approvals for specific commits.
    pub approvals: Vec<Approval>,

    /// Change requests for specific commits.
    pub change_requests: Vec<ChangeRequest>,

    /// Timestamp of creation.
    pub created_at_ms: u64,

    /// Timestamp of last activity.
    pub updated_at_ms: u64,
}

/// State of a patch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatchState {
    /// Patch is open and awaiting review/merge.
    #[default]
    Open,

    /// Patch has been merged.
    Merged {
        /// The commit hash that was merged.
        commit: [u8; 32],
    },

    /// Patch has been closed without merging.
    Closed {
        /// Optional reason for closing.
        reason: Option<String>,
    },
}

impl PatchState {
    /// Check if the patch is open.
    pub fn is_open(&self) -> bool {
        matches!(self, PatchState::Open)
    }

    /// Check if the patch is merged.
    pub fn is_merged(&self) -> bool {
        matches!(self, PatchState::Merged { .. })
    }

    /// Check if the patch is closed.
    pub fn is_closed(&self) -> bool {
        matches!(self, PatchState::Closed { .. })
    }

    /// Get the merged commit hash if merged.
    pub fn merged_commit(&self) -> Option<[u8; 32]> {
        match self {
            PatchState::Merged { commit } => Some(*commit),
            _ => None,
        }
    }
}

/// A revision (update) to a patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchRevision {
    /// The new head commit hash.
    pub head: [u8; 32],

    /// Optional message explaining the update.
    pub message: Option<String>,

    /// Author who pushed this revision.
    pub author: [u8; 32],

    /// Timestamp of the revision.
    pub timestamp_ms: u64,

    /// Hash of the change that created this revision.
    pub change_hash: [u8; 32],
}

/// A comment on a patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// Author's public key.
    pub author: [u8; 32],

    /// Comment body.
    pub body: String,

    /// Timestamp.
    pub timestamp_ms: u64,

    /// Hash of the change that created this comment.
    pub change_hash: [u8; 32],
}

/// An approval on a patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    /// Approver's public key.
    pub author: [u8; 32],

    /// The commit hash that was approved.
    pub commit: [u8; 32],

    /// Optional approval message.
    pub message: Option<String>,

    /// Timestamp.
    pub timestamp_ms: u64,

    /// Hash of the change that created this approval.
    pub change_hash: [u8; 32],
}

/// A request for changes on a patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRequest {
    /// Reviewer's public key.
    pub author: [u8; 32],

    /// The commit hash being reviewed.
    pub commit: [u8; 32],

    /// Review comments.
    pub comments: Vec<super::change::ReviewComment>,

    /// Timestamp.
    pub timestamp_ms: u64,

    /// Hash of the change that created this request.
    pub change_hash: [u8; 32],
}

impl Patch {
    /// Create a new patch with the given initial state.
    pub fn new(
        title: String,
        description: String,
        base: [u8; 32],
        head: [u8; 32],
        created_at_ms: u64,
    ) -> Self {
        Self {
            title,
            description,
            base,
            head,
            state: PatchState::Open,
            labels: HashSet::new(),
            comments: Vec::new(),
            revisions: Vec::new(),
            reactions: std::collections::HashMap::new(),
            assignees: HashSet::new(),
            approvals: Vec::new(),
            change_requests: Vec::new(),
            created_at_ms,
            updated_at_ms: created_at_ms,
        }
    }

    /// Apply a change to update the patch state.
    pub fn apply_change(
        &mut self,
        change_hash: blake3::Hash,
        author: &PublicKey,
        timestamp_ms: u64,
        op: &super::change::CobOperation,
    ) {
        use super::change::CobOperation;

        self.updated_at_ms = timestamp_ms.max(self.updated_at_ms);

        match op {
            CobOperation::CreatePatch {
                title,
                description,
                base,
                head,
            } => {
                self.title = title.clone();
                self.description = description.clone();
                self.base = *base;
                self.head = *head;
            }

            CobOperation::UpdatePatch { head, message } => {
                // Record the revision
                self.revisions.push(PatchRevision {
                    head: *head,
                    message: message.clone(),
                    author: *author.as_bytes(),
                    timestamp_ms,
                    change_hash: *change_hash.as_bytes(),
                });

                // Update current head
                self.head = *head;
            }

            CobOperation::Merge { commit } => {
                self.state = PatchState::Merged { commit: *commit };
            }

            CobOperation::Comment { body } => {
                self.comments.push(Comment {
                    author: *author.as_bytes(),
                    body: body.clone(),
                    timestamp_ms,
                    change_hash: *change_hash.as_bytes(),
                });
            }

            CobOperation::AddLabel { label } => {
                self.labels.insert(label.clone());
            }

            CobOperation::RemoveLabel { label } => {
                self.labels.remove(label);
            }

            CobOperation::Close { reason } => {
                self.state = PatchState::Closed {
                    reason: reason.clone(),
                };
            }

            CobOperation::Reopen => {
                // Can only reopen if closed (not merged)
                if matches!(self.state, PatchState::Closed { .. }) {
                    self.state = PatchState::Open;
                }
            }

            CobOperation::EditTitle { title } => {
                self.title = title.clone();
            }

            CobOperation::EditBody { body } => {
                self.description = body.clone();
            }

            CobOperation::React { emoji } => {
                self.reactions
                    .entry(emoji.clone())
                    .or_default()
                    .insert(*author.as_bytes());
            }

            CobOperation::Unreact { emoji } => {
                let author_bytes = *author.as_bytes();
                if let Some(reactors) = self.reactions.get_mut(emoji) {
                    reactors.remove(&author_bytes);
                    if reactors.is_empty() {
                        self.reactions.remove(emoji);
                    }
                }
            }

            CobOperation::Assign { assignee } => {
                self.assignees.insert(*assignee);
            }

            CobOperation::Unassign { assignee } => {
                self.assignees.remove(assignee);
            }

            CobOperation::Approve { commit, message } => {
                self.approvals.push(Approval {
                    author: *author.as_bytes(),
                    commit: *commit,
                    message: message.clone(),
                    timestamp_ms,
                    change_hash: *change_hash.as_bytes(),
                });
            }

            CobOperation::RequestChanges { commit, comments } => {
                self.change_requests.push(ChangeRequest {
                    author: *author.as_bytes(),
                    commit: *commit,
                    comments: comments.clone(),
                    timestamp_ms,
                    change_hash: *change_hash.as_bytes(),
                });
            }

            // Issue-specific operations are not applicable to patches
            CobOperation::CreateIssue { .. } => {
                // Ignore issue creation on patches
            }
        }
    }

    /// Check if the patch has been approved for a specific commit.
    pub fn is_approved_for(&self, commit: &[u8; 32]) -> bool {
        self.approvals.iter().any(|a| &a.commit == commit)
    }

    /// Get all approvers for a specific commit.
    pub fn approvers_for(&self, commit: &[u8; 32]) -> Vec<[u8; 32]> {
        self.approvals
            .iter()
            .filter(|a| &a.commit == commit)
            .map(|a| a.author)
            .collect()
    }

    /// Check if there are pending change requests for the current head.
    pub fn has_pending_changes_requested(&self) -> bool {
        // Check if any change request is for the current head
        // and there's no newer approval for that head
        self.change_requests
            .iter()
            .any(|cr| cr.commit == self.head && !self.is_approved_for(&self.head))
    }

    /// Get the number of revisions (updates) to this patch.
    pub fn revision_count(&self) -> usize {
        self.revisions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::cob::change::{CobOperation, ReviewComment, ReviewSide};

    fn test_key() -> PublicKey {
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        secret.public()
    }

    #[test]
    fn test_patch_creation() {
        let author = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let mut patch = Patch::default();

        let create_hash = blake3::hash(b"create");
        patch.apply_change(
            create_hash,
            &author,
            1000,
            &CobOperation::CreatePatch {
                title: "Add feature X".to_string(),
                description: "This patch adds feature X".to_string(),
                base,
                head,
            },
        );

        assert_eq!(patch.title, "Add feature X");
        assert_eq!(patch.description, "This patch adds feature X");
        assert_eq!(patch.base, base);
        assert_eq!(patch.head, head);
        assert!(patch.state.is_open());
    }

    #[test]
    fn test_patch_lifecycle() {
        let author = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head1 = *blake3::hash(b"head1").as_bytes();
        let head2 = *blake3::hash(b"head2").as_bytes();
        let merge_commit = *blake3::hash(b"merge").as_bytes();
        let mut patch = Patch::default();

        // Create patch
        patch.apply_change(
            blake3::hash(b"create"),
            &author,
            1000,
            &CobOperation::CreatePatch {
                title: "Feature".to_string(),
                description: "Description".to_string(),
                base,
                head: head1,
            },
        );

        assert!(patch.state.is_open());
        assert_eq!(patch.head, head1);
        assert_eq!(patch.revision_count(), 0);

        // Update patch with new revision
        patch.apply_change(
            blake3::hash(b"update"),
            &author,
            2000,
            &CobOperation::UpdatePatch {
                head: head2,
                message: Some("Fixed review comments".to_string()),
            },
        );

        assert_eq!(patch.head, head2);
        assert_eq!(patch.revision_count(), 1);
        assert_eq!(
            patch.revisions[0].message,
            Some("Fixed review comments".to_string())
        );

        // Merge patch
        patch.apply_change(
            blake3::hash(b"merge"),
            &author,
            3000,
            &CobOperation::Merge {
                commit: merge_commit,
            },
        );

        assert!(patch.state.is_merged());
        assert_eq!(patch.state.merged_commit(), Some(merge_commit));
    }

    #[test]
    fn test_patch_close_and_reopen() {
        let author = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let mut patch = Patch::new("Test".to_string(), "".to_string(), base, head, 0);

        // Close patch
        patch.apply_change(
            blake3::hash(b"close"),
            &author,
            1000,
            &CobOperation::Close {
                reason: Some("Superseded by #42".to_string()),
            },
        );

        assert!(patch.state.is_closed());
        assert!(!patch.state.is_open());

        // Reopen patch
        patch.apply_change(
            blake3::hash(b"reopen"),
            &author,
            2000,
            &CobOperation::Reopen,
        );

        assert!(patch.state.is_open());
    }

    #[test]
    fn test_patch_cannot_reopen_merged() {
        let author = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let merge_commit = *blake3::hash(b"merge").as_bytes();
        let mut patch = Patch::new("Test".to_string(), "".to_string(), base, head, 0);

        // Merge patch
        patch.apply_change(
            blake3::hash(b"merge"),
            &author,
            1000,
            &CobOperation::Merge {
                commit: merge_commit,
            },
        );

        assert!(patch.state.is_merged());

        // Try to reopen - should not work
        patch.apply_change(
            blake3::hash(b"reopen"),
            &author,
            2000,
            &CobOperation::Reopen,
        );

        // Should still be merged
        assert!(patch.state.is_merged());
    }

    #[test]
    fn test_patch_comments() {
        let author = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let mut patch = Patch::new("Test".to_string(), "".to_string(), base, head, 0);

        // Add comments
        patch.apply_change(
            blake3::hash(b"comment1"),
            &author,
            1000,
            &CobOperation::Comment {
                body: "Looks good!".to_string(),
            },
        );
        patch.apply_change(
            blake3::hash(b"comment2"),
            &author,
            2000,
            &CobOperation::Comment {
                body: "One small nit".to_string(),
            },
        );

        assert_eq!(patch.comments.len(), 2);
        assert_eq!(patch.comments[0].body, "Looks good!");
        assert_eq!(patch.comments[1].body, "One small nit");
    }

    #[test]
    fn test_patch_labels() {
        let author = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let mut patch = Patch::new("Test".to_string(), "".to_string(), base, head, 0);

        // Add labels
        patch.apply_change(
            blake3::hash(b"label1"),
            &author,
            1000,
            &CobOperation::AddLabel {
                label: "needs-review".to_string(),
            },
        );
        patch.apply_change(
            blake3::hash(b"label2"),
            &author,
            2000,
            &CobOperation::AddLabel {
                label: "wip".to_string(),
            },
        );

        assert!(patch.labels.contains("needs-review"));
        assert!(patch.labels.contains("wip"));
        assert_eq!(patch.labels.len(), 2);

        // Remove label
        patch.apply_change(
            blake3::hash(b"remove"),
            &author,
            3000,
            &CobOperation::RemoveLabel {
                label: "wip".to_string(),
            },
        );

        assert!(!patch.labels.contains("wip"));
        assert_eq!(patch.labels.len(), 1);
    }

    #[test]
    fn test_patch_approvals() {
        let _author = test_key();
        let reviewer = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let mut patch = Patch::new("Test".to_string(), "".to_string(), base, head, 0);

        // Approve
        patch.apply_change(
            blake3::hash(b"approve"),
            &reviewer,
            1000,
            &CobOperation::Approve {
                commit: head,
                message: Some("LGTM!".to_string()),
            },
        );

        assert!(patch.is_approved_for(&head));
        assert_eq!(patch.approvals.len(), 1);
        assert_eq!(patch.approvers_for(&head).len(), 1);
    }

    #[test]
    fn test_patch_change_requests() {
        let _author = test_key();
        let reviewer = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let mut patch = Patch::new("Test".to_string(), "".to_string(), base, head, 0);

        // Request changes
        patch.apply_change(
            blake3::hash(b"request"),
            &reviewer,
            1000,
            &CobOperation::RequestChanges {
                commit: head,
                comments: vec![ReviewComment {
                    path: "src/lib.rs".to_string(),
                    line: 42,
                    side: ReviewSide::New,
                    body: "Please add a comment here".to_string(),
                }],
            },
        );

        assert!(patch.has_pending_changes_requested());
        assert_eq!(patch.change_requests.len(), 1);

        // After approval, no longer pending
        patch.apply_change(
            blake3::hash(b"approve"),
            &reviewer,
            2000,
            &CobOperation::Approve {
                commit: head,
                message: None,
            },
        );

        assert!(!patch.has_pending_changes_requested());
    }

    #[test]
    fn test_patch_multiple_revisions() {
        let author = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head1 = *blake3::hash(b"head1").as_bytes();
        let head2 = *blake3::hash(b"head2").as_bytes();
        let head3 = *blake3::hash(b"head3").as_bytes();
        let mut patch = Patch::new("Test".to_string(), "".to_string(), base, head1, 0);

        // Update to v2
        patch.apply_change(
            blake3::hash(b"update1"),
            &author,
            1000,
            &CobOperation::UpdatePatch {
                head: head2,
                message: Some("v2: Addressed review".to_string()),
            },
        );

        // Update to v3
        patch.apply_change(
            blake3::hash(b"update2"),
            &author,
            2000,
            &CobOperation::UpdatePatch {
                head: head3,
                message: None,
            },
        );

        assert_eq!(patch.revision_count(), 2);
        assert_eq!(patch.head, head3);
        assert_eq!(patch.revisions[0].head, head2);
        assert_eq!(patch.revisions[1].head, head3);
    }

    #[test]
    fn test_patch_reactions() {
        let _author = test_key();
        let reactor = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let mut patch = Patch::new("Test".to_string(), "".to_string(), base, head, 0);

        // Add reaction
        patch.apply_change(
            blake3::hash(b"react"),
            &reactor,
            1000,
            &CobOperation::React {
                emoji: "+1".to_string(),
            },
        );

        assert!(patch.reactions.contains_key("+1"));
        assert!(patch.reactions["+1"].contains(reactor.as_bytes()));

        // Remove reaction
        patch.apply_change(
            blake3::hash(b"unreact"),
            &reactor,
            2000,
            &CobOperation::Unreact {
                emoji: "+1".to_string(),
            },
        );

        assert!(!patch.reactions.contains_key("+1"));
    }

    #[test]
    fn test_patch_assignees() {
        let author = test_key();
        let reviewer1 = test_key();
        let reviewer2 = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let mut patch = Patch::new("Test".to_string(), "".to_string(), base, head, 0);

        // Assign reviewers
        patch.apply_change(
            blake3::hash(b"assign1"),
            &author,
            1000,
            &CobOperation::Assign {
                assignee: *reviewer1.as_bytes(),
            },
        );
        patch.apply_change(
            blake3::hash(b"assign2"),
            &author,
            2000,
            &CobOperation::Assign {
                assignee: *reviewer2.as_bytes(),
            },
        );

        assert_eq!(patch.assignees.len(), 2);
        assert!(patch.assignees.contains(reviewer1.as_bytes()));
        assert!(patch.assignees.contains(reviewer2.as_bytes()));

        // Unassign
        patch.apply_change(
            blake3::hash(b"unassign"),
            &author,
            3000,
            &CobOperation::Unassign {
                assignee: *reviewer1.as_bytes(),
            },
        );

        assert_eq!(patch.assignees.len(), 1);
        assert!(!patch.assignees.contains(reviewer1.as_bytes()));
    }

    #[test]
    fn test_patch_edit_title_and_description() {
        let author = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let mut patch = Patch::new("Original".to_string(), "Original desc".to_string(), base, head, 0);

        patch.apply_change(
            blake3::hash(b"edit_title"),
            &author,
            1000,
            &CobOperation::EditTitle {
                title: "Updated Title".to_string(),
            },
        );

        patch.apply_change(
            blake3::hash(b"edit_body"),
            &author,
            2000,
            &CobOperation::EditBody {
                body: "Updated description".to_string(),
            },
        );

        assert_eq!(patch.title, "Updated Title");
        assert_eq!(patch.description, "Updated description");
    }

    #[test]
    fn test_patch_updated_at() {
        let author = test_key();
        let base = *blake3::hash(b"base").as_bytes();
        let head = *blake3::hash(b"head").as_bytes();
        let mut patch = Patch::new("Test".to_string(), "".to_string(), base, head, 1000);

        assert_eq!(patch.updated_at_ms, 1000);

        patch.apply_change(
            blake3::hash(b"comment"),
            &author,
            2000,
            &CobOperation::Comment {
                body: "Comment".to_string(),
            },
        );

        assert_eq!(patch.updated_at_ms, 2000);

        // Out-of-order timestamp should take max
        patch.apply_change(
            blake3::hash(b"old_comment"),
            &author,
            1500,
            &CobOperation::Comment {
                body: "Old comment".to_string(),
            },
        );

        assert_eq!(patch.updated_at_ms, 2000);
    }
}
