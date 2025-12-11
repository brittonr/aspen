//! Bucket lifecycle configuration operations.
//!
//! Implements S3 lifecycle configuration operations for the Aspen S3 service.

use crate::s3::metadata::{
    BucketLifecycleConfiguration, LifecycleExpiration, LifecycleRule, LifecycleRuleStatus,
    LifecycleTransition, NoncurrentVersionExpiration,
};
use crate::s3::service::AspenS3Service;
use chrono::DateTime;
use s3s::dto::*;
use s3s::s3_error;
use s3s::{S3Request, S3Response};
use tracing::{debug, info};

use super::bucket::BucketOps;
use super::helpers::chrono_to_timestamp;

/// Internal trait for lifecycle operations.
pub(crate) trait LifecycleOps {
    /// Convert internal LifecycleRule to s3s LifecycleRule.
    fn convert_lifecycle_rule_to_s3s(&self, rule: &LifecycleRule) -> s3s::dto::LifecycleRule;

    /// Convert s3s LifecycleRule to internal LifecycleRule.
    fn convert_s3s_lifecycle_rule(
        &self,
        rule: &s3s::dto::LifecycleRule,
    ) -> Result<LifecycleRule, String>;
}

impl LifecycleOps for AspenS3Service {
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

/// S3 lifecycle trait implementations.
pub(crate) mod s3_impl {
    use super::*;

    /// Implementation of GetBucketLifecycleConfiguration operation.
    pub async fn get_bucket_lifecycle_configuration(
        service: &AspenS3Service,
        req: S3Request<GetBucketLifecycleConfigurationInput>,
    ) -> s3s::S3Result<S3Response<GetBucketLifecycleConfigurationOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!(
            "S3 GetBucketLifecycleConfiguration request for bucket '{}'",
            bucket
        );

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
            .map(|r| service.convert_lifecycle_rule_to_s3s(r))
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

    /// Implementation of PutBucketLifecycleConfiguration operation.
    pub async fn put_bucket_lifecycle_configuration(
        service: &AspenS3Service,
        req: S3Request<PutBucketLifecycleConfigurationInput>,
    ) -> s3s::S3Result<S3Response<PutBucketLifecycleConfigurationOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!(
            "S3 PutBucketLifecycleConfiguration request for bucket '{}'",
            bucket
        );

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
                    .map(|r| service.convert_s3s_lifecycle_rule(r))
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
        if let Err(e) = service.save_bucket_metadata(&metadata).await {
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

    /// Implementation of DeleteBucketLifecycle operation.
    pub async fn delete_bucket_lifecycle(
        service: &AspenS3Service,
        req: S3Request<DeleteBucketLifecycleInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketLifecycleOutput>> {
        let bucket = req.input.bucket.as_str();

        debug!("S3 DeleteBucketLifecycle request for bucket '{}'", bucket);

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

        info!("DeleteBucketLifecycle for '{}'", bucket);

        metadata.lifecycle = None;

        // Save updated metadata
        if let Err(e) = service.save_bucket_metadata(&metadata).await {
            return Err(s3_error!(
                InternalError,
                "Failed to save bucket metadata: {}",
                e
            ));
        }

        Ok(S3Response::new(DeleteBucketLifecycleOutput {}))
    }
}
