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

    /// Encryption configuration.
    #[serde(default)]
    pub encryption: Option<BucketEncryptionConfiguration>,

    /// Lifecycle configuration.
    #[serde(default)]
    pub lifecycle: Option<BucketLifecycleConfiguration>,

    /// MFA delete status (for versioning).
    #[serde(default)]
    pub mfa_delete: bool,
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
            encryption: None,
            lifecycle: None,
            mfa_delete: false,
        }
    }
}

/// Bucket versioning configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BucketVersioning {
    /// Versioning is disabled (default).
    #[default]
    Disabled,
    /// Versioning is enabled.
    Enabled,
    /// Versioning is suspended.
    Suspended,
}

impl BucketVersioning {
    /// Convert from S3 versioning status string.
    pub fn from_s3_status(status: Option<&str>) -> Self {
        match status {
            Some("Enabled") => BucketVersioning::Enabled,
            Some("Suspended") => BucketVersioning::Suspended,
            _ => BucketVersioning::Disabled,
        }
    }

    /// Convert to S3 versioning status string.
    pub fn to_s3_status(&self) -> Option<&'static str> {
        match self {
            BucketVersioning::Disabled => None,
            BucketVersioning::Enabled => Some("Enabled"),
            BucketVersioning::Suspended => Some("Suspended"),
        }
    }

    /// Check if versioning is enabled or was enabled (suspended still tracks versions).
    pub fn is_version_tracking(&self) -> bool {
        matches!(
            self,
            BucketVersioning::Enabled | BucketVersioning::Suspended
        )
    }
}

/// Server-side encryption algorithm.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ServerSideEncryption {
    /// No encryption (default).
    #[default]
    None,
    /// AES-256 encryption (S3-managed keys).
    Aes256,
    /// AWS KMS encryption.
    AwsKms { key_id: Option<String> },
}

impl ServerSideEncryption {
    /// Convert to S3 algorithm string.
    pub fn to_algorithm_string(&self) -> Option<&str> {
        match self {
            ServerSideEncryption::None => None,
            ServerSideEncryption::Aes256 => Some("AES256"),
            ServerSideEncryption::AwsKms { .. } => Some("aws:kms"),
        }
    }

    /// Create from S3 algorithm string.
    pub fn from_algorithm_string(alg: Option<&str>, key_id: Option<String>) -> Self {
        match alg {
            Some("AES256") => ServerSideEncryption::Aes256,
            Some("aws:kms") => ServerSideEncryption::AwsKms { key_id },
            _ => ServerSideEncryption::None,
        }
    }
}

/// Lifecycle rule status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LifecycleRuleStatus {
    /// Rule is enabled.
    Enabled,
    /// Rule is disabled.
    Disabled,
}

/// Lifecycle expiration configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleExpiration {
    /// Number of days after creation to expire objects.
    pub days: Option<u32>,
    /// Specific date to expire objects.
    pub date: Option<DateTime<Utc>>,
    /// Delete expired object delete markers.
    pub expired_object_delete_marker: Option<bool>,
}

/// Lifecycle transition configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleTransition {
    /// Number of days after creation to transition.
    pub days: Option<u32>,
    /// Specific date to transition.
    pub date: Option<DateTime<Utc>>,
    /// Target storage class.
    pub storage_class: String,
}

/// Noncurrent version expiration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoncurrentVersionExpiration {
    /// Days after objects become noncurrent to expire.
    pub noncurrent_days: u32,
    /// Maximum number of noncurrent versions to retain.
    pub newer_noncurrent_versions: Option<u32>,
}

/// Lifecycle rule for bucket lifecycle configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleRule {
    /// Unique identifier for the rule (max 255 chars).
    pub id: String,
    /// Rule status (enabled/disabled).
    pub status: LifecycleRuleStatus,
    /// Object key prefix filter.
    pub prefix: Option<String>,
    /// Expiration configuration.
    pub expiration: Option<LifecycleExpiration>,
    /// Transition configurations.
    pub transitions: Vec<LifecycleTransition>,
    /// Noncurrent version expiration.
    pub noncurrent_version_expiration: Option<NoncurrentVersionExpiration>,
    /// Abort incomplete multipart uploads after days.
    pub abort_incomplete_multipart_upload_days: Option<u32>,
}

/// Bucket lifecycle configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BucketLifecycleConfiguration {
    /// Lifecycle rules (max 1000 per S3 spec).
    pub rules: Vec<LifecycleRule>,
}

/// Bucket encryption configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct BucketEncryptionConfiguration {
    /// Default server-side encryption.
    pub sse_algorithm: ServerSideEncryption,
    /// Whether to apply bucket key for KMS encryption (reduces KMS costs).
    pub bucket_key_enabled: bool,
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

/// Metadata for a multipart upload in progress.
///
/// Stored as JSON in the KV store at the upload metadata key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartUploadMetadata {
    /// Unique upload identifier (UUID).
    pub upload_id: String,

    /// Bucket name.
    pub bucket: String,

    /// Object key being uploaded.
    pub key: String,

    /// When the upload was initiated.
    pub initiated_at: DateTime<Utc>,

    /// MIME content type of the final object.
    pub content_type: Option<String>,

    /// Storage class (currently always "STANDARD").
    pub storage_class: String,

    /// Parts that have been uploaded (keyed by part number).
    ///
    /// Tiger Style: Bounded to 10000 parts (S3 maximum).
    pub parts: HashMap<u32, PartMetadata>,
}

impl MultipartUploadMetadata {
    /// Create new multipart upload metadata.
    pub fn new(
        upload_id: String,
        bucket: String,
        key: String,
        content_type: Option<String>,
    ) -> Self {
        Self {
            upload_id,
            bucket,
            key,
            initiated_at: Utc::now(),
            content_type,
            storage_class: "STANDARD".to_string(),
            parts: HashMap::new(),
        }
    }

    /// Add or update a part.
    pub fn add_part(&mut self, part: PartMetadata) {
        self.parts.insert(part.part_number, part);
    }

    /// Get all parts sorted by part number.
    pub fn get_sorted_parts(&self) -> Vec<&PartMetadata> {
        let mut parts: Vec<_> = self.parts.values().collect();
        parts.sort_by_key(|p| p.part_number);
        parts
    }
}

/// Metadata for an uploaded part of a multipart upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartMetadata {
    /// Part number (1-10000).
    pub part_number: u32,

    /// Size of the part in bytes.
    pub size_bytes: u64,

    /// ETag (MD5 hash) of the part data.
    pub etag: String,

    /// When the part was uploaded.
    pub last_modified: DateTime<Utc>,
}

impl PartMetadata {
    /// Create new part metadata.
    pub fn new(part_number: u32, size_bytes: u64, etag: String) -> Self {
        Self {
            part_number,
            size_bytes,
            etag,
            last_modified: Utc::now(),
        }
    }
}

/// Metadata for a delete marker (versioned deletion).
///
/// Delete markers are placed when an object is deleted in a versioned bucket.
/// They mark the current version as deleted without removing the previous versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMarkerMetadata {
    /// Object key this delete marker is for.
    pub key: String,

    /// Version ID of this delete marker.
    pub version_id: String,

    /// When the delete marker was created.
    pub last_modified: DateTime<Utc>,

    /// Whether this is the current (latest) version.
    pub is_latest: bool,
}

impl DeleteMarkerMetadata {
    /// Create a new delete marker.
    pub fn new(key: String, version_id: String, is_latest: bool) -> Self {
        Self {
            key,
            version_id,
            last_modified: Utc::now(),
            is_latest,
        }
    }
}

/// Represents either an object version or a delete marker.
///
/// Used in ListObjectVersions responses to represent both types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectVersionEntry {
    /// An actual object version.
    Version {
        key: String,
        version_id: String,
        is_latest: bool,
        last_modified: DateTime<Utc>,
        etag: String,
        size_bytes: u64,
        storage_class: String,
    },
    /// A delete marker.
    DeleteMarker {
        key: String,
        version_id: String,
        is_latest: bool,
        last_modified: DateTime<Utc>,
    },
}

/// Object tags for S3 object tagging feature.
///
/// Tags are key-value pairs that can be attached to objects.
/// S3 allows up to 10 tags per object.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ObjectTags {
    /// Tag key-value pairs (max 10 per S3 spec).
    pub tags: HashMap<String, String>,
}

impl ObjectTags {
    /// Create a new empty tag set.
    pub fn new() -> Self {
        Self {
            tags: HashMap::new(),
        }
    }

    /// Add a tag. Returns error if max tags exceeded.
    pub fn add_tag(&mut self, key: String, value: String) -> Result<(), &'static str> {
        // S3 allows max 10 tags per object
        if self.tags.len() >= 10 && !self.tags.contains_key(&key) {
            return Err("Maximum 10 tags allowed per object");
        }
        // S3 tag key max 128 chars, value max 256 chars
        if key.len() > 128 {
            return Err("Tag key exceeds maximum length of 128 characters");
        }
        if value.len() > 256 {
            return Err("Tag value exceeds maximum length of 256 characters");
        }
        self.tags.insert(key, value);
        Ok(())
    }

    /// Check if tags are empty.
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

/// Bucket tags for S3 bucket tagging feature.
///
/// Tags are key-value pairs for cost allocation and organization.
/// S3 allows up to 50 tags per bucket.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BucketTags {
    /// Tag key-value pairs (max 50 per S3 spec).
    pub tags: HashMap<String, String>,
}

impl BucketTags {
    /// Create a new empty tag set.
    pub fn new() -> Self {
        Self {
            tags: HashMap::new(),
        }
    }

    /// Add a tag. Returns error if max tags exceeded.
    pub fn add_tag(&mut self, key: String, value: String) -> Result<(), &'static str> {
        // S3 allows max 50 tags per bucket
        if self.tags.len() >= 50 && !self.tags.contains_key(&key) {
            return Err("Maximum 50 tags allowed per bucket");
        }
        // S3 tag key max 128 chars, value max 256 chars
        if key.len() > 128 {
            return Err("Tag key exceeds maximum length of 128 characters");
        }
        if value.len() > 256 {
            return Err("Tag value exceeds maximum length of 256 characters");
        }
        self.tags.insert(key, value);
        Ok(())
    }

    /// Check if tags are empty.
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}
