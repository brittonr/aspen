//! Object and bucket tagging operations.
//!
//! Implements S3 tagging operations for both objects and buckets
//! in the Aspen S3 service.

use crate::api::{DeleteRequest, KeyValueStoreError, ReadRequest, WriteCommand, WriteRequest};
use crate::s3::constants::*;
use crate::s3::service::AspenS3Service;
use s3s::dto::*;
use s3s::s3_error;
use s3s::{S3Request, S3Response};
use tracing::{debug, info};

use super::bucket::BucketOps;
use super::object::ObjectOps;

/// Internal trait for tagging operations.
pub(crate) trait TaggingOps {
    /// Generate a tags key for an object.
    fn object_tags_key(bucket: &str, key: &str) -> String;
}

impl TaggingOps for AspenS3Service {
    fn object_tags_key(bucket: &str, key: &str) -> String {
        use std::fmt::Write;
        // Pre-allocate: "vault:" + "s3" + ":" + bucket + ":" + "_tags" + ":" + key
        let capacity = 6
            + S3_VAULT_PREFIX.len()
            + 1
            + bucket.len()
            + 1
            + OBJECT_TAGS_PREFIX.len()
            + 1
            + key.len();
        let mut result = String::with_capacity(capacity);
        write!(
            &mut result,
            "vault:{}:{}:{}:{}",
            S3_VAULT_PREFIX, bucket, OBJECT_TAGS_PREFIX, key
        )
        .expect("String write should not fail");
        result
    }
}

/// S3 tagging trait implementations.
pub(crate) mod s3_impl {
    use super::*;

    /// Implementation of GetObjectTagging operation.
    pub async fn get_object_tagging(
        service: &AspenS3Service,
        req: S3Request<GetObjectTaggingInput>,
    ) -> s3s::S3Result<S3Response<GetObjectTaggingOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();

        debug!("S3 GetObjectTagging request for {}/{}", bucket, key);

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

        // Check if object exists
        match service.get_object_metadata(bucket, key).await {
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
        let tags_key = AspenS3Service::object_tags_key(bucket, key);
        let tag_set = match service.kv_store().read(ReadRequest { key: tags_key }).await {
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

    /// Implementation of PutObjectTagging operation.
    pub async fn put_object_tagging(
        service: &AspenS3Service,
        req: S3Request<PutObjectTaggingInput>,
    ) -> s3s::S3Result<S3Response<PutObjectTaggingOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();

        debug!("S3 PutObjectTagging request for {}/{}", bucket, key);

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

        // Check if object exists
        match service.get_object_metadata(bucket, key).await {
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
        let tags_key = AspenS3Service::object_tags_key(bucket, key);
        let tags_value = serde_json::to_string(&tags_map)
            .map_err(|e| s3_error!(InternalError, "Failed to serialize tags: {}", e))?;

        service
            .kv_store()
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

    /// Implementation of DeleteObjectTagging operation.
    pub async fn delete_object_tagging(
        service: &AspenS3Service,
        req: S3Request<DeleteObjectTaggingInput>,
    ) -> s3s::S3Result<S3Response<DeleteObjectTaggingOutput>> {
        let bucket = req.input.bucket.as_str();
        let key = req.input.key.as_str();

        debug!("S3 DeleteObjectTagging request for {}/{}", bucket, key);

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

        // Check if object exists
        match service.get_object_metadata(bucket, key).await {
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
        let tags_key = AspenS3Service::object_tags_key(bucket, key);
        let _ = service
            .kv_store()
            .delete(DeleteRequest { key: tags_key })
            .await;

        info!("DeleteObjectTagging for {}/{}", bucket, key);

        Ok(S3Response::new(DeleteObjectTaggingOutput {
            version_id: None,
        }))
    }

    /// Implementation of GetBucketTagging operation.
    pub async fn get_bucket_tagging(
        service: &AspenS3Service,
        req: S3Request<GetBucketTaggingInput>,
    ) -> s3s::S3Result<S3Response<GetBucketTaggingOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 GetBucketTagging request for bucket '{}'", bucket);

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

    /// Implementation of PutBucketTagging operation.
    pub async fn put_bucket_tagging(
        service: &AspenS3Service,
        req: S3Request<PutBucketTaggingInput>,
    ) -> s3s::S3Result<S3Response<PutBucketTaggingOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 PutBucketTagging request for bucket '{}'", bucket);

        // Get current bucket metadata
        let mut metadata = match service.get_bucket_metadata(bucket).await {
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
        if let Err(e) = service.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(PutBucketTaggingOutput {}))
    }

    /// Implementation of DeleteBucketTagging operation.
    pub async fn delete_bucket_tagging(
        service: &AspenS3Service,
        req: S3Request<DeleteBucketTaggingInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketTaggingOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 DeleteBucketTagging request for bucket '{}'", bucket);

        // Get current bucket metadata
        let mut metadata = match service.get_bucket_metadata(bucket).await {
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
        if let Err(e) = service.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(DeleteBucketTaggingOutput {}))
    }
}
