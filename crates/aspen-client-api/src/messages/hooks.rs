//! Hook response types.
//!
//! Response types for event-driven automation hook operations.

use serde::{Deserialize, Serialize};

/// Information about a configured hook handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookHandlerInfo {
    /// Handler name (unique identifier).
    pub name: String,
    /// Topic pattern this handler subscribes to (NATS-style wildcards).
    pub pattern: String,
    /// Handler type: "in_process", "shell", or "forward".
    pub handler_type: String,
    /// Execution mode: "direct" or "job".
    pub execution_mode: String,
    /// Whether the handler is enabled.
    pub enabled: bool,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Number of retries on failure.
    pub retry_count: u32,
}

/// Hook list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookListResultResponse {
    /// Whether the hook service is enabled.
    pub enabled: bool,
    /// List of configured handlers.
    pub handlers: Vec<HookHandlerInfo>,
}

/// Metrics for a single hook handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookHandlerMetrics {
    /// Handler name.
    pub name: String,
    /// Total successful executions.
    pub success_count: u64,
    /// Total failed executions.
    pub failure_count: u64,
    /// Total dropped events (due to concurrency limit).
    pub dropped_count: u64,
    /// Total jobs submitted (for job mode handlers).
    pub jobs_submitted: u64,
    /// Average execution duration in microseconds.
    pub avg_duration_us: u64,
    /// Maximum execution duration in microseconds.
    pub max_duration_us: u64,
}

/// Hook metrics result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMetricsResultResponse {
    /// Whether the hook service is enabled.
    pub enabled: bool,
    /// Global metrics (all handlers).
    pub total_events_processed: u64,
    /// Per-handler metrics.
    pub handlers: Vec<HookHandlerMetrics>,
}

/// Hook trigger result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookTriggerResultResponse {
    /// Whether the trigger was successful.
    pub success: bool,
    /// Number of handlers that matched and were dispatched to.
    pub dispatched_count: usize,
    /// Error message if the operation failed.
    pub error: Option<String>,
    /// Any handler failures (handler_name -> error message).
    pub handler_failures: Vec<(String, String)>,
}
