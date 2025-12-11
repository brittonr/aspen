//! Object operations (put, get, delete, head, copy, list).
//!
//! Implements S3 object management operations for the Aspen S3 service,
//! including support for chunked storage of large objects.

use crate::api::{
    DeleteRequest, KeyValueStoreError, ReadRequest, ScanRequest, WriteCommand, WriteRequest,
};
use crate::s3::constants::*;
use crate::s3::error::{S3Error, S3Result};
use crate::s3::metadata::ObjectMetadata;
use crate::s3::service::AspenS3Service;
use async_trait::async_trait;
use futures::TryStreamExt;
use s3s::dto::*;
use s3s::s3_error;
use s3s::{S3Request, S3Response};
use tracing::{debug, info, warn};

use super::bucket::BucketOps;
use super::helpers::{
    bytes_to_streaming_blob, chrono_to_timestamp, format_content_range, parse_content_type,
    resolve_range,
};

/// Internal trait for object operations.
///
/// This trait defines object-related helper methods on `AspenS3Service`.
#[async_trait]
pub(crate) trait ObjectOps {
    /// Generate a metadata key for an object.
    fn object_metadata_key(bucket: &str, key: &str) -> String;

    /// Generate a data key for an object (non-chunked).
    fn object_data_key(bucket: &str, key: &str) -> String;

    /// Generate a data key for an object chunk.
    fn object_chunk_key(bucket: &str, key: &str, chunk_index: u32) -> String;

    /// Validate object key according to S3 rules.
    fn validate_object_key(key: &str) -> S3Result<()>;

    /// Infer content type from file extension.
    fn infer_content_type(key: &str) -> String;

    /// Calculate MD5 hash and return as ETag.
    fn calculate_etag(data: &[u8]) -> String;

    /// Store object metadata.
    async fn store_object_metadata(
        &self,
        bucket: &str,
        key: &str,
        metadata: &ObjectMetadata,
    ) -> Result<(), KeyValueStoreError>;

    /// Get object metadata.
    async fn get_object_metadata(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Option<ObjectMetadata>, KeyValueStoreError>;

    /// Store a small object (non-chunked) directly.
    async fn store_simple_object(
        &self,
        bucket: &str,
        key: &str,
        data: &[u8],
        content_type: Option<String>,
    ) -> Result<ObjectMetadata, KeyValueStoreError>;

    /// Store a large object using chunking.
    async fn store_chunked_object(
        &self,
        bucket: &str,
        key: &str,
        data: &[u8],
        content_type: Option<String>,
    ) -> Result<ObjectMetadata, KeyValueStoreError>;

    /// Retrieve a simple (non-chunked) object.
    async fn get_simple_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, S3Error>;

    /// Retrieve a chunked object.
    async fn get_chunked_object(
        &self,
        bucket: &str,
        key: &str,
        metadata: &ObjectMetadata,
    ) -> Result<Vec<u8>, S3Error>;

    /// Delete all chunks and metadata for an object.
    async fn delete_object_data(&self, bucket: &str, key: &str) -> Result<bool, S3Error>;
}

#[async_trait]
impl ObjectOps for AspenS3Service {
    fn object_metadata_key(bucket: &str, key: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_METADATA_PREFIX, key
        )
    }

    fn object_data_key(bucket: &str, key: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_DATA_PREFIX, key
        )
    }

    fn object_chunk_key(bucket: &str, key: &str, chunk_index: u32) -> String {
        format!(
            "vault:{}:{}:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_DATA_PREFIX, key, CHUNK_KEY_COMPONENT, chunk_index
        )
    }

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

    fn calculate_etag(data: &[u8]) -> String {
        let digest = md5::compute(data);
        format!("\"{}\"", hex::encode(digest.as_ref()))
    }

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

        self.kv_store()
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: meta_key,
                    value: meta_value,
                },
            })
            .await?;

        Ok(())
    }

    async fn get_object_metadata(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Option<ObjectMetadata>, KeyValueStoreError> {
        let meta_key = Self::object_metadata_key(bucket, key);
        match self.kv_store().read(ReadRequest { key: meta_key }).await {
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
        self.kv_store()
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

            self.kv_store()
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

    async fn get_simple_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, S3Error> {
        let data_key = Self::object_data_key(bucket, key);

        let result = self
            .kv_store()
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
                .kv_store()
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
                let _ = self
                    .kv_store()
                    .delete(DeleteRequest { key: chunk_key })
                    .await;
            }
        } else {
            // Delete simple object data
            let data_key = Self::object_data_key(bucket, key);
            let _ = self
                .kv_store()
                .delete(DeleteRequest { key: data_key })
                .await;
        }

        // Delete metadata
        let meta_key = Self::object_metadata_key(bucket, key);
        let _ = self
            .kv_store()
            .delete(DeleteRequest { key: meta_key })
            .await;

        Ok(true)
    }
}

/// S3 object trait implementations.
pub(crate) mod s3_impl {
    use super::*;

    /// Implementation of PutObject operation.
    pub async fn put_object(
        service: &AspenS3Service,
        req: S3Request<PutObjectInput>,
    ) -> s3s::S3Result<S3Response<PutObjectOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        info!("S3 PutObject request for {}/{}", bucket, key);

        // Validate inputs
        AspenS3Service::validate_object_key(key)
            .map_err(|e| s3_error!(InvalidArgument, "Invalid object key: {}", e))?;

        // Check if bucket exists
        match service.bucket_exists(bucket).await {
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
            service
                .store_chunked_object(bucket, key, &body, content_type)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to store object: {}", e))?
        } else {
            debug!("Storing simple object {} bytes", body.len());
            service
                .store_simple_object(bucket, key, &body, content_type)
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

    /// Implementation of GetObject operation.
    pub async fn get_object(
        service: &AspenS3Service,
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
        match service.bucket_exists(bucket).await {
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
        let metadata = match service.get_object_metadata(bucket, key).await {
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
            service
                .get_chunked_object(bucket, key, &metadata)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to get object: {}", e))?
        } else {
            service
                .get_simple_object(bucket, key)
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

    /// Implementation of DeleteObject operation.
    pub async fn delete_object(
        service: &AspenS3Service,
        req: S3Request<DeleteObjectInput>,
    ) -> s3s::S3Result<S3Response<DeleteObjectOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        info!("S3 DeleteObject request for {}/{}", bucket, key);

        // Check if bucket exists
        match service.bucket_exists(bucket).await {
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
        let _ = service.delete_object_data(bucket, key).await;

        Ok(S3Response::new(DeleteObjectOutput::default()))
    }

    /// Implementation of DeleteObjects (batch delete) operation.
    pub async fn delete_objects(
        service: &AspenS3Service,
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
        match service.bucket_exists(bucket).await {
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

            match service.delete_object_data(bucket, key).await {
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

    /// Implementation of HeadObject operation.
    pub async fn head_object(
        service: &AspenS3Service,
        req: S3Request<HeadObjectInput>,
    ) -> s3s::S3Result<S3Response<HeadObjectOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        debug!("S3 HeadObject request for {}/{}", bucket, key);

        // Check if bucket exists
        match service.bucket_exists(bucket).await {
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
        let metadata = match service.get_object_metadata(bucket, key).await {
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

    /// Implementation of CopyObject operation.
    pub async fn copy_object(
        service: &AspenS3Service,
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
        match service.bucket_exists(src_bucket).await {
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
        match service.bucket_exists(dest_bucket).await {
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
        let src_metadata = match service.get_object_metadata(src_bucket, src_key).await {
            Ok(Some(m)) => m,
            Ok(None) => return Err(s3_error!(NoSuchKey, "Source object not found")),
            Err(e) => return Err(s3_error!(InternalError, "Failed to get metadata: {}", e)),
        };

        // Get source object data
        let data = if src_metadata.is_chunked() {
            service
                .get_chunked_object(src_bucket, src_key, &src_metadata)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to read source object: {}", e))?
        } else {
            service
                .get_simple_object(src_bucket, src_key)
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to read source object: {}", e))?
        };

        // Store to destination (reuse content type from source)
        let dest_metadata = if data.len() > S3_CHUNK_SIZE_BYTES as usize {
            service
                .store_chunked_object(
                    dest_bucket,
                    dest_key,
                    &data,
                    Some(src_metadata.content_type.clone()),
                )
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to store object: {}", e))?
        } else {
            service
                .store_simple_object(
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

    /// Implementation of ListObjectsV2 operation.
    pub async fn list_objects_v2(
        service: &AspenS3Service,
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
        match service.bucket_exists(bucket).await {
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
        let scan_result = service
            .kv_store()
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
}
