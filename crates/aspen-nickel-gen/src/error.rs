//! Error types for the Nickel contract generator.

use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur during schema generation.
#[derive(Debug, Error)]
pub enum GeneratorError {
    /// Failed to read source file.
    #[error("failed to read file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse Rust source code.
    #[error("failed to parse Rust source: {message}")]
    ParseError { message: String },

    /// Failed to write output file.
    #[error("failed to write output file {path}: {source}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Unsupported type encountered.
    #[error("unsupported type: {type_name}")]
    UnsupportedType { type_name: String },

    /// No configuration structs found.
    #[error("no configuration structs found in source file")]
    NoConfigStructs,
}
