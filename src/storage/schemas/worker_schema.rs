//! Worker Schema Module
//!
//! Schema for worker registry and heartbeats

use crate::storage::schemas::SchemaModule;
use async_trait::async_trait;

pub struct WorkerSchema;

#[async_trait]
impl SchemaModule for WorkerSchema {
    fn name(&self) -> &str {
        "workers"
    }

    fn table_definitions(&self) -> Vec<&'static str> {
        vec![
            "CREATE TABLE IF NOT EXISTS workers (
                id TEXT PRIMARY KEY,
                worker_type TEXT NOT NULL,
                status TEXT NOT NULL,
                endpoint_id TEXT NOT NULL,
                registered_at INTEGER NOT NULL,
                last_heartbeat INTEGER NOT NULL,
                cpu_cores INTEGER,
                memory_mb INTEGER,
                active_jobs INTEGER NOT NULL DEFAULT 0,
                total_jobs_completed INTEGER NOT NULL DEFAULT 0,
                metadata TEXT
            )",
            "CREATE TABLE IF NOT EXISTS heartbeats (
                node_id TEXT PRIMARY KEY,
                last_seen INTEGER NOT NULL,
                status TEXT NOT NULL
            )",
        ]
    }

    fn index_definitions(&self) -> Vec<&'static str> {
        vec![
            "CREATE INDEX IF NOT EXISTS idx_workers_status ON workers(status)",
            "CREATE INDEX IF NOT EXISTS idx_workers_type ON workers(worker_type)",
            "CREATE INDEX IF NOT EXISTS idx_workers_heartbeat ON workers(last_heartbeat)",
        ]
    }

    fn migrations(&self) -> Vec<&'static str> {
        vec![]
    }

    fn is_enabled(&self) -> bool {
        // Workers are core functionality
        true
    }
}