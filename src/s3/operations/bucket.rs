//! Bucket lifecycle operations (create, delete, list, head).
//!
//! Implements S3 bucket management operations for the Aspen S3 service.

use crate::api::{DeleteRequest, ReadRequest, ScanRequest, WriteCommand, WriteRequest};
use crate::s3::constants::*;
use crate::s3::error::{S3Error, S3Result};
use crate::s3::metadata::BucketMetadata;
use crate::s3::service::AspenS3Service;
use async_trait::async_trait;
use s3s::dto::*;
use s3s::s3_error;
use s3s::{S3Request, S3Response};
use tracing::{debug, info, warn};

use super::helpers::chrono_to_timestamp;

/// Internal trait for bucket operations.
///
/// This trait defines bucket-related helper methods on `AspenS3Service`.
#[async_trait]
pub(crate) trait BucketOps {
    /// Generate a metadata key for a bucket.
    fn bucket_metadata_key(bucket: &str) -> String;

    /// Validate bucket name according to S3 rules.
    fn validate_bucket_name(name: &str) -> S3Result<()>;

    /// Check if a bucket exists by reading its metadata.
    async fn bucket_exists(&self, bucket: &str) -> Result<bool, crate::api::KeyValueStoreError>;

    /// Get bucket metadata.
    async fn get_bucket_metadata(
        &self,
        bucket: &str,
    ) -> Result<Option<BucketMetadata>, crate::api::KeyValueStoreError>;

    /// Save bucket metadata to the KV store.
    async fn save_bucket_metadata(&self, metadata: &BucketMetadata) -> S3Result<()>;
}

#[async_trait]
impl BucketOps for AspenS3Service {
    fn bucket_metadata_key(bucket: &str) -> String {
        use std::fmt::Write;
        // Pre-allocate: "vault:" + "s3" + ":" + bucket + ":" + "_bucket_meta"
        let capacity =
            6 + S3_VAULT_PREFIX.len() + 1 + bucket.len() + 1 + BUCKET_METADATA_SUFFIX.len();
        let mut result = String::with_capacity(capacity);
        write!(
            &mut result,
            "vault:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, BUCKET_METADATA_SUFFIX
        )
        .expect("String write should not fail");
        result
    }

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

    async fn bucket_exists(&self, bucket: &str) -> Result<bool, crate::api::KeyValueStoreError> {
        let key = Self::bucket_metadata_key(bucket);
        match self.kv_store().read(ReadRequest { key }).await {
            Ok(_) => Ok(true),
            Err(crate::api::KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn get_bucket_metadata(
        &self,
        bucket: &str,
    ) -> Result<Option<BucketMetadata>, crate::api::KeyValueStoreError> {
        let key = Self::bucket_metadata_key(bucket);
        match self.kv_store().read(ReadRequest { key }).await {
            Ok(result) => {
                let meta: BucketMetadata = serde_json::from_str(&result.value).map_err(|e| {
                    crate::api::KeyValueStoreError::Failed {
                        reason: format!("Failed to deserialize bucket metadata: {}", e),
                    }
                })?;
                Ok(Some(meta))
            }
            Err(crate::api::KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn save_bucket_metadata(&self, metadata: &BucketMetadata) -> S3Result<()> {
        let key = Self::bucket_metadata_key(&metadata.name);
        let value = serde_json::to_string(metadata)?;

        self.kv_store()
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await
            .map_err(|e| S3Error::StorageError { source: e.into() })?;

        Ok(())
    }
}

/// S3 bucket trait implementations.
///
/// This module provides the S3 trait method implementations for bucket operations.
pub(crate) mod s3_impl {
    use super::*;

    /// Implementation of ListBuckets operation.
    pub async fn list_buckets(
        service: &AspenS3Service,
        _req: S3Request<ListBucketsInput>,
    ) -> s3s::S3Result<S3Response<ListBucketsOutput>> {
        debug!("S3 ListBuckets request");

        // Scan for all bucket metadata keys
        // Bucket metadata keys have format: vault:s3:{bucket}:_bucket_meta
        let prefix = format!("vault:{}:", S3_VAULT_PREFIX);
        let suffix = format!(":{}", BUCKET_METADATA_SUFFIX);

        let scan_result = service
            .kv_store()
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
                id: Some(service.node_id().to_string()),
            }),
            continuation_token: None,
            prefix: None,
        }))
    }

    /// Implementation of CreateBucket operation.
    pub async fn create_bucket(
        service: &AspenS3Service,
        req: S3Request<CreateBucketInput>,
    ) -> s3s::S3Result<S3Response<CreateBucketOutput>> {
        let bucket = req.input.bucket.as_str();
        info!("S3 CreateBucket request for bucket '{}'", bucket);

        // Validate bucket name
        AspenS3Service::validate_bucket_name(bucket)
            .map_err(|e| s3_error!(InvalidBucketName, "{}", e))?;

        // Check if bucket already exists
        match service.bucket_exists(bucket).await {
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
        let meta_key = AspenS3Service::bucket_metadata_key(bucket);
        let meta_value = serde_json::to_string(&metadata)
            .map_err(|e| s3_error!(InternalError, "Failed to serialize metadata: {}", e))?;

        service
            .kv_store()
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

    /// Implementation of DeleteBucket operation.
    pub async fn delete_bucket(
        service: &AspenS3Service,
        req: S3Request<DeleteBucketInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketOutput>> {
        let bucket = req.input.bucket.as_str();
        info!("S3 DeleteBucket request for bucket '{}'", bucket);

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

        // Check if bucket is empty by scanning for any objects
        let kv_prefix = format!(
            "vault:{}:{}:{}:",
            S3_VAULT_PREFIX, bucket, OBJECT_METADATA_PREFIX
        );

        let scan_result = service
            .kv_store()
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

        let meta_key = AspenS3Service::bucket_metadata_key(bucket);
        service
            .kv_store()
            .delete(DeleteRequest { key: meta_key })
            .await
            .map_err(|e| s3_error!(InternalError, "Failed to delete bucket: {}", e))?;

        info!("Deleted bucket '{}'", bucket);
        Ok(S3Response::new(DeleteBucketOutput {}))
    }

    /// Implementation of HeadBucket operation.
    pub async fn head_bucket(
        service: &AspenS3Service,
        req: S3Request<HeadBucketInput>,
    ) -> s3s::S3Result<S3Response<HeadBucketOutput>> {
        let bucket = req.input.bucket.as_str();
        debug!("S3 HeadBucket request for bucket '{}'", bucket);

        match service.bucket_exists(bucket).await {
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

    /// Implementation of GetBucketLocation operation.
    pub async fn get_bucket_location(
        service: &AspenS3Service,
        req: S3Request<GetBucketLocationInput>,
    ) -> s3s::S3Result<S3Response<GetBucketLocationOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 GetBucketLocation request for bucket '{}'", bucket);

        // Get bucket metadata
        let metadata = match service.get_bucket_metadata(bucket).await {
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
}
