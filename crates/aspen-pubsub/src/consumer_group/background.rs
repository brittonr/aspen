//! Background tasks for consumer group maintenance.
//!
//! Provides periodic background tasks for:
//! 1. **Visibility Timeout Processor**: Redelivers or dead-letters expired pending messages
//! 2. **Consumer Expiration Detector**: Removes consumers that haven't sent heartbeats
//!
//! # Architecture
//!
//! Both tasks follow the CancellationToken pattern for graceful shutdown and use
//! Tiger Style resource bounds to prevent unbounded work per iteration.

use std::sync::Arc;
use std::time::Duration;

use aspen_core::KeyValueStore;
use async_trait::async_trait;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::consumer_group::constants::CONSUMER_HEARTBEAT_TIMEOUT_MS;
use crate::consumer_group::error::Result;
use crate::consumer_group::pending::PendingEntriesManager;
use crate::consumer_group::pending::RaftPendingEntriesList;
use crate::consumer_group::storage;
use crate::consumer_group::types::ConsumerGroupId;
use crate::consumer_group::types::GroupState;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for background tasks.
#[derive(Debug, Clone)]
pub struct BackgroundTasksConfig {
    /// Interval for visibility timeout processing (default: 1 second).
    pub visibility_check_interval: Duration,

    /// Interval for consumer expiration checking (default: 5 seconds).
    pub consumer_expiry_interval: Duration,

    /// Maximum pending entries to process per visibility check iteration.
    pub max_pending_per_iteration: u32,

    /// Maximum consumers to check per expiry iteration.
    pub max_consumers_per_iteration: u32,

    /// Maximum number of groups to scan per iteration.
    pub max_groups_per_iteration: u32,
}

impl Default for BackgroundTasksConfig {
    fn default() -> Self {
        Self {
            visibility_check_interval: Duration::from_secs(1),
            consumer_expiry_interval: Duration::from_secs(5),
            max_pending_per_iteration: 100,
            max_consumers_per_iteration: 50,
            max_groups_per_iteration: 50,
        }
    }
}

// ============================================================================
// Visibility Timeout Processor
// ============================================================================

/// Provides consumer selection for redelivery of expired messages.
#[async_trait]
pub trait ConsumerSelector: Send + Sync {
    /// Select a consumer for redelivering an expired message.
    ///
    /// Returns `None` if no suitable consumer is available.
    async fn select_consumer_for_redelivery(&self, group_id: &ConsumerGroupId) -> Result<Option<RedeliveryTarget>>;
}

/// Target consumer for message redelivery.
#[derive(Debug, Clone)]
pub struct RedeliveryTarget {
    /// Consumer ID to redeliver to.
    pub consumer_id: crate::consumer_group::types::ConsumerId,
    /// Consumer's current fencing token.
    pub fencing_token: u64,
    /// Visibility timeout for the redelivered message.
    pub visibility_timeout_ms: u64,
}

/// Default consumer selector that picks an active consumer with lowest pending count.
pub struct DefaultConsumerSelector<K: KeyValueStore + ?Sized> {
    store: Arc<K>,
}

impl<K: KeyValueStore + ?Sized> DefaultConsumerSelector<K> {
    /// Create a new consumer selector.
    pub fn new(store: Arc<K>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<K: KeyValueStore + ?Sized + 'static> ConsumerSelector for DefaultConsumerSelector<K> {
    async fn select_consumer_for_redelivery(&self, group_id: &ConsumerGroupId) -> Result<Option<RedeliveryTarget>> {
        let now_ms = storage::now_unix_ms();
        let consumers = storage::list_consumers(&*self.store, group_id).await?;
        let group_state = storage::load_group_state(&*self.store, group_id).await?;

        // Find an active consumer with lowest pending count
        let mut best: Option<RedeliveryTarget> = None;
        let mut best_pending_count = u32::MAX;

        for consumer in consumers {
            // Skip consumers that have timed out
            let deadline = consumer.last_heartbeat_ms + CONSUMER_HEARTBEAT_TIMEOUT_MS;
            if now_ms > deadline {
                continue;
            }

            // Prefer consumer with lowest pending count
            if consumer.pending_count < best_pending_count {
                best_pending_count = consumer.pending_count;
                best = Some(RedeliveryTarget {
                    consumer_id: consumer.consumer_id,
                    fencing_token: consumer.fencing_token,
                    visibility_timeout_ms: group_state.visibility_timeout_ms,
                });
            }
        }

        Ok(best)
    }
}

/// Spawns the visibility timeout processor background task.
///
/// This task periodically scans for pending messages that have exceeded their
/// visibility timeout and either redelivers them to another consumer or moves
/// them to the dead letter queue if max delivery attempts exceeded.
///
/// # Returns
///
/// A `CancellationToken` that can be used to stop the task.
pub fn spawn_visibility_timeout_processor<K, S>(
    pending_manager: Arc<RaftPendingEntriesList<K>>,
    store: Arc<K>,
    consumer_selector: Arc<S>,
    config: BackgroundTasksConfig,
) -> CancellationToken
where
    K: KeyValueStore + ?Sized + 'static,
    S: ConsumerSelector + 'static,
{
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        run_visibility_timeout_loop(pending_manager, store, consumer_selector, config, cancel_clone).await;
    });

    cancel
}

async fn run_visibility_timeout_loop<K, S>(
    pending_manager: Arc<RaftPendingEntriesList<K>>,
    store: Arc<K>,
    consumer_selector: Arc<S>,
    config: BackgroundTasksConfig,
    cancel: CancellationToken,
) where
    K: KeyValueStore + ?Sized + 'static,
    S: ConsumerSelector + 'static,
{
    let mut ticker = interval(config.visibility_check_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!(
        interval_ms = config.visibility_check_interval.as_millis() as u64,
        max_pending = config.max_pending_per_iteration,
        "Visibility timeout processor started"
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Visibility timeout processor shutting down");
                break;
            }
            _ = ticker.tick() => {
                if let Err(e) = run_visibility_iteration(
                    &pending_manager,
                    &store,
                    &consumer_selector,
                    &config,
                ).await {
                    warn!(error = %e, "Visibility timeout iteration failed");
                }
            }
        }
    }
}

async fn run_visibility_iteration<K, S>(
    pending_manager: &RaftPendingEntriesList<K>,
    store: &Arc<K>,
    consumer_selector: &Arc<S>,
    config: &BackgroundTasksConfig,
) -> Result<()>
where
    K: KeyValueStore + ?Sized + 'static,
    S: ConsumerSelector + 'static,
{
    let now_ms = storage::now_unix_ms();

    // List all groups to process
    let groups = list_all_groups(&**store, config.max_groups_per_iteration).await?;

    let mut total_redelivered: u32 = 0;
    let mut total_dead_lettered: u32 = 0;

    for group in groups {
        let group_id = &group.group_id;
        let max_attempts = group.max_delivery_attempts;

        // Find expired pending entries for this group
        let expired = pending_manager.find_expired(group_id, now_ms, config.max_pending_per_iteration).await?;

        if expired.is_empty() {
            continue;
        }

        debug!(
            group_id = %group_id.as_str(),
            expired_count = expired.len(),
            "Processing expired pending entries"
        );

        for entry in expired {
            // Check if max delivery attempts exceeded
            if max_attempts > 0 && entry.delivery_attempt >= max_attempts {
                // Move to dead letter queue
                pending_manager.dead_letter(group_id, entry.cursor, "max_attempts_exceeded", now_ms).await?;
                total_dead_lettered += 1;
                continue;
            }

            // Try to select a consumer for redelivery
            match consumer_selector.select_consumer_for_redelivery(group_id).await? {
                Some(target) => {
                    // Redeliver to selected consumer
                    let params = crate::consumer_group::pending::RedeliveryParams {
                        group_id,
                        cursor: entry.cursor,
                        new_consumer_id: &target.consumer_id,
                        visibility_timeout_ms: target.visibility_timeout_ms,
                        fencing_token: target.fencing_token,
                        now_ms,
                    };
                    match pending_manager.redeliver(params).await {
                        Ok(_receipt_handle) => {
                            total_redelivered += 1;
                        }
                        Err(e) => {
                            warn!(
                                cursor = entry.cursor,
                                error = %e,
                                "Failed to redeliver message"
                            );
                        }
                    }
                }
                None => {
                    // No consumer available, message will be picked up next iteration
                    debug!(
                        group_id = %group_id.as_str(),
                        cursor = entry.cursor,
                        "No consumer available for redelivery"
                    );
                }
            }
        }
    }

    if total_redelivered > 0 || total_dead_lettered > 0 {
        info!(
            redelivered = total_redelivered,
            dead_lettered = total_dead_lettered,
            "Visibility timeout iteration completed"
        );
    } else {
        debug!("Visibility timeout: no expired entries");
    }

    Ok(())
}

// ============================================================================
// Consumer Expiration Detector
// ============================================================================

/// Spawns the consumer expiration detector background task.
///
/// This task periodically scans for consumers that haven't sent heartbeats
/// within the timeout period and removes them from their groups.
///
/// # Returns
///
/// A `CancellationToken` that can be used to stop the task.
pub fn spawn_consumer_expiration_detector<K>(store: Arc<K>, config: BackgroundTasksConfig) -> CancellationToken
where
    K: KeyValueStore + ?Sized + 'static,
{
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        run_consumer_expiration_loop(store, config, cancel_clone).await;
    });

    cancel
}

async fn run_consumer_expiration_loop<K>(store: Arc<K>, config: BackgroundTasksConfig, cancel: CancellationToken)
where
    K: KeyValueStore + ?Sized + 'static,
{
    let mut ticker = interval(config.consumer_expiry_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!(
        interval_ms = config.consumer_expiry_interval.as_millis() as u64,
        max_consumers = config.max_consumers_per_iteration,
        "Consumer expiration detector started"
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Consumer expiration detector shutting down");
                break;
            }
            _ = ticker.tick() => {
                if let Err(e) = run_expiration_iteration(&store, &config).await {
                    warn!(error = %e, "Consumer expiration iteration failed");
                }
            }
        }
    }
}

async fn run_expiration_iteration<K>(store: &Arc<K>, config: &BackgroundTasksConfig) -> Result<()>
where
    K: KeyValueStore + ?Sized + 'static,
{
    let now_ms = storage::now_unix_ms();

    // List all groups
    let groups = list_all_groups(&**store, config.max_groups_per_iteration).await?;

    let mut total_expired: u32 = 0;

    for group in groups {
        let group_id = &group.group_id;

        // List consumers for this group
        let consumers = storage::list_consumers(&**store, group_id).await?;

        let mut expired_consumers = Vec::new();

        for consumer in consumers.iter().take(config.max_consumers_per_iteration as usize) {
            let deadline = consumer.last_heartbeat_ms + CONSUMER_HEARTBEAT_TIMEOUT_MS;
            if now_ms > deadline {
                expired_consumers.push(consumer.consumer_id.clone());
            }
        }

        if expired_consumers.is_empty() {
            continue;
        }

        debug!(
            group_id = %group_id.as_str(),
            expired_count = expired_consumers.len(),
            "Removing expired consumers"
        );

        // Remove expired consumers
        for consumer_id in expired_consumers {
            if let Err(e) = storage::delete_consumer_state(&**store, group_id, &consumer_id).await {
                warn!(
                    group_id = %group_id.as_str(),
                    consumer_id = %consumer_id.as_str(),
                    error = %e,
                    "Failed to remove expired consumer"
                );
            } else {
                total_expired += 1;
            }
        }

        // Update group member count and state
        let remaining_count = storage::count_consumers(&**store, group_id).await?;
        let mut updated_group = group.clone();
        updated_group.member_count = remaining_count;
        updated_group.updated_at_ms = now_ms;

        // Update group state based on member count
        if remaining_count == 0 {
            updated_group.state = crate::consumer_group::types::GroupStateType::Empty;
            updated_group.leader_id = None;
        }

        storage::save_group_state(&**store, &updated_group).await?;
    }

    if total_expired > 0 {
        info!(expired_consumers = total_expired, "Consumer expiration iteration completed");
    } else {
        debug!("Consumer expiration: no expired consumers");
    }

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// List all consumer groups up to a limit.
async fn list_all_groups<K>(store: &K, limit: u32) -> Result<Vec<GroupState>>
where
    K: KeyValueStore + ?Sized,
{
    use aspen_core::kv::ScanRequest;

    use crate::consumer_group::keys::ConsumerGroupKeys;

    let (start, _end) = ConsumerGroupKeys::all_groups_range();
    let prefix = ConsumerGroupKeys::key_to_string(&start);

    let result = store
        .scan(ScanRequest {
            prefix,
            limit: Some(limit),
            continuation_token: None,
        })
        .await?;

    let mut groups = Vec::new();
    for kv in result.entries {
        // Only process state keys
        if ConsumerGroupKeys::is_group_state_key(kv.key.as_bytes()) {
            let state: GroupState = rmp_serde::from_slice(kv.value.as_bytes()).map_err(|e| {
                crate::consumer_group::error::ConsumerGroupError::SerializationFailed {
                    message: format!("failed to deserialize group state: {}", e),
                }
            })?;
            groups.push(state);
        }
    }

    Ok(groups)
}

// ============================================================================
// Combined Background Tasks Handle
// ============================================================================

/// Handle for all background tasks.
///
/// Provides a unified interface for starting and stopping background tasks.
pub struct BackgroundTasksHandle {
    visibility_cancel: CancellationToken,
    expiration_cancel: CancellationToken,
}

impl BackgroundTasksHandle {
    /// Spawn all background tasks.
    pub fn spawn<K>(
        pending_manager: Arc<RaftPendingEntriesList<K>>,
        store: Arc<K>,
        config: BackgroundTasksConfig,
    ) -> Self
    where
        K: KeyValueStore + ?Sized + 'static,
    {
        let consumer_selector = Arc::new(DefaultConsumerSelector::new(store.clone()));

        let visibility_cancel =
            spawn_visibility_timeout_processor(pending_manager, store.clone(), consumer_selector, config.clone());

        let expiration_cancel = spawn_consumer_expiration_detector(store, config);

        Self {
            visibility_cancel,
            expiration_cancel,
        }
    }

    /// Cancel all background tasks.
    pub fn cancel(&self) {
        self.visibility_cancel.cancel();
        self.expiration_cancel.cancel();
    }

    /// Check if all tasks have been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.visibility_cancel.is_cancelled() && self.expiration_cancel.is_cancelled()
    }
}

impl Drop for BackgroundTasksHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BackgroundTasksConfig::default();
        assert_eq!(config.visibility_check_interval, Duration::from_secs(1));
        assert_eq!(config.consumer_expiry_interval, Duration::from_secs(5));
        assert_eq!(config.max_pending_per_iteration, 100);
        assert_eq!(config.max_consumers_per_iteration, 50);
        assert_eq!(config.max_groups_per_iteration, 50);
    }

    #[test]
    fn test_config_timing_relationships() {
        let config = BackgroundTasksConfig::default();
        // Visibility check should be more frequent than consumer expiry
        assert!(config.visibility_check_interval < config.consumer_expiry_interval);
    }
}
