//! Hook system request handlers.
//!
//! Each handler mirrors the behavior of the native `HooksHandler`
//! in `aspen-hooks-handler`, operating through the host KV store.
//!
//! Hook configuration and metrics are read from KV, and trigger events
//! are written to KV for the native hook dispatch system to process.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::HookListResultResponse;
use aspen_client_api::HookMetricsResultResponse;
use aspen_client_api::HookTriggerResultResponse;

use crate::kv;
use crate::types::HookEvent;
use crate::types::HooksConfig;
use crate::types::HooksMetrics;
use crate::types::is_triggerable_event_type;

/// KV key for the hooks configuration.
const HOOKS_CONFIG_KEY: &str = "__hooks:config";

/// KV key for the hooks metrics snapshot.
const HOOKS_METRICS_KEY: &str = "__hooks:metrics";

/// KV key prefix for pending trigger events.
const HOOKS_TRIGGER_PREFIX: &str = "__hooks:trigger:";

/// Maximum number of pending trigger events.
const MAX_PENDING_TRIGGERS: u32 = 1000;

// ============================================================================
// Config helpers
// ============================================================================

/// Read the hooks configuration from KV.
fn read_config() -> Option<HooksConfig> {
    let bytes = kv::kv_get(HOOKS_CONFIG_KEY).ok()??;
    serde_json::from_slice(&bytes).ok()
}

/// Read the hooks metrics from KV.
fn read_metrics() -> Option<HooksMetrics> {
    let bytes = kv::kv_get(HOOKS_METRICS_KEY).ok()??;
    serde_json::from_slice(&bytes).ok()
}

// ============================================================================
// Handlers
// ============================================================================

/// Handle `HookList` request.
///
/// Returns information about all configured hook handlers by reading
/// the hooks configuration from the cluster KV store.
pub fn handle_hook_list() -> ClientRpcResponse {
    let config = read_config().unwrap_or_default();

    let handlers = config.handlers.iter().map(|h| h.to_info()).collect();

    ClientRpcResponse::HookListResult(HookListResultResponse {
        is_enabled: config.is_enabled,
        handlers,
    })
}

/// Handle `HookGetMetrics` request.
///
/// Returns execution metrics for hook handlers. When a handler name is
/// specified, only metrics for that handler are returned.
pub fn handle_hook_metrics(handler_name: Option<String>) -> ClientRpcResponse {
    let config = read_config().unwrap_or_default();

    if !config.is_enabled {
        return ClientRpcResponse::HookMetricsResult(HookMetricsResultResponse {
            is_enabled: false,
            total_events_processed: 0,
            handlers: vec![],
        });
    }

    let metrics = read_metrics().unwrap_or_default();

    let handlers = if let Some(ref name) = handler_name {
        metrics.handlers.iter().filter(|m| m.name == *name).map(|m| m.to_response()).collect()
    } else {
        metrics.handlers.iter().map(|m| m.to_response()).collect()
    };

    let total = metrics.global_successes + metrics.global_failures;

    ClientRpcResponse::HookMetricsResult(HookMetricsResultResponse {
        is_enabled: true,
        total_events_processed: total,
        handlers,
    })
}

/// Handle `HookTrigger` request.
///
/// Validates the event type and writes a trigger event to KV for
/// the native hook dispatch system to pick up and process.
pub fn handle_hook_trigger(event_type: String, payload_json: String) -> ClientRpcResponse {
    // Validate event type first — invalid types should always fail
    // regardless of whether hooks are enabled
    if !is_triggerable_event_type(&event_type) {
        return ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
            is_success: false,
            dispatched_count: 0,
            error: Some(format!("unknown event type: {event_type}")),
            handler_failures: vec![],
        });
    }

    // Check if hooks are enabled
    let config = read_config().unwrap_or_default();
    if !config.is_enabled {
        return ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
            is_success: false,
            dispatched_count: 0,
            error: Some("hooks not enabled".to_string()),
            handler_failures: vec![],
        });
    }

    // Parse JSON payload
    let payload: serde_json::Value = serde_json::from_str(&payload_json).unwrap_or_else(|_| serde_json::json!({}));

    let now = kv::now_ms();
    let node_id = aspen_wasm_guest_sdk::host::get_node_id();

    // Create the trigger event
    let event = HookEvent {
        event_type: event_type.clone(),
        node_id,
        timestamp_ms: now,
        payload,
    };

    // Check we haven't exceeded the pending trigger limit
    let pending = kv::kv_scan(HOOKS_TRIGGER_PREFIX, MAX_PENDING_TRIGGERS).unwrap_or_default();
    if pending.len() >= MAX_PENDING_TRIGGERS as usize {
        return ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
            is_success: false,
            dispatched_count: 0,
            error: Some("too many pending trigger events".to_string()),
            handler_failures: vec![],
        });
    }

    // Write the event to KV for the native dispatch system
    let event_key = format!("{HOOKS_TRIGGER_PREFIX}{now}:{event_type}");
    let event_bytes = match serde_json::to_vec(&event) {
        Ok(bytes) => bytes,
        Err(e) => {
            return ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
                is_success: false,
                dispatched_count: 0,
                error: Some(format!("failed to serialize event: {e}")),
                handler_failures: vec![],
            });
        }
    };

    match kv::kv_put(&event_key, &event_bytes) {
        Ok(()) => {
            // Count how many enabled handlers match this event type
            let dispatched_count = count_matching_handlers(&config, &event_type);

            ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
                is_success: true,
                dispatched_count,
                error: None,
                handler_failures: vec![],
            })
        }
        Err(e) => ClientRpcResponse::HookTriggerResult(HookTriggerResultResponse {
            is_success: false,
            dispatched_count: 0,
            error: Some(e),
            handler_failures: vec![],
        }),
    }
}

/// Count enabled handlers whose topic pattern matches the given event type.
///
/// Uses simple NATS-style wildcard matching:
/// - `hooks.>` matches all hook topics
/// - `hooks.kv.*` matches `hooks.kv.write_committed`, `hooks.kv.delete_committed`, etc.
/// - Exact match for fully-qualified topic names
fn count_matching_handlers(config: &HooksConfig, event_type: &str) -> u32 {
    let topic = event_type_to_topic(event_type);
    config.handlers.iter().filter(|h| h.is_enabled && topic_matches_pattern(&topic, &h.pattern)).count() as u32
}

/// Convert an event type string to a full topic name.
fn event_type_to_topic(event_type: &str) -> String {
    let suffix = match event_type {
        "write_committed" => "kv.write_committed",
        "delete_committed" => "kv.delete_committed",
        "leader_elected" => "cluster.leader_elected",
        "membership_changed" => "cluster.membership_changed",
        "node_added" => "cluster.node_added",
        "node_removed" => "cluster.node_removed",
        "snapshot_created" => "system.snapshot_created",
        "snapshot_installed" => "system.snapshot_installed",
        "health_changed" => "system.health_changed",
        "ttl_expired" => "kv.ttl_expired",
        "blob_added" => "blob.blob_added",
        "blob_deleted" => "blob.blob_deleted",
        "blob_downloaded" => "blob.blob_downloaded",
        "blob_protected" => "blob.blob_protected",
        "blob_unprotected" => "blob.blob_unprotected",
        "docs_sync_started" => "docs.sync_started",
        "docs_sync_completed" => "docs.sync_completed",
        "docs_entry_imported" => "docs.entry_imported",
        "docs_entry_exported" => "docs.entry_exported",
        other => return format!("hooks.{other}"),
    };
    format!("hooks.{suffix}")
}

/// Check if a topic matches a NATS-style pattern.
///
/// Supports:
/// - `>` at the end matches any remaining segments
/// - `*` matches exactly one segment
/// - Exact match otherwise
fn topic_matches_pattern(topic: &str, pattern: &str) -> bool {
    let topic_parts: Vec<&str> = topic.split('.').collect();
    let pattern_parts: Vec<&str> = pattern.split('.').collect();

    let mut ti = 0;
    let mut pi = 0;

    while pi < pattern_parts.len() && ti < topic_parts.len() {
        match pattern_parts[pi] {
            ">" => return true, // Matches all remaining segments
            "*" => {
                // Matches exactly one segment
                ti += 1;
                pi += 1;
            }
            exact => {
                if exact != topic_parts[ti] {
                    return false;
                }
                ti += 1;
                pi += 1;
            }
        }
    }

    // Both must be exhausted for a match (unless pattern ended with >)
    ti == topic_parts.len() && pi == pattern_parts.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ALL_EVENT_TYPES;
    use crate::types::HandlerMetrics;
    use crate::types::HookHandlerConfig;
    use crate::types::TRIGGERABLE_EVENT_TYPES;

    // ========================================================================
    // topic_matches_pattern
    // ========================================================================

    #[test]
    fn topic_matches_exact_match() {
        assert!(topic_matches_pattern("hooks.kv.write_committed", "hooks.kv.write_committed"));
    }

    #[test]
    fn topic_rejects_different_leaf() {
        assert!(!topic_matches_pattern("hooks.kv.write_committed", "hooks.kv.delete_committed"));
    }

    #[test]
    fn topic_rejects_different_prefix() {
        assert!(!topic_matches_pattern("other.kv.write_committed", "hooks.kv.write_committed"));
    }

    #[test]
    fn topic_gt_matches_any_remaining_segments() {
        assert!(topic_matches_pattern("hooks.kv.write_committed", "hooks.>"));
        assert!(topic_matches_pattern("hooks.cluster.leader_elected", "hooks.>"));
        assert!(topic_matches_pattern("hooks.system.snapshot_created", "hooks.>"));
        assert!(topic_matches_pattern("hooks.blob.blob_added", "hooks.>"));
        assert!(topic_matches_pattern("hooks.docs.sync_started", "hooks.>"));
    }

    #[test]
    fn topic_gt_requires_prefix_match() {
        assert!(!topic_matches_pattern("other.kv.write_committed", "hooks.>"));
        assert!(!topic_matches_pattern("nothooks.kv.foo", "hooks.>"));
    }

    #[test]
    fn topic_gt_at_root_matches_everything() {
        assert!(topic_matches_pattern("hooks.kv.write_committed", ">"));
        assert!(topic_matches_pattern("anything.at.all", ">"));
        assert!(topic_matches_pattern("single", ">"));
    }

    #[test]
    fn topic_star_matches_exactly_one_segment() {
        assert!(topic_matches_pattern("hooks.kv.write_committed", "hooks.kv.*"));
        assert!(topic_matches_pattern("hooks.kv.delete_committed", "hooks.kv.*"));
        assert!(topic_matches_pattern("hooks.kv.ttl_expired", "hooks.kv.*"));
    }

    #[test]
    fn topic_star_rejects_wrong_category() {
        assert!(!topic_matches_pattern("hooks.cluster.leader_elected", "hooks.kv.*"));
        assert!(!topic_matches_pattern("hooks.system.health_changed", "hooks.kv.*"));
    }

    #[test]
    fn topic_star_at_category_level() {
        assert!(topic_matches_pattern("hooks.kv.write_committed", "hooks.*.write_committed"));
        assert!(topic_matches_pattern("hooks.blob.write_committed", "hooks.*.write_committed"));
        assert!(!topic_matches_pattern("hooks.kv.delete_committed", "hooks.*.write_committed"));
    }

    #[test]
    fn topic_pattern_shorter_than_topic_no_match() {
        assert!(!topic_matches_pattern("hooks.kv.write_committed", "hooks.kv"));
        assert!(!topic_matches_pattern("hooks.kv.write_committed", "hooks"));
    }

    #[test]
    fn topic_pattern_longer_than_topic_no_match() {
        assert!(!topic_matches_pattern("hooks.kv", "hooks.kv.write_committed"));
        assert!(!topic_matches_pattern("hooks", "hooks.kv.write_committed"));
    }

    #[test]
    fn topic_single_segment_exact() {
        assert!(topic_matches_pattern("hooks", "hooks"));
        assert!(!topic_matches_pattern("hooks", "other"));
    }

    #[test]
    fn topic_single_segment_star() {
        assert!(topic_matches_pattern("hooks", "*"));
        assert!(topic_matches_pattern("anything", "*"));
    }

    #[test]
    fn topic_empty_strings() {
        assert!(topic_matches_pattern("", ""));
    }

    #[test]
    fn topic_star_in_middle_and_end() {
        assert!(topic_matches_pattern("hooks.kv.write_committed", "hooks.*.*"));
        assert!(!topic_matches_pattern("hooks.kv", "hooks.*.*"));
    }

    #[test]
    fn topic_multiple_stars() {
        assert!(topic_matches_pattern("a.b.c", "*.*.*"));
        assert!(!topic_matches_pattern("a.b", "*.*.*"));
        assert!(!topic_matches_pattern("a.b.c.d", "*.*.*"));
    }

    #[test]
    fn topic_category_wildcards() {
        // Ensure all category wildcards work as expected
        assert!(topic_matches_pattern("hooks.kv.write_committed", "hooks.kv.*"));
        assert!(topic_matches_pattern("hooks.cluster.leader_elected", "hooks.cluster.*"));
        assert!(topic_matches_pattern("hooks.system.snapshot_created", "hooks.system.*"));
        assert!(topic_matches_pattern("hooks.blob.blob_added", "hooks.blob.*"));
        assert!(topic_matches_pattern("hooks.docs.sync_started", "hooks.docs.*"));
    }

    // ========================================================================
    // event_type_to_topic — exhaustive coverage of all 19 event types
    // ========================================================================

    #[test]
    fn event_type_to_topic_kv_events() {
        assert_eq!(event_type_to_topic("write_committed"), "hooks.kv.write_committed");
        assert_eq!(event_type_to_topic("delete_committed"), "hooks.kv.delete_committed");
        assert_eq!(event_type_to_topic("ttl_expired"), "hooks.kv.ttl_expired");
    }

    #[test]
    fn event_type_to_topic_cluster_events() {
        assert_eq!(event_type_to_topic("leader_elected"), "hooks.cluster.leader_elected");
        assert_eq!(event_type_to_topic("membership_changed"), "hooks.cluster.membership_changed");
        assert_eq!(event_type_to_topic("node_added"), "hooks.cluster.node_added");
        assert_eq!(event_type_to_topic("node_removed"), "hooks.cluster.node_removed");
    }

    #[test]
    fn event_type_to_topic_system_events() {
        assert_eq!(event_type_to_topic("snapshot_created"), "hooks.system.snapshot_created");
        assert_eq!(event_type_to_topic("snapshot_installed"), "hooks.system.snapshot_installed");
        assert_eq!(event_type_to_topic("health_changed"), "hooks.system.health_changed");
    }

    #[test]
    fn event_type_to_topic_blob_events() {
        assert_eq!(event_type_to_topic("blob_added"), "hooks.blob.blob_added");
        assert_eq!(event_type_to_topic("blob_deleted"), "hooks.blob.blob_deleted");
        assert_eq!(event_type_to_topic("blob_downloaded"), "hooks.blob.blob_downloaded");
        assert_eq!(event_type_to_topic("blob_protected"), "hooks.blob.blob_protected");
        assert_eq!(event_type_to_topic("blob_unprotected"), "hooks.blob.blob_unprotected");
    }

    #[test]
    fn event_type_to_topic_docs_events() {
        assert_eq!(event_type_to_topic("docs_sync_started"), "hooks.docs.sync_started");
        assert_eq!(event_type_to_topic("docs_sync_completed"), "hooks.docs.sync_completed");
        assert_eq!(event_type_to_topic("docs_entry_imported"), "hooks.docs.entry_imported");
        assert_eq!(event_type_to_topic("docs_entry_exported"), "hooks.docs.entry_exported");
    }

    #[test]
    fn event_type_to_topic_unknown_falls_through() {
        // Unknown event types get a passthrough mapping
        assert_eq!(event_type_to_topic("custom_event"), "hooks.custom_event");
        assert_eq!(event_type_to_topic("anything"), "hooks.anything");
    }

    #[test]
    fn event_type_to_topic_matches_native_hook_event_type() {
        // Verify every ALL_EVENT_TYPES entry has a dedicated match arm
        // (i.e. doesn't fall through to the `other` branch).
        for &et in ALL_EVENT_TYPES {
            let topic = event_type_to_topic(et);
            // Dedicated arms always produce "hooks.{category}.{name}" with a dot
            // separator between category and event name.
            let after_hooks = topic.strip_prefix("hooks.").unwrap();
            assert!(after_hooks.contains('.'), "event type '{et}' fell through to passthrough (got '{topic}')");
        }
    }

    // ========================================================================
    // is_triggerable_event_type
    // ========================================================================

    #[test]
    fn triggerable_accepts_all_defined_types() {
        for &et in TRIGGERABLE_EVENT_TYPES {
            assert!(is_triggerable_event_type(et), "TRIGGERABLE_EVENT_TYPES entry '{et}' should be accepted");
        }
    }

    #[test]
    fn triggerable_rejects_non_triggerable_event_types() {
        // These are valid event types but NOT manually triggerable
        // (they are emitted internally by subsystem watchers).
        let non_triggerable = [
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
        for et in &non_triggerable {
            assert!(!is_triggerable_event_type(et), "'{et}' should NOT be triggerable");
        }
    }

    #[test]
    fn triggerable_rejects_garbage_input() {
        assert!(!is_triggerable_event_type(""));
        assert!(!is_triggerable_event_type("invalid_type"));
        assert!(!is_triggerable_event_type("WRITE_COMMITTED")); // case-sensitive
        assert!(!is_triggerable_event_type("write committed")); // space
        assert!(!is_triggerable_event_type("write_committed ")); // trailing space
        assert!(!is_triggerable_event_type(" write_committed")); // leading space
    }

    #[test]
    fn triggerable_is_subset_of_all_event_types() {
        for &et in TRIGGERABLE_EVENT_TYPES {
            assert!(ALL_EVENT_TYPES.contains(&et), "TRIGGERABLE_EVENT_TYPES entry '{et}' missing from ALL_EVENT_TYPES");
        }
    }

    // ========================================================================
    // count_matching_handlers
    // ========================================================================

    fn make_handler(name: &str, pattern: &str, enabled: bool) -> HookHandlerConfig {
        HookHandlerConfig {
            name: name.to_string(),
            pattern: pattern.to_string(),
            handler_type: "in_process".to_string(),
            execution_mode: "direct".to_string(),
            is_enabled: enabled,
            timeout_ms: 5000,
            retry_count: 3,
        }
    }

    #[test]
    fn count_matching_no_handlers() {
        let config = HooksConfig {
            is_enabled: true,
            handlers: vec![],
        };
        assert_eq!(count_matching_handlers(&config, "write_committed"), 0);
    }

    #[test]
    fn count_matching_all_handlers_match_wildcard() {
        let config = HooksConfig {
            is_enabled: true,
            handlers: vec![
                make_handler("a", "hooks.>", true),
                make_handler("b", "hooks.>", true),
                make_handler("c", "hooks.>", true),
            ],
        };
        assert_eq!(count_matching_handlers(&config, "write_committed"), 3);
        assert_eq!(count_matching_handlers(&config, "leader_elected"), 3);
    }

    #[test]
    fn count_matching_category_filter() {
        let config = HooksConfig {
            is_enabled: true,
            handlers: vec![
                make_handler("kv-watcher", "hooks.kv.*", true),
                make_handler("cluster-watcher", "hooks.cluster.*", true),
                make_handler("all-watcher", "hooks.>", true),
            ],
        };
        // write_committed is a kv event => matches kv-watcher + all-watcher
        assert_eq!(count_matching_handlers(&config, "write_committed"), 2);
        // leader_elected is a cluster event => matches cluster-watcher + all-watcher
        assert_eq!(count_matching_handlers(&config, "leader_elected"), 2);
    }

    #[test]
    fn count_matching_exact_filter() {
        let config = HooksConfig {
            is_enabled: true,
            handlers: vec![
                make_handler("write-only", "hooks.kv.write_committed", true),
                make_handler("delete-only", "hooks.kv.delete_committed", true),
            ],
        };
        assert_eq!(count_matching_handlers(&config, "write_committed"), 1);
        assert_eq!(count_matching_handlers(&config, "delete_committed"), 1);
        assert_eq!(count_matching_handlers(&config, "leader_elected"), 0);
    }

    #[test]
    fn count_matching_disabled_handlers_excluded() {
        let config = HooksConfig {
            is_enabled: true,
            handlers: vec![
                make_handler("enabled", "hooks.>", true),
                make_handler("disabled", "hooks.>", false),
            ],
        };
        assert_eq!(count_matching_handlers(&config, "write_committed"), 1);
    }

    #[test]
    fn count_matching_mixed_patterns() {
        let config = HooksConfig {
            is_enabled: true,
            handlers: vec![
                make_handler("all", "hooks.>", true),
                make_handler("kv", "hooks.kv.*", true),
                make_handler("exact", "hooks.kv.write_committed", true),
                make_handler("wrong", "hooks.cluster.*", true),
                make_handler("disabled-exact", "hooks.kv.write_committed", false),
            ],
        };
        // write_committed: all(✓) + kv(✓) + exact(✓) + wrong(✗) + disabled(✗)
        assert_eq!(count_matching_handlers(&config, "write_committed"), 3);
    }

    // ========================================================================
    // Metrics response conversion
    // ========================================================================

    #[test]
    fn handler_metrics_to_response_maps_all_fields() {
        let m = HandlerMetrics {
            name: "audit-logger".to_string(),
            successes: 100,
            failures: 5,
            dropped: 2,
            jobs_submitted: 50,
            avg_latency_us: 1234,
        };
        let resp = m.to_response();
        assert_eq!(resp.name, "audit-logger");
        assert_eq!(resp.success_count, 100);
        assert_eq!(resp.failure_count, 5);
        assert_eq!(resp.dropped_count, 2);
        assert_eq!(resp.jobs_submitted, 50);
        assert_eq!(resp.avg_duration_us, 1234);
        assert_eq!(resp.max_duration_us, 0); // Not tracked
    }

    #[test]
    fn handler_metrics_default_zeroed() {
        let m = HandlerMetrics::default();
        assert_eq!(m.name, "");
        assert_eq!(m.successes, 0);
        assert_eq!(m.failures, 0);
        assert_eq!(m.dropped, 0);
        assert_eq!(m.jobs_submitted, 0);
        assert_eq!(m.avg_latency_us, 0);
    }

    // ========================================================================
    // Handler config conversion
    // ========================================================================

    #[test]
    fn handler_config_to_info_maps_all_fields() {
        let cfg = make_handler("test-handler", "hooks.>", true);
        let info = cfg.to_info();
        assert_eq!(info.name, "test-handler");
        assert_eq!(info.pattern, "hooks.>");
        assert_eq!(info.handler_type, "in_process");
        assert_eq!(info.execution_mode, "direct");
        assert!(info.is_enabled);
        assert_eq!(info.timeout_ms, 5000);
        assert_eq!(info.retry_count, 3);
    }

    #[test]
    fn handler_config_to_info_disabled() {
        let cfg = make_handler("disabled-handler", "hooks.kv.*", false);
        let info = cfg.to_info();
        assert!(!info.is_enabled);
    }
}
