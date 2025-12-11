//! S3 API constants and limits following Tiger Style principles.
//!
//! All limits are fixed to prevent unbounded resource usage.
//! Sizes use explicit types (u64, u32) for portability.

/// Maximum S3 object size in bytes (100 MB).
///
/// Tiger Style: Fixed limit prevents unbounded memory allocation.
/// This aligns with Aspen's MAX_SNAPSHOT_SIZE constant.
pub const MAX_S3_OBJECT_SIZE_BYTES: u64 = 100 * 1024 * 1024;

/// S3 chunk size in bytes for large objects (1 MB).
///
/// Tiger Style: Matches MAX_VALUE_SIZE from Raft constants for consistency.
/// Fixed chunk size enables predictable memory allocation.
pub const S3_CHUNK_SIZE_BYTES: u32 = 1024 * 1024;

/// Maximum number of chunks per object.
///
/// Tiger Style: Bounded to prevent excessive iteration.
/// Derived from MAX_S3_OBJECT_SIZE / S3_CHUNK_SIZE = 100.
pub const MAX_CHUNKS_PER_OBJECT: u32 = 100;

/// Maximum S3 bucket name length in bytes (63).
///
/// Tiger Style: S3 specification compliance, prevents unbounded strings.
pub const MAX_BUCKET_NAME_LENGTH: usize = 63;

/// Minimum S3 bucket name length in bytes (3).
///
/// Tiger Style: S3 specification compliance.
pub const MIN_BUCKET_NAME_LENGTH: usize = 3;

/// Maximum S3 object key length in bytes (1024).
///
/// Tiger Style: S3 specification compliance, prevents unbounded keys.
pub const MAX_S3_KEY_LENGTH: usize = 1024;

/// Maximum objects per ListObjectsV2 response (1000).
///
/// Tiger Style: S3 specification default, bounded pagination.
pub const MAX_LIST_OBJECTS: u32 = 1000;

/// Maximum number of keys in a vault scan operation.
///
/// Tiger Style: Prevents unbounded memory usage during listing.
pub const MAX_VAULT_SCAN_KEYS: u32 = 10_000;

/// S3 vault prefix for all S3 buckets.
///
/// All S3 buckets are stored as Aspen vaults with this prefix.
pub const S3_VAULT_PREFIX: &str = "s3";

/// Metadata key suffix for bucket metadata.
pub const BUCKET_METADATA_SUFFIX: &str = "_bucket_meta";

/// Metadata key prefix for object metadata.
pub const OBJECT_METADATA_PREFIX: &str = "_meta";

/// Data key prefix for object data.
pub const OBJECT_DATA_PREFIX: &str = "_data";

/// Chunk key component for chunked objects.
pub const CHUNK_KEY_COMPONENT: &str = "chunk";

/// Default content type for objects without explicit type.
pub const DEFAULT_CONTENT_TYPE: &str = "application/octet-stream";

/// S3 API version string.
pub const S3_API_VERSION: &str = "2006-03-01";

/// S3 server port (MinIO/S3 compatible default).
pub const DEFAULT_S3_PORT: u16 = 9000;

/// Maximum number of parts per multipart upload (S3 limit: 10000).
///
/// Tiger Style: Bounded to prevent excessive resource usage.
pub const MAX_MULTIPART_PARTS: u32 = 10_000;

/// Minimum part size in bytes (5 MB, except for the last part).
///
/// Tiger Style: S3 specification compliance.
pub const MIN_PART_SIZE_BYTES: u64 = 5 * 1024 * 1024;

/// Maximum part size in bytes (5 GB).
///
/// Tiger Style: S3 specification compliance.
pub const MAX_PART_SIZE_BYTES: u64 = 5 * 1024 * 1024 * 1024;

/// Multipart upload metadata key prefix.
pub const MULTIPART_UPLOAD_PREFIX: &str = "_mpu";

/// Multipart upload part data key prefix.
pub const MULTIPART_PART_PREFIX: &str = "_mpu_part";

/// Version ID for the "null" version (used before versioning was enabled).
///
/// Tiger Style: Special sentinel value for non-versioned objects.
pub const NULL_VERSION_ID: &str = "null";

/// Version metadata key prefix for storing versioned objects.
pub const VERSION_METADATA_PREFIX: &str = "_ver_meta";

/// Version data key prefix for storing versioned object data.
pub const VERSION_DATA_PREFIX: &str = "_ver_data";

/// Maximum number of versions to return in ListObjectVersions (S3 default: 1000).
///
/// Tiger Style: Bounded pagination prevents unbounded memory usage.
pub const MAX_LIST_VERSIONS: u32 = 1000;

/// Object tags metadata key prefix.
pub const OBJECT_TAGS_PREFIX: &str = "_tags";

/// Maximum number of tags per object (S3 limit: 10).
///
/// Tiger Style: Bounded to prevent resource exhaustion.
pub const MAX_OBJECT_TAGS: usize = 10;

/// Maximum number of tags per bucket (S3 limit: 50).
///
/// Tiger Style: Bounded to prevent resource exhaustion.
pub const MAX_BUCKET_TAGS: usize = 50;

/// Maximum tag key length in bytes (S3 limit: 128 UTF-8 characters).
///
/// Tiger Style: S3 specification compliance.
pub const MAX_TAG_KEY_LENGTH: usize = 128;

/// Maximum tag value length in bytes (S3 limit: 256 UTF-8 characters).
///
/// Tiger Style: S3 specification compliance.
pub const MAX_TAG_VALUE_LENGTH: usize = 256;

/// Maximum number of multipart uploads to list (S3 default: 1000).
///
/// Tiger Style: Bounded pagination prevents unbounded memory usage.
pub const MAX_LIST_MULTIPART_UPLOADS: u32 = 1000;
