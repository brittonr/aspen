// VM Registry - Tracks all VMs with Hiqlite distributed persistence
//
// CONSISTENCY GUARANTEES:
// 1. Hiqlite is the single source of truth (durable, replicated)
// 2. In-memory structures are caches that follow Hiqlite
// 3. State transitions are atomic: persist-first with rollback on failure
// 4. Recovery rebuilds indices from Hiqlite data

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::vm_types::{JobRequirements, VmConfig, VmInstance, VmState};
use crate::hiqlite::HiqliteService;
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

    /// Get consistent state index key
    ///
    /// Centralizes state-to-string conversion to ensure consistency
    /// between updates and lookups. Uses JSON serialization to preserve
    /// exact state representation including variant data.
    fn state_index_key(state: &VmState) -> String {
        // Always use JSON serialization for new data
        // This preserves all variant fields (job_id, timestamps, etc)
        serde_json::to_string(state).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize VM state: {}", e);
            // Emergency fallback - should never happen with derived Serialize
            format!("{{\"error\":\"serialization_failed\"}}")
        })
    }

    /// Deserialize VM state from stored string
    ///
    /// Handles both JSON (new format) and Debug (legacy format) representations
    fn deserialize_vm_state(state_str: &str) -> Result<VmState> {
        // Try JSON deserialization first (preferred)
        if let Ok(state) = serde_json::from_str::<VmState>(state_str) {
            return Ok(state);
        }

        // Fall back to legacy Debug format parsing
        // NOTE: This loses data! Only for backward compatibility
        tracing::warn!(
            "Using legacy Debug format parser for state: '{}' - data loss may occur!",
            state_str
        );
        parse_vm_state(state_str)
    }

    /// Migrate a VM's state from Debug to JSON format in database
    /// Called during recovery to fix legacy data
    async fn migrate_vm_state_format(&self, vm_id: &Uuid, vm: &VmInstance) -> Result<()> {
        let state_json = Self::state_index_key(&vm.state);
        let now = chrono::Utc::now().timestamp();

        self.hiqlite
            .execute(
                "UPDATE vms SET state = ?1, updated_at = ?2 WHERE id = ?3",
                params![state_json, now, vm_id.to_string()],
            )
            .await
            .map(|_| ())
            .map_err(|e| anyhow!("Failed to migrate VM state format: {}", e))
    }

    /// Register a new VM instance
    ///
    /// Atomicity: Hiqlite persisted first, then cache updated
    pub async fn register(&self, vm: VmInstance) -> Result<()> {
        let vm_id = vm.config.id;

        // PHASE 1: Persist to Hiqlite (single source of truth)
        self.persist_vm(&vm).await?;

        // PHASE 2: Update in-memory structures
        let state_key = Self::state_index_key(&vm.state);
        self.by_state
            .entry(state_key.clone())
            .or_insert_with(HashSet::new)
            .insert(vm_id);

        // Store in cache (clone for logging after move)
        let ip_address = vm.ip_address.clone();
        self.vms.insert(vm_id, Arc::new(RwLock::new(vm)));

        // PHASE 3: Log registration event (best-effort)
        if let Err(e) = self.log_event(vm_id, "registered", None).await {
            tracing::warn!(vm_id = %vm_id, error = %e, "Failed to log registration event");
        }

        tracing::info!(
            vm_id = %vm_id,
            state = state_key,
            ip_address = ?ip_address,
            "VM registered"
        );

        Ok(())
    }

    /// Update VM state with atomic guarantees
    ///
    /// CONSISTENCY STRATEGY:
    /// 1. Hiqlite is the single source of truth (durable, distributed)
    /// 2. Persist to Hiqlite first (write-through cache pattern)
    /// 3. Update in-memory structures only on success
    /// 4. Roll back database on in-memory failure
    ///
    /// This ensures:
    /// - No window where indices are inconsistent
    /// - Database always reflects current state
    /// - Recovery can rebuild from database
    /// - Thread-safe via RwLock on individual VMs
    pub async fn update_state(&self, vm_id: Uuid, new_state: VmState) -> Result<()> {
        // Get VM from cache
        let vm_arc = self
            .vms
            .get(&vm_id)
            .ok_or_else(|| anyhow!("VM not found: {}", vm_id))?
            .clone();

        // Acquire write lock early to prevent concurrent state changes to same VM
        let mut vm = vm_arc.write().await;
        let old_state = vm.state.clone();
        let old_state_key = Self::state_index_key(&old_state);
        let new_state_key = Self::state_index_key(&new_state);

        // Optimization: no-op if state unchanged
        if old_state == new_state {
            return Ok(());
        }

        tracing::debug!(
            vm_id = %vm_id,
            old_state = %old_state_key,
            new_state = %new_state_key,
            "Starting state transition"
        );

        // PHASE 1: Persist to Hiqlite (single source of truth, durable)
        let now = chrono::Utc::now().timestamp();
        let state_json = Self::state_index_key(&new_state); // JSON serialized

        match self.hiqlite
            .execute(
                "UPDATE vms SET state = ?1, updated_at = ?2 WHERE id = ?3",
                params![state_json, now, vm_id.to_string()],
            )
            .await
        {
            Ok(_) => {
                tracing::trace!(vm_id = %vm_id, "Hiqlite state persisted successfully");
            }
            Err(e) => {
                // Persistence failed - no state was changed
                tracing::error!(
                    vm_id = %vm_id,
                    old_state = %old_state_key,
                    new_state = %new_state_key,
                    error = %e,
                    "Failed to persist state change to Hiqlite - aborting"
                );
                return Err(anyhow!("Persistence failure: {}", e));
            }
        }

        // PHASE 2: Update in-memory structures atomically
        // At this point, Hiqlite is updated. We must succeed or rollback.

        // Use panic guard to detect catastrophic failures
        let index_update_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Remove from old state index
            if let Some(mut old_set) = self.by_state.get_mut(&old_state_key) {
                old_set.remove(&vm_id);
                tracing::trace!(
                    vm_id = %vm_id,
                    old_state = %old_state_key,
                    "Removed from old state index"
                );
            }

            // Insert into new state index
            self.by_state
                .entry(new_state_key.clone())
                .or_insert_with(HashSet::new)
                .insert(vm_id);

            tracing::trace!(
                vm_id = %vm_id,
                new_state = %new_state_key,
                "Added to new state index"
            );
        }));

        // Handle catastrophic index update failure
        if let Err(panic_err) = index_update_result {
            tracing::error!(
                vm_id = %vm_id,
                old_state = %old_state_key,
                new_state = %new_state_key,
                "CRITICAL: State index update panicked - attempting rollback"
            );

            // Attempt to rollback Hiqlite to old state
            let old_state_json = Self::state_index_key(&old_state);
            if let Err(rollback_err) = self.hiqlite
                .execute(
                    "UPDATE vms SET state = ?1, updated_at = ?2 WHERE id = ?3",
                    params![old_state_json, now, vm_id.to_string()],
                )
                .await
            {
                tracing::error!(
                    vm_id = %vm_id,
                    error = %rollback_err,
                    "CRITICAL: Failed to rollback state after index panic - DATABASE INCONSISTENT"
                );
                // At this point, manual intervention required
                // Database has new state, but indices may be corrupted
            } else {
                tracing::warn!(
                    vm_id = %vm_id,
                    "Rollback successful after index panic"
                );
            }

            return Err(anyhow!("State index update failed catastrophically: {:?}", panic_err));
        }

        // Update VM instance state
        vm.state = new_state.clone();
        tracing::trace!(vm_id = %vm_id, "VM instance state updated");

        // PHASE 3: Log state change (best-effort, non-critical)
        // Even if logging fails, we've achieved consistency
        if let Err(e) = self.log_event(
            vm_id,
            "state_changed",
            Some(format!("{} -> {}", old_state_key, new_state_key)),
        ).await {
            tracing::warn!(
                vm_id = %vm_id,
                error = %e,
                "Failed to log state change event (non-critical)"
            );
        }

        tracing::info!(
            vm_id = %vm_id,
            old_state = %old_state_key,
            new_state = %new_state_key,
            "State transition completed atomically"
        );

        Ok(())
    }

    /// Update VM metrics
    pub async fn update_metrics(&self, vm_id: Uuid, metrics: crate::infrastructure::vm::vm_types::VmMetrics) -> Result<()> {
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
        let state_key = Self::state_index_key(&state);
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
        let state_key = Self::state_index_key(&state);
        self.by_state
            .get(&state_key)
            .map(|set| set.len())
            .unwrap_or(0)
    }

    /// Remove VM
    ///
    /// Atomicity: Database removed first, then cache
    pub async fn remove(&self, vm_id: Uuid) -> Result<()> {
        // Get VM for state info before removal
        let vm_arc = self
            .vms
            .get(&vm_id)
            .ok_or_else(|| anyhow!("VM not found: {}", vm_id))?;

        let vm = vm_arc.read().await;
        let state_key = Self::state_index_key(&vm.state);
        drop(vm); // Release read lock

        // PHASE 1: Remove from Hiqlite (single source of truth)
        self.hiqlite
            .execute("DELETE FROM vms WHERE id = ?1", params![vm_id.to_string()])
            .await?;

        // PHASE 2: Remove from in-memory structures
        // Remove from state index
        if let Some(mut set) = self.by_state.get_mut(&state_key) {
            set.remove(&vm_id);
        }

        // Remove from cache
        self.vms.remove(&vm_id);

        // PHASE 3: Log removal (best-effort)
        if let Err(e) = self.log_event(vm_id, "removed", None).await {
            tracing::warn!(vm_id = %vm_id, error = %e, "Failed to log removal event");
        }

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
    ///
    /// Rebuilds in-memory indices from Hiqlite (single source of truth)
    /// Performs consistency checks and self-healing
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
        let mut healed = 0;

        for row in rows {
            let vm_id = Uuid::parse_str(&row.id)?;
            let config: VmConfig = serde_json::from_str(&row.config)?;

            // Deserialize state with fallback handling
            let (state, needs_migration) = match Self::deserialize_vm_state(&row.state) {
                Ok(state) => {
                    // Check if this was parsed from legacy format
                    let is_legacy = !row.state.starts_with("{") && !row.state.starts_with("[");
                    (state, is_legacy)
                },
                Err(e) => {
                    tracing::warn!(
                        vm_id = %vm_id,
                        error = %e,
                        state_str = %row.state,
                        "Failed to deserialize VM state, using Failed state"
                    );
                    (VmState::Failed {
                        error: format!("State deserialization failed: {}", e),
                    }, true)
                }
            };

            let metrics = row.metrics
                .as_ref()
                .and_then(|m| serde_json::from_str(m).ok())
                .unwrap_or_default();

            let mut vm = VmInstance {
                config: Arc::new(config),
                state: state.clone(),
                pid: row.pid.map(|p| p as u32),
                created_at: row.created_at,
                control_socket: row.control_socket.map(|s| s.into()),
                job_dir: row.job_dir.map(|s| s.into()),
                ip_address: row.ip_address,
                metrics,
            };

            // Self-healing: Check if process is still alive (for running VMs)
            if let Some(pid) = vm.pid {
                if vm.state.is_running() && !is_process_alive(pid) {
                    tracing::warn!(
                        vm_id = %vm_id,
                        pid = pid,
                        "Process died while system was down - marking as failed"
                    );

                    // Mark as failed
                    vm.state = VmState::Failed {
                        error: "Process died while system was down".to_string(),
                    };

                    // Update in Hiqlite
                    let new_state_json = Self::state_index_key(&vm.state);
                    if let Err(e) = self.hiqlite
                        .execute(
                            "UPDATE vms SET state = ?1, updated_at = ?2 WHERE id = ?3",
                            params![
                                new_state_json,
                                chrono::Utc::now().timestamp(),
                                vm_id.to_string()
                            ],
                        )
                        .await
                    {
                        tracing::error!(
                            vm_id = %vm_id,
                            error = %e,
                            "Failed to update VM state after detecting dead process"
                        );
                    } else {
                        healed += 1;
                    }
                }
            }

            // Migrate legacy state format to JSON if needed
            if needs_migration {
                tracing::info!(
                    vm_id = %vm_id,
                    "Migrating VM state from legacy Debug format to JSON"
                );
                if let Err(e) = self.migrate_vm_state_format(&vm_id, &vm).await {
                    tracing::error!(
                        vm_id = %vm_id,
                        error = %e,
                        "Failed to migrate VM state format"
                    );
                }
            }

            // Rebuild state index from recovered VMs
            let state_key = Self::state_index_key(&vm.state);
            self.by_state
                .entry(state_key)
                .or_insert_with(HashSet::new)
                .insert(vm_id);

            // Store in cache
            self.vms.insert(vm_id, Arc::new(RwLock::new(vm)));
            recovered += 1;
        }

        tracing::info!(
            recovered,
            healed,
            node_id = %self.node_id,
            "VMs recovered from Hiqlite with self-healing"
        );

        Ok(recovered)
    }

    /// Verify consistency between Hiqlite and in-memory state
    ///
    /// Returns (total_checked, inconsistencies_found, inconsistencies_fixed)
    pub async fn verify_consistency(&self) -> Result<(usize, usize, usize)> {
        let mut checked = 0;
        let mut found = 0;
        let mut fixed = 0;

        // Get all VMs from Hiqlite (source of truth)
        let rows = self.hiqlite
            .query_as::<VmRow>(
                "SELECT id, state FROM vms WHERE node_id = ?1",
                params![self.node_id.clone()],
            )
            .await?;

        for row in rows {
            checked += 1;
            let vm_id = Uuid::parse_str(&row.id)?;

            let db_state = Self::deserialize_vm_state(&row.state)?;
            let db_state_key = Self::state_index_key(&db_state);

            // Check in-memory cache
            if let Some(vm_arc) = self.vms.get(&vm_id) {
                let vm = vm_arc.read().await;
                let mem_state_key = Self::state_index_key(&vm.state);

                // Check if states match
                if db_state_key != mem_state_key {
                    found += 1;
                    tracing::warn!(
                        vm_id = %vm_id,
                        db_state = %db_state_key,
                        mem_state = %mem_state_key,
                        "Inconsistency detected: database and memory states differ"
                    );

                    // Fix by using database as source of truth
                    drop(vm); // Release read lock
                    if let Err(e) = self.update_state(vm_id, db_state.clone()).await {
                        tracing::error!(
                            vm_id = %vm_id,
                            error = %e,
                            "Failed to fix inconsistency"
                        );
                    } else {
                        fixed += 1;
                    }
                }

                // Check if VM is in correct state index
                if !self.by_state.get(&db_state_key).map_or(false, |set| set.contains(&vm_id)) {
                    found += 1;
                    tracing::warn!(
                        vm_id = %vm_id,
                        state = %db_state_key,
                        "Inconsistency detected: VM missing from state index"
                    );

                    // Fix index
                    self.by_state
                        .entry(db_state_key.clone())
                        .or_insert_with(HashSet::new)
                        .insert(vm_id);
                    fixed += 1;
                }
            } else {
                found += 1;
                tracing::warn!(
                    vm_id = %vm_id,
                    "Inconsistency detected: VM in database but not in cache"
                );
                // This is recovered by the next call to recover_from_persistence
            }
        }

        if found > 0 {
            tracing::warn!(
                checked,
                found,
                fixed,
                "Consistency verification completed with issues"
            );
        } else {
            tracing::debug!(
                checked,
                "Consistency verification completed - all VMs consistent"
            );
        }

        Ok((checked, found, fixed))
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
            let state = Self::deserialize_vm_state(&row.state).unwrap_or_else(|_| {
                VmState::Failed {
                    error: "Unknown state".to_string(),
                }
            });

            // If it's supposed to be running but we don't have it in cache, it's stale
            if matches!(state, VmState::Ready | VmState::Starting | VmState::Busy { .. }) {
                if !self.vms.contains_key(&vm_id) {
                    // Mark as failed
                    let failed_state = VmState::Failed {
                        error: "VM not found in local cache during cleanup".to_string(),
                    };
                    let failed_state_json = Self::state_index_key(&failed_state);

                    self.hiqlite
                        .execute(
                            "UPDATE vms SET state = ?1, updated_at = ?2 WHERE id = ?3",
                            params![
                                failed_state_json,
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
        let state_json = Self::state_index_key(&vm.state); // JSON serialized
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
                    state_json,
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

// Parse VM state from legacy Debug string representation
// This is used for backward compatibility during recovery
fn parse_vm_state(state_str: &str) -> Result<VmState> {
    match state_str {
        "Starting" => Ok(VmState::Starting),
        "Ready" => Ok(VmState::Ready),
        "Draining" => Ok(VmState::Draining),
        s if s.starts_with("Idle") => Ok(VmState::Idle {
            jobs_completed: 0,
            last_job_at: 0,
        }),
        s if s.starts_with("Busy") => Ok(VmState::Busy {
            job_id: String::new(),
            started_at: 0,
        }),
        s if s.starts_with("Terminated") => Ok(VmState::Terminated {
            reason: String::from("Unknown"),
            exit_code: 0,
        }),
        s if s.starts_with("Failed") => Ok(VmState::Failed {
            error: String::new(),
        }),
        _ => Err(anyhow!("Unknown VM state: {}", state_str)),
    }
}

// Check if a process is alive
fn is_process_alive(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
}
