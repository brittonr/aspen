//! OpenTofu/Terraform HTTP State Backend Implementation
//!
//! This module implements the HTTP backend protocol for Terraform/OpenTofu state management
//! using Hiqlite as the distributed storage backend.

use anyhow::Result;
use serde_json;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    hiqlite::HiqliteService,
    params,
    tofu::types::*,
};

/// OpenTofu state backend using Hiqlite for distributed state management
#[derive(Clone)]
pub struct TofuStateBackend {
    hiqlite: Arc<HiqliteService>,
    lock_timeout_secs: i64,
}

impl TofuStateBackend {
    /// Create a new TofuStateBackend
    pub fn new(hiqlite: Arc<HiqliteService>) -> Self {
        Self {
            hiqlite,
            lock_timeout_secs: 300, // 5 minutes default
        }
    }

    /// Create or get a workspace
    pub async fn get_or_create_workspace(&self, name: &str) -> Result<TofuWorkspace> {
        // Try to get existing workspace
        let result: Result<Vec<WorkspaceRow>, _> = self.hiqlite.query_as(
            "SELECT name, created_at, updated_at, current_state, state_version, state_lineage,
                    lock_id, lock_acquired_at, lock_holder, lock_info
             FROM tofu_workspaces WHERE name = $1",
            params!(name),
        ).await;

        match result {
            Ok(rows) if !rows.is_empty() => {
                let row = &rows[0];
                Ok(self.row_to_workspace(row))
            }
            _ => {
                // Create new workspace
                let now = self.current_timestamp();
                self.hiqlite.execute(
                    "INSERT INTO tofu_workspaces (name, created_at, updated_at, state_version)
                     VALUES ($1, $2, $3, 0)",
                    params!(name, now, now),
                ).await?;

                Ok(TofuWorkspace {
                    name: name.to_string(),
                    created_at: now,
                    updated_at: now,
                    current_state: None,
                    state_version: 0,
                    state_lineage: None,
                    lock: None,
                })
            }
        }
    }

    /// Get the current state for a workspace
    pub async fn get_state(&self, workspace: &str) -> Result<Option<TofuState>> {
        let workspace = self.get_or_create_workspace(workspace).await?;
        Ok(workspace.current_state)
    }

    /// Update the state for a workspace
    pub async fn put_state(
        &self,
        workspace: &str,
        state: TofuState,
        expected_version: Option<i64>,
    ) -> Result<()> {
        let current_workspace = self.get_or_create_workspace(workspace).await?;

        // Check version if provided
        if let Some(expected) = expected_version {
            if current_workspace.state_version != expected {
                return Err(TofuError::StateVersionMismatch {
                    expected,
                    actual: current_workspace.state_version,
                }.into());
            }
        }

        let state_json = serde_json::to_string(&state)?;
        let new_version = current_workspace.state_version + 1;
        let now = self.current_timestamp();

        // Store current state in history before updating
        if current_workspace.current_state.is_some() {
            self.hiqlite.execute(
                "INSERT INTO tofu_state_history (workspace, state_data, state_version, created_at)
                 VALUES ($1, $2, $3, $4)",
                params!(
                    workspace,
                    current_workspace.current_state.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default()).unwrap_or_default(),
                    current_workspace.state_version,
                    now
                ),
            ).await?;
        }

        // Update current state
        let rows_affected = self.hiqlite.execute(
            "UPDATE tofu_workspaces
             SET current_state = $1, state_version = $2, updated_at = $3, state_lineage = $4
             WHERE name = $5 AND state_version = $6",
            params!(
                state_json,
                new_version,
                now,
                state.lineage.clone(),
                workspace,
                current_workspace.state_version
            ),
        ).await?;

        if rows_affected == 0 {
            return Err(TofuError::StateVersionMismatch {
                expected: current_workspace.state_version,
                actual: current_workspace.state_version + 1,
            }.into());
        }

        Ok(())
    }

    /// Lock a workspace for exclusive access
    pub async fn lock_workspace(&self, workspace: &str, lock_info: LockRequest) -> Result<()> {
        // Retry once if the workspace doesn't exist
        for attempt in 0..2 {
            let now = self.current_timestamp();

            // Check for stale locks and clean them up
            self.cleanup_stale_locks().await?;

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
                return Ok(());
            }

            // Check if workspace exists and is already locked
            let workspace_data = self.get_or_create_workspace(workspace).await?;
            if workspace_data.lock.is_some() {
                return Err(TofuError::WorkspaceLocked.into());
            }

            // If this is the first attempt and workspace was just created, retry
            if attempt == 0 {
                continue;
            }
        }

        Err(anyhow::anyhow!("Failed to acquire workspace lock after retries"))
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
            let workspace_data = self.get_or_create_workspace(workspace).await?;
            if let Some(lock) = workspace_data.lock {
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
        let workspace_data = self.get_or_create_workspace(workspace).await?;
        Ok(workspace_data.lock.is_some())
    }

    /// Get lock information for a workspace
    pub async fn get_lock_info(&self, workspace: &str) -> Result<Option<WorkspaceLock>> {
        let workspace_data = self.get_or_create_workspace(workspace).await?;
        Ok(workspace_data.lock)
    }

    /// Clean up stale locks (locks older than timeout)
    async fn cleanup_stale_locks(&self) -> Result<()> {
        let cutoff_time = self.current_timestamp() - self.lock_timeout_secs;

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

    /// Delete a workspace and all its history
    pub async fn delete_workspace(&self, workspace: &str) -> Result<()> {
        // Delete history first (foreign key constraint)
        self.hiqlite.execute(
            "DELETE FROM tofu_state_history WHERE workspace = $1",
            params!(workspace),
        ).await?;

        // Delete plans
        self.hiqlite.execute(
            "DELETE FROM tofu_plans WHERE workspace = $1",
            params!(workspace),
        ).await?;

        // Delete workspace
        self.hiqlite.execute(
            "DELETE FROM tofu_workspaces WHERE name = $1",
            params!(workspace),
        ).await?;

        Ok(())
    }

    /// List all workspaces
    pub async fn list_workspaces(&self) -> Result<Vec<String>> {
        let rows: Vec<WorkspaceNameRow> = self.hiqlite.query_as(
            "SELECT name FROM tofu_workspaces ORDER BY name",
            params!(),
        ).await?;

        Ok(rows.into_iter().map(|r| r.name).collect())
    }

    /// Get state history for a workspace
    pub async fn get_state_history(
        &self,
        workspace: &str,
        limit: Option<i64>,
    ) -> Result<Vec<(i64, TofuState, i64)>> {
        // Build dynamic query since we need LIMIT
        let rows: Vec<StateHistoryRow> = if let Some(limit) = limit {
            // For limited queries, we need to build the query string dynamically
            // Hiqlite doesn't support parameterized LIMIT, so we format it directly
            let query_str = format!(
                "SELECT state_data, state_version, created_at
                 FROM tofu_state_history
                 WHERE workspace = $1
                 ORDER BY created_at DESC
                 LIMIT {}",
                limit
            );
            self.hiqlite.query_as(query_str, params!(workspace)).await?
        } else {
            self.hiqlite.query_as(
                "SELECT state_data, state_version, created_at
                 FROM tofu_state_history
                 WHERE workspace = $1
                 ORDER BY created_at DESC",
                params!(workspace),
            ).await?
        };

        let mut history = Vec::new();
        for row in rows {
            if let Ok(state) = serde_json::from_str::<TofuState>(&row.state_data) {
                history.push((row.state_version, state, row.created_at));
            }
        }

        Ok(history)
    }

    /// Rollback to a previous state version
    pub async fn rollback_state(&self, workspace: &str, target_version: i64) -> Result<()> {
        // Get the target state from history
        let rows: Vec<StateHistoryRow> = self.hiqlite.query_as(
            "SELECT state_data, state_version, created_at
             FROM tofu_state_history
             WHERE workspace = $1 AND state_version = $2",
            params!(workspace, target_version),
        ).await?;

        if rows.is_empty() {
            return Err(anyhow::anyhow!("State version {} not found in history", target_version));
        }

        let state: TofuState = serde_json::from_str(&rows[0].state_data)?;

        // Put this state as the current state (will increment version)
        self.put_state(workspace, state, None).await?;

        Ok(())
    }

    /// Store a plan for later execution
    pub async fn store_plan(&self, plan: StoredPlan) -> Result<()> {
        let now = self.current_timestamp();
        let plan_json = plan.plan_json.clone().unwrap_or_default();
        let status = serde_json::to_string(&plan.status)?;

        self.hiqlite.execute(
            "INSERT INTO tofu_plans (id, workspace, created_at, plan_data, plan_json, status)
             VALUES ($1, $2, $3, $4, $5, $6)",
            params!(
                plan.id,
                plan.workspace,
                now,
                plan.plan_data,
                plan_json,
                status
            ),
        ).await?;

        Ok(())
    }

    /// Get a stored plan
    pub async fn get_plan(&self, plan_id: &str) -> Result<StoredPlan> {
        let rows: Vec<PlanRow> = self.hiqlite.query_as(
            "SELECT id, workspace, created_at, plan_data, plan_json, status, approved_by, executed_at
             FROM tofu_plans WHERE id = $1",
            params!(plan_id),
        ).await?;

        if rows.is_empty() {
            return Err(TofuError::PlanNotFound(plan_id.to_string()).into());
        }

        let row = &rows[0];
        Ok(StoredPlan {
            id: row.id.clone(),
            workspace: row.workspace.clone(),
            created_at: row.created_at,
            plan_data: row.plan_data.clone(),
            plan_json: row.plan_json.clone(),
            status: serde_json::from_str(&row.status)?,
            approved_by: row.approved_by.clone(),
            executed_at: row.executed_at,
        })
    }

    /// Update plan status
    pub async fn update_plan_status(
        &self,
        plan_id: &str,
        status: PlanStatus,
        approved_by: Option<String>,
    ) -> Result<()> {
        let status_str = serde_json::to_string(&status)?;
        let now = if status == PlanStatus::Applied {
            Some(self.current_timestamp())
        } else {
            None
        };

        self.hiqlite.execute(
            "UPDATE tofu_plans
             SET status = $1, approved_by = $2, executed_at = $3
             WHERE id = $4",
            params!(status_str, approved_by, now, plan_id),
        ).await?;

        Ok(())
    }

    /// List plans for a workspace
    pub async fn list_plans(&self, workspace: &str) -> Result<Vec<StoredPlan>> {
        let rows: Vec<PlanRow> = self.hiqlite.query_as(
            "SELECT id, workspace, created_at, plan_data, plan_json, status, approved_by, executed_at
             FROM tofu_plans
             WHERE workspace = $1
             ORDER BY created_at DESC",
            params!(workspace),
        ).await?;

        let mut plans = Vec::new();
        for row in rows {
            if let Ok(status) = serde_json::from_str(&row.status) {
                plans.push(StoredPlan {
                    id: row.id,
                    workspace: row.workspace,
                    created_at: row.created_at,
                    plan_data: row.plan_data,
                    plan_json: row.plan_json,
                    status,
                    approved_by: row.approved_by,
                    executed_at: row.executed_at,
                });
            }
        }

        Ok(plans)
    }

    /// Helper to get current timestamp
    fn current_timestamp(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs() as i64
    }

    /// Convert database row to workspace
    fn row_to_workspace(&self, row: &WorkspaceRow) -> TofuWorkspace {
        let current_state = row.current_state.as_ref()
            .and_then(|s| serde_json::from_str::<TofuState>(s).ok());

        let lock = row.lock_info.as_ref()
            .and_then(|s| serde_json::from_str::<WorkspaceLock>(s).ok());

        TofuWorkspace {
            name: row.name.clone(),
            created_at: row.created_at,
            updated_at: row.updated_at,
            current_state,
            state_version: row.state_version,
            state_lineage: row.state_lineage.clone(),
            lock,
        }
    }
}

// Database row types for deserialization
#[derive(Debug, Clone, serde::Deserialize)]
struct WorkspaceRow {
    name: String,
    created_at: i64,
    updated_at: i64,
    current_state: Option<String>,
    state_version: i64,
    state_lineage: Option<String>,
    lock_info: Option<String>,
}

impl From<hiqlite::Row<'static>> for WorkspaceRow {
    fn from(mut row: hiqlite::Row<'static>) -> Self {
        Self {
            name: row.get::<String>("name"),
            created_at: row.get::<i64>("created_at"),
            updated_at: row.get::<i64>("updated_at"),
            current_state: row.get::<Option<String>>("current_state"),
            state_version: row.get::<i64>("state_version"),
            state_lineage: row.get::<Option<String>>("state_lineage"),
            lock_info: row.get::<Option<String>>("lock_info"),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct WorkspaceNameRow {
    name: String,
}

impl From<hiqlite::Row<'static>> for WorkspaceNameRow {
    fn from(mut row: hiqlite::Row<'static>) -> Self {
        Self {
            name: row.get::<String>("name"),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct StateHistoryRow {
    state_data: String,
    state_version: i64,
    created_at: i64,
}

impl From<hiqlite::Row<'static>> for StateHistoryRow {
    fn from(mut row: hiqlite::Row<'static>) -> Self {
        Self {
            state_data: row.get::<String>("state_data"),
            state_version: row.get::<i64>("state_version"),
            created_at: row.get::<i64>("created_at"),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PlanRow {
    id: String,
    workspace: String,
    created_at: i64,
    plan_data: Vec<u8>,
    plan_json: Option<String>,
    status: String,
    approved_by: Option<String>,
    executed_at: Option<i64>,
}

impl From<hiqlite::Row<'static>> for PlanRow {
    fn from(mut row: hiqlite::Row<'static>) -> Self {
        Self {
            id: row.get::<String>("id"),
            workspace: row.get::<String>("workspace"),
            created_at: row.get::<i64>("created_at"),
            plan_data: row.get::<Vec<u8>>("plan_data"),
            plan_json: row.get::<Option<String>>("plan_json"),
            status: row.get::<String>("status"),
            approved_by: row.get::<Option<String>>("approved_by"),
            executed_at: row.get::<Option<i64>>("executed_at"),
        }
    }
}