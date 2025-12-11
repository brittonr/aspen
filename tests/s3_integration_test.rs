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
use s3s::{S3Request, S3};
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
    assert!(result.is_ok(), "Failed to create bucket: {:?}", result.err());

    let response = result.unwrap();
    assert_eq!(
        response.output.location,
        Some("/test-bucket".to_string())
    );
}

#[tokio::test]
async fn test_head_bucket_exists() {
    let service = create_test_service();

    // Create the bucket first
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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
async fn test_create_bucket_already_exists() {
    let service = create_test_service();

    // Create the bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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
    assert!(result.is_err(), "Should reject bucket name that is too short");
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
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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
    assert!(put_result.is_ok(), "Failed to put object: {:?}", put_result.err());

    // Get the object back
    let get_input = GetObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("test-key.txt".to_string())
        .build()
        .unwrap();

    let get_result = service.get_object(S3Request::new(get_input)).await;
    assert!(get_result.is_ok(), "Failed to get object: {:?}", get_result.err());

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
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

    // Delete a nonexistent object (S3 delete is idempotent)
    let delete_input = DeleteObjectInput::builder()
        .bucket("test-bucket".to_string())
        .key("nonexistent.txt".to_string())
        .build()
        .unwrap();

    let result = service.delete_object(S3Request::new(delete_input)).await;
    assert!(result.is_ok(), "Delete should succeed even for nonexistent object");
}

#[tokio::test]
async fn test_get_nonexistent_object() {
    let service = create_test_service();

    // Create bucket
    let create_input = CreateBucketInput::builder()
        .bucket("test-bucket".to_string())
        .build()
        .unwrap();
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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
    assert!(copy_result.is_ok(), "Failed to copy object: {:?}", copy_result.err());

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
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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
    assert!(put_result.is_ok(), "Failed to put large object: {:?}", put_result.err());

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
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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
    service.create_bucket(S3Request::new(create_input)).await.unwrap();

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

    let result = service.head_object(S3Request::new(head_input)).await.unwrap();

    // Content type should be inferred from extension
    if let Some(content_type) = result.output.content_type {
        assert!(
            content_type.to_string().contains("json"),
            "Content type should be JSON: {:?}",
            content_type
        );
    }
}

// ===== Helper Functions =====

/// Create a streaming blob from bytes for testing.
fn create_streaming_blob(data: Vec<u8>) -> StreamingBlob {
    use bytes::Bytes;
    use futures::stream;

    let bytes = Bytes::from(data);
    let data_stream = stream::once(async move { Ok::<_, std::io::Error>(bytes) });
    StreamingBlob::wrap(data_stream)
}
