//! Replication worker for cross-cluster data synchronization.
//!
//! This worker handles data replication tasks including range synchronization,
//! consistency verification, snapshot export, incremental backup, and data restoration.
//!
//! Supported job types:
//! - `sync_range`: Synchronize a key range to another cluster
//! - `verify_consistency`: Verify consistency between clusters
//! - `snapshot_export`: Export cluster snapshot to blob store
//! - `incremental_backup`: Create incremental backup since last checkpoint
//! - `restore_data`: Restore data from a backup

use std::sync::Arc;
use std::time::Instant;

use aspen_blob::BlobStore;
use aspen_core::storage::KvEntry;
use aspen_core::storage::SM_KV_TABLE;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
use redb::Database;
use redb::ReadableTable;
use serde_json::json;
use tracing::info;
use tracing::warn;

use crate::Job;
use crate::JobResult;
use crate::Worker;

/// Worker for handling replication tasks.
///
/// This worker requires access to:
/// - A Redb database for reading data
/// - A KeyValueStore for writing data
/// - A BlobStore for snapshot storage
pub struct ReplicationWorker {
    node_id: u64,
    cluster_id: String,
    /// Database for reading data.
    db: Arc<Database>,
    /// KeyValueStore for writing restored data.
    kv_store: Option<Arc<dyn KeyValueStore>>,
    /// Blob store for snapshot storage.
    blob_store: Option<Arc<dyn BlobStore>>,
}

impl ReplicationWorker {
    /// Create a new replication worker.
    pub fn new(node_id: u64, cluster_id: String, db: Arc<Database>) -> Self {
        Self {
            node_id,
            cluster_id,
            db,
            kv_store: None,
            blob_store: None,
        }
    }

    /// Add KeyValueStore for write operations.
    pub fn with_kv_store(mut self, kv_store: Arc<dyn KeyValueStore>) -> Self {
        self.kv_store = Some(kv_store);
        self
    }

    /// Add BlobStore for snapshot operations.
    pub fn with_blob_store(mut self, blob_store: Arc<dyn BlobStore>) -> Self {
        self.blob_store = Some(blob_store);
        self
    }

    /// Synchronize a key range.
    ///
    /// Reads all keys in the specified range and returns them for sync.
    async fn sync_range(
        &self,
        start_key: &str,
        end_key: &str,
        target_cluster: &str,
    ) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let read_txn = self.db.begin_read().map_err(|e| format!("failed to begin transaction: {}", e))?;

        let table = read_txn.open_table(SM_KV_TABLE).map_err(|e| format!("failed to open table: {}", e))?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;

        let mut keys_synced: u64 = 0;
        let mut bytes_transferred: u64 = 0;
        let mut sync_data: Vec<serde_json::Value> = Vec::new();

        // Iterate over the range
        for item in table.iter().map_err(|e| format!("failed to iterate: {}", e))? {
            let (key_guard, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

            let key_bytes = key_guard.value();
            let key_str = match std::str::from_utf8(key_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Check if key is in range
            if !start_key.is_empty() && key_str < start_key {
                continue;
            }
            if !end_key.is_empty() && key_str >= end_key {
                break;
            }

            // Deserialize entry
            let kv: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Skip expired entries
            if let Some(expires_at) = kv.expires_at_ms {
                if now_ms > expires_at {
                    continue;
                }
            }

            bytes_transferred += key_bytes.len() as u64 + kv.value.len() as u64;
            keys_synced += 1;

            // Limit sync data to prevent memory issues
            if sync_data.len() < 1000 {
                sync_data.push(json!({
                    "key": key_str,
                    "value": kv.value,
                    "version": kv.version,
                    "mod_revision": kv.mod_revision,
                }));
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            node_id = self.node_id,
            source_cluster = %self.cluster_id,
            target_cluster = target_cluster,
            keys_synced = keys_synced,
            bytes = bytes_transferred,
            "range sync completed"
        );

        Ok(json!({
            "node_id": self.node_id,
            "source_cluster": self.cluster_id,
            "target_cluster": target_cluster,
            "start_key": start_key,
            "end_key": end_key,
            "keys_synced": keys_synced,
            "bytes_transferred": bytes_transferred,
            "duration_ms": duration_ms,
            "sync_data": sync_data,
            "data_truncated": keys_synced > 1000
        }))
    }

    /// Verify consistency between local data and a checksum.
    async fn verify_consistency(&self, target_cluster: &str, sample_rate: f64) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let read_txn = self.db.begin_read().map_err(|e| format!("failed to begin transaction: {}", e))?;

        let table = read_txn.open_table(SM_KV_TABLE).map_err(|e| format!("failed to open table: {}", e))?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        let effective_sample_rate = sample_rate.clamp(0.001, 1.0);

        let mut keys_checked: u64 = 0;
        let mut total_keys: u64 = 0;
        let mut checksum: u64 = 0;
        let mut sample_hashes: Vec<String> = Vec::new();

        // Use a simple deterministic sampling based on key hash
        for item in table.iter().map_err(|e| format!("failed to iterate: {}", e))? {
            let (key_guard, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

            let key_bytes = key_guard.value();

            // Deserialize entry
            let kv: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Skip expired entries
            if let Some(expires_at) = kv.expires_at_ms {
                if now_ms > expires_at {
                    continue;
                }
            }

            total_keys += 1;

            // Sample based on key hash
            let key_hash = hash_bytes(key_bytes);
            if (key_hash % 10000) as f64 / 10000.0 <= effective_sample_rate {
                keys_checked += 1;

                // Compute checksum contribution
                let value_hash = hash_bytes(kv.value.as_bytes());
                checksum = checksum.wrapping_add(key_hash).wrapping_add(value_hash);

                // Store sample for detailed verification
                if sample_hashes.len() < 100 {
                    sample_hashes.push(format!("{:016x}", key_hash.wrapping_add(value_hash)));
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            node_id = self.node_id,
            source_cluster = %self.cluster_id,
            target_cluster = target_cluster,
            keys_checked = keys_checked,
            total_keys = total_keys,
            "consistency verification completed"
        );

        Ok(json!({
            "node_id": self.node_id,
            "source_cluster": self.cluster_id,
            "target_cluster": target_cluster,
            "total_keys": total_keys,
            "keys_checked": keys_checked,
            "sample_rate": effective_sample_rate,
            "checksum": format!("{:016x}", checksum),
            "sample_hashes": sample_hashes,
            "duration_ms": duration_ms,
            "verified": true // True means verification completed, not that data matches
        }))
    }

    /// Export a snapshot to blob store.
    async fn snapshot_export(&self, snapshot_id: &str) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let blob_store = self.blob_store.as_ref().ok_or("blob store not configured for snapshot export")?;

        let read_txn = self.db.begin_read().map_err(|e| format!("failed to begin transaction: {}", e))?;

        let table = read_txn.open_table(SM_KV_TABLE).map_err(|e| format!("failed to open table: {}", e))?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;

        // Build snapshot data
        let mut snapshot_entries: Vec<serde_json::Value> = Vec::new();
        let mut key_count: u64 = 0;

        for item in table.iter().map_err(|e| format!("failed to iterate: {}", e))? {
            let (key_guard, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

            let key_bytes = key_guard.value();
            let key_str = match std::str::from_utf8(key_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Deserialize entry
            let kv: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Skip expired entries
            if let Some(expires_at) = kv.expires_at_ms {
                if now_ms > expires_at {
                    continue;
                }
            }

            key_count += 1;

            snapshot_entries.push(json!({
                "k": key_str,
                "v": kv.value,
                "ver": kv.version,
                "cr": kv.create_revision,
                "mr": kv.mod_revision,
                "exp": kv.expires_at_ms,
                "lid": kv.lease_id,
            }));
        }

        // Serialize snapshot as JSON Lines for streaming
        let mut snapshot_data = Vec::new();
        for entry in &snapshot_entries {
            let line = serde_json::to_string(entry).map_err(|e| format!("serialization error: {}", e))?;
            snapshot_data.extend_from_slice(line.as_bytes());
            snapshot_data.push(b'\n');
        }

        let size_bytes = snapshot_data.len() as u64;

        // Store in blob store
        let result =
            blob_store.add_bytes(&snapshot_data).await.map_err(|e| format!("failed to store snapshot: {}", e))?;

        let blob_hash = result.blob_ref.hash.to_hex().to_string();

        // Protect snapshot from GC
        let tag_name = format!("snapshot:{}", snapshot_id);
        blob_store
            .protect(&result.blob_ref.hash, &tag_name)
            .await
            .map_err(|e| format!("failed to protect snapshot: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            node_id = self.node_id,
            cluster_id = %self.cluster_id,
            snapshot_id = snapshot_id,
            key_count = key_count,
            size_bytes = size_bytes,
            blob_hash = %blob_hash,
            "snapshot exported"
        );

        Ok(json!({
            "node_id": self.node_id,
            "cluster_id": self.cluster_id,
            "snapshot_id": snapshot_id,
            "blob_hash": blob_hash,
            "size_bytes": size_bytes,
            "key_count": key_count,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "duration_ms": duration_ms
        }))
    }

    /// Create an incremental backup since the last backup.
    async fn incremental_backup(
        &self,
        backup_id: &str,
        last_mod_revision: Option<i64>,
    ) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let blob_store = self.blob_store.as_ref().ok_or("blob store not configured for backup")?;

        let read_txn = self.db.begin_read().map_err(|e| format!("failed to begin transaction: {}", e))?;

        let table = read_txn.open_table(SM_KV_TABLE).map_err(|e| format!("failed to open table: {}", e))?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        let min_revision = last_mod_revision.unwrap_or(0);

        // Build incremental backup data
        let mut backup_entries: Vec<serde_json::Value> = Vec::new();
        let mut changes_backed_up: u64 = 0;
        let mut max_mod_revision: i64 = min_revision;

        for item in table.iter().map_err(|e| format!("failed to iterate: {}", e))? {
            let (key_guard, value_guard) = item.map_err(|e| format!("read error: {}", e))?;

            let key_bytes = key_guard.value();
            let key_str = match std::str::from_utf8(key_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Deserialize entry
            let kv: KvEntry = match bincode::deserialize(value_guard.value()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Skip expired entries
            if let Some(expires_at) = kv.expires_at_ms {
                if now_ms > expires_at {
                    continue;
                }
            }

            // Only include entries modified since last backup
            if kv.mod_revision <= min_revision {
                continue;
            }

            changes_backed_up += 1;
            max_mod_revision = max_mod_revision.max(kv.mod_revision);

            backup_entries.push(json!({
                "k": key_str,
                "v": kv.value,
                "ver": kv.version,
                "cr": kv.create_revision,
                "mr": kv.mod_revision,
                "exp": kv.expires_at_ms,
                "lid": kv.lease_id,
            }));
        }

        // Serialize backup as JSON Lines
        let mut backup_data = Vec::new();
        for entry in &backup_entries {
            let line = serde_json::to_string(entry).map_err(|e| format!("serialization error: {}", e))?;
            backup_data.extend_from_slice(line.as_bytes());
            backup_data.push(b'\n');
        }

        let size_bytes = backup_data.len() as u64;

        // Store in blob store
        let result = blob_store.add_bytes(&backup_data).await.map_err(|e| format!("failed to store backup: {}", e))?;

        let blob_hash = result.blob_ref.hash.to_hex().to_string();

        // Protect backup from GC
        let tag_name = format!("backup:{}", backup_id);
        blob_store
            .protect(&result.blob_ref.hash, &tag_name)
            .await
            .map_err(|e| format!("failed to protect backup: {}", e))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            node_id = self.node_id,
            cluster_id = %self.cluster_id,
            backup_id = backup_id,
            changes = changes_backed_up,
            size_bytes = size_bytes,
            "incremental backup completed"
        );

        Ok(json!({
            "node_id": self.node_id,
            "cluster_id": self.cluster_id,
            "backup_id": backup_id,
            "blob_hash": blob_hash,
            "changes_backed_up": changes_backed_up,
            "size_bytes": size_bytes,
            "from_revision": min_revision,
            "to_revision": max_mod_revision,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "duration_ms": duration_ms
        }))
    }

    /// Restore data from a backup.
    async fn restore_data(&self, source: &str, restore_point: Option<&str>) -> Result<serde_json::Value, String> {
        let start = Instant::now();

        let blob_store = self.blob_store.as_ref().ok_or("blob store not configured for restore")?;

        let kv_store = self.kv_store.as_ref().ok_or("KV store not configured for restore")?;

        // Parse the source hash
        let hash = if source.len() == 64 {
            let mut bytes = [0u8; 32];
            hex::decode_to_slice(source, &mut bytes).map_err(|e| format!("invalid hex hash: {}", e))?;
            iroh_blobs::Hash::from(bytes)
        } else {
            source.parse::<iroh_blobs::Hash>().map_err(|e| format!("invalid hash format: {}", e))?
        };

        // Get backup data from blob store
        let backup_data = blob_store
            .get_bytes(&hash)
            .await
            .map_err(|e| format!("failed to get backup: {}", e))?
            .ok_or("backup blob not found")?;

        let backup_str = std::str::from_utf8(&backup_data).map_err(|e| format!("invalid backup format: {}", e))?;

        let mut keys_restored: u64 = 0;
        let mut bytes_restored: u64 = 0;
        let mut errors: Vec<String> = Vec::new();

        // Parse and restore each line
        for line in backup_str.lines() {
            if line.is_empty() {
                continue;
            }

            let entry: serde_json::Value =
                serde_json::from_str(line).map_err(|e| format!("failed to parse backup line: {}", e))?;

            let key = entry["k"].as_str().unwrap_or("");
            let value = entry["v"].as_str().unwrap_or("");

            if key.is_empty() {
                continue;
            }

            // Skip entries after restore point if specified
            if let Some(rp) = restore_point {
                if let Some(mr) = entry["mr"].as_i64() {
                    if let Ok(rp_rev) = rp.parse::<i64>() {
                        if mr > rp_rev {
                            continue;
                        }
                    }
                }
            }

            // Write to KV store
            let request = WriteRequest {
                command: WriteCommand::Set {
                    key: key.to_string(),
                    value: value.to_string(),
                },
            };

            match kv_store.write(request).await {
                Ok(_) => {
                    keys_restored += 1;
                    bytes_restored += key.len() as u64 + value.len() as u64;
                }
                Err(e) => {
                    if errors.len() < 10 {
                        errors.push(format!("key {}: {}", key, e));
                    }
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        info!(
            node_id = self.node_id,
            cluster_id = %self.cluster_id,
            source = source,
            keys_restored = keys_restored,
            bytes_restored = bytes_restored,
            "data restoration completed"
        );

        Ok(json!({
            "node_id": self.node_id,
            "cluster_id": self.cluster_id,
            "source": source,
            "restore_point": restore_point,
            "keys_restored": keys_restored,
            "bytes_restored": bytes_restored,
            "errors": errors,
            "success": errors.is_empty(),
            "duration_ms": duration_ms
        }))
    }
}

/// Simple hash function for consistency checking.
fn hash_bytes(bytes: &[u8]) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

#[async_trait]
impl Worker for ReplicationWorker {
    async fn execute(&self, job: Job) -> JobResult {
        match job.spec.job_type.as_str() {
            "sync_range" => {
                let start_key = job.spec.payload["start_key"].as_str().unwrap_or("");
                let end_key = job.spec.payload["end_key"].as_str().unwrap_or("");
                let target_cluster = job.spec.payload["target_cluster"].as_str().unwrap_or("unknown");

                info!(
                    node_id = self.node_id,
                    source_cluster = %self.cluster_id,
                    target_cluster = target_cluster,
                    "syncing key range [{}, {})",
                    start_key,
                    end_key
                );

                match self.sync_range(start_key, end_key, target_cluster).await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "range sync failed");
                        JobResult::failure(format!("range sync failed: {}", e))
                    }
                }
            }

            "verify_consistency" => {
                let target_cluster = job.spec.payload["target_cluster"].as_str().unwrap_or("unknown");
                let sample_rate = job.spec.payload["sample_rate"].as_f64().unwrap_or(0.01);

                info!(
                    node_id = self.node_id,
                    source_cluster = %self.cluster_id,
                    target_cluster = target_cluster,
                    sample_rate = sample_rate,
                    "verifying cross-cluster consistency"
                );

                match self.verify_consistency(target_cluster, sample_rate).await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "consistency verification failed");
                        JobResult::failure(format!("consistency verification failed: {}", e))
                    }
                }
            }

            "snapshot_export" => {
                let snapshot_id = job.spec.payload["snapshot_id"].as_str().unwrap_or("snapshot-default");

                info!(
                    node_id = self.node_id,
                    cluster_id = %self.cluster_id,
                    snapshot_id = snapshot_id,
                    "exporting cluster snapshot"
                );

                match self.snapshot_export(snapshot_id).await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "snapshot export failed");
                        JobResult::failure(format!("snapshot export failed: {}", e))
                    }
                }
            }

            "incremental_backup" => {
                let backup_id = job.spec.payload["backup_id"].as_str().unwrap_or("backup-default");
                let last_revision = job.spec.payload["last_backup_revision"].as_i64();

                info!(
                    node_id = self.node_id,
                    cluster_id = %self.cluster_id,
                    backup_id = backup_id,
                    last_revision = ?last_revision,
                    "performing incremental backup"
                );

                match self.incremental_backup(backup_id, last_revision).await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "incremental backup failed");
                        JobResult::failure(format!("incremental backup failed: {}", e))
                    }
                }
            }

            "restore_data" => {
                let source = job.spec.payload["source"].as_str().unwrap_or("unknown");
                let restore_point = job.spec.payload["restore_point"].as_str();

                info!(
                    node_id = self.node_id,
                    cluster_id = %self.cluster_id,
                    source = source,
                    restore_point = ?restore_point,
                    "restoring data from backup"
                );

                match self.restore_data(source, restore_point).await {
                    Ok(result) => JobResult::success(result),
                    Err(e) => {
                        warn!(node_id = self.node_id, error = %e, "data restoration failed");
                        JobResult::failure(format!("data restoration failed: {}", e))
                    }
                }
            }

            _ => JobResult::failure(format!("unknown replication task: {}", job.spec.job_type)),
        }
    }

    fn job_types(&self) -> Vec<String> {
        vec![
            "sync_range".to_string(),
            "verify_consistency".to_string(),
            "snapshot_export".to_string(),
            "incremental_backup".to_string(),
            "restore_data".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_bytes() {
        let a = hash_bytes(b"hello");
        let b = hash_bytes(b"world");
        let a2 = hash_bytes(b"hello");

        // Same input should give same output
        assert_eq!(a, a2);
        // Different input should give different output (with high probability)
        assert_ne!(a, b);
    }
}
