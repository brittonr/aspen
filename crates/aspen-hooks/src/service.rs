//! Hook service for routing events to handlers.
//!
//! The HookService manages hook handlers and provides the core routing logic.
//! It supports both direct (fast path) and job-based (reliable path) execution modes.
//!
//! Note: Full subscription-based event routing requires integration with the
//! LogSubscriber from aspen-core. This module provides the handler management
//! and routing infrastructure.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::config::ExecutionMode;
use crate::config::HookHandlerConfig;
use crate::config::HooksConfig;
use crate::constants::MAX_CONCURRENT_HANDLER_EXECUTIONS;
use crate::error::Result;
use crate::event::HookEvent;
use crate::handlers::HookHandler;
use crate::handlers::InProcessHandlerRegistry;
use crate::handlers::create_handler_from_config;
use crate::metrics::ExecutionMetrics;
use crate::worker::create_hook_job_spec;

/// Hook service that routes events to handlers.
///
/// The service manages configured handlers and dispatches
/// events based on their execution mode:
///
/// - **Direct mode**: Events are handled immediately in-process with semaphore-controlled
///   concurrency.
/// - **Job mode**: Events are submitted to the job queue for reliable execution with retry and DLQ
///   support.
///
/// # Example
///
/// ```ignore
/// let config = HooksConfig::default();
/// let service = HookService::new(config);
///
/// // Register in-process handlers
/// service.register_handler("cache_invalidator", |event| async {
///     // Handle event
///     Ok(())
/// }).await;
///
/// // Dispatch an event
/// let event = HookEvent::new(HookEventType::WriteCommitted, 1, json!({}));
/// service.dispatch(&event).await?;
/// ```
pub struct HookService {
    /// Hook configuration.
    config: HooksConfig,

    /// Registry for in-process handlers.
    registry: Arc<InProcessHandlerRegistry>,

    /// Map of handler names to their configurations and implementations.
    handlers: HashMap<String, (Box<dyn HookHandler>, HookHandlerConfig)>,

    /// Concurrency semaphore for direct execution.
    semaphore: Arc<Semaphore>,

    /// Execution metrics.
    metrics: Arc<ExecutionMetrics>,

    /// Cancellation token for shutdown.
    cancel: CancellationToken,
}

impl HookService {
    /// Create a new hook service.
    pub fn new(config: HooksConfig) -> Self {
        let registry = Arc::new(InProcessHandlerRegistry::new());
        let mut handlers = HashMap::new();

        // Create handlers for each configured handler
        for handler_config in config.enabled_handlers() {
            let handler = create_handler_from_config(handler_config, Arc::clone(&registry));
            handlers.insert(handler_config.name.clone(), (handler, handler_config.clone()));
        }

        Self {
            config,
            registry,
            handlers,
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_HANDLER_EXECUTIONS)),
            metrics: Arc::new(ExecutionMetrics::new()),
            cancel: CancellationToken::new(),
        }
    }

    /// Set a custom in-process handler registry.
    pub fn with_registry(mut self, registry: Arc<InProcessHandlerRegistry>) -> Self {
        // Recreate handlers with new registry
        self.handlers.clear();
        for handler_config in self.config.enabled_handlers() {
            let handler = create_handler_from_config(handler_config, Arc::clone(&registry));
            self.handlers.insert(handler_config.name.clone(), (handler, handler_config.clone()));
        }
        self.registry = registry;
        self
    }

    /// Set a custom concurrency limit.
    pub fn with_concurrency_limit(mut self, limit: usize) -> Self {
        self.semaphore = Arc::new(Semaphore::new(limit));
        self
    }

    /// Register an in-process handler function.
    pub async fn register_handler<F, Fut>(&self, id: impl Into<String>, handler: F)
    where
        F: Fn(&HookEvent) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.registry.register(id, handler).await;
    }

    /// Get the in-process handler registry.
    pub fn registry(&self) -> Arc<InProcessHandlerRegistry> {
        Arc::clone(&self.registry)
    }

    /// Get execution metrics.
    pub fn metrics(&self) -> &ExecutionMetrics {
        &self.metrics
    }

    /// Check if the service is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Dispatch an event to matching handlers.
    ///
    /// For direct mode handlers, the event is processed immediately.
    /// For job mode handlers, a job spec is returned that should be
    /// submitted to the job manager.
    pub async fn dispatch(&self, event: &HookEvent) -> Result<DispatchResult> {
        if !self.config.enabled {
            return Ok(DispatchResult::Disabled);
        }

        let event_topic = event.topic();
        let mut direct_results = Vec::new();
        let mut job_specs = Vec::new();

        for (name, (handler, config)) in &self.handlers {
            // Check if the handler's pattern matches this event's topic
            if !pattern_matches(&config.pattern, &event_topic) {
                continue;
            }

            match config.execution_mode {
                ExecutionMode::Direct => {
                    // Try to acquire semaphore permit
                    let permit = match self.semaphore.clone().try_acquire_owned() {
                        Ok(p) => p,
                        Err(_) => {
                            warn!(
                                handler = %name,
                                "concurrency limit reached, dropping event"
                            );
                            self.metrics.record_dropped(name);
                            direct_results.push((
                                name.clone(),
                                Err(crate::error::HookError::ConcurrencyLimitReached {
                                    limit: MAX_CONCURRENT_HANDLER_EXECUTIONS,
                                }),
                            ));
                            continue;
                        }
                    };

                    // Execute handler
                    let start = std::time::Instant::now();
                    let result = handler.handle(event).await;
                    let duration = start.elapsed();

                    drop(permit);

                    match &result {
                        Ok(()) => {
                            debug!(
                                handler = %name,
                                duration_ms = %duration.as_millis(),
                                "handler executed successfully"
                            );
                            self.metrics.record_success(name, duration);
                        }
                        Err(e) => {
                            warn!(
                                handler = %name,
                                duration_ms = %duration.as_millis(),
                                error = ?e,
                                "handler execution failed"
                            );
                            self.metrics.record_failure(name, duration);
                        }
                    }

                    direct_results.push((name.clone(), result));
                }
                ExecutionMode::Job => {
                    // Create job spec for later submission
                    let job_spec = create_hook_job_spec(config, event.clone());
                    job_specs.push((name.clone(), job_spec));
                    self.metrics.record_job_submitted(name);
                }
            }
        }

        Ok(DispatchResult::Dispatched {
            direct_results,
            job_specs,
        })
    }

    /// Shutdown the service.
    pub fn shutdown(&self) {
        info!("shutting down hook service");
        self.cancel.cancel();
    }

    /// Get the cancellation token.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }
}

/// Result of dispatching an event.
#[derive(Debug)]
pub enum DispatchResult {
    /// Service is disabled.
    Disabled,

    /// Event was dispatched to handlers.
    Dispatched {
        /// Results from direct mode handlers.
        direct_results: Vec<(String, Result<()>)>,

        /// Job specs for job mode handlers (need to be submitted to JobManager).
        job_specs: Vec<(String, aspen_jobs::JobSpec)>,
    },
}

impl DispatchResult {
    /// Check if any direct handlers failed.
    pub fn has_failures(&self) -> bool {
        match self {
            DispatchResult::Disabled => false,
            DispatchResult::Dispatched { direct_results, .. } => direct_results.iter().any(|(_, r)| r.is_err()),
        }
    }

    /// Get the number of handlers that processed the event.
    pub fn handler_count(&self) -> usize {
        match self {
            DispatchResult::Disabled => 0,
            DispatchResult::Dispatched {
                direct_results,
                job_specs,
            } => direct_results.len() + job_specs.len(),
        }
    }
}

/// Check if a pattern matches a topic.
///
/// Supports NATS-style wildcards:
/// - `*` matches exactly one segment
/// - `>` matches zero or more segments (must be at end)
fn pattern_matches(pattern: &str, topic: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('.').collect();
    let topic_parts: Vec<&str> = topic.split('.').collect();

    let mut pi = 0;
    let mut ti = 0;

    while pi < pattern_parts.len() {
        let p = pattern_parts[pi];

        if p == ">" {
            // > matches rest of topic
            return true;
        }

        if ti >= topic_parts.len() {
            return false;
        }

        if p == "*" {
            // * matches exactly one segment
            pi += 1;
            ti += 1;
        } else if p == topic_parts[ti] {
            // Exact match
            pi += 1;
            ti += 1;
        } else {
            return false;
        }
    }

    // Must have consumed entire topic
    ti >= topic_parts.len()
}

/// Handle for controlling a running hook service.
pub struct HookServiceHandle {
    /// Cancellation token for shutdown.
    cancel: CancellationToken,

    /// Execution metrics.
    metrics: Arc<ExecutionMetrics>,

    /// Whether the service is enabled.
    enabled: bool,
}

impl HookServiceHandle {
    /// Create a handle for a disabled service.
    pub fn disabled() -> Self {
        Self {
            cancel: CancellationToken::new(),
            metrics: Arc::new(ExecutionMetrics::new()),
            enabled: false,
        }
    }

    /// Create a handle for an enabled service.
    pub fn new(cancel: CancellationToken, metrics: Arc<ExecutionMetrics>) -> Self {
        Self {
            cancel,
            metrics,
            enabled: true,
        }
    }

    /// Check if the service is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get execution metrics.
    pub fn metrics(&self) -> &ExecutionMetrics {
        &self.metrics
    }

    /// Shutdown the service gracefully.
    pub async fn shutdown(self) {
        if !self.enabled {
            return;
        }

        info!("shutting down hook service");
        self.cancel.cancel();
        info!("hook service shutdown complete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExecutionMode;
    use crate::config::HookHandlerConfig;
    use crate::config::HookHandlerType;
    use crate::event::HookEventType;

    #[test]
    fn test_pattern_matching() {
        // Exact match
        assert!(pattern_matches("hooks.kv.write_committed", "hooks.kv.write_committed"));
        assert!(!pattern_matches("hooks.kv.write_committed", "hooks.kv.delete_committed"));

        // Single wildcard
        assert!(pattern_matches("hooks.kv.*", "hooks.kv.write_committed"));
        assert!(pattern_matches("hooks.kv.*", "hooks.kv.delete_committed"));
        assert!(!pattern_matches("hooks.kv.*", "hooks.kv"));
        assert!(!pattern_matches("hooks.kv.*", "hooks.cluster.leader_elected"));

        // Multi wildcard
        assert!(pattern_matches("hooks.>", "hooks"));
        assert!(pattern_matches("hooks.>", "hooks.kv"));
        assert!(pattern_matches("hooks.>", "hooks.kv.write_committed"));
        assert!(pattern_matches("hooks.kv.>", "hooks.kv.write_committed"));
        assert!(!pattern_matches("hooks.kv.>", "hooks.cluster.leader_elected"));

        // Combined
        assert!(pattern_matches("hooks.*.>", "hooks.kv.write_committed"));
        assert!(pattern_matches("hooks.*.>", "hooks.cluster.leader_elected"));
    }

    #[test]
    fn test_disabled_service_handle() {
        let handle = HookServiceHandle::disabled();
        assert!(!handle.is_enabled());
    }

    #[test]
    fn test_config_validation() {
        let config = HooksConfig {
            enabled: true,
            publish_topics: vec!["hooks.>".to_string()],
            handlers: vec![HookHandlerConfig {
                name: "test".to_string(),
                pattern: "hooks.kv.*".to_string(),
                execution_mode: ExecutionMode::Direct,
                handler_type: HookHandlerType::InProcess {
                    handler_id: "test_handler".to_string(),
                },
                timeout_ms: 5000,
                retry_count: 3,
                job_priority: None,
                enabled: true,
            }],
        };

        assert!(config.validate().is_ok());
        assert_eq!(config.enabled_handlers().count(), 1);
    }

    #[test]
    fn test_disabled_handler_not_counted() {
        let config = HooksConfig {
            enabled: true,
            publish_topics: vec![],
            handlers: vec![
                HookHandlerConfig {
                    name: "enabled".to_string(),
                    pattern: "hooks.>".to_string(),
                    execution_mode: ExecutionMode::Direct,
                    handler_type: HookHandlerType::InProcess {
                        handler_id: "test".to_string(),
                    },
                    timeout_ms: 5000,
                    retry_count: 3,
                    job_priority: None,
                    enabled: true,
                },
                HookHandlerConfig {
                    name: "disabled".to_string(),
                    pattern: "hooks.>".to_string(),
                    execution_mode: ExecutionMode::Direct,
                    handler_type: HookHandlerType::InProcess {
                        handler_id: "test".to_string(),
                    },
                    timeout_ms: 5000,
                    retry_count: 3,
                    job_priority: None,
                    enabled: false,
                },
            ],
        };

        assert_eq!(config.enabled_handlers().count(), 1);
    }

    #[tokio::test]
    async fn test_service_dispatch_disabled() {
        let config = HooksConfig {
            enabled: false,
            publish_topics: vec![],
            handlers: vec![],
        };

        let service = HookService::new(config);
        let event = HookEvent::new(HookEventType::WriteCommitted, 1, serde_json::json!({}));

        let result = service.dispatch(&event).await.unwrap();
        assert!(matches!(result, DispatchResult::Disabled));
    }

    #[tokio::test]
    async fn test_service_dispatch_no_matching_handlers() {
        let config = HooksConfig {
            enabled: true,
            publish_topics: vec![],
            handlers: vec![HookHandlerConfig {
                name: "test".to_string(),
                pattern: "hooks.cluster.*".to_string(), // Won't match KV events
                execution_mode: ExecutionMode::Direct,
                handler_type: HookHandlerType::Shell {
                    command: "exit 0".to_string(),
                    working_dir: None,
                },
                timeout_ms: 5000,
                retry_count: 3,
                job_priority: None,
                enabled: true,
            }],
        };

        let service = HookService::new(config);
        let event = HookEvent::new(HookEventType::WriteCommitted, 1, serde_json::json!({}));

        let result = service.dispatch(&event).await.unwrap();
        assert_eq!(result.handler_count(), 0);
    }

    #[tokio::test]
    async fn test_service_dispatch_with_matching_handler() {
        let config = HooksConfig {
            enabled: true,
            publish_topics: vec![],
            handlers: vec![HookHandlerConfig {
                name: "test".to_string(),
                pattern: "hooks.kv.*".to_string(),
                execution_mode: ExecutionMode::Direct,
                handler_type: HookHandlerType::Shell {
                    command: "exit 0".to_string(),
                    working_dir: None,
                },
                timeout_ms: 5000,
                retry_count: 3,
                job_priority: None,
                enabled: true,
            }],
        };

        let service = HookService::new(config);
        let event = HookEvent::new(HookEventType::WriteCommitted, 1, serde_json::json!({}));

        let result = service.dispatch(&event).await.unwrap();
        assert_eq!(result.handler_count(), 1);
        assert!(!result.has_failures());
    }
}
