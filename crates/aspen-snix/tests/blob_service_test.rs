//! Integration tests for IrohBlobService.
//!
//! Tests the BlobService trait implementation against real storage backends.

use std::sync::Arc;

use aspen_blob::InMemoryBlobStore;
use aspen_snix::IrohBlobService;
use proptest::prelude::*;
use rstest::rstest;
use snix_castore::B3Digest;
use snix_castore::blobservice::BlobService;
use tokio::io::AsyncWriteExt;

/// Create a test blob service with in-memory backend.
fn make_test_service() -> IrohBlobService<InMemoryBlobStore> {
    let store = InMemoryBlobStore::new();
    IrohBlobService::new(store)
}

// =============================================================================
// Basic Operations
// =============================================================================

#[tokio::test]
async fn test_write_and_read_small_blob() {
    let service = make_test_service();
    let data = b"hello world";

    // Write
    let mut writer = service.open_write().await;
    writer.write_all(data).await.unwrap();
    let digest = writer.close().await.unwrap();

    // Verify exists
    assert!(service.has(&digest).await.unwrap());

    // Read back
    let reader = service.open_read(&digest).await.unwrap().unwrap();
    let bytes = read_all(reader).await;
    assert_eq!(bytes, data);
}

#[tokio::test]
async fn test_write_and_read_empty_blob() {
    let service = make_test_service();
    let data = b"";

    let mut writer = service.open_write().await;
    writer.write_all(data).await.unwrap();
    let digest = writer.close().await.unwrap();

    assert!(service.has(&digest).await.unwrap());

    let reader = service.open_read(&digest).await.unwrap().unwrap();
    let bytes = read_all(reader).await;
    assert!(bytes.is_empty());
}

#[tokio::test]
async fn test_blob_not_found() {
    let service = make_test_service();
    let fake_digest = B3Digest::from(&[0u8; 32]);

    assert!(!service.has(&fake_digest).await.unwrap());
    assert!(service.open_read(&fake_digest).await.unwrap().is_none());
}

#[tokio::test]
async fn test_chunks_for_existing_blob() {
    let service = make_test_service();
    let data = b"test data for chunks";

    let mut writer = service.open_write().await;
    writer.write_all(data).await.unwrap();
    let digest = writer.close().await.unwrap();

    // chunks() should return Some(empty) for existing blob
    // (iroh-blobs handles chunking internally)
    let chunks = service.chunks(&digest).await.unwrap();
    assert!(chunks.is_some());
    assert!(chunks.unwrap().is_empty());
}

#[tokio::test]
async fn test_chunks_for_missing_blob() {
    let service = make_test_service();
    let fake_digest = B3Digest::from(&[0u8; 32]);

    let chunks = service.chunks(&fake_digest).await.unwrap();
    assert!(chunks.is_none());
}

// =============================================================================
// Multiple Writes - Content Addressing
// =============================================================================

#[tokio::test]
async fn test_same_content_same_digest() {
    let service = make_test_service();
    let data = b"deterministic content";

    // Write same data twice
    let mut writer1 = service.open_write().await;
    writer1.write_all(data).await.unwrap();
    let digest1 = writer1.close().await.unwrap();

    let mut writer2 = service.open_write().await;
    writer2.write_all(data).await.unwrap();
    let digest2 = writer2.close().await.unwrap();

    // Should produce same digest (content-addressed)
    assert_eq!(digest1, digest2);
}

#[tokio::test]
async fn test_different_content_different_digest() {
    let service = make_test_service();

    let mut writer1 = service.open_write().await;
    writer1.write_all(b"content A").await.unwrap();
    let digest1 = writer1.close().await.unwrap();

    let mut writer2 = service.open_write().await;
    writer2.write_all(b"content B").await.unwrap();
    let digest2 = writer2.close().await.unwrap();

    assert_ne!(digest1, digest2);
}

// =============================================================================
// Incremental Writes
// =============================================================================

#[tokio::test]
async fn test_incremental_writes() {
    let service = make_test_service();

    // Write in chunks
    let mut writer = service.open_write().await;
    writer.write_all(b"hello ").await.unwrap();
    writer.write_all(b"world").await.unwrap();
    let digest = writer.close().await.unwrap();

    // Should be same as single write
    let mut writer2 = service.open_write().await;
    writer2.write_all(b"hello world").await.unwrap();
    let digest2 = writer2.close().await.unwrap();

    assert_eq!(digest, digest2);

    // Verify content
    let reader = service.open_read(&digest).await.unwrap().unwrap();
    let bytes = read_all(reader).await;
    assert_eq!(bytes, b"hello world");
}

// =============================================================================
// Size Limits (Tiger Style)
// =============================================================================

#[tokio::test]
async fn test_blob_size_limit() {
    let service = make_test_service();
    let max_size = aspen_snix::MAX_BLOB_SIZE_BYTES as usize;

    // Create data at exactly the limit
    let mut writer = service.open_write().await;

    // Write in chunks to avoid OOM
    let chunk_size = 1024 * 1024; // 1 MB chunks
    let mut written = 0;
    while written < max_size {
        let to_write = std::cmp::min(chunk_size, max_size - written);
        let chunk = vec![0u8; to_write];
        writer.write_all(&chunk).await.unwrap();
        written += to_write;
    }

    // Should succeed at exactly the limit
    let _ = writer.close().await.unwrap();

    // Now try to exceed limit
    let mut writer2 = service.open_write().await;
    let oversized = vec![0u8; max_size + 1];
    let result = writer2.write_all(&oversized).await;

    // Should fail
    assert!(result.is_err());
}

// =============================================================================
// Concurrent Access
// =============================================================================

#[tokio::test]
async fn test_concurrent_reads() {
    let service = Arc::new(make_test_service());
    let data = b"shared data for concurrent reads";

    // Write data
    let mut writer = service.open_write().await;
    writer.write_all(data).await.unwrap();
    let digest = writer.close().await.unwrap();

    // Read concurrently
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let svc = Arc::clone(&service);
            let d = digest;
            tokio::spawn(async move {
                let reader = svc.open_read(&d).await.unwrap().unwrap();
                read_all(reader).await
            })
        })
        .collect();

    for handle in handles {
        let bytes = handle.await.unwrap();
        assert_eq!(bytes, data);
    }
}

#[tokio::test]
async fn test_concurrent_writes_same_content() {
    let service = Arc::new(make_test_service());
    let data = b"content for concurrent writes";

    // Write same data concurrently
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let svc = Arc::clone(&service);
            let d = data.to_vec();
            tokio::spawn(async move {
                let mut writer = svc.open_write().await;
                writer.write_all(&d).await.unwrap();
                writer.close().await.unwrap()
            })
        })
        .collect();

    let mut digests = Vec::new();
    for handle in handles {
        digests.push(handle.await.unwrap());
    }

    // All should produce same digest (content-addressed)
    let first = &digests[0];
    for digest in &digests[1..] {
        assert_eq!(digest, first);
    }
}

// =============================================================================
// Writer State Management
// =============================================================================

#[tokio::test]
async fn test_writer_close_twice() {
    let service = make_test_service();

    let mut writer = service.open_write().await;
    writer.write_all(b"data").await.unwrap();
    let digest1 = writer.close().await.unwrap();

    // Second close should return same digest
    let digest2 = writer.close().await.unwrap();
    assert_eq!(digest1, digest2);
}

#[tokio::test]
async fn test_write_after_close() {
    let service = make_test_service();

    let mut writer = service.open_write().await;
    writer.write_all(b"data").await.unwrap();
    let _ = writer.close().await.unwrap();

    // Write after close should fail
    let result = writer.write_all(b"more data").await;
    assert!(result.is_err());
}

// =============================================================================
// Property-Based Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn prop_roundtrip_arbitrary_data(data in prop::collection::vec(any::<u8>(), 0..10_000)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = make_test_service();

            let mut writer = service.open_write().await;
            writer.write_all(&data).await.unwrap();
            let digest = writer.close().await.unwrap();

            let reader = service.open_read(&digest).await.unwrap().unwrap();
            let read_data = read_all(reader).await;

            prop_assert_eq!(read_data, data);
            Ok(())
        }).unwrap();
    }

    #[test]
    fn prop_content_addressing_deterministic(data in prop::collection::vec(any::<u8>(), 1..1_000)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = make_test_service();

            let mut writer1 = service.open_write().await;
            writer1.write_all(&data).await.unwrap();
            let digest1 = writer1.close().await.unwrap();

            let mut writer2 = service.open_write().await;
            writer2.write_all(&data).await.unwrap();
            let digest2 = writer2.close().await.unwrap();

            prop_assert_eq!(digest1, digest2);
            Ok(())
        }).unwrap();
    }

    #[test]
    fn prop_has_after_write(data in prop::collection::vec(any::<u8>(), 1..1_000)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = make_test_service();

            let mut writer = service.open_write().await;
            writer.write_all(&data).await.unwrap();
            let digest = writer.close().await.unwrap();

            prop_assert!(service.has(&digest).await.unwrap());
            Ok(())
        }).unwrap();
    }
}

// =============================================================================
// Parameterized Tests (rstest)
// =============================================================================

#[rstest]
#[case::empty(b"")]
#[case::single_byte(b"x")]
#[case::small(b"hello")]
#[case::medium(&[b'x'; 1024])]
#[case::large(&[b'y'; 65536])]
#[tokio::test]
async fn test_various_blob_sizes(#[case] data: &[u8]) {
    let service = make_test_service();

    let mut writer = service.open_write().await;
    writer.write_all(data).await.unwrap();
    let digest = writer.close().await.unwrap();

    let reader = service.open_read(&digest).await.unwrap().unwrap();
    let read_data = read_all(reader).await;
    assert_eq!(read_data, data);
}

// =============================================================================
// Helpers
// =============================================================================

/// Read all bytes from a BlobReader.
async fn read_all(reader: Box<dyn snix_castore::blobservice::BlobReader>) -> Vec<u8> {
    use tokio::io::AsyncReadExt;

    // Wrap in adapter for tokio AsyncRead
    struct TokioAdapter(Box<dyn snix_castore::blobservice::BlobReader>);

    impl tokio::io::AsyncRead for TokioAdapter {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut *self.0).poll_read(cx, buf)
        }
    }

    let mut adapter = TokioAdapter(reader);
    let mut buf = Vec::new();
    adapter.read_to_end(&mut buf).await.unwrap();
    buf
}
