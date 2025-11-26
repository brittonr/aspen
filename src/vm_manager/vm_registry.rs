// VM Registry - Tracks all VMs with Hiqlite distributed persistence

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::vm_types::{JobRequirements, VmConfig, VmInstance, VmState};
use crate::hiqlite_service::HiqliteService;
use crate::params;

/// Registry for tracking all VM instances with distributed persistence
pub struct VmRegistry {
    /// In-memory cache for fast lookups
    vms: DashMap<Uuid, Arc<RwLock<VmInstance>>>,
    /// Index by state for efficient queries
    by_state: DashMap<String, HashSet<Uuid>>,
    /// Hiqlite client for distributed persistence
    hiqlite: Arc<HiqliteService>,
    /// Node ID for tracking VM ownership
    node_id: String,
}

impl VmRegistry {
    /// Create new registry with Hiqlite persistence
    pub async fn new(hiqlite: Arc<HiqliteService>, state_dir: &Path) -> Result<Self> {
        // Ensure directory exists for local resources
        std::fs::create_dir_all(state_dir)?;

        // Get node ID for VM ownership tracking
        let node_id = std::env::var("HQL_NODE_ID")
            .unwrap_or_else(|_| format!("node-{}", uuid::Uuid::new_v4()));

        let registry = Self {
            vms: DashMap::new(),
            by_state: DashMap::new(),
            hiqlite,
            node_id,
        };

        // Recover existing VMs from Hiqlite
        registry.recover_from_persistence().await?;

        Ok(registry)
    }

    /// Register a new VM instance
    pub async fn register(&self, vm: VmInstance) -> Result<()> {
        let vm_id = vm.config.id;

        // Persist to Hiqlite (distributed write)
        self.persist_vm(&vm).await?;

        // Update state index
        let state_key = format!("{:?}", vm.state);
        self.by_state
            .entry(state_key.clone())
            .or_insert_with(HashSet::new)
            .insert(vm_id);

        // Store in cache (clone for logging after move)
        let ip_address = vm.ip_address.clone();
        self.vms.insert(vm_id, Arc::new(RwLock::new(vm)));

        // Log registration event
        self.log_event(vm_id, "registered", None).await?;

        tracing::info!(
            vm_id = %vm_id,
            state = state_key,
            ip_address = ?ip_address,
            "VM registered"
        );

        Ok(())
    }

    /// Update VM state
    pub async fn update_state(&self, vm_id: Uuid, new_state: VmState) -> Result<()> {
        // Get VM from cache
        let vm_arc = self
            .vms
            .get(&vm_id)
            .ok_or_else(|| anyhow!("VM not found: {}", vm_id))?;

        let mut vm = vm_arc.write().await;
        let old_state = format!("{:?}", vm.state);
        let new_state_str = format!("{:?}", new_state);

        // Update state index
        if let Some(mut old_set) = self.by_state.get_mut(&old_state) {
            old_set.remove(&vm_id);
        }
        self.by_state
            .entry(new_state_str.clone())
            .or_insert_with(HashSet::new)
            .insert(vm_id);

        // Update VM state
        vm.state = new_state.clone();

        // Persist to Hiqlite
        let now = chrono::Utc::now().timestamp();
        self.hiqlite
            .execute(
                "UPDATE vms SET state = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_state_str.clone(), now, vm_id.to_string()],
            )
            .await?;

        // Log state change
        self.log_event(
            vm_id,
            "state_changed",
            Some(format!("{} -> {}", old_state, new_state_str)),
        )
        .await?;

        Ok(())
    }

    /// Update VM metrics
    pub async fn update_metrics(&self, vm_id: Uuid, metrics: crate::vm_manager::vm_types::VmMetrics) -> Result<()> {
        let vm_arc = self
            .vms
            .get(&vm_id)
            .ok_or_else(|| anyhow!("VM not found: {}", vm_id))?;

        let mut vm = vm_arc.write().await;
        vm.metrics = metrics.clone();

        // Persist metrics to Hiqlite
        let metrics_json = serde_json::to_string(&metrics)?;
        self.hiqlite
            .execute(
                "UPDATE vms SET metrics = ?1, updated_at = ?2 WHERE id = ?3",
                params![
                    metrics_json,
                    chrono::Utc::now().timestamp(),
                    vm_id.to_string()
                ],
            )
            .await?;

        Ok(())
    }

    /// Get VM by ID
    pub async fn get(&self, vm_id: Uuid) -> Result<Option<Arc<RwLock<VmInstance>>>> {
        Ok(self.vms.get(&vm_id).map(|entry| Arc::clone(entry.value())))
    }

    /// List all VMs
    pub async fn list_all(&self) -> Result<Vec<VmInstance>> {
        let mut vms = Vec::new();
        for entry in self.vms.iter() {
            let vm = entry.value().read().await;
            vms.push(vm.clone());
        }
        Ok(vms)
    }

    /// List VMs by state
    pub async fn list_by_state(&self, state: &str) -> Result<Vec<VmInstance>> {
        let mut vms = Vec::new();
        if let Some(vm_ids) = self.by_state.get(state) {
            for vm_id in vm_ids.iter() {
                if let Some(vm_arc) = self.vms.get(vm_id) {
                    let vm = vm_arc.read().await;
                    vms.push(vm.clone());
                }
            }
        }
        Ok(vms)
    }

    /// List all VMs (for compatibility)
    pub async fn list_all_vms(&self) -> Result<Vec<VmInstance>> {
        self.list_all().await
    }

    /// List running VMs (for health checking)
    pub async fn list_running_vms(&self) -> Result<Vec<VmInstance>> {
        let mut vms = Vec::new();
        for entry in self.vms.iter() {
            let vm = entry.value().read().await;
            if vm.state.is_running() {
                vms.push(vm.clone());
            }
        }
        Ok(vms)
    }

    /// Get an available service VM
    pub async fn get_available_service_vm(&self) -> Option<Uuid> {
        for entry in self.vms.iter() {
            let vm = entry.value().read().await;
            if vm.state.is_available() && matches!(vm.config.mode, super::vm_types::VmMode::Service { .. }) {
                return Some(vm.config.id);
            }
        }
        None
    }

    /// List VMs by state pattern (for router)
    pub async fn list_vms_by_state(&self, state: VmState) -> Result<Vec<VmInstance>> {
        let state_key = format!("{:?}", state);
        self.list_by_state(&state_key).await
    }

    /// Find idle VM for job requirements
    pub async fn find_idle_vm(&self, requirements: &JobRequirements) -> Option<Uuid> {
        for entry in self.vms.iter() {
            let vm = entry.value().read().await;

            if vm.state == VmState::Ready {
                // Check if VM meets requirements
                if vm.config.memory_mb >= requirements.memory_mb
                    && vm.config.vcpus >= requirements.vcpus
                {
                    return Some(vm.config.id);
                }
            }
        }
        None
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

    /// Remove VM
    pub async fn remove(&self, vm_id: Uuid) -> Result<()> {
        // Get VM for state info
        let vm_arc = self
            .vms
            .remove(&vm_id)
            .ok_or_else(|| anyhow!("VM not found: {}", vm_id))?
            .1;

        let vm = vm_arc.read().await;
        let state_key = format!("{:?}", vm.state);

        // Update state index
        if let Some(mut set) = self.by_state.get_mut(&state_key) {
            set.remove(&vm_id);
        }

        // Remove from Hiqlite
        self.hiqlite
            .execute("DELETE FROM vms WHERE id = ?1", params![vm_id.to_string()])
            .await?;

        // Log removal
        self.log_event(vm_id, "removed", None).await?;

        Ok(())
    }

    /// Log VM event
    pub async fn log_event(&self, vm_id: Uuid, event_type: &str, details: Option<String>) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        self.hiqlite
            .execute(
                "INSERT INTO vm_events (vm_id, event_type, event_data, timestamp, node_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    vm_id.to_string(),
                    event_type,
                    details,
                    now,
                    self.node_id.clone()
                ],
            )
            .await?;

        Ok(())
    }

    /// Recover VMs from Hiqlite on startup
    pub async fn recover_from_persistence(&self) -> Result<usize> {
        // Query all VMs owned by this node
        let rows = self.hiqlite
            .query_as::<VmRow>(
                "SELECT id, config, state, created_at, updated_at, pid, control_socket,
                        job_dir, ip_address, metrics, node_id
                 FROM vms
                 WHERE node_id = ?1",
                params![self.node_id.clone()],
            )
            .await?;

        let mut recovered = 0;
        for row in rows {
            let vm_id = Uuid::parse_str(&row.id)?;
            let config: VmConfig = serde_json::from_str(&row.config)?;
            let state = parse_vm_state(&row.state)?;
            let metrics = row.metrics
                .as_ref()
                .and_then(|m| serde_json::from_str(m).ok())
                .unwrap_or_default();

            let mut vm = VmInstance {
                config,
                state: state.clone(),
                pid: row.pid.map(|p| p as u32),
                created_at: row.created_at,
                control_socket: row.control_socket.map(|s| s.into()),
                job_dir: row.job_dir.map(|s| s.into()),
                ip_address: row.ip_address,
                metrics,
            };

            // Check if process is still alive (for running VMs)
            if let Some(pid) = vm.pid {
                if !is_process_alive(pid) {
                    // Mark as failed if process died while we were down
                    vm.state = VmState::Failed {
                        error: "Process died while system was down".to_string(),
                    };

                    // Update in Hiqlite
                    if let Err(e) = self.update_state(vm_id, vm.state.clone()).await {
                        tracing::error!(vm_id = %vm_id, error = %e, "Failed to update VM state after detecting dead process");
                    }
                }
            }

            // Update state index
            let state_key = format!("{:?}", vm.state);
            self.by_state
                .entry(state_key)
                .or_insert_with(HashSet::new)
                .insert(vm_id);

            // Store in cache
            self.vms.insert(vm_id, Arc::new(RwLock::new(vm)));
            recovered += 1;
        }

        tracing::info!(recovered, node_id = %self.node_id, "VMs recovered from Hiqlite");
        Ok(recovered)
    }

    /// Cleanup stale VMs (mark as failed if owner node is dead)
    pub async fn cleanup_stale_vms(&self) -> Result<()> {
        // Get all VMs from all nodes
        let rows = self.hiqlite
            .query_as::<VmRow>(
                "SELECT id, node_id, state FROM vms WHERE state NOT LIKE '%Terminated%'",
                params![],
            )
            .await?;

        for row in rows {
            // Check if owner node is alive (would need heartbeat check)
            // For now, we only manage our own VMs
            if row.node_id != self.node_id {
                continue;
            }

            let vm_id = Uuid::parse_str(&row.id)?;
            let state = parse_vm_state(&row.state)?;

            // If it's supposed to be running but we don't have it in cache, it's stale
            if matches!(state, VmState::Ready | VmState::Starting | VmState::Busy { .. }) {
                if !self.vms.contains_key(&vm_id) {
                    // Mark as failed
                    self.hiqlite
                        .execute(
                            "UPDATE vms SET state = ?1, updated_at = ?2 WHERE id = ?3",
                            params![
                                "Failed",
                                chrono::Utc::now().timestamp(),
                                vm_id.to_string()
                            ],
                        )
                        .await?;

                    tracing::warn!(vm_id = %vm_id, "Marked stale VM as failed");
                }
            }
        }

        Ok(())
    }

    /// Persist VM to Hiqlite
    async fn persist_vm(&self, vm: &VmInstance) -> Result<()> {
        let config_json = serde_json::to_string(&vm.config)?;
        let state_str = format!("{:?}", vm.state);
        let metrics_json = serde_json::to_string(&vm.metrics)?;
        let now = chrono::Utc::now().timestamp();

        self.hiqlite
            .execute(
                "INSERT INTO vms (
                    id, config, state, created_at, updated_at, node_id,
                    pid, control_socket, job_dir, ip_address, metrics
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                    config = excluded.config,
                    state = excluded.state,
                    updated_at = excluded.updated_at,
                    pid = excluded.pid,
                    control_socket = excluded.control_socket,
                    job_dir = excluded.job_dir,
                    ip_address = excluded.ip_address,
                    metrics = excluded.metrics",
                params![
                    vm.config.id.to_string(),
                    config_json,
                    state_str,
                    vm.created_at,
                    now,
                    self.node_id.clone(),
                    vm.pid.map(|p| p as i64),
                    vm.control_socket.as_ref().map(|p| p.display().to_string()),
                    vm.job_dir.as_ref().map(|p| p.display().to_string()),
                    vm.ip_address.clone(),
                    metrics_json
                ],
            )
            .await?;

        Ok(())
    }
}

// Helper struct for deserializing VM rows from Hiqlite
#[derive(Debug, Clone, serde::Deserialize)]
struct VmRow {
    id: String,
    config: String,
    state: String,
    created_at: i64,
    updated_at: i64,
    node_id: String,
    pid: Option<i64>,
    control_socket: Option<String>,
    job_dir: Option<String>,
    ip_address: Option<String>,
    metrics: Option<String>,
}

impl From<hiqlite::Row<'static>> for VmRow {
    fn from(mut row: hiqlite::Row<'static>) -> Self {
        Self {
            id: row.get("id"),
            config: row.get("config"),
            state: row.get("state"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            node_id: row.get("node_id"),
            pid: row.get("pid"),
            control_socket: row.get("control_socket"),
            job_dir: row.get("job_dir"),
            ip_address: row.get("ip_address"),
            metrics: row.get("metrics"),
        }
    }
}

// Parse VM state from string representation
fn parse_vm_state(state_str: &str) -> Result<VmState> {
    // Simple parsing - would need proper implementation
    match state_str {
        "Starting" => Ok(VmState::Starting),
        "Ready" => Ok(VmState::Ready),
        "Idle" => Ok(VmState::Idle {
            jobs_completed: 0,
            last_job_at: 0,
        }),
        s if s.starts_with("Busy") => Ok(VmState::Busy {
            job_id: String::new(),
            started_at: 0,
        }),
        s if s.starts_with("Draining") => Ok(VmState::Draining),
        s if s.starts_with("Terminated") => Ok(VmState::Terminated {
            reason: String::from("Unknown"),
            exit_code: 0,
        }),
        s if s.starts_with("Failed") => Ok(VmState::Failed {
            error: String::new(),
        }),
        _ => Ok(VmState::Terminated {
            reason: String::from("Unknown state"),
            exit_code: 0,
        }),
    }
}

// Check if a process is alive
fn is_process_alive(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
}