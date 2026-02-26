//! Patch operations for `CobStore`.

use std::collections::HashMap;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;

use super::super::change::CobChange;
use super::super::change::CobOperation;
use super::super::change::CobType;
use super::super::change::FieldResolution;
use super::CobStore;
use crate::constants::MAX_TITLE_LENGTH_BYTES;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;

impl<B: BlobStore, K: KeyValueStore + ?Sized> CobStore<B, K> {
    /// Create a new patch.
    ///
    /// # Errors
    ///
    /// - `ForgeError::InvalidCobChange` if title exceeds limits
    pub async fn create_patch(
        &self,
        repo_id: &RepoId,
        title: impl Into<String>,
        description: impl Into<String>,
        base: blake3::Hash,
        head: blake3::Hash,
    ) -> ForgeResult<blake3::Hash> {
        let title = title.into();
        let description = description.into();

        // Validate
        if title.len() as u32 > MAX_TITLE_LENGTH_BYTES {
            return Err(ForgeError::InvalidCobChange {
                message: format!("title too long: {} > {}", title.len(), MAX_TITLE_LENGTH_BYTES),
            });
        }

        // Generate a unique COB ID from the content
        let cob_id = blake3::hash(
            &[
                repo_id.0.as_slice(),
                title.as_bytes(),
                base.as_bytes(),
                head.as_bytes(),
                &(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::ZERO)
                    .as_nanos() as u64)
                    .to_le_bytes(),
            ]
            .concat(),
        );

        let op = CobOperation::CreatePatch {
            title,
            description,
            base: *base.as_bytes(),
            head: *head.as_bytes(),
        };

        let change = CobChange::root(CobType::Patch, cob_id, op);
        self.store_change(repo_id, change).await?;

        // Return the COB ID, not the change hash
        Ok(cob_id)
    }

    /// Track scalar field values from a change operation for MergeHeads resolution.
    fn resolve_patch_track_field_value<'a>(
        hash: &blake3::Hash,
        op: &'a CobOperation,
        field_values: &mut HashMap<([u8; 32], &'a str), super::super::patch::ScalarFieldValue>,
        final_resolutions: &mut Vec<FieldResolution>,
    ) {
        match op {
            CobOperation::CreatePatch { title, description, .. } => {
                field_values
                    .insert((*hash.as_bytes(), "title"), super::super::patch::ScalarFieldValue::Title(title.clone()));
                field_values.insert(
                    (*hash.as_bytes(), "description"),
                    super::super::patch::ScalarFieldValue::Description(description.clone()),
                );
            }
            CobOperation::EditTitle { title } => {
                field_values
                    .insert((*hash.as_bytes(), "title"), super::super::patch::ScalarFieldValue::Title(title.clone()));
            }
            CobOperation::EditBody { body } => {
                field_values.insert(
                    (*hash.as_bytes(), "description"),
                    super::super::patch::ScalarFieldValue::Description(body.clone()),
                );
            }
            CobOperation::Merge { commit } => {
                field_values.insert(
                    (*hash.as_bytes(), "state"),
                    super::super::patch::ScalarFieldValue::State(super::super::patch::PatchState::Merged {
                        commit: *commit,
                    }),
                );
            }
            CobOperation::Close { reason } => {
                field_values.insert(
                    (*hash.as_bytes(), "state"),
                    super::super::patch::ScalarFieldValue::State(super::super::patch::PatchState::Closed {
                        reason: reason.clone(),
                    }),
                );
            }
            CobOperation::Reopen => {
                field_values.insert(
                    (*hash.as_bytes(), "state"),
                    super::super::patch::ScalarFieldValue::State(super::super::patch::PatchState::Open),
                );
            }
            CobOperation::MergeHeads { resolutions, .. } => {
                *final_resolutions = resolutions.clone();
            }
            _ => {}
        }
    }

    /// Resolve the current state of a patch by walking its change DAG.
    ///
    /// This method walks the change DAG, topologically sorts changes, and applies
    /// them in order. MergeHeads operations with field resolutions are tracked and
    /// applied at the end to resolve scalar field conflicts (title, description, state).
    ///
    /// # Errors
    ///
    /// - `ForgeError::CobNotFound` if the patch doesn't exist
    /// - `ForgeError::TooManyChanges` if the DAG is too large
    pub async fn resolve_patch(
        &self,
        repo_id: &RepoId,
        patch_id: &blake3::Hash,
    ) -> ForgeResult<super::super::patch::Patch> {
        let heads = self.get_heads(repo_id, CobType::Patch, patch_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Patch.to_string(),
                cob_id: hex::encode(patch_id.as_bytes()),
            });
        }

        let changes = self.collect_changes(heads).await?;
        let sorted = self.topological_sort(&changes)?;

        let mut field_values: HashMap<([u8; 32], &str), super::super::patch::ScalarFieldValue> = HashMap::new();
        let mut final_resolutions: Vec<FieldResolution> = Vec::new();
        let mut patch = super::super::patch::Patch::default();

        for (hash, signed) in &sorted {
            Self::resolve_patch_track_field_value(hash, &signed.payload.op, &mut field_values, &mut final_resolutions);
            patch.apply_change(*hash, &signed.author, &signed.hlc_timestamp, &signed.payload.op);
        }

        if !final_resolutions.is_empty() {
            patch.apply_field_resolutions(&final_resolutions, &field_values);
        }

        Ok(patch)
    }

    /// Update a patch to a new head commit.
    pub async fn update_patch(
        &self,
        repo_id: &RepoId,
        patch_id: &blake3::Hash,
        head: blake3::Hash,
        message: Option<String>,
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Patch, patch_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Patch.to_string(),
                cob_id: hex::encode(patch_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Patch, *patch_id, heads, CobOperation::UpdatePatch {
            head: *head.as_bytes(),
            message,
        });

        self.store_change(repo_id, change).await
    }

    /// Approve a patch.
    pub async fn approve_patch(
        &self,
        repo_id: &RepoId,
        patch_id: &blake3::Hash,
        commit: blake3::Hash,
        message: Option<String>,
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Patch, patch_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Patch.to_string(),
                cob_id: hex::encode(patch_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Patch, *patch_id, heads, CobOperation::Approve {
            commit: *commit.as_bytes(),
            message,
        });

        self.store_change(repo_id, change).await
    }

    /// Merge a patch.
    pub async fn merge_patch(
        &self,
        repo_id: &RepoId,
        patch_id: &blake3::Hash,
        commit: blake3::Hash,
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Patch, patch_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Patch.to_string(),
                cob_id: hex::encode(patch_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Patch, *patch_id, heads, CobOperation::Merge {
            commit: *commit.as_bytes(),
        });

        self.store_change(repo_id, change).await
    }

    /// Close a patch without merging.
    pub async fn close_patch(
        &self,
        repo_id: &RepoId,
        patch_id: &blake3::Hash,
        reason: Option<String>,
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Patch, patch_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Patch.to_string(),
                cob_id: hex::encode(patch_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Patch, *patch_id, heads, CobOperation::Close { reason });

        self.store_change(repo_id, change).await
    }
}
