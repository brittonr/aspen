//! Multipart upload operations.
//!
//! Implements S3 multipart upload operations for the Aspen S3 service,
//! allowing large objects to be uploaded in parts.

use crate::api::{
    DeleteRequest, KeyValueStoreError, ReadRequest, ScanRequest, WriteCommand, WriteRequest,
};
use crate::s3::constants::*;
use crate::s3::metadata::{MultipartUploadMetadata, PartMetadata};
use crate::s3::service::AspenS3Service;
use async_trait::async_trait;
use futures::TryStreamExt;
use s3s::dto::*;
use s3s::s3_error;
use s3s::{S3Request, S3Response};
use tracing::{debug, info};

use super::bucket::BucketOps;
use super::helpers::chrono_to_timestamp;
use super::object::ObjectOps;

/// Internal trait for multipart upload operations.
#[async_trait]
pub(crate) trait MultipartOps {
    /// Generate a multipart upload metadata key.
    fn multipart_upload_metadata_key(bucket: &str, upload_id: &str) -> String;

    /// Generate a multipart upload part data key.
    fn multipart_part_data_key(bucket: &str, upload_id: &str, part_number: u32) -> String;

    /// Store multipart upload metadata.
    async fn store_multipart_metadata(
        &self,
        metadata: &MultipartUploadMetadata,
    ) -> Result<(), KeyValueStoreError>;

    /// Get multipart upload metadata.
    async fn get_multipart_metadata(
        &self,
        bucket: &str,
        upload_id: &str,
    ) -> Result<Option<MultipartUploadMetadata>, KeyValueStoreError>;

    /// Delete multipart upload metadata and all parts.
    async fn delete_multipart_upload(
        &self,
        bucket: &str,
        upload_id: &str,
    ) -> Result<(), KeyValueStoreError>;

    /// Calculate composite ETag for multipart upload.
    fn calculate_multipart_etag(parts: &[&PartMetadata]) -> String;
}

#[async_trait]
impl MultipartOps for AspenS3Service {
    fn multipart_upload_metadata_key(bucket: &str, upload_id: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, MULTIPART_UPLOAD_PREFIX, upload_id
        )
    }

    fn multipart_part_data_key(bucket: &str, upload_id: &str, part_number: u32) -> String {
        format!(
            "vault:{}:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, MULTIPART_PART_PREFIX, upload_id, part_number
        )
    }

    async fn store_multipart_metadata(
        &self,
        metadata: &MultipartUploadMetadata,
    ) -> Result<(), KeyValueStoreError> {
        let meta_key = Self::multipart_upload_metadata_key(&metadata.bucket, &metadata.upload_id);
        let meta_value =
            serde_json::to_string(metadata).map_err(|e| KeyValueStoreError::Failed {
                reason: format!("Failed to serialize multipart metadata: {}", e),
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

    async fn get_multipart_metadata(
        &self,
        bucket: &str,
        upload_id: &str,
    ) -> Result<Option<MultipartUploadMetadata>, KeyValueStoreError> {
        let meta_key = Self::multipart_upload_metadata_key(bucket, upload_id);
        match self.kv_store().read(ReadRequest { key: meta_key }).await {
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
                let _ = self
                    .kv_store()
                    .delete(DeleteRequest { key: part_key })
                    .await;
            }
        }

        // Delete metadata
        let meta_key = Self::multipart_upload_metadata_key(bucket, upload_id);
        let _ = self
            .kv_store()
            .delete(DeleteRequest { key: meta_key })
            .await;

        Ok(())
    }

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
}

/// S3 multipart upload trait implementations.
pub(crate) mod s3_impl {
    use super::*;

    /// Implementation of CreateMultipartUpload operation.
    pub async fn create_multipart_upload(
        service: &AspenS3Service,
        req: S3Request<CreateMultipartUploadInput>,
    ) -> s3s::S3Result<S3Response<CreateMultipartUploadOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        info!("S3 CreateMultipartUpload request for {}/{}", bucket, key);

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

        service
            .store_multipart_metadata(&metadata)
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

    /// Implementation of UploadPart operation.
    pub async fn upload_part(
        service: &AspenS3Service,
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
        let mut metadata = match service.get_multipart_metadata(bucket, upload_id).await {
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
        let etag = AspenS3Service::calculate_etag(&body);

        // Store part data
        let part_key =
            AspenS3Service::multipart_part_data_key(bucket, upload_id, part_number as u32);
        let encoded_data =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &body);

        service
            .kv_store()
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
        service
            .store_multipart_metadata(&metadata)
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

    /// Implementation of CompleteMultipartUpload operation.
    pub async fn complete_multipart_upload(
        service: &AspenS3Service,
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
        let metadata = match service.get_multipart_metadata(bucket, upload_id).await {
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
            let part_key =
                AspenS3Service::multipart_part_data_key(bucket, upload_id, part.part_number);
            let result = service
                .kv_store()
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
        let etag = AspenS3Service::calculate_multipart_etag(&collected_parts);

        // Determine content type
        let content_type = metadata
            .content_type
            .unwrap_or_else(|| AspenS3Service::infer_content_type(key));

        // Store the final object using chunking if needed
        if object_data.len() > S3_CHUNK_SIZE_BYTES as usize {
            service
                .store_chunked_object(bucket, key, &object_data, Some(content_type))
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to store object: {}", e))?;
        } else {
            service
                .store_simple_object(bucket, key, &object_data, Some(content_type))
                .await
                .map_err(|e| s3_error!(InternalError, "Failed to store object: {}", e))?;
        }

        // Update object metadata with multipart ETag (overwrite the one from store_*_object)
        let mut obj_metadata = service
            .get_object_metadata(bucket, key)
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to get object metadata: {}", e))?
            .ok_or_else(|| s3_error!(InternalError, "Object metadata not found after store"))?;

        obj_metadata.etag = etag.clone();
        service
            .store_object_metadata(bucket, key, &obj_metadata)
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to update object metadata: {}", e))?;

        // Delete multipart upload data
        service
            .delete_multipart_upload(bucket, upload_id)
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

    /// Implementation of AbortMultipartUpload operation.
    pub async fn abort_multipart_upload(
        service: &AspenS3Service,
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
        let metadata = match service.get_multipart_metadata(bucket, upload_id).await {
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
        service
            .delete_multipart_upload(bucket, upload_id)
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to abort multipart upload: {}", e))?;

        info!("Aborted multipart upload for {}/{}", bucket, key);

        Ok(S3Response::new(AbortMultipartUploadOutput::default()))
    }

    /// Implementation of ListParts operation.
    pub async fn list_parts(
        service: &AspenS3Service,
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
        let metadata = match service.get_multipart_metadata(bucket, upload_id).await {
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

    /// Implementation of ListMultipartUploads operation.
    pub async fn list_multipart_uploads(
        service: &AspenS3Service,
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

        // Scan for multipart upload metadata
        // Keys have format: vault:s3:{bucket}:_mpu:{upload_id}
        let kv_prefix = format!(
            "vault:{}:{}:{}:",
            S3_VAULT_PREFIX, bucket, MULTIPART_UPLOAD_PREFIX
        );

        let scan_result = service
            .kv_store()
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
                        id: Some(service.node_id().to_string()),
                    }),
                    initiator: Some(Initiator {
                        display_name: Some("aspen".to_string()),
                        id: Some(service.node_id().to_string()),
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
