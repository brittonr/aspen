//! Bridge between Raft log broadcast and HookService.
//!
//! Converts LogEntryPayload events from the Raft state machine
//! into HookEvents and dispatches them to registered handlers.
//!
//! # Architecture
//!
//! The event bridge subscribes to the log broadcast channel (same channel used by
//! DocsExporter) and converts each committed entry into one or more HookEvents.
//! Events are dispatched asynchronously via tokio::spawn to ensure hook execution
//! never blocks Raft consensus.
//!
//! # Event Mapping
//!
//! | KvOperation | HookEventType |
//! |-------------|---------------|
//! | Set | WriteCommitted |
//! | SetWithTTL | WriteCommitted |
//! | SetWithLease | WriteCommitted |
//! | SetMulti | WriteCommitted (per key) |
//! | SetMultiWithTTL | WriteCommitted (per key) |
//! | SetMultiWithLease | WriteCommitted (per key) |
//! | Delete | DeleteCommitted |
//! | DeleteMulti | DeleteCommitted (per key) |
//! | MembershipChange | MembershipChanged |
//!
//! # Tiger Style
//!
//! - Bounded channel buffer (LOG_BROADCAST_BUFFER_SIZE = 1000)
//! - Lagged events are dropped with warning (backpressure)
//! - Non-blocking dispatch via tokio::spawn

use std::sync::Arc;

use aspen_hooks::HookEvent;
use aspen_hooks::HookEventType;
use aspen_hooks::HookService;
use aspen_hooks::KvDeletePayload;
use aspen_hooks::KvWritePayload;
use aspen_hooks::MembershipChangedPayload;
use aspen_transport::log_subscriber::KvOperation;
use aspen_transport::log_subscriber::LogEntryPayload;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Maximum number of in-flight dispatch tasks.
///
/// Tiger Style: Bounded to prevent unbounded task spawning under load.
/// When at capacity, completed tasks are drained before spawning new ones.
const MAX_IN_FLIGHT_DISPATCHES: usize = 100;

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

/// Run the event bridge that converts Raft log entries to hook events.
///
/// Subscribes to the log broadcast channel and dispatches matching
/// events to the HookService. Runs until cancellation.
///
/// # Non-blocking Guarantee
///
/// Each event dispatch is spawned as a separate task to ensure the bridge
/// never blocks on slow handlers. This matches the FoundationDB/CockroachDB
/// pattern where notifications are asynchronous.
///
/// # Task Tracking (Tiger Style)
///
/// Uses `JoinSet` to track spawned dispatch tasks. On shutdown, waits for
/// in-flight dispatches to complete (up to timeout). Bounded to prevent
/// unbounded task spawning under high load.
pub async fn run_event_bridge(
    mut receiver: broadcast::Receiver<LogEntryPayload>,
    service: Arc<HookService>,
    node_id: u64,
    cancel: CancellationToken,
) {
    debug!(node_id, "event bridge started");

    // Tiger Style: Track spawned tasks with JoinSet for graceful shutdown
    let mut dispatch_tasks: JoinSet<()> = JoinSet::new();

    loop {
        // Drain completed tasks before processing more events
        while dispatch_tasks.try_join_next().is_some() {}

        tokio::select! {
            _ = cancel.cancelled() => {
                debug!("event bridge shutting down");
                break;
            }
            result = receiver.recv() => {
                match result {
                    Ok(payload) => {
                        let events = convert_to_hook_events(&payload, node_id);
                        for event in events {
                            // Tiger Style: Apply backpressure if too many in-flight tasks
                            if dispatch_tasks.len() >= MAX_IN_FLIGHT_DISPATCHES {
                                warn!(
                                    in_flight = dispatch_tasks.len(),
                                    max = MAX_IN_FLIGHT_DISPATCHES,
                                    "dispatch tasks at capacity, waiting for completion"
                                );
                                // Wait for at least one to complete
                                let _ = dispatch_tasks.join_next().await;
                            }

                            let service_clone = Arc::clone(&service);
                            // Spawn dispatch as separate task, tracked by JoinSet
                            dispatch_tasks.spawn(async move {
                                if let Err(e) = service_clone.dispatch(&event).await {
                                    warn!(error = ?e, event_type = ?event.event_type, "failed to dispatch hook event");
                                }
                            });
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "event bridge lagged, events dropped");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("log broadcast channel closed");
                        break;
                    }
                }
            }
        }
    }

    // Tiger Style: Wait for in-flight dispatches to complete on shutdown
    let in_flight = dispatch_tasks.len();
    if in_flight > 0 {
        info!(in_flight, "waiting for in-flight hook dispatches to complete");
        while dispatch_tasks.join_next().await.is_some() {}
        debug!("all in-flight hook dispatches completed");
    }

    debug!("event bridge stopped");
}

/// Convert LogEntryPayload to HookEvents.
///
/// Returns Vec because batch operations produce multiple events (one per key).
/// This matches the user's decision to emit one event per key for SetMulti/DeleteMulti.
fn convert_to_hook_events(payload: &LogEntryPayload, node_id: u64) -> Vec<HookEvent> {
    match &payload.operation {
        // Single key operations
        KvOperation::Set { key, value } => {
            vec![create_write_event(payload, node_id, key, value.len() as u64, false)]
        }
        KvOperation::SetWithTTL { key, value, .. } => {
            vec![create_write_event(payload, node_id, key, value.len() as u64, false)]
        }
        KvOperation::SetWithLease { key, value, .. } => {
            vec![create_write_event(payload, node_id, key, value.len() as u64, false)]
        }
        KvOperation::Delete { key } => {
            vec![create_delete_event(payload, node_id, key, true)]
        }
        KvOperation::CompareAndSwap { key, new_value, .. } => {
            // CAS succeeded, treat as write
            vec![create_write_event(payload, node_id, key, new_value.len() as u64, false)]
        }
        KvOperation::CompareAndDelete { key, .. } => {
            // CAD succeeded, treat as delete
            vec![create_delete_event(payload, node_id, key, true)]
        }

        // Multi-key operations - emit one event per key
        KvOperation::SetMulti { pairs } => pairs
            .iter()
            .map(|(key, value)| create_write_event(payload, node_id, key, value.len() as u64, false))
            .collect(),
        KvOperation::SetMultiWithTTL { pairs, .. } => pairs
            .iter()
            .map(|(key, value)| create_write_event(payload, node_id, key, value.len() as u64, false))
            .collect(),
        KvOperation::SetMultiWithLease { pairs, .. } => pairs
            .iter()
            .map(|(key, value)| create_write_event(payload, node_id, key, value.len() as u64, false))
            .collect(),
        KvOperation::DeleteMulti { keys } => {
            keys.iter().map(|key| create_delete_event(payload, node_id, key, true)).collect()
        }

        // Batch operations - emit per operation
        KvOperation::Batch { operations } => operations
            .iter()
            .map(|(is_set, key, value)| {
                if *is_set {
                    create_write_event(payload, node_id, key, value.len() as u64, false)
                } else {
                    create_delete_event(payload, node_id, key, true)
                }
            })
            .collect(),
        KvOperation::ConditionalBatch { operations, .. } => operations
            .iter()
            .map(|(is_set, key, value)| {
                if *is_set {
                    create_write_event(payload, node_id, key, value.len() as u64, false)
                } else {
                    create_delete_event(payload, node_id, key, true)
                }
            })
            .collect(),

        // Transaction operations - emit per operation in write set
        KvOperation::Transaction { success, .. } => {
            // Success branch was executed (since this is committed)
            success
                .iter()
                .filter_map(|(op_type, key, value)| {
                    match op_type {
                        0 => Some(create_write_event(payload, node_id, key, value.len() as u64, false)), // PUT
                        1 => Some(create_delete_event(payload, node_id, key, true)),                     // DELETE
                        _ => None,
                    }
                })
                .collect()
        }
        KvOperation::OptimisticTransaction { write_set, .. } => write_set
            .iter()
            .map(|(is_set, key, value)| {
                if *is_set {
                    create_write_event(payload, node_id, key, value.len() as u64, false)
                } else {
                    create_delete_event(payload, node_id, key, true)
                }
            })
            .collect(),

        // Cluster events
        KvOperation::MembershipChange { description } => {
            vec![create_membership_event(payload, node_id, description)]
        }

        // Lease operations - no hooks for now
        KvOperation::LeaseGrant { .. } | KvOperation::LeaseRevoke { .. } | KvOperation::LeaseKeepalive { .. } => {
            // Lease operations don't trigger hooks in current design
            // Could be added as system events in the future
            vec![]
        }

        // No-op entries
        KvOperation::Noop => vec![],
    }
}

/// Create a WriteCommitted hook event.
fn create_write_event(
    payload: &LogEntryPayload,
    node_id: u64,
    key: &[u8],
    value_size_bytes: u64,
    is_create: bool,
) -> HookEvent {
    let kv_payload = KvWritePayload {
        key: String::from_utf8_lossy(key).to_string(),
        value: None, // Don't include value in hook event to avoid large payloads
        value_size_bytes,
        is_create,
    };

    HookEvent::new(HookEventType::WriteCommitted, node_id, serialize_payload(kv_payload, "WriteCommitted"))
        .with_log_index(payload.index)
        .with_timestamp(payload.hlc_timestamp.clone())
}

/// Create a DeleteCommitted hook event.
fn create_delete_event(payload: &LogEntryPayload, node_id: u64, key: &[u8], existed: bool) -> HookEvent {
    let kv_payload = KvDeletePayload {
        key: String::from_utf8_lossy(key).to_string(),
        existed,
    };

    HookEvent::new(HookEventType::DeleteCommitted, node_id, serialize_payload(kv_payload, "DeleteCommitted"))
        .with_log_index(payload.index)
        .with_timestamp(payload.hlc_timestamp.clone())
}

/// Create a MembershipChanged hook event.
fn create_membership_event(payload: &LogEntryPayload, node_id: u64, description: &str) -> HookEvent {
    // Parse change type from description (e.g., "Adding node 2 as learner")
    let change_type = if description.contains("learner") {
        "added_learner"
    } else if description.contains("removed") || description.contains("Removing") {
        "removed"
    } else if description.contains("promoted") || description.contains("Promoting") {
        "promoted"
    } else if description.contains("voter") || description.contains("Adding") {
        "added_voter"
    } else {
        "unknown"
    };

    let membership_payload = MembershipChangedPayload {
        voters: vec![],      // Would need to parse from Raft metrics
        learners: vec![],    // Would need to parse from Raft metrics
        affected_node: None, // Could parse from description
        change_type: change_type.to_string(),
    };

    HookEvent::new(
        HookEventType::MembershipChanged,
        node_id,
        serialize_payload(membership_payload, "MembershipChanged"),
    )
    .with_log_index(payload.index)
    .with_timestamp(payload.hlc_timestamp.clone())
}

#[cfg(test)]
mod tests {
    use aspen_core::SerializableTimestamp;

    use super::*;

    fn make_payload(operation: KvOperation) -> LogEntryPayload {
        // Create HLC instance for testing and generate a timestamp
        let hlc = aspen_core::hlc::create_hlc("test-node");
        let timestamp = SerializableTimestamp::from(hlc.new_timestamp());
        LogEntryPayload {
            index: 42,
            term: 1,
            hlc_timestamp: timestamp,
            operation,
        }
    }

    #[test]
    fn test_convert_set_operation() {
        let payload = make_payload(KvOperation::Set {
            key: b"test/key".to_vec(),
            value: b"value".to_vec(),
        });

        let events = convert_to_hook_events(&payload, 1);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, HookEventType::WriteCommitted);
        assert_eq!(events[0].log_index, Some(42));
        assert_eq!(events[0].node_id, 1);
    }

    #[test]
    fn test_convert_delete_operation() {
        let payload = make_payload(KvOperation::Delete {
            key: b"test/key".to_vec(),
        });

        let events = convert_to_hook_events(&payload, 1);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, HookEventType::DeleteCommitted);
    }

    #[test]
    fn test_convert_set_multi_emits_per_key() {
        let payload = make_payload(KvOperation::SetMulti {
            pairs: vec![
                (b"key1".to_vec(), b"value1".to_vec()),
                (b"key2".to_vec(), b"value2".to_vec()),
                (b"key3".to_vec(), b"value3".to_vec()),
            ],
        });

        let events = convert_to_hook_events(&payload, 1);

        // One event per key
        assert_eq!(events.len(), 3);
        for event in &events {
            assert_eq!(event.event_type, HookEventType::WriteCommitted);
        }
    }

    #[test]
    fn test_convert_delete_multi_emits_per_key() {
        let payload = make_payload(KvOperation::DeleteMulti {
            keys: vec![b"key1".to_vec(), b"key2".to_vec()],
        });

        let events = convert_to_hook_events(&payload, 1);

        assert_eq!(events.len(), 2);
        for event in &events {
            assert_eq!(event.event_type, HookEventType::DeleteCommitted);
        }
    }

    #[test]
    fn test_convert_membership_change() {
        let payload = make_payload(KvOperation::MembershipChange {
            description: "Adding node 2 as learner".to_string(),
        });

        let events = convert_to_hook_events(&payload, 1);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, HookEventType::MembershipChanged);
    }

    #[test]
    fn test_convert_noop_returns_empty() {
        let payload = make_payload(KvOperation::Noop);

        let events = convert_to_hook_events(&payload, 1);

        assert!(events.is_empty());
    }

    #[test]
    fn test_convert_lease_operations_return_empty() {
        let lease_grant = make_payload(KvOperation::LeaseGrant {
            lease_id: 1,
            ttl_seconds: 60,
        });
        let lease_revoke = make_payload(KvOperation::LeaseRevoke { lease_id: 1 });
        let lease_keepalive = make_payload(KvOperation::LeaseKeepalive { lease_id: 1 });

        assert!(convert_to_hook_events(&lease_grant, 1).is_empty());
        assert!(convert_to_hook_events(&lease_revoke, 1).is_empty());
        assert!(convert_to_hook_events(&lease_keepalive, 1).is_empty());
    }

    #[test]
    fn test_convert_batch_operation() {
        let payload = make_payload(KvOperation::Batch {
            operations: vec![
                (true, b"set_key".to_vec(), b"value".to_vec()),
                (false, b"delete_key".to_vec(), vec![]),
            ],
        });

        let events = convert_to_hook_events(&payload, 1);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, HookEventType::WriteCommitted);
        assert_eq!(events[1].event_type, HookEventType::DeleteCommitted);
    }
}
