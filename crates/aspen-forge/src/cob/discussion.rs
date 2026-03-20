//! Discussion state resolution.

use std::collections::HashSet;

use aspen_core::hlc::SerializableTimestamp;
use iroh::PublicKey;
use serde::Deserialize;
use serde::Serialize;

/// Resolved state of a discussion (standalone threaded conversation).
///
/// This is computed by walking the change DAG and applying all operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Discussion {
    /// Discussion title.
    pub title: String,

    /// Discussion body.
    pub body: String,

    /// Current state (open/closed/locked).
    pub state: DiscussionState,

    /// Labels.
    pub labels: HashSet<String>,

    /// Replies in chronological order.
    pub replies: Vec<DiscussionReply>,

    /// Reactions (emoji -> set of reactors).
    pub reactions: std::collections::HashMap<String, HashSet<[u8; 32]>>,

    /// Set of resolved thread root reply hashes.
    pub resolved_threads: HashSet<[u8; 32]>,

    /// Timestamp of creation.
    pub created_at_ms: u64,

    /// Timestamp of last activity.
    pub updated_at_ms: u64,
}

/// State of a discussion.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscussionState {
    /// Discussion is open for replies.
    #[default]
    Open,

    /// Discussion has been closed.
    Closed {
        /// Optional reason for closing.
        reason: Option<String>,
    },

    /// Discussion is locked (no new replies allowed).
    Locked,
}

impl DiscussionState {
    /// Check if the discussion is open.
    pub fn is_open(&self) -> bool {
        matches!(self, DiscussionState::Open)
    }

    /// Check if the discussion is closed.
    pub fn is_closed(&self) -> bool {
        matches!(self, DiscussionState::Closed { .. })
    }

    /// Check if the discussion is locked.
    pub fn is_locked(&self) -> bool {
        matches!(self, DiscussionState::Locked)
    }

    /// Check if replies are allowed (open but not locked).
    pub fn allows_replies(&self) -> bool {
        matches!(self, DiscussionState::Open)
    }
}

/// A reply in a discussion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscussionReply {
    /// Author's public key.
    pub author: [u8; 32],

    /// Reply body.
    pub body: String,

    /// Optional parent reply hash for threading.
    pub parent_reply: Option<[u8; 32]>,

    /// Timestamp.
    pub timestamp_ms: u64,

    /// Hash of the change that created this reply.
    pub change_hash: [u8; 32],
}

impl Discussion {
    /// Create a new discussion with the given initial state.
    pub fn new(title: String, body: String, created_at_ms: u64) -> Self {
        Self {
            title,
            body,
            state: DiscussionState::Open,
            labels: HashSet::new(),
            replies: Vec::new(),
            reactions: std::collections::HashMap::new(),
            resolved_threads: HashSet::new(),
            created_at_ms,
            updated_at_ms: created_at_ms,
        }
    }

    /// Apply a change to update the discussion state.
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
            CobOperation::CreateDiscussion { title, body, labels } => {
                self.title = title.clone();
                self.body = body.clone();
                for label in labels {
                    self.labels.insert(label.clone());
                }
            }

            CobOperation::Reply { body, parent_reply } => {
                self.replies.push(DiscussionReply {
                    author: *author.as_bytes(),
                    body: body.clone(),
                    parent_reply: *parent_reply,
                    timestamp_ms,
                    change_hash: *change_hash.as_bytes(),
                });
            }

            CobOperation::ResolveThread { reply_hash } => {
                self.resolved_threads.insert(*reply_hash);
            }

            CobOperation::UnresolveThread { reply_hash } => {
                self.resolved_threads.remove(reply_hash);
            }

            CobOperation::LockDiscussion => {
                self.state = DiscussionState::Locked;
            }

            CobOperation::UnlockDiscussion => {
                // Unlock returns to Open
                if self.state.is_locked() {
                    self.state = DiscussionState::Open;
                }
            }

            CobOperation::Comment { body } => {
                // Treat generic Comment as a top-level reply on discussions
                self.replies.push(DiscussionReply {
                    author: *author.as_bytes(),
                    body: body.clone(),
                    parent_reply: None,
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
                self.state = DiscussionState::Closed { reason: reason.clone() };
            }

            CobOperation::Reopen => {
                // Can reopen from Closed or Locked
                if !self.state.is_open() {
                    self.state = DiscussionState::Open;
                }
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

            CobOperation::MergeHeads { .. } => {
                // Merge commits don't directly modify discussion state
            }

            // Non-discussion operations are ignored
            CobOperation::CreateIssue { .. }
            | CobOperation::CreatePatch { .. }
            | CobOperation::UpdatePatch { .. }
            | CobOperation::Merge { .. }
            | CobOperation::Approve { .. }
            | CobOperation::RequestChanges { .. }
            | CobOperation::Assign { .. }
            | CobOperation::Unassign { .. } => {}
        }
    }

    /// Get the number of replies.
    pub fn reply_count(&self) -> usize {
        self.replies.len()
    }

    /// Check if a specific thread is resolved.
    pub fn is_thread_resolved(&self, reply_hash: &[u8; 32]) -> bool {
        self.resolved_threads.contains(reply_hash)
    }

    /// Get the number of resolved threads.
    pub fn resolved_thread_count(&self) -> usize {
        self.resolved_threads.len()
    }

    /// Apply field resolutions from a MergeHeads operation.
    pub fn apply_field_resolutions(
        &mut self,
        resolutions: &[super::change::FieldResolution],
        field_values: &std::collections::HashMap<([u8; 32], &str), DiscussionScalarFieldValue>,
    ) {
        for resolution in resolutions {
            let key = (resolution.selected, resolution.field.as_str());
            if let Some(value) = field_values.get(&key) {
                match value {
                    DiscussionScalarFieldValue::Title(title) => {
                        self.title = title.clone();
                    }
                    DiscussionScalarFieldValue::Body(body) => {
                        self.body = body.clone();
                    }
                    DiscussionScalarFieldValue::State(state) => {
                        self.state = state.clone();
                    }
                }
            }
        }
    }
}

/// Scalar field values that can be resolved during merge conflicts.
#[derive(Debug, Clone)]
pub enum DiscussionScalarFieldValue {
    /// Title field value.
    Title(String),
    /// Body field value.
    Body(String),
    /// State field value.
    State(DiscussionState),
}

#[cfg(test)]
mod tests {
    use aspen_core::hlc::create_hlc;

    use super::*;
    use crate::cob::change::CobOperation;

    fn test_key() -> PublicKey {
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        secret.public()
    }

    fn test_timestamp(hlc: &aspen_core::hlc::HLC) -> SerializableTimestamp {
        SerializableTimestamp::from(hlc.new_timestamp())
    }

    #[test]
    fn test_discussion_creation() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let mut discussion = Discussion::default();

        discussion.apply_change(
            blake3::hash(b"create"),
            &author,
            &test_timestamp(&hlc),
            &CobOperation::CreateDiscussion {
                title: "RFC: New API".to_string(),
                body: "Let's discuss the v2 API design".to_string(),
                labels: vec!["rfc".to_string()],
            },
        );

        assert_eq!(discussion.title, "RFC: New API");
        assert_eq!(discussion.body, "Let's discuss the v2 API design");
        assert!(discussion.state.is_open());
        assert!(discussion.labels.contains("rfc"));
        assert_eq!(discussion.reply_count(), 0);
    }

    #[test]
    fn test_discussion_replies() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let mut discussion = Discussion::new("Test".to_string(), "Body".to_string(), 0);

        // Top-level reply
        let reply1_hash = blake3::hash(b"reply1");
        discussion.apply_change(reply1_hash, &author, &test_timestamp(&hlc), &CobOperation::Reply {
            body: "I agree".to_string(),
            parent_reply: None,
        });

        assert_eq!(discussion.reply_count(), 1);
        assert!(discussion.replies[0].parent_reply.is_none());

        // Threaded reply
        discussion.apply_change(blake3::hash(b"reply2"), &author, &test_timestamp(&hlc), &CobOperation::Reply {
            body: "Can you elaborate?".to_string(),
            parent_reply: Some(*reply1_hash.as_bytes()),
        });

        assert_eq!(discussion.reply_count(), 2);
        assert_eq!(discussion.replies[1].parent_reply, Some(*reply1_hash.as_bytes()));
    }

    #[test]
    fn test_discussion_thread_resolution() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let reply_hash = *blake3::hash(b"reply1").as_bytes();
        let mut discussion = Discussion::new("Test".to_string(), "Body".to_string(), 0);

        // Resolve thread
        discussion.apply_change(
            blake3::hash(b"resolve"),
            &author,
            &test_timestamp(&hlc),
            &CobOperation::ResolveThread { reply_hash },
        );

        assert!(discussion.is_thread_resolved(&reply_hash));
        assert_eq!(discussion.resolved_thread_count(), 1);

        // Unresolve thread
        discussion.apply_change(
            blake3::hash(b"unresolve"),
            &author,
            &test_timestamp(&hlc),
            &CobOperation::UnresolveThread { reply_hash },
        );

        assert!(!discussion.is_thread_resolved(&reply_hash));
        assert_eq!(discussion.resolved_thread_count(), 0);
    }

    #[test]
    fn test_discussion_locking() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let mut discussion = Discussion::new("Test".to_string(), "Body".to_string(), 0);

        // Lock
        discussion.apply_change(blake3::hash(b"lock"), &author, &test_timestamp(&hlc), &CobOperation::LockDiscussion);

        assert!(discussion.state.is_locked());
        assert!(!discussion.state.allows_replies());

        // Unlock
        discussion.apply_change(
            blake3::hash(b"unlock"),
            &author,
            &test_timestamp(&hlc),
            &CobOperation::UnlockDiscussion,
        );

        assert!(discussion.state.is_open());
        assert!(discussion.state.allows_replies());
    }

    #[test]
    fn test_discussion_close_and_reopen() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let mut discussion = Discussion::new("Test".to_string(), "Body".to_string(), 0);

        // Close
        discussion.apply_change(blake3::hash(b"close"), &author, &test_timestamp(&hlc), &CobOperation::Close {
            reason: Some("Decided on option A".to_string()),
        });

        assert!(discussion.state.is_closed());

        // Reopen
        discussion.apply_change(blake3::hash(b"reopen"), &author, &test_timestamp(&hlc), &CobOperation::Reopen);

        assert!(discussion.state.is_open());
    }

    #[test]
    fn test_discussion_reopen_from_locked() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let mut discussion = Discussion::new("Test".to_string(), "Body".to_string(), 0);

        // Lock then reopen
        discussion.apply_change(blake3::hash(b"lock"), &author, &test_timestamp(&hlc), &CobOperation::LockDiscussion);
        discussion.apply_change(blake3::hash(b"reopen"), &author, &test_timestamp(&hlc), &CobOperation::Reopen);

        assert!(discussion.state.is_open());
    }

    #[test]
    fn test_discussion_labels_and_reactions() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let mut discussion = Discussion::new("Test".to_string(), "Body".to_string(), 0);

        // Add labels
        discussion.apply_change(blake3::hash(b"label1"), &author, &test_timestamp(&hlc), &CobOperation::AddLabel {
            label: "rfc".to_string(),
        });
        discussion.apply_change(blake3::hash(b"label2"), &author, &test_timestamp(&hlc), &CobOperation::AddLabel {
            label: "api".to_string(),
        });

        assert_eq!(discussion.labels.len(), 2);

        // Remove label
        discussion.apply_change(blake3::hash(b"remove"), &author, &test_timestamp(&hlc), &CobOperation::RemoveLabel {
            label: "api".to_string(),
        });

        assert_eq!(discussion.labels.len(), 1);

        // React
        discussion.apply_change(blake3::hash(b"react"), &author, &test_timestamp(&hlc), &CobOperation::React {
            emoji: "+1".to_string(),
        });

        assert!(discussion.reactions.contains_key("+1"));

        // Unreact
        discussion.apply_change(blake3::hash(b"unreact"), &author, &test_timestamp(&hlc), &CobOperation::Unreact {
            emoji: "+1".to_string(),
        });

        assert!(!discussion.reactions.contains_key("+1"));
    }

    #[test]
    fn test_discussion_edit_title_and_body() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let mut discussion = Discussion::new("Original".to_string(), "Original body".to_string(), 0);

        discussion.apply_change(
            blake3::hash(b"edit_title"),
            &author,
            &test_timestamp(&hlc),
            &CobOperation::EditTitle {
                title: "Updated Title".to_string(),
            },
        );

        discussion.apply_change(blake3::hash(b"edit_body"), &author, &test_timestamp(&hlc), &CobOperation::EditBody {
            body: "Updated body".to_string(),
        });

        assert_eq!(discussion.title, "Updated Title");
        assert_eq!(discussion.body, "Updated body");
    }

    #[test]
    fn test_discussion_updated_at() {
        let author = test_key();
        let hlc = create_hlc("test-node");
        let mut discussion = Discussion::new("Test".to_string(), "Body".to_string(), 1000);

        assert_eq!(discussion.updated_at_ms, 1000);

        discussion.apply_change(blake3::hash(b"reply"), &author, &test_timestamp(&hlc), &CobOperation::Reply {
            body: "A reply".to_string(),
            parent_reply: None,
        });

        assert!(discussion.updated_at_ms > 1000);
    }
}
