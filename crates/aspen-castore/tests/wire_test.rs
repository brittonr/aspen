//! Wire-level integration tests for the castore irpc protocol.
//!
//! Two iroh endpoints communicate over real QUIC connections:
//! - Server endpoint: runs CastoreServer with MemoryBlobService + RedbDirectoryService
//! - Client endpoint: uses IrpcBlobService + IrpcDirectoryService
//!
//! Tests are `#[ignore]` because they bind to real sockets.
//!
//! ```sh
//! cargo nextest run -p aspen-castore --test wire_test --run-ignored all
//! ```

use aspen_castore::CASTORE_ALPN;
use aspen_castore::client::IrpcBlobService;
use aspen_castore::client::IrpcDirectoryService;
use aspen_castore::server::CastoreServer;
use futures::StreamExt;
use iroh::Endpoint;
use iroh::EndpointAddr;
use iroh::protocol::Router;
use snix_castore::B3Digest;
use snix_castore::blobservice::BlobService;
use snix_castore::blobservice::MemoryBlobService;
use snix_castore::directoryservice::DirectoryService;
use snix_castore::directoryservice::RedbDirectoryService;
use snix_castore::directoryservice::RedbDirectoryServiceConfig;
use snix_castore::fixtures::*;
use tokio::io::AsyncWriteExt;

/// Spin up a server endpoint + router, return the server's EndpointAddr
/// and the Router handle (must be kept alive).
async fn start_server() -> (EndpointAddr, Router, Endpoint) {
    let endpoint = Endpoint::builder(iroh::endpoint::presets::N0)
        .clear_address_lookup()
        .bind()
        .await
        .expect("bind server endpoint");

    let blob_svc = MemoryBlobService::default();
    let dir_svc = RedbDirectoryService::new_temporary("wire-test".to_string(), RedbDirectoryServiceConfig::default())
        .expect("redb temp dir service");

    let server = CastoreServer::new(blob_svc, dir_svc);
    let handler = server.into_protocol_handler();

    let router = Router::builder(endpoint.clone()).accept(CASTORE_ALPN, handler).spawn();

    // Build the EndpointAddr with actual socket addresses
    let mut addr = EndpointAddr::new(endpoint.id());
    for sock_addr in endpoint.bound_sockets() {
        let mut fixed = sock_addr;
        if fixed.ip().is_unspecified() {
            fixed.set_ip(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
        }
        addr.addrs.insert(iroh::TransportAddr::Ip(fixed));
    }

    (addr, router, endpoint)
}

/// Create a client endpoint and connect to the server.
fn make_clients(endpoint: Endpoint, server_addr: EndpointAddr) -> (IrpcBlobService, IrpcDirectoryService) {
    let blob = IrpcBlobService::new(endpoint.clone(), server_addr.clone());
    let dir = IrpcDirectoryService::new(endpoint, server_addr);
    (blob, dir)
}

// ========================================================================
// Blob tests
// ========================================================================

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn wire_blob_roundtrip() {
    let (addr, _router, _server_ep) = start_server().await;
    let client_ep = Endpoint::builder(iroh::endpoint::presets::N0).clear_address_lookup().bind().await.unwrap();
    let (blob, _dir) = make_clients(client_ep, addr);

    let data = b"hello from the wire";

    // Write
    let mut writer = blob.open_write().await;
    writer.write_all(data).await.unwrap();
    let digest = writer.close().await.unwrap();

    // Has
    assert!(blob.has(&digest).await.unwrap());

    // Read
    let reader = blob.open_read(&digest).await.unwrap().expect("should exist");
    let mut buf = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut tokio::io::BufReader::new(BlobReaderAdapter(reader)), &mut buf)
        .await
        .unwrap();
    assert_eq!(buf, data);
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn wire_blob_not_found() {
    let (addr, _router, _server_ep) = start_server().await;
    let client_ep = Endpoint::builder(iroh::endpoint::presets::N0).clear_address_lookup().bind().await.unwrap();
    let (blob, _dir) = make_clients(client_ep, addr);

    let missing = B3Digest::from(&[0u8; 32]);
    assert!(!blob.has(&missing).await.unwrap());
    assert!(blob.open_read(&missing).await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn wire_blob_large() {
    let (addr, _router, _server_ep) = start_server().await;
    let client_ep = Endpoint::builder(iroh::endpoint::presets::N0).clear_address_lookup().bind().await.unwrap();
    let (blob, _dir) = make_clients(client_ep, addr);

    // 1 MB blob
    let data: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();

    let mut writer = blob.open_write().await;
    writer.write_all(&data).await.unwrap();
    let digest = writer.close().await.unwrap();

    let reader = blob.open_read(&digest).await.unwrap().unwrap();
    let mut buf = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut tokio::io::BufReader::new(BlobReaderAdapter(reader)), &mut buf)
        .await
        .unwrap();
    assert_eq!(buf.len(), data.len());
    assert_eq!(buf, data);
}

// ========================================================================
// Directory tests
// ========================================================================

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn wire_dir_put_get() {
    let (addr, _router, _server_ep) = start_server().await;
    let client_ep = Endpoint::builder(iroh::endpoint::presets::N0).clear_address_lookup().bind().await.unwrap();
    let (_blob, dir) = make_clients(client_ep, addr);

    let directory = DIRECTORY_WITH_KEEP.clone();
    let expected = directory.digest();

    let digest = dir.put(directory).await.unwrap();
    assert_eq!(digest, expected);

    let got = dir.get(&digest).await.unwrap().expect("should exist");
    assert_eq!(got.digest(), expected);
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn wire_dir_not_found() {
    let (addr, _router, _server_ep) = start_server().await;
    let client_ep = Endpoint::builder(iroh::endpoint::presets::N0).clear_address_lookup().bind().await.unwrap();
    let (_blob, dir) = make_clients(client_ep, addr);

    let missing = B3Digest::from(&[0u8; 32]);
    assert!(dir.get(&missing).await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn wire_dir_get_recursive() {
    let (addr, _router, _server_ep) = start_server().await;
    let client_ep = Endpoint::builder(iroh::endpoint::presets::N0).clear_address_lookup().bind().await.unwrap();
    let (_blob, dir) = make_clients(client_ep, addr);

    // Put leaf, then parent
    let _leaf_digest = dir.put(DIRECTORY_WITH_KEEP.clone()).await.unwrap();
    let parent_digest = dir.put(DIRECTORY_COMPLICATED.clone()).await.unwrap();

    // Stream recursive
    let dirs: Vec<_> = dir
        .get_recursive(&parent_digest)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert!(!dirs.is_empty());
    assert_eq!(dirs[0].digest(), parent_digest, "first should be root");
    assert!(dirs.len() >= 2, "should include child directory");
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn wire_dir_put_multiple() {
    let (addr, _router, _server_ep) = start_server().await;
    let client_ep = Endpoint::builder(iroh::endpoint::presets::N0).clear_address_lookup().bind().await.unwrap();
    let (_blob, dir) = make_clients(client_ep, addr);

    let mut putter = dir.put_multiple_start();

    // Leaves-to-root order
    let leaf = DIRECTORY_WITH_KEEP.clone();
    putter.put(leaf.clone()).await.unwrap();

    let parent = DIRECTORY_COMPLICATED.clone();
    putter.put(parent.clone()).await.unwrap();

    let root = putter.close().await.unwrap();
    assert_eq!(root, parent.digest());

    // Both retrievable
    assert!(dir.get(&leaf.digest()).await.unwrap().is_some());
    assert!(dir.get(&parent.digest()).await.unwrap().is_some());
}

// ========================================================================
// Combined test
// ========================================================================

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn wire_blob_and_dir_combined() {
    let (addr, _router, _server_ep) = start_server().await;
    let client_ep = Endpoint::builder(iroh::endpoint::presets::N0).clear_address_lookup().bind().await.unwrap();
    let (blob, dir) = make_clients(client_ep, addr);

    // Write blob
    let mut writer = blob.open_write().await;
    writer.write_all(HELLOWORLD_BLOB_CONTENTS).await.unwrap();
    let blob_digest = writer.close().await.unwrap();
    assert_eq!(blob_digest, *HELLOWORLD_BLOB_DIGEST);

    // Write directory
    let dir_digest = dir.put(DIRECTORY_WITH_KEEP.clone()).await.unwrap();

    // Both accessible
    assert!(blob.has(&blob_digest).await.unwrap());
    assert!(dir.get(&dir_digest).await.unwrap().is_some());
}

#[tokio::test]
#[ignore = "requires network access (iroh socket binding) - not available in Nix sandbox"]
async fn wire_multiple_clients() {
    let (addr, _router, _server_ep) = start_server().await;

    // Two independent clients
    let ep1 = Endpoint::builder(iroh::endpoint::presets::N0).clear_address_lookup().bind().await.unwrap();
    let ep2 = Endpoint::builder(iroh::endpoint::presets::N0).clear_address_lookup().bind().await.unwrap();

    let (blob1, _) = make_clients(ep1, addr.clone());
    let (blob2, _) = make_clients(ep2, addr);

    // Client 1 writes
    let mut writer = blob1.open_write().await;
    writer.write_all(b"shared data").await.unwrap();
    let digest = writer.close().await.unwrap();

    // Client 2 reads
    assert!(blob2.has(&digest).await.unwrap());
    let reader = blob2.open_read(&digest).await.unwrap().unwrap();
    let mut buf = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut tokio::io::BufReader::new(BlobReaderAdapter(reader)), &mut buf)
        .await
        .unwrap();
    assert_eq!(buf, b"shared data");
}

// ========================================================================
// Helpers
// ========================================================================

/// Adapter: Box<dyn BlobReader> → tokio::io::AsyncRead
struct BlobReaderAdapter(Box<dyn snix_castore::blobservice::BlobReader>);

impl tokio::io::AsyncRead for BlobReaderAdapter {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut *self.0).poll_read(cx, buf)
    }
}
