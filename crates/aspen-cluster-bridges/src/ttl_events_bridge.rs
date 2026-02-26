//! Bridge for TTL expiration hook events.
//!
//! This module extends the TTL cleanup task to emit TtlExpired hook events
//! when keys are deleted due to TTL expiration.
//!
//! # Architecture
//!
//! The TTL events bridge wraps the storage's delete_expired_keys functionality
//! and emits a TtlExpired event for each key that is deleted. Events are
//! dispatched asynchronously to avoid blocking the cleanup process.
//!
//! # Tiger Style
//!
//! - Fixed batch size prevents unbounded work per iteration
//! - Non-blocking dispatch via tokio::spawn
//! - Same cleanup interval as standard TTL cleanup task

use std::sync::Arc;
use std::time::Duration;

use aspen_constants::coordination::MAX_TTL_EVENT_DISPATCHES;
use aspen_hooks::HookEvent;
use aspen_hooks::HookEventType;
use aspen_hooks::HookService;
use aspen_hooks::TtlExpiredPayload;
use aspen_raft::storage_shared::SharedRedbStorage;
use aspen_raft::ttl_cleanup::TtlCleanupConfig;
use tokio::task::JoinSet;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Spawn the TTL cleanup task with hook event emission.
///
/// This is a drop-in replacement for `spawn_redb_ttl_cleanup_task` that also
/// emits TtlExpired hook events for each deleted key.
///
/// Returns a CancellationToken that can be used to stop the task.
pub fn spawn_ttl_events_bridge(
    storage: Arc<SharedRedbStorage>,
    config: TtlCleanupConfig,
    service: Arc<HookService>,
    node_id: u64,
) -> CancellationToken {
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        run_ttl_events_bridge(storage, config, service, node_id, cancel_clone).await;
    });

    cancel
}

/// Run the TTL cleanup loop with hook event emission.
///
/// # Task Tracking (Tiger Style)
///
/// Uses `JoinSet` to track spawned dispatch tasks. On shutdown, waits for
/// in-flight dispatches to complete. Bounded to prevent unbounded task
/// spawning under high expiration rates.
async fn run_ttl_events_bridge(
    storage: Arc<SharedRedbStorage>,
    config: TtlCleanupConfig,
    service: Arc<HookService>,
    node_id: u64,
    cancel: CancellationToken,
) {
    let mut ticker = interval(config.cleanup_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Tiger Style: Track spawned tasks with JoinSet for graceful shutdown
    let mut dispatch_tasks: JoinSet<()> = JoinSet::new();

    info!(
        node_id,
        interval_secs = config.cleanup_interval.as_secs(),
        batch_size_keys = config.batch_size_keys,
        max_batches = config.max_batches_per_run,
        "TTL events bridge started"
    );

    loop {
        // Drain completed tasks before processing more events
        while dispatch_tasks.try_join_next().is_some() {}

        tokio::select! {
            _ = cancel.cancelled() => {
                info!(node_id, "TTL events bridge shutting down");
                break;
            }
            _ = ticker.tick() => {
                run_cleanup_iteration_with_events(&storage, &config, &service, node_id, &mut dispatch_tasks).await;
            }
        }
    }

    // Tiger Style: Wait for in-flight dispatches to complete on shutdown
    let in_flight = dispatch_tasks.len();
    if in_flight > 0 {
        info!(in_flight, "waiting for in-flight TTL event dispatches to complete");
        while dispatch_tasks.join_next().await.is_some() {}
        debug!("all in-flight TTL event dispatches completed");
    }
}

/// Run a single cleanup iteration, emitting events for each deleted key.
///
/// Uses the shared `dispatch_tasks` JoinSet to track spawned event dispatches
/// with backpressure when at capacity.
async fn run_cleanup_iteration_with_events(
    storage: &SharedRedbStorage,
    config: &TtlCleanupConfig,
    service: &Arc<HookService>,
    node_id: u64,
    dispatch_tasks: &mut JoinSet<()>,
) {
    let mut total_deleted: u64 = 0;
    let mut batches_run: u32 = 0;

    loop {
        if batches_run >= config.max_batches_per_run {
            debug!(
                node_id,
                total_deleted,
                batches_run,
                max_batches = config.max_batches_per_run,
                "TTL cleanup reached max batches limit"
            );
            break;
        }

        // Get expired keys with their metadata before deletion
        match storage.get_expired_keys_with_metadata(config.batch_size_keys) {
            Ok(expired_keys) => {
                if expired_keys.is_empty() {
                    break;
                }

                let batch_count = expired_keys.len();

                // Emit TtlExpired events for each key
                for (key, ttl_set_at) in &expired_keys {
                    // Tiger Style: Apply backpressure if too many in-flight tasks
                    if dispatch_tasks.len() >= MAX_TTL_EVENT_DISPATCHES {
                        warn!(
                            in_flight = dispatch_tasks.len(),
                            max = MAX_TTL_EVENT_DISPATCHES,
                            "TTL event dispatch tasks at capacity, waiting for completion"
                        );
                        // Wait for at least one to complete
                        let _ = dispatch_tasks.join_next().await;
                    }

                    let event = create_ttl_expired_event(node_id, key, *ttl_set_at);
                    let service_clone = Arc::clone(service);
                    // Spawn dispatch as separate task, tracked by JoinSet
                    dispatch_tasks.spawn(async move {
                        if let Err(e) = service_clone.dispatch(&event).await {
                            warn!(error = ?e, "failed to dispatch TtlExpired event");
                        }
                    });
                }

                // Delete the expired keys
                match storage.delete_expired_keys(config.batch_size_keys) {
                    Ok(deleted) => {
                        total_deleted += deleted as u64;
                        batches_run += 1;

                        if deleted < config.batch_size_keys {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!(node_id, error = %e, "TTL cleanup batch failed");
                        break;
                    }
                }

                if batch_count < config.batch_size_keys as usize {
                    break;
                }
            }
            Err(e) => {
                // Fallback to standard delete without events if metadata query fails
                warn!(node_id, error = %e, "failed to get expired keys metadata, falling back to standard delete");
                match storage.delete_expired_keys(config.batch_size_keys) {
                    Ok(deleted) => {
                        total_deleted += deleted as u64;
                        batches_run += 1;
                        if deleted == 0 || deleted < config.batch_size_keys {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!(node_id, error = %e, "TTL cleanup batch failed");
                        break;
                    }
                }
            }
        }
    }

    if total_deleted > 0 {
        let remaining = storage.count_expired_keys().unwrap_or(0);
        let with_ttl = storage.count_keys_with_ttl().unwrap_or(0);

        info!(
            node_id,
            total_deleted,
            batches_run,
            remaining_expired = remaining,
            keys_with_ttl = with_ttl,
            "TTL cleanup iteration completed with events"
        );
    } else {
        debug!(node_id, "TTL cleanup: no expired keys to delete");
    }
}

/// Serialize payload to JSON with warning on failure.
///
/// Tiger Style: Never silently mask serialization errors. Log and use default.
fn serialize_payload<T: serde::Serialize>(payload: T, event_type: &str) -> serde_json::Value {
    match serde_json::to_value(payload) {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, event_type, "failed to serialize hook event payload");
            serde_json::Value::Object(Default::default())
        }
    }
}

/// Create a TtlExpired hook event.
fn create_ttl_expired_event(node_id: u64, key: &str, ttl_set_at: Option<u64>) -> HookEvent {
    let timestamp = ttl_set_at.map(|ms| {
        // Convert milliseconds to SerializableTimestamp
        aspen_core::SerializableTimestamp::from_millis(ms)
    });

    let payload = TtlExpiredPayload {
        key: key.to_string(),
        ttl_set_at: timestamp,
    };

    HookEvent::new(HookEventType::TtlExpired, node_id, serialize_payload(payload, "TtlExpired"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_expired_event_creation() {
        let event = create_ttl_expired_event(1, "test/key", Some(1000));

        assert_eq!(event.event_type, HookEventType::TtlExpired);
        assert_eq!(event.node_id, 1);

        let payload: TtlExpiredPayload = serde_json::from_value(event.payload).unwrap();
        assert_eq!(payload.key, "test/key");
        assert!(payload.ttl_set_at.is_some());
    }

    #[test]
    fn test_ttl_expired_event_no_timestamp() {
        let event = create_ttl_expired_event(1, "test/key", None);

        let payload: TtlExpiredPayload = serde_json::from_value(event.payload).unwrap();
        assert_eq!(payload.key, "test/key");
        assert!(payload.ttl_set_at.is_none());
    }

    #[test]
    fn test_ttl_expired_event_empty_key() {
        let event = create_ttl_expired_event(1, "", Some(5000));

        let payload: TtlExpiredPayload = serde_json::from_value(event.payload).unwrap();
        assert_eq!(payload.key, "");
    }

    #[test]
    fn test_ttl_expired_event_special_characters_in_key() {
        let event = create_ttl_expired_event(42, "path/to/key:with:colons", Some(12345));

        assert_eq!(event.node_id, 42);
        let payload: TtlExpiredPayload = serde_json::from_value(event.payload).unwrap();
        assert_eq!(payload.key, "path/to/key:with:colons");
    }

    #[test]
    fn test_ttl_expired_event_unicode_key() {
        let event = create_ttl_expired_event(1, "キー/テスト/日本語", Some(999));

        let payload: TtlExpiredPayload = serde_json::from_value(event.payload).unwrap();
        assert_eq!(payload.key, "キー/テスト/日本語");
    }

    #[test]
    fn test_ttl_expired_event_large_timestamp() {
        // Test with large timestamp value near u64::MAX
        let large_ts = u64::MAX / 2;
        let event = create_ttl_expired_event(1, "test/key", Some(large_ts));

        let payload: TtlExpiredPayload = serde_json::from_value(event.payload).unwrap();
        assert!(payload.ttl_set_at.is_some());
    }

    #[test]
    fn test_ttl_expired_event_zero_timestamp() {
        let event = create_ttl_expired_event(1, "test/key", Some(0));

        let payload: TtlExpiredPayload = serde_json::from_value(event.payload).unwrap();
        assert!(payload.ttl_set_at.is_some());
    }

    #[test]
    fn test_serialize_payload_success() {
        let payload = TtlExpiredPayload {
            key: "test/key".to_string(),
            ttl_set_at: None,
        };

        let result = serialize_payload(payload, "TtlExpired");
        assert!(result.is_object());
        assert_eq!(result["key"], "test/key");
    }

    #[test]
    fn test_serialize_payload_with_timestamp() {
        let payload = TtlExpiredPayload {
            key: "another/key".to_string(),
            ttl_set_at: Some(aspen_core::SerializableTimestamp::from_millis(5000)),
        };

        let result = serialize_payload(payload, "TtlExpired");
        assert!(result.is_object());
        assert_eq!(result["key"], "another/key");
        assert!(result.get("ttl_set_at").is_some());
    }

    #[test]
    fn test_ttl_cleanup_config_defaults() {
        let config = TtlCleanupConfig::default();
        // Verify we have sensible defaults
        assert!(config.batch_size_keys > 0);
        assert!(config.max_batches_per_run > 0);
        assert!(config.cleanup_interval.as_secs() > 0);
    }

    #[test]
    fn test_ttl_expired_event_node_id_propagation() {
        // Verify node_id is correctly set in the event
        for node_id in [0u64, 1, 42, u64::MAX] {
            let event = create_ttl_expired_event(node_id, "key", None);
            assert_eq!(event.node_id, node_id);
        }
    }

    #[test]
    fn test_ttl_expired_event_type() {
        let event = create_ttl_expired_event(1, "test", None);
        assert_eq!(event.event_type, HookEventType::TtlExpired);
    }
}
