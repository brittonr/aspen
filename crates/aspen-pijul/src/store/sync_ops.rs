//! Sync, rebuild, and conflict detection operations for PijulStore.

use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_forge::identity::RepoId;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::PijulStore;
use super::SyncResult;
use crate::apply::ChangeApplicator;
use crate::apply::ChangeDirectory;
use crate::error::PijulError;
use crate::error::PijulResult;
use crate::types::ChangeHash;

impl<B: BlobStore, K: KeyValueStore + ?Sized> PijulStore<B, K> {
    /// Download a change from a remote peer.
    #[instrument(skip(self))]
    pub async fn download_change(&self, hash: &ChangeHash, provider: iroh::PublicKey) -> PijulResult<()> {
        self.changes.download_from_peer(hash, provider).await
    }

    /// Sync local pristine state with cluster for a channel.
    ///
    /// This method fetches missing changes from the cluster and applies them
    /// to bring the local pristine up to date with the cluster's channel head.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name to sync
    ///
    /// # Returns
    ///
    /// The number of changes that were fetched and applied.
    #[instrument(skip(self))]
    pub async fn sync_channel_pristine(&self, repo_id: &RepoId, channel: &str) -> PijulResult<SyncResult> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get the cluster's channel head
        let cluster_head = self.refs.get_channel(repo_id, channel).await?;
        if cluster_head.is_none() {
            debug!(repo_id = %repo_id, channel = channel, "channel has no changes");
            return Ok(SyncResult {
                changes_fetched: 0,
                changes_applied: 0,
                already_synced: true,
                conflicts: None,
            });
        }

        // Get all changes from the cluster's log
        let cluster_log = self.get_change_log(repo_id, channel, 10_000).await?;
        if cluster_log.is_empty() {
            return Ok(SyncResult {
                changes_fetched: 0,
                changes_applied: 0,
                already_synced: true,
                conflicts: None,
            });
        }

        // Get or create local pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Get changes that are already applied locally
        let local_changes = self.get_local_channel_changes(&pristine, channel)?;

        // Determine missing changes (cluster has but local doesn't)
        let missing: Vec<ChangeHash> =
            cluster_log.iter().map(|m| m.hash).filter(|h| !local_changes.contains(h)).collect();

        if missing.is_empty() {
            debug!(repo_id = %repo_id, channel = channel, "pristine already synced");
            return Ok(SyncResult {
                changes_fetched: 0,
                changes_applied: 0,
                already_synced: true,
                conflicts: None,
            });
        }

        info!(
            repo_id = %repo_id,
            channel = channel,
            missing = missing.len(),
            "syncing pristine with cluster"
        );

        // Build dependency-ordered list (oldest first)
        let ordered = self.order_changes_by_dependencies(&cluster_log, &missing);

        // Create change directory and applicator
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));
        let applicator = ChangeApplicator::new(pristine, change_dir);

        // Fetch and apply each missing change in order
        let mut changes_fetched = 0u32;
        let mut changes_applied = 0u32;

        for hash in ordered {
            match applicator.fetch_and_apply(channel, &hash).await {
                Ok(_result) => {
                    changes_fetched += 1;
                    changes_applied += 1;
                    debug!(hash = %hash, "applied missing change");
                }
                Err(e) => {
                    warn!(hash = %hash, error = %e, "failed to apply change");
                    return Err(e);
                }
            }
        }

        info!(
            repo_id = %repo_id,
            channel = channel,
            fetched = changes_fetched,
            applied = changes_applied,
            "pristine sync complete"
        );

        Ok(SyncResult {
            changes_fetched,
            changes_applied,
            already_synced: false,
            conflicts: None,
        })
    }

    /// Rebuild a channel's pristine from scratch using cluster changes.
    ///
    /// This method creates a fresh pristine and applies all changes from the
    /// cluster's change log. Use this when the local pristine is corrupted
    /// or diverged from the cluster state.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name to rebuild
    ///
    /// # Returns
    ///
    /// The number of changes that were applied to rebuild the pristine.
    #[instrument(skip(self))]
    pub async fn rebuild_channel_pristine(&self, repo_id: &RepoId, channel: &str) -> PijulResult<SyncResult> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get the cluster's change log
        let cluster_log = self.get_change_log(repo_id, channel, 10_000).await?;

        info!(
            repo_id = %repo_id,
            channel = channel,
            changes = cluster_log.len(),
            "rebuilding pristine from cluster"
        );

        // Delete the existing pristine to start fresh
        let pristine_path = self.pristine_path(repo_id);
        if pristine_path.exists() {
            std::fs::remove_dir_all(&pristine_path).map_err(|e| PijulError::Io {
                message: format!("failed to remove pristine: {}", e),
            })?;
        }

        // Create a fresh pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Build dependency-ordered list (oldest first)
        let all_hashes: Vec<ChangeHash> = cluster_log.iter().map(|m| m.hash).collect();
        let ordered = self.order_changes_by_dependencies(&cluster_log, &all_hashes);

        // Create change directory and applicator
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));
        let applicator = ChangeApplicator::new(pristine, change_dir);

        // Apply all changes in order
        let mut changes_fetched = 0u32;
        let mut changes_applied = 0u32;

        for hash in ordered {
            match applicator.fetch_and_apply(channel, &hash).await {
                Ok(_result) => {
                    changes_fetched += 1;
                    changes_applied += 1;
                    debug!(hash = %hash, "applied change during rebuild");
                }
                Err(e) => {
                    warn!(hash = %hash, error = %e, "failed to apply change during rebuild");
                    return Err(e);
                }
            }
        }

        info!(
            repo_id = %repo_id,
            channel = channel,
            applied = changes_applied,
            "pristine rebuild complete"
        );

        Ok(SyncResult {
            changes_fetched,
            changes_applied,
            already_synced: false,
            conflicts: None,
        })
    }

    /// Check for conflicts on a channel by outputting to a temporary directory.
    ///
    /// This method outputs the channel's state to a temp directory and checks
    /// for any conflicts that libpijul detects. This is useful for detecting
    /// merge conflicts after syncing changes from multiple sources.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name to check
    ///
    /// # Returns
    ///
    /// Conflict state including paths of conflicting files.
    #[instrument(skip(self))]
    pub async fn check_conflicts(
        &self,
        repo_id: &RepoId,
        channel: &str,
    ) -> PijulResult<crate::types::ChannelConflictState> {
        use super::helpers::extract_conflict_info;
        use crate::output::WorkingDirOutput;
        use crate::types::ChannelConflictState;
        use crate::types::FileConflict;

        // Get the pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Create a temp directory for output
        let temp_dir = std::env::temp_dir().join(format!(
            "aspen-conflict-check-{}-{}",
            repo_id.to_hex(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).map_err(|e| PijulError::Io {
            message: format!("failed to create temp directory: {}", e),
        })?;

        // Create change directory and outputter
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));
        let outputter = WorkingDirOutput::new(pristine, change_dir, temp_dir.clone());

        // Output and check for conflicts
        let result = outputter.output(channel)?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;

        // Convert libpijul conflicts to our format
        let conflicts: Vec<FileConflict> = result
            .conflicts
            .iter()
            .map(|c| {
                // Extract path and involved changes from libpijul Conflict
                let (path, involved_changes) = extract_conflict_info(c);
                FileConflict {
                    path,
                    involved_changes,
                    detected_at_ms: now_ms,
                }
            })
            .collect();

        // Clean up temp directory
        if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
            debug!(path = %temp_dir.display(), error = %e, "failed to clean up temp directory");
        }

        let conflict_count = conflicts.len();
        if conflict_count > 0 {
            info!(
                repo_id = %repo_id,
                channel = channel,
                conflicts = conflict_count,
                "conflicts detected"
            );
        } else {
            debug!(repo_id = %repo_id, channel = channel, "no conflicts");
        }

        // Get the current channel head for the conflict state
        let current_head = self.refs.get_channel(repo_id, channel).await?;

        Ok(ChannelConflictState {
            conflicts,
            checked_at_ms: now_ms,
            head_at_check: current_head,
        })
    }

    /// Sync a channel and check for conflicts.
    ///
    /// This is a convenience method that combines `sync_channel_pristine` with
    /// `check_conflicts`, returning the sync result with conflict information.
    #[instrument(skip(self))]
    pub async fn sync_and_check_conflicts(&self, repo_id: &RepoId, channel: &str) -> PijulResult<SyncResult> {
        // First sync
        let mut result = self.sync_channel_pristine(repo_id, channel).await?;

        // Then check for conflicts if we applied changes
        if result.changes_applied > 0 {
            let conflict_state = self.check_conflicts(repo_id, channel).await?;
            result.conflicts = Some(conflict_state);
        }

        Ok(result)
    }

    /// Apply a downloaded change to the local pristine.
    ///
    /// This is a local-only operation that does NOT query Raft. It's used by
    /// follower nodes after P2P downloading a change - the change is already
    /// in the blob store, we just need to apply it to the local pristine.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name
    /// - `hash`: The change hash that was just downloaded
    ///
    /// # Returns
    ///
    /// Result indicating if the change was applied, with conflict information.
    #[instrument(skip(self))]
    pub async fn apply_downloaded_change(
        &self,
        repo_id: &RepoId,
        channel: &str,
        hash: &ChangeHash,
    ) -> PijulResult<SyncResult> {
        // Verify the change exists in our blob store
        if !self.changes.has_change(hash).await? {
            return Err(PijulError::ChangeNotFound { hash: hash.to_string() });
        }

        // Get or create local pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Create change directory and applicator
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));
        let applicator = ChangeApplicator::new(pristine, change_dir);

        // Apply the change
        match applicator.fetch_and_apply(channel, hash).await {
            Ok(_result) => {
                info!(hash = %hash, channel = channel, "applied downloaded change to local pristine");

                // Check for conflicts
                let conflict_state = self.check_conflicts(repo_id, channel).await?;
                let has_conflicts = conflict_state.has_conflicts();

                Ok(SyncResult {
                    changes_fetched: 0, // Already fetched
                    changes_applied: 1,
                    already_synced: false,
                    conflicts: if has_conflicts { Some(conflict_state) } else { None },
                })
            }
            Err(e) => {
                // If it's already applied, that's fine
                if e.to_string().contains("already applied") || e.to_string().contains("ChangeIsDependency") {
                    debug!(hash = %hash, "change already applied to pristine");
                    Ok(SyncResult {
                        changes_fetched: 0,
                        changes_applied: 0,
                        already_synced: true,
                        conflicts: None,
                    })
                } else {
                    Err(e)
                }
            }
        }
    }
}
