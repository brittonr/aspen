//! Execution Schema Module
//!
//! Schema for adapter-based execution tracking

use crate::storage::schemas::SchemaModule;
use async_trait::async_trait;

pub struct ExecutionSchema;

#[async_trait]
impl SchemaModule for ExecutionSchema {
    fn name(&self) -> &str {
        "execution"
    }

    fn table_definitions(&self) -> Vec<&'static str> {
        vec![
            "CREATE TABLE IF NOT EXISTS execution_instances (
                id TEXT PRIMARY KEY,
                backend_type TEXT NOT NULL,
                execution_handle TEXT NOT NULL,
                job_id TEXT,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                started_at INTEGER,
                completed_at INTEGER,
                node_id TEXT,
                metadata TEXT,
                FOREIGN KEY (job_id) REFERENCES workflows(id)
            )",
        ]
    }

    fn index_definitions(&self) -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_execution_instances_backend_type ON execution_instances(backend_type)",
            "CREATE INDEX IF NOT EXISTS idx_execution_instances_job_id ON execution_instances(job_id)",
            "CREATE INDEX IF NOT EXISTS idx_execution_instances_status ON execution_instances(status)",
            "CREATE INDEX IF NOT EXISTS idx_execution_instances_created_at ON execution_instances(created_at)",
        ]
    }

    fn migrations(&self) -> Vec<&'static str> {
        vec![]
    }

    fn is_enabled(&self) -> bool {
        // Execution tracking is always enabled as it's core functionality
        true
    }
}