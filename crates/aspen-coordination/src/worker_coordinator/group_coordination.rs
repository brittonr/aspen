//! Worker group creation and management.

use anyhow::Result;
use anyhow::bail;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::warn;

use super::DistributedWorkerCoordinator;
use super::constants::MAX_WORKERS_PER_GROUP;
use super::types::WorkerGroup;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> DistributedWorkerCoordinator<S> {
    /// Create a new worker group.
    ///
    /// Uses optimistic check with re-validation under write lock to prevent
    /// TOCTOU race conditions. Lock ordering: workers before groups.
    pub async fn create_group(&self, group: WorkerGroup) -> Result<()> {
        assert!(group.min_members > 0, "WORKER: group '{}' min_members must be > 0", group.group_id);
        assert!(
            group.max_members >= group.min_members,
            "WORKER: group '{}' max_members ({}) must be >= min_members ({})",
            group.group_id,
            group.max_members,
            group.min_members
        );
        // Early validation (optimistic, may have false positives from concurrent creations)
        {
            let groups = self.groups.read().await;
            if groups.len() >= self.config.max_groups {
                bail!("maximum group limit {} reached", self.config.max_groups);
            }
        }

        // Validate member count (stateless check)
        if group.members.len() > MAX_WORKERS_PER_GROUP {
            bail!("group exceeds maximum member limit {}", MAX_WORKERS_PER_GROUP);
        }

        // Store in KV (no local state changed yet)
        let key = verified::worker_group_key(&group.group_id);
        let value = serde_json::to_string(&group)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: key.clone(),
                    value,
                },
            })
            .await?;

        // Acquire locks in canonical order: workers FIRST, then groups
        let mut workers = self.workers.write().await;
        let mut groups = self.groups.write().await;

        // Re-check limit under write lock (TOCTOU protection)
        if groups.len() >= self.config.max_groups && !groups.contains_key(&group.group_id) {
            // Rollback: cleanup KV store (fire-and-forget with logging)
            if let Err(e) = self
                .store
                .write(WriteRequest {
                    command: WriteCommand::Delete { key },
                })
                .await
            {
                warn!(
                    error = %e,
                    group_id = %group.group_id,
                    "failed to rollback group creation from KV store"
                );
            }
            bail!("maximum group limit {} reached during creation", self.config.max_groups);
        }

        // Update worker memberships
        for member_id in &group.members {
            if let Some(worker) = workers.get_mut(member_id) {
                worker.groups.insert(group.group_id.clone());
            }
        }

        groups.insert(group.group_id.clone(), group);

        Ok(())
    }

    /// Get a worker group by ID.
    pub async fn get_group(&self, group_id: &str) -> Result<Option<WorkerGroup>> {
        let groups = self.groups.read().await;
        Ok(groups.get(group_id).cloned())
    }

    /// Add a worker to a group.
    ///
    /// Uses persist-first pattern to avoid cache/KV divergence.
    /// Lock ordering: workers before groups (no locks across await).
    pub async fn add_to_group(&self, group_id: &str, worker_id: &str) -> Result<()> {
        // Phase 1: Read-only validation and prepare update
        let updated_group = {
            let groups = self.groups.read().await;
            let group = groups.get(group_id).ok_or_else(|| anyhow::anyhow!("group {} not found", group_id))?;

            if group.members.len() >= group.max_members {
                bail!("group {} is at maximum capacity", group_id);
            }

            let mut updated = group.clone();
            updated.members.insert(worker_id.to_string());
            updated
        };

        // Phase 2: Persist FIRST (no locks held)
        let key = verified::worker_group_key(group_id);
        let value = serde_json::to_string(&updated_group)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await?;

        // Phase 3: Update cache SECOND with canonical lock order (workers first)
        let mut workers = self.workers.write().await;
        let mut groups = self.groups.write().await;

        // Re-validate group exists (may have been deleted concurrently)
        if let Some(group) = groups.get_mut(group_id) {
            group.members.insert(worker_id.to_string());
        }

        if let Some(worker) = workers.get_mut(worker_id) {
            worker.groups.insert(group_id.to_string());
        }

        Ok(())
    }

    /// Remove a worker from a group.
    ///
    /// Uses persist-first pattern to avoid cache/KV divergence.
    /// Lock ordering: workers before groups (no locks across await).
    pub async fn remove_from_group(&self, group_id: &str, worker_id: &str) -> Result<()> {
        // Phase 1: Read-only validation and prepare update
        let updated_group = {
            let groups = self.groups.read().await;
            let group = groups.get(group_id).ok_or_else(|| anyhow::anyhow!("group {} not found", group_id))?;

            let mut updated = group.clone();
            updated.members.remove(worker_id);

            // Update leader if needed
            if updated.leader.as_deref() == Some(worker_id) {
                updated.leader = updated.members.iter().next().cloned();
            }

            updated
        };

        // Phase 2: Persist FIRST (no locks held)
        let key = verified::worker_group_key(group_id);
        let value = serde_json::to_string(&updated_group)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await?;

        // Phase 3: Update cache SECOND with canonical lock order (workers first)
        let mut workers = self.workers.write().await;
        let mut groups = self.groups.write().await;

        // Re-validate group exists (may have been deleted concurrently)
        if let Some(group) = groups.get_mut(group_id) {
            group.members.remove(worker_id);
            if group.leader.as_deref() == Some(worker_id) {
                group.leader = group.members.iter().next().cloned();
            }
        }

        if let Some(worker) = workers.get_mut(worker_id) {
            worker.groups.remove(group_id);
        }

        Ok(())
    }
}
