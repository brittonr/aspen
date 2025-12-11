//! Bucket versioning operations.
//!
//! Implements S3 bucket versioning configuration operations
//! for the Aspen S3 service.

use crate::s3::metadata::BucketVersioning;
use crate::s3::service::AspenS3Service;
use s3s::dto::*;
use s3s::s3_error;
use s3s::{S3Request, S3Response};
use tracing::{debug, info};

use super::bucket::BucketOps;

/// S3 versioning trait implementations.
pub(crate) mod s3_impl {
    use super::*;

    /// Implementation of GetBucketVersioning operation.
    pub async fn get_bucket_versioning(
        service: &AspenS3Service,
        req: S3Request<GetBucketVersioningInput>,
    ) -> s3s::S3Result<S3Response<GetBucketVersioningOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 GetBucketVersioning request for bucket '{}'", bucket);

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

    /// Implementation of PutBucketVersioning operation.
    pub async fn put_bucket_versioning(
        service: &AspenS3Service,
        req: S3Request<PutBucketVersioningInput>,
    ) -> s3s::S3Result<S3Response<PutBucketVersioningOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 PutBucketVersioning request for bucket '{}'", bucket);

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
        if let Err(e) = service.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(PutBucketVersioningOutput {}))
    }
}
