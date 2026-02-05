//! Error types for the hook system.
//!
//! Uses snafu for structured error handling with context.

use snafu::Snafu;

/// Result type for hook operations.
pub type Result<T, E = HookError> = std::result::Result<T, E>;

/// Errors that can occur in the hook system.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum HookError {
    /// Handler name exceeds maximum length.
    #[snafu(display("handler name too long: {} bytes (max {})", length, max))]
    HandlerNameTooLong {
        /// Actual length of the handler name.
        length: usize,
        /// Maximum allowed length.
        max: usize,
    },

    /// Handler name is empty.
    #[snafu(display("handler name cannot be empty"))]
    HandlerNameEmpty,

    /// Too many handlers registered.
    #[snafu(display("too many handlers: {} (max {})", count, max))]
    TooManyHandlers {
        /// Current number of handlers.
        count: usize,
        /// Maximum allowed handlers.
        max: usize,
    },

    /// Handler with this name already exists.
    #[snafu(display("handler already exists: {}", name))]
    HandlerAlreadyExists {
        /// Name of the duplicate handler.
        name: String,
    },

    /// Handler not found.
    #[snafu(display("handler not found: {}", name))]
    HandlerNotFound {
        /// Name of the missing handler.
        name: String,
    },

    /// Topic pattern is invalid.
    #[snafu(display("invalid topic pattern: {}", reason))]
    InvalidPattern {
        /// Reason the pattern is invalid.
        reason: String,
    },

    /// Topic pattern exceeds maximum length.
    #[snafu(display("pattern too long: {} bytes (max {})", length, max))]
    PatternTooLong {
        /// Actual length of the pattern.
        length: usize,
        /// Maximum allowed length.
        max: usize,
    },

    /// Shell command exceeds maximum length.
    #[snafu(display("shell command too long: {} bytes (max {})", length, max))]
    ShellCommandTooLong {
        /// Actual length of the command.
        length: usize,
        /// Maximum allowed length.
        max: usize,
    },

    /// Shell command is empty.
    #[snafu(display("shell command cannot be empty"))]
    ShellCommandEmpty,

    /// Handler execution timed out.
    #[snafu(display("handler execution timed out after {} ms", timeout_ms))]
    ExecutionTimeout {
        /// Timeout that was exceeded.
        timeout_ms: u64,
    },

    /// Handler execution failed.
    #[snafu(display("handler execution failed: {}", message))]
    ExecutionFailed {
        /// Description of the failure.
        message: String,
    },

    /// Shell command execution failed.
    #[snafu(display("shell command failed with exit code {}: {}", exit_code, stderr))]
    ShellExecutionFailed {
        /// Exit code of the shell command.
        exit_code: i32,
        /// Standard error output.
        stderr: String,
    },

    /// Shell command was terminated by signal.
    #[snafu(display("shell command terminated by signal"))]
    ShellTerminated,

    /// Failed to spawn shell process.
    #[snafu(display("failed to spawn shell process: {}", source))]
    ShellSpawnFailed {
        /// Underlying IO error.
        source: std::io::Error,
    },

    /// Maximum retries exceeded.
    #[snafu(display("maximum retries exceeded ({} attempts)", attempts))]
    MaxRetriesExceeded {
        /// Number of attempts made.
        attempts: u32,
    },

    /// Invalid timeout value.
    #[snafu(display("invalid timeout: {} ms (max {} ms)", value, max))]
    InvalidTimeout {
        /// Provided timeout value.
        value: u64,
        /// Maximum allowed timeout.
        max: u64,
    },

    /// Invalid retry count.
    #[snafu(display("invalid retry count: {} (max {})", value, max))]
    InvalidRetryCount {
        /// Provided retry count.
        value: u32,
        /// Maximum allowed retries.
        max: u32,
    },

    /// Service is not running.
    #[snafu(display("hook service is not running"))]
    ServiceNotRunning,

    /// Service is already running.
    #[snafu(display("hook service is already running"))]
    ServiceAlreadyRunning,

    /// Service shutdown failed.
    #[snafu(display("service shutdown failed: {}", message))]
    ShutdownFailed {
        /// Description of the failure.
        message: String,
    },

    /// Failed to publish event.
    #[snafu(display("failed to publish event: {}", source))]
    PublishFailed {
        /// Underlying pub/sub error.
        source: crate::pubsub::error::PubSubError,
    },

    /// Failed to subscribe to topics.
    #[snafu(display("failed to subscribe: {}", source))]
    SubscribeFailed {
        /// Underlying pub/sub error.
        source: crate::pubsub::error::PubSubError,
    },

    /// Failed to submit job.
    #[snafu(display("failed to submit job: {}", message))]
    JobSubmitFailed {
        /// Description of the failure.
        message: String,
    },

    /// Configuration validation failed.
    #[snafu(display("configuration invalid: {}", message))]
    ConfigInvalid {
        /// Description of the validation failure.
        message: String,
    },

    /// Serialization failed.
    #[snafu(display("serialization failed: {}", source))]
    SerializationFailed {
        /// Underlying JSON error.
        source: serde_json::Error,
    },

    /// Deserialization failed.
    #[snafu(display("deserialization failed: {}", source))]
    DeserializationFailed {
        /// Underlying JSON error.
        source: serde_json::Error,
    },

    /// Forward handler connection failed.
    #[snafu(display("forward connection failed: {}", message))]
    ForwardConnectionFailed {
        /// Description of the failure.
        message: String,
    },

    /// Concurrency limit reached.
    #[snafu(display("concurrency limit reached: {} concurrent executions", limit))]
    ConcurrencyLimitReached {
        /// Current concurrency limit.
        limit: usize,
    },
}

impl From<crate::pubsub::error::PubSubError> for HookError {
    fn from(source: crate::pubsub::error::PubSubError) -> Self {
        HookError::PublishFailed { source }
    }
}

impl From<serde_json::Error> for HookError {
    fn from(source: serde_json::Error) -> Self {
        HookError::SerializationFailed { source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = HookError::HandlerNameTooLong { length: 200, max: 128 };
        assert!(err.to_string().contains("200"));
        assert!(err.to_string().contains("128"));

        let err = HookError::ExecutionTimeout { timeout_ms: 5000 };
        assert!(err.to_string().contains("5000"));

        let err = HookError::ShellExecutionFailed {
            exit_code: 1,
            stderr: "error".to_string(),
        };
        assert!(err.to_string().contains("exit code 1"));
    }

    #[test]
    fn test_result_type() {
        fn returns_ok() -> Result<u32> {
            Ok(42)
        }

        fn returns_err() -> Result<u32> {
            Err(HookError::HandlerNameEmpty)
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }
}
