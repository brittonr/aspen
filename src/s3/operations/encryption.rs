//! Bucket encryption configuration operations.
//!
//! Implements S3 bucket encryption configuration operations
//! for the Aspen S3 service.

use crate::s3::metadata::{
    BucketEncryptionConfiguration, ServerSideEncryption as AspenServerSideEncryption,
};
use crate::s3::service::AspenS3Service;
use s3s::dto::*;
use s3s::s3_error;
use s3s::{S3Request, S3Response};
use tracing::{debug, info};

use super::bucket::BucketOps;

/// S3 encryption trait implementations.
pub(crate) mod s3_impl {
    use super::*;

    /// Implementation of GetBucketEncryption operation.
    pub async fn get_bucket_encryption(
        service: &AspenS3Service,
        req: S3Request<GetBucketEncryptionInput>,
    ) -> s3s::S3Result<S3Response<GetBucketEncryptionOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 GetBucketEncryption request for bucket '{}'", bucket);

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

    /// Implementation of PutBucketEncryption operation.
    pub async fn put_bucket_encryption(
        service: &AspenS3Service,
        req: S3Request<PutBucketEncryptionInput>,
    ) -> s3s::S3Result<S3Response<PutBucketEncryptionOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 PutBucketEncryption request for bucket '{}'", bucket);

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
        if let Err(e) = service.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(PutBucketEncryptionOutput {}))
    }

    /// Implementation of DeleteBucketEncryption operation.
    pub async fn delete_bucket_encryption(
        service: &AspenS3Service,
        req: S3Request<DeleteBucketEncryptionInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketEncryptionOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 DeleteBucketEncryption request for bucket '{}'", bucket);

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

        info!("DeleteBucketEncryption for '{}'", bucket);

        metadata.encryption = None;

        // Save updated metadata
        if let Err(e) = service.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(DeleteBucketEncryptionOutput {}))
    }
}
