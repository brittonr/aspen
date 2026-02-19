//! Plugin event router for hook event delivery.
//!
//! Manages hook subscriptions for a single WASM plugin and delivers
//! matching events by calling the guest's `plugin_on_hook_event` export.
//!
//! ## Design
//!
//! Each plugin gets its own [`PluginEventRouter`] that holds a set of
//! active subscription patterns. When `deliver` is called with a hook
//! event, the router checks all patterns against the event's topic.
//! If any match, the event is serialized and passed to the guest via
//! `spawn_blocking` (since hyperlight calls are CPU-bound).
//!
//! Topic patterns use NATS-style wildcards:
//! - `*` matches exactly one segment
//! - `>` matches zero or more trailing segments
//!
//! Tiger Style: Bounded subscription counts and pattern lengths prevent
//! resource exhaustion from runaway plugins.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Router that manages hook subscriptions for a single WASM plugin
/// and delivers matching events to the guest sandbox.
pub struct PluginEventRouter {
    /// Active subscription patterns.
    patterns: Arc<RwLock<HashSet<String>>>,
    /// Plugin name for logging.
    plugin_name: String,
    /// Shared sandbox for calling guest functions.
    sandbox: Arc<std::sync::Mutex<hyperlight_wasm::LoadedWasmSandbox>>,
    /// Execution timeout for event delivery callbacks.
    execution_timeout: Duration,
}

impl PluginEventRouter {
    /// Create a new event router for a plugin.
    pub fn new(
        plugin_name: String,
        sandbox: Arc<std::sync::Mutex<hyperlight_wasm::LoadedWasmSandbox>>,
        execution_timeout: Duration,
    ) -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashSet::new())),
            plugin_name,
            sandbox,
            execution_timeout,
        }
    }

    /// Add a subscription pattern.
    ///
    /// Returns `Ok(())` on success, `Err` if the subscription limit is reached.
    pub async fn subscribe(&self, pattern: String) -> Result<(), String> {
        let mut patterns = self.patterns.write().await;
        if patterns.contains(&pattern) {
            return Ok(()); // Already subscribed, idempotent
        }
        if patterns.len() >= aspen_plugin_api::MAX_HOOK_SUBSCRIPTIONS_PER_PLUGIN {
            return Err(format!(
                "subscription limit reached: plugin '{}' already has {} subscriptions (max {})",
                self.plugin_name,
                patterns.len(),
                aspen_plugin_api::MAX_HOOK_SUBSCRIPTIONS_PER_PLUGIN
            ));
        }
        info!(
            plugin = %self.plugin_name,
            pattern = %pattern,
            "hook subscription added"
        );
        patterns.insert(pattern);
        Ok(())
    }

    /// Remove a subscription pattern.
    ///
    /// Returns `true` if the pattern was found and removed.
    pub async fn unsubscribe(&self, pattern: &str) -> bool {
        let mut patterns = self.patterns.write().await;
        let removed = patterns.remove(pattern);
        if removed {
            info!(
                plugin = %self.plugin_name,
                pattern = %pattern,
                "hook subscription removed"
            );
        }
        removed
    }

    /// Remove all subscriptions. Called during plugin shutdown.
    pub async fn unsubscribe_all(&self) {
        let mut patterns = self.patterns.write().await;
        let count = patterns.len();
        patterns.clear();
        if count > 0 {
            info!(
                plugin = %self.plugin_name,
                removed = count,
                "all hook subscriptions removed"
            );
        }
    }

    /// Check if any subscription pattern matches the given topic.
    pub async fn has_matching_subscription(&self, topic: &str) -> bool {
        let patterns = self.patterns.read().await;
        patterns.iter().any(|p| pattern_matches(p, topic))
    }

    /// Return the number of active subscriptions.
    pub async fn subscription_count(&self) -> usize {
        self.patterns.read().await.len()
    }

    /// Deliver a hook event to the guest if any subscription matches.
    ///
    /// The event is serialized as JSON and passed to the guest's
    /// `plugin_on_hook_event` export. Returns `true` if the event
    /// was delivered (a subscription matched), `false` if skipped.
    pub async fn deliver(&self, topic: &str, event_json: &[u8]) -> bool {
        if !self.has_matching_subscription(topic).await {
            return false;
        }

        let sandbox = Arc::clone(&self.sandbox);
        let plugin_name = self.plugin_name.clone();
        let timeout = self.execution_timeout;
        let topic_owned = topic.to_string();
        let event_owned = event_json.to_vec();

        // Build the input payload: JSON object with topic + event
        let input = match serde_json::to_vec(&serde_json::json!({
            "topic": topic_owned,
            "event": serde_json::from_slice::<serde_json::Value>(&event_owned).unwrap_or_default(),
        })) {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    plugin = %plugin_name,
                    topic = %topic_owned,
                    error = %e,
                    "failed to serialize hook event for delivery"
                );
                return false;
            }
        };

        let result = tokio::time::timeout(
            timeout,
            tokio::task::spawn_blocking(move || {
                let mut guard = match sandbox.lock() {
                    Ok(g) => g,
                    Err(e) => {
                        warn!(
                            plugin = %plugin_name,
                            topic = %topic_owned,
                            "sandbox mutex poisoned: {e}"
                        );
                        return;
                    }
                };
                match guard.call_guest_function::<Vec<u8>>("plugin_on_hook_event", input) {
                    Ok(_) => {
                        debug!(
                            plugin = %plugin_name,
                            topic = %topic_owned,
                            "hook event delivered"
                        );
                    }
                    Err(e) => {
                        warn!(
                            plugin = %plugin_name,
                            topic = %topic_owned,
                            error = %e,
                            "hook event delivery failed"
                        );
                    }
                }
            }),
        )
        .await;

        if result.is_err() {
            warn!(
                plugin = %self.plugin_name,
                topic = %topic,
                "hook event delivery exceeded execution timeout"
            );
        }

        true
    }
}

/// Check if a NATS-style pattern matches a dot-delimited topic.
///
/// - `*` matches exactly one segment
/// - `>` matches zero or more trailing segments (must be at end)
///
/// Examples:
/// - `hooks.kv.*` matches `hooks.kv.write_committed` but not `hooks.cluster.leader_elected`
/// - `hooks.>` matches `hooks.kv.write_committed`, `hooks.cluster.leader_elected`, etc.
/// - `>` matches everything
fn pattern_matches(pattern: &str, topic: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('.').collect();
    let topic_parts: Vec<&str> = topic.split('.').collect();

    let mut pi = 0;
    let mut ti = 0;

    while pi < pattern_parts.len() {
        let p = pattern_parts[pi];

        if p == ">" {
            return true;
        }

        if ti >= topic_parts.len() {
            return false;
        }

        if p != "*" && p != topic_parts[ti] {
            return false;
        }
        pi += 1;
        ti += 1;
    }

    ti >= topic_parts.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // pattern_matches
    // -------------------------------------------------------------------------

    #[test]
    fn exact_match() {
        assert!(pattern_matches("hooks.kv.write_committed", "hooks.kv.write_committed"));
    }

    #[test]
    fn exact_mismatch() {
        assert!(!pattern_matches("hooks.kv.write_committed", "hooks.kv.delete_committed"));
    }

    #[test]
    fn single_wildcard_matches_one_segment() {
        assert!(pattern_matches("hooks.kv.*", "hooks.kv.write_committed"));
        assert!(pattern_matches("hooks.kv.*", "hooks.kv.delete_committed"));
    }

    #[test]
    fn single_wildcard_rejects_wrong_prefix() {
        assert!(!pattern_matches("hooks.kv.*", "hooks.cluster.leader_elected"));
    }

    #[test]
    fn single_wildcard_rejects_too_few_segments() {
        assert!(!pattern_matches("hooks.kv.*", "hooks.kv"));
    }

    #[test]
    fn multi_wildcard_matches_trailing() {
        assert!(pattern_matches("hooks.>", "hooks.kv.write_committed"));
        assert!(pattern_matches("hooks.>", "hooks.cluster.leader_elected"));
        assert!(pattern_matches("hooks.>", "hooks"));
    }

    #[test]
    fn multi_wildcard_at_root() {
        assert!(pattern_matches(">", "hooks.kv.write_committed"));
        assert!(pattern_matches(">", "anything"));
    }

    #[test]
    fn combined_wildcards() {
        assert!(pattern_matches("hooks.*.>", "hooks.kv.write_committed"));
        assert!(pattern_matches("hooks.*.>", "hooks.cluster.leader_elected"));
    }

    #[test]
    fn no_match_shorter_topic() {
        assert!(!pattern_matches("hooks.kv.write_committed", "hooks.kv"));
    }

    #[test]
    fn no_match_longer_topic() {
        assert!(!pattern_matches("hooks.kv", "hooks.kv.write_committed"));
    }
}
