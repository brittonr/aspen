//! Error types for Automerge document operations.
//!
//! Provides specific error types for document management and sync operations.

use snafu::Snafu;

/// Errors that can occur during Automerge document operations.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum AutomergeError {
    /// Document not found.
    #[snafu(display("document not found: {document_id}"))]
    DocumentNotFound {
        /// The document ID that was not found.
        document_id: String,
    },

    /// Invalid document ID format.
    #[snafu(display("invalid document ID: {reason}"))]
    InvalidDocumentId {
        /// Description of why the ID is invalid.
        reason: String,
    },

    /// Document already exists.
    #[snafu(display("document already exists: {document_id}"))]
    DocumentAlreadyExists {
        /// The document ID that already exists.
        document_id: String,
    },

    /// Document exceeds maximum size.
    #[snafu(display("document size {size} exceeds maximum {max}"))]
    DocumentTooLarge {
        /// Actual size of the document in bytes.
        size: usize,
        /// Maximum allowed size in bytes.
        max: usize,
    },

    /// Change exceeds maximum size.
    #[snafu(display("change size {size} exceeds maximum {max}"))]
    ChangeTooLarge {
        /// Actual size of the change in bytes.
        size: usize,
        /// Maximum allowed size in bytes.
        max: usize,
    },

    /// Too many changes in batch.
    #[snafu(display("batch contains {count} changes, maximum is {max}"))]
    TooManyChanges {
        /// Number of changes in the batch.
        count: usize,
        /// Maximum allowed changes.
        max: usize,
    },

    /// Invalid change data.
    #[snafu(display("invalid change: {reason}"))]
    InvalidChange {
        /// Description of why the change is invalid.
        reason: String,
    },

    /// Merge conflict that could not be resolved.
    #[snafu(display("merge conflict: {reason}"))]
    MergeConflict {
        /// Description of the conflict.
        reason: String,
    },

    /// Automerge library error.
    #[snafu(display("automerge error: {message}"))]
    AutomergeLib {
        /// Human-readable description of the error.
        message: String,
    },

    /// Serialization error.
    #[snafu(display("serialization error: {reason}"))]
    Serialization {
        /// Description of the serialization failure.
        reason: String,
    },

    /// Storage operation failed.
    #[snafu(display("storage error: {reason}"))]
    Storage {
        /// Description of the storage failure.
        reason: String,
    },

    /// Namespace not found.
    #[snafu(display("namespace not found: {namespace}"))]
    NamespaceNotFound {
        /// The namespace that was not found.
        namespace: String,
    },

    /// Maximum documents per namespace exceeded.
    #[snafu(display("namespace {namespace} has reached maximum documents ({max})"))]
    MaxDocumentsExceeded {
        /// The namespace that exceeded the limit.
        namespace: String,
        /// Maximum allowed documents.
        max: u32,
    },

    /// Sync operation failed.
    #[snafu(display("sync error: {reason}"))]
    SyncFailed {
        /// Description of the sync failure.
        reason: String,
    },

    /// Sync operation timed out.
    #[snafu(display("sync timed out after {duration_ms}ms"))]
    SyncTimeout {
        /// Duration in milliseconds before timeout.
        duration_ms: u64,
    },

    /// Operation requires write access.
    #[snafu(display("write access required for operation: {operation}"))]
    WriteAccessRequired {
        /// The operation that requires write access.
        operation: String,
    },

    /// Invalid actor ID.
    #[snafu(display("invalid actor ID: {reason}"))]
    InvalidActorId {
        /// Description of why the actor ID is invalid.
        reason: String,
    },
}

/// Result type for Automerge document operations.
pub type AutomergeResult<T> = Result<T, AutomergeError>;

impl From<serde_json::Error> for AutomergeError {
    fn from(err: serde_json::Error) -> Self {
        AutomergeError::Serialization {
            reason: err.to_string(),
        }
    }
}

impl From<aspen_core::KeyValueStoreError> for AutomergeError {
    fn from(err: aspen_core::KeyValueStoreError) -> Self {
        AutomergeError::Storage {
            reason: err.to_string(),
        }
    }
}

impl From<automerge::AutomergeError> for AutomergeError {
    fn from(err: automerge::AutomergeError) -> Self {
        AutomergeError::AutomergeLib {
            message: err.to_string(),
        }
    }
}

impl From<automerge::LoadChangeError> for AutomergeError {
    fn from(err: automerge::LoadChangeError) -> Self {
        AutomergeError::InvalidChange {
            reason: err.to_string(),
        }
    }
}
