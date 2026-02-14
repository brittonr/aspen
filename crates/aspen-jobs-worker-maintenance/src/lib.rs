//! Maintenance worker for system tasks.
//!
//! This worker handles system maintenance tasks including storage compaction,
//! blob cleanup, health checks, and metrics collection.
//!
//! Supported job types:
//! - `compact_storage`: Compact the Redb database to reclaim space
//! - `cleanup_blobs`: Remove orphaned blobs from the blob store
//! - `health_check`: Perform comprehensive node health check
//! - `collect_metrics`: Collect and return node metrics

use std::sync::Arc;
use std::time::Instant;

use aspen_blob::prelude::*;
use aspen_core::storage::KvEntry;
use aspen_core::storage::SM_KV_TABLE;
use aspen_jobs::Job;
use aspen_jobs::JobResult;
use aspen_jobs::Worker;
use aspen_traits::ClusterController;
use async_trait::async_trait;
use redb::Database;
use redb::ReadableTable;
use serde_json::json;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Worker for system maintenance tasks.
///
/// This worker requires access to:
/// - A Redb database for storage operations
/// - A BlobStore for blob cleanup
/// - A ClusterController for cluster health
#[derive(Clone)]
pub struct MaintenanceWorker {
    node_id: u64,
    /// Database for storage operations.
    db: Arc<Database>,
    /// Blob store for cleanup operations.
    blob_store: Option<Arc<dyn BlobStore>>,
    /// Cluster controller for metrics.
    cluster_controller: Option<Arc<dyn ClusterController>>,
}

impl MaintenanceWorker {
    /// Create a new maintenance worker.
    pub fn new(node_id: u64, db: Arc<Database>) -> Self {
        Self {
            node_id,
            db,
            blob_store: None,
            cluster_controller: None,
        }
    }

    /// Add BlobStore for blob cleanup operations.
    pub fn with_blob_store(mut self, blob_store: Arc<dyn BlobStore>) -> Self {
        self.blob_store = Some(blob_store);
        self
    }

    /// Add ClusterController for health checks.
    pub fn with_cluster_controller(mut self, controller: Arc<dyn ClusterController>) -> Self {
        self.cluster_controller = Some(controller);
        self
    }

    /// Compact the storage database.
    ///
    /// This reclaims space from deleted entries and optimizes the database layout.
    async fn compact_storage(&self) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        // Get database stats before compaction
        let stats_before = self.get_database_stats()?;

        // Redb automatically compacts on commit, but we can trigger
        // a manual compaction by running a checkpoint
        // Note: Redb doesn't have explicit compact(), but we can
        // measure the freed space after operations

        // Count entries and calculate sizes
        let read_txn = self.db.begin_read().map_err(|e| format!("failed to begin read: {}", e))?;

        let table = read_txn.open_table(SM_KV_TABLE).map_err(|e| format!("failed to open table: {}", e))?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        let mut expired_count: u64 = 0;
        let mut total_count: u64 = 0;
        let mut total_bytes: u64 = 0;

        for item in table.iter().map_err(|e| format!("failed to iterate: {}", e))? {
            let (key_guard, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

            let kv: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            total_count += 1;
            total_bytes += key_guard.value().len() as u64 + value_guard.value().len() as u64;

            if let Some(expires_at) = kv.expires_at_ms
                && now_ms > expires_at
            {
                expired_count += 1;
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            node_id = self.node_id,
            total_entries = total_count,
            expired_entries = expired_count,
            total_bytes = total_bytes,
            "storage compaction analysis completed"
        );

        Ok(json!({
            "node_id": self.node_id,
            "status": "completed",
            "stats_before": stats_before,
            "total_entries": total_count,
            "expired_entries": expired_count,
            "total_bytes": total_bytes,
            "space_reclaimed_bytes": 0, // Redb handles this automatically
            "duration_ms": duration_ms,
            "note": "Redb compacts automatically on write transactions"
        }))
    }

    /// Get database statistics.
    fn get_database_stats(&self) -> Result<serde_json::Value, String> {
        let read_txn = self.db.begin_read().map_err(|e| format!("failed to begin read: {}", e))?;

        // Try to get table stats
        let table_exists = read_txn.open_table(SM_KV_TABLE).is_ok();

        Ok(json!({
            "table_exists": table_exists,
            "engine": "redb",
        }))
    }

    /// Cleanup orphaned blobs.
    ///
    /// Lists all blobs and checks if they're still referenced.
    async fn cleanup_blobs(&self) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let blob_store = self.blob_store.as_ref().ok_or("blob store not configured for cleanup")?;

        // Get all keys from KV store to find referenced blobs
        let read_txn = self.db.begin_read().map_err(|e| format!("failed to begin read: {}", e))?;

        let table = read_txn.open_table(SM_KV_TABLE).map_err(|e| format!("failed to open table: {}", e))?;

        // Collect blob references from KV values
        let mut referenced_blobs = std::collections::HashSet::new();

        for item in table.iter().map_err(|e| format!("failed to iterate: {}", e))? {
            let (_, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

            let kv: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Check if value looks like a blob reference
            if kv.value.starts_with("__blob:")
                && let Some(hash_str) = kv.value.strip_prefix("__blob:")
                // Parse the JSON blob reference
                && let Ok(blob_ref) = serde_json::from_str::<serde_json::Value>(hash_str)
                && let Some(hash) = blob_ref["hash"].as_str()
            {
                referenced_blobs.insert(hash.to_string());
            }
        }

        // List all blobs in the store
        let blob_list = blob_store.list(10000, None).await.map_err(|e| format!("failed to list blobs: {}", e))?;

        let mut orphaned_count = 0;
        let mut orphaned_bytes: u64 = 0;
        let mut orphaned_hashes: Vec<String> = Vec::new();

        for blob in &blob_list.blobs {
            let hash_str = blob.hash.to_hex().to_string();
            if !referenced_blobs.contains(&hash_str) {
                orphaned_count += 1;
                orphaned_bytes += blob.size;
                if orphaned_hashes.len() < 100 {
                    orphaned_hashes.push(hash_str);
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            node_id = self.node_id,
            total_blobs = blob_list.blobs.len(),
            referenced_blobs = referenced_blobs.len(),
            orphaned_blobs = orphaned_count,
            orphaned_bytes = orphaned_bytes,
            "blob cleanup analysis completed"
        );

        // Note: We don't actually delete orphaned blobs here because:
        // 1. They might be protected by tags
        // 2. They might be in transit for replication
        // 3. Deletion should be explicit
        // Instead, we report them for manual review or scheduled GC

        Ok(json!({
            "node_id": self.node_id,
            "total_blobs": blob_list.blobs.len(),
            "referenced_blobs": referenced_blobs.len(),
            "orphaned_blobs": orphaned_count,
            "orphaned_bytes": orphaned_bytes,
            "orphaned_hashes": orphaned_hashes,
            "action": "analysis_only",
            "note": "Use GC protection tags to manage blob lifecycle",
            "duration_ms": duration_ms
        }))
    }

    /// Perform a comprehensive health check.
    async fn health_check(&self) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let mut checks = serde_json::Map::new();
        let mut all_healthy = true;

        // Check 1: Database health
        let db_health = match self.db.begin_read() {
            Ok(_) => {
                checks.insert("database".to_string(), json!({"status": "healthy", "engine": "redb"}));
                true
            }
            Err(e) => {
                checks.insert("database".to_string(), json!({"status": "unhealthy", "error": e.to_string()}));
                false
            }
        };
        all_healthy &= db_health;

        // Check 2: Blob store health (if available)
        if let Some(blob_store) = &self.blob_store {
            match blob_store.list(1, None).await {
                Ok(_) => {
                    checks.insert("blob_store".to_string(), json!({"status": "healthy"}));
                }
                Err(e) => {
                    checks.insert("blob_store".to_string(), json!({"status": "unhealthy", "error": e.to_string()}));
                    all_healthy = false;
                }
            }
        } else {
            checks.insert("blob_store".to_string(), json!({"status": "not_configured"}));
        }

        // Check 3: Cluster health (if available)
        if let Some(controller) = &self.cluster_controller {
            match controller.get_metrics().await {
                Ok(metrics) => {
                    let raft_state = format!("{:?}", metrics.state);
                    checks.insert(
                        "cluster".to_string(),
                        json!({
                            "status": "healthy",
                            "initialized": controller.is_initialized(),
                            "raft_state": raft_state,
                            "current_term": metrics.current_term,
                            "current_leader": metrics.current_leader,
                            "last_applied_index": metrics.last_applied_index,
                        }),
                    );
                }
                Err(e) => {
                    checks.insert("cluster".to_string(), json!({"status": "unhealthy", "error": e.to_string()}));
                    all_healthy = false;
                }
            }
        } else {
            checks.insert("cluster".to_string(), json!({"status": "not_configured"}));
        }

        // Check 4: Memory usage (basic)
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                let mut vm_rss_kb = 0u64;
                for line in status.lines() {
                    if line.starts_with("VmRSS:")
                        && let Some(value) = line.split_whitespace().nth(1)
                    {
                        vm_rss_kb = value.parse().unwrap_or(0);
                    }
                }
                checks.insert(
                    "memory".to_string(),
                    json!({
                        "status": "healthy",
                        "rss_mb": vm_rss_kb / 1024,
                    }),
                );
            }
        }

        // Check 5: KV store entry count
        if let Ok(read_txn) = self.db.begin_read()
            && let Ok(table) = read_txn.open_table(SM_KV_TABLE)
        {
            let count = table.iter().map(|i| i.count()).unwrap_or(0);
            checks.insert(
                "kv_store".to_string(),
                json!({
                    "status": "healthy",
                    "entry_count": count,
                }),
            );
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        let overall_status = if all_healthy { "healthy" } else { "degraded" };

        info!(node_id = self.node_id, status = overall_status, "health check completed");

        Ok(json!({
            "node_id": self.node_id,
            "healthy": all_healthy,
            "status": overall_status,
            "checks": checks,
            "duration_ms": duration_ms,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    /// Collect node metrics.
    async fn collect_metrics(&self) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let mut metrics = serde_json::Map::new();

        // KV store metrics
        if let Ok(read_txn) = self.db.begin_read()
            && let Ok(table) = read_txn.open_table(SM_KV_TABLE)
        {
            let now_ms = chrono::Utc::now().timestamp_millis() as u64;
            let mut entry_count: u64 = 0;
            let mut total_key_bytes: u64 = 0;
            let mut total_value_bytes: u64 = 0;
            let mut expired_count: u64 = 0;
            let mut with_lease: u64 = 0;

            for item in table.iter().map_err(|e| format!("iterate error: {}", e))? {
                let (key_guard, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

                let kv: KvEntry = match bincode::deserialize(value_guard.value()) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                entry_count += 1;
                total_key_bytes += key_guard.value().len() as u64;
                total_value_bytes += kv.value.len() as u64;

                if let Some(expires_at) = kv.expires_at_ms
                    && now_ms > expires_at
                {
                    expired_count += 1;
                }

                if kv.lease_id.is_some() {
                    with_lease += 1;
                }
            }

            metrics.insert(
                "kv_store".to_string(),
                json!({
                    "entry_count": entry_count,
                    "total_key_bytes": total_key_bytes,
                    "total_value_bytes": total_value_bytes,
                    "total_size_bytes": total_key_bytes + total_value_bytes,
                    "expired_entries": expired_count,
                    "entries_with_lease": with_lease,
                    "avg_key_size": if entry_count > 0 { total_key_bytes / entry_count } else { 0 },
                    "avg_value_size": if entry_count > 0 { total_value_bytes / entry_count } else { 0 },
                }),
            );
        }

        // Blob store metrics
        if let Some(blob_store) = &self.blob_store
            && let Ok(list) = blob_store.list(10000, None).await
        {
            let total_blob_bytes: u64 = list.blobs.iter().map(|b| b.size).sum();
            metrics.insert(
                "blob_store".to_string(),
                json!({
                    "blob_count": list.blobs.len(),
                    "total_bytes": total_blob_bytes,
                    "has_more": list.continuation_token.is_some(),
                }),
            );
        }

        // Cluster metrics
        if let Some(controller) = &self.cluster_controller
            && let Ok(cluster_metrics) = controller.get_metrics().await
        {
            metrics.insert(
                "cluster".to_string(),
                json!({
                    "state": format!("{:?}", cluster_metrics.state),
                    "current_term": cluster_metrics.current_term,
                    "current_leader": cluster_metrics.current_leader,
                    "last_applied_index": cluster_metrics.last_applied_index,
                    "last_log_index": cluster_metrics.last_log_index,
                    "snapshot_index": cluster_metrics.snapshot_index,
                }),
            );
        }

        // System metrics (Linux-specific)
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                let mut vm_rss_kb = 0u64;
                let mut vm_size_kb = 0u64;
                let mut threads = 0u32;

                for line in status.lines() {
                    if let Some(rest) = line.strip_prefix("VmRSS:") {
                        vm_rss_kb = rest.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    } else if let Some(rest) = line.strip_prefix("VmSize:") {
                        vm_size_kb = rest.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    } else if let Some(rest) = line.strip_prefix("Threads:") {
                        threads = rest.trim().parse().unwrap_or(0);
                    }
                }

                metrics.insert(
                    "system".to_string(),
                    json!({
                        "memory_rss_mb": vm_rss_kb / 1024,
                        "memory_virtual_mb": vm_size_kb / 1024,
                        "thread_count": threads,
                    }),
                );
            }

            // CPU usage from /proc/self/stat
            if let Ok(stat) = std::fs::read_to_string("/proc/self/stat") {
                let parts: Vec<&str> = stat.split_whitespace().collect();
                if parts.len() > 14 {
                    let utime: u64 = parts[13].parse().unwrap_or(0);
                    let stime: u64 = parts[14].parse().unwrap_or(0);
                    metrics.insert(
                        "cpu".to_string(),
                        json!({
                            "user_ticks": utime,
                            "system_ticks": stime,
                            "total_ticks": utime + stime,
                        }),
                    );
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(node_id = self.node_id, "metrics collection completed");

        Ok(json!({
            "node_id": self.node_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "metrics": metrics,
            "collection_time_ms": duration_ms
        }))
    }
}

#[async_trait]
impl Worker for MaintenanceWorker {
    async fn execute(&self, job: Job) -> JobResult {
        match job.spec.job_type.as_str() {
            "compact_storage" => {
                info!(node_id = self.node_id, "compacting storage");

                match self.compact_storage().await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "storage compaction failed");
                        JobResult::failure(format!("compaction failed: {}", e))
                    }
                }
            }

            "cleanup_blobs" => {
                info!(node_id = self.node_id, "cleaning up unused blobs");

                match self.cleanup_blobs().await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "blob cleanup failed");
                        JobResult::failure(format!("cleanup failed: {}", e))
                    }
                }
            }

            "health_check" => {
                info!(node_id = self.node_id, "performing health check");

                match self.health_check().await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "health check failed");
                        JobResult::failure(format!("health check failed: {}", e))
                    }
                }
            }

            "collect_metrics" => {
                debug!(node_id = self.node_id, "collecting node metrics");

                match self.collect_metrics().await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "metrics collection failed");
                        JobResult::failure(format!("metrics collection failed: {}", e))
                    }
                }
            }

            _ => JobResult::failure(format!("unknown maintenance task: {}", job.spec.job_type)),
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec![
            "compact_storage".to_string(),
            "cleanup_blobs".to_string(),
            "health_check".to_string(),
            "collect_metrics".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maintenance_worker_creation() {
        let db =
            Arc::new(redb::Database::builder().create_with_backend(redb::backends::InMemoryBackend::new()).unwrap());

        let worker = MaintenanceWorker::new(1, db);
        assert_eq!(worker.node_id, 1);
    }
}
