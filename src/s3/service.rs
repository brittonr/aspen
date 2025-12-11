/// S3 service implementation for Aspen.
///
/// Maps S3 operations to Aspen's distributed key-value store.
use crate::api::KeyValueStore;
use crate::s3::constants::*;
use crate::s3::error::{S3Error, S3Result};
use async_trait::async_trait;
use s3s::dto::*;
use s3s::s3_error;
use s3s::{S3, S3Request, S3Response};
use std::sync::Arc;
use tracing::{debug, info};

/// S3 service implementation backed by Aspen's KV store.
pub struct AspenS3Service {
    /// The underlying key-value store (Raft-backed).
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    fn bucket_vault_name(bucket: &str) -> String {
        format!("{}:{}", S3_VAULT_PREFIX, bucket)
    }

    /// Generate a metadata key for a bucket.
    #[allow(dead_code)]
    fn bucket_metadata_key(bucket: &str) -> String {
        format!(
            "vault:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, BUCKET_METADATA_SUFFIX
        )
    }

    /// Generate a metadata key for an object.
    #[allow(dead_code)]
    fn object_metadata_key(bucket: &str, key: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_METADATA_PREFIX, key
        )
    }

    /// Generate a data key for an object (non-chunked).
    #[allow(dead_code)]
    fn object_data_key(bucket: &str, key: &str) -> String {
        format!(
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_DATA_PREFIX, key
        )
    }

    /// Generate a data key for an object chunk.
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

        // TODO: Implement bucket listing
        // For now, return empty list
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

        // TODO: Check if bucket exists and create it
        // For now, return success

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

        // TODO: Check if bucket exists and is empty, then delete
        // For now, return success

        Ok(S3Response::new(DeleteBucketOutput {}))
    }

    async fn head_bucket(
        &self,
        req: S3Request<HeadBucketInput>,
    ) -> s3s::S3Result<S3Response<HeadBucketOutput>> {
        let bucket = req.input.bucket.as_str();
        debug!("S3 HeadBucket request for bucket '{}'", bucket);

        // TODO: Check if bucket exists
        // For now, return success

        Ok(S3Response::new(HeadBucketOutput {
            bucket_region: Some("us-east-1".to_string()),
            ..Default::default()
        }))
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

        // TODO: Read body, check size, store object
        // For now, return dummy ETag

        Ok(S3Response::new(PutObjectOutput {
            e_tag: Some("\"d41d8cd98f00b204e9800998ecf8427e\"".to_string()),
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

        // TODO: Retrieve object from storage
        // For now, return not found

        Err(s3_error!(NoSuchKey, "Object not found"))
    }

    async fn delete_object(
        &self,
        req: S3Request<DeleteObjectInput>,
    ) -> s3s::S3Result<S3Response<DeleteObjectOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        info!("S3 DeleteObject request for {}/{}", bucket, key);

        // TODO: Delete object from storage
        // For now, return success

        Ok(S3Response::new(DeleteObjectOutput::default()))
    }

    async fn head_object(
        &self,
        req: S3Request<HeadObjectInput>,
    ) -> s3s::S3Result<S3Response<HeadObjectOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();
        debug!("S3 HeadObject request for {}/{}", bucket, key);

        // TODO: Retrieve object metadata
        // For now, return not found

        Err(s3_error!(NoSuchKey, "Object not found"))
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

        // TODO: List objects from storage
        // For now, return empty list

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
        let source = match &req.input.copy_source {
            CopySource::Bucket {
                bucket,
                key,
                version_id: _,
            } => {
                format!("{}/{}", bucket, key)
            }
            CopySource::AccessPoint { .. } => {
                return Err(s3_error!(
                    NotImplemented,
                    "Access point copy not yet supported"
                ));
            }
        };

        info!(
            "S3 CopyObject from '{}' to {}/{}",
            source, dest_bucket, dest_key
        );

        // TODO: Parse source, copy object
        // For now, return not implemented

        Err(s3_error!(
            NotImplemented,
            "Copy operation not yet implemented"
        ))
    }
}
