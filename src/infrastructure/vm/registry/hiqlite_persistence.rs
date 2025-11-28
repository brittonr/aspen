// Hiqlite-backed VM Persistence
//
// Implements VmPersistence trait using Hiqlite distributed database.
// Handles JSON serialization, SQL operations, and event logging.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::hiqlite::HiqliteService;
use crate::infrastructure::vm::vm_types::{VmConfig, VmInstance, VmMetrics, VmState};
use crate::params;

use super::persistence::VmPersistence;

/// Row representation for database queries
#[derive(Debug, serde::Deserialize)]
struct VmRow {
    id: String,
    config: String,
    state: String,
    created_at: i64,
    pid: Option<i64>,
    control_socket: Option<String>,
    job_dir: Option<String>,
    ip_address: Option<String>,
    metrics: String,
}

impl From<hiqlite::Row<'static>> for VmRow {
    fn from(mut row: hiqlite::Row<'static>) -> Self {
        Self {
            id: row.get("id"),
            config: row.get("config"),
            state: row.get("state"),
            created_at: row.get("created_at"),
            pid: row.get("pid"),
            control_socket: row.get("control_socket"),
            job_dir: row.get("job_dir"),
            ip_address: row.get("ip_address"),
            metrics: row.get("metrics"),
        }
    }
}

pub struct HiqliteVmPersistence {
    hiqlite: Arc<HiqliteService>,
    node_id: String,
}

impl HiqliteVmPersistence {
    pub fn new(hiqlite: Arc<HiqliteService>, node_id: String) -> Self {
        Self { hiqlite, node_id }
    }

    /// Serialize state to JSON for storage
    fn serialize_state(state: &VmState) -> String {
        serde_json::to_string(state).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize VM state: {}", e);
            String::from("{\"error\":\"serialization_failed\"}")
        })
    }

    /// Deserialize state from JSON or legacy Debug format
    fn deserialize_state(state_str: &str) -> Result<VmState> {
        // Try JSON first (preferred format)
        if let Ok(state) = serde_json::from_str::<VmState>(state_str) {
            return Ok(state);
        }

        // Legacy Debug format fallback (with data loss warning)
        tracing::warn!(
            "Using legacy Debug format parser for state: '{}' - data loss may occur",
            state_str
        );
        parse_vm_state_legacy(state_str)
    }
}

#[async_trait]
impl VmPersistence for HiqliteVmPersistence {
    async fn save(&self, vm: &VmInstance) -> Result<()> {
        let config_json = serde_json::to_string(&vm.config)?;
        let state_json = Self::serialize_state(&vm.state);
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

    async fn load(&self, vm_id: &Uuid) -> Result<Option<VmInstance>> {
        let rows: Vec<VmRow> = self
            .hiqlite
            .query_as(
                "SELECT id, config, state, created_at, pid, control_socket, job_dir, ip_address, metrics
                 FROM vms WHERE id = ?1 AND node_id = ?2",
                params![vm_id.to_string(), self.node_id.clone()],
            )
            .await?;

        if let Some(row) = rows.into_iter().next() {
            let config: VmConfig = serde_json::from_str(&row.config)?;
            let state = Self::deserialize_state(&row.state)?;
            let metrics: VmMetrics = serde_json::from_str(&row.metrics)?;

            Ok(Some(VmInstance {
                config: Arc::new(config),
                state,
                created_at: row.created_at,
                pid: row.pid.map(|p| p as u32),
                control_socket: row.control_socket.map(|s| s.into()),
                job_dir: row.job_dir.map(|s| s.into()),
                ip_address: row.ip_address,
                metrics,
            }))
        } else {
            Ok(None)
        }
    }

    async fn load_all(&self) -> Result<Vec<VmInstance>> {
        let rows: Vec<VmRow> = self
            .hiqlite
            .query_as(
                "SELECT id, config, state, created_at, pid, control_socket, job_dir, ip_address, metrics
                 FROM vms WHERE node_id = ?1",
                params![self.node_id.clone()],
            )
            .await?;

        let mut vms = Vec::new();
        for row in rows {
            let config: VmConfig = serde_json::from_str(&row.config)?;
            let state = Self::deserialize_state(&row.state)?;
            let metrics: VmMetrics = serde_json::from_str(&row.metrics)?;

            vms.push(VmInstance {
                config: Arc::new(config),
                state,
                created_at: row.created_at,
                pid: row.pid.map(|p| p as u32),
                control_socket: row.control_socket.map(|s| s.into()),
                job_dir: row.job_dir.map(|s| s.into()),
                ip_address: row.ip_address,
                metrics,
            });
        }

        Ok(vms)
    }

    async fn delete(&self, vm_id: &Uuid) -> Result<()> {
        self.hiqlite
            .execute(
                "DELETE FROM vms WHERE id = ?1",
                params![vm_id.to_string()],
            )
            .await?;
        Ok(())
    }

    async fn update_state(&self, vm_id: &Uuid, state: &VmState) -> Result<()> {
        let state_json = Self::serialize_state(state);
        let now = chrono::Utc::now().timestamp();

        self.hiqlite
            .execute(
                "UPDATE vms SET state = ?1, updated_at = ?2 WHERE id = ?3",
                params![state_json, now, vm_id.to_string()],
            )
            .await?;
        Ok(())
    }

    async fn update_metrics(&self, vm_id: &Uuid, metrics: &VmMetrics) -> Result<()> {
        let metrics_json = serde_json::to_string(metrics)?;
        let now = chrono::Utc::now().timestamp();

        self.hiqlite
            .execute(
                "UPDATE vms SET metrics = ?1, updated_at = ?2 WHERE id = ?3",
                params![metrics_json, now, vm_id.to_string()],
            )
            .await?;
        Ok(())
    }

    async fn query_by_state(&self, state: &VmState) -> Result<Vec<Uuid>> {
        let state_json = Self::serialize_state(state);

        #[derive(Debug, serde::Deserialize)]
        struct Row {
            id: String,
        }

        impl From<hiqlite::Row<'static>> for Row {
            fn from(mut row: hiqlite::Row<'static>) -> Self {
                Self {
                    id: row.get("id"),
                }
            }
        }

        let rows: Vec<Row> = self
            .hiqlite
            .query_as(
                "SELECT id FROM vms WHERE state = ?1 AND node_id = ?2",
                params![state_json, self.node_id.clone()],
            )
            .await?;

        rows.into_iter()
            .map(|row| Uuid::parse_str(&row.id).map_err(|e| anyhow!("Invalid UUID: {}", e)))
            .collect()
    }

    async fn log_event(&self, vm_id: &Uuid, event: &str, data: Option<String>) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        self.hiqlite
            .execute(
                "INSERT INTO vm_events (vm_id, event_type, event_data, timestamp, node_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    vm_id.to_string(),
                    event,
                    data,
                    now,
                    self.node_id.clone()
                ],
            )
            .await?;

        Ok(())
    }
}

/// Legacy Debug format parser (fallback for old data)
fn parse_vm_state_legacy(s: &str) -> Result<VmState> {
    let s = s.trim();

    // Simple enum variants
    match s {
        "Starting" => return Ok(VmState::Starting),
        "Ready" => return Ok(VmState::Ready),
        "Draining" => return Ok(VmState::Draining),
        _ => {}
    }

    // Complex variants (data loss!)
    if s.starts_with("Busy") {
        // Lost job_id and started_at
        return Ok(VmState::Busy {
            job_id: String::from("unknown"),
            started_at: 0,
        });
    }

    if s.starts_with("Idle") {
        // Lost jobs_completed and last_job_at
        return Ok(VmState::Idle {
            jobs_completed: 0,
            last_job_at: 0,
        });
    }

    if s.starts_with("Terminated") {
        // Lost reason and exit_code
        return Ok(VmState::Terminated {
            reason: String::from("unknown"),
            exit_code: 0,
        });
    }

    if s.starts_with("Failed") {
        // Try to extract error message
        let error = s
            .strip_prefix("Failed { error: ")
            .and_then(|s| s.strip_suffix(" }"))
            .unwrap_or("unknown error")
            .trim_matches('"')
            .to_string();

        return Ok(VmState::Failed { error });
    }

    Err(anyhow!("Unknown VM state format: {}", s))
}
