//! Background tasks for consumer group maintenance.
//!
//! This module provides background task runners for:
//! - Visibility timeout enforcement (redelivering expired messages)
//! - Consumer session cleanup (removing dead consumers)
//! - Rebalance coordination (for partitioned mode)

use std::sync::Arc;
use std::time::Duration;

use aspen_core::KeyValueStore;
use tokio::sync::watch;
use tracing::debug;
use tracing::error;
use tracing::warn;

use super::constants::CONSUMER_HEARTBEAT_TIMEOUT_MS;
use super::pending::PendingEntriesManager;
use super::pending::RedeliveryParams;
use super::storage;
use super::types::ConsumerGroupId;
use super::types::GroupStateType;

/// Background task for enforcing visibility timeouts.
///
/// Periodically scans for expired pending entries and redelivers them
/// to other consumers or moves them to the DLQ.
pub struct VisibilityTimeoutEnforcer<K: KeyValueStore + ?Sized, P: PendingEntriesManager> {
    store: Arc<K>,
    pending_manager: Arc<P>,
    group_id: ConsumerGroupId,
    scan_interval: Duration,
}

impl<K: KeyValueStore + ?Sized + 'static, P: PendingEntriesManager + 'static> VisibilityTimeoutEnforcer<K, P> {
    /// Create a new visibility timeout enforcer.
    pub fn new(store: Arc<K>, pending_manager: Arc<P>, group_id: ConsumerGroupId) -> Self {
        Self {
            store,
            pending_manager,
            group_id,
            scan_interval: Duration::from_secs(5),
        }
    }

    /// Set the scan interval.
    pub fn with_scan_interval(mut self, interval: Duration) -> Self {
        self.scan_interval = interval;
        self
    }

    /// Run the enforcer until shutdown is signaled.
    pub async fn run(&self, mut shutdown: watch::Receiver<bool>) {
        let mut interval = tokio::time::interval(self.scan_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.scan_and_redeliver().await {
                        error!(group_id = %self.group_id, error = %e, "visibility timeout scan failed");
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        debug!(group_id = %self.group_id, "visibility timeout enforcer shutting down");
                        break;
                    }
                }
            }
        }
    }

    /// Scan for expired entries and redeliver them.
    async fn scan_and_redeliver(&self) -> super::error::Result<()> {
        let now_ms = storage::now_unix_ms();

        // Find expired entries
        let expired = self.pending_manager.find_expired(&self.group_id, now_ms, 100).await?;

        if expired.is_empty() {
            return Ok(());
        }

        debug!(
            group_id = %self.group_id,
            count = expired.len(),
            "found expired pending entries"
        );

        // Load group state to get config
        let group_state = match storage::try_load_group_state(&*self.store, &self.group_id).await? {
            Some(state) => state,
            None => {
                warn!(group_id = %self.group_id, "group not found during visibility scan");
                return Ok(());
            }
        };

        // Get available consumers
        let consumers = storage::list_consumers(&*self.store, &self.group_id).await?;
        if consumers.is_empty() {
            debug!(group_id = %self.group_id, "no consumers available for redelivery");
            return Ok(());
        }

        // Redeliver each expired entry
        for entry in expired {
            // Check if max attempts exceeded
            if group_state.max_delivery_attempts > 0 && entry.delivery_attempt >= group_state.max_delivery_attempts {
                // Move to DLQ
                debug!(
                    group_id = %self.group_id,
                    cursor = entry.cursor,
                    attempts = entry.delivery_attempt,
                    "moving message to DLQ - max attempts exceeded"
                );
                self.pending_manager
                    .dead_letter(&self.group_id, entry.cursor, "max_attempts_exceeded", now_ms)
                    .await?;
                continue;
            }

            // Pick a consumer for redelivery (simple round-robin for now)
            // In a more sophisticated implementation, we'd consider load balancing
            let consumer_idx = (entry.cursor as usize) % consumers.len();
            let consumer = &consumers[consumer_idx];

            // Redeliver
            let params = RedeliveryParams {
                group_id: &self.group_id,
                cursor: entry.cursor,
                new_consumer_id: &consumer.consumer_id,
                visibility_timeout_ms: group_state.visibility_timeout_ms,
                fencing_token: consumer.fencing_token,
                now_ms,
            };

            match self.pending_manager.redeliver(params).await {
                Ok(_) => {
                    debug!(
                        group_id = %self.group_id,
                        cursor = entry.cursor,
                        new_consumer = %consumer.consumer_id,
                        "redelivered expired message"
                    );
                }
                Err(e) => {
                    warn!(
                        group_id = %self.group_id,
                        cursor = entry.cursor,
                        error = %e,
                        "failed to redeliver message"
                    );
                }
            }
        }

        Ok(())
    }
}

/// Background task for cleaning up dead consumers.
///
/// Periodically scans for consumers that haven't sent a heartbeat
/// within the timeout period and removes them from the group.
pub struct ConsumerSessionCleanup<K: KeyValueStore + ?Sized> {
    store: Arc<K>,
    group_id: ConsumerGroupId,
    scan_interval: Duration,
    heartbeat_timeout_ms: u64,
}

impl<K: KeyValueStore + ?Sized + 'static> ConsumerSessionCleanup<K> {
    /// Create a new consumer session cleanup task.
    pub fn new(store: Arc<K>, group_id: ConsumerGroupId) -> Self {
        Self {
            store,
            group_id,
            scan_interval: Duration::from_secs(10),
            heartbeat_timeout_ms: CONSUMER_HEARTBEAT_TIMEOUT_MS,
        }
    }

    /// Set the scan interval.
    pub fn with_scan_interval(mut self, interval: Duration) -> Self {
        self.scan_interval = interval;
        self
    }

    /// Set the heartbeat timeout.
    pub fn with_heartbeat_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.heartbeat_timeout_ms = timeout_ms;
        self
    }

    /// Run the cleanup task until shutdown is signaled.
    pub async fn run(&self, mut shutdown: watch::Receiver<bool>) {
        let mut interval = tokio::time::interval(self.scan_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.cleanup_dead_consumers().await {
                        error!(group_id = %self.group_id, error = %e, "consumer cleanup failed");
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        debug!(group_id = %self.group_id, "consumer cleanup shutting down");
                        break;
                    }
                }
            }
        }
    }

    /// Remove consumers that have exceeded the heartbeat timeout.
    async fn cleanup_dead_consumers(&self) -> super::error::Result<()> {
        let now_ms = storage::now_unix_ms();

        // List all consumers
        let consumers = storage::list_consumers(&*self.store, &self.group_id).await?;

        let mut removed_count = 0u32;
        for consumer in consumers {
            if consumer.is_expired(self.heartbeat_timeout_ms, now_ms) {
                debug!(
                    group_id = %self.group_id,
                    consumer_id = %consumer.consumer_id,
                    last_heartbeat_ms = consumer.last_heartbeat_ms,
                    "removing dead consumer"
                );

                // Delete consumer state
                storage::delete_consumer_state(&*self.store, &self.group_id, &consumer.consumer_id).await?;

                removed_count += 1;
            }
        }

        if removed_count > 0 {
            // Update group state
            let mut group_state = storage::load_group_state(&*self.store, &self.group_id).await?;
            group_state.member_count = group_state.member_count.saturating_sub(removed_count);
            group_state.updated_at_ms = now_ms;

            // If no members left, transition to Empty state
            if group_state.member_count == 0 {
                group_state.state = GroupStateType::Empty;
                group_state.leader_id = None;
            }

            storage::save_group_state(&*self.store, &group_state).await?;

            debug!(
                group_id = %self.group_id,
                removed = removed_count,
                remaining = group_state.member_count,
                "cleaned up dead consumers"
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Background task tests require integration testing infrastructure
}
