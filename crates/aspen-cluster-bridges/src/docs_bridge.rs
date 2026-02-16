//! Bridge between docs events and HookService.
//!
//! Converts DocsEvent from the docs system into HookEvents and dispatches
//! them to registered handlers. This enables external programs to react
//! to docs lifecycle events via the hook system.
//!
//! # Architecture
//!
//! ```text
//! PeerManager / DocsExporter operations
//!        |
//!        v
//! DocsEventBroadcaster
//!        |
//!        v
//! broadcast::Receiver<DocsEvent>
//!        |
//!        v
//! run_docs_bridge (this module)
//!        |
//!        v
//! HookService.dispatch()
//! ```
//!
//! # Event Mapping
//!
//! | DocsEventType | HookEventType |
//! |---------------|---------------|
//! | SyncStarted | DocsSyncStarted |
//! | SyncCompleted | DocsSyncCompleted |
//! | EntryImported | DocsEntryImported |
//! | EntryExported | DocsEntryExported |
//!
//! # Tiger Style
//!
//! - Bounded channel buffer (DOCS_EVENT_BUFFER_SIZE = 500)
//! - Lagged events are dropped with warning (backpressure)
//! - Non-blocking dispatch via tokio::spawn

use std::sync::Arc;

use aspen_constants::coordination::MAX_DOCS_EVENT_DISPATCHES;
use aspen_docs::DocsEvent;
use aspen_docs::DocsEventType;
use aspen_docs::ImportResultType;
use aspen_hooks::DocsEntryExportedPayload;
use aspen_hooks::DocsEntryImportedPayload;
use aspen_hooks::DocsSyncCompletedPayload;
use aspen_hooks::DocsSyncStartedPayload;
use aspen_hooks::HookEvent;
use aspen_hooks::HookEventType;
use aspen_hooks::HookService;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Run the docs event bridge that converts docs events to hook events.
///
/// Subscribes to the docs event broadcast channel and dispatches matching
/// events to the HookService. Runs until cancellation.
///
/// # Non-blocking Guarantee
///
/// Each event dispatch is spawned as a separate task to ensure the bridge
/// never blocks on slow handlers.
///
/// # Task Tracking (Tiger Style)
///
/// Uses `JoinSet` to track spawned dispatch tasks. On shutdown, waits for
/// in-flight dispatches to complete. Bounded to prevent unbounded task
/// spawning under high docs sync throughput.
pub async fn run_docs_bridge(
    mut receiver: broadcast::Receiver<DocsEvent>,
    service: Arc<HookService>,
    node_id: u64,
    cancel: CancellationToken,
) {
    debug!(node_id, "docs event bridge started");

    // Tiger Style: Track spawned tasks with JoinSet for graceful shutdown
    let mut dispatch_tasks: JoinSet<()> = JoinSet::new();

    loop {
        // Drain completed tasks before processing more events
        while dispatch_tasks.try_join_next().is_some() {}

        tokio::select! {
            _ = cancel.cancelled() => {
                debug!("docs event bridge shutting down");
                break;
            }
            result = receiver.recv() => {
                match result {
                    Ok(docs_event) => {
                        let hook_event = convert_to_hook_event(&docs_event, node_id);
                        let service_clone = Arc::clone(&service);

                        // Tiger Style: Apply backpressure if too many in-flight tasks
                        if dispatch_tasks.len() >= MAX_DOCS_EVENT_DISPATCHES {
                            warn!(
                                in_flight = dispatch_tasks.len(),
                                max = MAX_DOCS_EVENT_DISPATCHES,
                                "docs event dispatch tasks at capacity, waiting for completion"
                            );
                            // Wait for at least one to complete
                            let _ = dispatch_tasks.join_next().await;
                        }

                        // Spawn dispatch as separate task, tracked by JoinSet
                        dispatch_tasks.spawn(async move {
                            if let Err(e) = service_clone.dispatch(&hook_event).await {
                                warn!(error = ?e, event_type = ?hook_event.event_type, "failed to dispatch docs hook event");
                            }
                        });
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "docs event bridge lagged, events dropped");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("docs event broadcast channel closed");
                        break;
                    }
                }
            }
        }
    }

    // Tiger Style: Wait for in-flight dispatches to complete on shutdown
    let in_flight = dispatch_tasks.len();
    if in_flight > 0 {
        info!(in_flight, "waiting for in-flight docs event dispatches to complete");
        while dispatch_tasks.join_next().await.is_some() {}
        debug!("all in-flight docs event dispatches completed");
    }

    debug!("docs event bridge stopped");
}

/// Convert a DocsEvent to a HookEvent.
fn convert_to_hook_event(docs_event: &DocsEvent, node_id: u64) -> HookEvent {
    match docs_event.event_type {
        DocsEventType::SyncStarted => create_sync_started_event(docs_event, node_id),
        DocsEventType::SyncCompleted => create_sync_completed_event(docs_event, node_id),
        DocsEventType::EntryImported => create_entry_imported_event(docs_event, node_id),
        DocsEventType::EntryExported => create_entry_exported_event(docs_event, node_id),
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

/// Create a DocsSyncStarted hook event.
fn create_sync_started_event(docs_event: &DocsEvent, node_id: u64) -> HookEvent {
    let payload = DocsSyncStartedPayload {
        cluster_id: docs_event.cluster_id.clone(),
        peer_count: docs_event.peer_count.unwrap_or(0),
    };

    HookEvent::new(HookEventType::DocsSyncStarted, node_id, serialize_payload(payload, "DocsSyncStarted"))
}

/// Create a DocsSyncCompleted hook event.
fn create_sync_completed_event(docs_event: &DocsEvent, node_id: u64) -> HookEvent {
    let payload = DocsSyncCompletedPayload {
        cluster_id: docs_event.cluster_id.clone(),
        entries_synced: docs_event.entry_count.unwrap_or(0),
        duration_ms: docs_event.duration_ms.unwrap_or(0),
        is_success: true, // Bridge only emits successful completions
        error: None,
    };

    HookEvent::new(HookEventType::DocsSyncCompleted, node_id, serialize_payload(payload, "DocsSyncCompleted"))
}

/// Create a DocsEntryImported hook event.
fn create_entry_imported_event(docs_event: &DocsEvent, node_id: u64) -> HookEvent {
    let key = docs_event.key.as_ref().map(|k| String::from_utf8_lossy(k).to_string()).unwrap_or_default();

    // Determine if this is an update based on the import result
    let is_update = docs_event.import_result.as_ref().map(|r| matches!(r, ImportResultType::Imported)).unwrap_or(false);

    let payload = DocsEntryImportedPayload {
        key,
        source_cluster: docs_event.cluster_id.clone(),
        priority: docs_event.priority.unwrap_or(0),
        value_size: docs_event.value_size.unwrap_or(0),
        is_update,
    };

    HookEvent::new(HookEventType::DocsEntryImported, node_id, serialize_payload(payload, "DocsEntryImported"))
}

/// Create a DocsEntryExported hook event.
fn create_entry_exported_event(docs_event: &DocsEvent, node_id: u64) -> HookEvent {
    let payload = DocsEntryExportedPayload {
        batch_size: docs_event.batch_size.unwrap_or(0),
        total_exported: docs_event.entry_count.unwrap_or(0),
        duration_ms: docs_event.duration_ms.unwrap_or(0),
    };

    HookEvent::new(HookEventType::DocsEntryExported, node_id, serialize_payload(payload, "DocsEntryExported"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_sync_started() {
        let docs_event = DocsEvent::sync_started("cluster-a", 3);

        let hook_event = convert_to_hook_event(&docs_event, 1);

        assert_eq!(hook_event.event_type, HookEventType::DocsSyncStarted);
        assert_eq!(hook_event.node_id, 1);

        let payload: DocsSyncStartedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert_eq!(payload.cluster_id, "cluster-a");
        assert_eq!(payload.peer_count, 3);
    }

    #[test]
    fn test_convert_sync_completed() {
        let docs_event = DocsEvent::sync_completed("cluster-b", 100, 5000);

        let hook_event = convert_to_hook_event(&docs_event, 1);

        assert_eq!(hook_event.event_type, HookEventType::DocsSyncCompleted);

        let payload: DocsSyncCompletedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert_eq!(payload.cluster_id, "cluster-b");
        assert_eq!(payload.entries_synced, 100);
        assert_eq!(payload.duration_ms, 5000);
        assert!(payload.success);
        assert!(payload.error.is_none());
    }

    #[test]
    fn test_convert_entry_imported() {
        let docs_event =
            DocsEvent::entry_imported("cluster-c", b"test/key".to_vec(), 1024, 5, ImportResultType::Imported);

        let hook_event = convert_to_hook_event(&docs_event, 1);

        assert_eq!(hook_event.event_type, HookEventType::DocsEntryImported);

        let payload: DocsEntryImportedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert_eq!(payload.key, "test/key");
        assert_eq!(payload.source_cluster, "cluster-c");
        assert_eq!(payload.priority, 5);
        assert_eq!(payload.value_size, 1024);
        assert!(payload.is_update); // Imported entries are updates
    }

    #[test]
    fn test_convert_entry_imported_priority_skipped() {
        let docs_event =
            DocsEvent::entry_imported("cluster-d", b"other/key".to_vec(), 512, 2, ImportResultType::PrioritySkipped);

        let hook_event = convert_to_hook_event(&docs_event, 1);

        let payload: DocsEntryImportedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert!(!payload.is_update); // Skipped entries are not updates
    }

    #[test]
    fn test_convert_entry_exported() {
        let docs_event = DocsEvent::entry_exported(50, 45);

        let hook_event = convert_to_hook_event(&docs_event, 1);

        assert_eq!(hook_event.event_type, HookEventType::DocsEntryExported);

        let payload: DocsEntryExportedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert_eq!(payload.batch_size, 50);
        assert_eq!(payload.total_exported, 45);
    }
}
