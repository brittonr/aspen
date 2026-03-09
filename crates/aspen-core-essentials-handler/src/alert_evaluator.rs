//! Periodic alert evaluation background task.
//!
//! Runs on the Raft leader node and periodically evaluates all enabled alert
//! rules against current metric data. Evaluations only happen on the leader to
//! avoid duplicate state transitions across cluster nodes.
//!
//! # Architecture
//!
//! The evaluator is a background `tokio::spawn` task that:
//! 1. Waits for the configured interval
//! 2. Checks if this node is the Raft leader
//! 3. Scans all alert rules (`_sys:alerts:rule:*`)
//! 4. For each enabled rule, sends an `AlertEvaluate` RPC through the handler
//! 5. Logs any state transitions (Ok→Pending→Firing→Ok)
//!
//! # Leader-Only Execution
//!
//! Alert evaluation writes state to KV (`_sys:alerts:state:*`), which requires
//! Raft leadership. The evaluator checks leadership each tick and skips the
//! cycle if this node is not the leader. This prevents duplicate evaluations
//! and NOT_LEADER errors.

use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use aspen_client_api::AlertRuleWire;
use aspen_client_api::AlertStatus;
use aspen_client_api::ClientRpcRequest;
use aspen_core::KeyValueStore;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::core::CoreHandler;

/// Configuration for the periodic alert evaluator.
#[derive(Debug, Clone)]
pub struct AlertEvaluatorConfig {
    /// Evaluation interval in seconds.
    /// Clamped to [MIN_INTERVAL, MAX_INTERVAL] at construction.
    pub interval_seconds: u64,
}

impl Default for AlertEvaluatorConfig {
    fn default() -> Self {
        Self {
            interval_seconds: aspen_constants::api::ALERT_EVALUATION_INTERVAL_SECONDS,
        }
    }
}

impl AlertEvaluatorConfig {
    /// Create a new config with the given interval, clamped to valid bounds.
    pub fn with_interval(interval_seconds: u64) -> Self {
        Self {
            interval_seconds: interval_seconds.clamp(
                aspen_constants::api::ALERT_EVALUATION_MIN_INTERVAL_SECONDS,
                aspen_constants::api::ALERT_EVALUATION_MAX_INTERVAL_SECONDS,
            ),
        }
    }
}

/// Handle to the running alert evaluator task.
///
/// Drop or call `shutdown()` to stop the background task.
pub struct AlertEvaluatorHandle {
    cancel: CancellationToken,
    join_handle: tokio::task::JoinHandle<()>,
}

impl AlertEvaluatorHandle {
    /// Stop the evaluator and wait for it to finish.
    pub async fn shutdown(self) {
        self.cancel.cancel();
        if let Err(e) = self.join_handle.await {
            warn!(error = %e, "alert evaluator task panicked");
        }
    }
}

/// Spawn the periodic alert evaluator as a background task.
///
/// The evaluator runs until the cancellation token is triggered or the
/// returned handle is dropped/shutdown.
///
/// # Arguments
///
/// * `ctx` - Client protocol context (provides KV store access)
/// * `config` - Evaluator configuration (interval)
/// * `shutdown_token` - Parent cancellation token for coordinated shutdown
pub fn spawn_alert_evaluator(
    ctx: ClientProtocolContext,
    config: AlertEvaluatorConfig,
    shutdown_token: CancellationToken,
) -> AlertEvaluatorHandle {
    let cancel = shutdown_token.child_token();
    let cancel_clone = cancel.clone();
    let interval_seconds = config.interval_seconds;

    let join_handle = tokio::spawn(async move {
        run_evaluation_loop(ctx, config, cancel_clone).await;
    });

    info!(interval_seconds = interval_seconds, "periodic alert evaluator started");

    AlertEvaluatorHandle { cancel, join_handle }
}

/// Main evaluation loop. Runs until cancelled.
async fn run_evaluation_loop(ctx: ClientProtocolContext, config: AlertEvaluatorConfig, cancel: CancellationToken) {
    let mut interval = tokio::time::interval(Duration::from_secs(config.interval_seconds));
    interval.tick().await; // Skip first immediate tick

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("periodic alert evaluator shutting down");
                return;
            }
            _ = interval.tick() => {
                evaluate_all_rules(&ctx).await;
            }
        }
    }
}

/// Evaluate all enabled alert rules.
///
/// Scans `_sys:alerts:rule:*` for rules, skips disabled ones, and evaluates
/// each enabled rule. Only runs on the leader — skips the cycle otherwise.
async fn evaluate_all_rules(ctx: &ClientProtocolContext) {
    // Check leadership via a lightweight KV scan (ReadIndex).
    // If the scan fails with NOT_LEADER, we skip this cycle.
    let rule_prefix = "_sys:alerts:rule:";
    let scan_req = aspen_core::ScanRequest {
        prefix: rule_prefix.to_string(),
        limit_results: Some(aspen_constants::MAX_ALERT_RULES),
        continuation_token: None,
    };

    let rules = match ctx.kv_store.scan(scan_req).await {
        Ok(result) => result.entries,
        Err(e) => {
            // NOT_LEADER errors are expected on follower nodes — just skip quietly
            let err_msg = format!("{}", e);
            if err_msg.contains("NotLeader") || err_msg.contains("not leader") {
                debug!("skipping alert evaluation cycle: not leader");
            } else {
                warn!(error = %e, "failed to scan alert rules");
            }
            return;
        }
    };

    if rules.is_empty() {
        debug!("no alert rules defined, skipping evaluation");
        return;
    }

    let now_us = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_micros() as u64;

    let mut evaluated_count: u32 = 0;
    let mut transition_count: u32 = 0;

    for entry in &rules {
        let rule: AlertRuleWire = match serde_json::from_str(&entry.value) {
            Ok(r) => r,
            Err(e) => {
                warn!(key = %entry.key, error = %e, "failed to parse alert rule");
                continue;
            }
        };

        if !rule.is_enabled {
            continue;
        }

        // Dispatch through the handler so all evaluation logic is centralized
        let request = ClientRpcRequest::AlertEvaluate {
            name: rule.name.clone(),
            now_us,
        };

        let handler = CoreHandler;
        match handler.handle(request, ctx).await {
            Ok(aspen_client_api::ClientRpcResponse::AlertEvaluateResult(result)) => {
                evaluated_count = evaluated_count.saturating_add(1);

                if result.did_transition {
                    transition_count = transition_count.saturating_add(1);
                    let prev = result.previous_status.as_ref().map(format_status).unwrap_or("unknown");
                    let current = format_status(&result.status);

                    info!(
                        rule = %result.rule_name,
                        from = %prev,
                        to = %current,
                        value = ?result.computed_value,
                        threshold = result.threshold,
                        "alert state transition"
                    );
                }

                if let Some(ref error) = result.error {
                    warn!(rule = %result.rule_name, error = %error, "alert evaluation error");
                }
            }
            Ok(other) => {
                warn!(rule = %rule.name, response = ?other, "unexpected response from alert evaluation");
            }
            Err(e) => {
                warn!(rule = %rule.name, error = %e, "alert evaluation failed");
            }
        }
    }

    if evaluated_count > 0 {
        debug!(
            rules_evaluated = evaluated_count,
            transitions = transition_count,
            total_rules = rules.len(),
            "alert evaluation cycle complete"
        );
    }
}

fn format_status(status: &AlertStatus) -> &'static str {
    match status {
        AlertStatus::Ok => "Ok",
        AlertStatus::Pending => "Pending",
        AlertStatus::Firing => "Firing",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_rpc_core::test_support::MockEndpointProvider;
    use aspen_rpc_core::test_support::TestContextBuilder;

    use super::*;

    async fn test_ctx() -> ClientProtocolContext {
        let mock_endpoint = Arc::new(MockEndpointProvider::with_seed(54321).await);
        TestContextBuilder::new()
            .with_node_id(1)
            .with_endpoint_manager(mock_endpoint)
            .with_cookie("test")
            .build()
    }

    #[test]
    fn test_config_default() {
        let config = AlertEvaluatorConfig::default();
        assert_eq!(config.interval_seconds, aspen_constants::api::ALERT_EVALUATION_INTERVAL_SECONDS);
    }

    #[test]
    fn test_config_clamped_low() {
        let config = AlertEvaluatorConfig::with_interval(1);
        assert_eq!(config.interval_seconds, aspen_constants::api::ALERT_EVALUATION_MIN_INTERVAL_SECONDS);
    }

    #[test]
    fn test_config_clamped_high() {
        let config = AlertEvaluatorConfig::with_interval(999_999);
        assert_eq!(config.interval_seconds, aspen_constants::api::ALERT_EVALUATION_MAX_INTERVAL_SECONDS);
    }

    #[test]
    fn test_config_within_bounds() {
        let config = AlertEvaluatorConfig::with_interval(30);
        assert_eq!(config.interval_seconds, 30);
    }

    #[tokio::test]
    async fn test_evaluator_shutdown() {
        // Verify the evaluator can be spawned and cleanly shut down
        let ctx = test_ctx().await;
        let config = AlertEvaluatorConfig::with_interval(aspen_constants::api::ALERT_EVALUATION_MIN_INTERVAL_SECONDS);
        let shutdown = CancellationToken::new();

        let handle = spawn_alert_evaluator(ctx, config, shutdown.clone());

        // Immediately shut down
        shutdown.cancel();
        handle.shutdown().await;
    }

    #[tokio::test]
    async fn test_evaluate_no_rules() {
        // With empty KV store, evaluate_all_rules should complete without error
        let ctx = test_ctx().await;
        evaluate_all_rules(&ctx).await;
        // No panic = success
    }

    #[tokio::test]
    async fn test_evaluate_with_disabled_rule() {
        use aspen_client_api::AlertComparison;
        use aspen_client_api::AlertSeverity;
        use aspen_core::WriteRequest;

        let ctx = test_ctx().await;

        // Create a disabled rule
        let rule = AlertRuleWire {
            name: "test-disabled".to_string(),
            metric_name: "cpu_usage".to_string(),
            label_filters: vec![],
            aggregation: "avg".to_string(),
            window_duration_us: 300_000_000,
            comparison: AlertComparison::GreaterThan,
            threshold: 90.0,
            for_duration_us: 0,
            severity: AlertSeverity::Warning,
            description: "test".to_string(),
            is_enabled: false,
            created_at_us: 1000,
            updated_at_us: 1000,
        };

        let key = format!("_sys:alerts:rule:{}", rule.name);
        let value = serde_json::to_string(&rule).unwrap();
        ctx.kv_store.write(WriteRequest::set(key, value)).await.unwrap();

        // Evaluate — disabled rule should be skipped, no state written
        evaluate_all_rules(&ctx).await;

        // Verify no state was written (DeterministicKeyValueStore returns NotFound)
        let state_key = format!("_sys:alerts:state:{}", rule.name);
        let result = ctx.kv_store.read(aspen_core::ReadRequest::new(state_key)).await;
        assert!(result.is_err(), "disabled rule should not create state");
    }

    #[tokio::test]
    async fn test_evaluate_with_enabled_rule_no_metrics() {
        use aspen_client_api::AlertComparison;
        use aspen_client_api::AlertSeverity;
        use aspen_core::WriteRequest;

        let ctx = test_ctx().await;

        // Create an enabled rule
        let rule = AlertRuleWire {
            name: "test-enabled".to_string(),
            metric_name: "cpu_usage".to_string(),
            label_filters: vec![],
            aggregation: "avg".to_string(),
            window_duration_us: 300_000_000,
            comparison: AlertComparison::GreaterThan,
            threshold: 90.0,
            for_duration_us: 0,
            severity: AlertSeverity::Warning,
            description: "test".to_string(),
            is_enabled: true,
            created_at_us: 1000,
            updated_at_us: 1000,
        };

        let key = format!("_sys:alerts:rule:{}", rule.name);
        let value = serde_json::to_string(&rule).unwrap();
        ctx.kv_store.write(WriteRequest::set(key, value)).await.unwrap();

        // Evaluate — no metrics means rule stays Ok (no transition)
        evaluate_all_rules(&ctx).await;

        // With no metric data, the handler returns Ok status with no state write
        // (no transition from Ok→Ok when no data exists)
    }

    #[tokio::test]
    async fn test_evaluate_rule_fires_on_threshold() {
        use std::time::SystemTime;
        use std::time::UNIX_EPOCH;

        use aspen_client_api::AlertComparison;
        use aspen_client_api::AlertSeverity;
        use aspen_core::WriteRequest;

        let ctx = test_ctx().await;

        let now_us = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64;

        // Create a rule that fires immediately (for_duration_us = 0)
        let rule = AlertRuleWire {
            name: "cpu-high".to_string(),
            metric_name: "cpu_usage".to_string(),
            label_filters: vec![],
            aggregation: "avg".to_string(),
            window_duration_us: 300_000_000, // 5 minutes
            comparison: AlertComparison::GreaterThan,
            threshold: 80.0,
            for_duration_us: 0,
            severity: AlertSeverity::Critical,
            description: "CPU too high".to_string(),
            is_enabled: true,
            created_at_us: now_us,
            updated_at_us: now_us,
        };

        let rule_key = format!("_sys:alerts:rule:{}", rule.name);
        let rule_value = serde_json::to_string(&rule).unwrap();
        ctx.kv_store.write(WriteRequest::set(rule_key, rule_value)).await.unwrap();

        // Ingest a metric data point above threshold
        let metric_ts = now_us.saturating_sub(10_000_000); // 10s ago
        let metric_key = format!("_sys:metrics:cpu_usage:{:020}", metric_ts);
        let metric_value = serde_json::json!({
            "name": "cpu_usage",
            "metric_type": "Gauge",
            "timestamp_us": metric_ts,
            "value": 95.0,
            "labels": []
        });
        ctx.kv_store.write(WriteRequest::set(metric_key, metric_value.to_string())).await.unwrap();

        // Evaluate — should trigger Ok→Firing transition
        evaluate_all_rules(&ctx).await;

        // Verify state was written with Firing status
        let state_key = "_sys:alerts:state:cpu-high".to_string();
        let result = ctx.kv_store.read(aspen_core::ReadRequest::new(state_key)).await;

        // DeterministicKeyValueStore returns Ok(ReadResult{kv: Some(...)}) for found keys
        let read_result = result.expect("alert state read should succeed");
        assert!(read_result.kv.is_some(), "alert state should be written");
        let state: aspen_client_api::AlertStateWire = serde_json::from_str(&read_result.kv.unwrap().value).unwrap();
        assert_eq!(state.status, AlertStatus::Firing);
        assert_eq!(state.rule_name, "cpu-high");
    }
}
