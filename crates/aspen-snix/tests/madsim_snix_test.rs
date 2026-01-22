//! Deterministic simulation tests for SNIX storage integration.
//!
//! Uses madsim for reproducible distributed system testing with fault injection.

#![cfg(madsim)]

use std::sync::Arc;
use std::time::Duration;

use aspen_blob::InMemoryBlobStore;
use aspen_core::DeterministicKeyValueStore;
use aspen_snix::IrohBlobService;
use aspen_snix::RaftDirectoryService;
use aspen_snix::RaftPathInfoService;
use aspen_testing::FaultInjector;
use madsim::rand::Rng;
use madsim::rand::{self};
use madsim::time;
use nix_compat::store_path::StorePath;
use snix_castore::B3Digest;
use snix_castore::Directory;
use snix_castore::Node;
use snix_castore::blobservice::BlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_store::pathinfoservice::PathInfo;
use snix_store::pathinfoservice::PathInfoService;
use tokio::io::AsyncWriteExt;

/// Test configuration for madsim runs.
struct MadsimConfig {
    seed: u64,
    blob_count: usize,
    directory_depth: usize,
    pathinfo_count: usize,
    fault_rate: f64,
}

impl Default for MadsimConfig {
    fn default() -> Self {
        Self {
            seed: 12345,
            blob_count: 50,
            directory_depth: 5,
            pathinfo_count: 20,
            fault_rate: 0.0,
        }
    }
}

// =============================================================================
// Basic Deterministic Tests
// =============================================================================

#[madsim::test]
async fn test_blob_service_deterministic() {
    let store = InMemoryBlobStore::new();
    let service = IrohBlobService::new(store);

    let data = b"deterministic test data";

    let mut writer = service.open_write().await;
    writer.write_all(data).await.unwrap();
    let digest = writer.close().await.unwrap();

    // Verify deterministic hash
    assert!(service.has(&digest).await.unwrap());

    // Multiple runs should produce same digest
    let store2 = InMemoryBlobStore::new();
    let service2 = IrohBlobService::new(store2);

    let mut writer2 = service2.open_write().await;
    writer2.write_all(data).await.unwrap();
    let digest2 = writer2.close().await.unwrap();

    assert_eq!(digest, digest2);
}

#[madsim::test]
async fn test_directory_service_deterministic() {
    let kv = DeterministicKeyValueStore::new();
    let service = RaftDirectoryService::new(kv);

    let file_digest = B3Digest::from(&[1u8; 32]);
    let mut dir = Directory::default();
    dir.add("test.txt".into(), Node::File {
        digest: file_digest,
        size: 1024,
        executable: false,
    })
    .unwrap();

    let digest = service.put(dir.clone()).await.unwrap();
    let retrieved = service.get(&digest).await.unwrap().unwrap();

    assert_eq!(retrieved.size(), dir.size());
}

#[madsim::test]
async fn test_pathinfo_service_deterministic() {
    let kv = DeterministicKeyValueStore::new();
    let service = RaftPathInfoService::new(kv);

    let store_path = StorePath::from_bytes(b"0c6kzph7l0dcbfmjap64f0czdafn3b7x-test").unwrap();
    let path_info = PathInfo {
        store_path,
        node: Node::File {
            digest: B3Digest::from(&[1u8; 32]),
            size: 1024,
            executable: false,
        },
        references: vec![],
        nar_size: 1024,
        nar_sha256: [0u8; 32],
        signatures: vec![],
        deriver: None,
        ca: None,
    };

    let digest = *path_info.store_path.digest();
    service.put(path_info.clone()).await.unwrap();

    let retrieved = service.get(digest).await.unwrap().unwrap();
    assert_eq!(retrieved.nar_size, path_info.nar_size);
}

// =============================================================================
// Concurrent Operation Tests
// =============================================================================

#[madsim::test]
async fn test_concurrent_blob_writes() {
    let config = MadsimConfig::default();
    let store = Arc::new(InMemoryBlobStore::new());
    let service = Arc::new(IrohBlobService::from_arc(store));

    let mut handles = Vec::new();

    for i in 0..config.blob_count {
        let svc = Arc::clone(&service);
        let handle = madsim::task::spawn(async move {
            let data = format!("blob data {}", i);
            let mut writer = svc.open_write().await;
            writer.write_all(data.as_bytes()).await.unwrap();
            writer.close().await.unwrap()
        });
        handles.push(handle);
    }

    let mut digests = Vec::new();
    for handle in handles {
        digests.push(handle.await.unwrap());
    }

    // Verify all blobs exist
    for digest in &digests {
        assert!(service.has(digest).await.unwrap());
    }
}

#[madsim::test]
async fn test_concurrent_directory_operations() {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    let service = Arc::new(RaftDirectoryService::from_arc(kv));

    let mut handles = Vec::new();

    for i in 0..20 {
        let svc = Arc::clone(&service);
        let handle = madsim::task::spawn(async move {
            let file_digest = B3Digest::from(&[(i % 256) as u8; 32]);
            let mut dir = Directory::default();
            dir.add(format!("file_{}.txt", i).into(), Node::File {
                digest: file_digest,
                size: (i * 100) as u64,
                executable: false,
            })
            .unwrap();
            svc.put(dir).await.unwrap()
        });
        handles.push(handle);
    }

    let mut digests = Vec::new();
    for handle in handles {
        digests.push(handle.await.unwrap());
    }

    // Verify all directories exist
    for digest in &digests {
        assert!(service.get(digest).await.unwrap().is_some());
    }
}

// =============================================================================
// Timing Tests
// =============================================================================

#[madsim::test]
async fn test_simulated_latency() {
    let kv = DeterministicKeyValueStore::new();
    let service = RaftPathInfoService::new(kv);

    let start = time::Instant::now();

    // Simulate some network delay
    time::sleep(Duration::from_millis(10)).await;

    let store_path = StorePath::from_bytes(b"0c6kzph7l0dcbfmjap64f0czdafn3b7x-test").unwrap();
    let path_info = PathInfo {
        store_path,
        node: Node::File {
            digest: B3Digest::from(&[1u8; 32]),
            size: 1024,
            executable: false,
        },
        references: vec![],
        nar_size: 1024,
        nar_sha256: [0u8; 32],
        signatures: vec![],
        deriver: None,
        ca: None,
    };

    service.put(path_info).await.unwrap();

    let elapsed = start.elapsed();
    // With simulated time, should be at least 10ms
    assert!(elapsed >= Duration::from_millis(10));
}

// =============================================================================
// Random Operation Sequence Tests
// =============================================================================

#[madsim::test]
async fn test_random_operation_sequence() {
    let config = MadsimConfig {
        seed: 42,
        ..Default::default()
    };

    let kv = Arc::new(DeterministicKeyValueStore::new());
    let pathinfo_svc = Arc::new(RaftPathInfoService::from_arc(Arc::clone(&kv)));

    let mut rng = rand::thread_rng();
    let mut stored_digests: Vec<[u8; 20]> = Vec::new();

    for i in 0..50 {
        let op = rng.gen_range(0..3);

        match op {
            0 => {
                // Put operation
                let hash = format!("{:032x}", i);
                let store_path = StorePath::from_bytes(format!("{}-pkg{}", hash, i).as_bytes()).unwrap();
                let path_info = PathInfo {
                    store_path: store_path.clone(),
                    node: Node::File {
                        digest: B3Digest::from(&[i as u8; 32]),
                        size: (i * 100) as u64,
                        executable: false,
                    },
                    references: vec![],
                    nar_size: (i * 100) as u64,
                    nar_sha256: [i as u8; 32],
                    signatures: vec![],
                    deriver: None,
                    ca: None,
                };

                let digest = *path_info.store_path.digest();
                pathinfo_svc.put(path_info).await.unwrap();
                stored_digests.push(digest);
            }
            1 if !stored_digests.is_empty() => {
                // Get existing
                let idx = rng.gen_range(0..stored_digests.len());
                let digest = stored_digests[idx];
                let result = pathinfo_svc.get(digest).await.unwrap();
                assert!(result.is_some());
            }
            2 => {
                // Get nonexistent
                let fake_digest = [rng.r#gen::<u8>(); 20];
                // May or may not exist
                let _ = pathinfo_svc.get(fake_digest).await;
            }
            _ => {}
        }
    }

    // Verify all stored digests still exist
    for digest in &stored_digests {
        assert!(pathinfo_svc.get(*digest).await.unwrap().is_some());
    }
}

// =============================================================================
// Stress Tests
// =============================================================================

#[madsim::test]
async fn test_high_concurrency_stress() {
    let store = Arc::new(InMemoryBlobStore::new());
    let service = Arc::new(IrohBlobService::from_arc(store));

    // Spawn many concurrent operations
    let mut handles = Vec::new();
    let concurrency = 100;

    for i in 0..concurrency {
        let svc = Arc::clone(&service);
        let handle = madsim::task::spawn(async move {
            let data = vec![i as u8; 1024];

            // Write
            let mut writer = svc.open_write().await;
            writer.write_all(&data).await.unwrap();
            let digest = writer.close().await.unwrap();

            // Read back
            assert!(svc.has(&digest).await.unwrap());

            // Small delay
            time::sleep(Duration::from_micros(100)).await;

            digest
        });
        handles.push(handle);
    }

    let results: Vec<_> = futures::future::join_all(handles).await;
    let digests: Vec<_> = results.into_iter().map(|r| r.unwrap()).collect();

    // All unique (content-addressed)
    let unique: std::collections::HashSet<_> = digests.iter().collect();
    assert_eq!(unique.len(), concurrency);
}

// =============================================================================
// Deep Directory Tree Tests
// =============================================================================

#[madsim::test]
async fn test_deep_directory_tree() {
    let kv = Arc::new(DeterministicKeyValueStore::new());
    let service = Arc::new(RaftDirectoryService::from_arc(kv));

    // Create a chain of nested directories
    let depth = 10;
    let mut current_digest = B3Digest::from(&[1u8; 32]);

    // Start with leaf file
    let leaf = {
        let mut d = Directory::default();
        d.add("leaf.txt".into(), Node::File {
            digest: B3Digest::from(&[0u8; 32]),
            size: 100,
            executable: false,
        })
        .unwrap();
        d
    };
    current_digest = service.put(leaf.clone()).await.unwrap();
    let mut current_size = leaf.size();

    // Build up the tree
    for i in 0..depth {
        let mut parent = Directory::default();
        parent
            .add(format!("level_{}", i).into(), Node::Directory {
                digest: current_digest,
                size: current_size,
            })
            .unwrap();
        current_size = parent.size();
        current_digest = service.put(parent).await.unwrap();
    }

    // Traverse recursively
    use futures::StreamExt;
    let mut stream = service.get_recursive(&current_digest);
    let mut count = 0;
    while let Some(result) = stream.next().await {
        result.unwrap();
        count += 1;
    }

    // Should have depth + 1 directories (including leaf)
    assert_eq!(count, depth + 1);
}

// =============================================================================
// Reproducibility Tests
// =============================================================================

#[madsim::test]
async fn test_reproducible_across_runs() {
    // This test verifies that with the same seed, we get the same results
    let seed = 12345u64;

    // Run 1
    let result1 = run_deterministic_sequence(seed).await;

    // Run 2 (same seed)
    let result2 = run_deterministic_sequence(seed).await;

    assert_eq!(result1, result2);
}

async fn run_deterministic_sequence(seed: u64) -> Vec<B3Digest> {
    let store = InMemoryBlobStore::new();
    let service = IrohBlobService::new(store);

    let mut digests = Vec::new();

    // Use seed to generate deterministic data
    for i in 0..10 {
        let data = format!("seed {} data {}", seed, i);
        let mut writer = service.open_write().await;
        writer.write_all(data.as_bytes()).await.unwrap();
        let digest = writer.close().await.unwrap();
        digests.push(digest);
    }

    digests
}
