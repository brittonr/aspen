//! Work stealing coordination between workers.

use anyhow::Result;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::DistributedWorkerCoordinator;
use super::constants::MAX_HINT_CLEANUP_BATCH;
use super::constants::MAX_STEAL_BATCH;
use super::constants::MAX_STEAL_HINTS_PER_WORKER;
use super::types::StealHint;
use super::types::WorkerInfo;
use crate::registry::HealthStatus;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> DistributedWorkerCoordinator<S> {
    /// Find workers that can steal work (low load).
    pub async fn find_steal_targets(&self) -> Result<Vec<WorkerInfo>> {
        // Tiger Style: config thresholds must be valid
        debug_assert!(self.config.steal_load_threshold > 0.0, "WORK_STEAL: steal_load_threshold must be positive");
        debug_assert!(self.config.heartbeat_timeout_ms > 0, "WORK_STEAL: heartbeat_timeout_ms must be positive");

        let workers = self.workers.read().await;

        let targets: Vec<_> = workers
            .values()
            .filter(|w| {
                w.health == HealthStatus::Healthy
                    && w.is_alive(self.config.heartbeat_timeout_ms)
                    && w.load < self.config.steal_load_threshold
                    && w.active_jobs < w.max_concurrent
            })
            .cloned()
            .collect();

        Ok(targets)
    }

    /// Find workers that are good sources for work stealing (high load).
    pub async fn find_steal_sources(&self) -> Result<Vec<WorkerInfo>> {
        let workers = self.workers.read().await;

        let sources: Vec<_> = workers
            .values()
            .filter(|w| {
                w.health == HealthStatus::Healthy
                    && w.is_alive(self.config.heartbeat_timeout_ms)
                    && w.queue_depth > self.config.steal_queue_threshold
            })
            .cloned()
            .collect();

        Ok(sources)
    }

    /// Coordinate work stealing between workers.
    ///
    /// Identifies target workers (low load) and source workers (high queue depth),
    /// then creates steal hints for consumption by the job manager.
    ///
    /// Uses round-robin source selection to distribute stealing load evenly
    /// across all available sources.
    pub(super) async fn coordinate_work_stealing(&self) -> Result<()> {
        let targets = self.find_steal_targets().await?;
        let sources = self.find_steal_sources().await?;

        if targets.is_empty() || sources.is_empty() {
            return Ok(());
        }

        // Sort sources by queue depth (descending) for deterministic selection
        let mut sorted_sources = sources;
        sorted_sources.sort_by(|a, b| b.queue_depth.cmp(&a.queue_depth));

        // Get and update round-robin counter
        let mut counter = self.steal_source_counter.write().await;
        let mut hints_created = 0usize;

        // Limit targets per coordination round (Tiger Style)
        const MAX_TARGETS_PER_ROUND: usize = 5;

        for target in targets.iter().take(MAX_TARGETS_PER_ROUND) {
            // Round-robin through sources
            let sources_count = sorted_sources.len().min(u32::MAX as usize) as u32;
            let source_index = (*counter % sources_count) as usize;
            *counter = (*counter + 1) % sources_count;

            let source = &sorted_sources[source_index];

            // Skip if target and source are the same worker
            if target.worker_id == source.worker_id {
                continue;
            }

            debug!(
                target = %target.worker_id,
                source = %source.worker_id,
                source_depth = source.queue_depth,
                source_index = source_index,
                "coordinating work stealing"
            );

            debug_assert!(
                target.load < self.config.steal_load_threshold,
                "WORK_STEAL: target '{}' load ({}) must be below threshold ({})",
                target.worker_id,
                target.load,
                self.config.steal_load_threshold
            );
            debug_assert!(
                source.queue_depth > self.config.steal_queue_threshold,
                "WORK_STEAL: source '{}' queue depth ({}) must exceed threshold ({})",
                source.worker_id,
                source.queue_depth,
                self.config.steal_queue_threshold
            );

            // Create typed steal hint with TTL
            let hint = StealHint::new(
                target.worker_id.clone(),
                source.worker_id.clone(),
                MAX_STEAL_BATCH,
                source_index as u32,
            );

            // Store hint with composite key
            let key = verified::steal_hint_key(&target.worker_id, &source.worker_id);
            let value = serde_json::to_string(&hint)?;

            self.store
                .write(WriteRequest {
                    command: WriteCommand::Set { key, value },
                })
                .await?;

            hints_created += 1;
        }

        // Cleanup expired hints at the end of each coordination round
        let cleaned = self.cleanup_expired_hints().await?;
        if cleaned > 0 || hints_created > 0 {
            debug!(hints_created = hints_created, hints_cleaned = cleaned, "work stealing coordination completed");
        }

        Ok(())
    }

    /// Get pending steal hints for a worker.
    ///
    /// Returns all non-expired steal hints where the given worker is the target.
    /// The job manager should call this periodically to check for stealing opportunities.
    pub async fn get_steal_hints(&self, worker_id: &str) -> Result<Vec<StealHint>> {
        // Tiger Style: argument validation
        debug_assert!(!worker_id.is_empty(), "WORK_STEAL: worker_id must not be empty for get_steal_hints");

        // Scan for hints targeting this worker
        let prefix = verified::steal_hint_prefix(worker_id);

        let scan_result = self
            .store
            .scan(aspen_core::ScanRequest {
                prefix,
                limit_results: Some(MAX_STEAL_HINTS_PER_WORKER),
                continuation_token: None,
            })
            .await?;

        let mut hints = Vec::new();
        for entry in scan_result.entries {
            match serde_json::from_str::<StealHint>(&entry.value) {
                Ok(hint) => {
                    // Only include non-expired hints
                    if !hint.is_expired() {
                        hints.push(hint);
                    }
                }
                Err(e) => {
                    // Log and skip malformed entries
                    warn!(
                        key = %entry.key,
                        error = %e,
                        "failed to parse steal hint, skipping"
                    );
                }
            }
        }

        Ok(hints)
    }

    /// Consume (acknowledge and delete) a steal hint.
    ///
    /// Call this after successfully stealing work from the source worker,
    /// or to dismiss a hint that is no longer actionable.
    ///
    /// This operation is idempotent - calling it on an already-consumed or
    /// non-existent hint returns success.
    pub async fn consume_steal_hint(&self, target_id: &str, source_id: &str) -> Result<()> {
        // Tiger Style: argument validation
        debug_assert!(!target_id.is_empty(), "WORK_STEAL: target_id must not be empty");
        debug_assert!(!source_id.is_empty(), "WORK_STEAL: source_id must not be empty");
        debug_assert!(target_id != source_id, "WORK_STEAL: target_id must differ from source_id");

        let key = verified::steal_hint_key(target_id, source_id);

        // Delete is idempotent - succeeds even if key doesn't exist
        let _ = self
            .store
            .write(WriteRequest {
                command: WriteCommand::Delete { key },
            })
            .await;

        debug!(target = target_id, source = source_id, "steal hint consumed");

        Ok(())
    }

    /// Check if a specific steal hint exists and is valid.
    pub async fn has_steal_hint(&self, target_id: &str, source_id: &str) -> Result<bool> {
        let key = verified::steal_hint_key(target_id, source_id);

        match self.store.read(aspen_core::ReadRequest::new(key)).await {
            Ok(result) => {
                if let Some(kv) = result.kv {
                    match serde_json::from_str::<StealHint>(&kv.value) {
                        Ok(hint) => Ok(!hint.is_expired()),
                        Err(_) => Ok(false),
                    }
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }

    /// Clean up expired steal hints.
    ///
    /// Scans for all steal hints and removes those that have passed their
    /// expiration deadline.
    pub(super) async fn cleanup_expired_hints(&self) -> Result<usize> {
        let scan_result = self
            .store
            .scan(aspen_core::ScanRequest {
                prefix: verified::STEAL_HINT_PREFIX.to_string(),
                limit_results: Some(MAX_HINT_CLEANUP_BATCH),
                continuation_token: None,
            })
            .await?;

        let mut deleted = 0usize;

        for entry in scan_result.entries {
            let should_delete = match serde_json::from_str::<StealHint>(&entry.value) {
                Ok(hint) => hint.is_expired(),
                Err(_) => {
                    // Malformed entry - delete it
                    warn!(key = %entry.key, "deleting malformed steal hint");
                    true
                }
            };

            if should_delete {
                let _ = self
                    .store
                    .write(WriteRequest {
                        command: WriteCommand::Delete { key: entry.key },
                    })
                    .await;
                deleted += 1;
            }
        }

        Ok(deleted)
    }

    /// Force cleanup of all steal hints (for testing or shutdown).
    pub async fn clear_all_steal_hints(&self) -> Result<usize> {
        let scan_result = self
            .store
            .scan(aspen_core::ScanRequest {
                prefix: verified::STEAL_HINT_PREFIX.to_string(),
                limit_results: Some(MAX_HINT_CLEANUP_BATCH),
                continuation_token: None,
            })
            .await?;

        let mut deleted = 0usize;

        for entry in scan_result.entries {
            let _ = self
                .store
                .write(WriteRequest {
                    command: WriteCommand::Delete { key: entry.key },
                })
                .await;
            deleted += 1;
        }

        if deleted > 0 {
            info!(count = deleted, "cleared all steal hints");
        }

        Ok(deleted)
    }
}
