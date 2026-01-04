//! Issue state resolution.

use std::collections::HashSet;

use aspen_core::hlc::SerializableTimestamp;
use iroh::PublicKey;
use serde::Deserialize;
use serde::Serialize;

/// Resolved state of an issue.
///
/// This is computed by walking the change DAG and applying all operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Issue {
    /// Issue title.
    pub title: String,

    /// Issue body/description.
    pub body: String,

    /// Current state (open/closed).
    pub state: IssueState,

    /// Labels.
    pub labels: HashSet<String>,

    /// Comments in chronological order.
    pub comments: Vec<Comment>,

    /// Reactions (emoji → set of reactors).
    pub reactions: std::collections::HashMap<String, HashSet<[u8; 32]>>,

    /// Assignees.
    pub assignees: HashSet<[u8; 32]>,

    /// Timestamp of creation.
    pub created_at_ms: u64,

    /// Timestamp of last activity.
    pub updated_at_ms: u64,
}

/// State of an issue.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueState {
    /// Issue is open.
    #[default]
    Open,

    /// Issue is closed.
    Closed {
        /// Optional reason for closing.
        reason: Option<String>,
    },
}

impl IssueState {
    /// Check if the issue is open.
    pub fn is_open(&self) -> bool {
        matches!(self, IssueState::Open)
    }

    /// Check if the issue is closed.
    pub fn is_closed(&self) -> bool {
        matches!(self, IssueState::Closed { .. })
    }
}

/// A comment on an issue.
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

impl Issue {
    /// Create a new issue with the given initial state.
    pub fn new(title: String, body: String, labels: Vec<String>, created_at_ms: u64) -> Self {
        Self {
            title,
            body,
            state: IssueState::Open,
            labels: labels.into_iter().collect(),
            comments: Vec::new(),
            reactions: std::collections::HashMap::new(),
            assignees: HashSet::new(),
            created_at_ms,
            updated_at_ms: created_at_ms,
        }
    }

    /// Apply a change to update the issue state.
    pub fn apply_change(
        &mut self,
        change_hash: blake3::Hash,
        author: &PublicKey,
        hlc_timestamp: &SerializableTimestamp,
        op: &super::change::CobOperation,
    ) {
        use super::change::CobOperation;

        let timestamp_ms = hlc_timestamp.to_unix_ms();
        self.updated_at_ms = timestamp_ms.max(self.updated_at_ms);

        match op {
            CobOperation::CreateIssue { title, body, labels } => {
                self.title = title.clone();
                self.body = body.clone();
                self.labels = labels.iter().cloned().collect();
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
                self.state = IssueState::Closed { reason: reason.clone() };
            }

            CobOperation::Reopen => {
                self.state = IssueState::Open;
            }

            CobOperation::EditTitle { title } => {
                self.title = title.clone();
            }

            CobOperation::EditBody { body } => {
                self.body = body.clone();
            }

            CobOperation::React { emoji } => {
                self.reactions.entry(emoji.clone()).or_default().insert(*author.as_bytes());
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

            // Patch-specific operations are not applicable to issues
            CobOperation::CreatePatch { .. }
            | CobOperation::UpdatePatch { .. }
            | CobOperation::Merge { .. }
            | CobOperation::Approve { .. }
            | CobOperation::RequestChanges { .. } => {
                // Ignore patch operations on issues
            }

            // Merge operations are handled at the store level during resolution.
            // By the time we apply changes, the merge has already been processed
            // and the resolutions have been incorporated into the topological order.
            CobOperation::MergeHeads { .. } => {
                // Merge commits don't directly modify issue state - they just
                // serve to unify divergent heads in the DAG.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cob::change::CobOperation;
    use aspen_core::hlc::create_hlc;

    fn test_key() -> PublicKey {
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        secret.public()
    }

    fn test_timestamp(hlc: &aspen_core::hlc::HLC) -> SerializableTimestamp {
        SerializableTimestamp::from(hlc.new_timestamp())
    }

    #[test]
    fn test_issue_lifecycle() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let mut issue = Issue::default();

        // Create issue
        let create_hash = blake3::hash(b"create");
        issue.apply_change(
            create_hash,
            &author,
            &test_timestamp(&hlc),
            &CobOperation::CreateIssue {
                title: "Bug report".to_string(),
                body: "Something is broken".to_string(),
                labels: vec!["bug".to_string()],
            },
        );

        assert_eq!(issue.title, "Bug report");
        assert!(issue.state.is_open());
        assert!(issue.labels.contains("bug"));

        // Add comment
        let comment_hash = blake3::hash(b"comment");
        issue.apply_change(
            comment_hash,
            &author,
            &test_timestamp(&hlc),
            &CobOperation::Comment {
                body: "I can reproduce this".to_string(),
            },
        );

        assert_eq!(issue.comments.len(), 1);
        assert_eq!(issue.comments[0].body, "I can reproduce this");

        // Close issue
        let close_hash = blake3::hash(b"close");
        issue.apply_change(
            close_hash,
            &author,
            &test_timestamp(&hlc),
            &CobOperation::Close {
                reason: Some("Fixed".to_string()),
            },
        );

        assert!(issue.state.is_closed());

        // Reopen
        let reopen_hash = blake3::hash(b"reopen");
        issue.apply_change(reopen_hash, &author, &test_timestamp(&hlc), &CobOperation::Reopen);

        assert!(issue.state.is_open());
    }

    #[test]
    fn test_issue_labels() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let mut issue = Issue::new("Test".to_string(), "".to_string(), vec![], 0);

        // Add labels
        issue.apply_change(
            blake3::hash(b"1"),
            &author,
            &test_timestamp(&hlc),
            &CobOperation::AddLabel {
                label: "bug".to_string(),
            },
        );
        issue.apply_change(
            blake3::hash(b"2"),
            &author,
            &test_timestamp(&hlc),
            &CobOperation::AddLabel {
                label: "critical".to_string(),
            },
        );

        assert!(issue.labels.contains("bug"));
        assert!(issue.labels.contains("critical"));
        assert_eq!(issue.labels.len(), 2);

        // Remove label
        issue.apply_change(
            blake3::hash(b"3"),
            &author,
            &test_timestamp(&hlc),
            &CobOperation::RemoveLabel {
                label: "bug".to_string(),
            },
        );

        assert!(!issue.labels.contains("bug"));
        assert_eq!(issue.labels.len(), 1);
    }
}
