//! Discussion operations for `CobStore`.

use std::collections::HashMap;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;

use super::super::change::CobChange;
use super::super::change::CobOperation;
use super::super::change::CobType;
use super::super::change::FieldResolution;
use super::super::discussion::Discussion;
use super::super::discussion::DiscussionScalarFieldValue;
use super::super::discussion::DiscussionState;
use super::CobStore;
use crate::constants::MAX_DISCUSSION_REPLY_BODY_BYTES;
use crate::constants::MAX_LABEL_LENGTH_BYTES;
use crate::constants::MAX_LABELS;
use crate::constants::MAX_TITLE_LENGTH_BYTES;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;
use crate::types::SignedObject;

impl<B: BlobStore, K: KeyValueStore + ?Sized> CobStore<B, K> {
    /// Create a new discussion.
    ///
    /// # Errors
    ///
    /// - `ForgeError::InvalidCobChange` if title or labels exceed limits
    pub async fn create_discussion(
        &self,
        repo_id: &RepoId,
        title: impl Into<String>,
        body: impl Into<String>,
        labels: Vec<String>,
    ) -> ForgeResult<blake3::Hash> {
        let title = title.into();
        let body = body.into();

        // Validate
        if title.len() as u32 > MAX_TITLE_LENGTH_BYTES {
            return Err(ForgeError::InvalidCobChange {
                message: format!("title too long: {} > {}", title.len(), MAX_TITLE_LENGTH_BYTES),
            });
        }

        if labels.len() as u32 > MAX_LABELS {
            return Err(ForgeError::InvalidCobChange {
                message: format!("too many labels: {} > {}", labels.len(), MAX_LABELS),
            });
        }

        for label in &labels {
            if label.len() as u32 > MAX_LABEL_LENGTH_BYTES {
                return Err(ForgeError::InvalidCobChange {
                    message: format!("label too long: {} > {}", label.len(), MAX_LABEL_LENGTH_BYTES),
                });
            }
        }

        let cob_id = blake3::hash(
            &[
                repo_id.0.as_slice(),
                b"discussion:",
                title.as_bytes(),
                &(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::ZERO)
                    .as_nanos() as u64)
                    .to_le_bytes(),
            ]
            .concat(),
        );

        let op = CobOperation::CreateDiscussion { title, body, labels };
        let change = CobChange::root(CobType::Discussion, cob_id, op);
        self.store_change(repo_id, change).await?;

        Ok(cob_id)
    }

    /// Add a reply to a discussion.
    ///
    /// # Errors
    ///
    /// - `ForgeError::CobNotFound` if the discussion doesn't exist
    /// - `ForgeError::DiscussionLocked` if the discussion is locked
    /// - `ForgeError::InvalidCobChange` if the reply body exceeds limits
    pub async fn add_reply(
        &self,
        repo_id: &RepoId,
        discussion_id: &blake3::Hash,
        body: impl Into<String>,
        parent_reply: Option<[u8; 32]>,
    ) -> ForgeResult<blake3::Hash> {
        let body = body.into();

        if body.len() as u32 > MAX_DISCUSSION_REPLY_BODY_BYTES {
            return Err(ForgeError::InvalidCobChange {
                message: format!("reply body too long: {} > {}", body.len(), MAX_DISCUSSION_REPLY_BODY_BYTES),
            });
        }

        let heads = self.get_heads(repo_id, CobType::Discussion, discussion_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Discussion.to_string(),
                cob_id: hex::encode(discussion_id.as_bytes()),
            });
        }

        // Check if discussion is locked
        let discussion = self.resolve_discussion(repo_id, discussion_id).await?;
        if discussion.state.is_locked() {
            return Err(ForgeError::DiscussionLocked {
                discussion_id: hex::encode(discussion_id.as_bytes()),
            });
        }

        let change =
            CobChange::new(CobType::Discussion, *discussion_id, heads, CobOperation::Reply { body, parent_reply });

        self.store_change(repo_id, change).await
    }

    /// Resolve a thread in a discussion.
    pub async fn resolve_thread(
        &self,
        repo_id: &RepoId,
        discussion_id: &blake3::Hash,
        reply_hash: [u8; 32],
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Discussion, discussion_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Discussion.to_string(),
                cob_id: hex::encode(discussion_id.as_bytes()),
            });
        }

        let change =
            CobChange::new(CobType::Discussion, *discussion_id, heads, CobOperation::ResolveThread { reply_hash });

        self.store_change(repo_id, change).await
    }

    /// Unresolve a thread in a discussion.
    pub async fn unresolve_thread(
        &self,
        repo_id: &RepoId,
        discussion_id: &blake3::Hash,
        reply_hash: [u8; 32],
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Discussion, discussion_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Discussion.to_string(),
                cob_id: hex::encode(discussion_id.as_bytes()),
            });
        }

        let change =
            CobChange::new(CobType::Discussion, *discussion_id, heads, CobOperation::UnresolveThread { reply_hash });

        self.store_change(repo_id, change).await
    }

    /// Lock a discussion.
    pub async fn lock_discussion(&self, repo_id: &RepoId, discussion_id: &blake3::Hash) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Discussion, discussion_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Discussion.to_string(),
                cob_id: hex::encode(discussion_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Discussion, *discussion_id, heads, CobOperation::LockDiscussion);

        self.store_change(repo_id, change).await
    }

    /// Unlock a discussion.
    pub async fn unlock_discussion(&self, repo_id: &RepoId, discussion_id: &blake3::Hash) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Discussion, discussion_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Discussion.to_string(),
                cob_id: hex::encode(discussion_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Discussion, *discussion_id, heads, CobOperation::UnlockDiscussion);

        self.store_change(repo_id, change).await
    }

    /// Close a discussion.
    pub async fn close_discussion(
        &self,
        repo_id: &RepoId,
        discussion_id: &blake3::Hash,
        reason: Option<String>,
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Discussion, discussion_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Discussion.to_string(),
                cob_id: hex::encode(discussion_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Discussion, *discussion_id, heads, CobOperation::Close { reason });

        self.store_change(repo_id, change).await
    }

    /// Reopen a closed discussion.
    pub async fn reopen_discussion(&self, repo_id: &RepoId, discussion_id: &blake3::Hash) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Discussion, discussion_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Discussion.to_string(),
                cob_id: hex::encode(discussion_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Discussion, *discussion_id, heads, CobOperation::Reopen);

        self.store_change(repo_id, change).await
    }

    /// Resolve the current state of a discussion by walking its change DAG.
    pub async fn resolve_discussion(&self, repo_id: &RepoId, discussion_id: &blake3::Hash) -> ForgeResult<Discussion> {
        let heads = self.get_heads(repo_id, CobType::Discussion, discussion_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Discussion.to_string(),
                cob_id: hex::encode(discussion_id.as_bytes()),
            });
        }

        let changes = self.collect_changes(heads).await?;
        let sorted = self.topological_sort(&changes)?;

        let mut field_values: HashMap<([u8; 32], &str), DiscussionScalarFieldValue> = HashMap::new();
        let mut final_resolutions: Vec<FieldResolution> = Vec::new();

        let mut discussion = Discussion::default();
        for (hash, signed) in &sorted {
            Self::resolve_discussion_track_field_values(hash, signed, &mut field_values, &mut final_resolutions);
            discussion.apply_change(*hash, &signed.author, &signed.hlc_timestamp, &signed.payload.op);
        }

        if !final_resolutions.is_empty() {
            discussion.apply_field_resolutions(&final_resolutions, &field_values);
        }

        Ok(discussion)
    }

    /// Track scalar field values from a change for MergeHeads resolution.
    fn resolve_discussion_track_field_values<'a>(
        hash: &blake3::Hash,
        signed: &'a SignedObject<CobChange>,
        field_values: &mut HashMap<([u8; 32], &'a str), DiscussionScalarFieldValue>,
        final_resolutions: &mut Vec<FieldResolution>,
    ) {
        match &signed.payload.op {
            CobOperation::CreateDiscussion { title, body, .. } => {
                field_values.insert((*hash.as_bytes(), "title"), DiscussionScalarFieldValue::Title(title.clone()));
                field_values.insert((*hash.as_bytes(), "body"), DiscussionScalarFieldValue::Body(body.clone()));
            }
            CobOperation::EditTitle { title } => {
                field_values.insert((*hash.as_bytes(), "title"), DiscussionScalarFieldValue::Title(title.clone()));
            }
            CobOperation::EditBody { body } => {
                field_values.insert((*hash.as_bytes(), "body"), DiscussionScalarFieldValue::Body(body.clone()));
            }
            CobOperation::Close { reason } => {
                field_values.insert(
                    (*hash.as_bytes(), "state"),
                    DiscussionScalarFieldValue::State(DiscussionState::Closed { reason: reason.clone() }),
                );
            }
            CobOperation::Reopen => {
                field_values
                    .insert((*hash.as_bytes(), "state"), DiscussionScalarFieldValue::State(DiscussionState::Open));
            }
            CobOperation::LockDiscussion => {
                field_values
                    .insert((*hash.as_bytes(), "state"), DiscussionScalarFieldValue::State(DiscussionState::Locked));
            }
            CobOperation::MergeHeads { resolutions, .. } => {
                *final_resolutions = resolutions.clone();
            }
            _ => {}
        }
    }
}
