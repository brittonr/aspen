//! Error types for the hook type system.
//!
//! Uses snafu for structured error handling with context.
//! This module contains only errors related to configuration and type validation,
//! not runtime errors (which remain in aspen-hooks).

use snafu::Snafu;

/// Result type for hook type operations.
pub type Result<T, E = HookTypeError> = std::result::Result<T, E>;

/// Errors that can occur in hook configuration and type validation.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum HookTypeError {
    /// Handler name exceeds maximum length.
    #[snafu(display("handler name too long: {} bytes (max {})", length, max))]
    HandlerNameTooLong {
        /// Actual length of the handler name.
        length: u32,
        /// Maximum allowed length.
        max: u32,
    },

    /// Handler name is empty.
    #[snafu(display("handler name cannot be empty"))]
    HandlerNameEmpty,

    /// Too many handlers registered.
    #[snafu(display("too many handlers: {} (max {})", count, max))]
    TooManyHandlers {
        /// Current number of handlers.
        count: u32,
        /// Maximum allowed handlers.
        max: u32,
    },

    /// Topic pattern exceeds maximum length.
    #[snafu(display("pattern too long: {} bytes (max {})", length, max))]
    PatternTooLong {
        /// Actual length of the pattern.
        length: u32,
        /// Maximum allowed length.
        max: u32,
    },

    /// Shell command exceeds maximum length.
    #[snafu(display("shell command too long: {} bytes (max {})", length, max))]
    ShellCommandTooLong {
        /// Actual length of the command.
        length: u32,
        /// Maximum allowed length.
        max: u32,
    },

    /// Shell command is empty.
    #[snafu(display("shell command cannot be empty"))]
    ShellCommandEmpty,

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
}

impl From<serde_json::Error> for HookTypeError {
    fn from(source: serde_json::Error) -> Self {
        HookTypeError::SerializationFailed { source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = HookTypeError::HandlerNameTooLong { length: 200, max: 128 };
        assert!(err.to_string().contains("200"));
        assert!(err.to_string().contains("128"));

        let err = HookTypeError::ConfigInvalid {
            message: "test error".to_string(),
        };
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_result_type() {
        fn returns_ok() -> Result<u32> {
            Ok(42)
        }

        fn returns_err() -> Result<u32> {
            Err(HookTypeError::HandlerNameEmpty)
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }
}
