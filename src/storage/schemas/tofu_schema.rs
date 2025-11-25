//! OpenTofu/Terraform Schema Module
//!
//! Schema for OpenTofu state backend (only enabled when tofu-support feature is used)

use crate::storage::schemas::SchemaModule;
use async_trait::async_trait;

pub struct TofuSchema;

#[async_trait]
impl SchemaModule for TofuSchema {
    fn name(&self) -> &str {
        "tofu"
    }

    fn table_definitions(&self) -> Vec<&'static str> {
        vec![
            "CREATE TABLE IF NOT EXISTS tofu_workspaces (
                name TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                current_state TEXT,
                state_version INTEGER NOT NULL DEFAULT 0,
                state_lineage TEXT,
                lock_id TEXT,
                lock_acquired_at INTEGER,
                lock_holder TEXT,
                lock_info TEXT
            )",
            "CREATE TABLE IF NOT EXISTS tofu_plans (
                id TEXT PRIMARY KEY,
                workspace TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                plan_data BLOB NOT NULL,
                plan_json TEXT,
                status TEXT NOT NULL,
                approved_by TEXT,
                executed_at INTEGER,
                FOREIGN KEY (workspace) REFERENCES tofu_workspaces(name)
            )",
            "CREATE TABLE IF NOT EXISTS tofu_state_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace TEXT NOT NULL,
                state_data TEXT NOT NULL,
                state_version INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                created_by TEXT,
                FOREIGN KEY (workspace) REFERENCES tofu_workspaces(name)
            )",
        ]
    }

    fn index_definitions(&self) -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_tofu_workspaces_lock_id ON tofu_workspaces(lock_id)",
            "CREATE INDEX IF NOT EXISTS idx_tofu_plans_workspace ON tofu_plans(workspace)",
            "CREATE INDEX IF NOT EXISTS idx_tofu_plans_status ON tofu_plans(status)",
            "CREATE INDEX IF NOT EXISTS idx_tofu_state_history_workspace ON tofu_state_history(workspace)",
            "CREATE INDEX IF NOT EXISTS idx_tofu_state_history_created_at ON tofu_state_history(created_at DESC)",
        ]
    }

    fn migrations(&self) -> Vec<&'static str> {
        vec![]
    }

    fn is_enabled(&self) -> bool {
        // Check if tofu-support feature is enabled
        #[cfg(feature = "tofu-support")]
        {
            true
        }
        #[cfg(not(feature = "tofu-support"))]
        {
            // Default to true for now until feature flags are implemented
            true
        }
    }
}