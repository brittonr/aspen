//! Bridge between blob events and HookService.
//!
//! Converts BlobEvent from the blob store into HookEvents and dispatches
//! them to registered handlers. This enables external programs to react
//! to blob lifecycle events via the hook system.
//!
//! # Architecture
//!
//! ```text
//! IrohBlobStore operations
//!        |
//!        v
//! BlobEventBroadcaster
//!        |
//!        v
//! broadcast::Receiver<BlobEvent>
//!        |
//!        v
//! run_blob_bridge (this module)
//!        |
//!        v
//! HookService.dispatch()
//! ```
//!
//! # Event Mapping
//!
//! | BlobEventType | HookEventType |
//! |---------------|---------------|
//! | Added | BlobAdded |
//! | Downloaded | BlobDownloaded |
//! | Protected | BlobProtected |
//! | Unprotected | BlobUnprotected |
//!
//! # Blob Content Delivery
//!
//! For blob events that include content (Added, Downloaded), the content
//! is included inline if under the threshold (64KB), otherwise a ticket
//! is generated for the handler to fetch the blob.
//!
//! # Tiger Style
//!
//! - Bounded channel buffer (BLOB_EVENT_BUFFER_SIZE = 500)
//! - Lagged events are dropped with warning (backpressure)
//! - Non-blocking dispatch via tokio::spawn

use std::sync::Arc;

use aspen_blob::BlobEvent;
use aspen_blob::BlobEventType;
use aspen_blob::BlobSource;
use aspen_blob::INLINE_BLOB_THRESHOLD;
use aspen_hooks::BlobAddedPayload;
use aspen_hooks::BlobDownloadedPayload;
use aspen_hooks::BlobProtectedPayload;
use aspen_hooks::BlobUnprotectedPayload;
use aspen_hooks::HookEvent;
use aspen_hooks::HookEventType;
use aspen_hooks::HookService;
use base64::Engine;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::warn;

/// Grace period for blob GC in seconds (from aspen-blob constants).
const BLOB_GC_GRACE_PERIOD_SECS: u64 = 300;

/// Run the blob event bridge that converts blob events to hook events.
///
/// Subscribes to the blob event broadcast channel and dispatches matching
/// events to the HookService. Runs until cancellation.
///
/// # Non-blocking Guarantee
///
/// Each event dispatch is spawned as a separate task to ensure the bridge
/// never blocks on slow handlers.
pub async fn run_blob_bridge(
    mut receiver: broadcast::Receiver<BlobEvent>,
    service: Arc<HookService>,
    node_id: u64,
    cancel: CancellationToken,
) {
    debug!(node_id, "blob event bridge started");

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                debug!("blob event bridge shutting down");
                break;
            }
            result = receiver.recv() => {
                match result {
                    Ok(blob_event) => {
                        let hook_event = convert_to_hook_event(&blob_event, node_id);
                        let service_clone = Arc::clone(&service);

                        // Spawn dispatch as separate task to never block the bridge
                        tokio::spawn(async move {
                            if let Err(e) = service_clone.dispatch(&hook_event).await {
                                warn!(error = ?e, event_type = ?hook_event.event_type, "failed to dispatch blob hook event");
                            }
                        });
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "blob event bridge lagged, events dropped");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("blob event broadcast channel closed");
                        break;
                    }
                }
            }
        }
    }

    debug!("blob event bridge stopped");
}

/// Convert a BlobEvent to a HookEvent.
fn convert_to_hook_event(blob_event: &BlobEvent, node_id: u64) -> HookEvent {
    match blob_event.event_type {
        BlobEventType::Added => create_blob_added_event(blob_event, node_id),
        BlobEventType::Downloaded => create_blob_downloaded_event(blob_event, node_id),
        BlobEventType::Protected => create_blob_protected_event(blob_event, node_id),
        BlobEventType::Unprotected => create_blob_unprotected_event(blob_event, node_id),
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

/// Create a BlobAdded hook event.
fn create_blob_added_event(blob_event: &BlobEvent, node_id: u64) -> HookEvent {
    let content_base64 = blob_event.content.as_ref().map(|c| base64::engine::general_purpose::STANDARD.encode(c));

    // Generate ticket if content is too large (no inline content)
    // In production, this would call blob_store.ticket() but we don't have access here
    // The caller should provide the ticket in the BlobEvent if needed
    let blob_ticket = if blob_event.content.is_none() && blob_event.size_bytes > INLINE_BLOB_THRESHOLD as u64 {
        // Placeholder - in production, the BlobEventBroadcaster would generate this
        Some(format!("blobticket:{}", blob_event.hash.to_hex()))
    } else {
        None
    };

    let source = blob_event.source.map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string());

    let payload = BlobAddedPayload {
        hash: blob_event.hash.to_hex().to_string(),
        size_bytes: blob_event.size_bytes,
        source,
        content_base64,
        blob_ticket,
    };

    HookEvent::new(HookEventType::BlobAdded, node_id, serialize_payload(payload, "BlobAdded"))
}

/// Create a BlobDownloaded hook event.
fn create_blob_downloaded_event(blob_event: &BlobEvent, node_id: u64) -> HookEvent {
    let content_base64 = blob_event.content.as_ref().map(|c| base64::engine::general_purpose::STANDARD.encode(c));

    let blob_ticket = if blob_event.content.is_none() && blob_event.size_bytes > INLINE_BLOB_THRESHOLD as u64 {
        Some(format!("blobticket:{}", blob_event.hash.to_hex()))
    } else {
        None
    };

    let payload = BlobDownloadedPayload {
        hash: blob_event.hash.to_hex().to_string(),
        size_bytes: blob_event.size_bytes,
        provider_id: blob_event.provider_id.clone().unwrap_or_else(|| "unknown".to_string()),
        duration_ms: blob_event.duration_ms,
        content_base64,
        blob_ticket,
    };

    HookEvent::new(HookEventType::BlobDownloaded, node_id, serialize_payload(payload, "BlobDownloaded"))
}

/// Create a BlobProtected hook event.
fn create_blob_protected_event(blob_event: &BlobEvent, node_id: u64) -> HookEvent {
    let payload = BlobProtectedPayload {
        hash: blob_event.hash.to_hex().to_string(),
        tag_name: blob_event.tag_name.clone().unwrap_or_default(),
    };

    HookEvent::new(HookEventType::BlobProtected, node_id, serialize_payload(payload, "BlobProtected"))
}

/// Create a BlobUnprotected hook event.
fn create_blob_unprotected_event(blob_event: &BlobEvent, node_id: u64) -> HookEvent {
    let hash = if blob_event.hash == iroh_blobs::Hash::from_bytes([0u8; 32]) {
        None
    } else {
        Some(blob_event.hash.to_hex().to_string())
    };

    let reason = blob_event.unprotect_reason.as_ref().map(|r| r.to_string()).unwrap_or_else(|| "unknown".to_string());

    let payload = BlobUnprotectedPayload {
        hash,
        tag_name: blob_event.tag_name.clone().unwrap_or_default(),
        reason,
        grace_period_secs: BLOB_GC_GRACE_PERIOD_SECS,
    };

    HookEvent::new(HookEventType::BlobUnprotected, node_id, serialize_payload(payload, "BlobUnprotected"))
}

#[cfg(test)]
mod tests {
    use aspen_blob::UnprotectReason;
    use iroh_blobs::Hash;

    use super::*;

    #[test]
    fn test_convert_blob_added_with_inline_content() {
        let hash = Hash::from_bytes([1u8; 32]);
        let content = vec![1, 2, 3, 4, 5];
        let blob_event = BlobEvent::added(hash, 5, BlobSource::AddBytes, Some(content.clone()), true);

        let hook_event = convert_to_hook_event(&blob_event, 1);

        assert_eq!(hook_event.event_type, HookEventType::BlobAdded);
        assert_eq!(hook_event.node_id, 1);

        let payload: BlobAddedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert_eq!(payload.size_bytes, 5);
        assert_eq!(payload.source, "add_bytes");
        assert!(payload.content_base64.is_some());
        assert!(payload.blob_ticket.is_none());
    }

    #[test]
    fn test_convert_blob_added_with_ticket() {
        let hash = Hash::from_bytes([2u8; 32]);
        // Large blob - no inline content
        let blob_event = BlobEvent::added(hash, 100_000, BlobSource::AddPath, None, true);

        let hook_event = convert_to_hook_event(&blob_event, 1);

        let payload: BlobAddedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert!(payload.content_base64.is_none());
        assert!(payload.blob_ticket.is_some());
    }

    #[test]
    fn test_convert_blob_downloaded() {
        let hash = Hash::from_bytes([3u8; 32]);
        let blob_event = BlobEvent::downloaded(hash, 2048, "peer123".to_string(), 150, Some(vec![1, 2, 3]));

        let hook_event = convert_to_hook_event(&blob_event, 1);

        assert_eq!(hook_event.event_type, HookEventType::BlobDownloaded);

        let payload: BlobDownloadedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert_eq!(payload.provider_id, "peer123");
        assert_eq!(payload.duration_ms, Some(150));
        assert!(payload.content_base64.is_some());
    }

    #[test]
    fn test_convert_blob_protected() {
        let hash = Hash::from_bytes([4u8; 32]);
        let blob_event = BlobEvent::protected(hash, "kv:test/key".to_string());

        let hook_event = convert_to_hook_event(&blob_event, 1);

        assert_eq!(hook_event.event_type, HookEventType::BlobProtected);

        let payload: BlobProtectedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert_eq!(payload.tag_name, "kv:test/key");
    }

    #[test]
    fn test_convert_blob_unprotected() {
        let hash = Hash::from_bytes([5u8; 32]);
        let blob_event = BlobEvent::unprotected(Some(hash), "kv:test/key".to_string(), UnprotectReason::KvDelete);

        let hook_event = convert_to_hook_event(&blob_event, 1);

        assert_eq!(hook_event.event_type, HookEventType::BlobUnprotected);

        let payload: BlobUnprotectedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert_eq!(payload.tag_name, "kv:test/key");
        assert_eq!(payload.reason, "kv_delete");
        assert_eq!(payload.grace_period_secs, 300);
    }

    #[test]
    fn test_convert_blob_unprotected_unknown_hash() {
        let blob_event = BlobEvent::unprotected(None, "user:temp".to_string(), UnprotectReason::UserAction);

        let hook_event = convert_to_hook_event(&blob_event, 1);

        let payload: BlobUnprotectedPayload = serde_json::from_value(hook_event.payload).unwrap();
        assert!(payload.hash.is_none());
        assert_eq!(payload.reason, "user_action");
    }
}
