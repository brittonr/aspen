// VM Registry with Hiqlite - Distributed VM state management
//
// This version uses Hiqlite for distributed, replicated VM tracking

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use hiqlite::{Client, Param};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::hiqlite::HiqliteService;
use super::vm_types::{VmConfig, VmInstance, VmMode, VmState};

/// Registry for tracking all VM instances using Hiqlite
pub struct VmRegistryHiqlite {
    /// In-memory cache for fast lookups
    vms: DashMap<Uuid, Arc<RwLock<VmInstance>>>,
    /// Index by state for efficient queries
    by_state: DashMap<String, HashSet<Uuid>>,
    /// Hiqlite client for distributed persistence
    hiqlite: HiqliteService,
}

impl VmRegistryHiqlite {
    /// Create new registry with Hiqlite persistence
    pub async fn new(hiqlite: HiqliteService) -> Result<Self> {
        // Initialize VM tables in Hiqlite
        Self::init_schema(&hiqlite).await?;

        // Recover existing VMs from Hiqlite
        let registry = Self {
            vms: DashMap::new(),
            by_state: DashMap::new(),
            hiqlite,
        };

        registry.recover_from_hiqlite().await?;

        Ok(registry)
    }

    /// Initialize Hiqlite schema for VMs
    async fn init_schema(hiqlite: &HiqliteService) -> Result<()> {
        // Create VM table with distributed replication
        hiqlite.query_write(
            "CREATE TABLE IF NOT EXISTS vms (
                id TEXT PRIMARY KEY,
                config TEXT NOT NULL,
                state TEXT NOT NULL,
                pid INTEGER,
                started_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                metadata TEXT
            )",
            vec![],
        ).await?;

        // Create index for state queries
        hiqlite.query_write(
            "CREATE INDEX IF NOT EXISTS idx_vms_state ON vms(state)",
            vec![],
        ).await?;

        // Create VM events table for audit trail
        hiqlite.query_write(
            "CREATE TABLE IF NOT EXISTS vm_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vm_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                event_data TEXT,
                timestamp INTEGER NOT NULL,
                node_id TEXT,
                FOREIGN KEY (vm_id) REFERENCES vms(id)
            )",
            vec![],
        ).await?;

        Ok(())
    }

    /// Register a new VM
    pub async fn register(&self, vm: VmInstance) -> Result<()> {
        let vm_id = vm.config.id;

        // Serialize VM configuration
        let config_json = serde_json::to_string(&vm.config)?;
        let state_str = format!("{:?}", vm.state);

        // Insert into Hiqlite (distributed write)
        self.hiqlite.query_write(
            "INSERT INTO vms (id, config, state, pid, started_at, updated_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            vec![
                Param::Text(vm_id.to_string()),
                Param::Text(config_json),
                Param::Text(state_str.clone()),
                Param::from(vm.pid.map(|p| p as i64)),
                Param::from(vm.started_at as i64),
                Param::from(chrono::Utc::now().timestamp()),
                Param::Text(serde_json::to_string(&vm.metadata)?),
            ],
        ).await?;

        // Log event
        self.log_event_internal(vm_id, "registered", None).await?;

        // Update in-memory cache
        self.vms.insert(vm_id, Arc::new(RwLock::new(vm)));
        self.by_state.entry(state_str).or_default().insert(vm_id);

        Ok(())
    }

    /// Update VM state (distributed operation)
    pub async fn update_state(&self, vm_id: Uuid, new_state: VmState) -> Result<()> {
        let state_str = format!("{:?}", new_state);

        // Update in Hiqlite (Raft-replicated write)
        let rows = self.hiqlite.query_write(
            "UPDATE vms SET state = ?1, updated_at = ?2 WHERE id = ?3",
            vec![
                Param::Text(state_str.clone()),
                Param::from(chrono::Utc::now().timestamp()),
                Param::Text(vm_id.to_string()),
            ],
        ).await?;

        if rows == 0 {
            return Err(anyhow!("VM not found: {}", vm_id));
        }

        // Log state change event
        self.log_event_internal(
            vm_id,
            "state_changed",
            Some(format!("new_state: {}", state_str)),
        ).await?;

        // Update cache
        if let Some(vm_lock) = self.vms.get(&vm_id) {
            let mut vm = vm_lock.write().await;

            // Remove from old state index
            let old_state_str = format!("{:?}", vm.state);
            if let Some(mut old_set) = self.by_state.get_mut(&old_state_str) {
                old_set.remove(&vm_id);
            }

            // Update state
            vm.state = new_state;

            // Add to new state index
            self.by_state.entry(state_str).or_default().insert(vm_id);
        }

        Ok(())
    }

    /// List VMs by state (uses Hiqlite for consistency)
    pub async fn list_by_state(&self, state: VmState) -> Result<Vec<VmInstance>> {
        let state_str = format!("{:?}", state);

        // Query from Hiqlite (ensures consistency across nodes)
        let rows = self.hiqlite.query_read(
            "SELECT id, config, pid, started_at, metadata
             FROM vms
             WHERE state = ?1
             ORDER BY started_at DESC",
            vec![Param::Text(state_str)],
        ).await?;

        let mut vms = Vec::new();
        for row in rows {
            // Reconstruct VM instance from Hiqlite data
            let id: String = row.get(0)?;
            let config_json: String = row.get(1)?;
            let pid: Option<i64> = row.get(2)?;
            let started_at: i64 = row.get(3)?;
            let metadata_json: String = row.get(4)?;

            let config: VmConfig = serde_json::from_str(&config_json)?;
            let metadata = serde_json::from_str(&metadata_json)?;

            let vm = VmInstance {
                config,
                state: state.clone(),
                pid: pid.map(|p| p as u32),
                started_at: started_at as u64,
                control_socket: None,  // Would need to store this too
                job_dir: None,
                metadata,
            };

            vms.push(vm);
        }

        Ok(vms)
    }

    /// Get VM statistics from Hiqlite
    pub async fn get_stats(&self) -> Result<VmStats> {
        // Single query to get all stats (atomic read across cluster)
        let row = self.hiqlite.query_read_one(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN state = 'Ready' THEN 1 ELSE 0 END) as ready,
                SUM(CASE WHEN state LIKE 'Busy%' THEN 1 ELSE 0 END) as busy,
                SUM(CASE WHEN state LIKE 'Failed%' THEN 1 ELSE 0 END) as failed
             FROM vms",
            vec![],
        ).await?;

        Ok(VmStats {
            total_vms: row.get(0)?,
            ready_vms: row.get(1)?,
            busy_vms: row.get(2)?,
            failed_vms: row.get(3)?,
        })
    }

    /// Recover VMs from Hiqlite on startup
    async fn recover_from_hiqlite(&self) -> Result<usize> {
        let rows = self.hiqlite.query_read(
            "SELECT id, config, state, pid, started_at, metadata FROM vms",
            vec![],
        ).await?;

        let mut count = 0;
        for row in rows {
            let id: String = row.get(0)?;
            let config_json: String = row.get(1)?;
            let state_str: String = row.get(2)?;
            let pid: Option<i64> = row.get(3)?;
            let started_at: i64 = row.get(4)?;
            let metadata_json: String = row.get(5)?;

            let vm_id = Uuid::parse_str(&id)?;
            let config: VmConfig = serde_json::from_str(&config_json)?;

            // Parse state (would need proper deserialization)
            let state = match state_str.as_str() {
                "Ready" => VmState::Ready,
                "Starting" => VmState::Starting,
                s if s.starts_with("Busy") => VmState::Busy {
                    job_id: String::new(),
                    started_at: 0,
                },
                _ => VmState::Terminated { exit_code: None },
            };

            let vm = VmInstance {
                config,
                state: state.clone(),
                pid: pid.map(|p| p as u32),
                started_at: started_at as u64,
                control_socket: None,
                job_dir: None,
                metadata: serde_json::from_str(&metadata_json)?,
            };

            self.vms.insert(vm_id, Arc::new(RwLock::new(vm)));
            self.by_state.entry(state_str).or_default().insert(vm_id);
            count += 1;
        }

        Ok(count)
    }

    /// Log VM event to distributed audit trail
    async fn log_event_internal(&self, vm_id: Uuid, event: &str, data: Option<String>) -> Result<()> {
        let node_id = self.hiqlite.get_node_id().await?;

        self.hiqlite.query_write(
            "INSERT INTO vm_events (vm_id, event_type, event_data, timestamp, node_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            vec![
                Param::Text(vm_id.to_string()),
                Param::Text(event.to_string()),
                Param::from(data),
                Param::from(chrono::Utc::now().timestamp()),
                Param::Text(node_id),
            ],
        ).await?;

        Ok(())
    }

    /// Clean up terminated VMs older than specified duration
    pub async fn cleanup_old_vms(&self, max_age_secs: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - max_age_secs;

        let deleted = self.hiqlite.query_write(
            "DELETE FROM vms
             WHERE state LIKE 'Terminated%'
             AND updated_at < ?1",
            vec![Param::from(cutoff)],
        ).await?;

        // Also clean cache
        for entry in self.vms.iter() {
            let vm = entry.value().read().await;
            if matches!(vm.state, VmState::Terminated { .. }) {
                self.vms.remove(entry.key());
            }
        }

        Ok(deleted as usize)
    }
}

#[derive(Debug, Clone)]
pub struct VmStats {
    pub total_vms: usize,
    pub ready_vms: usize,
    pub busy_vms: usize,
    pub failed_vms: usize,
}

// Implement required Hiqlite traits
impl HiqliteService {
    pub async fn query_write(&self, sql: &str, params: Vec<Param>) -> Result<usize> {
        // This would call the actual Hiqlite client
        // self.client.execute(sql, params).await
        Ok(0)  // Placeholder
    }

    pub async fn query_read(&self, sql: &str, params: Vec<Param>) -> Result<Vec<Row>> {
        // This would call the actual Hiqlite client
        // self.client.query(sql, params).await
        Ok(vec![])  // Placeholder
    }

    pub async fn query_read_one(&self, sql: &str, params: Vec<Param>) -> Result<Row> {
        // This would call the actual Hiqlite client
        // self.client.query_one(sql, params).await
        Err(anyhow!("Not implemented"))  // Placeholder
    }

    pub async fn get_node_id(&self) -> Result<String> {
        Ok("node-1".to_string())  // Placeholder
    }
}

// Placeholder for row type
struct Row;

impl Row {
    fn get<T>(&self, _index: usize) -> Result<T> {
        Err(anyhow!("Not implemented"))
    }
}