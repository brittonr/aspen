//! S3 service implementation for Aspen.
//!
//! Maps S3 operations to Aspen's distributed key-value store.
//! The actual operation implementations are in the `operations` submodules.

use crate::api::KeyValueStore;
use async_trait::async_trait;
use s3s::dto::*;
use s3s::{S3, S3Request, S3Response};
use std::sync::Arc;
use tracing::info;

// Import operation modules for delegation
use super::operations::{bucket, encryption, lifecycle, multipart, object, tagging, versioning};

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

    /// Get a reference to the underlying KV store.
    #[inline]
    pub(crate) fn kv_store(&self) -> &Arc<dyn KeyValueStore> {
        &self.kv_store
    }

    /// Get the node ID.
    #[inline]
    pub(crate) fn node_id(&self) -> u64 {
        self.node_id
    }
}

/// S3 trait implementation.
///
/// This is the main interface required by the s3s crate.
/// Operations are delegated to the appropriate operation modules.
#[async_trait]
impl S3 for AspenS3Service {
    // ===== Bucket Operations =====

    async fn list_buckets(
        &self,
        req: S3Request<ListBucketsInput>,
    ) -> s3s::S3Result<S3Response<ListBucketsOutput>> {
        bucket::s3_impl::list_buckets(self, req).await
    }

    async fn create_bucket(
        &self,
        req: S3Request<CreateBucketInput>,
    ) -> s3s::S3Result<S3Response<CreateBucketOutput>> {
        bucket::s3_impl::create_bucket(self, req).await
    }

    async fn delete_bucket(
        &self,
        req: S3Request<DeleteBucketInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketOutput>> {
        bucket::s3_impl::delete_bucket(self, req).await
    }

    async fn head_bucket(
        &self,
        req: S3Request<HeadBucketInput>,
    ) -> s3s::S3Result<S3Response<HeadBucketOutput>> {
        bucket::s3_impl::head_bucket(self, req).await
    }

    async fn get_bucket_location(
        &self,
        req: S3Request<GetBucketLocationInput>,
    ) -> s3s::S3Result<S3Response<GetBucketLocationOutput>> {
        bucket::s3_impl::get_bucket_location(self, req).await
    }

    // ===== Object Operations =====

    async fn put_object(
        &self,
        req: S3Request<PutObjectInput>,
    ) -> s3s::S3Result<S3Response<PutObjectOutput>> {
        object::s3_impl::put_object(self, req).await
    }

    async fn get_object(
        &self,
        req: S3Request<GetObjectInput>,
    ) -> s3s::S3Result<S3Response<GetObjectOutput>> {
        object::s3_impl::get_object(self, req).await
    }

    async fn delete_object(
        &self,
        req: S3Request<DeleteObjectInput>,
    ) -> s3s::S3Result<S3Response<DeleteObjectOutput>> {
        object::s3_impl::delete_object(self, req).await
    }

    async fn delete_objects(
        &self,
        req: S3Request<DeleteObjectsInput>,
    ) -> s3s::S3Result<S3Response<DeleteObjectsOutput>> {
        object::s3_impl::delete_objects(self, req).await
    }

    async fn head_object(
        &self,
        req: S3Request<HeadObjectInput>,
    ) -> s3s::S3Result<S3Response<HeadObjectOutput>> {
        object::s3_impl::head_object(self, req).await
    }

    async fn copy_object(
        &self,
        req: S3Request<CopyObjectInput>,
    ) -> s3s::S3Result<S3Response<CopyObjectOutput>> {
        object::s3_impl::copy_object(self, req).await
    }

    async fn list_objects_v2(
        &self,
        req: S3Request<ListObjectsV2Input>,
    ) -> s3s::S3Result<S3Response<ListObjectsV2Output>> {
        object::s3_impl::list_objects_v2(self, req).await
    }

    // ===== Multipart Upload Operations =====

    async fn create_multipart_upload(
        &self,
        req: S3Request<CreateMultipartUploadInput>,
    ) -> s3s::S3Result<S3Response<CreateMultipartUploadOutput>> {
        multipart::s3_impl::create_multipart_upload(self, req).await
    }

    async fn upload_part(
        &self,
        req: S3Request<UploadPartInput>,
    ) -> s3s::S3Result<S3Response<UploadPartOutput>> {
        multipart::s3_impl::upload_part(self, req).await
    }

    async fn complete_multipart_upload(
        &self,
        req: S3Request<CompleteMultipartUploadInput>,
    ) -> s3s::S3Result<S3Response<CompleteMultipartUploadOutput>> {
        multipart::s3_impl::complete_multipart_upload(self, req).await
    }

    async fn abort_multipart_upload(
        &self,
        req: S3Request<AbortMultipartUploadInput>,
    ) -> s3s::S3Result<S3Response<AbortMultipartUploadOutput>> {
        multipart::s3_impl::abort_multipart_upload(self, req).await
    }

    async fn list_parts(
        &self,
        req: S3Request<ListPartsInput>,
    ) -> s3s::S3Result<S3Response<ListPartsOutput>> {
        multipart::s3_impl::list_parts(self, req).await
    }

    async fn list_multipart_uploads(
        &self,
        req: S3Request<ListMultipartUploadsInput>,
    ) -> s3s::S3Result<S3Response<ListMultipartUploadsOutput>> {
        multipart::s3_impl::list_multipart_uploads(self, req).await
    }

    // ===== Bucket Versioning Operations =====

    async fn get_bucket_versioning(
        &self,
        req: S3Request<GetBucketVersioningInput>,
    ) -> s3s::S3Result<S3Response<GetBucketVersioningOutput>> {
        versioning::s3_impl::get_bucket_versioning(self, req).await
    }

    async fn put_bucket_versioning(
        &self,
        req: S3Request<PutBucketVersioningInput>,
    ) -> s3s::S3Result<S3Response<PutBucketVersioningOutput>> {
        versioning::s3_impl::put_bucket_versioning(self, req).await
    }

    // ===== Bucket Encryption Operations =====

    async fn get_bucket_encryption(
        &self,
        req: S3Request<GetBucketEncryptionInput>,
    ) -> s3s::S3Result<S3Response<GetBucketEncryptionOutput>> {
        encryption::s3_impl::get_bucket_encryption(self, req).await
    }

    async fn put_bucket_encryption(
        &self,
        req: S3Request<PutBucketEncryptionInput>,
    ) -> s3s::S3Result<S3Response<PutBucketEncryptionOutput>> {
        encryption::s3_impl::put_bucket_encryption(self, req).await
    }

    async fn delete_bucket_encryption(
        &self,
        req: S3Request<DeleteBucketEncryptionInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketEncryptionOutput>> {
        encryption::s3_impl::delete_bucket_encryption(self, req).await
    }

    // ===== Bucket Lifecycle Operations =====

    async fn get_bucket_lifecycle_configuration(
        &self,
        req: S3Request<GetBucketLifecycleConfigurationInput>,
    ) -> s3s::S3Result<S3Response<GetBucketLifecycleConfigurationOutput>> {
        lifecycle::s3_impl::get_bucket_lifecycle_configuration(self, req).await
    }

    async fn put_bucket_lifecycle_configuration(
        &self,
        req: S3Request<PutBucketLifecycleConfigurationInput>,
    ) -> s3s::S3Result<S3Response<PutBucketLifecycleConfigurationOutput>> {
        lifecycle::s3_impl::put_bucket_lifecycle_configuration(self, req).await
    }

    async fn delete_bucket_lifecycle(
        &self,
        req: S3Request<DeleteBucketLifecycleInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketLifecycleOutput>> {
        lifecycle::s3_impl::delete_bucket_lifecycle(self, req).await
    }

    // ===== Object Tagging Operations =====

    async fn get_object_tagging(
        &self,
        req: S3Request<GetObjectTaggingInput>,
    ) -> s3s::S3Result<S3Response<GetObjectTaggingOutput>> {
        tagging::s3_impl::get_object_tagging(self, req).await
    }

    async fn put_object_tagging(
        &self,
        req: S3Request<PutObjectTaggingInput>,
    ) -> s3s::S3Result<S3Response<PutObjectTaggingOutput>> {
        tagging::s3_impl::put_object_tagging(self, req).await
    }

    async fn delete_object_tagging(
        &self,
        req: S3Request<DeleteObjectTaggingInput>,
    ) -> s3s::S3Result<S3Response<DeleteObjectTaggingOutput>> {
        tagging::s3_impl::delete_object_tagging(self, req).await
    }

    // ===== Bucket Tagging Operations =====

    async fn get_bucket_tagging(
        &self,
        req: S3Request<GetBucketTaggingInput>,
    ) -> s3s::S3Result<S3Response<GetBucketTaggingOutput>> {
        tagging::s3_impl::get_bucket_tagging(self, req).await
    }

    async fn put_bucket_tagging(
        &self,
        req: S3Request<PutBucketTaggingInput>,
    ) -> s3s::S3Result<S3Response<PutBucketTaggingOutput>> {
        tagging::s3_impl::put_bucket_tagging(self, req).await
    }

    async fn delete_bucket_tagging(
        &self,
        req: S3Request<DeleteBucketTaggingInput>,
    ) -> s3s::S3Result<S3Response<DeleteBucketTaggingOutput>> {
        tagging::s3_impl::delete_bucket_tagging(self, req).await
    }
}
