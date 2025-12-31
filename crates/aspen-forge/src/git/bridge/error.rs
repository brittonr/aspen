//! Error types for the Git bridge module.
//!
//! Uses `snafu` for structured error handling with context.

use snafu::Snafu;

use crate::ForgeError;

/// Errors that can occur during Git bridge operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum BridgeError {
    // ========================================================================
    // Hash Errors
    // ========================================================================
    /// Hash mapping not found.
    #[snafu(display("hash mapping not found: {hash}"))]
    MappingNotFound { hash: String },

    /// Invalid hash length.
    #[snafu(display("invalid hash length: expected {expected}, got {actual}"))]
    InvalidHashLength { expected: usize, actual: usize },

    /// Invalid hex encoding.
    #[snafu(display("invalid hex encoding: {message}"))]
    InvalidHexEncoding { message: String },

    // ========================================================================
    // Object Errors
    // ========================================================================
    /// Object not found in storage.
    #[snafu(display("object not found: {hash}"))]
    ObjectNotFound { hash: String },

    /// Malformed git object.
    #[snafu(display("malformed git object: {message}"))]
    MalformedObject { message: String },

    /// Unknown git object type.
    #[snafu(display("unknown git object type: {type_str}"))]
    UnknownObjectType { type_str: String },

    /// Object conversion failed.
    #[snafu(display("object conversion failed: {message}"))]
    ConversionFailed { message: String },

    // ========================================================================
    // Tree Errors
    // ========================================================================
    /// Malformed tree entry.
    #[snafu(display("malformed tree entry: {message}"))]
    MalformedTreeEntry { message: String },

    /// Invalid tree entry mode.
    #[snafu(display("invalid tree entry mode: {mode}"))]
    InvalidTreeMode { mode: String },

    // ========================================================================
    // Commit Errors
    // ========================================================================
    /// Malformed commit.
    #[snafu(display("malformed commit: {message}"))]
    MalformedCommit { message: String },

    /// Invalid author/committer line.
    #[snafu(display("invalid author line: {message}"))]
    InvalidAuthorLine { message: String },

    // ========================================================================
    // DAG Errors
    // ========================================================================
    /// Cycle detected in object graph.
    #[snafu(display("cycle detected in object graph"))]
    CycleDetected,

    /// DAG traversal depth exceeded.
    #[snafu(display("DAG traversal depth exceeded: {depth} > {max}"))]
    DepthExceeded { depth: usize, max: usize },

    // ========================================================================
    // Batch Limits (Tiger Style)
    // ========================================================================
    /// Import batch size exceeded.
    #[snafu(display("import batch size exceeded: {count} > {max}"))]
    ImportBatchExceeded { count: usize, max: usize },

    /// Push object count exceeded.
    #[snafu(display("push would include too many objects: {count} > {max}"))]
    PushTooLarge { count: usize, max: usize },

    // ========================================================================
    // Ref Errors
    // ========================================================================
    /// Ref not found.
    #[snafu(display("ref not found: {ref_name}"))]
    RefNotFound { ref_name: String },

    /// Invalid ref specification.
    #[snafu(display("invalid refspec: {refspec}"))]
    InvalidRefspec { refspec: String },

    /// Push rejected (non-fast-forward without force).
    #[snafu(display("push rejected: non-fast-forward update to {ref_name}"))]
    NonFastForward { ref_name: String },

    // ========================================================================
    // Protocol Errors
    // ========================================================================
    /// Invalid remote helper command.
    #[snafu(display("invalid helper command: {command}"))]
    InvalidCommand { command: String },

    /// Invalid URL format.
    #[snafu(display("invalid URL: {url}"))]
    InvalidUrl { url: String },

    /// Remote helper protocol error.
    #[snafu(display("protocol error: {message}"))]
    ProtocolError { message: String },

    // ========================================================================
    // Authentication Errors
    // ========================================================================
    /// Authentication failed.
    #[snafu(display("authentication failed: {message}"))]
    AuthenticationFailed { message: String },

    /// Credentials not found.
    #[snafu(display("credentials not found for {url}"))]
    CredentialsNotFound { url: String },

    // ========================================================================
    // Network Errors
    // ========================================================================
    /// Network operation timed out.
    #[snafu(display("operation timed out: {operation}"))]
    Timeout { operation: String },

    /// Network connection failed.
    #[snafu(display("connection failed: {message}"))]
    ConnectionFailed { message: String },

    /// Remote returned an error.
    #[snafu(display("remote error: {message}"))]
    RemoteError { message: String },

    // ========================================================================
    // Storage Errors
    // ========================================================================
    /// Blob storage error.
    #[snafu(display("blob storage error: {message}"))]
    BlobStorage { message: String },

    /// KV storage error.
    #[snafu(display("KV storage error: {message}"))]
    KvStorage { message: String },

    /// Serialization error.
    #[snafu(display("serialization error: {message}"))]
    Serialization { message: String },

    // ========================================================================
    // Wrapped Errors
    // ========================================================================
    /// Underlying Forge error.
    #[snafu(display("forge error: {source}"))]
    Forge { source: ForgeError },
}

impl From<ForgeError> for BridgeError {
    fn from(e: ForgeError) -> Self {
        BridgeError::Forge { source: e }
    }
}

impl From<postcard::Error> for BridgeError {
    fn from(e: postcard::Error) -> Self {
        BridgeError::Serialization {
            message: e.to_string(),
        }
    }
}

impl From<std::string::FromUtf8Error> for BridgeError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        BridgeError::MalformedObject {
            message: format!("invalid UTF-8: {e}"),
        }
    }
}

impl From<std::str::Utf8Error> for BridgeError {
    fn from(e: std::str::Utf8Error) -> Self {
        BridgeError::MalformedObject {
            message: format!("invalid UTF-8: {e}"),
        }
    }
}

impl From<std::num::ParseIntError> for BridgeError {
    fn from(e: std::num::ParseIntError) -> Self {
        BridgeError::MalformedObject {
            message: format!("invalid number: {e}"),
        }
    }
}

impl From<std::io::Error> for BridgeError {
    fn from(e: std::io::Error) -> Self {
        BridgeError::ConnectionFailed {
            message: e.to_string(),
        }
    }
}

/// Result type alias for bridge operations.
pub type BridgeResult<T> = Result<T, BridgeError>;
