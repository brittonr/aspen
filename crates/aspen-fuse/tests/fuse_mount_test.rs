//! Integration test: mount AspenFs via /dev/fuse and do real file I/O.
//!
//! This test exercises the full kernel FUSE path:
//!
//! ```text
//! std::fs::write("hello.txt")
//!        │
//!        ▼
//! Linux VFS → /dev/fuse → FuseSession → Server<AspenFs>
//!        │
//!        ▼
//! AspenFs (FileSystem trait) → InMemory BTreeMap
//! ```
//!
//! Real syscalls (open, write, read, readdir, unlink, symlink, readlink)
//! pass through the kernel FUSE layer into our filesystem implementation.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use aspen_fuse::AspenFs;
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
                loop {
                    match ch.get_request() {
                        Ok(Some((reader, writer))) => {
                            let _ = server.handle_message(reader, writer.into(), None, None);
                        }
                        Ok(None) | Err(_) => break,
                    }
                }
            })
        })
        .collect()
}

#[test]
fn test_fuse_mount_file_operations() {
    let mount_dir = tempfile::tempdir().unwrap();
    let mount_path = mount_dir.path().to_path_buf();

    // Create in-memory filesystem owned by current user
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    let fs = AspenFs::new_in_memory(uid, gid);
    let server = Arc::new(Server::new(fs));

    // Mount via /dev/fuse (falls back to fusermount3 for non-root)
    let mut session = FuseSession::new(&mount_path, "aspen-test", "aspen", false).unwrap();
    session.set_allow_other(false);
    session.mount().unwrap();

    let workers = spawn_fuse_workers(&server, &session, 2);

    // Give FUSE_INIT handshake time to complete
    thread::sleep(Duration::from_millis(200));

    // --- Create and write a file ---
    let test_file = mount_path.join("hello.txt");
    std::fs::write(&test_file, b"hello world").unwrap();

    // --- Read it back ---
    let content = std::fs::read(&test_file).unwrap();
    assert_eq!(content, b"hello world");

    // --- Overwrite with different content ---
    std::fs::write(&test_file, b"updated").unwrap();
    let content = std::fs::read(&test_file).unwrap();
    assert_eq!(content, b"updated");

    // --- Create a subdirectory ---
    let sub_dir = mount_path.join("subdir");
    std::fs::create_dir(&sub_dir).unwrap();

    // --- Create file in subdirectory ---
    let nested_file = sub_dir.join("nested.txt");
    std::fs::write(&nested_file, b"nested content").unwrap();
    let content = std::fs::read(&nested_file).unwrap();
    assert_eq!(content, b"nested content");

    // --- List root directory ---
    let entries: Vec<String> = std::fs::read_dir(&mount_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(entries.contains(&"hello.txt".to_string()), "missing hello.txt in {entries:?}");
    assert!(entries.contains(&"subdir".to_string()), "missing subdir in {entries:?}");

    // --- Delete a file ---
    std::fs::remove_file(&test_file).unwrap();
    assert!(!test_file.exists());

    // --- Symlink ---
    std::os::unix::fs::symlink("/some/target", mount_path.join("mylink")).unwrap();
    let target = std::fs::read_link(mount_path.join("mylink")).unwrap();
    assert_eq!(target.to_str().unwrap(), "/some/target");

    // --- File metadata ---
    let meta = std::fs::metadata(&nested_file).unwrap();
    assert!(meta.is_file());
    assert_eq!(meta.len(), 14); // "nested content".len()

    let dir_meta = std::fs::metadata(&sub_dir).unwrap();
    assert!(dir_meta.is_dir());

    // --- Cleanup ---
    session.wake().unwrap();
    session.umount().unwrap();
    for w in workers {
        w.join().unwrap();
    }
}
