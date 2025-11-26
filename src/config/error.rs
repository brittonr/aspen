//! Configuration error types

/// Configuration error types
#[derive(Debug)]
pub enum ConfigError {
    /// A configuration value is invalid
    InvalidValue {
        key: String,
        value: String,
        reason: String,
    },
    /// A required configuration value is missing
    MissingRequired {
        key: String,
        hint: String,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidValue { key, value, reason } => {
                write!(f, "Invalid configuration for {}: '{}' ({})", key, value, reason)
            }
            ConfigError::MissingRequired { key, hint } => {
                write!(f, "Missing required configuration: {} ({})", key, hint)
            }
        }
    }
}

impl std::error::Error for ConfigError {}
