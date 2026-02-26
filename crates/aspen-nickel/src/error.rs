//! Error types for Nickel configuration loading.

use std::io;
use std::path::PathBuf;

use snafu::Snafu;

/// Errors that can occur when loading Nickel configuration files.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum NickelConfigError {
    /// Nickel evaluation failed.
    ///
    /// This includes syntax errors, type errors, and contract violations.
    #[snafu(display("Nickel evaluation failed: {message}"))]
    Evaluation {
        /// The error message from Nickel
        message: String,
    },

    /// Failed to deserialize Nickel value to Rust type.
    #[snafu(display("Failed to deserialize config: {message}"))]
    Deserialization {
        /// The deserialization error message
        message: String,
    },

    /// Configuration file is too large.
    ///
    /// Tiger Style: Bounded resources prevent DoS attacks.
    #[snafu(display("Configuration file too large: {size} bytes (max {max} bytes)"))]
    FileTooLarge {
        /// Actual file size in bytes
        size: u64,
        /// Maximum allowed size in bytes
        max: u64,
    },

    /// Failed to read configuration file.
    #[snafu(display("Failed to read config file {}: {source}", path.display()))]
    ReadFile {
        /// Path to the file that failed to read
        path: PathBuf,
        /// The underlying I/O error
        source: io::Error,
    },

    /// Configuration file not found.
    #[snafu(display("Configuration file not found: {}", path.display()))]
    FileNotFound {
        /// Path to the missing file
        path: PathBuf,
    },

    /// Invalid configuration value.
    #[snafu(display("Invalid configuration value for '{field}': {message}"))]
    InvalidValue {
        /// The field name with the invalid value
        field: String,
        /// Description of why the value is invalid
        message: String,
    },
}

impl From<nickel_lang::Error> for NickelConfigError {
    fn from(err: nickel_lang::Error) -> Self {
        // Format the error to a string for display
        let mut message = Vec::new();
        if err.format(&mut message, nickel_lang::ErrorFormat::Text).is_ok()
            && let Ok(msg) = String::from_utf8(message)
        {
            return NickelConfigError::Evaluation { message: msg };
        }
        // Fallback to debug format
        NickelConfigError::Evaluation {
            message: format!("{err:?}"),
        }
    }
}
