//! Plugin timer scheduler.
//!
//! Manages periodic and one-shot timers for WASM plugins. When a timer fires,
//! the scheduler calls the guest's `plugin_on_timer` export via the shared
//! sandbox mutex.
//!
//! ## Design
//!
//! Each plugin gets its own [`PluginScheduler`] that holds a map of active
//! timers (as `JoinHandle`s). Timer tasks share the sandbox `Arc<Mutex<...>>`
//! and execute callbacks through `spawn_blocking`.
//!
//! Tiger Style: Bounded timer counts and clamped intervals prevent resource
//! exhaustion from runaway plugins.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::debug;
use tracing::info;
use tracing::warn;

/// Scheduler that manages timers for a single WASM plugin.
pub struct PluginScheduler {
    /// Active timer tasks keyed by timer name.
    timers: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    /// Plugin name for logging.
    plugin_name: String,
    /// Shared sandbox for calling guest functions.
    sandbox: Arc<std::sync::Mutex<hyperlight_wasm::LoadedWasmSandbox>>,
    /// Execution timeout for timer callbacks.
    execution_timeout: Duration,
}

impl PluginScheduler {
    /// Create a new scheduler for a plugin.
    pub fn new(
        plugin_name: String,
        sandbox: Arc<std::sync::Mutex<hyperlight_wasm::LoadedWasmSandbox>>,
        execution_timeout: Duration,
    ) -> Self {
        Self {
            timers: Arc::new(RwLock::new(HashMap::new())),
            plugin_name,
            sandbox,
            execution_timeout,
        }
    }

    /// Schedule a timer. Replaces any existing timer with the same name.
    ///
    /// Returns `Ok(())` on success, `Err` if limits are exceeded.
    pub async fn schedule(&self, config: aspen_plugin_api::TimerConfig) -> Result<(), String> {
        // Clamp interval
        let interval_ms = config
            .interval_ms
            .clamp(aspen_plugin_api::MIN_TIMER_INTERVAL_MS, aspen_plugin_api::MAX_TIMER_INTERVAL_MS);

        // Check timer count limit
        let mut timers = self.timers.write().await;
        if !timers.contains_key(&config.name) && timers.len() >= aspen_plugin_api::MAX_TIMERS_PER_PLUGIN {
            return Err(format!(
                "timer limit reached: plugin '{}' already has {} timers (max {})",
                self.plugin_name,
                timers.len(),
                aspen_plugin_api::MAX_TIMERS_PER_PLUGIN
            ));
        }

        // Cancel existing timer with same name
        if let Some(handle) = timers.remove(&config.name) {
            handle.abort();
        }

        let timer_name = config.name.clone();
        let sandbox = Arc::clone(&self.sandbox);
        let plugin_name = self.plugin_name.clone();
        let timeout = self.execution_timeout;
        let repeating = config.repeating;

        let handle = tokio::spawn(async move {
            let interval = Duration::from_millis(interval_ms);

            // Initial delay
            tokio::time::sleep(interval).await;

            loop {
                // Call the guest's plugin_on_timer export
                let sb = Arc::clone(&sandbox);
                let tn = timer_name.clone();
                let pn = plugin_name.clone();

                let result = tokio::time::timeout(
                    timeout,
                    tokio::task::spawn_blocking(move || {
                        let mut guard = match sb.lock() {
                            Ok(g) => g,
                            Err(e) => {
                                warn!(plugin = %pn, timer = %tn, "sandbox mutex poisoned: {e}");
                                return;
                            }
                        };
                        let input = serde_json::to_vec(&tn).unwrap_or_default();
                        match guard.call_guest_function::<Vec<u8>>("plugin_on_timer", input) {
                            Ok(_) => {
                                debug!(plugin = %pn, timer = %tn, "timer callback completed");
                            }
                            Err(e) => {
                                warn!(plugin = %pn, timer = %tn, error = %e, "timer callback failed");
                            }
                        }
                    }),
                )
                .await;

                if result.is_err() {
                    warn!(
                        plugin = %plugin_name,
                        timer = %timer_name,
                        "timer callback exceeded execution timeout"
                    );
                }

                if !repeating {
                    break;
                }

                tokio::time::sleep(interval).await;
            }
        });

        info!(
            plugin = %self.plugin_name,
            timer = %config.name,
            interval_ms,
            repeating,
            "timer scheduled"
        );

        timers.insert(config.name, handle);
        Ok(())
    }

    /// Cancel a specific timer by name.
    pub async fn cancel(&self, name: &str) -> bool {
        let mut timers = self.timers.write().await;
        if let Some(handle) = timers.remove(name) {
            handle.abort();
            info!(plugin = %self.plugin_name, timer = %name, "timer cancelled");
            true
        } else {
            false
        }
    }

    /// Cancel all timers. Called during plugin shutdown.
    pub async fn cancel_all(&self) {
        let mut timers = self.timers.write().await;
        let count = timers.len();
        for (name, handle) in timers.drain() {
            handle.abort();
            debug!(plugin = %self.plugin_name, timer = %name, "timer cancelled (shutdown)");
        }
        if count > 0 {
            info!(plugin = %self.plugin_name, cancelled = count, "all timers cancelled");
        }
    }
}

impl Drop for PluginScheduler {
    fn drop(&mut self) {
        // Best-effort: abort all timer tasks when scheduler is dropped.
        if let Ok(timers) = self.timers.try_write() {
            for (_, handle) in timers.iter() {
                handle.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn timer_config_clamps_below_minimum() {
        let interval_ms: u64 = 100; // Below minimum
        let clamped =
            interval_ms.clamp(aspen_plugin_api::MIN_TIMER_INTERVAL_MS, aspen_plugin_api::MAX_TIMER_INTERVAL_MS);
        assert_eq!(clamped, 1_000);
    }

    #[test]
    fn timer_config_clamps_above_maximum() {
        let interval_ms: u64 = 100_000_000; // Above maximum
        let clamped =
            interval_ms.clamp(aspen_plugin_api::MIN_TIMER_INTERVAL_MS, aspen_plugin_api::MAX_TIMER_INTERVAL_MS);
        assert_eq!(clamped, 86_400_000);
    }
}
