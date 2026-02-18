//! Integration test: mount AspenFs via /dev/fuse backed by a real Raft cluster.
//!
//! Full data path:
//!
//! ```text
//! std::fs::write("hello.txt")
//!        |
//!        v
//! Linux VFS -> /dev/fuse -> FuseSession -> Server<AspenFs>
//!        |
//!        v
//! AspenFs -> FuseSyncClient -> Iroh QUIC -> ClientProtocolHandler -> RaftNode
//! ```
//!
//! This validates end-to-end: POSIX syscalls flow through FUSE into the real
//! Raft consensus layer with in-memory storage.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::InitRequest;
use aspen::api::KeyValueStore;
use aspen::api::ReadRequest;
use aspen::node::NodeBuilder;
use aspen::node::NodeId;
use aspen::raft::storage::StorageBackend;
use aspen_fuse::AspenFs;
use aspen_fuse::FuseSyncClient;
use fuse_backend_rs::api::server::Server;
use fuse_backend_rs::transport::FuseSession;

/// Spawn FUSE worker threads that process requests from /dev/fuse.
fn spawn_fuse_workers(
    server: &Arc<Server<AspenFs>>,
    session: &FuseSession,
    count: usize,
) -> Vec<thread::JoinHandle<()>> {
    (0..count)
        .map(|_| {
            let server = server.clone();
            let mut ch = session.new_channel().unwrap();
            thread::spawn(move || {
                while let Ok(Some((reader, writer))) = ch.get_request() {
                    let _ = server.handle_message(reader, writer.into(), None, None);
                }
            })
        })
        .collect()
}

/// End-to-end test: FUSE mount backed by a single-node Raft cluster.
///
/// Requires /dev/fuse access (run with `--run-ignored all`).
#[tokio::test]
#[ignore]
async fn test_fuse_mount_with_raft_cluster() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().join("node-1");
    let mount_dir = tempfile::tempdir().unwrap();
    let mount_path = mount_dir.path().to_path_buf();

    // --- Start single-node Raft cluster ---
    let mut node = NodeBuilder::new(NodeId(1), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_cookie("fuse-raft-test")
        .with_mdns(false)
        .start()
        .await
        .unwrap();

    // Register CLIENT_ALPN handler (required for FuseSyncClient)
    node.spawn_router();

    // Initialize cluster
    let init_request = InitRequest {
        initial_members: vec![ClusterNode::with_iroh_addr(1, node.endpoint_addr())],
    };
    node.raft_node().init(init_request).await.unwrap();

    // Wait for leader election
    tokio::time::sleep(Duration::from_millis(500)).await;

    // --- Create FuseSyncClient connected to the Raft cluster ---
    // FuseSyncClient creates its own tokio runtime, so use block_in_place
    // to avoid nesting runtimes
    let endpoint_addr = node.endpoint_addr();
    let client = tokio::task::block_in_place(|| Arc::new(FuseSyncClient::new(endpoint_addr).unwrap()));

    // --- Create AspenFs backed by real cluster ---
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    let fs = AspenFs::new(uid, gid, client);
    let server = Arc::new(Server::new(fs));

    // --- Mount via /dev/fuse ---
    let mut session = FuseSession::new(&mount_path, "aspen-raft-test", "aspen", false).unwrap();
    session.set_allow_other(false);
    session.mount().unwrap();

    let workers = spawn_fuse_workers(&server, &session, 2);

    // Give FUSE_INIT handshake time to complete
    thread::sleep(Duration::from_millis(200));

    // --- Write a file through FUSE ---
    let test_file = mount_path.join("raft_hello.txt");
    std::fs::write(&test_file, b"hello from raft").unwrap();

    // --- Read it back through FUSE ---
    let content = std::fs::read(&test_file).unwrap();
    assert_eq!(content, b"hello from raft");

    // --- Create subdirectory + nested file ---
    let sub_dir = mount_path.join("raft_subdir");
    std::fs::create_dir(&sub_dir).unwrap();
    let nested = sub_dir.join("nested.txt");
    std::fs::write(&nested, b"nested raft data").unwrap();
    let content = std::fs::read(&nested).unwrap();
    assert_eq!(content, b"nested raft data");

    // --- List root directory ---
    let entries: Vec<String> = std::fs::read_dir(&mount_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(entries.contains(&"raft_hello.txt".to_string()), "missing raft_hello.txt in {entries:?}");
    assert!(entries.contains(&"raft_subdir".to_string()), "missing raft_subdir in {entries:?}");

    // --- Verify data exists in Raft store ---
    // AspenFs stores file content at the path key (without leading slash)
    // Use ASCII data so String conversion works cleanly
    let raft_node = node.raft_node();
    let read_result = raft_node.read(ReadRequest::new("raft_hello.txt")).await;
    assert!(read_result.is_ok(), "should be able to read key from Raft: {:?}", read_result);

    // --- Clean shutdown ---
    session.wake().unwrap();
    session.umount().unwrap();
    for w in workers {
        w.join().unwrap();
    }

    node.shutdown().await.unwrap();
}
