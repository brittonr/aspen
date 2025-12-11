/// S3 service implementation for Aspen.
///
/// Maps S3 operations to Aspen's distributed key-value store.
use crate::api::{
    DeleteRequest, KeyValueStore, KeyValueStoreError, ReadRequest, ScanRequest, WriteCommand,
    WriteRequest,
};
use crate::s3::constants::*;
use crate::s3::error::{S3Error, S3Result};
use crate::s3::metadata::{
    BucketEncryptionConfiguration, BucketLifecycleConfiguration, BucketMetadata, BucketVersioning,
    DeleteMarkerMetadata, LifecycleExpiration, LifecycleRule, LifecycleRuleStatus,
    LifecycleTransition, MultipartUploadMetadata, NoncurrentVersionExpiration, ObjectMetadata,
    ObjectVersionEntry, PartMetadata, ServerSideEncryption as AspenServerSideEncryption,
};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use futures::stream;
use s3s::dto::*;
use s3s::s3_error;
use s3s::{S3, S3Request, S3Response};
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// Convert chrono DateTime to s3s Timestamp.
fn chrono_to_timestamp(dt: DateTime<Utc>) -> Timestamp {
    // Convert to SystemTime
    let duration = dt.timestamp() as u64;
    let nanos = dt.timestamp_subsec_nanos();
    let system_time = SystemTime::UNIX_EPOCH + std::time::Duration::new(duration, nanos);
    Timestamp::from(system_time)
}

/// Parse a content type string into a ContentType.
fn parse_content_type(s: &str) -> Option<ContentType> {
    // ContentType wraps mime::Mime, parse the string as a Mime type
    s.parse::<mime::Mime>().ok()
}

/// Create a streaming blob from bytes.
fn bytes_to_streaming_blob(data: Vec<u8>) -> StreamingBlob {
    let bytes = Bytes::from(data);
    // Create a stream that yields one chunk
    let data_stream = stream::once(async move { Ok::<_, std::io::Error>(bytes) });
    StreamingBlob::wrap(data_stream)
}

/// S3 service implementation backed by Aspen's KV store.
pub struct AspenS3Service {
    /// The underlying key-value store (Raft-backed).
    kv_store: Arc<dyn KeyValueStore>,

    /// Node ID for this S3 service instance.
    node_id: u64,
}

/// Resolved byte range for partial content requests.
struct ResolvedRange {
    /// Start byte position (inclusive).
    start: u64,
    /// End byte position (inclusive).
    end: u64,
}

/// Resolve an HTTP Range header to actual byte positions.
///
/// Returns the resolved range (start, end) inclusive, or an error if
/// the range is not satisfiable.
fn resolve_range(range: &Range, object_size: u64) -> S3Result<ResolvedRange> {
    if object_size == 0 {
        return Err(S3Error::InvalidRange {
            reason: "Cannot request range of empty object".to_string(),
        });
    }

    match range {
        Range::Int { first, last } => {
            // first-last: bytes first to last (inclusive)
            // first-: bytes first to end
            if *first >= object_size {
                return Err(S3Error::InvalidRange {
                    reason: format!("Start position {} >= object size {}", first, object_size),
                });
            }

            let end = match last {
                Some(l) => (*l).min(object_size - 1),
                None => object_size - 1,
            };

            if end < *first {
                return Err(S3Error::InvalidRange {
                    reason: format!("End position {} < start position {}", end, first),
                });
            }

            Ok(ResolvedRange { start: *first, end })
        }
        Range::Suffix { length } => {
            // -length: last length bytes
            if *length == 0 {
                return Err(S3Error::InvalidRange {
                    reason: "Suffix length cannot be zero".to_string(),
                });
            }

            let start = object_size.saturating_sub(*length);
            Ok(ResolvedRange {
                start,
                end: object_size - 1,
            })
        }
    }
}

/// Format a Content-Range header value.
fn format_content_range(start: u64, end: u64, total: u64) -> String {
    format!("bytes {}-{}/{}", start, end, total)
}

impl AspenS3Service {
    /// Create a new S3 service instance.
    pub fn new(kv_store: Arc<dyn KeyValueStore>, node_id: u64) -> Self {
        info!("Initializing Aspen S3 service on node {}", node_id);
        Self { kv_store, node_id }
    }

    /// Generate a vault name for an S3 bucket.
    #[allow(dead_code)]
    fn bucket_vault_name(bucket: &str) -> String {
        format!("{}:{}", S3_VAULT_PREFIX, bucket)
    }

    /// Generate a metadata key for a bucket.
    fn bucket_metadata_key(bucket: &str) -> String {
        format!(
            "vault:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, BUCKET_METADATA_SUFFIX
        )
    }

    /// Generate a metadata key for an object.
    fn object_metadata_key(bucket: &str, key: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_METADATA_PREFIX, key
        )
    }

    /// Generate a data key for an object (non-chunked).
    fn object_data_key(bucket: &str, key: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_DATA_PREFIX, key
        )
    }

    /// Generate a tags key for an object.
    fn object_tags_key(bucket: &str, key: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_TAGS_PREFIX, key
        )
    }

    /// Generate a data key for an object chunk.
    fn object_chunk_key(bucket: &str, key: &str, chunk_index: u32) -> String {
        format!(
            "vault:{}:{}:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_DATA_PREFIX, key, CHUNK_KEY_COMPONENT, chunk_index
        )
    }

    /// Validate bucket name according to S3 rules.
    fn validate_bucket_name(name: &str) -> S3Result<()> {
        let len = name.len();

        if !(MIN_BUCKET_NAME_LENGTH..=MAX_BUCKET_NAME_LENGTH).contains(&len) {
            return Err(S3Error::InvalidBucketName {
                name: name.to_string(),
                reason: format!(
                    "Bucket name must be between {} and {} characters",
                    MIN_BUCKET_NAME_LENGTH, MAX_BUCKET_NAME_LENGTH
                ),
            });
        }

        // Check for valid characters (alphanumeric, hyphens, periods)
        // S3 bucket naming rules are complex, simplified for MVP
        for ch in name.chars() {
            if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' && ch != '.' {
                return Err(S3Error::InvalidBucketName {
                    name: name.to_string(),
                    reason: "Bucket names must contain only lowercase letters, numbers, hyphens, and periods".to_string(),
                });
            }
        }

        // Must start and end with letter or number
        if !name.chars().next().unwrap().is_ascii_alphanumeric()
            || !name.chars().last().unwrap().is_ascii_alphanumeric()
        {
            return Err(S3Error::InvalidBucketName {
                name: name.to_string(),
                reason: "Bucket names must start and end with a letter or number".to_string(),
            });
        }

        Ok(())
    }

    /// Validate object key according to S3 rules.
    fn validate_object_key(key: &str) -> S3Result<()> {
        if key.is_empty() {
            return Err(S3Error::InvalidObjectKey {
                key: key.to_string(),
                reason: "Object key cannot be empty".to_string(),
            });
        }

        if key.len() > MAX_S3_KEY_LENGTH {
            return Err(S3Error::InvalidObjectKey {
                key: key.to_string(),
                reason: format!(
                    "Object key exceeds maximum length of {} bytes",
                    MAX_S3_KEY_LENGTH
                ),
            });
        }

        Ok(())
    }

    /// Infer content type from file extension.
    fn infer_content_type(key: &str) -> String {
        let extension = key.rsplit('.').next().unwrap_or("").to_lowercase();

        match extension.as_str() {
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "json" => "application/json",
            "xml" => "application/xml",
            "txt" | "text" => "text/plain",
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",
            _ => DEFAULT_CONTENT_TYPE,
        }
        .to_string()
    }

    /// Check if a bucket exists by reading its metadata.
    async fn bucket_exists(&self, bucket: &str) -> Result<bool, KeyValueStoreError> {
        let key = Self::bucket_metadata_key(bucket);
        match self.kv_store.read(ReadRequest { key }).await {
            Ok(_) => Ok(true),
            Err(KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get bucket metadata.
    async fn get_bucket_metadata(
        &self,
        bucket: &str,
    ) -> Result<Option<BucketMetadata>, KeyValueStoreError> {
        let key = Self::bucket_metadata_key(bucket);
        match self.kv_store.read(ReadRequest { key }).await {
            Ok(result) => {
                let meta: BucketMetadata = serde_json::from_str(&result.value).map_err(|e| {
                    KeyValueStoreError::Failed {
                        reason: format!("Failed to deserialize bucket metadata: {}", e),
                    }
                })?;
                Ok(Some(meta))
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Calculate MD5 hash and return as ETag.
    fn calculate_etag(data: &[u8]) -> String {
        let digest = md5::compute(data);
        format!("\"{}\"", hex::encode(digest.as_ref()))
    }

    /// Store object metadata.
    async fn store_object_metadata(
        &self,
        bucket: &str,
        key: &str,
        metadata: &ObjectMetadata,
    ) -> Result<(), KeyValueStoreError> {
        let meta_key = Self::object_metadata_key(bucket, key);
        let meta_value =
            serde_json::to_string(metadata).map_err(|e| KeyValueStoreError::Failed {
                reason: format!("Failed to serialize object metadata: {}", e),
            })?;

        self.kv_store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: meta_key,
                    value: meta_value,
                },
            })
            .await?;

        Ok(())
    }

    /// Get object metadata.
    async fn get_object_metadata(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Option<ObjectMetadata>, KeyValueStoreError> {
        let meta_key = Self::object_metadata_key(bucket, key);
        match self.kv_store.read(ReadRequest { key: meta_key }).await {
            Ok(result) => {
                let meta: ObjectMetadata = serde_json::from_str(&result.value).map_err(|e| {
                    KeyValueStoreError::Failed {
                        reason: format!("Failed to deserialize object metadata: {}", e),
                    }
                })?;
                Ok(Some(meta))
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Store a small object (non-chunked) directly.
    async fn store_simple_object(
        &self,
        bucket: &str,
        key: &str,
        data: &[u8],
        content_type: Option<String>,
    ) -> Result<ObjectMetadata, KeyValueStoreError> {
        let data_key = Self::object_data_key(bucket, key);

        // Encode data as base64 for safe storage
        let encoded_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data);

        // Store the data
        self.kv_store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: data_key,
                    value: encoded_data,
                },
            })
            .await?;

        // Calculate ETag
        let etag = Self::calculate_etag(data);

        // Determine content type
        let content_type = content_type.unwrap_or_else(|| Self::infer_content_type(key));

        // Create and store metadata
        let metadata = ObjectMetadata::new_simple(data.len() as u64, etag, content_type);
        self.store_object_metadata(bucket, key, &metadata).await?;

        Ok(metadata)
    }

    /// Store a large object using chunking.
    async fn store_chunked_object(
        &self,
        bucket: &str,
        key: &str,
        data: &[u8],
        content_type: Option<String>,
    ) -> Result<ObjectMetadata, KeyValueStoreError> {
        let chunk_size = S3_CHUNK_SIZE_BYTES as usize;
        let total_size = data.len();
        let chunk_count = total_size.div_ceil(chunk_size) as u32;

        // Validate chunk count doesn't exceed limit
        if chunk_count > MAX_CHUNKS_PER_OBJECT {
            return Err(KeyValueStoreError::Failed {
                reason: format!(
                    "Object would require {} chunks, exceeding maximum of {}",
                    chunk_count, MAX_CHUNKS_PER_OBJECT
                ),
            });
        }

        // Store each chunk
        for i in 0..chunk_count {
            let start = (i as usize) * chunk_size;
            let end = std::cmp::min(start + chunk_size, total_size);
            let chunk_data = &data[start..end];

            let chunk_key = Self::object_chunk_key(bucket, key, i);
            let encoded_chunk =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, chunk_data);

            self.kv_store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: chunk_key,
                        value: encoded_chunk,
                    },
                })
                .await?;
        }

        // Calculate ETag (for chunked objects, we still hash the whole thing)
        let etag = Self::calculate_etag(data);

        // Determine content type
        let content_type = content_type.unwrap_or_else(|| Self::infer_content_type(key));

        // Create and store metadata
        let metadata = ObjectMetadata::new_chunked(
            total_size as u64,
            etag,
            content_type,
            chunk_count,
            S3_CHUNK_SIZE_BYTES,
        );
        self.store_object_metadata(bucket, key, &metadata).await?;

        Ok(metadata)
    }

    /// Retrieve a simple (non-chunked) object.
    async fn get_simple_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, S3Error> {
        let data_key = Self::object_data_key(bucket, key);

        let result = self
            .kv_store
            .read(ReadRequest { key: data_key })
            .await
            .map_err(|e| match e {
                KeyValueStoreError::NotFound { .. } => S3Error::ObjectNotFound {
                    bucket: bucket.to_string(),
                    key: key.to_string(),
                },
                _ => S3Error::StorageError {
                    source: anyhow::anyhow!("{}", e),
                },
            })?;

        // Decode from base64
        let data =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &result.value)
                .map_err(|e| S3Error::StorageError {
                    source: anyhow::anyhow!("Failed to decode object data: {}", e),
                })?;

        Ok(data)
    }

    /// Retrieve a chunked object.
    async fn get_chunked_object(
        &self,
        bucket: &str,
        key: &str,
        metadata: &ObjectMetadata,
    ) -> Result<Vec<u8>, S3Error> {
        let mut data = Vec::with_capacity(metadata.size_bytes as usize);

        for i in 0..metadata.chunk_count {
            let chunk_key = Self::object_chunk_key(bucket, key, i);

            let result = self
                .kv_store
                .read(ReadRequest { key: chunk_key })
                .await
                .map_err(|e| S3Error::ChunkingError {
                    key: key.to_string(),
                    reason: format!("Failed to read chunk {}: {}", i, e),
                })?;

            // Decode from base64
            let chunk_data =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &result.value)
                    .map_err(|e| S3Error::ChunkingError {
                    key: key.to_string(),
                    reason: format!("Failed to decode chunk {}: {}", i, e),
                })?;

            data.extend_from_slice(&chunk_data);
        }

        Ok(data)
    }

    /// Generate a multipart upload metadata key.
    fn multipart_upload_metadata_key(bucket: &str, upload_id: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, MULTIPART_UPLOAD_PREFIX, upload_id
        )
    }

    /// Generate a multipart upload part data key.
    fn multipart_part_data_key(bucket: &str, upload_id: &str, part_number: u32) -> String {
        format!(
            "vault:{}:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, MULTIPART_PART_PREFIX, upload_id, part_number
        )
    }

    /// Store multipart upload metadata.
    async fn store_multipart_metadata(
        &self,
        metadata: &MultipartUploadMetadata,
    ) -> Result<(), KeyValueStoreError> {
        let meta_key = Self::multipart_upload_metadata_key(&metadata.bucket, &metadata.upload_id);
        let meta_value =
            serde_json::to_string(metadata).map_err(|e| KeyValueStoreError::Failed {
                reason: format!("Failed to serialize multipart metadata: {}", e),
            })?;

        self.kv_store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: meta_key,
                    value: meta_value,
                },
            })
            .await?;

        Ok(())
    }

    /// Get multipart upload metadata.
    async fn get_multipart_metadata(
        &self,
        bucket: &str,
        upload_id: &str,
    ) -> Result<Option<MultipartUploadMetadata>, KeyValueStoreError> {
        let meta_key = Self::multipart_upload_metadata_key(bucket, upload_id);
        match self.kv_store.read(ReadRequest { key: meta_key }).await {
            Ok(result) => {
                let meta: MultipartUploadMetadata =
                    serde_json::from_str(&result.value).map_err(|e| {
                        KeyValueStoreError::Failed {
                            reason: format!("Failed to deserialize multipart metadata: {}", e),
                        }
                    })?;
                Ok(Some(meta))
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Delete multipart upload metadata and all parts.
    async fn delete_multipart_upload(
        &self,
        bucket: &str,
        upload_id: &str,
    ) -> Result<(), KeyValueStoreError> {
        // Get metadata to find all parts
        if let Some(metadata) = self.get_multipart_metadata(bucket, upload_id).await? {
            // Delete all part data
            for part_number in metadata.parts.keys() {
                let part_key = Self::multipart_part_data_key(bucket, upload_id, *part_number);
                let _ = self.kv_store.delete(DeleteRequest { key: part_key }).await;
            }
        }

        // Delete metadata
        let meta_key = Self::multipart_upload_metadata_key(bucket, upload_id);
        let _ = self.kv_store.delete(DeleteRequest { key: meta_key }).await;

        Ok(())
    }

    /// Calculate composite ETag for multipart upload.
    ///
    /// S3 multipart ETags are: MD5(concatenated MD5s of all parts) + "-{part_count}"
    fn calculate_multipart_etag(parts: &[&PartMetadata]) -> String {
        // Tiger Style: Pre-allocate with known capacity (16 bytes per MD5 hash).
        let mut concatenated_md5s: Vec<u8> = Vec::with_capacity(parts.len() * 16);

        for part in parts {
            // Strip quotes from part ETags and decode hex
            let etag_hex = part.etag.trim_matches('"');
            if let Ok(bytes) = hex::decode(etag_hex) {
                concatenated_md5s.extend_from_slice(&bytes);
            }
        }

        let digest = md5::compute(&concatenated_md5s);
        format!("\"{:x}-{}\"", digest, parts.len())
    }

    /// Delete all chunks and metadata for an object.
    async fn delete_object_data(&self, bucket: &str, key: &str) -> Result<bool, S3Error> {
        // First check if object exists and get its metadata
        let metadata = match self.get_object_metadata(bucket, key).await {
            Ok(Some(m)) => m,
            Ok(None) => return Ok(false), // Object doesn't exist
            Err(e) => {
                return Err(S3Error::StorageError {
                    source: anyhow::anyhow!("{}", e),
                });
            }
        };

        // Delete chunks if object is chunked
        if metadata.is_chunked() {
            for i in 0..metadata.chunk_count {
                let chunk_key = Self::object_chunk_key(bucket, key, i);
                let _ = self.kv_store.delete(DeleteRequest { key: chunk_key }).await;
            }
        } else {
            // Delete simple object data
            let data_key = Self::object_data_key(bucket, key);
            let _ = self.kv_store.delete(DeleteRequest { key: data_key }).await;
        }

        // Delete metadata
        let meta_key = Self::object_metadata_key(bucket, key);
        let _ = self.kv_store.delete(DeleteRequest { key: meta_key }).await;

        Ok(true)
    }

    /// Save bucket metadata to the KV store.
    async fn save_bucket_metadata(&self, metadata: &BucketMetadata) -> S3Result<()> {
        let key = Self::bucket_metadata_key(&metadata.name);
        let value = serde_json::to_string(metadata)?;

        self.kv_store
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| S3Error::StorageError { source: e.into() })?;

        Ok(())
    }

    // ===== Versioning Helper Methods =====

    /// Generate a new version ID.
    ///
    /// Version IDs are timestamp-based UUIDs for ordering and uniqueness.
    #[allow(dead_code)]
    fn generate_version_id() -> String {
        // Use timestamp prefix for natural ordering + UUID for uniqueness
        let timestamp = Utc::now().timestamp_millis();
        let uuid = uuid::Uuid::new_v4();
        format!("{:016x}-{}", timestamp, uuid)
    }

    /// Generate a versioned metadata key for an object.
    #[allow(dead_code)]
    fn versioned_metadata_key(bucket: &str, key: &str, version_id: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, VERSION_METADATA_PREFIX, key, version_id
        )
    }

    /// Generate a versioned data key for an object (non-chunked).
    #[allow(dead_code)]
    fn versioned_data_key(bucket: &str, key: &str, version_id: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, VERSION_DATA_PREFIX, key, version_id
        )
    }

    /// Generate a versioned data key for an object chunk.
    #[allow(dead_code)]
    fn versioned_chunk_key(bucket: &str, key: &str, version_id: &str, chunk_index: u32) -> String {
        format!(
            "vault:{}:{}:{}:{}:{}:{}:{}",
            S3_VAULT_PREFIX,
            bucket,
            VERSION_DATA_PREFIX,
            key,
            version_id,
            CHUNK_KEY_COMPONENT,
            chunk_index
        )
    }

    /// Generate a delete marker metadata key.
    #[allow(dead_code)]
    fn delete_marker_key(bucket: &str, key: &str, version_id: &str) -> String {
        format!(
            "vault:{}:{}:_del_marker:{}:{}",
            S3_VAULT_PREFIX, bucket, key, version_id
        )
    }

    /// Store a versioned object.
    #[allow(dead_code)]
    async fn store_versioned_object(
        &self,
        bucket: &str,
        key: &str,
        data: &[u8],
        content_type: Option<String>,
        version_id: &str,
    ) -> Result<ObjectMetadata, KeyValueStoreError> {
        let chunk_size = S3_CHUNK_SIZE_BYTES as usize;
        let total_size = data.len();

        // Calculate ETag
        let etag = Self::calculate_etag(data);

        // Determine content type
        let content_type = content_type.unwrap_or_else(|| Self::infer_content_type(key));

        if total_size > chunk_size {
            // Store as chunked object
            let chunk_count = total_size.div_ceil(chunk_size) as u32;

            if chunk_count > MAX_CHUNKS_PER_OBJECT {
                return Err(KeyValueStoreError::Failed {
                    reason: format!(
                        "Object would require {} chunks, exceeding maximum of {}",
                        chunk_count, MAX_CHUNKS_PER_OBJECT
                    ),
                });
            }

            // Store each chunk with version ID
            for i in 0..chunk_count {
                let start = (i as usize) * chunk_size;
                let end = std::cmp::min(start + chunk_size, total_size);
                let chunk_data = &data[start..end];

                let chunk_key = Self::versioned_chunk_key(bucket, key, version_id, i);
                let encoded_chunk =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, chunk_data);

                self.kv_store
                    .write(WriteRequest {
                        command: WriteCommand::Set {
                            key: chunk_key,
                            value: encoded_chunk,
                        },
                    })
                    .await?;
            }

            // Create metadata
            let mut metadata = ObjectMetadata::new_chunked(
                total_size as u64,
                etag,
                content_type,
                chunk_count,
                S3_CHUNK_SIZE_BYTES,
            );
            metadata.version_id = Some(version_id.to_string());

            // Store versioned metadata
            let meta_key = Self::versioned_metadata_key(bucket, key, version_id);
            let meta_value =
                serde_json::to_string(&metadata).map_err(|e| KeyValueStoreError::Failed {
                    reason: format!("Failed to serialize object metadata: {}", e),
                })?;

            self.kv_store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: meta_key,
                        value: meta_value,
                    },
                })
                .await?;

            Ok(metadata)
        } else {
            // Store as simple object
            let data_key = Self::versioned_data_key(bucket, key, version_id);
            let encoded_data =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data);

            self.kv_store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: data_key,
                        value: encoded_data,
                    },
                })
                .await?;

            // Create metadata
            let mut metadata = ObjectMetadata::new_simple(total_size as u64, etag, content_type);
            metadata.version_id = Some(version_id.to_string());

            // Store versioned metadata
            let meta_key = Self::versioned_metadata_key(bucket, key, version_id);
            let meta_value =
                serde_json::to_string(&metadata).map_err(|e| KeyValueStoreError::Failed {
                    reason: format!("Failed to serialize object metadata: {}", e),
                })?;

            self.kv_store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: meta_key,
                        value: meta_value,
                    },
                })
                .await?;

            Ok(metadata)
        }
    }

    /// Get versioned object data.
    #[allow(dead_code)]
    async fn get_versioned_object(
        &self,
        bucket: &str,
        key: &str,
        version_id: &str,
        metadata: &ObjectMetadata,
    ) -> Result<Vec<u8>, S3Error> {
        if metadata.is_chunked() {
            // Get chunked data
            let mut data = Vec::with_capacity(metadata.size_bytes as usize);

            for i in 0..metadata.chunk_count {
                let chunk_key = Self::versioned_chunk_key(bucket, key, version_id, i);

                let result = self
                    .kv_store
                    .read(ReadRequest { key: chunk_key })
                    .await
                    .map_err(|e| S3Error::ChunkingError {
                        key: key.to_string(),
                        reason: format!("Failed to read chunk {}: {}", i, e),
                    })?;

                let chunk_data = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &result.value,
                )
                .map_err(|e| S3Error::ChunkingError {
                    key: key.to_string(),
                    reason: format!("Failed to decode chunk {}: {}", i, e),
                })?;

                data.extend_from_slice(&chunk_data);
            }

            Ok(data)
        } else {
            // Get simple data
            let data_key = Self::versioned_data_key(bucket, key, version_id);

            let result = self
                .kv_store
                .read(ReadRequest { key: data_key })
                .await
                .map_err(|e| match e {
                    KeyValueStoreError::NotFound { .. } => S3Error::ObjectNotFound {
                        bucket: bucket.to_string(),
                        key: key.to_string(),
                    },
                    _ => S3Error::StorageError {
                        source: anyhow::anyhow!("{}", e),
                    },
                })?;

            let data =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &result.value)
                    .map_err(|e| S3Error::StorageError {
                    source: anyhow::anyhow!("Failed to decode object data: {}", e),
                })?;

            Ok(data)
        }
    }

    /// Get versioned object metadata.
    #[allow(dead_code)]
    async fn get_versioned_metadata(
        &self,
        bucket: &str,
        key: &str,
        version_id: &str,
    ) -> Result<Option<ObjectMetadata>, KeyValueStoreError> {
        let meta_key = Self::versioned_metadata_key(bucket, key, version_id);
        match self.kv_store.read(ReadRequest { key: meta_key }).await {
            Ok(result) => {
                let meta: ObjectMetadata = serde_json::from_str(&result.value).map_err(|e| {
                    KeyValueStoreError::Failed {
                        reason: format!("Failed to deserialize object metadata: {}", e),
                    }
                })?;
                Ok(Some(meta))
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Store a delete marker for versioned deletion.
    #[allow(dead_code)]
    async fn store_delete_marker(
        &self,
        bucket: &str,
        key: &str,
        version_id: &str,
    ) -> Result<DeleteMarkerMetadata, KeyValueStoreError> {
        let marker = DeleteMarkerMetadata::new(key.to_string(), version_id.to_string(), true);

        let marker_key = Self::delete_marker_key(bucket, key, version_id);
        let marker_value =
            serde_json::to_string(&marker).map_err(|e| KeyValueStoreError::Failed {
                reason: format!("Failed to serialize delete marker: {}", e),
            })?;

        self.kv_store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: marker_key,
                    value: marker_value,
                },
            })
            .await?;

        Ok(marker)
    }

    /// Check if a delete marker exists for a key.
    #[allow(dead_code)]
    async fn get_delete_marker(
        &self,
        bucket: &str,
        key: &str,
        version_id: &str,
    ) -> Result<Option<DeleteMarkerMetadata>, KeyValueStoreError> {
        let marker_key = Self::delete_marker_key(bucket, key, version_id);
        match self.kv_store.read(ReadRequest { key: marker_key }).await {
            Ok(result) => {
                let marker: DeleteMarkerMetadata =
                    serde_json::from_str(&result.value).map_err(|e| {
                        KeyValueStoreError::Failed {
                            reason: format!("Failed to deserialize delete marker: {}", e),
                        }
                    })?;
                Ok(Some(marker))
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Update the "current" pointer for a versioned object.
    ///
    /// This stores the latest version ID in the standard object metadata location.
    #[allow(dead_code)]
    async fn update_current_version(
        &self,
        bucket: &str,
        key: &str,
        metadata: &ObjectMetadata,
    ) -> Result<(), KeyValueStoreError> {
        self.store_object_metadata(bucket, key, metadata).await
    }

    /// Delete versioned object data (a specific version).
    #[allow(dead_code)]
    async fn delete_versioned_object(
        &self,
        bucket: &str,
        key: &str,
        version_id: &str,
    ) -> Result<bool, S3Error> {
        // Get version metadata
        let metadata = match self.get_versioned_metadata(bucket, key, version_id).await {
            Ok(Some(m)) => m,
            Ok(None) => return Ok(false),
            Err(e) => {
                return Err(S3Error::StorageError {
                    source: anyhow::anyhow!("{}", e),
                });
            }
        };

        // Delete data
        if metadata.is_chunked() {
            for i in 0..metadata.chunk_count {
                let chunk_key = Self::versioned_chunk_key(bucket, key, version_id, i);
                let _ = self.kv_store.delete(DeleteRequest { key: chunk_key }).await;
            }
        } else {
            let data_key = Self::versioned_data_key(bucket, key, version_id);
            let _ = self.kv_store.delete(DeleteRequest { key: data_key }).await;
        }

        // Delete metadata
        let meta_key = Self::versioned_metadata_key(bucket, key, version_id);
        let _ = self.kv_store.delete(DeleteRequest { key: meta_key }).await;

        Ok(true)
    }

    /// List all versions of an object.
    #[allow(dead_code)]
    async fn list_object_versions(
        &self,
        bucket: &str,
        prefix: Option<&str>,
        max_keys: u32,
    ) -> Result<Vec<ObjectVersionEntry>, S3Error> {
        let prefix_str = prefix.unwrap_or("");

        // Scan for versioned object metadata
        let version_meta_prefix = format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, VERSION_METADATA_PREFIX, prefix_str
        );

        let scan_result = self
            .kv_store
            .scan(ScanRequest {
                prefix: version_meta_prefix.clone(),
                limit: Some(max_keys),
                continuation_token: None,
            })
            .await
            .map_err(|e| S3Error::StorageError {
                source: anyhow::anyhow!("Failed to scan versions: {}", e),
            })?;

        let base_prefix = format!(
            "vault:{}:{}:{}:",
            S3_VAULT_PREFIX, bucket, VERSION_METADATA_PREFIX
        );

        // Tiger Style: Pre-allocate with capacity based on scan results.
        let mut entries: Vec<ObjectVersionEntry> = Vec::with_capacity(scan_result.entries.len());

        for entry in scan_result.entries {
            // Extract key and version ID from the KV key
            if let Some(remainder) = entry.key.strip_prefix(&base_prefix) {
                // Format: {object_key}:{version_id}
                if let Some(last_colon) = remainder.rfind(':') {
                    let object_key = &remainder[..last_colon];
                    let version_id = &remainder[last_colon + 1..];

                    if let Ok(meta) = serde_json::from_str::<ObjectMetadata>(&entry.value) {
                        entries.push(ObjectVersionEntry::Version {
                            key: object_key.to_string(),
                            version_id: version_id.to_string(),
                            is_latest: false, // Will be updated later
                            last_modified: meta.last_modified,
                            etag: meta.etag.clone(),
                            size_bytes: meta.size_bytes,
                            storage_class: meta.storage_class.clone(),
                        });
                    }
                }
            }
        }

        // Also scan for delete markers
        let delete_marker_prefix = format!(
            "vault:{}:{}:_del_marker:{}",
            S3_VAULT_PREFIX, bucket, prefix_str
        );

        let delete_scan_result = self
            .kv_store
            .scan(ScanRequest {
                prefix: delete_marker_prefix,
                limit: Some(max_keys),
                continuation_token: None,
            })
            .await
            .map_err(|e| S3Error::StorageError {
                source: anyhow::anyhow!("Failed to scan delete markers: {}", e),
            })?;

        let delete_base_prefix = format!("vault:{}:{}:_del_marker:", S3_VAULT_PREFIX, bucket);

        for entry in delete_scan_result.entries {
            if let Some(remainder) = entry.key.strip_prefix(&delete_base_prefix) {
                // Format: {object_key}:{version_id}
                if let Some(last_colon) = remainder.rfind(':') {
                    let object_key = &remainder[..last_colon];
                    let version_id = &remainder[last_colon + 1..];

                    if let Ok(marker) = serde_json::from_str::<DeleteMarkerMetadata>(&entry.value) {
                        entries.push(ObjectVersionEntry::DeleteMarker {
                            key: object_key.to_string(),
                            version_id: version_id.to_string(),
                            is_latest: marker.is_latest,
                            last_modified: marker.last_modified,
                        });
                    }
                }
            }
        }

        // Sort by key, then by version_id (timestamp prefix ensures chronological order)
        entries.sort_by(|a, b| {
            let (key_a, ver_a) = match a {
                ObjectVersionEntry::Version {
                    key, version_id, ..
                } => (key, version_id),
                ObjectVersionEntry::DeleteMarker {
                    key, version_id, ..
                } => (key, version_id),
            };
            let (key_b, ver_b) = match b {
                ObjectVersionEntry::Version {
                    key, version_id, ..
                } => (key, version_id),
                ObjectVersionEntry::DeleteMarker {
                    key, version_id, ..
                } => (key, version_id),
            };
            // Sort by key ascending, then version descending (latest first)
            match key_a.cmp(key_b) {
                std::cmp::Ordering::Equal => ver_b.cmp(ver_a),
                other => other,
            }
        });

        // Mark the latest version for each key
        let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
        for entry in &mut entries {
            match entry {
                ObjectVersionEntry::Version { key, is_latest, .. } => {
                    if !seen_keys.contains(key) {
                        *is_latest = true;
                        seen_keys.insert(key.clone());
                    }
                }
                ObjectVersionEntry::DeleteMarker { key, is_latest, .. } => {
                    if !seen_keys.contains(key) {
                        *is_latest = true;
                        seen_keys.insert(key.clone());
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Convert internal LifecycleRule to s3s LifecycleRule.
    fn convert_lifecycle_rule_to_s3s(&self, rule: &LifecycleRule) -> s3s::dto::LifecycleRule {
        // Build expiration
        let expiration = rule
            .expiration
            .as_ref()
            .map(|exp| s3s::dto::LifecycleExpiration {
                days: exp.days.map(|d| d as i32),
                date: exp.date.map(chrono_to_timestamp),
                expired_object_delete_marker: exp.expired_object_delete_marker,
            });

        // Build transitions
        let transitions: Option<Vec<s3s::dto::Transition>> = if rule.transitions.is_empty() {
            None
        } else {
            Some(
                rule.transitions
                    .iter()
                    .map(|t| s3s::dto::Transition {
                        days: t.days.map(|d| d as i32),
                        date: t.date.map(chrono_to_timestamp),
                        storage_class: Some(TransitionStorageClass::from(t.storage_class.clone())),
                    })
                    .collect(),
            )
        };

        // Build noncurrent version expiration
        let noncurrent_version_expiration =
            rule.noncurrent_version_expiration.as_ref().map(|nve| {
                s3s::dto::NoncurrentVersionExpiration {
                    noncurrent_days: Some(nve.noncurrent_days as i32),
                    newer_noncurrent_versions: nve.newer_noncurrent_versions.map(|v| v as i32),
                }
            });

        // Build abort incomplete multipart upload
        let abort_incomplete_multipart_upload =
            rule.abort_incomplete_multipart_upload_days.map(|days| {
                s3s::dto::AbortIncompleteMultipartUpload {
                    days_after_initiation: Some(days as i32),
                }
            });

        // Build filter if there's a prefix
        let filter = rule.prefix.as_ref().map(|p| s3s::dto::LifecycleRuleFilter {
            prefix: Some(p.clone()),
            ..Default::default()
        });

        // ExpirationStatus is a wrapper around String, not an enum
        let status = match rule.status {
            LifecycleRuleStatus::Enabled => {
                ExpirationStatus::from_static(ExpirationStatus::ENABLED)
            }
            LifecycleRuleStatus::Disabled => {
                ExpirationStatus::from_static(ExpirationStatus::DISABLED)
            }
        };

        s3s::dto::LifecycleRule {
            id: Some(rule.id.clone()),
            status,
            filter,
            expiration,
            transitions,
            noncurrent_version_expiration,
            abort_incomplete_multipart_upload,
            noncurrent_version_transitions: None,
            prefix: None,
        }
    }

    /// Convert s3s LifecycleRule to internal LifecycleRule.
    fn convert_s3s_lifecycle_rule(
        &self,
        rule: &s3s::dto::LifecycleRule,
    ) -> Result<LifecycleRule, String> {
        // Get rule ID (required)
        let id = rule.id.clone().ok_or("Rule ID is required")?;

        // Tiger Style: Limit ID length to 255 chars per S3 spec
        if id.len() > 255 {
            return Err("Rule ID exceeds maximum length of 255 characters".to_string());
        }

        // Get status - ExpirationStatus is a wrapper type with as_str()
        let status = if rule.status.as_str() == ExpirationStatus::ENABLED {
            LifecycleRuleStatus::Enabled
        } else {
            LifecycleRuleStatus::Disabled
        };

        // Get prefix from filter (LifecycleRuleFilter is a struct, not an enum)
        let prefix = rule.filter.as_ref().and_then(|f| {
            // Check for prefix directly
            if let Some(p) = &f.prefix {
                Some(p.clone())
            } else if let Some(and) = &f.and {
                and.prefix.clone()
            } else {
                None
            }
        });

        // Convert expiration
        let expiration = rule.expiration.as_ref().map(|exp| LifecycleExpiration {
            days: exp.days.map(|d| d as u32),
            date: exp.date.as_ref().map(|ts| {
                // Convert s3s Timestamp to chrono DateTime
                // Timestamp wraps time::OffsetDateTime
                let offset_dt: time::OffsetDateTime = ts.clone().into();
                DateTime::from_timestamp(offset_dt.unix_timestamp(), 0).unwrap_or_default()
            }),
            expired_object_delete_marker: exp.expired_object_delete_marker,
        });

        // Convert transitions
        let transitions: Vec<LifecycleTransition> = rule
            .transitions
            .as_ref()
            .map(|ts| {
                ts.iter()
                    .map(|t| LifecycleTransition {
                        days: t.days.map(|d| d as u32),
                        date: t.date.as_ref().map(|ts| {
                            let offset_dt: time::OffsetDateTime = ts.clone().into();
                            DateTime::from_timestamp(offset_dt.unix_timestamp(), 0)
                                .unwrap_or_default()
                        }),
                        storage_class: t
                            .storage_class
                            .as_ref()
                            .map(|sc| sc.as_str().to_string())
                            .unwrap_or_else(|| "STANDARD".to_string()),
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Convert noncurrent version expiration
        let noncurrent_version_expiration =
            rule.noncurrent_version_expiration
                .as_ref()
                .map(|nve| NoncurrentVersionExpiration {
                    noncurrent_days: nve.noncurrent_days.unwrap_or(0) as u32,
                    newer_noncurrent_versions: nve.newer_noncurrent_versions.map(|v| v as u32),
                });

        // Get abort incomplete multipart upload days
        let abort_incomplete_multipart_upload_days = rule
            .abort_incomplete_multipart_upload
            .as_ref()
            .and_then(|a| a.days_after_initiation.map(|d| d as u32));

        Ok(LifecycleRule {
            id,
            status,
            prefix,
            expiration,
            transitions,
            noncurrent_version_expiration,
            abort_incomplete_multipart_upload_days,
        })
    }
}

/// S3 trait implementation.
///
/// This is the main interface required by the s3s crate.
/// We implement a subset of S3 operations for MVP.
#[async_trait]
impl S3 for AspenS3Service {
    // ===== Bucket Operations =====

    async fn list_buckets(
        &self,
        _req: S3Request<ListBucketsInput>,
    ) -> s3s::S3Result<S3Response<ListBucketsOutput>> {
        debug!("S3 ListBuckets request");

        // Scan for all bucket metadata keys
        // Bucket metadata keys have format: vault:s3:{bucket}:_bucket_meta
        let prefix = format!("vault:{}:", S3_VAULT_PREFIX);
        let suffix = format!(":{}", BUCKET_METADATA_SUFFIX);

        let scan_result = self
            .kv_store
            .scan(ScanRequest {
                prefix: prefix.clone(),
                limit: Some(MAX_VAULT_SCAN_KEYS),
                continuation_token: None,
            })
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to scan buckets: {}", e))?;

        // Extract bucket names from keys that end with _bucket_meta suffix
        // Tiger Style: Pre-allocate with capacity based on scan results.
        let mut buckets = Vec::with_capacity(scan_result.entries.len());
        for entry in scan_result.entries {
            if entry.key.ends_with(&suffix) {
                // Parse bucket metadata to get creation date
                if let Ok(meta) = serde_json::from_str::<BucketMetadata>(&entry.value) {
                    buckets.push(Bucket {
                        name: Some(meta.name),
                        creation_date: Some(chrono_to_timestamp(meta.created_at)),
                        bucket_region: None,
                    });
                }
            }
        }

        // Sort buckets by name for consistent ordering
        buckets.sort_by(|a, b| a.name.cmp(&b.name));

        debug!("ListBuckets found {} buckets", buckets.len());

        Ok(S3Response::new(ListBucketsOutput {
            buckets: Some(buckets),
            owner: Some(Owner {
                display_name: Some("aspen".to_string()),
                id: Some(self.node_id.to_string()),
            }),
            continuation_token: None,
            prefix: None,
        }))
    }

    async fn create_bucket(
        &self,
        req: S3Request<CreateBucketInput>,
    ) -> s3s::S3Result<S3Response<CreateBucketOutput>> {
        let bucket = req.input.bucket.as_str();
        info!("S3 CreateBucket request for bucket '{}'", bucket);

        // Validate bucket name
        Self::validate_bucket_name(bucket).map_err(|e| s3_error!(InvalidBucketName, "{}", e))?;

        // Check if bucket already exists
        match self.bucket_exists(bucket).await {
            Ok(true) => {
                return Err(s3_error!(
                    BucketAlreadyExists,
                    "Bucket '{}' already exists",
                    bucket
                ));
            }
            Ok(false) => {}
            Err(e) => {
                warn!("Failed to check bucket existence: {}", e);
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Create bucket metadata
        let metadata = BucketMetadata::new(bucket.to_string());
        let meta_key = Self::bucket_metadata_key(bucket);
        let meta_value = serde_json::to_string(&metadata)
            .map_err(|e| s3_error!(InternalError, "Failed to serialize metadata: {}", e))?;

        self.kv_store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: meta_key,
                    value: meta_value,
                },
            })
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to create bucket: {}", e))?;

        info!("Created bucket '{}'", bucket);
        Ok(S3Response::new(CreateBucketOutput {
            location: Some(format!("/{}", bucket)),
        }))
    }

    async fn delete_bucket(
        &self,
        req: S3Request<DeleteBucketInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketOutput>> {
        let bucket = req.input.bucket.as_str();
        info!("S3 DeleteBucket request for bucket '{}'", bucket);

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Check if bucket is empty by scanning for any objects
        let kv_prefix = format!(
            "vault:{}:{}:{}:",
            S3_VAULT_PREFIX, bucket, OBJECT_METADATA_PREFIX
        );

        let scan_result = self
            .kv_store
            .scan(ScanRequest {
                prefix: kv_prefix,
                limit: Some(1), // We only need to know if any objects exist
                continuation_token: None,
            })
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to check bucket contents: {}", e))?;

        if !scan_result.entries.is_empty() {
            return Err(s3_error!(
                BucketNotEmpty,
                "Bucket '{}' is not empty",
                bucket
            ));
        }

        let meta_key = Self::bucket_metadata_key(bucket);
        self.kv_store
            .delete(DeleteRequest { key: meta_key })
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to delete bucket: {}", e))?;

        info!("Deleted bucket '{}'", bucket);
        Ok(S3Response::new(DeleteBucketOutput {}))
    }

    async fn head_bucket(
        &self,
        req: S3Request<HeadBucketInput>,
    ) -> s3s::S3Result<S3Response<HeadBucketOutput>> {
        let bucket = req.input.bucket.as_str();
        debug!("S3 HeadBucket request for bucket '{}'", bucket);

        match self.bucket_exists(bucket).await {
            Ok(true) => Ok(S3Response::new(HeadBucketOutput {
                bucket_region: Some("us-east-1".to_string()),
                ..Default::default()
            })),
            Ok(false) => Err(s3_error!(
                NoSuchBucket,
                "Bucket '{}' does not exist",
                bucket
            )),
            Err(e) => Err(s3_error!(InternalError, "Failed to check bucket: {}", e)),
        }
    }

    // ===== Object Operations =====

    async fn put_object(
        &self,
        req: S3Request<PutObjectInput>,
    ) -> s3s::S3Result<S3Response<PutObjectOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        info!("S3 PutObject request for {}/{}", bucket, key);

        // Validate inputs
        Self::validate_object_key(key)
            .map_err(|e| s3_error!(InvalidArgument, "Invalid object key: {}", e))?;

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Read body data
        // Tiger Style: Use content_length hint for pre-allocation when available,
        // falling back to chunk size as a reasonable default capacity.
        let initial_capacity = req
            .input
            .content_length
            .map(|len| len as usize)
            .unwrap_or(S3_CHUNK_SIZE_BYTES as usize);
        let body = match req.input.body {
            Some(stream) => {
                let bytes: Vec<u8> = stream
                    .try_fold(
                        Vec::with_capacity(initial_capacity),
                        |mut acc, chunk| async move {
                            acc.extend_from_slice(&chunk);
                            Ok(acc)
                        },
                    )
                    .await
                    .map_err(|e| {
                        s3_error!(InternalError, "Failed to read request body: {:?}", e)
                    })?;
                bytes
            }
            // Tiger Style: Empty body still needs explicit Vec, but with zero capacity
            // to avoid unnecessary allocation.
            None => Vec::with_capacity(0),
        };

        // Check size limit
        if body.len() as u64 > MAX_S3_OBJECT_SIZE_BYTES {
            return Err(s3_error!(
                EntityTooLarge,
                "Object size {} exceeds maximum {}",
                body.len(),
                MAX_S3_OBJECT_SIZE_BYTES
            ));
        }

        // Get content type from request or infer from key
        let content_type = req.input.content_type.map(|s| s.to_string());

        // Store object (chunked if > 1MB)
        let metadata = if body.len() > S3_CHUNK_SIZE_BYTES as usize {
            debug!("Storing chunked object {} bytes", body.len());
            self.store_chunked_object(bucket, key, &body, content_type)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to store object: {}", e))?
        } else {
            debug!("Storing simple object {} bytes", body.len());
            self.store_simple_object(bucket, key, &body, content_type)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to store object: {}", e))?
        };

        info!(
            "Stored object {}/{} ({} bytes, {} chunks)",
            bucket,
            key,
            metadata.size_bytes,
            if metadata.is_chunked() {
                metadata.chunk_count
            } else {
                0
            }
        );

        Ok(S3Response::new(PutObjectOutput {
            e_tag: Some(metadata.etag),
            ..Default::default()
        }))
    }

    async fn get_object(
        &self,
        req: S3Request<GetObjectInput>,
    ) -> s3s::S3Result<S3Response<GetObjectOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        let range = req.input.range.as_ref();
        info!(
            "S3 GetObject request for {}/{} (range: {:?})",
            bucket, key, range
        );

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Get object metadata
        let metadata = match self.get_object_metadata(bucket, key).await {
            Ok(Some(m)) => m,
            Ok(None) => return Err(s3_error!(NoSuchKey, "Object not found")),
            Err(e) => return Err(s3_error!(InternalError, "Failed to get metadata: {}", e)),
        };

        // Resolve byte range if requested
        let resolved_range = match range {
            Some(r) => Some(
                resolve_range(r, metadata.size_bytes)
                    .map_err(|e| s3_error!(InvalidRange, "Invalid range: {}", e))?,
            ),
            None => None,
        };

        // Retrieve object data
        let full_data = if metadata.is_chunked() {
            self.get_chunked_object(bucket, key, &metadata)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to get object: {}", e))?
        } else {
            self.get_simple_object(bucket, key)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to get object: {}", e))?
        };

        // Extract requested byte range if applicable
        let (data, content_length, content_range) = match resolved_range {
            Some(r) => {
                let start = r.start as usize;
                let end = r.end as usize;
                // Validate range against actual data length
                if start >= full_data.len() {
                    return Err(s3_error!(
                        InvalidRange,
                        "Range start {} exceeds data length {}",
                        start,
                        full_data.len()
                    ));
                }
                let actual_end = end.min(full_data.len() - 1);
                let partial_data = full_data[start..=actual_end].to_vec();
                let range_len = partial_data.len() as i64;
                let range_header =
                    format_content_range(start as u64, actual_end as u64, metadata.size_bytes);
                debug!(
                    "Returning partial content: {} bytes ({}-{})",
                    range_len, start, actual_end
                );
                (partial_data, range_len, Some(range_header))
            }
            None => (full_data, metadata.size_bytes as i64, None),
        };

        // Convert to streaming body
        let body = bytes_to_streaming_blob(data);

        Ok(S3Response::new(GetObjectOutput {
            body: Some(body),
            content_length: Some(content_length),
            content_type: parse_content_type(&metadata.content_type),
            content_range,
            accept_ranges: Some("bytes".to_string()),
            e_tag: Some(metadata.etag),
            last_modified: Some(chrono_to_timestamp(metadata.last_modified)),
            ..Default::default()
        }))
    }

    async fn delete_object(
        &self,
        req: S3Request<DeleteObjectInput>,
    ) -> s3s::S3Result<S3Response<DeleteObjectOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        info!("S3 DeleteObject request for {}/{}", bucket, key);

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Delete object (S3 delete is idempotent - no error if object doesn't exist)
        let _ = self.delete_object_data(bucket, key).await;

        Ok(S3Response::new(DeleteObjectOutput::default()))
    }

    async fn delete_objects(
        &self,
        req: S3Request<DeleteObjectsInput>,
    ) -> s3s::S3Result<S3Response<DeleteObjectsOutput>> {
        let bucket = req.input.bucket.as_str();
        let objects = &req.input.delete.objects;
        let quiet = req.input.delete.quiet.unwrap_or(false);

        info!(
            "S3 DeleteObjects request for bucket '{}' with {} objects, quiet={}",
            bucket,
            objects.len(),
            quiet
        );

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Tiger Style: Pre-allocate with capacity based on input size.
        let mut deleted: Vec<DeletedObject> = Vec::with_capacity(objects.len());
        let mut errors: Vec<s3s::dto::Error> = Vec::with_capacity(objects.len());

        // Delete each object
        for obj in objects {
            let key = obj.key.as_str();

            match self.delete_object_data(bucket, key).await {
                Ok(_) => {
                    // Only report deleted objects if not in quiet mode
                    if !quiet {
                        deleted.push(DeletedObject {
                            key: Some(key.to_string()),
                            delete_marker: None,
                            delete_marker_version_id: None,
                            version_id: None,
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to delete object {}/{}: {}", bucket, key, e);
                    errors.push(s3s::dto::Error {
                        code: Some("InternalError".to_string()),
                        key: Some(key.to_string()),
                        message: Some(e.to_string()),
                        version_id: None,
                    });
                }
            }
        }

        debug!(
            "DeleteObjects completed: {} deleted, {} errors",
            deleted.len(),
            errors.len()
        );

        Ok(S3Response::new(DeleteObjectsOutput {
            deleted: if deleted.is_empty() {
                None
            } else {
                Some(deleted)
            },
            errors: if errors.is_empty() {
                None
            } else {
                Some(errors)
            },
            request_charged: None,
        }))
    }

    async fn head_object(
        &self,
        req: S3Request<HeadObjectInput>,
    ) -> s3s::S3Result<S3Response<HeadObjectOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        debug!("S3 HeadObject request for {}/{}", bucket, key);

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Get object metadata
        let metadata = match self.get_object_metadata(bucket, key).await {
            Ok(Some(m)) => m,
            Ok(None) => return Err(s3_error!(NoSuchKey, "Object not found")),
            Err(e) => return Err(s3_error!(InternalError, "Failed to get metadata: {}", e)),
        };

        Ok(S3Response::new(HeadObjectOutput {
            content_length: Some(metadata.size_bytes as i64),
            content_type: parse_content_type(&metadata.content_type),
            accept_ranges: Some("bytes".to_string()),
            e_tag: Some(metadata.etag),
            last_modified: Some(chrono_to_timestamp(metadata.last_modified)),
            ..Default::default()
        }))
    }

    async fn list_objects_v2(
        &self,
        req: S3Request<ListObjectsV2Input>,
    ) -> s3s::S3Result<S3Response<ListObjectsV2Output>> {
        let bucket = req.input.bucket.as_str();
        let s3_prefix = req.input.prefix.as_deref().unwrap_or("");
        let delimiter = req.input.delimiter.as_deref();
        let max_keys = req
            .input
            .max_keys
            .unwrap_or(1000)
            .min(MAX_LIST_OBJECTS as i32) as usize;

        debug!(
            "S3 ListObjectsV2 request for bucket '{}' with prefix '{}', delimiter {:?}, max_keys {}",
            bucket, s3_prefix, delimiter, max_keys
        );

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Build the KV prefix for scanning object metadata keys
        // Object metadata keys have format: vault:s3:{bucket}:_meta:{key}
        let kv_prefix = format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_METADATA_PREFIX, s3_prefix
        );

        // Use continuation token from request (S3 continuation token maps to our KV token)
        let continuation_token = req.input.continuation_token.clone();

        // Request one extra to detect truncation
        let scan_result = self
            .kv_store
            .scan(ScanRequest {
                prefix: kv_prefix.clone(),
                limit: Some((max_keys + 1) as u32),
                continuation_token,
            })
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to scan objects: {}", e))?;

        // Extract the base prefix length to strip from keys
        let base_prefix = format!(
            "vault:{}:{}:{}:",
            S3_VAULT_PREFIX, bucket, OBJECT_METADATA_PREFIX
        );
        let base_prefix_len = base_prefix.len();

        // Process entries into S3 objects and common prefixes
        // Tiger Style: Pre-allocate with capacity based on scan results.
        let mut contents: Vec<Object> = Vec::with_capacity(scan_result.entries.len());
        let mut common_prefixes_set: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();

        for entry in &scan_result.entries {
            // Extract the S3 object key from the KV key
            if entry.key.len() <= base_prefix_len {
                continue;
            }
            let object_key = &entry.key[base_prefix_len..];

            // Handle delimiter-based hierarchical listing
            if let Some(delim) = delimiter
                && let Some(rel_key) = object_key.strip_prefix(s3_prefix)
                && let Some(pos) = rel_key.find(delim)
            {
                // This is a "directory" - add to common prefixes
                let common_prefix = format!("{}{}{}", s3_prefix, &rel_key[..pos], delim);
                common_prefixes_set.insert(common_prefix);
                continue;
            }

            // Parse object metadata
            if let Ok(meta) = serde_json::from_str::<ObjectMetadata>(&entry.value) {
                contents.push(Object {
                    key: Some(object_key.to_string()),
                    size: Some(meta.size_bytes as i64),
                    e_tag: Some(meta.etag),
                    last_modified: Some(chrono_to_timestamp(meta.last_modified)),
                    storage_class: Some(ObjectStorageClass::STANDARD.to_string().into()),
                    ..Default::default()
                });
            }

            // Stop if we've reached max_keys
            if contents.len() >= max_keys {
                break;
            }
        }

        // Determine if truncated
        let is_truncated = scan_result.is_truncated || scan_result.entries.len() > max_keys;

        // Build common prefixes list
        let common_prefixes: Vec<CommonPrefix> = common_prefixes_set
            .into_iter()
            .map(|p| CommonPrefix { prefix: Some(p) })
            .collect();

        // Generate next continuation token if truncated
        let next_continuation_token = if is_truncated {
            scan_result.continuation_token
        } else {
            None
        };

        let key_count = contents.len() as i32;

        debug!(
            "ListObjectsV2 found {} objects, {} common prefixes, truncated: {}",
            key_count,
            common_prefixes.len(),
            is_truncated
        );

        Ok(S3Response::new(ListObjectsV2Output {
            name: Some(bucket.to_string()),
            prefix: req.input.prefix.clone(),
            delimiter: req.input.delimiter.clone(),
            max_keys: Some(max_keys as i32),
            is_truncated: Some(is_truncated),
            key_count: Some(key_count),
            contents: Some(contents),
            common_prefixes: Some(common_prefixes),
            next_continuation_token,
            continuation_token: req.input.continuation_token.clone(),
            start_after: req.input.start_after.clone(),
            ..Default::default()
        }))
    }

    // ===== Copy Operation =====

    async fn copy_object(
        &self,
        req: S3Request<CopyObjectInput>,
    ) -> s3s::S3Result<S3Response<CopyObjectOutput>> {
        let dest_bucket = req.input.bucket.as_str();
        let dest_key = req.input.key.as_str();

        let (src_bucket, src_key) = match &req.input.copy_source {
            CopySource::Bucket {
                bucket,
                key,
                version_id: _,
            } => (bucket.as_ref(), key.as_ref()),
            CopySource::AccessPoint { .. } => {
                return Err(s3_error!(
                    NotImplemented,
                    "Access point copy not yet supported"
                ));
            }
        };

        info!(
            "S3 CopyObject from {}/{} to {}/{}",
            src_bucket, src_key, dest_bucket, dest_key
        );

        // Check source bucket exists
        match self.bucket_exists(src_bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Source bucket '{}' does not exist",
                    src_bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Check destination bucket exists
        match self.bucket_exists(dest_bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Destination bucket '{}' does not exist",
                    dest_bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Get source object metadata
        let src_metadata = match self.get_object_metadata(src_bucket, src_key).await {
            Ok(Some(m)) => m,
            Ok(None) => return Err(s3_error!(NoSuchKey, "Source object not found")),
            Err(e) => return Err(s3_error!(InternalError, "Failed to get metadata: {}", e)),
        };

        // Get source object data
        let data = if src_metadata.is_chunked() {
            self.get_chunked_object(src_bucket, src_key, &src_metadata)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to read source object: {}", e))?
        } else {
            self.get_simple_object(src_bucket, src_key)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to read source object: {}", e))?
        };

        // Store to destination (reuse content type from source)
        let dest_metadata = if data.len() > S3_CHUNK_SIZE_BYTES as usize {
            self.store_chunked_object(
                dest_bucket,
                dest_key,
                &data,
                Some(src_metadata.content_type.clone()),
            )
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to store object: {}", e))?
        } else {
            self.store_simple_object(
                dest_bucket,
                dest_key,
                &data,
                Some(src_metadata.content_type.clone()),
            )
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to store object: {}", e))?
        };

        Ok(S3Response::new(CopyObjectOutput {
            copy_object_result: Some(CopyObjectResult {
                e_tag: Some(dest_metadata.etag),
                last_modified: Some(chrono_to_timestamp(dest_metadata.last_modified)),
                ..Default::default()
            }),
            ..Default::default()
        }))
    }

    // ===== Multipart Upload Operations =====

    async fn create_multipart_upload(
        &self,
        req: S3Request<CreateMultipartUploadInput>,
    ) -> s3s::S3Result<S3Response<CreateMultipartUploadOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        info!("S3 CreateMultipartUpload request for {}/{}", bucket, key);

        // Validate inputs
        Self::validate_object_key(key)
            .map_err(|e| s3_error!(InvalidArgument, "Invalid object key: {}", e))?;

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Generate unique upload ID
        let upload_id = uuid::Uuid::new_v4().to_string();

        // Get content type from request
        let content_type = req.input.content_type.map(|s| s.to_string());

        // Create and store upload metadata
        let metadata = MultipartUploadMetadata::new(
            upload_id.clone(),
            bucket.to_string(),
            key.to_string(),
            content_type,
        );

        self.store_multipart_metadata(&metadata)
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to create multipart upload: {}", e))?;

        info!(
            "Created multipart upload for {}/{} with upload_id={}",
            bucket, key, upload_id
        );

        Ok(S3Response::new(CreateMultipartUploadOutput {
            bucket: Some(bucket.to_string()),
            key: Some(key.to_string()),
            upload_id: Some(upload_id),
            ..Default::default()
        }))
    }

    async fn upload_part(
        &self,
        req: S3Request<UploadPartInput>,
    ) -> s3s::S3Result<S3Response<UploadPartOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        let upload_id = req.input.upload_id.as_str();
        let part_number = req.input.part_number;

        info!(
            "S3 UploadPart request for {}/{} upload_id={} part={}",
            bucket, key, upload_id, part_number
        );

        // Validate part number
        if part_number < 1 || part_number > MAX_MULTIPART_PARTS as i32 {
            return Err(s3_error!(
                InvalidArgument,
                "Part number must be between 1 and {}",
                MAX_MULTIPART_PARTS
            ));
        }

        // Get multipart upload metadata
        let mut metadata = match self.get_multipart_metadata(bucket, upload_id).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(
                    NoSuchUpload,
                    "Upload ID '{}' not found",
                    upload_id
                ));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get upload metadata: {}",
                    e
                ));
            }
        };

        // Verify key matches
        if metadata.key != key {
            return Err(s3_error!(
                InvalidArgument,
                "Key mismatch for upload ID '{}'",
                upload_id
            ));
        }

        // Read body data
        // Tiger Style: Use content_length hint for pre-allocation when available,
        // falling back to chunk size as a reasonable default capacity.
        let initial_capacity = req
            .input
            .content_length
            .map(|len| len as usize)
            .unwrap_or(S3_CHUNK_SIZE_BYTES as usize);
        let body = match req.input.body {
            Some(stream) => {
                let bytes: Vec<u8> = stream
                    .try_fold(
                        Vec::with_capacity(initial_capacity),
                        |mut acc, chunk| async move {
                            acc.extend_from_slice(&chunk);
                            Ok(acc)
                        },
                    )
                    .await
                    .map_err(|e| {
                        s3_error!(InternalError, "Failed to read request body: {:?}", e)
                    })?;
                bytes
            }
            // Tiger Style: Empty body still needs explicit Vec, but with zero capacity
            // to avoid unnecessary allocation.
            None => Vec::with_capacity(0),
        };

        // Validate part size
        if body.len() as u64 > MAX_PART_SIZE_BYTES {
            return Err(s3_error!(
                EntityTooLarge,
                "Part size {} exceeds maximum {}",
                body.len(),
                MAX_PART_SIZE_BYTES
            ));
        }

        // Calculate ETag for this part
        let etag = Self::calculate_etag(&body);

        // Store part data
        let part_key = Self::multipart_part_data_key(bucket, upload_id, part_number as u32);
        let encoded_data =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &body);

        self.kv_store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: part_key,
                    value: encoded_data,
                },
            })
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to store part: {}", e))?;

        // Update metadata with this part
        let part_metadata = PartMetadata::new(part_number as u32, body.len() as u64, etag.clone());
        metadata.add_part(part_metadata);
        self.store_multipart_metadata(&metadata)
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to update upload metadata: {}", e))?;

        info!(
            "Uploaded part {} for {}/{} ({} bytes)",
            part_number,
            bucket,
            key,
            body.len()
        );

        Ok(S3Response::new(UploadPartOutput {
            e_tag: Some(etag),
            ..Default::default()
        }))
    }

    async fn complete_multipart_upload(
        &self,
        req: S3Request<CompleteMultipartUploadInput>,
    ) -> s3s::S3Result<S3Response<CompleteMultipartUploadOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        let upload_id = req.input.upload_id.as_str();

        info!(
            "S3 CompleteMultipartUpload request for {}/{} upload_id={}",
            bucket, key, upload_id
        );

        // Get multipart upload metadata
        let metadata = match self.get_multipart_metadata(bucket, upload_id).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(
                    NoSuchUpload,
                    "Upload ID '{}' not found",
                    upload_id
                ));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get upload metadata: {}",
                    e
                ));
            }
        };

        // Verify key matches
        if metadata.key != key {
            return Err(s3_error!(
                InvalidArgument,
                "Key mismatch for upload ID '{}'",
                upload_id
            ));
        }

        // Get parts from request
        // Tiger Style: Use with_capacity(0) for empty cases to avoid allocation overhead.
        let requested_parts = match &req.input.multipart_upload {
            Some(upload) => match &upload.parts {
                Some(parts) => parts.clone(),
                None => Vec::with_capacity(0),
            },
            None => Vec::with_capacity(0),
        };

        if requested_parts.is_empty() {
            return Err(s3_error!(MalformedXML, "No parts specified for completion"));
        }

        // Verify parts are in order and match
        let mut total_size: u64 = 0;
        // Tiger Style: Pre-allocate with capacity based on requested parts count.
        let mut collected_parts: Vec<&PartMetadata> = Vec::with_capacity(requested_parts.len());
        let mut last_part_number: i32 = 0;

        for completed_part in &requested_parts {
            let part_number = completed_part
                .part_number
                .ok_or_else(|| s3_error!(InvalidPart, "Part number is required"))?;

            // Parts must be in ascending order
            if part_number <= last_part_number {
                return Err(s3_error!(
                    InvalidPartOrder,
                    "Parts must be in ascending order"
                ));
            }
            last_part_number = part_number;

            // Find the part in our metadata
            let stored_part = metadata
                .parts
                .get(&(part_number as u32))
                .ok_or_else(|| s3_error!(InvalidPart, "Part {} not found", part_number))?;

            // Verify ETag matches
            if let Some(ref etag) = completed_part.e_tag {
                let requested_etag = etag.as_str().trim_matches('"');
                let stored_etag = stored_part.etag.trim_matches('"');
                if requested_etag != stored_etag {
                    return Err(s3_error!(
                        InvalidPart,
                        "ETag mismatch for part {}: expected {}, got {}",
                        part_number,
                        stored_etag,
                        requested_etag
                    ));
                }
            }

            total_size += stored_part.size_bytes;
            collected_parts.push(stored_part);
        }

        // Assemble the object from parts
        let mut object_data = Vec::with_capacity(total_size as usize);
        for part in &collected_parts {
            let part_key = Self::multipart_part_data_key(bucket, upload_id, part.part_number);
            let result = self
                .kv_store
                .read(ReadRequest { key: part_key })
                .await
                .map_err(|e| {
                    s3_error!(
                        InternalError,
                        "Failed to read part {}: {}",
                        part.part_number,
                        e
                    )
                })?;

            let part_data =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &result.value)
                    .map_err(|e| {
                    s3_error!(
                        InternalError,
                        "Failed to decode part {}: {}",
                        part.part_number,
                        e
                    )
                })?;

            object_data.extend_from_slice(&part_data);
        }

        // Calculate composite ETag
        let etag = Self::calculate_multipart_etag(&collected_parts);

        // Determine content type
        let content_type = metadata
            .content_type
            .unwrap_or_else(|| Self::infer_content_type(key));

        // Store the final object using chunking if needed
        if object_data.len() > S3_CHUNK_SIZE_BYTES as usize {
            self.store_chunked_object(bucket, key, &object_data, Some(content_type))
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to store object: {}", e))?;
        } else {
            self.store_simple_object(bucket, key, &object_data, Some(content_type))
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to store object: {}", e))?;
        }

        // Update object metadata with multipart ETag (overwrite the one from store_*_object)
        let mut obj_metadata = self
            .get_object_metadata(bucket, key)
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to get object metadata: {}", e))?
            .ok_or_else(|| s3_error!(InternalError, "Object metadata not found after store"))?;

        obj_metadata.etag = etag.clone();
        self.store_object_metadata(bucket, key, &obj_metadata)
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to update object metadata: {}", e))?;

        // Delete multipart upload data
        self.delete_multipart_upload(bucket, upload_id)
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to cleanup multipart upload: {}", e))?;

        info!(
            "Completed multipart upload for {}/{} ({} bytes, {} parts)",
            bucket,
            key,
            total_size,
            collected_parts.len()
        );

        Ok(S3Response::new(CompleteMultipartUploadOutput {
            bucket: Some(bucket.to_string()),
            key: Some(key.to_string()),
            e_tag: Some(etag),
            location: Some(format!("/{}/{}", bucket, key)),
            ..Default::default()
        }))
    }

    async fn abort_multipart_upload(
        &self,
        req: S3Request<AbortMultipartUploadInput>,
    ) -> s3s::S3Result<S3Response<AbortMultipartUploadOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        let upload_id = req.input.upload_id.as_str();

        info!(
            "S3 AbortMultipartUpload request for {}/{} upload_id={}",
            bucket, key, upload_id
        );

        // Verify upload exists
        let metadata = match self.get_multipart_metadata(bucket, upload_id).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(
                    NoSuchUpload,
                    "Upload ID '{}' not found",
                    upload_id
                ));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get upload metadata: {}",
                    e
                ));
            }
        };

        // Verify key matches
        if metadata.key != key {
            return Err(s3_error!(
                InvalidArgument,
                "Key mismatch for upload ID '{}'",
                upload_id
            ));
        }

        // Delete all parts and metadata
        self.delete_multipart_upload(bucket, upload_id)
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to abort multipart upload: {}", e))?;

        info!("Aborted multipart upload for {}/{}", bucket, key);

        Ok(S3Response::new(AbortMultipartUploadOutput::default()))
    }

    async fn list_parts(
        &self,
        req: S3Request<ListPartsInput>,
    ) -> s3s::S3Result<S3Response<ListPartsOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        let upload_id = req.input.upload_id.as_str();

        debug!(
            "S3 ListParts request for {}/{} upload_id={}",
            bucket, key, upload_id
        );

        // Get multipart upload metadata
        let metadata = match self.get_multipart_metadata(bucket, upload_id).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(
                    NoSuchUpload,
                    "Upload ID '{}' not found",
                    upload_id
                ));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get upload metadata: {}",
                    e
                ));
            }
        };

        // Verify key matches
        if metadata.key != key {
            return Err(s3_error!(
                InvalidArgument,
                "Key mismatch for upload ID '{}'",
                upload_id
            ));
        }

        // Convert parts to s3s Part type
        let sorted_parts = metadata.get_sorted_parts();
        let parts: Vec<Part> = sorted_parts
            .iter()
            .map(|p| Part {
                part_number: Some(p.part_number as i32),
                size: Some(p.size_bytes as i64),
                e_tag: Some(p.etag.clone()),
                last_modified: Some(chrono_to_timestamp(p.last_modified)),
                ..Default::default()
            })
            .collect();

        Ok(S3Response::new(ListPartsOutput {
            bucket: Some(bucket.to_string()),
            key: Some(key.to_string()),
            upload_id: Some(upload_id.to_string()),
            parts: Some(parts),
            is_truncated: Some(false),
            storage_class: Some(StorageClass::from(metadata.storage_class.clone())),
            ..Default::default()
        }))
    }

    // ===== Bucket Versioning Operations =====

    async fn get_bucket_versioning(
        &self,
        req: S3Request<GetBucketVersioningInput>,
    ) -> s3s::S3Result<S3Response<GetBucketVersioningOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 GetBucketVersioning request for bucket '{}'", bucket);

        // Get bucket metadata
        let metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        // Convert versioning status to s3s type
        // BucketVersioningStatus is a wrapper struct around String, not an enum
        let status = match metadata.versioning {
            BucketVersioning::Enabled => Some(BucketVersioningStatus::from_static(
                BucketVersioningStatus::ENABLED,
            )),
            BucketVersioning::Suspended => Some(BucketVersioningStatus::from_static(
                BucketVersioningStatus::SUSPENDED,
            )),
            BucketVersioning::Disabled => None,
        };

        // MFADeleteStatus is also a wrapper struct
        let mfa_delete = if metadata.mfa_delete {
            Some(MFADeleteStatus::from_static(MFADeleteStatus::ENABLED))
        } else {
            None
        };

        info!(
            "GetBucketVersioning for '{}': status={:?}, mfa_delete={:?}",
            bucket, status, mfa_delete
        );

        Ok(S3Response::new(GetBucketVersioningOutput {
            status,
            mfa_delete,
        }))
    }

    async fn put_bucket_versioning(
        &self,
        req: S3Request<PutBucketVersioningInput>,
    ) -> s3s::S3Result<S3Response<PutBucketVersioningOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 PutBucketVersioning request for bucket '{}'", bucket);

        // Get current bucket metadata
        let mut metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        // Update versioning status from the request
        // VersioningConfiguration is NOT wrapped in Option in the input
        let new_status = {
            let versioning_config = &req.input.versioning_configuration;
            match &versioning_config.status {
                Some(s) => {
                    // BucketVersioningStatus is a wrapper struct, use as_str() to compare
                    if s.as_str() == BucketVersioningStatus::ENABLED {
                        BucketVersioning::Enabled
                    } else if s.as_str() == BucketVersioningStatus::SUSPENDED {
                        BucketVersioning::Suspended
                    } else {
                        BucketVersioning::Disabled
                    }
                }
                None => metadata.versioning, // Keep existing if not specified
            }
        };

        info!(
            "PutBucketVersioning for '{}': {:?} -> {:?}",
            bucket, metadata.versioning, new_status
        );

        metadata.versioning = new_status;

        // Save updated metadata
        if let Err(e) = self.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(PutBucketVersioningOutput {}))
    }

    // ===== Bucket Encryption Operations =====

    async fn get_bucket_encryption(
        &self,
        req: S3Request<GetBucketEncryptionInput>,
    ) -> s3s::S3Result<S3Response<GetBucketEncryptionOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 GetBucketEncryption request for bucket '{}'", bucket);

        // Get bucket metadata
        let metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        // Check if encryption is configured
        let encryption = match &metadata.encryption {
            Some(enc) => enc,
            None => {
                return Err(s3_error!(
                    ServerSideEncryptionConfigurationNotFoundError,
                    "No encryption configuration for bucket '{}'",
                    bucket
                ));
            }
        };

        // Convert to s3s type
        // s3s::dto::ServerSideEncryption is a wrapper struct around String
        let sse_algo = match &encryption.sse_algorithm {
            AspenServerSideEncryption::Aes256 => {
                s3s::dto::ServerSideEncryption::from_static(s3s::dto::ServerSideEncryption::AES256)
            }
            AspenServerSideEncryption::AwsKms { .. } => {
                s3s::dto::ServerSideEncryption::from_static(s3s::dto::ServerSideEncryption::AWS_KMS)
            }
            AspenServerSideEncryption::None => {
                return Err(s3_error!(
                    ServerSideEncryptionConfigurationNotFoundError,
                    "No encryption configured for bucket '{}'",
                    bucket
                ));
            }
        };

        let kms_key_id = match &encryption.sse_algorithm {
            AspenServerSideEncryption::AwsKms { key_id } => key_id.clone(),
            _ => None,
        };

        let rule = ServerSideEncryptionRule {
            apply_server_side_encryption_by_default: Some(ServerSideEncryptionByDefault {
                sse_algorithm: sse_algo,
                kms_master_key_id: kms_key_id,
            }),
            bucket_key_enabled: Some(encryption.bucket_key_enabled),
        };

        info!(
            "GetBucketEncryption for '{}': algorithm={:?}, bucket_key={}",
            bucket,
            encryption.sse_algorithm.to_algorithm_string(),
            encryption.bucket_key_enabled
        );

        Ok(S3Response::new(GetBucketEncryptionOutput {
            server_side_encryption_configuration: Some(ServerSideEncryptionConfiguration {
                rules: vec![rule],
            }),
        }))
    }

    async fn put_bucket_encryption(
        &self,
        req: S3Request<PutBucketEncryptionInput>,
    ) -> s3s::S3Result<S3Response<PutBucketEncryptionOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 PutBucketEncryption request for bucket '{}'", bucket);

        // Get current bucket metadata
        let mut metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        // Parse the encryption configuration from the request
        // s3s::dto::ServerSideEncryption is a wrapper struct around String
        let new_encryption = {
            let config = &req.input.server_side_encryption_configuration;
            if let Some(rule) = config.rules.first() {
                if let Some(default_enc) = &rule.apply_server_side_encryption_by_default {
                    let sse_algorithm = {
                        let algo_str = default_enc.sse_algorithm.as_str();
                        if algo_str == s3s::dto::ServerSideEncryption::AES256 {
                            AspenServerSideEncryption::Aes256
                        } else if algo_str == s3s::dto::ServerSideEncryption::AWS_KMS {
                            AspenServerSideEncryption::AwsKms {
                                key_id: default_enc.kms_master_key_id.clone(),
                            }
                        } else {
                            AspenServerSideEncryption::None
                        }
                    };

                    BucketEncryptionConfiguration {
                        sse_algorithm,
                        bucket_key_enabled: rule.bucket_key_enabled.unwrap_or(false),
                    }
                } else {
                    BucketEncryptionConfiguration::default()
                }
            } else {
                BucketEncryptionConfiguration::default()
            }
        };

        info!(
            "PutBucketEncryption for '{}': algorithm={:?}, bucket_key={}",
            bucket,
            new_encryption.sse_algorithm.to_algorithm_string(),
            new_encryption.bucket_key_enabled
        );

        metadata.encryption = Some(new_encryption);

        // Save updated metadata
        if let Err(e) = self.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(PutBucketEncryptionOutput {}))
    }

    async fn delete_bucket_encryption(
        &self,
        req: S3Request<DeleteBucketEncryptionInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketEncryptionOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 DeleteBucketEncryption request for bucket '{}'", bucket);

        // Get current bucket metadata
        let mut metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        info!("DeleteBucketEncryption for '{}'", bucket);

        metadata.encryption = None;

        // Save updated metadata
        if let Err(e) = self.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(DeleteBucketEncryptionOutput {}))
    }

    // ===== Bucket Lifecycle Operations =====

    async fn get_bucket_lifecycle_configuration(
        &self,
        req: S3Request<GetBucketLifecycleConfigurationInput>,
    ) -> s3s::S3Result<S3Response<GetBucketLifecycleConfigurationOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!(
            "S3 GetBucketLifecycleConfiguration request for bucket '{}'",
            bucket
        );

        // Get bucket metadata
        let metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        // Check if lifecycle is configured
        let lifecycle = match &metadata.lifecycle {
            Some(lc) if !lc.rules.is_empty() => lc,
            _ => {
                return Err(s3_error!(
                    NoSuchLifecycleConfiguration,
                    "No lifecycle configuration for bucket '{}'",
                    bucket
                ));
            }
        };

        // Convert internal rules to s3s rules
        let rules: Vec<s3s::dto::LifecycleRule> = lifecycle
            .rules
            .iter()
            .map(|r| self.convert_lifecycle_rule_to_s3s(r))
            .collect();

        info!(
            "GetBucketLifecycleConfiguration for '{}': {} rules",
            bucket,
            rules.len()
        );

        Ok(S3Response::new(GetBucketLifecycleConfigurationOutput {
            rules: Some(rules),
            ..Default::default()
        }))
    }

    async fn put_bucket_lifecycle_configuration(
        &self,
        req: S3Request<PutBucketLifecycleConfigurationInput>,
    ) -> s3s::S3Result<S3Response<PutBucketLifecycleConfigurationOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!(
            "S3 PutBucketLifecycleConfiguration request for bucket '{}'",
            bucket
        );

        // Get current bucket metadata
        let mut metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        // Parse lifecycle rules from request
        // lifecycle_configuration is Option<BucketLifecycleConfiguration> and rules is Vec
        let rules: Vec<LifecycleRule> = match &req.input.lifecycle_configuration {
            Some(lifecycle_config) => {
                let s3s_rules = &lifecycle_config.rules;

                // Tiger Style: Max 1000 rules per S3 spec
                if s3s_rules.len() > 1000 {
                    return Err(s3_error!(
                        InvalidRequest,
                        "Maximum 1000 lifecycle rules allowed, got {}",
                        s3s_rules.len()
                    ));
                }

                s3s_rules
                    .iter()
                    .map(|r| self.convert_s3s_lifecycle_rule(r))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| s3_error!(InvalidRequest, "Invalid lifecycle rule: {}", e))?
            }
            // Tiger Style: Use with_capacity(0) to explicitly indicate an empty collection
            // without heap allocation.
            None => Vec::with_capacity(0),
        };

        info!(
            "PutBucketLifecycleConfiguration for '{}': {} rules",
            bucket,
            rules.len()
        );

        metadata.lifecycle = if rules.is_empty() {
            None
        } else {
            Some(BucketLifecycleConfiguration { rules })
        };

        // Save updated metadata
        if let Err(e) = self.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(PutBucketLifecycleConfigurationOutput {
            ..Default::default()
        }))
    }

    async fn delete_bucket_lifecycle(
        &self,
        req: S3Request<DeleteBucketLifecycleInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketLifecycleOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 DeleteBucketLifecycle request for bucket '{}'", bucket);

        // Get current bucket metadata
        let mut metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        info!("DeleteBucketLifecycle for '{}'", bucket);

        metadata.lifecycle = None;

        // Save updated metadata
        if let Err(e) = self.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(DeleteBucketLifecycleOutput {}))
    }

    // ===== Object Tagging Operations =====

    async fn get_object_tagging(
        &self,
        req: S3Request<GetObjectTaggingInput>,
    ) -> s3s::S3Result<S3Response<GetObjectTaggingOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();

        debug!("S3 GetObjectTagging request for {}/{}", bucket, key);

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Check if object exists
        match self.get_object_metadata(bucket, key).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                return Err(s3_error!(NoSuchKey, "Object '{}' not found", key));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get object metadata: {}",
                    e
                ));
            }
        }

        // Get object tags
        let tags_key = Self::object_tags_key(bucket, key);
        let tag_set = match self.kv_store.read(ReadRequest { key: tags_key }).await {
            Ok(result) => {
                let tags: std::collections::HashMap<String, String> =
                    serde_json::from_str(&result.value)
                        .map_err(|e| s3_error!(InternalError, "Failed to parse tags: {}", e))?;
                tags.into_iter()
                    .map(|(k, v)| Tag {
                        key: Some(k),
                        value: Some(v),
                    })
                    .collect()
            }
            // Tiger Style: Explicitly empty collection with no allocation.
            Err(KeyValueStoreError::NotFound { .. }) => Vec::with_capacity(0),
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to read tags: {}", e));
            }
        };

        info!(
            "GetObjectTagging for {}/{}: {} tags",
            bucket,
            key,
            tag_set.len()
        );

        Ok(S3Response::new(GetObjectTaggingOutput {
            tag_set,
            version_id: None,
        }))
    }

    async fn put_object_tagging(
        &self,
        req: S3Request<PutObjectTaggingInput>,
    ) -> s3s::S3Result<S3Response<PutObjectTaggingOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();

        debug!("S3 PutObjectTagging request for {}/{}", bucket, key);

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Check if object exists
        match self.get_object_metadata(bucket, key).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                return Err(s3_error!(NoSuchKey, "Object '{}' not found", key));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get object metadata: {}",
                    e
                ));
            }
        }

        // Validate and convert tags
        let tag_set = &req.input.tagging.tag_set;

        // Tiger Style: Bounded tag count
        if tag_set.len() > MAX_OBJECT_TAGS {
            return Err(s3_error!(
                InvalidArgument,
                "Maximum {} tags allowed per object, got {}",
                MAX_OBJECT_TAGS,
                tag_set.len()
            ));
        }

        let mut tags_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for tag in tag_set {
            // Validate tag key/value lengths
            let tag_key = tag
                .key
                .as_ref()
                .ok_or_else(|| s3_error!(InvalidTag, "Tag key is required"))?;
            let tag_value = tag
                .value
                .as_ref()
                .ok_or_else(|| s3_error!(InvalidTag, "Tag value is required"))?;

            if tag_key.len() > MAX_TAG_KEY_LENGTH {
                return Err(s3_error!(
                    InvalidTag,
                    "Tag key exceeds maximum length of {} characters",
                    MAX_TAG_KEY_LENGTH
                ));
            }
            if tag_value.len() > MAX_TAG_VALUE_LENGTH {
                return Err(s3_error!(
                    InvalidTag,
                    "Tag value exceeds maximum length of {} characters",
                    MAX_TAG_VALUE_LENGTH
                ));
            }
            tags_map.insert(tag_key.clone(), tag_value.clone());
        }

        // Store tags
        let tags_key = Self::object_tags_key(bucket, key);
        let tags_value = serde_json::to_string(&tags_map)
            .map_err(|e| s3_error!(InternalError, "Failed to serialize tags: {}", e))?;

        self.kv_store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: tags_key,
                    value: tags_value,
                },
            })
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to store tags: {}", e))?;

        info!(
            "PutObjectTagging for {}/{}: {} tags",
            bucket,
            key,
            tags_map.len()
        );

        Ok(S3Response::new(PutObjectTaggingOutput { version_id: None }))
    }

    async fn delete_object_tagging(
        &self,
        req: S3Request<DeleteObjectTaggingInput>,
    ) -> s3s::S3Result<S3Response<DeleteObjectTaggingOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();

        debug!("S3 DeleteObjectTagging request for {}/{}", bucket, key);

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Check if object exists
        match self.get_object_metadata(bucket, key).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                return Err(s3_error!(NoSuchKey, "Object '{}' not found", key));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get object metadata: {}",
                    e
                ));
            }
        }

        // Delete tags (idempotent - no error if tags don't exist)
        let tags_key = Self::object_tags_key(bucket, key);
        let _ = self.kv_store.delete(DeleteRequest { key: tags_key }).await;

        info!("DeleteObjectTagging for {}/{}", bucket, key);

        Ok(S3Response::new(DeleteObjectTaggingOutput {
            version_id: None,
        }))
    }

    // ===== Bucket Tagging Operations =====

    async fn get_bucket_tagging(
        &self,
        req: S3Request<GetBucketTaggingInput>,
    ) -> s3s::S3Result<S3Response<GetBucketTaggingOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 GetBucketTagging request for bucket '{}'", bucket);

        // Get bucket metadata
        let metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        // Check if tags exist - S3 returns NoSuchTagSet if no tags are configured
        if metadata.tags.is_empty() {
            return Err(s3_error!(
                NoSuchTagSet,
                "No tags configured for bucket '{}'",
                bucket
            ));
        }

        let tag_set: Vec<Tag> = metadata
            .tags
            .into_iter()
            .map(|(k, v)| Tag {
                key: Some(k),
                value: Some(v),
            })
            .collect();

        info!("GetBucketTagging for '{}': {} tags", bucket, tag_set.len());

        Ok(S3Response::new(GetBucketTaggingOutput { tag_set }))
    }

    async fn put_bucket_tagging(
        &self,
        req: S3Request<PutBucketTaggingInput>,
    ) -> s3s::S3Result<S3Response<PutBucketTaggingOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 PutBucketTagging request for bucket '{}'", bucket);

        // Get current bucket metadata
        let mut metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        // Validate and convert tags
        let tag_set = &req.input.tagging.tag_set;

        // Tiger Style: Bounded tag count
        if tag_set.len() > MAX_BUCKET_TAGS {
            return Err(s3_error!(
                InvalidArgument,
                "Maximum {} tags allowed per bucket, got {}",
                MAX_BUCKET_TAGS,
                tag_set.len()
            ));
        }

        let mut tags_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for tag in tag_set {
            // Validate tag key/value lengths
            let tag_key = tag
                .key
                .as_ref()
                .ok_or_else(|| s3_error!(InvalidTag, "Tag key is required"))?;
            let tag_value = tag
                .value
                .as_ref()
                .ok_or_else(|| s3_error!(InvalidTag, "Tag value is required"))?;

            if tag_key.len() > MAX_TAG_KEY_LENGTH {
                return Err(s3_error!(
                    InvalidTag,
                    "Tag key exceeds maximum length of {} characters",
                    MAX_TAG_KEY_LENGTH
                ));
            }
            if tag_value.len() > MAX_TAG_VALUE_LENGTH {
                return Err(s3_error!(
                    InvalidTag,
                    "Tag value exceeds maximum length of {} characters",
                    MAX_TAG_VALUE_LENGTH
                ));
            }
            tags_map.insert(tag_key.clone(), tag_value.clone());
        }

        info!("PutBucketTagging for '{}': {} tags", bucket, tags_map.len());

        metadata.tags = tags_map;

        // Save updated metadata
        if let Err(e) = self.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(PutBucketTaggingOutput {}))
    }

    async fn delete_bucket_tagging(
        &self,
        req: S3Request<DeleteBucketTaggingInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketTaggingOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 DeleteBucketTagging request for bucket '{}'", bucket);

        // Get current bucket metadata
        let mut metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        info!("DeleteBucketTagging for '{}'", bucket);

        metadata.tags.clear();

        // Save updated metadata
        if let Err(e) = self.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(DeleteBucketTaggingOutput {}))
    }

    // ===== Bucket Location =====

    async fn get_bucket_location(
        &self,
        req: S3Request<GetBucketLocationInput>,
    ) -> s3s::S3Result<S3Response<GetBucketLocationOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 GetBucketLocation request for bucket '{}'", bucket);

        // Get bucket metadata
        let metadata = match self.get_bucket_metadata(bucket).await {
            Ok(Some(m)) => m,
            Ok(None) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' not found", bucket));
            }
            Err(e) => {
                return Err(s3_error!(
                    InternalError,
                    "Failed to get bucket metadata: {}",
                    e
                ));
            }
        };

        info!("GetBucketLocation for '{}': {}", bucket, metadata.region);

        // Note: For us-east-1, S3 returns null (None) as the location constraint
        let location_constraint = if metadata.region == "us-east-1" {
            None
        } else {
            Some(BucketLocationConstraint::from(metadata.region))
        };

        Ok(S3Response::new(GetBucketLocationOutput {
            location_constraint,
        }))
    }

    // ===== List Multipart Uploads =====

    async fn list_multipart_uploads(
        &self,
        req: S3Request<ListMultipartUploadsInput>,
    ) -> s3s::S3Result<S3Response<ListMultipartUploadsOutput>> {
        let bucket = req.input.bucket.as_str();
        let prefix = req.input.prefix.as_deref().unwrap_or("");
        let max_uploads = req
            .input
            .max_uploads
            .unwrap_or(MAX_LIST_MULTIPART_UPLOADS as i32)
            .min(MAX_LIST_MULTIPART_UPLOADS as i32) as u32;

        debug!(
            "S3 ListMultipartUploads request for bucket '{}' with prefix '{}'",
            bucket, prefix
        );

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(
                    NoSuchBucket,
                    "Bucket '{}' does not exist",
                    bucket
                ));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Scan for multipart upload metadata
        // Keys have format: vault:s3:{bucket}:_mpu:{upload_id}
        let kv_prefix = format!(
            "vault:{}:{}:{}:",
            S3_VAULT_PREFIX, bucket, MULTIPART_UPLOAD_PREFIX
        );

        let scan_result = self
            .kv_store
            .scan(ScanRequest {
                prefix: kv_prefix.clone(),
                limit: Some(max_uploads + 1),
                continuation_token: req.input.upload_id_marker.clone(),
            })
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to scan multipart uploads: {}", e))?;

        let base_prefix_len = kv_prefix.len();

        // Tiger Style: Pre-allocate with capacity based on scan results.
        let mut uploads: Vec<MultipartUpload> = Vec::with_capacity(scan_result.entries.len());

        for entry in &scan_result.entries {
            if uploads.len() >= max_uploads as usize {
                break;
            }

            // Extract upload ID from key
            if entry.key.len() <= base_prefix_len {
                continue;
            }
            let upload_id = &entry.key[base_prefix_len..];

            // Parse metadata
            if let Ok(meta) = serde_json::from_str::<MultipartUploadMetadata>(&entry.value) {
                // Filter by prefix
                if !meta.key.starts_with(prefix) {
                    continue;
                }

                uploads.push(MultipartUpload {
                    upload_id: Some(upload_id.to_string()),
                    key: Some(meta.key.clone()),
                    initiated: Some(chrono_to_timestamp(meta.initiated_at)),
                    storage_class: Some(StorageClass::from(meta.storage_class.clone())),
                    owner: Some(Owner {
                        display_name: Some("aspen".to_string()),
                        id: Some(self.node_id.to_string()),
                    }),
                    initiator: Some(Initiator {
                        display_name: Some("aspen".to_string()),
                        id: Some(self.node_id.to_string()),
                    }),
                    checksum_algorithm: None,
                    checksum_type: None,
                });
            }
        }

        let is_truncated =
            scan_result.is_truncated || scan_result.entries.len() > max_uploads as usize;
        let next_upload_id_marker = if is_truncated {
            uploads.last().and_then(|u| u.upload_id.clone())
        } else {
            None
        };
        let next_key_marker = if is_truncated {
            uploads.last().and_then(|u| u.key.clone())
        } else {
            None
        };

        info!(
            "ListMultipartUploads for '{}': {} uploads, truncated: {}",
            bucket,
            uploads.len(),
            is_truncated
        );

        Ok(S3Response::new(ListMultipartUploadsOutput {
            bucket: Some(bucket.to_string()),
            prefix: req.input.prefix.clone(),
            key_marker: req.input.key_marker.clone(),
            upload_id_marker: req.input.upload_id_marker.clone(),
            next_key_marker,
            next_upload_id_marker,
            max_uploads: Some(max_uploads as i32),
            is_truncated: Some(is_truncated),
            uploads: if uploads.is_empty() {
                None
            } else {
                Some(uploads)
            },
            common_prefixes: None,
            delimiter: req.input.delimiter.clone(),
            encoding_type: None,
            request_charged: None,
        }))
    }
}
