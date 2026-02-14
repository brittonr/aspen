//! Bridge for snapshot hook events (SnapshotCreated, SnapshotInstalled).
//!
//! This module subscribes to snapshot events from storage and emits hook events
//! when snapshots are created or installed.
//!
//! # Architecture
//!
//! The snapshot events bridge runs as a background task that receives snapshot
//! events from the storage broadcast channel. When events are received, it creates
//! and dispatches the appropriate hook events.
//!
//! # Tiger Style
//!
//! - Non-blocking dispatch via tokio::spawn
//! - Graceful shutdown via CancellationToken

use std::sync::Arc;

use aspen_hooks::HookEvent;
use aspen_hooks::HookEventType;
use aspen_hooks::HookService;
use aspen_hooks::SnapshotPayload;
use aspen_raft::storage_shared::SnapshotEvent;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Run the snapshot events bridge that listens for snapshot events from storage.
///
/// This task receives snapshot events from the storage broadcast channel and
/// emits hook events when:
/// - A snapshot is created (SnapshotCreated)
/// - A snapshot is installed (SnapshotInstalled)
///
/// # Non-blocking Guarantee
///
/// Each event dispatch is spawned as a separate task to ensure the bridge
/// never blocks on slow handlers.
///
/// # Task Tracking (Tiger Style)
///
/// Uses `JoinSet` to track spawned dispatch tasks. On shutdown, waits for
/// in-flight dispatches to complete.
pub async fn run_snapshot_events_bridge(
    mut receiver: broadcast::Receiver<SnapshotEvent>,
    service: Arc<HookService>,
    node_id: u64,
    cancel: CancellationToken,
) {
    info!(node_id, "snapshot events bridge started");

    // Tiger Style: Track spawned tasks with JoinSet for graceful shutdown
    let mut dispatch_tasks: JoinSet<()> = JoinSet::new();

    loop {
        // Drain completed tasks
        while dispatch_tasks.try_join_next().is_some() {}

        tokio::select! {
            _ = cancel.cancelled() => {
                debug!(node_id, "snapshot events bridge shutting down");
                break;
            }
            result = receiver.recv() => {
                match result {
                    Ok(snapshot_event) => {
                        dispatch_snapshot_event(&service, node_id, snapshot_event, &mut dispatch_tasks);
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        warn!(node_id, count, "snapshot events bridge lagged, missed events");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!(node_id, "snapshot events broadcast channel closed");
                        break;
                    }
                }
            }
        }
    }

    // Tiger Style: Wait for in-flight dispatches to complete on shutdown
    let in_flight = dispatch_tasks.len();
    if in_flight > 0 {
        info!(node_id, in_flight, "waiting for in-flight snapshot event dispatches to complete");
        while dispatch_tasks.join_next().await.is_some() {}
        debug!(node_id, "all in-flight snapshot event dispatches completed");
    }

    debug!(node_id, "snapshot events bridge stopped");
}

/// Dispatch a snapshot event to the hook service.
///
/// Tiger Style: Uses provided JoinSet to track spawned dispatch tasks.
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

fn dispatch_snapshot_event(
    service: &Arc<HookService>,
    node_id: u64,
    snapshot_event: SnapshotEvent,
    dispatch_tasks: &mut JoinSet<()>,
) {
    let (event_type, payload) = match snapshot_event {
        SnapshotEvent::Created {
            snapshot_id,
            last_log_index,
            term,
            entry_count,
            size_bytes,
        } => {
            info!(
                node_id,
                snapshot_id, last_log_index, term, entry_count, size_bytes, "emitting SnapshotCreated hook event"
            );
            (HookEventType::SnapshotCreated, SnapshotPayload {
                snapshot_index: last_log_index,
                term,
                entry_count: Some(entry_count),
                size_bytes: Some(size_bytes),
            })
        }
        SnapshotEvent::Installed {
            snapshot_id,
            last_log_index,
            term,
            entry_count,
        } => {
            info!(node_id, snapshot_id, last_log_index, term, entry_count, "emitting SnapshotInstalled hook event");
            (HookEventType::SnapshotInstalled, SnapshotPayload {
                snapshot_index: last_log_index,
                term,
                entry_count: Some(entry_count),
                size_bytes: None,
            })
        }
    };

    let event = HookEvent::new(event_type, node_id, serialize_payload(payload, &format!("{:?}", event_type)));

    let service_clone = Arc::clone(service);
    // Tiger Style: Track task in JoinSet
    dispatch_tasks.spawn(async move {
        if let Err(e) = service_clone.dispatch(&event).await {
            warn!(error = ?e, "failed to dispatch snapshot event");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_created_payload() {
        let payload = SnapshotPayload {
            snapshot_index: 100,
            term: 5,
            entry_count: Some(50),
            size_bytes: Some(1024),
        };

        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["snapshot_index"], 100);
        assert_eq!(json["term"], 5);
        assert_eq!(json["entry_count"], 50);
        assert_eq!(json["size_bytes"], 1024);
    }

    #[test]
    fn test_snapshot_installed_payload() {
        let payload = SnapshotPayload {
            snapshot_index: 200,
            term: 10,
            entry_count: Some(100),
            size_bytes: None,
        };

        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["snapshot_index"], 200);
        assert_eq!(json["term"], 10);
        assert_eq!(json["entry_count"], 100);
        // size_bytes should be absent when None
        assert!(json.get("size_bytes").is_none());
    }

    #[test]
    fn test_serialize_payload_success() {
        let payload = SnapshotPayload {
            snapshot_index: 42,
            term: 3,
            entry_count: Some(10),
            size_bytes: Some(512),
        };

        let result = serialize_payload(payload, "SnapshotCreated");
        assert!(result.is_object());
        assert_eq!(result["snapshot_index"], 42);
        assert_eq!(result["term"], 3);
    }

    #[test]
    fn test_serialize_payload_with_none_fields() {
        let payload = SnapshotPayload {
            snapshot_index: 0,
            term: 0,
            entry_count: None,
            size_bytes: None,
        };

        let result = serialize_payload(payload, "SnapshotInstalled");
        assert!(result.is_object());
        // None fields should be absent from JSON
        assert!(result.get("entry_count").is_none());
        assert!(result.get("size_bytes").is_none());
    }

    #[tokio::test]
    async fn test_run_snapshot_events_bridge_shutdown() {
        use aspen_hooks::HooksConfig;

        // Test that the bridge shuts down gracefully when cancelled
        let (_tx, rx) = broadcast::channel::<SnapshotEvent>(16);
        let config = HooksConfig::default();
        let service = Arc::new(HookService::new(config));
        let cancel = CancellationToken::new();

        // Spawn the bridge task
        let cancel_clone = cancel.clone();
        let handle = tokio::spawn(async move {
            run_snapshot_events_bridge(rx, service, 1, cancel_clone).await;
        });

        // Give it a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Cancel and verify it shuts down
        cancel.cancel();
        let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), handle).await;
        assert!(result.is_ok(), "bridge should shut down within timeout");
    }

    #[tokio::test]
    async fn test_run_snapshot_events_bridge_channel_closed() {
        use aspen_hooks::HooksConfig;

        // Test that the bridge exits when the broadcast channel is closed
        let (tx, rx) = broadcast::channel::<SnapshotEvent>(16);
        let config = HooksConfig::default();
        let service = Arc::new(HookService::new(config));
        let cancel = CancellationToken::new();

        // Spawn the bridge task
        let handle = tokio::spawn(async move {
            run_snapshot_events_bridge(rx, service, 1, cancel).await;
        });

        // Give it a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Drop the sender to close the channel
        drop(tx);

        // Verify bridge exits
        let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), handle).await;
        assert!(result.is_ok(), "bridge should exit when channel closes");
    }

    #[tokio::test]
    async fn test_run_snapshot_events_bridge_receives_created_event() {
        use aspen_hooks::HooksConfig;

        let (tx, rx) = broadcast::channel::<SnapshotEvent>(16);
        let config = HooksConfig::default();
        let service = Arc::new(HookService::new(config));
        let cancel = CancellationToken::new();

        // Spawn the bridge task
        let cancel_clone = cancel.clone();
        let handle = tokio::spawn(async move {
            run_snapshot_events_bridge(rx, service, 1, cancel_clone).await;
        });

        // Send a Created event
        let event = SnapshotEvent::Created {
            snapshot_id: "snapshot-1".to_string(),
            last_log_index: 100,
            term: 5,
            entry_count: 50,
            size_bytes: 1024,
        };
        tx.send(event).unwrap();

        // Give time for the event to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Cancel and wait for shutdown
        cancel.cancel();
        let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_snapshot_events_bridge_receives_installed_event() {
        use aspen_hooks::HooksConfig;

        let (tx, rx) = broadcast::channel::<SnapshotEvent>(16);
        let config = HooksConfig::default();
        let service = Arc::new(HookService::new(config));
        let cancel = CancellationToken::new();

        // Spawn the bridge task
        let cancel_clone = cancel.clone();
        let handle = tokio::spawn(async move {
            run_snapshot_events_bridge(rx, service, 1, cancel_clone).await;
        });

        // Send an Installed event
        let event = SnapshotEvent::Installed {
            snapshot_id: "snapshot-2".to_string(),
            last_log_index: 200,
            term: 10,
            entry_count: 100,
        };
        tx.send(event).unwrap();

        // Give time for the event to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Cancel and wait for shutdown
        cancel.cancel();
        let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_snapshot_events_bridge_handles_lag() {
        use aspen_hooks::HooksConfig;

        // Test that the bridge handles lagged messages gracefully
        let (tx, rx) = broadcast::channel::<SnapshotEvent>(1); // Small buffer to induce lag
        let config = HooksConfig::default();
        let service = Arc::new(HookService::new(config));
        let cancel = CancellationToken::new();

        // Spawn the bridge task
        let cancel_clone = cancel.clone();
        let handle = tokio::spawn(async move {
            run_snapshot_events_bridge(rx, service, 1, cancel_clone).await;
        });

        // Flood the channel to cause lag
        for i in 0..10 {
            let _ = tx.send(SnapshotEvent::Created {
                snapshot_id: format!("snapshot-{}", i),
                last_log_index: i as u64,
                term: 1,
                entry_count: 10,
                size_bytes: 100,
            });
        }

        // Give time for processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Cancel and wait for shutdown
        cancel.cancel();
        let result = tokio::time::timeout(tokio::time::Duration::from_secs(1), handle).await;
        assert!(result.is_ok(), "bridge should handle lag gracefully");
    }

    #[test]
    fn test_snapshot_payload_zero_values() {
        // Edge case: all zeros
        let payload = SnapshotPayload {
            snapshot_index: 0,
            term: 0,
            entry_count: Some(0),
            size_bytes: Some(0),
        };

        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["snapshot_index"], 0);
        assert_eq!(json["term"], 0);
        assert_eq!(json["entry_count"], 0);
        assert_eq!(json["size_bytes"], 0);
    }

    #[test]
    fn test_snapshot_payload_large_values() {
        // Edge case: large values near u64::MAX
        let payload = SnapshotPayload {
            snapshot_index: u64::MAX,
            term: u64::MAX,
            entry_count: Some(u64::MAX),
            size_bytes: Some(u64::MAX),
        };

        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["snapshot_index"], u64::MAX);
        assert_eq!(json["term"], u64::MAX);
    }
}
