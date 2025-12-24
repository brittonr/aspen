//! Automatic shard split/merge triggering based on metrics.
//!
//! This module provides background automation for dynamic shard management:
//! - Monitors shard metrics (size, QPS)
//! - Triggers splits when shards exceed thresholds
//! - Triggers merges for underutilized adjacent shards
//!
//! # Leader-Only Operation
//!
//! Automation only runs on the current Raft leader to prevent conflicts.
//! Non-leaders simply sleep and check periodically.
//!
//! # Tiger Style
//!
//! - Fixed check interval (10 seconds) to prevent runaway automation
//! - Maximum concurrent operations (1 split or merge at a time)
//! - Graceful shutdown via CancellationToken
//! - All decisions logged for observability

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use super::metrics::METRICS_CHECK_INTERVAL;
use super::metrics::ShardMetricsCollector;
use super::router::ShardId;
use super::topology::DEFAULT_MERGE_MAX_COMBINED_BYTES;
use super::topology::ShardState;
use super::topology::ShardTopology;
use crate::api::ClusterController;

/// Configuration for the shard automation manager.
#[derive(Debug, Clone)]
pub struct AutomationConfig {
    /// How often to check for split/merge triggers.
    pub check_interval: Duration,
    /// Whether to actually trigger operations or just log them (dry-run mode).
    pub dry_run: bool,
    /// Maximum number of concurrent split operations.
    pub max_concurrent_splits: usize,
    /// Maximum number of concurrent merge operations.
    pub max_concurrent_merges: usize,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self {
            check_interval: METRICS_CHECK_INTERVAL,
            dry_run: false,
            max_concurrent_splits: 1,
            max_concurrent_merges: 1,
        }
    }
}

/// Background manager for automatic shard split/merge operations.
///
/// Periodically checks shard metrics and triggers topology changes when
/// thresholds are exceeded. Only the cluster leader performs automation.
pub struct ShardAutomationManager {
    /// Metrics collector for all shards.
    metrics: Arc<ShardMetricsCollector>,
    /// Shard topology for state queries and updates.
    topology: Arc<RwLock<ShardTopology>>,
    /// Cluster controller for leader checks.
    controller: Arc<dyn ClusterController>,
    /// Node ID for leader comparison.
    node_id: u64,
    /// Configuration options.
    config: AutomationConfig,
    /// Cancellation token for graceful shutdown.
    cancel_token: CancellationToken,
}

impl ShardAutomationManager {
    /// Create a new automation manager.
    pub fn new(
        metrics: Arc<ShardMetricsCollector>,
        topology: Arc<RwLock<ShardTopology>>,
        controller: Arc<dyn ClusterController>,
        node_id: u64,
        config: AutomationConfig,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            metrics,
            topology,
            controller,
            node_id,
            config,
            cancel_token,
        }
    }

    /// Spawn the automation task as a background task.
    ///
    /// Returns a JoinHandle that can be used to await completion.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(self.run())
    }

    /// Run the automation loop.
    ///
    /// Periodically checks metrics and triggers operations when appropriate.
    async fn run(self) {
        let mut check_interval = interval(self.config.check_interval);
        check_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        info!(
            node_id = self.node_id,
            check_interval_secs = self.config.check_interval.as_secs(),
            dry_run = self.config.dry_run,
            "Starting shard automation manager"
        );

        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    info!("Shard automation manager shutting down");
                    break;
                }
                _ = check_interval.tick() => {
                    if let Err(e) = self.check_and_trigger().await {
                        warn!(error = %e, "Automation check failed");
                    }
                }
            }
        }
    }

    /// Check metrics and trigger split/merge operations if needed.
    async fn check_and_trigger(&self) -> anyhow::Result<()> {
        // Only run on leader
        if !self.is_leader().await? {
            debug!("Not leader, skipping automation check");
            return Ok(());
        }

        // Check for shards that should be split
        let shards_to_split = self.metrics.shards_to_split();
        for shard_id in shards_to_split.into_iter().take(self.config.max_concurrent_splits) {
            if let Err(e) = self.trigger_split(shard_id).await {
                warn!(shard_id, error = %e, "Failed to trigger split");
            }
        }

        // Check for shards that could be merged
        if let Some((source, target)) = self.find_merge_candidates().await
            && let Err(e) = self.trigger_merge(source, target).await
        {
            warn!(source, target, error = %e, "Failed to trigger merge");
        }

        // Reset metrics windows for the next period
        self.metrics.reset_all_windows();

        Ok(())
    }

    /// Check if this node is the current Raft leader.
    async fn is_leader(&self) -> anyhow::Result<bool> {
        match self.controller.get_leader().await {
            Ok(Some(leader_id)) => Ok(leader_id == self.node_id),
            Ok(None) => Ok(false), // No leader elected
            Err(e) => {
                debug!(error = %e, "Failed to get leader, assuming not leader");
                Ok(false)
            }
        }
    }

    /// Trigger a split operation for the given shard.
    async fn trigger_split(&self, shard_id: ShardId) -> anyhow::Result<()> {
        let topology = self.topology.read().await;

        // Verify shard is still in splittable state
        let shard = topology.get_shard(shard_id).ok_or_else(|| anyhow::anyhow!("shard {} not found", shard_id))?;

        if !matches!(shard.state, ShardState::Active) {
            debug!(shard_id, state = ?shard.state, "Shard not in active state, skipping split");
            return Ok(());
        }

        // Calculate split key (use middle of range for now)
        let split_key = self.calculate_split_key(shard_id, &topology).await?;

        // Get next shard ID
        let new_shard_id = topology.next_shard_id();

        drop(topology); // Release read lock

        if self.config.dry_run {
            info!(
                shard_id,
                split_key = %split_key,
                new_shard_id,
                "[DRY RUN] Would trigger split"
            );
            return Ok(());
        }

        info!(
            shard_id,
            split_key = %split_key,
            new_shard_id,
            "Triggering automatic shard split"
        );

        // Apply the split to the topology
        let mut topology = self.topology.write().await;
        let timestamp =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

        topology.apply_split(shard_id, split_key, new_shard_id, timestamp)?;

        info!(shard_id, new_shard_id, new_version = topology.version, "Split completed");

        Ok(())
    }

    /// Calculate a good split key for a shard.
    ///
    /// Currently uses a simple approach of finding the midpoint of the key range.
    /// Future improvements could analyze actual key distribution.
    async fn calculate_split_key(&self, shard_id: ShardId, topology: &ShardTopology) -> anyhow::Result<String> {
        let shard = topology.get_shard(shard_id).ok_or_else(|| anyhow::anyhow!("shard {} not found", shard_id))?;

        let start = &shard.key_range.start_key;
        let end = &shard.key_range.end_key;

        // Simple midpoint calculation
        // For empty start, use first char. For empty end, extend from start.
        let split_key = if start.is_empty() && end.is_empty() {
            // Full range - split at "8" (middle of 0-f hex space)
            "8".to_string()
        } else if start.is_empty() {
            // ["", end) - split at first char of end's midpoint
            let first_char = end.chars().next().unwrap_or('8');
            let mid_char = ((first_char as u8) / 2) as char;
            mid_char.to_string()
        } else if end.is_empty() {
            // [start, "") - extend start with midpoint char
            format!("{}8", start)
        } else {
            // [start, end) - find common prefix and split there
            let common_len = start.chars().zip(end.chars()).take_while(|(a, b)| a == b).count();

            if common_len < start.len() && common_len < end.len() {
                // Find midpoint between differing chars
                let start_char = start.chars().nth(common_len).unwrap() as u8;
                let end_char = end.chars().nth(common_len).unwrap() as u8;
                let mid_char = ((start_char + end_char) / 2) as char;
                format!("{}{}", &start[..common_len], mid_char)
            } else {
                // Fallback: use start + "8"
                format!("{}8", start)
            }
        };

        // Validate the split key is within range
        if !shard.key_range.contains(&split_key) || split_key == *start {
            return Err(anyhow::anyhow!(
                "calculated split key '{}' is not valid for range [{}, {})",
                split_key,
                start,
                end
            ));
        }

        Ok(split_key)
    }

    /// Find a pair of adjacent shards that can be merged.
    ///
    /// Returns (source_shard_id, target_shard_id) if a merge candidate is found.
    async fn find_merge_candidates(&self) -> Option<(ShardId, ShardId)> {
        let mergeable = self.metrics.shards_to_merge();
        if mergeable.len() < 2 {
            return None;
        }

        let topology = self.topology.read().await;

        // Find pairs of adjacent shards that can be merged
        for &source in &mergeable {
            for &target in &mergeable {
                if source == target {
                    continue;
                }

                // Check if shards are adjacent
                let source_info = topology.get_shard(source)?;
                let target_info = topology.get_shard(target)?;

                if !source_info.key_range.is_adjacent_to(&target_info.key_range) {
                    continue;
                }

                // Check both are active
                if !matches!(source_info.state, ShardState::Active) || !matches!(target_info.state, ShardState::Active)
                {
                    continue;
                }

                // Check combined size is acceptable
                let source_metrics = self.metrics.get(source)?;
                let target_metrics = self.metrics.get(target)?;
                let combined_size = source_metrics.size_bytes() + target_metrics.size_bytes();

                if combined_size <= DEFAULT_MERGE_MAX_COMBINED_BYTES {
                    return Some((source, target));
                }
            }
        }

        None
    }

    /// Trigger a merge operation from source to target shard.
    async fn trigger_merge(&self, source: ShardId, target: ShardId) -> anyhow::Result<()> {
        if self.config.dry_run {
            info!(source, target, "[DRY RUN] Would trigger merge");
            return Ok(());
        }

        info!(source, target, "Triggering automatic shard merge");

        let mut topology = self.topology.write().await;
        let timestamp =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

        topology.apply_merge(source, target, timestamp)?;

        info!(source, target, new_version = topology.version, "Merge completed");

        Ok(())
    }
}

impl std::fmt::Debug for ShardAutomationManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShardAutomationManager")
            .field("node_id", &self.node_id)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeterministicClusterController;

    fn create_test_manager(dry_run: bool) -> ShardAutomationManager {
        let metrics = Arc::new(ShardMetricsCollector::new());
        let topology = Arc::new(RwLock::new(ShardTopology::new(2, 1000)));
        let controller = Arc::new(DeterministicClusterController::new());
        let config = AutomationConfig {
            dry_run,
            ..Default::default()
        };
        let cancel_token = CancellationToken::new();

        ShardAutomationManager::new(metrics, topology, controller, 1, config, cancel_token)
    }

    #[tokio::test]
    async fn test_automation_manager_creation() {
        let manager = create_test_manager(true);
        assert_eq!(manager.node_id, 1);
        assert!(manager.config.dry_run);
    }

    #[tokio::test]
    async fn test_calculate_split_key_full_range() {
        let manager = create_test_manager(true);

        // Create topology with single shard covering full range
        let topology = ShardTopology::new(1, 1000);

        let split_key = manager.calculate_split_key(0, &topology).await.expect("should calculate split key");

        assert!(!split_key.is_empty(), "split key should not be empty");
    }

    #[tokio::test]
    async fn test_find_merge_candidates_no_candidates() {
        let manager = create_test_manager(true);

        // No shards registered, so no candidates
        let candidates = manager.find_merge_candidates().await;
        assert!(candidates.is_none());
    }

    #[tokio::test]
    async fn test_is_leader_no_leader() {
        let manager = create_test_manager(true);

        // DeterministicClusterController returns None for leader
        let is_leader = manager.is_leader().await.expect("should not error");
        assert!(!is_leader);
    }
}
