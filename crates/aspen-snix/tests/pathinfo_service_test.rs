//! Integration tests for RaftPathInfoService.
//!
//! Tests the PathInfoService trait implementation against Raft KV storage.

use std::sync::Arc;

use aspen_snix::RaftPathInfoService;
use aspen_testing::DeterministicKeyValueStore;
use futures::StreamExt;
use nix_compat::store_path::StorePath;
use proptest::prelude::*;
use rstest::rstest;
use snix_castore::B3Digest;
use snix_castore::Node;
use snix_store::pathinfoservice::PathInfo;
use snix_store::pathinfoservice::PathInfoService;

/// Create a test path info service with in-memory KV backend.
fn make_test_service() -> RaftPathInfoService<DeterministicKeyValueStore> {
    RaftPathInfoService::from_arc(DeterministicKeyValueStore::new())
}

/// Valid characters for Nix base32 encoding (excludes e, o, t, u).
const NIX_BASE32_CHARS: &[u8; 32] = b"0123456789abcdfghijklmnpqrsvwxyz";

/// Generate a valid Nix base32 store hash from an index.
/// Store hashes are 32 characters (20 bytes encoded).
fn make_nix_hash(index: usize) -> String {
    let mut hash = String::with_capacity(32);
    let mut n = index;
    for _ in 0..32 {
        hash.push(NIX_BASE32_CHARS[n % 32] as char);
        n /= 32;
    }
    hash
}

/// Create a test PathInfo with given parameters.
fn make_path_info(hash: &str, name: &str, nar_size: u64, refs: &[&str]) -> PathInfo {
    let store_path = StorePath::from_bytes(format!("{}-{}", hash, name).as_bytes()).unwrap();

    let root_digest = B3Digest::from(&[1u8; 32]);
    let node = Node::File {
        digest: root_digest,
        size: nar_size,
        executable: false,
    };

    let references: Vec<StorePath<String>> =
        refs.iter().filter_map(|r| StorePath::from_bytes(r.as_bytes()).ok()).collect();

    let mut nar_sha256 = [0u8; 32];
    nar_sha256[0] = (nar_size % 256) as u8;

    PathInfo {
        store_path,
        node,
        references,
        nar_size,
        nar_sha256,
        signatures: vec![],
        deriver: None,
        ca: None,
    }
}

/// Create a simple test PathInfo.
fn simple_path_info() -> PathInfo {
    make_path_info("0c6kzph7l0dcbfmjap64f0czdafn3b7x", "hello-2.10", 1024, &[])
}

// =============================================================================
// Basic Operations
// =============================================================================

#[tokio::test]
async fn test_put_and_get() {
    let service = make_test_service();
    let path_info = simple_path_info();
    let digest = *path_info.store_path.digest();

    // Put
    let returned = service.put(path_info.clone()).await.unwrap();
    assert_eq!(returned.store_path, path_info.store_path);

    // Get
    let retrieved = service.get(digest).await.unwrap().unwrap();
    assert_eq!(retrieved.store_path, path_info.store_path);
    assert_eq!(retrieved.nar_size, path_info.nar_size);
}

#[tokio::test]
async fn test_get_nonexistent() {
    let service = make_test_service();
    let fake_digest = [0u8; 20];

    let result = service.get(fake_digest).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_put_with_references() {
    let service = make_test_service();

    // Create path with references
    let path_info = make_path_info("0c6kzph7l0dcbfmjap64f0czdafn3b7x", "hello-2.10", 1024, &[
        "1234567890123456789012345678901-dep1",
        "abcdefghijklmnopqrstuvwxyz12345-dep2",
    ]);

    let digest = *path_info.store_path.digest();
    service.put(path_info.clone()).await.unwrap();

    let retrieved = service.get(digest).await.unwrap().unwrap();
    assert_eq!(retrieved.references.len(), path_info.references.len());
}

#[tokio::test]
async fn test_put_with_deriver() {
    let service = make_test_service();
    let mut path_info = simple_path_info();

    // Add deriver
    path_info.deriver = StorePath::from_bytes(b"zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzy-hello-2.10.drv").ok();

    let digest = *path_info.store_path.digest();
    service.put(path_info.clone()).await.unwrap();

    let retrieved = service.get(digest).await.unwrap().unwrap();
    assert!(retrieved.deriver.is_some());
}

// =============================================================================
// Update Operations
// =============================================================================

#[tokio::test]
async fn test_put_overwrites() {
    let service = make_test_service();

    let path_info1 = make_path_info("0c6kzph7l0dcbfmjap64f0czdafn3b7x", "hello-2.10", 1024, &[]);
    let digest = *path_info1.store_path.digest();

    service.put(path_info1).await.unwrap();

    // Put with same store path but different size
    let path_info2 = make_path_info("0c6kzph7l0dcbfmjap64f0czdafn3b7x", "hello-2.10", 2048, &[]);
    service.put(path_info2).await.unwrap();

    let retrieved = service.get(digest).await.unwrap().unwrap();
    assert_eq!(retrieved.nar_size, 2048);
}

// =============================================================================
// List Operations
// =============================================================================

#[tokio::test]
async fn test_list_empty() {
    let service = make_test_service();

    let mut stream = service.list();
    let first = stream.next().await;

    assert!(first.is_none());
}

#[tokio::test]
async fn test_list_multiple() {
    let service = make_test_service();

    // Put several path infos
    let hashes = [
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "cccccccccccccccccccccccccccccccc",
    ];

    for (i, hash) in hashes.iter().enumerate() {
        let path_info = make_path_info(hash, &format!("pkg-{}", i), (i * 1000) as u64, &[]);
        service.put(path_info).await.unwrap();
    }

    // List all
    let mut stream = service.list();
    let mut count = 0;
    while let Some(result) = stream.next().await {
        result.unwrap();
        count += 1;
    }

    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_list_pagination() {
    let service = make_test_service();

    // Put many path infos to trigger pagination
    for i in 0..150 {
        let hash = make_nix_hash(i);
        let path_info = make_path_info(&hash, &format!("pkg-{}", i), i as u64, &[]);
        service.put(path_info).await.unwrap();
    }

    // List all - should paginate internally
    let mut stream = service.list();
    let mut count = 0;
    while let Some(result) = stream.next().await {
        result.unwrap();
        count += 1;
    }

    assert_eq!(count, 150);
}

// =============================================================================
// Node Types
// =============================================================================

#[tokio::test]
async fn test_file_node() {
    let service = make_test_service();

    let store_path = StorePath::from_bytes(b"0c6kzph7l0dcbfmjap64f0czdafn3b7x-file").unwrap();
    let path_info = PathInfo {
        store_path,
        node: Node::File {
            digest: B3Digest::from(&[1u8; 32]),
            size: 1024,
            executable: true,
        },
        references: vec![],
        nar_size: 1024,
        nar_sha256: [0u8; 32],
        signatures: vec![],
        deriver: None,
        ca: None,
    };

    let digest = *path_info.store_path.digest();
    service.put(path_info).await.unwrap();

    let retrieved = service.get(digest).await.unwrap().unwrap();
    match retrieved.node {
        Node::File { executable, .. } => assert!(executable),
        _ => panic!("expected file node"),
    }
}

#[tokio::test]
async fn test_directory_node() {
    let service = make_test_service();

    let store_path = StorePath::from_bytes(b"0c6kzph7l0dcbfmjap64f0czdafn3b7x-dir").unwrap();
    let path_info = PathInfo {
        store_path,
        node: Node::Directory {
            digest: B3Digest::from(&[2u8; 32]),
            size: 4096,
        },
        references: vec![],
        nar_size: 4096,
        nar_sha256: [0u8; 32],
        signatures: vec![],
        deriver: None,
        ca: None,
    };

    let digest = *path_info.store_path.digest();
    service.put(path_info).await.unwrap();

    let retrieved = service.get(digest).await.unwrap().unwrap();
    assert!(matches!(retrieved.node, Node::Directory { .. }));
}

#[tokio::test]
async fn test_symlink_node() {
    let service = make_test_service();

    let store_path = StorePath::from_bytes(b"0c6kzph7l0dcbfmjap64f0czdafn3b7x-link").unwrap();
    let path_info = PathInfo {
        store_path,
        node: Node::Symlink {
            target: "/nix/store/target".try_into().unwrap(),
        },
        references: vec![],
        nar_size: 32,
        nar_sha256: [0u8; 32],
        signatures: vec![],
        deriver: None,
        ca: None,
    };

    let digest = *path_info.store_path.digest();
    service.put(path_info).await.unwrap();

    let retrieved = service.get(digest).await.unwrap().unwrap();
    match retrieved.node {
        Node::Symlink { target } => assert_eq!(target.as_ref(), b"/nix/store/target"),
        _ => panic!("expected symlink node"),
    }
}

// =============================================================================
// Concurrent Access
// =============================================================================

#[tokio::test]
async fn test_concurrent_puts() {
    let service = Arc::new(make_test_service());

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let svc = Arc::clone(&service);
            let hash = make_nix_hash(i);
            tokio::spawn(async move {
                let path_info = make_path_info(&hash, &format!("pkg-{}", i), (i * 100) as u64, &[]);
                svc.put(path_info).await.unwrap()
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all were stored
    let mut stream = service.list();
    let mut count = 0;
    while let Some(result) = stream.next().await {
        result.unwrap();
        count += 1;
    }
    assert_eq!(count, 10);
}

#[tokio::test]
async fn test_concurrent_get_put() {
    let service = Arc::new(make_test_service());

    // Pre-populate some data
    let path_info = simple_path_info();
    let digest = *path_info.store_path.digest();
    service.put(path_info).await.unwrap();

    // Concurrent reads and writes
    let handles: Vec<_> = (0..20)
        .map(|i| {
            let svc = Arc::clone(&service);
            let hash = make_nix_hash(i + 1000); // Offset to avoid collision with simple_path_info
            if i % 2 == 0 {
                // Reader
                tokio::spawn(async move { svc.get(digest).await.unwrap() })
            } else {
                // Writer (new path info)
                tokio::spawn(async move {
                    let pi = make_path_info(&hash, &format!("pkg-{}", i), i as u64, &[]);
                    svc.put(pi).await.unwrap();
                    None
                })
            }
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    #[test]
    fn prop_pathinfo_roundtrip(nar_size in 1..1_000_000u64, ref_count in 0..5usize) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = make_test_service();

            let refs: Vec<&str> = (0..ref_count)
                .map(|i| match i {
                    0 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-dep0",
                    1 => "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-dep1",
                    2 => "cccccccccccccccccccccccccccccccc-dep2",
                    3 => "dddddddddddddddddddddddddddddddd-dep3",
                    _ => "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee-dep4",
                })
                .collect();

            let path_info = make_path_info(
                "0c6kzph7l0dcbfmjap64f0czdafn3b7x",
                "test-pkg",
                nar_size,
                &refs,
            );

            let digest = *path_info.store_path.digest();
            service.put(path_info.clone()).await.unwrap();

            let retrieved = service.get(digest).await.unwrap().unwrap();
            prop_assert_eq!(retrieved.nar_size, nar_size);
            prop_assert_eq!(retrieved.references.len(), ref_count);
            Ok(())
        }).unwrap();
    }

    #[test]
    fn prop_different_paths_different_digests(index1 in 0usize..100, index2 in 100usize..200) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let path1 = make_path_info(
                &make_nix_hash(index1),
                "pkg1",
                1024,
                &[],
            );
            let path2 = make_path_info(
                &make_nix_hash(index2),
                "pkg2",
                1024,
                &[],
            );

            let digest1 = path1.store_path.digest();
            let digest2 = path2.store_path.digest();

            prop_assert_ne!(digest1, digest2);
            Ok(())
        }).unwrap();
    }
}

// =============================================================================
// Parameterized Tests (rstest)
// =============================================================================

#[rstest]
#[case::small(100)]
#[case::medium(10_000)]
#[case::large(1_000_000)]
#[tokio::test]
async fn test_various_nar_sizes(#[case] nar_size: u64) {
    let service = make_test_service();
    let path_info = make_path_info("0c6kzph7l0dcbfmjap64f0czdafn3b7x", "test", nar_size, &[]);
    let digest = *path_info.store_path.digest();

    service.put(path_info).await.unwrap();
    let retrieved = service.get(digest).await.unwrap().unwrap();

    assert_eq!(retrieved.nar_size, nar_size);
}
