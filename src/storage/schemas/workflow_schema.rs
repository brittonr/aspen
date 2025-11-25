//! Workflow Schema Module
//!
//! Core schema for workflow/job storage

use crate::storage::schemas::SchemaModule;
use async_trait::async_trait;

pub struct WorkflowSchema;

#[async_trait]
impl SchemaModule for WorkflowSchema {
    fn name(&self) -> &str {
        "workflows"
    }

    fn table_definitions(&self) -> Vec<&'static str> {
        vec![
            "CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                claimed_by TEXT,
                completed_by TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                started_at INTEGER,
                error_message TEXT,
                retry_count INTEGER NOT NULL DEFAULT 0,
                data TEXT,
                compatible_worker_types TEXT,
                assigned_worker_id TEXT
            )",
        ]
    }

    fn index_definitions(&self) -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_workflows_status ON workflows(status)",
            "CREATE INDEX IF NOT EXISTS idx_workflows_claimed_by ON workflows(claimed_by)",
            "CREATE INDEX IF NOT EXISTS idx_workflows_updated_at ON workflows(updated_at)",
        ]
    }

    fn migrations(&self) -> Vec<&'static str> {
        vec![
            // These migrations handle existing databases that may not have newer columns
            "ALTER TABLE workflows ADD COLUMN started_at INTEGER",
            "ALTER TABLE workflows ADD COLUMN error_message TEXT",
            "ALTER TABLE workflows ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0",
            "ALTER TABLE workflows ADD COLUMN compatible_worker_types TEXT",
            "ALTER TABLE workflows ADD COLUMN assigned_worker_id TEXT",
        ]
    }

    fn is_enabled(&self) -> bool {
        // Workflows are always enabled as they're core functionality
        true
    }
}