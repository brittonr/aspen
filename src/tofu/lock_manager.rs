//! Workspace locking management for OpenTofu/Terraform state
//!
//! Provides distributed locking mechanisms to prevent concurrent state modifications.

use anyhow::Result;
use serde_json;
use std::sync::Arc;

use crate::{
    hiqlite::HiqliteService,
    params,
    tofu::types::*,
};

/// Manager for workspace locking operations
#[derive(Clone)]
pub struct TofuLockManager {
    hiqlite: Arc<HiqliteService>,
    lock_timeout_secs: i64,
}

impl TofuLockManager {
    /// Create a new lock manager
    pub fn new(hiqlite: Arc<HiqliteService>) -> Self {
        Self {
            hiqlite,
            lock_timeout_secs: 300, // 5 minutes default
        }
    }

    /// Create a lock manager with custom timeout
    pub fn with_timeout(hiqlite: Arc<HiqliteService>, timeout_secs: i64) -> Self {
        Self {
            hiqlite,
            lock_timeout_secs: timeout_secs,
        }
    }

    /// Lock a workspace for exclusive access
    pub async fn lock_workspace(&self, workspace: &str, lock_info: LockRequest, now: i64) -> Result<()> {
        // Clean up stale locks first
        self.cleanup_stale_locks(now).await?;

        // Try to acquire lock
        let lock_json = serde_json::to_string(&WorkspaceLock {
            id: lock_info.id.clone(),
            operation: lock_info.operation.clone(),
            info: lock_info.info.clone(),
            who: lock_info.who.clone(),
            version: lock_info.version.clone(),
            created: now,
            path: lock_info.path.clone(),
        })?;

        let rows_affected = self.hiqlite.execute(
            "UPDATE tofu_workspaces
             SET lock_id = $1, lock_acquired_at = $2, lock_holder = $3, lock_info = $4
             WHERE name = $5 AND lock_id IS NULL",
            params!(
                lock_info.id.clone(),
                now,
                lock_info.who.clone(),
                lock_json,
                workspace
            ),
        ).await?;

        if rows_affected > 0 {
            Ok(())
        } else {
            // Check if workspace is already locked
            if self.is_locked_internal(workspace).await? {
                Err(TofuError::WorkspaceLocked.into())
            } else {
                Err(anyhow::anyhow!("Failed to acquire workspace lock"))
            }
        }
    }

    /// Unlock a workspace
    pub async fn unlock_workspace(&self, workspace: &str, lock_id: &str) -> Result<()> {
        let rows_affected = self.hiqlite.execute(
            "UPDATE tofu_workspaces
             SET lock_id = NULL, lock_acquired_at = NULL, lock_holder = NULL, lock_info = NULL
             WHERE name = $1 AND lock_id = $2",
            params!(workspace, lock_id),
        ).await?;

        if rows_affected == 0 {
            // Check if the lock exists with a different ID
            if let Some(lock) = self.get_lock_info_internal(workspace).await? {
                if lock.id != lock_id {
                    return Err(TofuError::LockNotFound(lock_id.to_string()).into());
                }
            }
        }

        Ok(())
    }

    /// Force unlock a workspace (admin operation)
    pub async fn force_unlock_workspace(&self, workspace: &str) -> Result<()> {
        self.hiqlite.execute(
            "UPDATE tofu_workspaces
             SET lock_id = NULL, lock_acquired_at = NULL, lock_holder = NULL, lock_info = NULL
             WHERE name = $1",
            params!(workspace),
        ).await?;
        Ok(())
    }

    /// Check if a workspace is locked
    pub async fn is_locked(&self, workspace: &str) -> Result<bool> {
        self.is_locked_internal(workspace).await
    }

    /// Get lock information for a workspace
    pub async fn get_lock_info(&self, workspace: &str) -> Result<Option<WorkspaceLock>> {
        self.get_lock_info_internal(workspace).await
    }

    /// Clean up stale locks (locks older than timeout)
    pub async fn cleanup_stale_locks(&self, now: i64) -> Result<()> {
        let cutoff_time = now - self.lock_timeout_secs;

        let rows_affected = self.hiqlite.execute(
            "UPDATE tofu_workspaces
             SET lock_id = NULL, lock_acquired_at = NULL, lock_holder = NULL, lock_info = NULL
             WHERE lock_acquired_at < $1 AND lock_id IS NOT NULL",
            params!(cutoff_time),
        ).await?;

        if rows_affected > 0 {
            tracing::info!("Cleaned up {} stale workspace locks", rows_affected);
        }

        Ok(())
    }

    /// Internal helper to check if workspace is locked
    async fn is_locked_internal(&self, workspace: &str) -> Result<bool> {
        let lock_info = self.get_lock_info_internal(workspace).await?;
        Ok(lock_info.is_some())
    }

    /// Internal helper to get lock info
    async fn get_lock_info_internal(&self, workspace: &str) -> Result<Option<WorkspaceLock>> {
        let rows: Vec<LockInfoRow> = self.hiqlite.query_as(
            "SELECT lock_info FROM tofu_workspaces WHERE name = $1",
            params!(workspace),
        ).await?;

        if rows.is_empty() {
            return Ok(None);
        }

        Ok(rows[0].lock_info.as_ref()
            .and_then(|s| serde_json::from_str::<WorkspaceLock>(s).ok()))
    }
}

// Database row types
#[derive(Debug, Clone, serde::Deserialize)]
struct LockInfoRow {
    lock_info: Option<String>,
}

impl From<hiqlite::Row<'static>> for LockInfoRow {
    fn from(mut row: hiqlite::Row<'static>) -> Self {
        Self {
            lock_info: row.get::<Option<String>>("lock_info"),
        }
    }
}
