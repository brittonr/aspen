/// S3 object and bucket metadata types.
///
/// Uses explicit types and units following Tiger Style.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata for an S3 object.
///
/// Stored as JSON in the KV store at the metadata key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    /// Size of the object in bytes.
    pub size_bytes: u64,

    /// ETag (entity tag) for the object.
    ///
    /// For simple objects, this is the MD5 hash.
    /// For multipart uploads, it includes a suffix with the part count.
    pub etag: String,

    /// MIME content type of the object.
    pub content_type: String,

    /// Last modification timestamp.
    pub last_modified: DateTime<Utc>,

    /// Number of chunks for large objects.
    ///
    /// 0 indicates the object is not chunked (stored in single key).
    pub chunk_count: u32,

    /// Size of each chunk in bytes (except possibly the last).
    ///
    /// Only meaningful when chunk_count > 0.
    pub chunk_size_bytes: u32,

    /// Optional content encoding (e.g., "gzip").
    pub content_encoding: Option<String>,

    /// Optional cache control header value.
    pub cache_control: Option<String>,

    /// Optional content disposition header value.
    pub content_disposition: Option<String>,

    /// Optional content language.
    pub content_language: Option<String>,

    /// Custom user metadata.
    ///
    /// Keys must start with "x-amz-meta-" prefix.
    pub user_metadata: HashMap<String, String>,

    /// Storage class (currently always "STANDARD").
    pub storage_class: String,

    /// Version ID (reserved for future versioning support).
    pub version_id: Option<String>,
}

impl ObjectMetadata {
    /// Create new metadata for a simple (non-chunked) object.
    pub fn new_simple(size_bytes: u64, etag: String, content_type: String) -> Self {
        Self {
            size_bytes,
            etag,
            content_type,
            last_modified: Utc::now(),
            chunk_count: 0,
            chunk_size_bytes: 0,
            content_encoding: None,
            cache_control: None,
            content_disposition: None,
            content_language: None,
            user_metadata: HashMap::new(),
            storage_class: "STANDARD".to_string(),
            version_id: None,
        }
    }

    /// Create new metadata for a chunked object.
    pub fn new_chunked(
        size_bytes: u64,
        etag: String,
        content_type: String,
        chunk_count: u32,
        chunk_size_bytes: u32,
    ) -> Self {
        Self {
            size_bytes,
            etag,
            content_type,
            last_modified: Utc::now(),
            chunk_count,
            chunk_size_bytes,
            content_encoding: None,
            cache_control: None,
            content_disposition: None,
            content_language: None,
            user_metadata: HashMap::new(),
            storage_class: "STANDARD".to_string(),
            version_id: None,
        }
    }

    /// Check if the object is chunked.
    pub fn is_chunked(&self) -> bool {
        self.chunk_count > 0
    }

    /// Get the last chunk index (0-based).
    pub fn last_chunk_index(&self) -> Option<u32> {
        if self.is_chunked() {
            Some(self.chunk_count - 1)
        } else {
            None
        }
    }
}

/// Metadata for an S3 bucket.
///
/// Stored as JSON in the KV store at the bucket metadata key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketMetadata {
    /// Bucket name.
    pub name: String,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// AWS region (defaults to "us-east-1").
    pub region: String,

    /// Bucket versioning status.
    pub versioning: BucketVersioning,

    /// Optional bucket tags.
    pub tags: HashMap<String, String>,

    /// Access control list (simplified for now).
    pub acl: BucketAcl,
}

impl BucketMetadata {
    /// Create new bucket metadata.
    pub fn new(name: String) -> Self {
        Self {
            name,
            created_at: Utc::now(),
            region: "us-east-1".to_string(),
            versioning: BucketVersioning::Disabled,
            tags: HashMap::new(),
            acl: BucketAcl::Private,
        }
    }
}

/// Bucket versioning configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BucketVersioning {
    /// Versioning is disabled (default).
    Disabled,
    /// Versioning is enabled.
    Enabled,
    /// Versioning is suspended.
    Suspended,
}

/// Simplified bucket ACL.
///
/// Full ACL support deferred to future phases.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BucketAcl {
    /// Private access (default).
    Private,
    /// Public read access.
    PublicRead,
    /// Public read/write access.
    PublicReadWrite,
}

/// List objects response entry.
#[derive(Debug, Clone)]
pub struct ListObjectEntry {
    /// Object key.
    pub key: String,

    /// Object size in bytes.
    pub size_bytes: u64,

    /// Last modified timestamp.
    pub last_modified: DateTime<Utc>,

    /// Object ETag.
    pub etag: String,

    /// Storage class.
    pub storage_class: String,
}

/// Common prefix for hierarchical listing.
#[derive(Debug, Clone)]
pub struct CommonPrefix {
    /// Prefix string.
    pub prefix: String,
}
