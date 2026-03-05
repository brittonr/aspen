//! NAR round-trip integration test.
//!
//! Tests the full pipeline: NAR ingestion → BlobService + DirectoryService + PathInfoService →
//! readback. Uses IrohBlobService<InMemoryBlobStore> + RaftDirectoryService + RaftPathInfoService
//! backed by DeterministicKeyValueStore.

use std::io::Cursor;
use std::sync::Arc;

use aspen_snix::IrohBlobService;
use aspen_snix::RaftDirectoryService;
use aspen_snix::RaftPathInfoService;
use aspen_testing::DeterministicKeyValueStore;
use n0_future::StreamExt;
use nix_compat::store_path::StorePath;
use snix_castore::blobservice::BlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_store::nar::ingest_nar_and_hash;
use snix_store::pathinfoservice::PathInfo;
use snix_store::pathinfoservice::PathInfoService;

/// Create all three snix services backed by in-memory stores.
fn make_services() -> (
    Arc<IrohBlobService<aspen_blob::InMemoryBlobStore>>,
    Arc<RaftDirectoryService<dyn aspen_core::KeyValueStore>>,
    Arc<RaftPathInfoService<dyn aspen_core::KeyValueStore>>,
) {
    let blob_store = aspen_blob::InMemoryBlobStore::new();
    let blob_svc = Arc::new(IrohBlobService::new(blob_store));

    let kv = Arc::new(DeterministicKeyValueStore::new()) as Arc<dyn aspen_core::KeyValueStore>;
    let dir_svc = Arc::new(RaftDirectoryService::from_arc(kv.clone()));
    let pathinfo_svc = Arc::new(RaftPathInfoService::from_arc(kv));

    (blob_svc, dir_svc, pathinfo_svc)
}

// =============================================================================
// NAR ingestion tests
// =============================================================================

#[tokio::test]
async fn nar_ingest_helloworld() {
    let (blob_svc, dir_svc, pathinfo_svc) = make_services();

    // 1. Ingest the standard Hello World NAR
    let nar_data = snix_store::fixtures::NAR_CONTENTS_HELLOWORLD;
    let mut nar_reader = Cursor::new(&nar_data);
    let (root_node, nar_sha256, nar_size) =
        ingest_nar_and_hash(blob_svc.clone(), dir_svc.clone(), &mut nar_reader, &None)
            .await
            .expect("NAR ingestion should succeed");

    // Verify NAR metadata
    assert_eq!(nar_size, nar_data.len() as u64, "NAR size should match");
    assert!(!nar_sha256.iter().all(|&b| b == 0), "SHA256 should be non-zero");

    // 2. Root should be a File node (Hello World is a single file)
    match &root_node {
        snix_castore::Node::File { digest, size, .. } => {
            assert!(blob_svc.has(digest).await.unwrap(), "blob should exist for file");
            assert!(*size > 0, "file should have non-zero size");
        }
        other => panic!("expected File root node for helloworld NAR, got {other:?}"),
    }

    // 3. Store PathInfo and verify round-trip
    let store_path =
        StorePath::<String>::from_bytes(b"00bgd045z0d4icpbc2yyz4gx48ak44la-net-tools-2.10").expect("valid store path");

    let pathinfo = PathInfo {
        store_path: store_path.clone(),
        node: root_node,
        references: vec![],
        nar_sha256,
        nar_size,
        signatures: vec![],
        deriver: None,
        ca: None,
    };

    pathinfo_svc.put(pathinfo).await.expect("put PathInfo should succeed");

    let retrieved = pathinfo_svc
        .get(*store_path.digest())
        .await
        .expect("get should not error")
        .expect("PathInfo should exist");

    assert_eq!(retrieved.store_path, store_path);
    assert_eq!(retrieved.nar_size, nar_size);
    assert_eq!(retrieved.nar_sha256, nar_sha256);
}

#[tokio::test]
async fn nar_ingest_complicated() {
    let (blob_svc, dir_svc, _pathinfo_svc) = make_services();

    // The "complicated" NAR contains a directory tree with files and subdirectories
    let nar_data = snix_store::fixtures::NAR_CONTENTS_COMPLICATED.to_vec();
    let mut nar_reader = Cursor::new(&nar_data);

    let (root_node, _nar_sha256, nar_size) =
        ingest_nar_and_hash(blob_svc.clone(), dir_svc.clone(), &mut nar_reader, &None)
            .await
            .expect("complicated NAR ingestion should succeed");

    assert_eq!(nar_size, nar_data.len() as u64);

    // Root should be a directory
    match &root_node {
        snix_castore::Node::Directory { digest, .. } => {
            let dir = dir_svc.get(digest).await.unwrap().expect("root directory should exist in store");

            // Should have child entries
            let nodes: Vec<_> = dir.nodes().collect();
            assert!(!nodes.is_empty(), "complicated dir should have children");
        }
        other => panic!("expected Directory root node for complicated NAR, got {other:?}"),
    }
}

#[tokio::test]
async fn nar_ingest_symlink() {
    let (blob_svc, dir_svc, _pathinfo_svc) = make_services();

    let nar_data = snix_store::fixtures::NAR_CONTENTS_SYMLINK.to_vec();
    let mut nar_reader = Cursor::new(&nar_data);

    let (root_node, _nar_sha256, nar_size) =
        ingest_nar_and_hash(blob_svc.clone(), dir_svc.clone(), &mut nar_reader, &None)
            .await
            .expect("symlink NAR ingestion should succeed");

    assert_eq!(nar_size, nar_data.len() as u64);

    // Root should be a symlink
    match &root_node {
        snix_castore::Node::Symlink { target } => {
            assert!(!target.as_ref().is_empty(), "symlink target should be non-empty");
        }
        other => panic!("expected Symlink root node, got {other:?}"),
    }
}

// =============================================================================
// PathInfo list test
// =============================================================================

#[tokio::test]
async fn pathinfo_list_after_multiple_puts() {
    let (_blob_svc, _dir_svc, pathinfo_svc) = make_services();

    // Store multiple PathInfo entries with symlink nodes (simple, no blob/dir needed)
    let paths = [
        "00bgd045z0d4icpbc2yyz4gx48ak44la-net-tools-2.10",
        "0c0bfm8bkmjsg3x0rg2mfi7c03w27gx3-hello-2.12.1",
        "7zzn8kcnaxs3jwh76yvmsqx87fry4ddx-pkg-1.0",
    ];

    for path_str in &paths {
        let store_path = StorePath::<String>::from_bytes(path_str.as_bytes()).expect("valid store path");

        let pathinfo = PathInfo {
            store_path,
            node: snix_castore::Node::Symlink {
                target: "/nix/store/dummy".try_into().unwrap(),
            },
            references: vec![],
            nar_sha256: [0u8; 32],
            nar_size: 100,
            signatures: vec![],
            deriver: None,
            ca: None,
        };

        pathinfo_svc.put(pathinfo).await.expect("put should succeed");
    }

    // List all and verify count
    let listed: Vec<PathInfo> = pathinfo_svc
        .list()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .expect("list should succeed");

    assert_eq!(listed.len(), paths.len(), "should list all stored paths");
}

// =============================================================================
// Idempotent ingestion
// =============================================================================

#[tokio::test]
async fn nar_ingest_idempotent() {
    let (blob_svc, dir_svc, _pathinfo_svc) = make_services();

    let nar_data = snix_store::fixtures::NAR_CONTENTS_HELLOWORLD;

    // Ingest twice
    let mut reader1 = Cursor::new(&nar_data);
    let (node1, sha1, size1) =
        ingest_nar_and_hash(blob_svc.clone(), dir_svc.clone(), &mut reader1, &None).await.unwrap();

    let mut reader2 = Cursor::new(&nar_data);
    let (node2, sha2, size2) =
        ingest_nar_and_hash(blob_svc.clone(), dir_svc.clone(), &mut reader2, &None).await.unwrap();

    // Should produce identical results
    assert_eq!(size1, size2);
    assert_eq!(sha1, sha2);
    assert_eq!(format!("{node1:?}"), format!("{node2:?}"));
}
