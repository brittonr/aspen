/// S3-specific error types following Tiger Style error handling principles.
///
/// Uses snafu for context-rich errors.
/// Fail fast on invalid operations.
use snafu::Snafu;

/// S3 operation errors.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum S3Error {
    /// Bucket already exists.
    #[snafu(display("Bucket '{}' already exists", name))]
    BucketAlreadyExists { name: String },

    /// Bucket not found.
    #[snafu(display("Bucket '{}' not found", name))]
    BucketNotFound { name: String },

    /// Bucket is not empty (cannot delete).
    #[snafu(display("Bucket '{}' is not empty", name))]
    BucketNotEmpty { name: String },

    /// Invalid bucket name.
    #[snafu(display("Invalid bucket name '{}': {}", name, reason))]
    InvalidBucketName { name: String, reason: String },

    /// Object not found.
    #[snafu(display("Object '{}' not found in bucket '{}'", key, bucket))]
    ObjectNotFound { bucket: String, key: String },

    /// Object too large.
    #[snafu(display("Object size {} bytes exceeds maximum {} bytes", size, max_size))]
    ObjectTooLarge { size: u64, max_size: u64 },

    /// Invalid object key.
    #[snafu(display("Invalid object key '{}': {}", key, reason))]
    InvalidObjectKey { key: String, reason: String },

    /// Storage operation failed.
    #[snafu(display("Storage operation failed: {}", source))]
    StorageError { source: anyhow::Error },

    /// Serialization/deserialization error.
    #[snafu(display("Serialization error: {}", source))]
    SerializationError { source: serde_json::Error },

    /// Chunking error.
    #[snafu(display("Chunking error for object '{}': {}", key, reason))]
    ChunkingError { key: String, reason: String },

    /// Invalid ETag.
    #[snafu(display("ETag mismatch for object '{}'", key))]
    ETagMismatch { key: String },

    /// Internal error.
    #[snafu(display("Internal S3 service error: {}", message))]
    InternalError { message: String },

    /// Authentication failed.
    #[snafu(display("Authentication failed: {}", reason))]
    AuthenticationFailed { reason: String },

    /// Access denied.
    #[snafu(display("Access denied to {} '{}'", resource_type, resource))]
    AccessDenied {
        resource_type: String,
        resource: String,
    },

    /// Invalid range request.
    #[snafu(display("Invalid range request: {}", reason))]
    InvalidRange { reason: String },

    /// Precondition failed.
    #[snafu(display("Precondition failed: {}", reason))]
    PreconditionFailed { reason: String },
}

/// Result type for S3 operations.
pub type S3Result<T> = Result<T, S3Error>;

impl From<anyhow::Error> for S3Error {
    fn from(err: anyhow::Error) -> Self {
        S3Error::StorageError { source: err }
    }
}

impl From<serde_json::Error> for S3Error {
    fn from(err: serde_json::Error) -> Self {
        S3Error::SerializationError { source: err }
    }
}

/// Convert S3Error to s3s error code for proper HTTP status mapping.
impl S3Error {
    /// Get the S3 error code string for this error.
    pub fn error_code(&self) -> &str {
        match self {
            S3Error::BucketAlreadyExists { .. } => "BucketAlreadyExists",
            S3Error::BucketNotFound { .. } => "NoSuchBucket",
            S3Error::BucketNotEmpty { .. } => "BucketNotEmpty",
            S3Error::InvalidBucketName { .. } => "InvalidBucketName",
            S3Error::ObjectNotFound { .. } => "NoSuchKey",
            S3Error::ObjectTooLarge { .. } => "EntityTooLarge",
            S3Error::InvalidObjectKey { .. } => "InvalidObjectKey",
            S3Error::StorageError { .. } => "InternalError",
            S3Error::SerializationError { .. } => "InternalError",
            S3Error::ChunkingError { .. } => "InternalError",
            S3Error::ETagMismatch { .. } => "InvalidETag",
            S3Error::InternalError { .. } => "InternalError",
            S3Error::AuthenticationFailed { .. } => "SignatureDoesNotMatch",
            S3Error::AccessDenied { .. } => "AccessDenied",
            S3Error::InvalidRange { .. } => "InvalidRange",
            S3Error::PreconditionFailed { .. } => "PreconditionFailed",
        }
    }
}
