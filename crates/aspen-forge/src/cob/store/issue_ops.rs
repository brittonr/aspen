//! Issue operations for `CobStore`.

use std::collections::HashMap;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;

use super::super::change::CobChange;
use super::super::change::CobOperation;
use super::super::change::CobType;
use super::super::change::FieldResolution;
use super::super::issue::Issue;
use super::CobStore;
use crate::constants::MAX_LABEL_LENGTH_BYTES;
use crate::constants::MAX_LABELS;
use crate::constants::MAX_TITLE_LENGTH_BYTES;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;

impl<B: BlobStore, K: KeyValueStore + ?Sized> CobStore<B, K> {
    /// Create a new issue.
    ///
    /// # Errors
    ///
    /// - `ForgeError::InvalidCobChange` if title or labels exceed limits
    pub async fn create_issue(
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

        // Generate a unique COB ID from the content
        let cob_id = blake3::hash(
            &[
                repo_id.0.as_slice(),
                title.as_bytes(),
                &(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or(std::time::Duration::ZERO)
                    .as_nanos() as u64)
                    .to_le_bytes(),
            ]
            .concat(),
        );

        let op = CobOperation::CreateIssue { title, body, labels };

        let change = CobChange::root(CobType::Issue, cob_id, op);
        self.store_change(repo_id, change).await?;

        // Return the COB ID, not the change hash
        Ok(cob_id)
    }

    /// Add a comment to an issue.
    pub async fn add_comment(
        &self,
        repo_id: &RepoId,
        issue_id: &blake3::Hash,
        body: impl Into<String>,
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Issue, issue_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Issue.to_string(),
                cob_id: hex::encode(issue_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Issue, *issue_id, heads, CobOperation::Comment { body: body.into() });

        self.store_change(repo_id, change).await
    }

    /// Close an issue.
    pub async fn close_issue(
        &self,
        repo_id: &RepoId,
        issue_id: &blake3::Hash,
        reason: Option<String>,
    ) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Issue, issue_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Issue.to_string(),
                cob_id: hex::encode(issue_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Issue, *issue_id, heads, CobOperation::Close { reason });

        self.store_change(repo_id, change).await
    }

    /// Reopen a closed issue.
    pub async fn reopen_issue(&self, repo_id: &RepoId, issue_id: &blake3::Hash) -> ForgeResult<blake3::Hash> {
        let heads = self.get_heads(repo_id, CobType::Issue, issue_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Issue.to_string(),
                cob_id: hex::encode(issue_id.as_bytes()),
            });
        }

        let change = CobChange::new(CobType::Issue, *issue_id, heads, CobOperation::Reopen);

        self.store_change(repo_id, change).await
    }

    /// Resolve the current state of an issue by walking its change DAG.
    ///
    /// This method walks the change DAG, topologically sorts changes, and applies
    /// them in order. MergeHeads operations with field resolutions are tracked and
    /// applied at the end to resolve scalar field conflicts (title, body, state).
    ///
    /// # Errors
    ///
    /// - `ForgeError::CobNotFound` if the issue doesn't exist
    /// - `ForgeError::TooManyChanges` if the DAG is too large
    pub async fn resolve_issue(&self, repo_id: &RepoId, issue_id: &blake3::Hash) -> ForgeResult<Issue> {
        let heads = self.get_heads(repo_id, CobType::Issue, issue_id).await?;

        if heads.is_empty() {
            return Err(ForgeError::CobNotFound {
                cob_type: CobType::Issue.to_string(),
                cob_id: hex::encode(issue_id.as_bytes()),
            });
        }

        // Collect all changes by walking the DAG
        let changes = self.collect_changes(heads).await?;

        // Topologically sort changes (parents before children)
        let sorted = self.topological_sort(&changes)?;

        // Track scalar field values set by each change for MergeHeads resolution
        let mut field_values: HashMap<([u8; 32], &str), super::super::issue::IssueScalarFieldValue> = HashMap::new();

        // Track the most recent MergeHeads resolutions (later ones override earlier)
        let mut final_resolutions: Vec<FieldResolution> = Vec::new();

        // Apply changes in order
        let mut issue = Issue::default();
        for (hash, signed) in &sorted {
            // Track scalar field values before applying
            match &signed.payload.op {
                CobOperation::CreateIssue { title, body, .. } => {
                    field_values.insert(
                        (*hash.as_bytes(), "title"),
                        super::super::issue::IssueScalarFieldValue::Title(title.clone()),
                    );
                    field_values.insert(
                        (*hash.as_bytes(), "body"),
                        super::super::issue::IssueScalarFieldValue::Body(body.clone()),
                    );
                }
                CobOperation::EditTitle { title } => {
                    field_values.insert(
                        (*hash.as_bytes(), "title"),
                        super::super::issue::IssueScalarFieldValue::Title(title.clone()),
                    );
                }
                CobOperation::EditBody { body } => {
                    field_values.insert(
                        (*hash.as_bytes(), "body"),
                        super::super::issue::IssueScalarFieldValue::Body(body.clone()),
                    );
                }
                CobOperation::Close { reason } => {
                    field_values.insert(
                        (*hash.as_bytes(), "state"),
                        super::super::issue::IssueScalarFieldValue::State(super::super::issue::IssueState::Closed {
                            reason: reason.clone(),
                        }),
                    );
                }
                CobOperation::Reopen => {
                    field_values.insert(
                        (*hash.as_bytes(), "state"),
                        super::super::issue::IssueScalarFieldValue::State(super::super::issue::IssueState::Open),
                    );
                }
                CobOperation::MergeHeads { resolutions, .. } => {
                    // Collect field resolutions from MergeHeads operations
                    // Later MergeHeads operations override earlier ones
                    final_resolutions = resolutions.clone();
                }
                _ => {}
            }

            issue.apply_change(*hash, &signed.author, &signed.hlc_timestamp, &signed.payload.op);
        }

        // Apply field resolutions from the final MergeHeads operation
        if !final_resolutions.is_empty() {
            issue.apply_field_resolutions(&final_resolutions, &field_values);
        }

        Ok(issue)
    }
}
