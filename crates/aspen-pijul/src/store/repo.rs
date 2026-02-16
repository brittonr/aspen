//! Repository management operations for PijulStore.

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_forge::identity::RepoId;
use snafu::ResultExt;
use tracing::debug;
use tracing::info;
use tracing::instrument;

use super::PijulStore;
use crate::constants::KV_PREFIX_PIJUL_REPOS;
use crate::error::CreateDirSnafu;
use crate::error::PijulError;
use crate::error::PijulResult;
use crate::types::PijulRepoIdentity;

impl<B: BlobStore, K: KeyValueStore + ?Sized> PijulStore<B, K> {
    /// Create a new Pijul repository.
    ///
    /// # Arguments
    ///
    /// - `identity`: Repository identity with name, delegates, etc.
    ///
    /// # Returns
    ///
    /// The computed `RepoId` (BLAKE3 hash of the identity).
    #[instrument(skip(self, identity), fields(name = %identity.name))]
    pub async fn create_repo(&self, identity: PijulRepoIdentity) -> PijulResult<RepoId> {
        let repo_id = identity.repo_id()?;

        // Check if already exists
        if self.get_repo(&repo_id).await?.is_some() {
            return Err(PijulError::RepoAlreadyExists {
                repo_id: repo_id.to_string(),
            });
        }

        // Store identity in KV
        let key = format!("{}{}", KV_PREFIX_PIJUL_REPOS, repo_id);
        let value = postcard::to_allocvec(&identity)?;
        let value_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &value);

        self.kv
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set { key, value: value_b64 },
            })
            .await?;

        // Create pristine directory
        let pristine_dir = self.pristine_path(&repo_id);
        tokio::fs::create_dir_all(&pristine_dir).await.context(CreateDirSnafu {
            path: pristine_dir.clone(),
        })?;

        // Create the default channel (empty, no head yet)
        let default_channel = identity.default_channel.clone();
        self.refs.create_empty_channel(&repo_id, &default_channel).await?;

        // Cache the identity
        {
            let mut cache = self.repo_cache.write();
            cache.insert(repo_id, identity.clone());
        }

        info!(repo_id = %repo_id, name = %identity.name, default_channel = %default_channel, "created Pijul repository");

        // Emit event
        let _ = self.event_tx.send(super::PijulStoreEvent::RepoCreated { repo_id, identity });

        Ok(repo_id)
    }

    /// Get a repository's identity.
    #[instrument(skip(self))]
    pub async fn get_repo(&self, repo_id: &RepoId) -> PijulResult<Option<PijulRepoIdentity>> {
        // Check cache first
        {
            let cache = self.repo_cache.read();
            if let Some(identity) = cache.get(repo_id) {
                return Ok(Some(identity.clone()));
            }
        }

        // Load from KV
        let key = format!("{}{}", KV_PREFIX_PIJUL_REPOS, repo_id);

        let result = match self
            .kv
            .read(aspen_core::ReadRequest {
                key,
                consistency: aspen_core::ReadConsistency::Linearizable,
            })
            .await
        {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                return Ok(None);
            }
            Err(e) => return Err(PijulError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(value_b64) => {
                let value =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &value_b64).map_err(|e| {
                        PijulError::Serialization {
                            message: format!("invalid base64: {}", e),
                        }
                    })?;

                let identity: PijulRepoIdentity = postcard::from_bytes(&value)?;

                // Cache it
                {
                    let mut cache = self.repo_cache.write();
                    cache.insert(*repo_id, identity.clone());
                }

                Ok(Some(identity))
            }
            None => Ok(None),
        }
    }

    /// Check if a repository exists.
    pub async fn repo_exists(&self, repo_id: &RepoId) -> PijulResult<bool> {
        Ok(self.get_repo(repo_id).await?.is_some())
    }

    /// List all repositories.
    ///
    /// Returns a list of (repo_id, identity) pairs for all known repositories.
    ///
    /// # Arguments
    ///
    /// - `limit`: Maximum number of repos to return (default 100, max 1000)
    #[instrument(skip(self))]
    pub async fn list_repos(&self, limit: u32) -> PijulResult<Vec<(RepoId, PijulRepoIdentity)>> {
        use aspen_core::ScanRequest;

        let limit = limit.min(1000);

        let result = self
            .kv
            .scan(ScanRequest {
                prefix: KV_PREFIX_PIJUL_REPOS.to_string(),
                limit_results: Some(limit),
                continuation_token: None,
            })
            .await?;

        let mut repos = Vec::with_capacity(result.entries.len());

        for entry in result.entries {
            // Key format: "pijul:repos:{repo_id}"
            let repo_id_str = entry.key.strip_prefix(KV_PREFIX_PIJUL_REPOS).unwrap_or(&entry.key);
            let repo_id = match RepoId::from_hex(repo_id_str) {
                Ok(id) => id,
                Err(_) => {
                    debug!(key = %entry.key, "skipping invalid repo_id in KV");
                    continue;
                }
            };

            // Decode identity
            let value = match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &entry.value) {
                Ok(v) => v,
                Err(e) => {
                    debug!(key = %entry.key, error = %e, "skipping repo with invalid base64");
                    continue;
                }
            };

            let identity: PijulRepoIdentity = match postcard::from_bytes(&value) {
                Ok(id) => id,
                Err(e) => {
                    debug!(key = %entry.key, error = %e, "skipping repo with invalid identity");
                    continue;
                }
            };

            repos.push((repo_id, identity));
        }

        Ok(repos)
    }
}
