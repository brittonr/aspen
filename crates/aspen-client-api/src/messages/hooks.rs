//! Hook operation types.
//!
//! Request/response types for event-driven automation hook operations.

use serde::Deserialize;
use serde::Serialize;

/// Hooks domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HooksRequest {
    /// List configured hook handlers.
    HookList,
    /// Get hook execution metrics.
    HookGetMetrics { handler_name: Option<String> },
    /// Manually trigger a hook event for testing.
    HookTrigger { event_type: String, payload_json: String },
}

impl HooksRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::HookList | Self::HookGetMetrics { .. } => Some(Operation::Read {
                key: "_hooks:".to_string(),
            }),
            Self::HookTrigger { .. } => Some(Operation::Write {
                key: "_hooks:".to_string(),
                value: vec![],
            }),
        }
    }
}

/// Information about a configured hook handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookHandlerInfo {
    /// Handler name (unique identifier).
    pub name: String,
    /// Topic pattern this handler subscribes to.
    pub pattern: String,
    /// Handler type: "in_process", "shell", or "forward".
    pub handler_type: String,
    /// Execution mode: "direct" or "job".
    pub execution_mode: String,
    /// Whether the handler is enabled.
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Number of retries on failure.
    pub retry_count: u32,
}

/// Hook list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookListResultResponse {
    /// Whether the hook service is enabled.
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
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
    /// Total dropped events.
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
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    /// Global metrics (all handlers).
    pub total_events_processed: u64,
    /// Per-handler metrics.
    pub handlers: Vec<HookHandlerMetrics>,
}

/// Hook trigger result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookTriggerResultResponse {
    /// Whether the trigger was successful.
    pub is_success: bool,
    /// Number of handlers that matched and were dispatched to.
    pub dispatched_count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
    /// Any handler failures (handler_name -> error message).
    pub handler_failures: Vec<(String, String)>,
}
