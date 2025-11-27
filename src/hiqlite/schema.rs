//! Database schema initialization for Hiqlite
//!
//! This module contains all table definitions and index creation logic
//! for the distributed database. Schema is initialized once at startup.

use anyhow::Result;
use crate::params;

use super::HiqliteService;

impl HiqliteService {
    /// Initialize database schema for workflow state
    ///
    /// This should be called once during application startup to ensure
    /// the necessary tables exist.
    pub async fn initialize_schema(&self) -> Result<()> {
        tracing::info!("Initializing hiqlite schema");

        self.create_workflows_table().await?;
        self.create_heartbeats_table().await?;
        self.create_workers_table().await?;
        self.create_vm_tables().await?;
        self.create_execution_instances_table().await?;
        self.create_tofu_tables().await?;

        tracing::info!("Schema initialization complete");
        Ok(())
    }

    /// Create workflows table
    async fn create_workflows_table(&self) -> Result<()> {
        self.execute(
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
                compatible_worker_types TEXT,
                assigned_worker_id TEXT,
                data TEXT
            )",
            params!(),
        ).await?;
        Ok(())
    }

    /// Create heartbeats table for node health tracking
    async fn create_heartbeats_table(&self) -> Result<()> {
        self.execute(
            "CREATE TABLE IF NOT EXISTS heartbeats (
                node_id TEXT PRIMARY KEY,
                last_seen INTEGER NOT NULL,
                status TEXT NOT NULL
            )",
            params!(),
        ).await?;
        Ok(())
    }

    /// Create workers table with indices
    async fn create_workers_table(&self) -> Result<()> {
        self.execute(
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
            params!(),
        ).await?;

        // Create indices for efficient worker queries
        self.create_index_if_not_exists(
            "idx_workers_status",
            "workers(status)",
        ).await;

        self.create_index_if_not_exists(
            "idx_workers_type",
            "workers(worker_type)",
        ).await;

        self.create_index_if_not_exists(
            "idx_workers_heartbeat",
            "workers(last_heartbeat)",
        ).await;

        Ok(())
    }

    /// Create VM-related tables (vms and vm_events)
    async fn create_vm_tables(&self) -> Result<()> {
        // Create VM registry table for distributed VM lifecycle management
        self.execute(
            "CREATE TABLE IF NOT EXISTS vms (
                id TEXT PRIMARY KEY,
                config TEXT NOT NULL,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                node_id TEXT NOT NULL,
                pid INTEGER,
                control_socket TEXT,
                job_dir TEXT,
                ip_address TEXT,
                metrics TEXT
            )",
            params!(),
        ).await?;

        // Create indices for VM queries
        self.create_index_if_not_exists(
            "idx_vms_state",
            "vms(state)",
        ).await;

        self.create_index_if_not_exists(
            "idx_vms_node_id",
            "vms(node_id)",
        ).await;

        // Create VM events table for audit trail
        self.execute(
            "CREATE TABLE IF NOT EXISTS vm_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vm_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                event_data TEXT,
                timestamp INTEGER NOT NULL,
                node_id TEXT,
                FOREIGN KEY (vm_id) REFERENCES vms(id)
            )",
            params!(),
        ).await?;

        self.create_index_if_not_exists(
            "idx_vm_events_vm_id",
            "vm_events(vm_id)",
        ).await;

        self.create_index_if_not_exists(
            "idx_vm_events_timestamp",
            "vm_events(timestamp)",
        ).await;

        Ok(())
    }

    /// Create execution_instances table for adapter-based execution tracking
    async fn create_execution_instances_table(&self) -> Result<()> {
        self.execute(
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
            params!(),
        ).await?;

        // Create indices for execution_instances queries
        self.create_index_if_not_exists(
            "idx_execution_instances_backend_type",
            "execution_instances(backend_type)",
        ).await;

        self.create_index_if_not_exists(
            "idx_execution_instances_job_id",
            "execution_instances(job_id)",
        ).await;

        self.create_index_if_not_exists(
            "idx_execution_instances_status",
            "execution_instances(status)",
        ).await;

        self.create_index_if_not_exists(
            "idx_execution_instances_created_at",
            "execution_instances(created_at)",
        ).await;

        Ok(())
    }

    /// Create OpenTofu state backend tables
    async fn create_tofu_tables(&self) -> Result<()> {
        // Create workspaces table
        self.execute(
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
            params!(),
        ).await?;

        self.create_index_if_not_exists(
            "idx_tofu_workspaces_lock_id",
            "tofu_workspaces(lock_id)",
        ).await;

        // Create table for OpenTofu plan storage
        self.execute(
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
            params!(),
        ).await?;

        self.create_index_if_not_exists(
            "idx_tofu_plans_workspace",
            "tofu_plans(workspace)",
        ).await;

        self.create_index_if_not_exists(
            "idx_tofu_plans_status",
            "tofu_plans(status)",
        ).await;

        // Create table for OpenTofu state history (for rollback)
        self.execute(
            "CREATE TABLE IF NOT EXISTS tofu_state_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace TEXT NOT NULL,
                state_data TEXT NOT NULL,
                state_version INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                created_by TEXT,
                FOREIGN KEY (workspace) REFERENCES tofu_workspaces(name)
            )",
            params!(),
        ).await?;

        self.create_index_if_not_exists(
            "idx_tofu_state_history_workspace",
            "tofu_state_history(workspace)",
        ).await;

        self.create_index_if_not_exists(
            "idx_tofu_state_history_created_at",
            "tofu_state_history(created_at DESC)",
        ).await;

        Ok(())
    }

    /// Helper to create an index, logging warnings on failure
    async fn create_index_if_not_exists(&self, index_name: &str, on_columns: &str) {
        let query = format!("CREATE INDEX IF NOT EXISTS {} ON {}", index_name, on_columns);
        if let Err(e) = self.execute(query, params!()).await {
            tracing::warn!("Failed to create index {}: {}", index_name, e);
        }
    }
}
