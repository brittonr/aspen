//! Standalone hook types for the WASM guest.
//!
//! These mirror the types from `aspen_hooks_types` but are self-contained
//! so the plugin has zero dependency on native hooks crates.
//! The response types come from `aspen_client_api`.

use serde::Deserialize;
use serde::Serialize;

// ============================================================================
// Hook handler configuration (stored in KV)
// ============================================================================

/// Configuration for the hook system, stored in KV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    /// Whether the hook system is enabled.
    pub is_enabled: bool,
    /// List of configured hook handlers.
    pub handlers: Vec<HookHandlerConfig>,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            is_enabled: true,
            handlers: Vec::new(),
        }
    }
}

/// Configuration for a single hook handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookHandlerConfig {
    /// Unique handler name.
    pub name: String,
    /// Topic pattern this handler subscribes to.
    pub pattern: String,
    /// Handler type: "in_process", "shell", or "forward".
    pub handler_type: String,
    /// Execution mode: "direct" or "job".
    pub execution_mode: String,
    /// Whether this handler is enabled.
    pub is_enabled: bool,
    /// Timeout for handler execution in milliseconds.
    pub timeout_ms: u64,
    /// Number of retry attempts on failure.
    pub retry_count: u32,
}

impl HookHandlerConfig {
    /// Convert to the client RPC response format.
    pub fn to_info(&self) -> aspen_client_api::HookHandlerInfo {
        aspen_client_api::HookHandlerInfo {
            name: self.name.clone(),
            pattern: self.pattern.clone(),
            handler_type: self.handler_type.clone(),
            execution_mode: self.execution_mode.clone(),
            is_enabled: self.is_enabled,
            timeout_ms: self.timeout_ms,
            retry_count: self.retry_count,
        }
    }
}

// ============================================================================
// Metrics (stored in KV)
// ============================================================================

/// Aggregated metrics for all hook handlers, stored in KV.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksMetrics {
    /// Per-handler metrics keyed by handler name.
    pub handlers: Vec<HandlerMetrics>,
    /// Global counters.
    pub global_successes: u64,
    pub global_failures: u64,
}

/// Metrics for a single hook handler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HandlerMetrics {
    /// Handler name.
    pub name: String,
    /// Total successful executions.
    pub successes: u64,
    /// Total failed executions.
    pub failures: u64,
    /// Total dropped events.
    pub dropped: u64,
    /// Total jobs submitted (for job mode handlers).
    pub jobs_submitted: u64,
    /// Average execution duration in microseconds.
    pub avg_latency_us: u64,
}

impl HandlerMetrics {
    /// Convert to the client RPC response format.
    pub fn to_response(&self) -> aspen_client_api::HookHandlerMetrics {
        aspen_client_api::HookHandlerMetrics {
            name: self.name.clone(),
            success_count: self.successes,
            failure_count: self.failures,
            dropped_count: self.dropped,
            jobs_submitted: self.jobs_submitted,
            avg_duration_us: self.avg_latency_us,
            max_duration_us: 0, // Not tracked in KV metrics
        }
    }
}

// ============================================================================
// Hook events (for trigger)
// ============================================================================

/// A hook event stored in KV for dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    /// Type of the event.
    pub event_type: String,
    /// Node ID where the event originated.
    pub node_id: u64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Event-specific payload data.
    pub payload: serde_json::Value,
}

/// Event types that can be manually triggered via `HookTrigger`.
///
/// This matches the native `HooksHandler::handle_hook_trigger` match arms
/// exactly. The native handler only exposes a subset of all `HookEventType`
/// variants for manual triggering — the rest are emitted internally by
/// the Raft state machine or subsystem watchers.
pub const TRIGGERABLE_EVENT_TYPES: &[&str] = &[
    "write_committed",
    "delete_committed",
    "membership_changed",
    "leader_elected",
    "snapshot_created",
];

/// Check whether an event type string is valid for manual triggering.
pub fn is_triggerable_event_type(event_type: &str) -> bool {
    TRIGGERABLE_EVENT_TYPES.contains(&event_type)
}

/// All event types known to the hook system (superset of triggerable types).
///
/// Used for topic mapping in `event_type_to_topic`. These correspond 1:1
/// with the `HookEventType` enum in the native `aspen-hooks-types` crate.
pub const ALL_EVENT_TYPES: &[&str] = &[
    "write_committed",
    "delete_committed",
    "membership_changed",
    "leader_elected",
    "snapshot_created",
    "snapshot_installed",
    "health_changed",
    "ttl_expired",
    "node_added",
    "node_removed",
    "blob_added",
    "blob_deleted",
    "blob_downloaded",
    "blob_protected",
    "blob_unprotected",
    "docs_sync_started",
    "docs_sync_completed",
    "docs_entry_imported",
    "docs_entry_exported",
];

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // HooksConfig
    // ========================================================================

    #[test]
    fn hooks_config_default_is_enabled() {
        let config = HooksConfig::default();
        assert!(config.is_enabled);
        assert!(config.handlers.is_empty());
    }

    #[test]
    fn hooks_config_serialization_roundtrip() {
        let config = HooksConfig {
            is_enabled: true,
            handlers: vec![HookHandlerConfig {
                name: "audit-logger".to_string(),
                pattern: "hooks.>".to_string(),
                handler_type: "shell".to_string(),
                execution_mode: "job".to_string(),
                is_enabled: true,
                timeout_ms: 10_000,
                retry_count: 5,
            }],
        };
        let json = serde_json::to_vec(&config).unwrap();
        let decoded: HooksConfig = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded.is_enabled, config.is_enabled);
        assert_eq!(decoded.handlers.len(), 1);
        assert_eq!(decoded.handlers[0].name, "audit-logger");
        assert_eq!(decoded.handlers[0].pattern, "hooks.>");
        assert_eq!(decoded.handlers[0].handler_type, "shell");
        assert_eq!(decoded.handlers[0].execution_mode, "job");
        assert!(decoded.handlers[0].is_enabled);
        assert_eq!(decoded.handlers[0].timeout_ms, 10_000);
        assert_eq!(decoded.handlers[0].retry_count, 5);
    }

    #[test]
    fn hooks_config_disabled_roundtrip() {
        let config = HooksConfig {
            is_enabled: false,
            handlers: vec![],
        };
        let json = serde_json::to_vec(&config).unwrap();
        let decoded: HooksConfig = serde_json::from_slice(&json).unwrap();
        assert!(!decoded.is_enabled);
    }

    #[test]
    fn hooks_config_multiple_handlers_roundtrip() {
        let config = HooksConfig {
            is_enabled: true,
            handlers: vec![
                HookHandlerConfig {
                    name: "handler-a".to_string(),
                    pattern: "hooks.kv.*".to_string(),
                    handler_type: "in_process".to_string(),
                    execution_mode: "direct".to_string(),
                    is_enabled: true,
                    timeout_ms: 5000,
                    retry_count: 3,
                },
                HookHandlerConfig {
                    name: "handler-b".to_string(),
                    pattern: "hooks.cluster.*".to_string(),
                    handler_type: "forward".to_string(),
                    execution_mode: "job".to_string(),
                    is_enabled: false,
                    timeout_ms: 30_000,
                    retry_count: 0,
                },
            ],
        };
        let json = serde_json::to_vec(&config).unwrap();
        let decoded: HooksConfig = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded.handlers.len(), 2);
        assert_eq!(decoded.handlers[0].name, "handler-a");
        assert_eq!(decoded.handlers[1].name, "handler-b");
        assert!(decoded.handlers[0].is_enabled);
        assert!(!decoded.handlers[1].is_enabled);
    }

    // ========================================================================
    // HookHandlerConfig::to_info
    // ========================================================================

    #[test]
    fn handler_config_to_info_all_handler_types() {
        for handler_type in &["in_process", "shell", "forward"] {
            let cfg = HookHandlerConfig {
                name: "test".to_string(),
                pattern: "hooks.>".to_string(),
                handler_type: handler_type.to_string(),
                execution_mode: "direct".to_string(),
                is_enabled: true,
                timeout_ms: 5000,
                retry_count: 3,
            };
            let info = cfg.to_info();
            assert_eq!(info.handler_type, *handler_type);
        }
    }

    #[test]
    fn handler_config_to_info_all_execution_modes() {
        for mode in &["direct", "job"] {
            let cfg = HookHandlerConfig {
                name: "test".to_string(),
                pattern: "hooks.>".to_string(),
                handler_type: "in_process".to_string(),
                execution_mode: mode.to_string(),
                is_enabled: true,
                timeout_ms: 5000,
                retry_count: 3,
            };
            let info = cfg.to_info();
            assert_eq!(info.execution_mode, *mode);
        }
    }

    #[test]
    fn handler_config_to_info_preserves_zero_timeout() {
        let cfg = HookHandlerConfig {
            name: "zero-timeout".to_string(),
            pattern: "hooks.>".to_string(),
            handler_type: "in_process".to_string(),
            execution_mode: "direct".to_string(),
            is_enabled: true,
            timeout_ms: 0,
            retry_count: 0,
        };
        let info = cfg.to_info();
        assert_eq!(info.timeout_ms, 0);
        assert_eq!(info.retry_count, 0);
    }

    // ========================================================================
    // HooksMetrics
    // ========================================================================

    #[test]
    fn hooks_metrics_default_zeroed() {
        let m = HooksMetrics::default();
        assert!(m.handlers.is_empty());
        assert_eq!(m.global_successes, 0);
        assert_eq!(m.global_failures, 0);
    }

    #[test]
    fn hooks_metrics_serialization_roundtrip() {
        let metrics = HooksMetrics {
            handlers: vec![
                HandlerMetrics {
                    name: "handler-a".to_string(),
                    successes: 100,
                    failures: 5,
                    dropped: 2,
                    jobs_submitted: 50,
                    avg_latency_us: 1234,
                },
                HandlerMetrics {
                    name: "handler-b".to_string(),
                    successes: 200,
                    failures: 10,
                    dropped: 0,
                    jobs_submitted: 0,
                    avg_latency_us: 567,
                },
            ],
            global_successes: 300,
            global_failures: 15,
        };
        let json = serde_json::to_vec(&metrics).unwrap();
        let decoded: HooksMetrics = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded.handlers.len(), 2);
        assert_eq!(decoded.global_successes, 300);
        assert_eq!(decoded.global_failures, 15);
        assert_eq!(decoded.handlers[0].name, "handler-a");
        assert_eq!(decoded.handlers[0].successes, 100);
        assert_eq!(decoded.handlers[1].name, "handler-b");
        assert_eq!(decoded.handlers[1].successes, 200);
    }

    #[test]
    fn hooks_metrics_empty_handlers_roundtrip() {
        let metrics = HooksMetrics {
            handlers: vec![],
            global_successes: 42,
            global_failures: 7,
        };
        let json = serde_json::to_vec(&metrics).unwrap();
        let decoded: HooksMetrics = serde_json::from_slice(&json).unwrap();
        assert!(decoded.handlers.is_empty());
        assert_eq!(decoded.global_successes, 42);
    }

    // ========================================================================
    // HandlerMetrics
    // ========================================================================

    #[test]
    fn handler_metrics_to_response_zeroed() {
        let m = HandlerMetrics::default();
        let resp = m.to_response();
        assert_eq!(resp.name, "");
        assert_eq!(resp.success_count, 0);
        assert_eq!(resp.failure_count, 0);
        assert_eq!(resp.dropped_count, 0);
        assert_eq!(resp.jobs_submitted, 0);
        assert_eq!(resp.avg_duration_us, 0);
        assert_eq!(resp.max_duration_us, 0);
    }

    #[test]
    fn handler_metrics_to_response_large_values() {
        let m = HandlerMetrics {
            name: "high-throughput".to_string(),
            successes: u64::MAX,
            failures: u64::MAX / 2,
            dropped: 1_000_000,
            jobs_submitted: 500_000,
            avg_latency_us: 999_999,
        };
        let resp = m.to_response();
        assert_eq!(resp.success_count, u64::MAX);
        assert_eq!(resp.failure_count, u64::MAX / 2);
        assert_eq!(resp.dropped_count, 1_000_000);
    }

    #[test]
    fn handler_metrics_serialization_roundtrip() {
        let m = HandlerMetrics {
            name: "test-handler".to_string(),
            successes: 42,
            failures: 3,
            dropped: 1,
            jobs_submitted: 20,
            avg_latency_us: 789,
        };
        let json = serde_json::to_vec(&m).unwrap();
        let decoded: HandlerMetrics = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded.name, "test-handler");
        assert_eq!(decoded.successes, 42);
        assert_eq!(decoded.failures, 3);
        assert_eq!(decoded.dropped, 1);
        assert_eq!(decoded.jobs_submitted, 20);
        assert_eq!(decoded.avg_latency_us, 789);
    }

    // ========================================================================
    // HookEvent
    // ========================================================================

    #[test]
    fn hook_event_serialization_roundtrip() {
        let event = HookEvent {
            event_type: "write_committed".to_string(),
            node_id: 42,
            timestamp_ms: 1_700_000_000_000,
            payload: serde_json::json!({"key": "test/key", "value": "hello"}),
        };
        let json = serde_json::to_vec(&event).unwrap();
        let decoded: HookEvent = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded.event_type, "write_committed");
        assert_eq!(decoded.node_id, 42);
        assert_eq!(decoded.timestamp_ms, 1_700_000_000_000);
        assert_eq!(decoded.payload["key"], "test/key");
        assert_eq!(decoded.payload["value"], "hello");
    }

    #[test]
    fn hook_event_empty_payload() {
        let event = HookEvent {
            event_type: "leader_elected".to_string(),
            node_id: 1,
            timestamp_ms: 0,
            payload: serde_json::json!({}),
        };
        let json = serde_json::to_vec(&event).unwrap();
        let decoded: HookEvent = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded.payload, serde_json::json!({}));
    }

    #[test]
    fn hook_event_complex_payload() {
        let payload = serde_json::json!({
            "key": "user/123",
            "value": "data",
            "nested": { "a": 1, "b": [2, 3, 4] },
            "null_field": null,
            "bool_field": true,
        });
        let event = HookEvent {
            event_type: "write_committed".to_string(),
            node_id: 99,
            timestamp_ms: 12345,
            payload: payload.clone(),
        };
        let json = serde_json::to_vec(&event).unwrap();
        let decoded: HookEvent = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded.payload, payload);
    }

    // ========================================================================
    // Event type constants
    // ========================================================================

    #[test]
    fn all_event_types_has_19_entries() {
        assert_eq!(ALL_EVENT_TYPES.len(), 19, "expected 19 event types");
    }

    #[test]
    fn triggerable_event_types_has_5_entries() {
        assert_eq!(TRIGGERABLE_EVENT_TYPES.len(), 5, "expected 5 triggerable event types");
    }

    #[test]
    fn no_duplicate_event_types() {
        let mut seen = std::collections::HashSet::new();
        for &et in ALL_EVENT_TYPES {
            assert!(seen.insert(et), "duplicate event type: {et}");
        }
    }

    #[test]
    fn no_duplicate_triggerable_types() {
        let mut seen = std::collections::HashSet::new();
        for &et in TRIGGERABLE_EVENT_TYPES {
            assert!(seen.insert(et), "duplicate triggerable type: {et}");
        }
    }

    #[test]
    fn triggerable_is_strict_subset_of_all() {
        for &et in TRIGGERABLE_EVENT_TYPES {
            assert!(ALL_EVENT_TYPES.contains(&et), "triggerable type '{et}' not in ALL_EVENT_TYPES");
        }
        // And it's a strict subset (not equal)
        assert!(TRIGGERABLE_EVENT_TYPES.len() < ALL_EVENT_TYPES.len());
    }

    #[test]
    fn all_event_types_non_empty_strings() {
        for &et in ALL_EVENT_TYPES {
            assert!(!et.is_empty(), "event type should not be empty");
            assert!(!et.contains(' '), "event type should not contain spaces: '{et}'");
        }
    }
}
