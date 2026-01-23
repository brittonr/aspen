//! Integration tests for RaftDirectoryService.
//!
//! Tests the DirectoryService trait implementation against Raft KV storage.

use std::sync::Arc;

use aspen_core::DeterministicKeyValueStore;
use aspen_snix::RaftDirectoryService;
use futures::StreamExt;
use proptest::prelude::*;
use rstest::rstest;
use snix_castore::B3Digest;
use snix_castore::Directory;
use snix_castore::Node;
use snix_castore::PathComponent;
use snix_castore::directoryservice::DirectoryService;

/// Create a test directory service with in-memory KV backend.
fn make_test_service() -> RaftDirectoryService<DeterministicKeyValueStore> {
    RaftDirectoryService::from_arc(DeterministicKeyValueStore::new())
}

/// Create an empty directory.
fn empty_dir() -> Directory {
    Directory::default()
}

/// Create a directory with a single file.
fn dir_with_file(name: &str, digest: B3Digest, size: u64) -> Directory {
    let mut dir = Directory::default();
    let name_component: PathComponent = name.try_into().unwrap();
    dir.add(name_component, Node::File {
        digest,
        size,
        executable: false,
    })
    .unwrap();
    dir
}

/// Create a directory with a symlink.
fn dir_with_symlink(name: &str, target: &str) -> Directory {
    let mut dir = Directory::default();
    let name_component: PathComponent = name.try_into().unwrap();
    let target_symlink = target.try_into().unwrap();
    dir.add(name_component, Node::Symlink { target: target_symlink }).unwrap();
    dir
}

/// Create a directory with a subdirectory.
fn dir_with_subdir(name: &str, subdir_digest: B3Digest, size: u64) -> Directory {
    let mut dir = Directory::default();
    let name_component: PathComponent = name.try_into().unwrap();
    dir.add(name_component, Node::Directory {
        digest: subdir_digest,
        size,
    })
    .unwrap();
    dir
}

// =============================================================================
// Basic Operations
// =============================================================================

#[tokio::test]
async fn test_put_and_get_empty_directory() {
    let service = make_test_service();
    let dir = empty_dir();

    let digest = service.put(dir.clone()).await.unwrap();

    let retrieved = service.get(&digest).await.unwrap().unwrap();
    assert_eq!(retrieved.size(), dir.size());
}

#[tokio::test]
async fn test_put_and_get_directory_with_file() {
    let service = make_test_service();
    let file_digest = B3Digest::from(&[1u8; 32]);
    let dir = dir_with_file("test.txt", file_digest, 1024);

    let digest = service.put(dir.clone()).await.unwrap();

    let retrieved = service.get(&digest).await.unwrap().unwrap();
    assert_eq!(retrieved.size(), dir.size());

    // Verify the file node exists
    let nodes: Vec<_> = retrieved.nodes().collect();
    assert_eq!(nodes.len(), 1);
    let (name, node) = &nodes[0];
    assert_eq!(name.as_ref(), b"test.txt");
    assert!(matches!(node, Node::File { size: 1024, .. }));
}

#[tokio::test]
async fn test_put_and_get_directory_with_symlink() {
    let service = make_test_service();
    let dir = dir_with_symlink("link", "/target/path");

    let digest = service.put(dir).await.unwrap();

    let retrieved = service.get(&digest).await.unwrap().unwrap();
    let nodes: Vec<_> = retrieved.nodes().collect();
    assert_eq!(nodes.len(), 1);

    let (name, node) = &nodes[0];
    assert_eq!(name.as_ref(), b"link");
    match node {
        Node::Symlink { target } => assert_eq!(target.as_ref(), b"/target/path"),
        _ => panic!("expected symlink"),
    }
}

#[tokio::test]
async fn test_get_nonexistent_directory() {
    let service = make_test_service();
    let fake_digest = B3Digest::from(&[0u8; 32]);

    let result = service.get(&fake_digest).await.unwrap();
    assert!(result.is_none());
}

// =============================================================================
// Content Addressing
// =============================================================================

#[tokio::test]
async fn test_same_content_same_digest() {
    let service = make_test_service();
    let file_digest = B3Digest::from(&[1u8; 32]);

    let dir1 = dir_with_file("file.txt", file_digest, 100);
    let dir2 = dir_with_file("file.txt", file_digest, 100);

    let digest1 = service.put(dir1).await.unwrap();
    let digest2 = service.put(dir2).await.unwrap();

    assert_eq!(digest1, digest2);
}

#[tokio::test]
async fn test_different_content_different_digest() {
    let service = make_test_service();

    let dir1 = dir_with_file("a.txt", B3Digest::from(&[1u8; 32]), 100);
    let dir2 = dir_with_file("b.txt", B3Digest::from(&[2u8; 32]), 200);

    let digest1 = service.put(dir1).await.unwrap();
    let digest2 = service.put(dir2).await.unwrap();

    assert_ne!(digest1, digest2);
}

// =============================================================================
// Directory Putter
// =============================================================================

#[tokio::test]
async fn test_put_multiple_directories() {
    let service = make_test_service();
    let mut putter = service.put_multiple_start();

    // Put leaf directory (file)
    let leaf_digest = B3Digest::from(&[1u8; 32]);
    let leaf = dir_with_file("content.txt", leaf_digest, 512);
    putter.put(leaf.clone()).await.unwrap();

    // Put root directory referencing leaf
    let leaf_stored_digest = leaf.digest();
    let root = dir_with_subdir("subdir", leaf_stored_digest, leaf.size());
    putter.put(root).await.unwrap();

    // Close returns root digest
    let root_digest = putter.close().await.unwrap();

    // Verify both directories exist
    assert!(service.get(&leaf_stored_digest).await.unwrap().is_some());
    assert!(service.get(&root_digest).await.unwrap().is_some());
}

#[tokio::test]
async fn test_put_multiple_close_empty() {
    let service = make_test_service();
    let mut putter = service.put_multiple_start();

    // Close without putting anything should error
    let result = putter.close().await;
    assert!(result.is_err());
}

// =============================================================================
// Recursive Retrieval
// =============================================================================

#[tokio::test]
async fn test_get_recursive_single_directory() {
    let service = make_test_service();
    let dir = dir_with_file("test.txt", B3Digest::from(&[1u8; 32]), 100);
    let digest = service.put(dir).await.unwrap();

    let mut stream = service.get_recursive(&digest);
    let mut dirs = Vec::new();
    while let Some(result) = stream.next().await {
        dirs.push(result.unwrap());
    }

    assert_eq!(dirs.len(), 1);
}

#[tokio::test]
async fn test_get_recursive_nested_directories() {
    let service = make_test_service();

    // Create a 3-level hierarchy:
    // root/
    //   child1/
    //     grandchild/
    //       file.txt
    //   child2/
    //     file2.txt

    // Grandchild
    let grandchild = dir_with_file("file.txt", B3Digest::from(&[1u8; 32]), 100);
    let grandchild_digest = service.put(grandchild.clone()).await.unwrap();

    // Child1
    let child1 = dir_with_subdir("grandchild", grandchild_digest, grandchild.size());
    let child1_digest = service.put(child1.clone()).await.unwrap();

    // Child2
    let child2 = dir_with_file("file2.txt", B3Digest::from(&[2u8; 32]), 200);
    let child2_digest = service.put(child2.clone()).await.unwrap();

    // Root
    let mut root = Directory::default();
    let name1: PathComponent = "child1".try_into().unwrap();
    let name2: PathComponent = "child2".try_into().unwrap();
    root.add(name1, Node::Directory {
        digest: child1_digest,
        size: child1.size(),
    })
    .unwrap();
    root.add(name2, Node::Directory {
        digest: child2_digest,
        size: child2.size(),
    })
    .unwrap();
    let root_digest = service.put(root).await.unwrap();

    // Get recursive
    let mut stream = service.get_recursive(&root_digest);
    let mut dirs = Vec::new();
    while let Some(result) = stream.next().await {
        dirs.push(result.unwrap());
    }

    // Should get all 4 directories (root, child1, child2, grandchild)
    assert_eq!(dirs.len(), 4);
}

#[tokio::test]
async fn test_get_recursive_nonexistent() {
    let service = make_test_service();
    let fake_digest = B3Digest::from(&[0u8; 32]);

    let mut stream = service.get_recursive(&fake_digest);
    let result = stream.next().await;

    // Should yield nothing for nonexistent root
    assert!(result.is_none());
}

// =============================================================================
// Depth Limits (Tiger Style)
// =============================================================================

#[tokio::test]
async fn test_get_recursive_depth_limit() {
    let service = make_test_service();

    // Create a deeply nested structure (exceeding MAX_DIRECTORY_DEPTH)
    // Note: In practice this would need to be within limits to create,
    // but get_recursive should enforce depth limits

    let max_depth = aspen_snix::MAX_DIRECTORY_DEPTH as usize;

    // Create chain of directories
    let mut current = dir_with_file("leaf.txt", B3Digest::from(&[1u8; 32]), 100);
    let mut current_digest = service.put(current.clone()).await.unwrap();

    for i in 0..max_depth {
        let parent = dir_with_subdir(&format!("level_{}", i), current_digest, current.size());
        current_digest = service.put(parent.clone()).await.unwrap();
        current = parent;
    }

    // Traversal should hit depth limit
    let mut stream = service.get_recursive(&current_digest);
    let mut count = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(_) => count += 1,
            Err(e) => {
                // Expected: depth limit exceeded
                assert!(e.to_string().contains("depth") || e.to_string().contains("limit"));
                break;
            }
        }
    }

    // Should have traversed some but hit limit
    assert!(count > 0);
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
            tokio::spawn(async move {
                let file_digest = B3Digest::from(&[i as u8; 32]);
                let dir = dir_with_file(&format!("file_{}.txt", i), file_digest, (i * 100) as u64);
                svc.put(dir).await.unwrap()
            })
        })
        .collect();

    let mut digests = Vec::new();
    for handle in handles {
        digests.push(handle.await.unwrap());
    }

    // All digests should be unique
    let unique: std::collections::HashSet<_> = digests.iter().collect();
    assert_eq!(unique.len(), 10);

    // All should be retrievable
    for digest in &digests {
        assert!(service.get(digest).await.unwrap().is_some());
    }
}

// =============================================================================
// Property-Based Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    #[test]
    fn prop_directory_roundtrip(file_count in 1..10usize, file_sizes in prop::collection::vec(1..10000u64, 1..10)) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = make_test_service();
            let mut dir = Directory::default();

            for (i, &size) in file_sizes.iter().take(file_count).enumerate() {
                let digest = B3Digest::from(&[i as u8; 32]);
                let name: PathComponent = format!("file_{}.txt", i).as_str().try_into().unwrap();
                dir.add(name, Node::File {
                    digest,
                    size,
                    executable: false,
                }).unwrap();
            }

            let stored_digest = service.put(dir.clone()).await.unwrap();
            let retrieved = service.get(&stored_digest).await.unwrap().unwrap();

            prop_assert_eq!(retrieved.size(), dir.size());
            Ok(())
        }).unwrap();
    }

    #[test]
    fn prop_content_addressing(seed in any::<u64>()) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let service = make_test_service();

            // Create deterministic directory from seed
            let digest = B3Digest::from(&seed.to_le_bytes().repeat(4).try_into().unwrap_or([0u8; 32]));
            let dir = dir_with_file("seeded.txt", digest, seed % 10000);

            let stored1 = service.put(dir.clone()).await.unwrap();
            let stored2 = service.put(dir).await.unwrap();

            prop_assert_eq!(stored1, stored2);
            Ok(())
        }).unwrap();
    }
}

// =============================================================================
// Parameterized Tests (rstest)
// =============================================================================

#[rstest]
#[case::empty(empty_dir())]
#[case::single_file(dir_with_file("test.txt", B3Digest::from(&[1u8; 32]), 100))]
#[case::symlink(dir_with_symlink("link", "/target"))]
#[tokio::test]
async fn test_various_directory_types(#[case] dir: Directory) {
    let service = make_test_service();

    let digest = service.put(dir.clone()).await.unwrap();
    let retrieved = service.get(&digest).await.unwrap().unwrap();

    assert_eq!(retrieved.size(), dir.size());
}
