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
//! Real syscalls (open, write, read, readdir, unlink, symlink, readlink, rename, truncate, rmdir,
//! xattr, statfs) pass through the kernel FUSE layer into our filesystem implementation.

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
                while let Ok(Some((reader, writer))) = ch.get_request() {
                    let _ = server.handle_message(reader, writer.into(), None, None);
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

    // --- Rename file ---
    // Note: after FUSE_RENAME, the kernel may cache a negative dentry for the
    // destination from the pre-rename LOOKUP. Reading the destination directly
    // can fail with ENOENT until the cache expires (ENTRY_TTL=1s). We verify
    // via readdir which bypasses the dentry cache.
    let rename_src = mount_path.join("rename_src.txt");
    let rename_dst = mount_path.join("rename_dst.txt");
    std::fs::write(&rename_src, b"rename me").unwrap();
    std::fs::rename(&rename_src, &rename_dst).unwrap();
    assert!(!rename_src.exists(), "rename source should be gone");
    let entries: Vec<String> = std::fs::read_dir(&mount_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        entries.contains(&"rename_dst.txt".to_string()),
        "rename_dst.txt should appear in readdir after rename, got: {entries:?}"
    );

    // --- Rename directory ---
    let dir_before = mount_path.join("dir_before");
    std::fs::create_dir(&dir_before).unwrap();
    std::fs::write(dir_before.join("child.txt"), b"child data").unwrap();
    let dir_after = mount_path.join("dir_after");
    std::fs::rename(&dir_before, &dir_after).unwrap();
    assert!(!dir_before.exists(), "renamed dir should be gone");
    let dir_entries: Vec<String> = std::fs::read_dir(&mount_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        dir_entries.contains(&"dir_after".to_string()),
        "dir_after should appear in readdir after rename, got: {dir_entries:?}"
    );

    // --- Truncate via set_len ---
    let trunc_file = mount_path.join("truncate.txt");
    std::fs::write(&trunc_file, b"0123456789").unwrap();
    {
        let f = std::fs::OpenOptions::new().write(true).open(&trunc_file).unwrap();
        f.set_len(5).unwrap();
    }
    let content = std::fs::read(&trunc_file).unwrap();
    assert_eq!(content.len(), 5);
    assert_eq!(&content[..5], b"01234");
    // Extend with zero-fill
    {
        let f = std::fs::OpenOptions::new().write(true).open(&trunc_file).unwrap();
        f.set_len(8).unwrap();
    }
    let content = std::fs::read(&trunc_file).unwrap();
    assert_eq!(content.len(), 8);
    assert_eq!(&content[..5], b"01234");
    assert_eq!(&content[5..], b"\0\0\0");

    // --- Append writes ---
    let append_file = mount_path.join("append.txt");
    std::fs::write(&append_file, b"hello").unwrap();
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().append(true).open(&append_file).unwrap();
        f.write_all(b" world").unwrap();
    }
    let content = std::fs::read(&append_file).unwrap();
    assert_eq!(content, b"hello world");

    // --- rmdir (empty) ---
    let empty_dir = mount_path.join("empty_dir");
    std::fs::create_dir(&empty_dir).unwrap();
    std::fs::remove_dir(&empty_dir).unwrap();
    assert!(!empty_dir.exists(), "empty dir should be removed");

    // --- rmdir non-empty fails ---
    let nonempty_dir = mount_path.join("nonempty_dir");
    std::fs::create_dir(&nonempty_dir).unwrap();
    std::fs::write(nonempty_dir.join("file.txt"), b"data").unwrap();
    let result = std::fs::remove_dir(&nonempty_dir);
    assert!(result.is_err(), "rmdir on non-empty dir should fail");

    // --- xattr operations ---
    let xattr_file = mount_path.join("xattr_test.txt");
    std::fs::write(&xattr_file, b"xattr content").unwrap();
    xattr::set(&xattr_file, "user.myattr", b"myvalue").unwrap();
    let val = xattr::get(&xattr_file, "user.myattr").unwrap().unwrap();
    assert_eq!(val, b"myvalue");
    let names: Vec<_> = xattr::list(&xattr_file).unwrap().collect();
    assert!(
        names.iter().any(|n| n.to_string_lossy() == "user.myattr"),
        "xattr list should include user.myattr, got: {names:?}"
    );
    xattr::remove(&xattr_file, "user.myattr").unwrap();
    let val = xattr::get(&xattr_file, "user.myattr").unwrap();
    assert!(val.is_none(), "xattr should be removed");

    // --- statfs ---
    {
        use nix::sys::statvfs::statvfs;
        let stat = statvfs(&mount_path).unwrap();
        assert!(stat.block_size() > 0, "block_size should be > 0");
    }

    // --- Cleanup ---
    session.wake().unwrap();
    session.umount().unwrap();
    for w in workers {
        w.join().unwrap();
    }
}
