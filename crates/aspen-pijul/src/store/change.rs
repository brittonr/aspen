//! Change storage and retrieval operations for PijulStore.

use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_forge::identity::RepoId;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::PijulStore;
use super::PijulStoreEvent;
use crate::apply::ApplyResult;
use crate::apply::ChangeApplicator;
use crate::apply::ChangeDirectory;
use crate::error::PijulError;
use crate::error::PijulResult;
use crate::types::ChangeHash;
use crate::types::ChangeMetadata;

impl<B: BlobStore, K: KeyValueStore + ?Sized> PijulStore<B, K> {
    /// Store a change and update the channel head.
    ///
    /// This is the primary method for recording new changes. The change should
    /// already be serialized in libpijul's format (bincode + zstd compressed).
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository to store the change in
    /// - `channel`: Channel to update
    /// - `change_bytes`: Serialized change data
    /// - `metadata`: Metadata about the change
    ///
    /// # Returns
    ///
    /// The BLAKE3 hash of the stored change.
    #[instrument(skip(self, change_bytes, metadata), fields(size = change_bytes.len()))]
    pub async fn store_change(
        &self,
        repo_id: &RepoId,
        channel: &str,
        change_bytes: &[u8],
        metadata: ChangeMetadata,
    ) -> PijulResult<ChangeHash> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Store the change in blob store
        let hash = self.changes.store_change(change_bytes).await?;

        // Store metadata in KV
        let meta_key = format!("{}{}:{}", crate::constants::KV_PREFIX_PIJUL_CHANGE_META, repo_id, hash);
        let meta_bytes = postcard::to_allocvec(&metadata)?;
        let meta_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &meta_bytes);

        self.kv
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set {
                    key: meta_key,
                    value: meta_b64,
                },
            })
            .await?;

        // Update channel head
        self.refs.set_channel(repo_id, channel, hash).await?;

        info!(repo_id = %repo_id, channel = channel, hash = %hash, "stored change");

        // Emit event
        let _ = self.event_tx.send(PijulStoreEvent::ChangeRecorded {
            repo_id: *repo_id,
            channel: channel.to_string(),
            change_hash: hash,
        });

        Ok(hash)
    }

    /// Get a change by hash.
    ///
    /// Returns the raw serialized change bytes.
    pub async fn get_change(&self, hash: &ChangeHash) -> PijulResult<Option<Vec<u8>>> {
        self.changes.get_change(hash).await
    }

    /// Check if a change exists.
    pub async fn has_change(&self, hash: &ChangeHash) -> PijulResult<bool> {
        self.changes.has_change(hash).await
    }

    /// Get change metadata.
    #[instrument(skip(self))]
    pub async fn get_change_metadata(
        &self,
        repo_id: &RepoId,
        hash: &ChangeHash,
    ) -> PijulResult<Option<ChangeMetadata>> {
        let meta_key = format!("{}{}:{}", crate::constants::KV_PREFIX_PIJUL_CHANGE_META, repo_id, hash);

        let result = match self
            .kv
            .read(aspen_core::ReadRequest {
                key: meta_key,
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

                let metadata: ChangeMetadata = postcard::from_bytes(&value)?;
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
    }

    /// Get blame/attribution for a file.
    ///
    /// Returns a list of changes that have contributed to the current state of
    /// the specified file. Currently provides change-level attribution (all
    /// changes in the channel); per-line blame requires additional implementation.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel to blame on
    /// - `path`: File path to get blame for
    #[instrument(skip(self))]
    pub async fn blame_file(
        &self,
        repo_id: &RepoId,
        channel: &str,
        path: &str,
    ) -> PijulResult<crate::blame::BlameResult> {
        use crate::blame::BlameResult;
        use crate::blame::FileAttribution;

        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get change log for the channel (this gives us all changes)
        // TODO: In the future, filter to only changes that touched the path
        let change_log = self.get_change_log(repo_id, channel, 10_000).await?;

        let mut result = BlameResult::new(path.to_string(), channel.to_string());

        for metadata in change_log {
            let author = metadata.authors.first();
            result.add_attribution(FileAttribution {
                change_hash: metadata.hash,
                author: author.map(|a| a.name.clone()),
                author_email: author.and_then(|a| a.email.clone()),
                message: metadata.message.lines().next().unwrap_or_default().to_string(),
                recorded_at_ms: metadata.recorded_at_ms,
                path: path.to_string(),
                change_type: "unknown".to_string(), // Would need change parsing to determine
            });
        }

        Ok(result)
    }

    /// Get the change log for a channel.
    ///
    /// Returns a list of changes from the channel head back through the
    /// dependency chain, limited by the specified count.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name
    /// - `limit`: Maximum number of changes to return (default 100, max 1000)
    ///
    /// # Returns
    ///
    /// A list of change metadata in reverse chronological order (newest first).
    #[instrument(skip(self))]
    pub async fn get_change_log(
        &self,
        repo_id: &RepoId,
        channel: &str,
        limit: u32,
    ) -> PijulResult<Vec<ChangeMetadata>> {
        let limit = limit.min(1000);

        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get channel head
        let head = match self.refs.get_channel(repo_id, channel).await? {
            Some(h) => h,
            None => {
                // Channel exists but has no changes
                return Ok(Vec::new());
            }
        };

        // Traverse the change DAG following dependencies
        let mut log = Vec::with_capacity(limit as usize);
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back(head);

        while let Some(hash) = queue.pop_front() {
            if log.len() >= limit as usize {
                break;
            }

            if visited.contains(&hash) {
                continue;
            }
            visited.insert(hash);

            // Get metadata for this change
            if let Some(meta) = self.get_change_metadata(repo_id, &hash).await? {
                // Add dependencies to the queue for BFS traversal
                for dep in &meta.dependencies {
                    if !visited.contains(dep) {
                        queue.push_back(*dep);
                    }
                }
                log.push(meta);
            } else {
                warn!(hash = %hash, "change metadata not found in log traversal");
            }
        }

        // Sort by recorded_at_ms descending (newest first)
        log.sort_by(|a, b| b.recorded_at_ms.cmp(&a.recorded_at_ms));

        debug!(repo_id = %repo_id, channel = channel, count = log.len(), "retrieved change log");
        Ok(log)
    }

    /// Apply a change to a repository channel.
    ///
    /// This fetches the change from storage (if not cached locally) and applies
    /// it to the pristine database, updating the channel state.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository to apply the change to
    /// - `channel`: Channel to update
    /// - `hash`: Hash of the change to apply
    ///
    /// # Returns
    ///
    /// The result of the apply operation including the number of operations
    /// applied and the new merkle hash.
    #[instrument(skip(self))]
    pub async fn apply_change(&self, repo_id: &RepoId, channel: &str, hash: &ChangeHash) -> PijulResult<ApplyResult> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get or create the pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Create the change directory for this repo
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));

        // Create the applicator
        let applicator = ChangeApplicator::new(pristine, change_dir);

        // Fetch and apply the change
        let result = applicator.fetch_and_apply(channel, hash).await?;

        // Update channel head in refs
        self.refs.set_channel(repo_id, channel, *hash).await?;

        info!(repo_id = %repo_id, channel = channel, hash = %hash, ops = result.changes_applied, "applied change");

        // Emit event
        let _ = self.event_tx.send(PijulStoreEvent::ChangeApplied {
            repo_id: *repo_id,
            channel: channel.to_string(),
            change_hash: *hash,
        });

        Ok(result)
    }

    /// Apply multiple changes in order.
    ///
    /// Changes are applied sequentially. If any change fails, the operation
    /// stops and returns an error.
    #[instrument(skip(self, hashes))]
    pub async fn apply_changes(
        &self,
        repo_id: &RepoId,
        channel: &str,
        hashes: &[ChangeHash],
    ) -> PijulResult<Vec<ApplyResult>> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get or create the pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Create the change directory for this repo
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));

        // Create the applicator
        let applicator = ChangeApplicator::new(pristine, change_dir);

        // Apply all changes
        let results = applicator.apply_changes(channel, hashes).await?;

        // Update channel head to the last applied change
        if let Some(last_hash) = hashes.last() {
            self.refs.set_channel(repo_id, channel, *last_hash).await?;

            // Emit event for each applied change
            for hash in hashes {
                let _ = self.event_tx.send(PijulStoreEvent::ChangeApplied {
                    repo_id: *repo_id,
                    channel: channel.to_string(),
                    change_hash: *hash,
                });
            }
        }

        info!(repo_id = %repo_id, channel = channel, count = results.len(), "applied changes");
        Ok(results)
    }

    /// Unrecord (remove) a change from a channel.
    ///
    /// This removes a change from a channel's history. The change itself is not
    /// deleted from storage (it may be referenced by other channels), but it
    /// will no longer be part of this channel's state.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository to unrecord the change from
    /// - `channel`: Channel to update
    /// - `hash`: Hash of the change to unrecord
    ///
    /// # Returns
    ///
    /// `true` if the change was successfully unrecorded, `false` if it was not
    /// in the channel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The repository doesn't exist
    /// - The change has dependents that haven't been unrecorded yet
    /// - There's a pristine storage error
    #[instrument(skip(self))]
    pub async fn unrecord_change(&self, repo_id: &RepoId, channel: &str, hash: &ChangeHash) -> PijulResult<bool> {
        use libpijul::MutTxnTExt;

        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get the pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Create the change directory for this repo
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));
        change_dir.ensure_dir()?;

        // Get the libpijul change store
        let store = change_dir.libpijul_store();

        // Convert our hash to libpijul's Hash
        let pijul_hash = ChangeDirectory::<B>::to_pijul_hash(hash);

        // Start a mutable transaction
        let mut txn = pristine.mut_txn_begin()?;

        // Open the channel
        let channel_ref = txn.open_or_create_channel(channel)?;

        // Unrecord the change using MutTxnTExt trait method
        let result = {
            let txn_inner = txn.txn_mut();
            txn_inner.unrecord(&store, &channel_ref, &pijul_hash, 0).map_err(|e| PijulError::UnrecordFailed {
                message: format!("{:?}", e),
            })?
        };

        // Commit the transaction
        txn.commit()?;

        if result {
            info!(repo_id = %repo_id, channel = channel, hash = %hash, "unrecorded change");
        } else {
            debug!(repo_id = %repo_id, channel = channel, hash = %hash, "change was not in channel");
        }

        Ok(result)
    }
}
