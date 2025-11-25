// VM Registry - Tracks all VMs with SQLite persistence

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::vm_types::{JobRequirements, VmConfig, VmInstance, VmMode, VmState};

/// Registry for tracking all VM instances
pub struct VmRegistry {
    /// In-memory cache for fast lookups
    vms: DashMap<Uuid, Arc<RwLock<VmInstance>>>,
    /// Index by state for efficient queries
    by_state: DashMap<String, HashSet<Uuid>>,
    /// SQLite connection for persistence
    db_path: std::path::PathBuf,
}

impl VmRegistry {
    /// Create new registry with SQLite persistence
    pub async fn new(state_dir: &Path) -> Result<Self> {
        let db_path = state_dir.join("vm_registry.db");

        // Ensure directory exists
        std::fs::create_dir_all(state_dir)?;

        // Initialize database schema
        Self::init_database(&db_path)?;

        Ok(Self {
            vms: DashMap::new(),
            by_state: DashMap::new(),
            db_path,
        })
    }

    /// Initialize SQLite database schema
    fn init_database(db_path: &Path) -> Result<()> {
        let conn = Connection::open(db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS vms (
                id TEXT PRIMARY KEY,
                config TEXT NOT NULL,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                pid INTEGER,
                control_socket TEXT,
                job_dir TEXT,
                ip_address TEXT,
                metrics TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS vm_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                vm_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                details TEXT,
                FOREIGN KEY (vm_id) REFERENCES vms(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_vms_state ON vms(state)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_events_vm_id ON vm_events(vm_id)",
            [],
        )?;

        Ok(())
    }

    /// Register a new VM instance
    pub async fn register(&self, vm: VmInstance) -> Result<()> {
        let vm_id = vm.config.id;

        // Persist to database
        self.persist_vm(&vm).await?;

        // Update state index
        let state_key = format!("{:?}", vm.state);
        self.by_state
            .entry(state_key.clone())
            .or_insert_with(HashSet::new)
            .insert(vm_id);

        // Store in memory cache
        self.vms.insert(vm_id, Arc::new(RwLock::new(vm)));

        tracing::info!(vm_id = %vm_id, state = %state_key, "VM registered");

        Ok(())
    }

    /// Get VM by ID
    pub async fn get(&self, vm_id: Uuid) -> Result<Option<Arc<RwLock<VmInstance>>>> {
        Ok(self.vms.get(&vm_id).map(|entry| Arc::clone(&entry)))
    }

    /// Find available service VM matching requirements
    pub async fn get_available_service_vm(
        &self,
        requirements: &JobRequirements,
    ) -> Option<Uuid> {
        // Look for idle service VMs
        for entry in self.vms.iter() {
            let vm_lock = entry.value();
            let vm = match vm_lock.try_read() {
                Ok(vm) => vm,
                Err(_) => continue, // Skip if locked
            };

            // Check if VM is service mode and available
            if !matches!(vm.config.mode, VmMode::Service { .. }) {
                continue;
            }

            if !vm.state.is_available() {
                continue;
            }

            // Check if VM meets requirements
            if vm.config.memory_mb < requirements.memory_mb {
                continue;
            }

            if vm.config.vcpus < requirements.vcpus {
                continue;
            }

            if vm.config.isolation_level != requirements.isolation_level {
                continue;
            }

            // Check capabilities
            let has_all_capabilities = requirements
                .capabilities
                .iter()
                .all(|cap| vm.config.capabilities.contains(cap));

            if !has_all_capabilities {
                continue;
            }

            // Check if VM should be recycled
            if vm.should_recycle() {
                continue;
            }

            return Some(vm.config.id);
        }

        None
    }

    /// Update VM state
    pub async fn update_state(&self, vm_id: Uuid, new_state: VmState) -> Result<()> {
        if let Some(entry) = self.vms.get(&vm_id) {
            let mut vm = entry.write().await;
            let old_state_key = format!("{:?}", vm.state);
            let new_state_key = format!("{:?}", new_state);

            vm.state = new_state.clone();

            // Update state index
            if let Some(mut old_set) = self.by_state.get_mut(&old_state_key) {
                old_set.remove(&vm_id);
            }

            self.by_state
                .entry(new_state_key.clone())
                .or_insert_with(HashSet::new)
                .insert(vm_id);

            // Persist state change
            self.persist_state_change(&vm_id, &new_state).await?;

            tracing::debug!(
                vm_id = %vm_id,
                old_state = %old_state_key,
                new_state = %new_state_key,
                "VM state updated"
            );
        }

        Ok(())
    }

    /// List all VMs
    pub async fn list_all_vms(&self) -> Result<Vec<VmInstance>> {
        let mut vms = Vec::new();

        for entry in self.vms.iter() {
            let vm = entry.read().await;
            vms.push(vm.clone());
        }

        Ok(vms)
    }

    /// List VMs in specific state
    pub async fn list_vms_by_state(&self, state: VmState) -> Result<Vec<VmInstance>> {
        let state_key = format!("{:?}", state);
        let mut vms = Vec::new();

        if let Some(vm_ids) = self.by_state.get(&state_key) {
            for vm_id in vm_ids.iter() {
                if let Some(entry) = self.vms.get(vm_id) {
                    let vm = entry.read().await;
                    vms.push(vm.clone());
                }
            }
        }

        Ok(vms)
    }

    /// List running VMs
    pub async fn list_running_vms(&self) -> Result<Vec<VmInstance>> {
        let mut vms = Vec::new();

        for entry in self.vms.iter() {
            let vm = entry.read().await;
            if vm.state.is_running() {
                vms.push(vm.clone());
            }
        }

        Ok(vms)
    }

    /// Count all VMs
    pub async fn count_all(&self) -> usize {
        self.vms.len()
    }

    /// Count VMs by state
    pub async fn count_by_state(&self, state: VmState) -> usize {
        let state_key = format!("{:?}", state);
        self.by_state
            .get(&state_key)
            .map(|set| set.len())
            .unwrap_or(0)
    }

    /// Remove VM from registry
    pub async fn remove(&self, vm_id: Uuid) -> Result<()> {
        if let Some((_, vm_arc)) = self.vms.remove(&vm_id) {
            let vm = vm_arc.read().await;
            let state_key = format!("{:?}", vm.state);

            // Remove from state index
            if let Some(mut set) = self.by_state.get_mut(&state_key) {
                set.remove(&vm_id);
            }

            // Remove from database
            self.delete_from_db(&vm_id).await?;

            tracing::info!(vm_id = %vm_id, "VM removed from registry");
        }

        Ok(())
    }

    /// Persist VM to database
    async fn persist_vm(&self, vm: &VmInstance) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;

        conn.execute(
            "INSERT OR REPLACE INTO vms (
                id, config, state, created_at, updated_at, pid,
                control_socket, job_dir, ip_address, metrics
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                vm.config.id.to_string(),
                serde_json::to_string(&vm.config)?,
                serde_json::to_string(&vm.state)?,
                vm.created_at,
                chrono::Utc::now().timestamp(),
                vm.pid.map(|p| p as i64),
                vm.control_socket.as_ref().and_then(|p| p.to_str()),
                vm.job_dir.as_ref().and_then(|p| p.to_str()),
                vm.ip_address.as_ref(),
                serde_json::to_string(&vm.metrics)?,
            ],
        )?;

        Ok(())
    }

    /// Persist state change
    async fn persist_state_change(&self, vm_id: &Uuid, state: &VmState) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;

        conn.execute(
            "UPDATE vms SET state = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                serde_json::to_string(state)?,
                chrono::Utc::now().timestamp(),
                vm_id.to_string(),
            ],
        )?;

        Ok(())
    }

    /// Delete VM from database
    async fn delete_from_db(&self, vm_id: &Uuid) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;

        conn.execute(
            "DELETE FROM vms WHERE id = ?1",
            params![vm_id.to_string()],
        )?;

        Ok(())
    }

    /// Recover VMs from persistence (on startup)
    pub async fn recover_from_persistence(&self) -> Result<usize> {
        let conn = Connection::open(&self.db_path)?;

        let mut stmt = conn.prepare(
            "SELECT id, config, state, created_at, pid, control_socket,
                    job_dir, ip_address, metrics
             FROM vms WHERE state NOT IN ('Terminated', 'Failed')",
        )?;

        let vm_iter = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let config: String = row.get(1)?;
            let state: String = row.get(2)?;
            let created_at: i64 = row.get(3)?;
            let pid: Option<i64> = row.get(4)?;
            let control_socket: Option<String> = row.get(5)?;
            let job_dir: Option<String> = row.get(6)?;
            let ip_address: Option<String> = row.get(7)?;
            let metrics: String = row.get(8)?;

            Ok((
                id, config, state, created_at, pid, control_socket, job_dir, ip_address, metrics,
            ))
        })?;

        let mut count = 0;
        for vm_result in vm_iter {
            let (id, config, state, created_at, pid, control_socket, job_dir, ip_address, metrics) =
                vm_result?;

            let vm_id = Uuid::parse_str(&id)?;
            let config: VmConfig = serde_json::from_str(&config)?;
            let state: VmState = serde_json::from_str(&state)?;
            let metrics = serde_json::from_str(&metrics)?;

            let vm = VmInstance {
                config: config.clone(),
                state: state.clone(),
                created_at,
                pid: pid.map(|p| p as u32),
                control_socket: control_socket.map(PathBuf::from),
                job_dir: job_dir.map(PathBuf::from),
                ip_address,
                metrics,
            };

            // Update state index
            let state_key = format!("{:?}", state);
            self.by_state
                .entry(state_key)
                .or_insert_with(HashSet::new)
                .insert(vm_id);

            // Store in memory cache
            self.vms.insert(vm_id, Arc::new(RwLock::new(vm)));

            count += 1;
        }

        Ok(count)
    }

    /// Clean up stale VMs (processes that no longer exist)
    pub async fn cleanup_stale_vms(&self) -> Result<()> {
        let vms = self.list_running_vms().await?;

        for vm in vms {
            if let Some(pid) = vm.pid {
                // Check if process still exists
                let pid = nix::unistd::Pid::from_raw(pid as i32);
                if nix::sys::signal::kill(pid, None).is_err() {
                    // Process doesn't exist, mark as failed
                    self.update_state(
                        vm.config.id,
                        VmState::Failed {
                            error: "Process no longer exists".to_string(),
                        },
                    )
                    .await?;

                    tracing::warn!(
                        vm_id = %vm.config.id,
                        pid = %pid,
                        "Marked stale VM as failed"
                    );
                }
            }
        }

        Ok(())
    }

    /// Log VM event for auditing
    pub async fn log_event(
        &self,
        vm_id: Uuid,
        event_type: &str,
        details: Option<serde_json::Value>,
    ) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;

        conn.execute(
            "INSERT INTO vm_events (vm_id, timestamp, event_type, details) VALUES (?1, ?2, ?3, ?4)",
            params![
                vm_id.to_string(),
                chrono::Utc::now().timestamp(),
                event_type,
                details.map(|d| d.to_string()),
            ],
        )?;

        Ok(())
    }
}