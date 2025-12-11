/// S3 service implementation for Aspen.
///
/// Maps S3 operations to Aspen's distributed key-value store.
use crate::api::{
    DeleteRequest, KeyValueStore, KeyValueStoreError, ReadRequest, WriteCommand, WriteRequest,
};
use crate::s3::constants::*;
use crate::s3::error::{S3Error, S3Result};
use crate::s3::metadata::{BucketMetadata, ObjectMetadata};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::stream;
use futures::TryStreamExt;
use s3s::dto::*;
use s3s::s3_error;
use s3s::{S3Request, S3Response, S3};
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
    s.parse::<mime::Mime>().ok().map(ContentType::from)
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

impl AspenS3Service {
    /// Create a new S3 service instance.
    pub fn new(kv_store: Arc<dyn KeyValueStore>, node_id: u64) -> Self {
        info!("Initializing Aspen S3 service on node {}", node_id);
        Self { kv_store, node_id }
    }

    /// Generate a vault name for an S3 bucket.
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
                let meta: BucketMetadata =
                    serde_json::from_str(&result.value).map_err(|e| KeyValueStoreError::Failed {
                        reason: format!("Failed to deserialize bucket metadata: {}", e),
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
        let meta_value = serde_json::to_string(metadata).map_err(|e| KeyValueStoreError::Failed {
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
        let chunk_count = ((total_size + chunk_size - 1) / chunk_size) as u32;

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
        let data = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &result.value)
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

    /// Delete all chunks and metadata for an object.
    async fn delete_object_data(&self, bucket: &str, key: &str) -> Result<bool, S3Error> {
        // First check if object exists and get its metadata
        let metadata = match self.get_object_metadata(bucket, key).await {
            Ok(Some(m)) => m,
            Ok(None) => return Ok(false), // Object doesn't exist
            Err(e) => {
                return Err(S3Error::StorageError {
                    source: anyhow::anyhow!("{}", e),
                })
            }
        };

        // Delete chunks if object is chunked
        if metadata.is_chunked() {
            for i in 0..metadata.chunk_count {
                let chunk_key = Self::object_chunk_key(bucket, key, i);
                let _ = self
                    .kv_store
                    .delete(DeleteRequest { key: chunk_key })
                    .await;
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

        // TODO: Implement proper bucket listing by scanning keys with vault prefix
        // For now, return empty list since we don't have scan implemented yet
        Ok(S3Response::new(ListBucketsOutput {
            buckets: Some(vec![]),
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
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' does not exist", bucket));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // TODO: Check if bucket is empty (requires scan implementation)
        // For now, just delete the bucket metadata

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
            Ok(false) => Err(s3_error!(NoSuchBucket, "Bucket '{}' does not exist", bucket)),
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
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' does not exist", bucket));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // Read body data
        let body = match req.input.body {
            Some(stream) => {
                let bytes: Vec<u8> = stream
                    .try_fold(Vec::new(), |mut acc, chunk| async move {
                        acc.extend_from_slice(&chunk);
                        Ok(acc)
                    })
                    .await
                    .map_err(|e| {
                        s3_error!(InternalError, "Failed to read request body: {:?}", e)
                    })?;
                bytes
            }
            None => Vec::new(),
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
        info!("S3 GetObject request for {}/{}", bucket, key);

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' does not exist", bucket));
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

        // Retrieve object data
        let data = if metadata.is_chunked() {
            self.get_chunked_object(bucket, key, &metadata)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to get object: {}", e))?
        } else {
            self.get_simple_object(bucket, key)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to get object: {}", e))?
        };

        // Convert to streaming body
        let body = bytes_to_streaming_blob(data);

        Ok(S3Response::new(GetObjectOutput {
            body: Some(body),
            content_length: Some(metadata.size_bytes as i64),
            content_type: parse_content_type(&metadata.content_type),
            e_tag: Some(metadata.etag.into()),
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
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' does not exist", bucket));
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
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' does not exist", bucket));
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
            e_tag: Some(metadata.etag.into()),
            last_modified: Some(chrono_to_timestamp(metadata.last_modified)),
            ..Default::default()
        }))
    }

    async fn list_objects_v2(
        &self,
        req: S3Request<ListObjectsV2Input>,
    ) -> s3s::S3Result<S3Response<ListObjectsV2Output>> {
        let bucket = req.input.bucket.as_str();
        let prefix = req.input.prefix.as_deref().unwrap_or("");
        debug!(
            "S3 ListObjectsV2 request for bucket '{}' with prefix '{}'",
            bucket, prefix
        );

        // Check if bucket exists
        match self.bucket_exists(bucket).await {
            Ok(false) => {
                return Err(s3_error!(NoSuchBucket, "Bucket '{}' does not exist", bucket));
            }
            Ok(true) => {}
            Err(e) => {
                return Err(s3_error!(InternalError, "Failed to check bucket: {}", e));
            }
        }

        // TODO: Implement proper listing by scanning keys with prefix
        // For now, return empty list since we don't have scan implemented yet
        Ok(S3Response::new(ListObjectsV2Output {
            name: Some(bucket.to_string()),
            prefix: req.input.prefix.clone(),
            delimiter: req.input.delimiter.clone(),
            max_keys: Some(req.input.max_keys.unwrap_or(1000)),
            is_truncated: Some(false),
            key_count: Some(0),
            contents: Some(vec![]),
            common_prefixes: Some(vec![]),
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
                e_tag: Some(dest_metadata.etag.into()),
                last_modified: Some(chrono_to_timestamp(dest_metadata.last_modified)),
                ..Default::default()
            }),
            ..Default::default()
        }))
    }
}
