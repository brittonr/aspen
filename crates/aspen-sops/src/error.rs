//! Error types for SOPS operations.

use std::path::PathBuf;

use snafu::Snafu;

/// Errors that can occur during SOPS operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum SopsError {
    /// Failed to connect to Aspen Transit.
    #[snafu(display(
        "failed to connect to Aspen Transit: {reason}\n\
         Hint: Check cluster ticket and ensure the cluster is running"
    ))]
    TransitConnect { reason: String },

    /// Transit encrypt operation failed.
    #[snafu(display("Transit encrypt failed for key '{key_name}': {reason}"))]
    TransitEncrypt { key_name: String, reason: String },

    /// Transit decrypt operation failed.
    #[snafu(display(
        "Transit decrypt failed for key '{key_name}': {reason}\n\
         Hint: Check that the Transit key exists and has not been rotated \
         past min_decryption_version"
    ))]
    TransitDecrypt { key_name: String, reason: String },

    /// Failed to read file.
    #[snafu(display("failed to read file at {}: {source}", path.display()))]
    FileRead { path: PathBuf, source: std::io::Error },

    /// Failed to write file.
    #[snafu(display("failed to write file at {}: {source}", path.display()))]
    FileWrite { path: PathBuf, source: std::io::Error },

    /// File exceeds maximum size.
    #[snafu(display(
        "file at {} is too large: {size_bytes} bytes (max: {max_bytes} bytes)",
        path.display()
    ))]
    FileTooLarge {
        path: PathBuf,
        size_bytes: u64,
        max_bytes: u64,
    },

    /// Failed to parse file.
    #[snafu(display("failed to parse file at {}: {reason}", path.display()))]
    ParseFile { path: PathBuf, reason: String },

    /// Unsupported or invalid file format.
    #[snafu(display(
        "unsupported file format: {reason}\n\
         Hint: Only TOML files are supported in v1. \
         YAML/JSON support is planned."
    ))]
    InvalidFormat { reason: String },

    /// MAC verification failed — file may have been tampered with.
    #[snafu(display(
        "MAC verification failed: file integrity check failed\n\
         Hint: The file may have been tampered with, or the data key \
         does not match. Re-encrypt the file to fix."
    ))]
    MacVerificationFailed,

    /// No matching key group found in SOPS metadata.
    #[snafu(display(
        "no matching key group found in SOPS metadata\n\
         Hint: The file has no aspen_transit or age key groups that \
         this identity can decrypt. Check --cluster-ticket or age identity."
    ))]
    NoMatchingKeyGroup,

    /// Editor process failed.
    #[snafu(display("editor process failed: {reason}"))]
    EditorFailed { reason: String },

    /// Failed to bind key service socket.
    #[snafu(display(
        "failed to bind key service at {path}: {reason}\n\
         Hint: Check that the socket path is writable and not in use"
    ))]
    KeyServiceBind { path: String, reason: String },

    /// Value encryption failed.
    #[snafu(display("failed to encrypt value at path '{key_path}': {reason}"))]
    ValueEncrypt { key_path: String, reason: String },

    /// Value decryption failed.
    #[snafu(display("failed to decrypt value at path '{key_path}': {reason}"))]
    ValueDecrypt { key_path: String, reason: String },

    /// Invalid ciphertext format.
    #[snafu(display(
        "invalid ciphertext format: {reason}\n\
         Hint: Expected ENC[AES256_GCM,data:...,iv:...,tag:...,type:...]"
    ))]
    InvalidCiphertext { reason: String },

    /// Serialization error.
    #[snafu(display("serialization error: {reason}"))]
    Serialization { reason: String },

    /// Too many values in file.
    #[snafu(display("too many values in file: {count} (max: {max})"))]
    TooManyValues { count: u32, max: u32 },

    /// Key path too long.
    #[snafu(display("key path too long: {length} bytes (max: {max})"))]
    KeyPathTooLong { length: u32, max: u32 },

    /// Age encryption/decryption error.
    #[cfg(feature = "age-fallback")]
    #[snafu(display("age error: {reason}"))]
    AgeError { reason: String },

    /// SOPS metadata missing or invalid.
    #[snafu(display(
        "SOPS metadata missing or invalid: {reason}\n\
         Hint: Ensure the file was encrypted with aspen-sops or sops"
    ))]
    InvalidMetadata { reason: String },
}

/// Result type for SOPS operations.
pub type Result<T> = std::result::Result<T, SopsError>;
