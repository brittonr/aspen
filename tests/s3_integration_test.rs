//! S3 integration tests for Aspen.
//!
//! Tests the S3 service implementation including:
//! - Bucket operations (create, head, delete)
//! - Object operations (put, get, head, delete, copy)
//! - Large object chunking
//! - Error handling

use aspen::api::DeterministicKeyValueStore;
use aspen::s3::AspenS3Service;
use s3s::dto::*;
use s3s::{S3, S3Request};
use std::sync::Arc;

/// Create a test S3 service with in-memory storage.
fn create_test_service() -> AspenS3Service {
    let kv_store = Arc::new(DeterministicKeyValueStore::new());
    AspenS3Service::new(kv_store, 1)
}

// ===== Bucket Tests =====

#[tokio::test]
async fn test_create_bucket() {
    let service = create_test_service();

    let input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    let req = S3Request::new(input);

    let result = service.create_bucket(req).await;
    assert!(
        result.is_ok(),
        "Failed to create bucket: {:?}",
        result.err()
    );

    let response = result.unwrap();
    assert_eq!(response.output.location, Some("/test-bucket".to_string()));
}

#[tokio::test]
async fn test_head_bucket_exists() {
    let service = create_test_service();

    // Create the bucket first
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Now check it exists
    let head_input = HeadBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    let result = service.head_bucket(S3Request::new(head_input)).await;
    assert!(result.is_ok(), "Bucket should exist");
}

#[tokio::test]
async fn test_head_bucket_not_exists() {
    let service = create_test_service();

    let input = HeadBucketInput::builder()
        .bucket("nonexistent-bucket".to_string())
        .build()
        .unwrap();

    let result = service.head_bucket(S3Request::new(input)).await;
    assert!(result.is_err(), "Should fail for nonexistent bucket");
}

#[tokio::test]
async fn test_delete_bucket() {
    let service = create_test_service();

    // Create the bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Delete it
    let delete_input = DeleteBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    let result = service.delete_bucket(S3Request::new(delete_input)).await;
    assert!(result.is_ok(), "Failed to delete bucket");

    // Verify it's gone
    let head_input = HeadBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    let head_result = service.head_bucket(S3Request::new(head_input)).await;
    assert!(head_result.is_err(), "Bucket should not exist after delete");
}

#[tokio::test]
async fn test_delete_bucket_not_empty() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put an object
    let data = b"test data".to_vec();
    let body = create_streaming_blob(data);

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("file.txt".to_string())
        .body(Some(body))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Try to delete the bucket (should fail)
    let delete_input = DeleteBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    let result = service.delete_bucket(S3Request::new(delete_input)).await;
    assert!(result.is_err(), "Should fail to delete non-empty bucket");

    // Delete the object first
    let delete_obj_input = DeleteObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("file.txt".to_string())
        .build()
        .unwrap();
    service
        .delete_object(S3Request::new(delete_obj_input))
        .await
        .unwrap();

    // Now bucket deletion should succeed
    let delete_input2 = DeleteBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    let result2 = service.delete_bucket(S3Request::new(delete_input2)).await;
    assert!(result2.is_ok(), "Should delete empty bucket");
}

#[tokio::test]
async fn test_create_bucket_already_exists() {
    let service = create_test_service();

    // Create the bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Try to create it again
    let create_input2 = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    let result = service.create_bucket(S3Request::new(create_input2)).await;
    assert!(result.is_err(), "Should fail for duplicate bucket");
}

#[tokio::test]
async fn test_invalid_bucket_name_too_short() {
    let service = create_test_service();

    let input = CreateBucketInput::builder()
        .bucket("ab".to_string()) // Too short (min is 3)
        .build()
        .unwrap();

    let result = service.create_bucket(S3Request::new(input)).await;
    assert!(
        result.is_err(),
        "Should reject bucket name that is too short"
    );
}

#[tokio::test]
async fn test_invalid_bucket_name_uppercase() {
    let service = create_test_service();

    let input = CreateBucketInput::builder()
        .bucket("TestBucket".to_string()) // Has uppercase
        .build()
        .unwrap();

    let result = service.create_bucket(S3Request::new(input)).await;
    assert!(result.is_err(), "Should reject bucket name with uppercase");
}

// ===== Object Tests =====

#[tokio::test]
async fn test_put_and_get_object() {
    let service = create_test_service();

    // Create bucket first
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put an object
    let data = b"Hello, World!".to_vec();
    let body = create_streaming_blob(data.clone());

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("test-key.txt".to_string())
        .body(Some(body))
        .content_type(Some("text/plain".parse().unwrap()))
        .build()
        .unwrap();

    let put_result = service.put_object(S3Request::new(put_input)).await;
    assert!(
        put_result.is_ok(),
        "Failed to put object: {:?}",
        put_result.err()
    );

    // Get the object back
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("test-key.txt".to_string())
        .build()
        .unwrap();

    let get_result = service.get_object(S3Request::new(get_input)).await;
    assert!(
        get_result.is_ok(),
        "Failed to get object: {:?}",
        get_result.err()
    );

    let response = get_result.unwrap();
    assert_eq!(response.output.content_length, Some(data.len() as i64));
}

#[tokio::test]
async fn test_head_object() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put an object
    let data = b"Test data for head".to_vec();
    let body = create_streaming_blob(data.clone());

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("head-test.txt".to_string())
        .body(Some(body))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Head the object
    let head_input = HeadObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("head-test.txt".to_string())
        .build()
        .unwrap();

    let result = service.head_object(S3Request::new(head_input)).await;
    assert!(result.is_ok(), "Failed to head object");

    let response = result.unwrap();
    assert_eq!(response.output.content_length, Some(data.len() as i64));
}

#[tokio::test]
async fn test_delete_object() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put an object
    let data = b"Delete me".to_vec();
    let body = create_streaming_blob(data);

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("to-delete.txt".to_string())
        .body(Some(body))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Delete the object
    let delete_input = DeleteObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("to-delete.txt".to_string())
        .build()
        .unwrap();

    let delete_result = service.delete_object(S3Request::new(delete_input)).await;
    assert!(delete_result.is_ok(), "Failed to delete object");

    // Verify it's gone
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("to-delete.txt".to_string())
        .build()
        .unwrap();

    let get_result = service.get_object(S3Request::new(get_input)).await;
    assert!(get_result.is_err(), "Object should not exist after delete");
}

#[tokio::test]
async fn test_delete_nonexistent_object() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Delete a nonexistent object (S3 delete is idempotent)
    let delete_input = DeleteObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("nonexistent.txt".to_string())
        .build()
        .unwrap();

    let result = service.delete_object(S3Request::new(delete_input)).await;
    assert!(
        result.is_ok(),
        "Delete should succeed even for nonexistent object"
    );
}

#[tokio::test]
async fn test_get_nonexistent_object() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Try to get nonexistent object
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("nonexistent.txt".to_string())
        .build()
        .unwrap();

    let result = service.get_object(S3Request::new(get_input)).await;
    assert!(result.is_err(), "Should fail for nonexistent object");
}

#[tokio::test]
async fn test_put_object_nonexistent_bucket() {
    let service = create_test_service();

    let data = b"Hello".to_vec();
    let body = create_streaming_blob(data);

    let put_input = PutObjectInput::builder()
        .bucket("nonexistent-bucket".to_string())
        .key("test.txt".to_string())
        .body(Some(body))
        .build()
        .unwrap();

    let result = service.put_object(S3Request::new(put_input)).await;
    assert!(result.is_err(), "Should fail for nonexistent bucket");
}

// ===== Copy Object Tests =====

#[tokio::test]
async fn test_copy_object() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put source object
    let data = b"Copy me!".to_vec();
    let body = create_streaming_blob(data.clone());

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("source.txt".to_string())
        .body(Some(body))
        .content_type(Some("text/plain".parse().unwrap()))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Copy the object
    let copy_input = CopyObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("destination.txt".to_string())
        .copy_source(CopySource::Bucket {
            bucket: "test-bucket".into(),
            key: "source.txt".into(),
            version_id: None,
        })
        .build()
        .unwrap();

    let copy_result = service.copy_object(S3Request::new(copy_input)).await;
    assert!(
        copy_result.is_ok(),
        "Failed to copy object: {:?}",
        copy_result.err()
    );

    // Verify the copy exists
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("destination.txt".to_string())
        .build()
        .unwrap();

    let get_result = service.get_object(S3Request::new(get_input)).await;
    assert!(get_result.is_ok(), "Copied object should exist");
}

// ===== Large Object Chunking Tests =====

#[tokio::test]
async fn test_large_object_chunking() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Create a 2MB object (will be chunked into 2 chunks)
    let chunk_size = 1024 * 1024; // 1MB
    let data_size = 2 * chunk_size + 1000; // Just over 2 chunks
    let data: Vec<u8> = (0..data_size).map(|i| (i % 256) as u8).collect();
    let body = create_streaming_blob(data.clone());

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("large-file.bin".to_string())
        .body(Some(body))
        .build()
        .unwrap();

    let put_result = service.put_object(S3Request::new(put_input)).await;
    assert!(
        put_result.is_ok(),
        "Failed to put large object: {:?}",
        put_result.err()
    );

    // Get the object back and verify size
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("large-file.bin".to_string())
        .build()
        .unwrap();

    let get_result = service.get_object(S3Request::new(get_input)).await;
    assert!(get_result.is_ok(), "Failed to get large object");

    let response = get_result.unwrap();
    assert_eq!(response.output.content_length, Some(data_size as i64));
}

#[tokio::test]
async fn test_list_objects_empty_bucket() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // List objects (empty bucket)
    let list_input = ListObjectsV2Input::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();

    let result = service.list_objects_v2(S3Request::new(list_input)).await;
    assert!(result.is_ok(), "Failed to list objects");

    let response = result.unwrap();
    assert_eq!(response.output.key_count, Some(0));
    assert_eq!(response.output.is_truncated, Some(false));
}

#[tokio::test]
async fn test_content_type_inference() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put JSON file (should infer application/json)
    let data = br#"{"key": "value"}"#.to_vec();
    let body = create_streaming_blob(data);

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("data.json".to_string())
        .body(Some(body))
        // No content_type - let it infer
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Head to check content type
    let head_input = HeadObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("data.json".to_string())
        .build()
        .unwrap();

    let result = service
        .head_object(S3Request::new(head_input))
        .await
        .unwrap();

    // Content type should be inferred from extension
    if let Some(content_type) = result.output.content_type {
        assert!(
            content_type.to_string().contains("json"),
            "Content type should be JSON: {:?}",
            content_type
        );
    }
}

// ===== Multipart Upload Tests =====

#[tokio::test]
async fn test_multipart_upload_basic() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Create multipart upload
    let create_mpu_input = CreateMultipartUploadInput::builder()
        .bucket("test-bucket".to_string())
        .key("large-file.bin".to_string())
        .content_type(Some("application/octet-stream".parse().unwrap()))
        .build()
        .unwrap();

    let create_mpu_result = service
        .create_multipart_upload(S3Request::new(create_mpu_input))
        .await;
    assert!(
        create_mpu_result.is_ok(),
        "Failed to create multipart upload: {:?}",
        create_mpu_result.err()
    );

    let create_mpu_response = create_mpu_result.unwrap();
    let upload_id = create_mpu_response.output.upload_id.unwrap();
    assert!(!upload_id.is_empty(), "Upload ID should not be empty");

    // Upload part 1
    let part1_data = b"Part 1 data here!".to_vec();
    let part1_body = create_streaming_blob(part1_data.clone());

    let upload_part1_input = UploadPartInput::builder()
        .bucket("test-bucket".to_string())
        .key("large-file.bin".to_string())
        .upload_id(upload_id.clone())
        .part_number(1)
        .body(Some(part1_body))
        .build()
        .unwrap();

    let upload_part1_result = service
        .upload_part(S3Request::new(upload_part1_input))
        .await;
    assert!(
        upload_part1_result.is_ok(),
        "Failed to upload part 1: {:?}",
        upload_part1_result.err()
    );

    let part1_etag = upload_part1_result
        .unwrap()
        .output
        .e_tag
        .unwrap()
        .to_string();

    // Upload part 2
    let part2_data = b"Part 2 data here too!".to_vec();
    let part2_body = create_streaming_blob(part2_data.clone());

    let upload_part2_input = UploadPartInput::builder()
        .bucket("test-bucket".to_string())
        .key("large-file.bin".to_string())
        .upload_id(upload_id.clone())
        .part_number(2)
        .body(Some(part2_body))
        .build()
        .unwrap();

    let upload_part2_result = service
        .upload_part(S3Request::new(upload_part2_input))
        .await;
    assert!(
        upload_part2_result.is_ok(),
        "Failed to upload part 2: {:?}",
        upload_part2_result.err()
    );

    let part2_etag = upload_part2_result
        .unwrap()
        .output
        .e_tag
        .unwrap()
        .to_string();

    // List parts
    let list_parts_input = ListPartsInput::builder()
        .bucket("test-bucket".to_string())
        .key("large-file.bin".to_string())
        .upload_id(upload_id.clone())
        .build()
        .unwrap();

    let list_parts_result = service.list_parts(S3Request::new(list_parts_input)).await;
    assert!(list_parts_result.is_ok(), "Failed to list parts");
    let parts = list_parts_result.unwrap().output.parts.unwrap();
    assert_eq!(parts.len(), 2, "Should have 2 parts");

    // Complete multipart upload
    let completed_parts = vec![
        CompletedPart {
            part_number: Some(1),
            e_tag: Some(part1_etag),
            ..Default::default()
        },
        CompletedPart {
            part_number: Some(2),
            e_tag: Some(part2_etag),
            ..Default::default()
        },
    ];

    let complete_input = CompleteMultipartUploadInput::builder()
        .bucket("test-bucket".to_string())
        .key("large-file.bin".to_string())
        .upload_id(upload_id)
        .multipart_upload(Some(CompletedMultipartUpload {
            parts: Some(completed_parts),
        }))
        .build()
        .unwrap();

    let complete_result = service
        .complete_multipart_upload(S3Request::new(complete_input))
        .await;
    assert!(
        complete_result.is_ok(),
        "Failed to complete multipart upload: {:?}",
        complete_result.err()
    );

    // Verify the object exists
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("large-file.bin".to_string())
        .build()
        .unwrap();

    let get_result = service.get_object(S3Request::new(get_input)).await;
    assert!(
        get_result.is_ok(),
        "Object should exist after multipart upload"
    );

    // Verify the size
    let expected_size = (part1_data.len() + part2_data.len()) as i64;
    assert_eq!(
        get_result.unwrap().output.content_length,
        Some(expected_size)
    );
}

#[tokio::test]
async fn test_abort_multipart_upload() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Create multipart upload
    let create_mpu_input = CreateMultipartUploadInput::builder()
        .bucket("test-bucket".to_string())
        .key("aborted-file.bin".to_string())
        .build()
        .unwrap();

    let create_mpu_result = service
        .create_multipart_upload(S3Request::new(create_mpu_input))
        .await
        .unwrap();
    let upload_id = create_mpu_result.output.upload_id.unwrap();

    // Upload a part
    let part_data = b"Part data".to_vec();
    let part_body = create_streaming_blob(part_data);

    let upload_part_input = UploadPartInput::builder()
        .bucket("test-bucket".to_string())
        .key("aborted-file.bin".to_string())
        .upload_id(upload_id.clone())
        .part_number(1)
        .body(Some(part_body))
        .build()
        .unwrap();

    service
        .upload_part(S3Request::new(upload_part_input))
        .await
        .unwrap();

    // Abort the multipart upload
    let abort_input = AbortMultipartUploadInput::builder()
        .bucket("test-bucket".to_string())
        .key("aborted-file.bin".to_string())
        .upload_id(upload_id.clone())
        .build()
        .unwrap();

    let abort_result = service
        .abort_multipart_upload(S3Request::new(abort_input))
        .await;
    assert!(
        abort_result.is_ok(),
        "Failed to abort multipart upload: {:?}",
        abort_result.err()
    );

    // Verify the upload is gone (list_parts should fail)
    let list_parts_input = ListPartsInput::builder()
        .bucket("test-bucket".to_string())
        .key("aborted-file.bin".to_string())
        .upload_id(upload_id)
        .build()
        .unwrap();

    let list_parts_result = service.list_parts(S3Request::new(list_parts_input)).await;
    assert!(
        list_parts_result.is_err(),
        "Upload should not exist after abort"
    );
}

// ===== List Buckets Tests =====

#[tokio::test]
async fn test_list_buckets_empty() {
    let service = create_test_service();

    let input = ListBucketsInput::builder().build().unwrap();
    let result = service.list_buckets(S3Request::new(input)).await;
    assert!(result.is_ok(), "Failed to list buckets");

    let response = result.unwrap();
    let buckets = response.output.buckets.unwrap_or_default();
    assert!(buckets.is_empty(), "Should have no buckets");
}

#[tokio::test]
async fn test_list_buckets_multiple() {
    let service = create_test_service();

    // Create several buckets
    let bucket_names = ["alpha-bucket", "beta-bucket", "gamma-bucket"];
    for name in &bucket_names {
        let create_input = CreateBucketInput::builder()
            .bucket(name.to_string())
            .build()
            .unwrap();
        service
            .create_bucket(S3Request::new(create_input))
            .await
            .unwrap();
    }

    // List all buckets
    let list_input = ListBucketsInput::builder().build().unwrap();
    let result = service.list_buckets(S3Request::new(list_input)).await;
    assert!(result.is_ok(), "Failed to list buckets");

    let response = result.unwrap();
    let buckets = response.output.buckets.unwrap();
    assert_eq!(buckets.len(), 3, "Should have 3 buckets");

    // Verify bucket names are returned (sorted)
    let names: Vec<_> = buckets.iter().filter_map(|b| b.name.as_ref()).collect();
    assert!(names.contains(&&"alpha-bucket".to_string()));
    assert!(names.contains(&&"beta-bucket".to_string()));
    assert!(names.contains(&&"gamma-bucket".to_string()));

    // Verify owner info is present
    assert!(response.output.owner.is_some());
}

// ===== Delete Objects (Batch Delete) Tests =====

#[tokio::test]
async fn test_delete_objects_batch() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put multiple objects
    for i in 0..5 {
        let data = format!("content-{}", i).into_bytes();
        let body = create_streaming_blob(data);

        let put_input = PutObjectInput::builder()
            .bucket("test-bucket".to_string())
            .key(format!("file-{}.txt", i))
            .body(Some(body))
            .build()
            .unwrap();
        service.put_object(S3Request::new(put_input)).await.unwrap();
    }

    // Batch delete 3 objects
    let objects_to_delete: Vec<ObjectIdentifier> = (0..3)
        .map(|i| ObjectIdentifier {
            key: format!("file-{}.txt", i),
            version_id: None,
            e_tag: None,
            last_modified_time: None,
            size: None,
        })
        .collect();

    let delete_input = DeleteObjectsInput::builder()
        .bucket("test-bucket".to_string())
        .delete(Delete {
            objects: objects_to_delete,
            quiet: Some(false),
        })
        .build()
        .unwrap();

    let result = service.delete_objects(S3Request::new(delete_input)).await;
    assert!(
        result.is_ok(),
        "Failed to delete objects: {:?}",
        result.err()
    );

    let response = result.unwrap();
    let deleted = response.output.deleted.unwrap_or_default();
    assert_eq!(deleted.len(), 3, "Should have deleted 3 objects");

    // Verify deleted objects are gone
    for i in 0..3 {
        let get_input = GetObjectInput::builder()
            .bucket("test-bucket".to_string())
            .key(format!("file-{}.txt", i))
            .build()
            .unwrap();
        let get_result = service.get_object(S3Request::new(get_input)).await;
        assert!(get_result.is_err(), "Object {} should be deleted", i);
    }

    // Verify remaining objects still exist
    for i in 3..5 {
        let get_input = GetObjectInput::builder()
            .bucket("test-bucket".to_string())
            .key(format!("file-{}.txt", i))
            .build()
            .unwrap();
        let get_result = service.get_object(S3Request::new(get_input)).await;
        assert!(get_result.is_ok(), "Object {} should still exist", i);
    }
}

#[tokio::test]
async fn test_delete_objects_quiet_mode() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put an object
    let data = b"test data".to_vec();
    let body = create_streaming_blob(data);

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("to-delete.txt".to_string())
        .body(Some(body))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Delete with quiet mode (should not report deleted objects)
    let delete_input = DeleteObjectsInput::builder()
        .bucket("test-bucket".to_string())
        .delete(Delete {
            objects: vec![ObjectIdentifier {
                key: "to-delete.txt".to_string(),
                version_id: None,
                e_tag: None,
                last_modified_time: None,
                size: None,
            }],
            quiet: Some(true),
        })
        .build()
        .unwrap();

    let result = service.delete_objects(S3Request::new(delete_input)).await;
    assert!(result.is_ok(), "Failed to delete objects");

    let response = result.unwrap();
    // In quiet mode, deleted list should be empty or None
    let deleted = response.output.deleted.unwrap_or_default();
    assert!(
        deleted.is_empty(),
        "Quiet mode should not report deleted objects"
    );

    // But the object should still be deleted
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("to-delete.txt".to_string())
        .build()
        .unwrap();
    let get_result = service.get_object(S3Request::new(get_input)).await;
    assert!(get_result.is_err(), "Object should be deleted");
}

// ===== ListObjectsV2 Advanced Tests =====

#[tokio::test]
async fn test_list_objects_with_prefix() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put objects with different prefixes
    let keys = [
        "docs/readme.txt",
        "docs/guide.txt",
        "images/logo.png",
        "images/banner.png",
        "root.txt",
    ];

    for key in &keys {
        let data = format!("content for {}", key).into_bytes();
        let body = create_streaming_blob(data);

        let put_input = PutObjectInput::builder()
            .bucket("test-bucket".to_string())
            .key(key.to_string())
            .body(Some(body))
            .build()
            .unwrap();
        service.put_object(S3Request::new(put_input)).await.unwrap();
    }

    // List objects with prefix "docs/"
    let list_input = ListObjectsV2Input::builder()
        .bucket("test-bucket".to_string())
        .prefix(Some("docs/".to_string()))
        .build()
        .unwrap();

    let result = service.list_objects_v2(S3Request::new(list_input)).await;
    assert!(result.is_ok(), "Failed to list objects with prefix");

    let response = result.unwrap();
    let contents = response.output.contents.unwrap_or_default();
    assert_eq!(contents.len(), 2, "Should have 2 objects with docs/ prefix");

    // Verify returned keys
    let returned_keys: Vec<_> = contents.iter().filter_map(|o| o.key.as_ref()).collect();
    assert!(returned_keys.contains(&&"docs/readme.txt".to_string()));
    assert!(returned_keys.contains(&&"docs/guide.txt".to_string()));
}

#[tokio::test]
async fn test_list_objects_with_delimiter() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put objects with hierarchical structure
    let keys = [
        "docs/readme.txt",
        "docs/guide.txt",
        "docs/api/reference.txt",
        "images/logo.png",
        "root.txt",
    ];

    for key in &keys {
        let data = format!("content for {}", key).into_bytes();
        let body = create_streaming_blob(data);

        let put_input = PutObjectInput::builder()
            .bucket("test-bucket".to_string())
            .key(key.to_string())
            .body(Some(body))
            .build()
            .unwrap();
        service.put_object(S3Request::new(put_input)).await.unwrap();
    }

    // List at root with delimiter (should show "virtual directories")
    let list_input = ListObjectsV2Input::builder()
        .bucket("test-bucket".to_string())
        .delimiter(Some("/".to_string()))
        .build()
        .unwrap();

    let result = service.list_objects_v2(S3Request::new(list_input)).await;
    assert!(result.is_ok(), "Failed to list objects with delimiter");

    let response = result.unwrap();
    let contents = response.output.contents.unwrap_or_default();
    let common_prefixes = response.output.common_prefixes.unwrap_or_default();

    // root.txt should be in contents (it's at the root level)
    assert_eq!(contents.len(), 1, "Should have 1 object at root level");
    assert_eq!(
        contents[0].key.as_ref().unwrap(),
        "root.txt",
        "Root object should be root.txt"
    );

    // docs/ and images/ should be in common_prefixes
    assert_eq!(
        common_prefixes.len(),
        2,
        "Should have 2 common prefixes (docs/, images/)"
    );
    let prefixes: Vec<_> = common_prefixes
        .iter()
        .filter_map(|p| p.prefix.as_ref())
        .collect();
    assert!(prefixes.contains(&&"docs/".to_string()));
    assert!(prefixes.contains(&&"images/".to_string()));
}

#[tokio::test]
async fn test_list_objects_max_keys() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put 10 objects
    for i in 0..10 {
        let data = format!("content-{}", i).into_bytes();
        let body = create_streaming_blob(data);

        let put_input = PutObjectInput::builder()
            .bucket("test-bucket".to_string())
            .key(format!("file-{:02}.txt", i))
            .body(Some(body))
            .build()
            .unwrap();
        service.put_object(S3Request::new(put_input)).await.unwrap();
    }

    // List with max_keys = 3
    let list_input = ListObjectsV2Input::builder()
        .bucket("test-bucket".to_string())
        .max_keys(Some(3))
        .build()
        .unwrap();

    let result = service.list_objects_v2(S3Request::new(list_input)).await;
    assert!(result.is_ok(), "Failed to list objects with max_keys");

    let response = result.unwrap();
    let contents = response.output.contents.unwrap_or_default();
    assert_eq!(contents.len(), 3, "Should return exactly 3 objects");
    assert_eq!(
        response.output.is_truncated,
        Some(true),
        "Results should be truncated"
    );
}

// ===== Range Request Tests =====

#[tokio::test]
async fn test_get_object_with_range_int() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put an object with known content
    let data = b"0123456789ABCDEFGHIJ".to_vec(); // 20 bytes
    let body = create_streaming_blob(data.clone());

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("range-test.txt".to_string())
        .body(Some(body))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Request bytes 5-9 (inclusive) -> "56789"
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("range-test.txt".to_string())
        .range(Some(Range::Int {
            first: 5,
            last: Some(9),
        }))
        .build()
        .unwrap();

    let result = service.get_object(S3Request::new(get_input)).await;
    assert!(result.is_ok(), "Range request failed: {:?}", result.err());

    let response = result.unwrap();

    // Check content_range header
    assert!(
        response.output.content_range.is_some(),
        "Content-Range header should be present"
    );
    assert_eq!(
        response.output.content_range.as_deref(),
        Some("bytes 5-9/20"),
        "Content-Range should indicate partial content"
    );

    // Check content_length
    assert_eq!(
        response.output.content_length,
        Some(5),
        "Content-Length should be 5 bytes"
    );

    // Check accept_ranges header
    assert_eq!(
        response.output.accept_ranges.as_deref(),
        Some("bytes"),
        "Accept-Ranges should indicate bytes support"
    );

    // Verify the actual data
    let body_data = collect_streaming_blob(response.output.body.unwrap()).await;
    assert_eq!(body_data, b"56789", "Partial content should match");
}

#[tokio::test]
async fn test_get_object_with_range_suffix() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put an object with known content
    let data = b"0123456789ABCDEFGHIJ".to_vec(); // 20 bytes
    let body = create_streaming_blob(data.clone());

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("range-test.txt".to_string())
        .body(Some(body))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Request last 5 bytes -> "FGHIJ"
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("range-test.txt".to_string())
        .range(Some(Range::Suffix { length: 5 }))
        .build()
        .unwrap();

    let result = service.get_object(S3Request::new(get_input)).await;
    assert!(
        result.is_ok(),
        "Suffix range request failed: {:?}",
        result.err()
    );

    let response = result.unwrap();

    // Check content_range header
    assert_eq!(
        response.output.content_range.as_deref(),
        Some("bytes 15-19/20"),
        "Content-Range should indicate last 5 bytes"
    );

    // Check content_length
    assert_eq!(
        response.output.content_length,
        Some(5),
        "Content-Length should be 5 bytes"
    );

    // Verify the actual data
    let body_data = collect_streaming_blob(response.output.body.unwrap()).await;
    assert_eq!(body_data, b"FGHIJ", "Suffix content should match");
}

#[tokio::test]
async fn test_get_object_with_range_open_end() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put an object with known content
    let data = b"0123456789ABCDEFGHIJ".to_vec(); // 20 bytes
    let body = create_streaming_blob(data.clone());

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("range-test.txt".to_string())
        .body(Some(body))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Request bytes from position 15 to end -> "FGHIJ"
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("range-test.txt".to_string())
        .range(Some(Range::Int {
            first: 15,
            last: None,
        }))
        .build()
        .unwrap();

    let result = service.get_object(S3Request::new(get_input)).await;
    assert!(
        result.is_ok(),
        "Open-end range request failed: {:?}",
        result.err()
    );

    let response = result.unwrap();

    // Check content_range header
    assert_eq!(
        response.output.content_range.as_deref(),
        Some("bytes 15-19/20"),
        "Content-Range should show full remaining range"
    );

    // Check content_length
    assert_eq!(
        response.output.content_length,
        Some(5),
        "Content-Length should be 5 bytes"
    );

    // Verify the actual data
    let body_data = collect_streaming_blob(response.output.body.unwrap()).await;
    assert_eq!(body_data, b"FGHIJ", "Open-end content should match");
}

#[tokio::test]
async fn test_get_object_without_range_has_accept_ranges() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put an object
    let data = b"Test content".to_vec();
    let body = create_streaming_blob(data.clone());

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("test.txt".to_string())
        .body(Some(body))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Get without range
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("test.txt".to_string())
        .build()
        .unwrap();

    let result = service.get_object(S3Request::new(get_input)).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // Should still have accept_ranges even without a range request
    assert_eq!(
        response.output.accept_ranges.as_deref(),
        Some("bytes"),
        "Accept-Ranges should indicate bytes support"
    );

    // content_range should be None for full object
    assert!(
        response.output.content_range.is_none(),
        "Content-Range should not be present for full object"
    );
}

#[tokio::test]
async fn test_head_object_has_accept_ranges() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put an object
    let data = b"Test content".to_vec();
    let body = create_streaming_blob(data.clone());

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("test.txt".to_string())
        .body(Some(body))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Head the object
    let head_input = HeadObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("test.txt".to_string())
        .build()
        .unwrap();

    let result = service.head_object(S3Request::new(head_input)).await;
    assert!(result.is_ok());

    let response = result.unwrap();

    // HeadObject should indicate range support
    assert_eq!(
        response.output.accept_ranges.as_deref(),
        Some("bytes"),
        "HeadObject should indicate bytes support in Accept-Ranges"
    );
}

#[tokio::test]
async fn test_get_object_range_on_large_chunked_object() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Put a large object that will be chunked (> 1MB)
    // Use a pattern that's easy to verify
    let data: Vec<u8> = (0..1_500_000u32).map(|i| (i % 256) as u8).collect();
    let body = create_streaming_blob(data.clone());

    let put_input = PutObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("large-file.bin".to_string())
        .body(Some(body))
        .build()
        .unwrap();
    service.put_object(S3Request::new(put_input)).await.unwrap();

    // Request a range that spans chunk boundaries
    // Chunk size is 1MB (1_048_576 bytes), so request from 1MB-100 to 1MB+100
    let start = 1_048_476u64; // 100 bytes before 1MB boundary
    let end = 1_048_676u64; // 100 bytes after 1MB boundary (201 total bytes)

    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("large-file.bin".to_string())
        .range(Some(Range::Int {
            first: start,
            last: Some(end),
        }))
        .build()
        .unwrap();

    let result = service.get_object(S3Request::new(get_input)).await;
    assert!(
        result.is_ok(),
        "Range request on chunked object failed: {:?}",
        result.err()
    );

    let response = result.unwrap();

    // Check content_length (201 bytes)
    assert_eq!(
        response.output.content_length,
        Some(201),
        "Content-Length should be 201 bytes"
    );

    // Verify the actual data matches the expected pattern
    let body_data = collect_streaming_blob(response.output.body.unwrap()).await;
    let expected: Vec<u8> = (start..=end).map(|i| (i % 256) as u8).collect();
    assert_eq!(
        body_data, expected,
        "Chunked range data should match pattern"
    );
}

// ===== Helper Functions =====

/// Collect all bytes from a streaming blob.
async fn collect_streaming_blob(blob: StreamingBlob) -> Vec<u8> {
    use futures::TryStreamExt;

    let mut result = Vec::new();
    let mut stream = blob.into_stream();
    while let Some(chunk) = stream.try_next().await.unwrap() {
        result.extend_from_slice(&chunk);
    }
    result
}

/// Create a streaming blob from bytes for testing.
fn create_streaming_blob(data: Vec<u8>) -> StreamingBlob {
    use bytes::Bytes;
    use futures::stream;

    let bytes = Bytes::from(data);
    let data_stream = stream::once(async move { Ok::<_, std::io::Error>(bytes) });
    StreamingBlob::wrap(data_stream)
}

// ===== Bucket Versioning Tests =====

#[tokio::test]
async fn test_bucket_versioning_default_disabled() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("version-test".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Get versioning status - should be disabled by default (None)
    let get_input = GetBucketVersioningInput::builder()
        .bucket("version-test".to_string())
        .build()
        .unwrap();

    let result = service
        .get_bucket_versioning(S3Request::new(get_input))
        .await;
    assert!(result.is_ok(), "Failed to get bucket versioning");

    let response = result.unwrap();
    // Status should be None for disabled buckets
    assert!(
        response.output.status.is_none(),
        "Versioning should be disabled by default"
    );
}

#[tokio::test]
async fn test_enable_bucket_versioning() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("version-test".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Enable versioning
    let put_input = PutBucketVersioningInput::builder()
        .bucket("version-test".to_string())
        .versioning_configuration(VersioningConfiguration {
            status: Some(BucketVersioningStatus::from_static(
                BucketVersioningStatus::ENABLED,
            )),
            mfa_delete: None,
        })
        .build()
        .unwrap();

    let put_result = service
        .put_bucket_versioning(S3Request::new(put_input))
        .await;
    assert!(put_result.is_ok(), "Failed to enable versioning");

    // Verify it's enabled
    let get_input = GetBucketVersioningInput::builder()
        .bucket("version-test".to_string())
        .build()
        .unwrap();

    let get_result = service
        .get_bucket_versioning(S3Request::new(get_input))
        .await;
    assert!(get_result.is_ok(), "Failed to get bucket versioning");

    let response = get_result.unwrap();
    assert!(response.output.status.is_some(), "Status should be present");
    assert_eq!(
        response.output.status.as_ref().unwrap().as_str(),
        "Enabled",
        "Versioning should be enabled"
    );
}

#[tokio::test]
async fn test_suspend_bucket_versioning() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("version-test".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Enable versioning first
    let enable_input = PutBucketVersioningInput::builder()
        .bucket("version-test".to_string())
        .versioning_configuration(VersioningConfiguration {
            status: Some(BucketVersioningStatus::from_static(
                BucketVersioningStatus::ENABLED,
            )),
            mfa_delete: None,
        })
        .build()
        .unwrap();
    service
        .put_bucket_versioning(S3Request::new(enable_input))
        .await
        .unwrap();

    // Suspend versioning
    let suspend_input = PutBucketVersioningInput::builder()
        .bucket("version-test".to_string())
        .versioning_configuration(VersioningConfiguration {
            status: Some(BucketVersioningStatus::from_static(
                BucketVersioningStatus::SUSPENDED,
            )),
            mfa_delete: None,
        })
        .build()
        .unwrap();

    let suspend_result = service
        .put_bucket_versioning(S3Request::new(suspend_input))
        .await;
    assert!(suspend_result.is_ok(), "Failed to suspend versioning");

    // Verify it's suspended
    let get_input = GetBucketVersioningInput::builder()
        .bucket("version-test".to_string())
        .build()
        .unwrap();

    let get_result = service
        .get_bucket_versioning(S3Request::new(get_input))
        .await;
    assert!(get_result.is_ok(), "Failed to get bucket versioning");

    let response = get_result.unwrap();
    assert_eq!(
        response.output.status.as_ref().unwrap().as_str(),
        "Suspended",
        "Versioning should be suspended"
    );
}

// ===== Bucket Encryption Tests =====

#[tokio::test]
async fn test_put_and_get_bucket_encryption_aes256() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("encryption-test".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Set AES256 encryption
    let put_input = PutBucketEncryptionInput::builder()
        .bucket("encryption-test".to_string())
        .server_side_encryption_configuration(ServerSideEncryptionConfiguration {
            rules: vec![ServerSideEncryptionRule {
                apply_server_side_encryption_by_default: Some(ServerSideEncryptionByDefault {
                    sse_algorithm: s3s::dto::ServerSideEncryption::from_static(
                        s3s::dto::ServerSideEncryption::AES256,
                    ),
                    kms_master_key_id: None,
                }),
                bucket_key_enabled: Some(false),
            }],
        })
        .build()
        .unwrap();

    let put_result = service
        .put_bucket_encryption(S3Request::new(put_input))
        .await;
    assert!(
        put_result.is_ok(),
        "Failed to set bucket encryption: {:?}",
        put_result.err()
    );

    // Get encryption config
    let get_input = GetBucketEncryptionInput::builder()
        .bucket("encryption-test".to_string())
        .build()
        .unwrap();

    let get_result = service
        .get_bucket_encryption(S3Request::new(get_input))
        .await;
    assert!(
        get_result.is_ok(),
        "Failed to get bucket encryption: {:?}",
        get_result.err()
    );

    let response = get_result.unwrap();
    let config = response
        .output
        .server_side_encryption_configuration
        .expect("Should have encryption config");
    assert!(!config.rules.is_empty(), "Should have at least one rule");

    let rule = &config.rules[0];
    let default_enc = rule
        .apply_server_side_encryption_by_default
        .as_ref()
        .expect("Should have default encryption");
    assert_eq!(
        default_enc.sse_algorithm.as_str(),
        "AES256",
        "Should be AES256"
    );
}

#[tokio::test]
async fn test_delete_bucket_encryption() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("encryption-test".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Set encryption first
    let put_input = PutBucketEncryptionInput::builder()
        .bucket("encryption-test".to_string())
        .server_side_encryption_configuration(ServerSideEncryptionConfiguration {
            rules: vec![ServerSideEncryptionRule {
                apply_server_side_encryption_by_default: Some(ServerSideEncryptionByDefault {
                    sse_algorithm: s3s::dto::ServerSideEncryption::from_static(
                        s3s::dto::ServerSideEncryption::AES256,
                    ),
                    kms_master_key_id: None,
                }),
                bucket_key_enabled: None,
            }],
        })
        .build()
        .unwrap();
    service
        .put_bucket_encryption(S3Request::new(put_input))
        .await
        .unwrap();

    // Delete encryption
    let delete_input = DeleteBucketEncryptionInput::builder()
        .bucket("encryption-test".to_string())
        .build()
        .unwrap();

    let delete_result = service
        .delete_bucket_encryption(S3Request::new(delete_input))
        .await;
    assert!(
        delete_result.is_ok(),
        "Failed to delete bucket encryption: {:?}",
        delete_result.err()
    );

    // Verify encryption is gone (should return an error)
    let get_input = GetBucketEncryptionInput::builder()
        .bucket("encryption-test".to_string())
        .build()
        .unwrap();

    let get_result = service
        .get_bucket_encryption(S3Request::new(get_input))
        .await;
    assert!(
        get_result.is_err(),
        "Should fail when no encryption configured"
    );
}

#[tokio::test]
async fn test_get_encryption_no_config_returns_error() {
    let service = create_test_service();

    // Create bucket without setting encryption
    let create_input = CreateBucketInput::builder()
        .bucket("no-encryption-test".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Get encryption config should fail
    let get_input = GetBucketEncryptionInput::builder()
        .bucket("no-encryption-test".to_string())
        .build()
        .unwrap();

    let get_result = service
        .get_bucket_encryption(S3Request::new(get_input))
        .await;
    assert!(
        get_result.is_err(),
        "Should error when no encryption configured"
    );
}

// ===== Bucket Lifecycle Tests =====

#[tokio::test]
async fn test_put_and_get_lifecycle_configuration() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("lifecycle-test".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Set lifecycle configuration with expiration rule
    let put_input = PutBucketLifecycleConfigurationInput::builder()
        .bucket("lifecycle-test".to_string())
        .lifecycle_configuration(Some(BucketLifecycleConfiguration {
            rules: vec![s3s::dto::LifecycleRule {
                id: Some("expire-old-objects".to_string()),
                status: ExpirationStatus::from_static(ExpirationStatus::ENABLED),
                filter: Some(LifecycleRuleFilter {
                    prefix: Some("logs/".to_string()),
                    ..Default::default()
                }),
                expiration: Some(LifecycleExpiration {
                    days: Some(30),
                    date: None,
                    expired_object_delete_marker: None,
                }),
                transitions: None,
                noncurrent_version_expiration: None,
                noncurrent_version_transitions: None,
                abort_incomplete_multipart_upload: None,
                prefix: None,
            }],
        }))
        .build()
        .unwrap();

    let put_result = service
        .put_bucket_lifecycle_configuration(S3Request::new(put_input))
        .await;
    assert!(
        put_result.is_ok(),
        "Failed to set lifecycle configuration: {:?}",
        put_result.err()
    );

    // Get lifecycle config
    let get_input = GetBucketLifecycleConfigurationInput::builder()
        .bucket("lifecycle-test".to_string())
        .build()
        .unwrap();

    let get_result = service
        .get_bucket_lifecycle_configuration(S3Request::new(get_input))
        .await;
    assert!(
        get_result.is_ok(),
        "Failed to get lifecycle configuration: {:?}",
        get_result.err()
    );

    let response = get_result.unwrap();
    let rules = response.output.rules.expect("Should have rules");
    assert_eq!(rules.len(), 1, "Should have one rule");

    let rule = &rules[0];
    assert_eq!(
        rule.id.as_deref(),
        Some("expire-old-objects"),
        "Rule ID should match"
    );
    assert_eq!(rule.status.as_str(), "Enabled", "Rule should be enabled");

    let expiration = rule.expiration.as_ref().expect("Should have expiration");
    assert_eq!(expiration.days, Some(30), "Expiration should be 30 days");
}

#[tokio::test]
async fn test_delete_lifecycle_configuration() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("lifecycle-test".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Set lifecycle config first
    let put_input = PutBucketLifecycleConfigurationInput::builder()
        .bucket("lifecycle-test".to_string())
        .lifecycle_configuration(Some(BucketLifecycleConfiguration {
            rules: vec![s3s::dto::LifecycleRule {
                id: Some("test-rule".to_string()),
                status: ExpirationStatus::from_static(ExpirationStatus::ENABLED),
                filter: None,
                expiration: Some(LifecycleExpiration {
                    days: Some(7),
                    date: None,
                    expired_object_delete_marker: None,
                }),
                transitions: None,
                noncurrent_version_expiration: None,
                noncurrent_version_transitions: None,
                abort_incomplete_multipart_upload: None,
                prefix: None,
            }],
        }))
        .build()
        .unwrap();
    service
        .put_bucket_lifecycle_configuration(S3Request::new(put_input))
        .await
        .unwrap();

    // Delete lifecycle config
    let delete_input = DeleteBucketLifecycleInput::builder()
        .bucket("lifecycle-test".to_string())
        .build()
        .unwrap();

    let delete_result = service
        .delete_bucket_lifecycle(S3Request::new(delete_input))
        .await;
    assert!(
        delete_result.is_ok(),
        "Failed to delete lifecycle config: {:?}",
        delete_result.err()
    );

    // Verify lifecycle is gone
    let get_input = GetBucketLifecycleConfigurationInput::builder()
        .bucket("lifecycle-test".to_string())
        .build()
        .unwrap();

    let get_result = service
        .get_bucket_lifecycle_configuration(S3Request::new(get_input))
        .await;
    assert!(
        get_result.is_err(),
        "Should fail when no lifecycle configured"
    );
}

#[tokio::test]
async fn test_lifecycle_with_transition_rule() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("lifecycle-test".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Set lifecycle with transition rule
    let put_input = PutBucketLifecycleConfigurationInput::builder()
        .bucket("lifecycle-test".to_string())
        .lifecycle_configuration(Some(BucketLifecycleConfiguration {
            rules: vec![s3s::dto::LifecycleRule {
                id: Some("archive-old-data".to_string()),
                status: ExpirationStatus::from_static(ExpirationStatus::ENABLED),
                filter: Some(LifecycleRuleFilter {
                    prefix: Some("archive/".to_string()),
                    ..Default::default()
                }),
                expiration: None,
                transitions: Some(vec![Transition {
                    days: Some(90),
                    date: None,
                    storage_class: Some(TransitionStorageClass::from("GLACIER".to_string())),
                }]),
                noncurrent_version_expiration: None,
                noncurrent_version_transitions: None,
                abort_incomplete_multipart_upload: None,
                prefix: None,
            }],
        }))
        .build()
        .unwrap();

    let put_result = service
        .put_bucket_lifecycle_configuration(S3Request::new(put_input))
        .await;
    assert!(
        put_result.is_ok(),
        "Failed to set lifecycle with transition: {:?}",
        put_result.err()
    );

    // Get and verify
    let get_input = GetBucketLifecycleConfigurationInput::builder()
        .bucket("lifecycle-test".to_string())
        .build()
        .unwrap();

    let get_result = service
        .get_bucket_lifecycle_configuration(S3Request::new(get_input))
        .await;
    assert!(get_result.is_ok());

    let response = get_result.unwrap();
    let rules = response.output.rules.expect("Should have rules");
    let rule = &rules[0];

    let transitions = rule.transitions.as_ref().expect("Should have transitions");
    assert_eq!(transitions.len(), 1, "Should have one transition");
    assert_eq!(
        transitions[0].days,
        Some(90),
        "Should transition after 90 days"
    );
}

#[tokio::test]
async fn test_get_lifecycle_no_config_returns_error() {
    let service = create_test_service();

    // Create bucket without lifecycle config
    let create_input = CreateBucketInput::builder()
        .bucket("no-lifecycle-test".to_string())
        .build()
        .unwrap();
    service
        .create_bucket(S3Request::new(create_input))
        .await
        .unwrap();

    // Get lifecycle config should fail
    let get_input = GetBucketLifecycleConfigurationInput::builder()
        .bucket("no-lifecycle-test".to_string())
        .build()
        .unwrap();

    let get_result = service
        .get_bucket_lifecycle_configuration(S3Request::new(get_input))
        .await;
    assert!(
        get_result.is_err(),
        "Should error when no lifecycle configured"
    );
}
