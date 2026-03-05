//! NAR ingestion + rendering through our castore over irpc.
//!
//! Verifies the full Nix workflow:
//!   1. Ingest a NAR (binary) into the castore → blobs + directory tree
//!   2. Render it back out as a NAR → identical bytes
//!
//! This proves our irpc-backed BlobService and DirectoryService are
//! fully compatible with the snix NAR pipeline.
//!
//! Tests are `#[ignore]` because they bind to real sockets.
//!
//! ```sh
//! cargo nextest run -p aspen-castore --test nar_test --run-ignored all
//! ```

use std::io::Cursor;
use std::sync::Arc;

use aspen_castore::CASTORE_ALPN;
use aspen_castore::client::IrpcBlobService;
use aspen_castore::client::IrpcDirectoryService;
use aspen_castore::server::CastoreServer;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::protocol::Router;
use sha2::Digest;
use sha2::Sha256;
use snix_castore::Node;
use snix_castore::blobservice::BlobService;
use snix_castore::blobservice::MemoryBlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_castore::directoryservice::RedbDirectoryService;
use snix_castore::directoryservice::RedbDirectoryServiceConfig;
use snix_castore::fixtures::*;
use snix_store::nar::ingest_nar;
use snix_store::nar::write_nar;

// NAR fixtures from snix-store
// These are real NAR byte sequences that the Nix daemon would produce.

/// NAR for a symlink pointing to /nix/store/somewhereelse
const NAR_SYMLINK: [u8; 136] = [
    13, 0, 0, 0, 0, 0, 0, 0, b'n', b'i', b'x', b'-', b'a', b'r', b'c', b'h', b'i', b'v', b'e', b'-', b'1', 0, 0, 0, 1,
    0, 0, 0, 0, 0, 0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, b't', b'y', b'p', b'e', 0, 0, 0, 0, 7, 0,
    0, 0, 0, 0, 0, 0, b's', b'y', b'm', b'l', b'i', b'n', b'k', 0, 6, 0, 0, 0, 0, 0, 0, 0, b't', b'a', b'r', b'g',
    b'e', b't', 0, 0, 24, 0, 0, 0, 0, 0, 0, 0, b'/', b'n', b'i', b'x', b'/', b's', b't', b'o', b'r', b'e', b'/', b's',
    b'o', b'm', b'e', b'w', b'h', b'e', b'r', b'e', b'e', b'l', b's', b'e', 1, 0, 0, 0, 0, 0, 0, 0, b')', 0, 0, 0, 0,
    0, 0, 0,
];

/// NAR for a regular file containing "Hello World!"
const NAR_HELLOWORLD: [u8; 128] = [
    13, 0, 0, 0, 0, 0, 0, 0, b'n', b'i', b'x', b'-', b'a', b'r', b'c', b'h', b'i', b'v', b'e', b'-', b'1', 0, 0, 0, 1,
    0, 0, 0, 0, 0, 0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, b't', b'y', b'p', b'e', 0, 0, 0, 0, 7, 0,
    0, 0, 0, 0, 0, 0, b'r', b'e', b'g', b'u', b'l', b'a', b'r', 0, 8, 0, 0, 0, 0, 0, 0, 0, b'c', b'o', b'n', b't',
    b'e', b'n', b't', b's', 12, 0, 0, 0, 0, 0, 0, 0, b'H', b'e', b'l', b'l', b'o', b' ', b'W', b'o', b'r', b'l', b'd',
    b'!', 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b')', 0, 0, 0, 0, 0, 0, 0,
];

/// NAR for a complicated directory structure (directory with subdirs, files, symlinks).
const NAR_COMPLICATED: [u8; 840] = [
    13, 0, 0, 0, 0, 0, 0, 0, b'n', b'i', b'x', b'-', b'a', b'r', b'c', b'h', b'i', b'v', b'e', b'-', b'1', 0, 0, 0, 1,
    0, 0, 0, 0, 0, 0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, b't', b'y', b'p', b'e', 0, 0, 0, 0, 9, 0,
    0, 0, 0, 0, 0, 0, b'd', b'i', b'r', b'e', b'c', b't', b'o', b'r', b'y', 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0,
    0, b'e', b'n', b't', b'r', b'y', 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0,
    0, b'n', b'a', b'm', b'e', 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, b'.', b'k', b'e', b'e', b'p', 0, 0, 0, 4, 0, 0, 0,
    0, 0, 0, 0, b'n', b'o', b'd', b'e', 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0,
    0, 0, 0, b't', b'y', b'p', b'e', 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, b'r', b'e', b'g', b'u', b'l', b'a', b'r', 0,
    8, 0, 0, 0, 0, 0, 0, 0, b'c', b'o', b'n', b't', b'e', b'n', b't', b's', 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, b')', 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b')', 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, b'e',
    b'n', b't', b'r', b'y', 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, b'n',
    b'a', b'm', b'e', 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, b'a', b'a', 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, b'n',
    b'o', b'd', b'e', 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, b't',
    b'y', b'p', b'e', 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, b's', b'y', b'm', b'l', b'i', b'n', b'k', 0, 6, 0, 0, 0, 0,
    0, 0, 0, b't', b'a', b'r', b'g', b'e', b't', 0, 0, 24, 0, 0, 0, 0, 0, 0, 0, b'/', b'n', b'i', b'x', b'/', b's',
    b't', b'o', b'r', b'e', b'/', b's', b'o', b'm', b'e', b'w', b'h', b'e', b'r', b'e', b'e', b'l', b's', b'e', 1, 0,
    0, 0, 0, 0, 0, 0, b')', 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b')', 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0,
    0, 0, b'e', b'n', b't', b'r', b'y', 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0,
    0, 0, b'n', b'a', b'm', b'e', 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, b'k', b'e', b'e', b'p', 0, 0, 0, 0, 4, 0, 0, 0,
    0, 0, 0, 0, b'n', b'o', b'd', b'e', 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0,
    0, 0, 0, b't', b'y', b'p', b'e', 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, b'd', b'i', b'r', b'e', b'c', b't', b'o',
    b'r', b'y', 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, b'e', b'n', b't', b'r', b'y', 0, 0, 0, 1, 0, 0, 0, 0, 0,
    0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, b'n', b'a', b'm', b'e', 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0,
    0, b'.', b'k', b'e', b'e', b'p', 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, b'n', b'o', b'd', b'e', 0, 0, 0, 0, 1, 0, 0, 0,
    0, 0, 0, 0, b'(', 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, b't', b'y', b'p', b'e', 0, 0, 0, 0, 7, 0, 0, 0, 0,
    0, 0, 0, b'r', b'e', b'g', b'u', b'l', b'a', b'r', 0, 8, 0, 0, 0, 0, 0, 0, 0, b'c', b'o', b'n', b't', b'e', b'n',
    b't', b's', 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b')', 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
    b')', 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b')', 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b')', 0, 0,
    0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, b')', 0, 0, 0, 0, 0, 0, 0,
];

/// Spin up a server endpoint + router, return clients.
async fn setup() -> (Arc<IrpcBlobService>, Arc<IrpcDirectoryService>, Router, Endpoint) {
    let server_ep = Endpoint::builder().clear_discovery().bind().await.expect("bind server");

    let blob_svc = MemoryBlobService::default();
    let dir_svc = RedbDirectoryService::new_temporary("nar-test".to_string(), RedbDirectoryServiceConfig::default())
        .expect("redb temp");

    let server = CastoreServer::new(blob_svc, dir_svc);
    let handler = server.into_protocol_handler();

    let router = Router::builder(server_ep.clone()).accept(CASTORE_ALPN, handler).spawn();

    let mut addr = EndpointAddr::new(server_ep.id());
    for sock in server_ep.bound_sockets() {
        let mut fixed = sock;
        if fixed.ip().is_unspecified() {
            fixed.set_ip(std::net::Ipv4Addr::LOCALHOST.into());
        }
        addr.addrs.insert(iroh::TransportAddr::Ip(fixed));
    }

    let client_ep = Endpoint::builder().clear_discovery().bind().await.expect("bind client");

    let blob = Arc::new(IrpcBlobService::new(client_ep.clone(), addr.clone()));
    let dir = Arc::new(IrpcDirectoryService::new(client_ep, addr));

    (blob, dir, router, server_ep)
}

// ========================================================================
// NAR ingestion tests
// ========================================================================

#[tokio::test]
#[ignore]
async fn nar_ingest_symlink() {
    let (blob, dir, _router, _ep) = setup().await;

    let root_node = ingest_nar(blob.as_ref().clone(), dir.as_ref().clone(), &mut Cursor::new(&NAR_SYMLINK))
        .await
        .expect("must parse symlink NAR");

    assert_eq!(root_node, Node::Symlink {
        target: "/nix/store/somewhereelse".try_into().unwrap()
    });
}

#[tokio::test]
#[ignore]
async fn nar_ingest_file() {
    let (blob, dir, _router, _ep) = setup().await;

    let root_node = ingest_nar(blob.as_ref().clone(), dir.as_ref().clone(), &mut Cursor::new(&NAR_HELLOWORLD))
        .await
        .expect("must parse helloworld NAR");

    assert_eq!(root_node, Node::File {
        digest: *HELLOWORLD_BLOB_DIGEST,
        size: HELLOWORLD_BLOB_CONTENTS.len() as u64,
        executable: false,
    });

    // Blob must be in the store
    assert!(blob.has(&HELLOWORLD_BLOB_DIGEST).await.unwrap());
}

#[tokio::test]
#[ignore]
async fn nar_ingest_complicated_directory() {
    let (blob, dir, _router, _ep) = setup().await;

    let root_node = ingest_nar(blob.as_ref().clone(), dir.as_ref().clone(), &mut Cursor::new(&NAR_COMPLICATED))
        .await
        .expect("must parse complicated NAR");

    assert_eq!(root_node, Node::Directory {
        digest: DIRECTORY_COMPLICATED.digest(),
        size: DIRECTORY_COMPLICATED.size(),
    });

    // Blob for .keep files must exist
    assert!(blob.has(&EMPTY_BLOB_DIGEST).await.unwrap());

    // Both directories must be retrievable
    assert!(dir.get(&DIRECTORY_COMPLICATED.digest()).await.unwrap().is_some());
    assert!(dir.get(&DIRECTORY_WITH_KEEP.digest()).await.unwrap().is_some());
}

// ========================================================================
// NAR roundtrip: ingest → render → compare
// ========================================================================

#[tokio::test]
#[ignore]
async fn nar_roundtrip_symlink() {
    let (blob, dir, _router, _ep) = setup().await;

    let root_node = ingest_nar(blob.as_ref().clone(), dir.as_ref().clone(), &mut Cursor::new(&NAR_SYMLINK))
        .await
        .unwrap();

    let mut rendered = Vec::new();
    write_nar(&mut rendered, &root_node, blob.as_ref().clone(), dir.as_ref().clone()).await.unwrap();

    assert_eq!(rendered, NAR_SYMLINK, "NAR roundtrip must be identical");
}

#[tokio::test]
#[ignore]
async fn nar_roundtrip_file() {
    let (blob, dir, _router, _ep) = setup().await;

    let root_node = ingest_nar(blob.as_ref().clone(), dir.as_ref().clone(), &mut Cursor::new(&NAR_HELLOWORLD))
        .await
        .unwrap();

    let mut rendered = Vec::new();
    write_nar(&mut rendered, &root_node, blob.as_ref().clone(), dir.as_ref().clone()).await.unwrap();

    assert_eq!(rendered, NAR_HELLOWORLD, "NAR roundtrip must be identical");
}

#[tokio::test]
#[ignore]
async fn nar_roundtrip_complicated() {
    let (blob, dir, _router, _ep) = setup().await;

    let root_node = ingest_nar(blob.as_ref().clone(), dir.as_ref().clone(), &mut Cursor::new(&NAR_COMPLICATED))
        .await
        .unwrap();

    let mut rendered = Vec::new();
    write_nar(&mut rendered, &root_node, blob.as_ref().clone(), dir.as_ref().clone()).await.unwrap();

    assert_eq!(rendered, NAR_COMPLICATED, "NAR roundtrip must be identical");
}

/// Full pipeline: ingest NAR → verify SHA256 → render back → compare bytes.
#[tokio::test]
#[ignore]
async fn nar_full_pipeline_with_hash() {
    let (blob, dir, _router, _ep) = setup().await;

    // Hash the original NAR
    let original_hash: [u8; 32] = Sha256::digest(NAR_COMPLICATED).into();

    // Ingest
    let root_node = ingest_nar(blob.as_ref().clone(), dir.as_ref().clone(), &mut Cursor::new(&NAR_COMPLICATED))
        .await
        .unwrap();

    // Render
    let mut rendered = Vec::new();
    write_nar(&mut rendered, &root_node, blob.as_ref().clone(), dir.as_ref().clone()).await.unwrap();

    // Hash must match
    let rendered_hash: [u8; 32] = Sha256::digest(&rendered).into();
    assert_eq!(original_hash, rendered_hash, "SHA256 must match after roundtrip");
    assert_eq!(rendered.len(), NAR_COMPLICATED.len(), "size must match");
}
