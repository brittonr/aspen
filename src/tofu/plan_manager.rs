//! Plan storage management for OpenTofu/Terraform
//!
//! Handles storing, retrieving, and managing execution plans.

use anyhow::Result;
use serde_json;
use std::sync::Arc;

use crate::{
    hiqlite::HiqliteService,
    params,
    tofu::types::*,
};

/// Manager for plan storage operations
#[derive(Clone)]
pub struct TofuPlanManager {
    hiqlite: Arc<HiqliteService>,
}

impl TofuPlanManager {
    /// Create a new plan manager
    pub fn new(hiqlite: Arc<HiqliteService>) -> Self {
        Self { hiqlite }
    }

    /// Store a plan for later execution
    pub async fn store_plan(&self, plan: StoredPlan, now: i64) -> Result<()> {
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
        now: i64,
    ) -> Result<()> {
        let status_str = serde_json::to_string(&status)?;
        let executed_at = if status == PlanStatus::Applied {
            Some(now)
        } else {
            None
        };

        self.hiqlite.execute(
            "UPDATE tofu_plans
             SET status = $1, approved_by = $2, executed_at = $3
             WHERE id = $4",
            params!(status_str, approved_by, executed_at, plan_id),
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

    /// Delete all plans for a workspace
    pub async fn delete_workspace_plans(&self, workspace: &str) -> Result<()> {
        self.hiqlite.execute(
            "DELETE FROM tofu_plans WHERE workspace = $1",
            params!(workspace),
        ).await?;
        Ok(())
    }
}

// Database row types
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
