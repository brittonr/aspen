/// S3-compatible API implementation for Aspen.
///
/// Provides a subset of the Amazon S3 REST API on top of Aspen's
/// distributed key-value store.
///
/// # Architecture
///
/// The S3 layer maps S3 buckets to Aspen vaults and objects to
/// key-value pairs. Large objects are automatically chunked to
/// respect the maximum value size limits.
///
/// # Supported Operations
///
/// - Bucket operations: Create, Delete, List
/// - Object operations: Put, Get, Delete, Head, List
/// - Chunking for objects > 1MB
///
/// # Example
///
/// ```no_run
/// use aspen::s3::AspenS3Service;
/// use aspen::api::KeyValueStore;
/// use std::sync::Arc;
///
/// async fn run_s3_server(kv_store: Arc<dyn KeyValueStore>) {
///     let s3_service = AspenS3Service::new(kv_store, 1);
///     // Configure and start HTTP server with s3s
/// }
/// ```
pub mod constants;
pub mod error;
pub mod metadata;
pub mod presign;
pub mod service;

// Re-export commonly used types
pub use constants::*;
pub use error::{S3Error, S3Result};
pub use metadata::{
    BucketAcl, BucketEncryptionConfiguration, BucketLifecycleConfiguration, BucketMetadata,
    BucketVersioning, LifecycleExpiration, LifecycleRule, LifecycleRuleStatus, LifecycleTransition,
    MultipartUploadMetadata, NoncurrentVersionExpiration, ObjectMetadata, PartMetadata,
    ServerSideEncryption,
};
pub use presign::{HttpMethod, PresignError, PresignedUrlBuilder};
pub use service::AspenS3Service;
