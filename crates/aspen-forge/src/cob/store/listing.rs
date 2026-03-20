//! COB listing operations for `CobStore`.

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;

use super::super::change::CobType;
use super::CobStore;
use crate::constants::KV_PREFIX_COB_HEADS;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::identity::RepoId;

impl<B: BlobStore, K: KeyValueStore + ?Sized> CobStore<B, K> {
    /// List all COBs of a given type in a repository.
    ///
    /// Scans the KV store for keys matching the COB heads prefix for the
    /// specified repository and type, then extracts the COB IDs.
    ///
    /// # Errors
    ///
    /// - `ForgeError::KvOperation` if the scan fails (except NotFound, which returns empty vec)
    pub async fn list_cobs(&self, repo_id: &RepoId, cob_type: CobType) -> ForgeResult<Vec<blake3::Hash>> {
        let prefix = format!("{}{}:{}:", KV_PREFIX_COB_HEADS, repo_id.to_hex(), cob_type.as_str());

        let scan_result = match self
            .kv
            .scan(aspen_core::ScanRequest {
                prefix: prefix.clone(),
                limit_results: None,
                continuation_token: None,
            })
            .await
        {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                // No keys found means no COBs of this type
                return Ok(vec![]);
            }
            Err(e) => return Err(ForgeError::from(e)),
        };

        // Extract COB IDs from keys
        // Key format: forge:cob:heads:{repo_id}:{cob_type}:{cob_id}
        let cob_ids: Vec<blake3::Hash> = scan_result
            .entries
            .iter()
            .filter_map(|entry| {
                // Strip the prefix to get the COB ID (hex-encoded)
                let cob_id_hex = entry.key.strip_prefix(&prefix)?;
                let bytes = hex::decode(cob_id_hex).ok()?;
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Some(blake3::Hash::from_bytes(arr))
                } else {
                    None
                }
            })
            .collect();

        Ok(cob_ids)
    }

    /// List all issues in a repository.
    ///
    /// Convenience method that calls `list_cobs` with `CobType::Issue`.
    ///
    /// # Errors
    ///
    /// - `ForgeError::KvOperation` if the scan fails
    pub async fn list_issues(&self, repo_id: &RepoId) -> ForgeResult<Vec<blake3::Hash>> {
        self.list_cobs(repo_id, CobType::Issue).await
    }

    /// List all patches in a repository.
    ///
    /// Convenience method that calls `list_cobs` with `CobType::Patch`.
    ///
    /// # Errors
    ///
    /// - `ForgeError::KvOperation` if the scan fails
    pub async fn list_patches(&self, repo_id: &RepoId) -> ForgeResult<Vec<blake3::Hash>> {
        self.list_cobs(repo_id, CobType::Patch).await
    }

    /// List all discussions in a repository.
    ///
    /// Convenience method that calls `list_cobs` with `CobType::Discussion`.
    ///
    /// # Errors
    ///
    /// - `ForgeError::KvOperation` if the scan fails
    pub async fn list_discussions(&self, repo_id: &RepoId) -> ForgeResult<Vec<blake3::Hash>> {
        self.list_cobs(repo_id, CobType::Discussion).await
    }

    /// List discussions with state filtering.
    ///
    /// Resolves each discussion to check its state and returns only those
    /// matching the filter. Results are limited to `limit` entries.
    ///
    /// # Arguments
    ///
    /// * `repo_id` - Repository to list discussions from
    /// * `state_filter` - Optional state to filter by (None = all states)
    /// * `limit` - Maximum number of results (capped at 1000)
    pub async fn list_discussions_filtered(
        &self,
        repo_id: &RepoId,
        state_filter: Option<&super::super::discussion::DiscussionState>,
        limit: u32,
    ) -> ForgeResult<Vec<(blake3::Hash, super::super::discussion::Discussion)>> {
        let limit = limit.min(1000); // Tiger Style: bound results
        let cob_ids = self.list_discussions(repo_id).await?;

        let mut results = Vec::new();
        for cob_id in cob_ids {
            if results.len() as u32 >= limit {
                break;
            }

            match self.resolve_discussion(repo_id, &cob_id).await {
                Ok(discussion) => {
                    let matches = match state_filter {
                        None => true,
                        Some(filter) => std::mem::discriminant(&discussion.state) == std::mem::discriminant(filter),
                    };
                    if matches {
                        results.push((cob_id, discussion));
                    }
                }
                Err(ForgeError::CobNotFound { .. }) => continue,
                Err(e) => return Err(e),
            }
        }

        // Sort by updated_at_ms descending
        results.sort_by_key(|b| std::cmp::Reverse(b.1.updated_at_ms));

        Ok(results)
    }
}
