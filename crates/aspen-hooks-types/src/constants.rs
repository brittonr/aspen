//! Tiger Style constants for the hook system.
//!
//! All resource limits are explicitly bounded to prevent unbounded resource
//! consumption and ensure predictable behavior under load.

use std::time::Duration;

// Handler limits
/// Maximum number of handlers that can be registered.
pub const MAX_HANDLERS: usize = 64;

/// Maximum length of a handler name.
pub const MAX_HANDLER_NAME_SIZE: usize = 128;

/// Maximum length of a topic pattern for subscriptions.
pub const MAX_PATTERN_SIZE: usize = 256;

/// Maximum length of a shell command.
pub const MAX_SHELL_COMMAND_SIZE: usize = 4096;

// Execution limits
/// Maximum number of concurrent handler executions across all handlers.
pub const MAX_CONCURRENT_HANDLER_EXECUTIONS: usize = 32;

/// Maximum timeout for a single handler execution (milliseconds).
pub const MAX_HANDLER_TIMEOUT_MS: u64 = 30_000;

/// Default timeout for handler execution (milliseconds).
pub const DEFAULT_HANDLER_TIMEOUT_MS: u64 = 5_000;

/// Maximum number of retries for a handler execution.
pub const MAX_HANDLER_RETRY_COUNT: u32 = 5;

/// Default number of retries for handler execution.
pub const DEFAULT_HANDLER_RETRY_COUNT: u32 = 3;

// Retry backoff
/// Initial backoff delay for retry (milliseconds).
pub const RETRY_INITIAL_BACKOFF_MS: u64 = 100;

/// Maximum backoff delay for retry (milliseconds).
pub const RETRY_MAX_BACKOFF_MS: u64 = 10_000;

/// Backoff multiplier for exponential backoff.
pub const RETRY_BACKOFF_MULTIPLIER: f64 = 2.0;

// Shell specific
/// Maximum output size captured from shell commands (bytes).
pub const MAX_SHELL_OUTPUT_SIZE: usize = 64 * 1024;

/// Grace period for shell process termination before SIGKILL.
pub const SHELL_GRACE_PERIOD: Duration = Duration::from_secs(5);

/// Environment variable name for passing hook event data to shell commands.
pub const HOOK_EVENT_ENV_VAR: &str = "ASPEN_HOOK_EVENT";

/// Environment variable name for the event topic.
pub const HOOK_TOPIC_ENV_VAR: &str = "ASPEN_HOOK_TOPIC";

/// Environment variable name for the event type.
pub const HOOK_EVENT_TYPE_ENV_VAR: &str = "ASPEN_HOOK_EVENT_TYPE";

// Topic prefixes (reserved namespace)
/// Prefix for all hook event topics.
pub const HOOK_TOPIC_PREFIX: &str = "hooks";

/// Separator used in topic hierarchies.
pub const TOPIC_SEPARATOR: char = '.';

// Forward handler limits
/// Maximum number of forward handler connections.
pub const MAX_FORWARD_CONNECTIONS: usize = 32;

/// Timeout for establishing forward connections (milliseconds).
pub const FORWARD_CONNECT_TIMEOUT_MS: u64 = 5_000;

// Metrics
/// Interval for aggregating metrics (milliseconds).
pub const METRICS_AGGREGATION_INTERVAL_MS: u64 = 1_000;

/// Maximum number of metrics samples to keep in memory.
pub const MAX_METRICS_SAMPLES: usize = 1_000;

// Job-based execution
/// Default job priority for hook executions.
pub const DEFAULT_JOB_PRIORITY: &str = "normal";

/// Job type identifier for hook jobs.
pub const HOOK_JOB_TYPE: &str = "hook";

// Compile-time assertions for sanity
const _: () = {
    assert!(MAX_HANDLERS >= 1);
    assert!(MAX_HANDLERS <= 256);
    assert!(MAX_CONCURRENT_HANDLER_EXECUTIONS >= 1);
    assert!(MAX_CONCURRENT_HANDLER_EXECUTIONS <= 128);
    assert!(DEFAULT_HANDLER_TIMEOUT_MS <= MAX_HANDLER_TIMEOUT_MS);
    assert!(DEFAULT_HANDLER_RETRY_COUNT <= MAX_HANDLER_RETRY_COUNT);
    assert!(RETRY_INITIAL_BACKOFF_MS < RETRY_MAX_BACKOFF_MS);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_are_reasonable() {
        // Verify timeouts make sense
        assert!(Duration::from_millis(DEFAULT_HANDLER_TIMEOUT_MS) > Duration::from_millis(100));
        assert!(Duration::from_millis(MAX_HANDLER_TIMEOUT_MS) < Duration::from_secs(60));

        // Verify shell limits
        assert!(MAX_SHELL_OUTPUT_SIZE >= 1024);
        assert!(MAX_SHELL_COMMAND_SIZE >= 256);

        // Verify handler limits
        assert!(MAX_HANDLER_NAME_SIZE >= 16);
        assert!(MAX_PATTERN_SIZE >= 32);
    }

    #[test]
    fn test_topic_prefix() {
        assert!(!HOOK_TOPIC_PREFIX.is_empty());
        assert!(!HOOK_TOPIC_PREFIX.contains(TOPIC_SEPARATOR));
    }
}
