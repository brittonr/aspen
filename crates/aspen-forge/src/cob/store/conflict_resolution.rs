//! Conflict detection and resolution for `CobStore`.

use std::collections::HashMap;
use std::collections::HashSet;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;

use super::super::change::CobChange;
use super::super::change::CobOperation;
use super::super::change::CobType;
use super::super::change::FieldResolution;
use super::super::change::MergeStrategy;
use super::CobStore;
use super::types::ConflictReport;
use super::types::ConflictingValue;
use super::types::FieldConflict;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;

impl<B: BlobStore, K: KeyValueStore + ?Sized> CobStore<B, K> {
    /// Check if a COB has conflicts (multiple divergent heads).
    ///
    /// Returns `true` if there are multiple heads with conflicting scalar field values
    /// (title, body, state). Returns `false` if there's only one head or if multiple
    /// heads don't have conflicting values (e.g., only added comments/labels which auto-merge).
    ///
    /// # Errors
    ///
    /// - `ForgeError::CobNotFound` if the COB doesn't exist
    pub async fn has_conflicts(&self, repo_id: &RepoId, cob_type: CobType, cob_id: &blake3::Hash) -> ForgeResult<bool> {
        let heads = self.get_heads(repo_id, cob_type, cob_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: cob_type.to_string(),
                cob_id: hex::encode(cob_id.as_bytes()),
            });
        }

        // Single head = no conflicts possible
        if heads.len() == 1 {
            return Ok(false);
        }

        // Multiple heads - check for conflicting scalar values
        let report = self.get_conflicts_internal(repo_id, cob_type, cob_id, &heads).await?;
        Ok(!report.field_conflicts.is_empty())
    }

    /// Get detailed conflict information for a COB.
    ///
    /// Returns a `ConflictReport` describing which fields have conflicting values
    /// and what those values are. This information is needed to create explicit
    /// resolutions for the `merge_heads` method.
    ///
    /// # Errors
    ///
    /// - `ForgeError::CobNotFound` if the COB doesn't exist
    pub async fn get_conflicts(
        &self,
        repo_id: &RepoId,
        cob_type: CobType,
        cob_id: &blake3::Hash,
    ) -> ForgeResult<ConflictReport> {
        let heads = self.get_heads(repo_id, cob_type, cob_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: cob_type.to_string(),
                cob_id: hex::encode(cob_id.as_bytes()),
            });
        }

        self.get_conflicts_internal(repo_id, cob_type, cob_id, &heads).await
    }

    /// Create a merge change that resolves conflicts between divergent heads.
    ///
    /// This creates a new change with all current heads as parents, applying the
    /// specified merge strategy and resolutions. After this change is stored,
    /// the COB will have a single head.
    ///
    /// # Git-like Behavior
    ///
    /// - **Auto-merge**: Labels, reactions, assignees are unioned; comments are appended
    /// - **Conflicts**: Divergent title, body, or state require explicit resolution
    /// - With `MergeStrategy::Explicit`, all conflicting fields must have resolutions
    /// - With `MergeStrategy::LastWriteWins`, conflicts are resolved by timestamp
    ///
    /// # Errors
    ///
    /// - `ForgeError::CobNotFound` if the COB doesn't exist
    /// - `ForgeError::InvalidCobChange` if explicit resolution is missing for conflicting fields
    pub async fn merge_heads(
        &self,
        repo_id: &RepoId,
        cob_type: CobType,
        cob_id: &blake3::Hash,
        strategy: MergeStrategy,
        resolutions: Vec<FieldResolution>,
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, cob_type, cob_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: cob_type.to_string(),
                cob_id: hex::encode(cob_id.as_bytes()),
            });
        }

        // If only one head, no merge needed
        if heads.len() == 1 {
            return Ok(heads[0]);
        }

        // Validate explicit resolutions cover all conflicting fields
        if strategy == MergeStrategy::Explicit {
            let report = self.get_conflicts_internal(repo_id, cob_type, cob_id, &heads).await?;

            let resolution_fields: HashSet<&str> = resolutions.iter().map(|r| r.field.as_str()).collect();

            for conflict in &report.field_conflicts {
                if !resolution_fields.contains(conflict.field.as_str()) {
                    return Err(ForgeError::InvalidCobChange {
                        message: format!("explicit resolution required for conflicting field: {}", conflict.field),
                    });
                }
            }
        }

        // Create merge change
        let change = CobChange::new(cob_type, *cob_id, heads, CobOperation::MergeHeads { strategy, resolutions });

        self.store_change(repo_id, change).await
    }

    /// Internal helper to detect conflicts between heads.
    pub(super) async fn get_conflicts_internal(
        &self,
        _repo_id: &RepoId,
        cob_type: CobType,
        cob_id: &blake3::Hash,
        heads: &[blake3::Hash],
    ) -> ForgeResult<ConflictReport> {
        // For each head, resolve to a point-in-time state and compare scalar fields
        let mut field_values: HashMap<String, Vec<ConflictingValue>> = HashMap::new();

        for head in heads {
            let signed = self.get_change(head).await?;

            // Extract scalar field values from the operation
            // We track the change that last modified each field
            match &signed.payload.op {
                CobOperation::CreateIssue { title, body, .. } => {
                    field_values.entry("title".to_string()).or_default().push(ConflictingValue {
                        change_hash: *head,
                        value: title.clone(),
                        hlc_timestamp: signed.hlc_timestamp.clone(),
                        author: signed.author,
                    });
                    field_values.entry("body".to_string()).or_default().push(ConflictingValue {
                        change_hash: *head,
                        value: body.clone(),
                        hlc_timestamp: signed.hlc_timestamp.clone(),
                        author: signed.author,
                    });
                }
                CobOperation::EditTitle { title } => {
                    field_values.entry("title".to_string()).or_default().push(ConflictingValue {
                        change_hash: *head,
                        value: title.clone(),
                        hlc_timestamp: signed.hlc_timestamp.clone(),
                        author: signed.author,
                    });
                }
                CobOperation::EditBody { body } => {
                    field_values.entry("body".to_string()).or_default().push(ConflictingValue {
                        change_hash: *head,
                        value: body.clone(),
                        hlc_timestamp: signed.hlc_timestamp.clone(),
                        author: signed.author,
                    });
                }
                CobOperation::Close { reason } => {
                    let state_str = match reason {
                        Some(r) => format!("closed:{}", r),
                        None => "closed".to_string(),
                    };
                    field_values.entry("state".to_string()).or_default().push(ConflictingValue {
                        change_hash: *head,
                        value: state_str,
                        hlc_timestamp: signed.hlc_timestamp.clone(),
                        author: signed.author,
                    });
                }
                CobOperation::Reopen => {
                    field_values.entry("state".to_string()).or_default().push(ConflictingValue {
                        change_hash: *head,
                        value: "open".to_string(),
                        hlc_timestamp: signed.hlc_timestamp.clone(),
                        author: signed.author,
                    });
                }
                CobOperation::CreatePatch { title, description, .. } => {
                    field_values.entry("title".to_string()).or_default().push(ConflictingValue {
                        change_hash: *head,
                        value: title.clone(),
                        hlc_timestamp: signed.hlc_timestamp.clone(),
                        author: signed.author,
                    });
                    field_values.entry("description".to_string()).or_default().push(ConflictingValue {
                        change_hash: *head,
                        value: description.clone(),
                        hlc_timestamp: signed.hlc_timestamp.clone(),
                        author: signed.author,
                    });
                }
                // Non-conflicting operations (auto-merge)
                CobOperation::Comment { .. }
                | CobOperation::AddLabel { .. }
                | CobOperation::RemoveLabel { .. }
                | CobOperation::React { .. }
                | CobOperation::Unreact { .. }
                | CobOperation::Assign { .. }
                | CobOperation::Unassign { .. }
                | CobOperation::UpdatePatch { .. }
                | CobOperation::Merge { .. }
                | CobOperation::Approve { .. }
                | CobOperation::RequestChanges { .. }
                | CobOperation::MergeHeads { .. } => {
                    // These don't create scalar field conflicts
                }
            }
        }

        // Identify actual conflicts (different values for the same field)
        let field_conflicts: Vec<FieldConflict> = field_values
            .into_iter()
            .filter_map(|(field, values)| {
                // Collect unique values
                let unique_values: HashSet<&str> = values.iter().map(|v| v.value.as_str()).collect();
                if unique_values.len() > 1 {
                    Some(FieldConflict { field, values })
                } else {
                    None
                }
            })
            .collect();

        Ok(ConflictReport {
            cob_type,
            cob_id: *cob_id,
            heads: heads.to_vec(),
            field_conflicts,
        })
    }
}
