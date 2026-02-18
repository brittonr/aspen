#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use fuse_backend_rs::api::filesystem::Context;
    use fuse_backend_rs::api::filesystem::FileSystem;
    use fuse_backend_rs::api::filesystem::FsOptions;

    use crate::constants::ATTR_TTL;
    use crate::constants::BLOCK_SIZE;
    use crate::constants::DEFAULT_DIR_MODE;
    use crate::constants::DEFAULT_FILE_MODE;
    use crate::constants::DEFAULT_SYMLINK_MODE;
    use crate::constants::ENTRY_TTL;
    use crate::constants::MAX_KEY_SIZE;
    use crate::constants::MAX_VALUE_SIZE;
    use crate::constants::MAX_XATTR_NAME_SIZE;
    use crate::constants::MAX_XATTR_VALUE_SIZE;
    use crate::constants::MAX_XATTRS_PER_FILE;
    use crate::constants::ROOT_INODE;
    use crate::constants::SYMLINK_SUFFIX;
    use crate::constants::XATTR_PREFIX;
    use crate::fs::AspenFs;
    use crate::inode::EntryType;

    /// Create a mock Context for testing.
    fn mock_context() -> Context {
        Context::default()
    }

    // === Path/Key Conversion Tests ===

    #[test]
    fn test_path_to_key_strips_leading_slash() {
        assert_eq!(AspenFs::path_to_key("/myapp/config").unwrap(), "myapp/config");
        assert_eq!(AspenFs::path_to_key("/simple").unwrap(), "simple");
        assert_eq!(AspenFs::path_to_key("/a/b/c").unwrap(), "a/b/c");
    }

    #[test]
    fn test_path_to_key_empty_path() {
        assert_eq!(AspenFs::path_to_key("/").unwrap(), "");
        assert_eq!(AspenFs::path_to_key("").unwrap(), "");
    }

    #[test]
    fn test_path_to_key_no_leading_slash() {
        // Paths without leading slash pass through
        assert_eq!(AspenFs::path_to_key("already/no/slash").unwrap(), "already/no/slash");
    }

    #[test]
    fn test_path_to_key_rejects_parent_traversal() {
        // Security: Reject paths with parent directory traversal
        assert!(AspenFs::path_to_key("/app/../etc/passwd").is_err());
        assert!(AspenFs::path_to_key("../etc/passwd").is_err());
        assert!(AspenFs::path_to_key("/app/data/../../etc").is_err());
        assert!(AspenFs::path_to_key("..").is_err());
    }

    #[test]
    fn test_path_to_key_normalizes_current_dir() {
        // Current directory components are skipped
        assert_eq!(AspenFs::path_to_key("/app/./config").unwrap(), "app/config");
        assert_eq!(AspenFs::path_to_key("./app/config").unwrap(), "app/config");
        assert_eq!(AspenFs::path_to_key("/./app/./data/.").unwrap(), "app/data");
    }

    #[test]
    fn test_path_to_key_normalizes_multiple_slashes() {
        // Multiple slashes are normalized
        assert_eq!(AspenFs::path_to_key("//app//config").unwrap(), "app/config");
        assert_eq!(AspenFs::path_to_key("/app///data").unwrap(), "app/data");
    }

    #[test]
    fn test_key_to_path_non_empty() {
        assert_eq!(AspenFs::key_to_path("myapp/config"), "myapp/config");
        assert_eq!(AspenFs::key_to_path("simple"), "simple");
    }

    #[test]
    fn test_key_to_path_empty() {
        assert_eq!(AspenFs::key_to_path(""), "");
    }

    // === Mock Filesystem Tests ===

    #[test]
    fn test_new_mock_creates_filesystem() {
        let fs = AspenFs::new_mock(1000, 1000);
        assert!(matches!(fs.backend, crate::fs::KvBackend::None));
    }

    #[test]
    fn test_mock_kv_read_returns_none() {
        let fs = AspenFs::new_mock(1000, 1000);
        let result = fs.kv_read("any_key").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_mock_kv_write_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let result = fs.kv_write("key", b"value");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_kv_delete_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let result = fs.kv_delete("key");
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_kv_scan_returns_empty() {
        let fs = AspenFs::new_mock(1000, 1000);
        let result = fs.kv_scan("prefix/").unwrap();
        assert!(result.is_empty());
    }

    // === Size Limit Tests ===

    #[test]
    fn test_kv_write_rejects_oversized_key() {
        let fs = AspenFs::new_mock(1000, 1000);
        let oversized_key = "k".repeat(MAX_KEY_SIZE + 1);
        let result = fs.kv_write(&oversized_key, b"value");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("key too large"));
    }

    #[test]
    fn test_kv_write_rejects_oversized_value() {
        let fs = AspenFs::new_mock(1000, 1000);
        let oversized_value = vec![0u8; MAX_VALUE_SIZE + 1];
        let result = fs.kv_write("key", &oversized_value);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("value too large"));
    }

    #[test]
    fn test_kv_write_accepts_max_key_size() {
        let fs = AspenFs::new_mock(1000, 1000);
        let max_key = "k".repeat(MAX_KEY_SIZE);
        let result = fs.kv_write(&max_key, b"value");
        assert!(result.is_ok());
    }

    #[test]
    fn test_kv_write_accepts_max_value_size() {
        let fs = AspenFs::new_mock(1000, 1000);
        let max_value = vec![0u8; MAX_VALUE_SIZE];
        let result = fs.kv_write("key", &max_value);
        assert!(result.is_ok());
    }

    // === Attribute Generation Tests ===

    #[test]
    fn test_make_attr_file() {
        let fs = AspenFs::new_mock(1000, 1000);
        let attr = fs.make_attr(42, EntryType::File, 1024);

        assert_eq!(attr.st_ino, 42);
        assert_eq!(attr.st_mode, DEFAULT_FILE_MODE);
        assert_eq!(attr.st_nlink, 1);
        assert_eq!(attr.st_uid, 1000);
        assert_eq!(attr.st_gid, 1000);
        assert_eq!(attr.st_size, 1024);
        assert_eq!(attr.st_blksize, i64::from(BLOCK_SIZE));
    }

    #[test]
    fn test_make_attr_directory() {
        let fs = AspenFs::new_mock(1000, 1000);
        let attr = fs.make_attr(1, EntryType::Directory, 0);

        assert_eq!(attr.st_ino, 1);
        assert_eq!(attr.st_mode, DEFAULT_DIR_MODE);
        assert_eq!(attr.st_nlink, 2); // Directories have nlink=2
        assert_eq!(attr.st_size, 0);
    }

    #[test]
    fn test_make_attr_block_count() {
        let fs = AspenFs::new_mock(1000, 1000);

        // Empty file
        let attr = fs.make_attr(1, EntryType::File, 0);
        assert_eq!(attr.st_blocks, 0);

        // 1 byte file (1 block)
        let attr = fs.make_attr(1, EntryType::File, 1);
        assert_eq!(attr.st_blocks, 1);

        // Exactly 1 block
        let attr = fs.make_attr(1, EntryType::File, u64::from(BLOCK_SIZE));
        assert_eq!(attr.st_blocks, 1);

        // Just over 1 block
        let attr = fs.make_attr(1, EntryType::File, u64::from(BLOCK_SIZE) + 1);
        assert_eq!(attr.st_blocks, 2);
    }

    // === Entry Generation Tests ===

    #[test]
    fn test_make_entry_file() {
        let fs = AspenFs::new_mock(1000, 1000);
        let entry = fs.make_entry(42, EntryType::File, 512);

        assert_eq!(entry.inode, 42);
        assert_eq!(entry.generation, 0);
        assert_eq!(entry.attr.st_ino, 42);
        assert_eq!(entry.attr.st_size, 512);
        assert_eq!(entry.attr_timeout, ATTR_TTL);
        assert_eq!(entry.entry_timeout, ENTRY_TTL);
    }

    #[test]
    fn test_make_entry_directory() {
        let fs = AspenFs::new_mock(1000, 1000);
        let entry = fs.make_entry(1, EntryType::Directory, 0);

        assert_eq!(entry.inode, 1);
        assert_eq!(entry.attr.st_mode, DEFAULT_DIR_MODE);
    }

    // === Existence Checks ===

    #[test]
    fn test_exists_returns_none_in_mock_mode() {
        let fs = AspenFs::new_mock(1000, 1000);
        // In mock mode, no keys exist
        let result = fs.exists("any/path").unwrap();
        assert!(result.is_none());
    }

    // === FileSystem Trait Method Tests ===

    #[test]
    fn test_init_returns_empty_options() {
        let fs = AspenFs::new_mock(1000, 1000);
        let options = fs.init(FsOptions::empty()).unwrap();
        assert!(options.is_empty());
    }

    #[test]
    fn test_getattr_root() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let (attr, ttl) = fs.getattr(&ctx, ROOT_INODE, None).unwrap();

        assert_eq!(attr.st_ino, ROOT_INODE);
        assert_eq!(attr.st_mode, DEFAULT_DIR_MODE);
        assert_eq!(ttl, ATTR_TTL);
    }

    #[test]
    fn test_getattr_unknown_inode() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.getattr(&ctx, 99999, None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_opendir_root() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let (handle, options) = fs.opendir(&ctx, ROOT_INODE, 0).unwrap();

        assert!(handle.is_none()); // Stateless
        assert!(options.is_empty());
    }

    #[test]
    fn test_opendir_unknown_inode() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.opendir(&ctx, 99999, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_open_root_fails() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // Root is a directory, open should fail
        let result = fs.open(&ctx, ROOT_INODE, 0, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::IsADirectory);
    }

    #[test]
    fn test_mkdir_and_rmdir() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // Create directory under root
        let name = CString::new("testdir").unwrap();
        let entry = fs.mkdir(&ctx, ROOT_INODE, &name, 0o755, 0o022).unwrap();

        assert_ne!(entry.inode, ROOT_INODE);
        assert_eq!(entry.attr.st_mode, DEFAULT_DIR_MODE);

        // Remove directory
        let result = fs.rmdir(&ctx, ROOT_INODE, &name);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fsync_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.fsync(&ctx, ROOT_INODE, false, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_flush_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.flush(&ctx, ROOT_INODE, 0, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_release_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.release(&ctx, ROOT_INODE, 0, 0, false, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_destroy_is_noop() {
        let fs = AspenFs::new_mock(1000, 1000);
        // Should not panic
        fs.destroy();
    }

    // === Inode Manager Integration Tests ===

    #[test]
    fn test_inode_manager_integration() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // Create a directory
        let name = CString::new("mydir").unwrap();
        let entry = fs.mkdir(&ctx, ROOT_INODE, &name, 0o755, 0o022).unwrap();

        // Inode should be retrievable via getattr
        let (attr, _) = fs.getattr(&ctx, entry.inode, None).unwrap();
        assert_eq!(attr.st_ino, entry.inode);
        assert_eq!(attr.st_mode, DEFAULT_DIR_MODE);
    }

    #[test]
    fn test_lookup_root_child() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // First create a directory so there's something to look up
        let name = CString::new("subdir").unwrap();
        let _created = fs.mkdir(&ctx, ROOT_INODE, &name, 0o755, 0o022).unwrap();

        // Lookup should fail in mock mode (no actual KV entries)
        // because exists() returns None
        let result = fs.lookup(&ctx, ROOT_INODE, &name);

        // In mock mode, lookup fails because kv_read returns None
        // and directory check (kv_scan) returns empty
        assert!(result.is_err());
    }

    // === Symlink Tests ===

    #[test]
    fn test_make_attr_symlink() {
        let fs = AspenFs::new_mock(1000, 1000);
        let attr = fs.make_attr(42, EntryType::Symlink, 10);

        assert_eq!(attr.st_ino, 42);
        assert_eq!(attr.st_mode, DEFAULT_SYMLINK_MODE);
        assert_eq!(attr.st_nlink, 1);
        assert_eq!(attr.st_size, 10);
    }

    #[test]
    fn test_symlink_creates_entry() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let name = CString::new("mylink").unwrap();
        let target = CString::new("/target/path").unwrap();

        let entry = fs.symlink(&ctx, &target, ROOT_INODE, &name).unwrap();

        assert_ne!(entry.inode, ROOT_INODE);
        assert_eq!(entry.attr.st_mode, DEFAULT_SYMLINK_MODE);
    }

    #[test]
    fn test_readlink_requires_known_inode() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // Unknown inode should fail
        let result = fs.readlink(&ctx, 99999);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_readlink_on_symlink() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // Create a symlink first
        let name = CString::new("link").unwrap();
        let target = CString::new("/some/target").unwrap();
        let entry = fs.symlink(&ctx, &target, ROOT_INODE, &name).unwrap();

        // Read the link target - in mock mode returns empty
        // because kv_read returns None
        let result = fs.readlink(&ctx, entry.inode);
        assert!(result.is_err()); // Mock mode has no KV data
    }

    // === Statfs Tests ===

    #[test]
    fn test_statfs_returns_valid_data() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let stat = fs.statfs(&ctx, ROOT_INODE).unwrap();

        // Check basic fields
        assert_eq!(stat.f_bsize, u64::from(BLOCK_SIZE));
        assert_eq!(stat.f_namemax, MAX_KEY_SIZE as u64);
        assert!(stat.f_blocks > 0);
        assert!(stat.f_bfree > 0);
        assert!(stat.f_bavail > 0);
    }

    // === Access Tests ===

    #[test]
    fn test_access_root_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // Access check on root should succeed
        let result = fs.access(&ctx, ROOT_INODE, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_access_unknown_inode_fails() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.access(&ctx, 99999, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    // === Extended Attributes Tests ===

    #[test]
    fn test_getxattr_unknown_inode_fails() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let name = CString::new("user.test").unwrap();
        let result = fs.getxattr(&ctx, 99999, &name, 0);
        // getxattr returns GetxattrReply, check for error via match
        match result {
            Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
            Ok(_) => panic!("Expected error for unknown inode"),
        }
    }

    #[test]
    fn test_getxattr_on_root() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let name = CString::new("user.test").unwrap();

        // In mock mode, xattr doesn't exist (returns ENODATA)
        let result = fs.getxattr(&ctx, ROOT_INODE, &name, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_setxattr_on_root() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let name = CString::new("user.test").unwrap();
        let value = b"test value";

        // Set xattr on root (should succeed in mock mode)
        let result = fs.setxattr(&ctx, ROOT_INODE, &name, value, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_setxattr_rejects_oversized_name() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let oversized_name = "x".repeat(MAX_XATTR_NAME_SIZE + 1);
        let name = CString::new(oversized_name).unwrap();
        let value = b"test";

        let result = fs.setxattr(&ctx, ROOT_INODE, &name, value, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_setxattr_rejects_oversized_value() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let name = CString::new("user.test").unwrap();
        let oversized_value = vec![0u8; MAX_XATTR_VALUE_SIZE + 1];

        let result = fs.setxattr(&ctx, ROOT_INODE, &name, &oversized_value, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_listxattr_on_root() {
        use fuse_backend_rs::api::filesystem::ListxattrReply;

        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // In mock mode, returns empty list
        let result = fs.listxattr(&ctx, ROOT_INODE, 0).unwrap();
        match result {
            ListxattrReply::Names(names) => assert!(names.is_empty()),
            ListxattrReply::Count(count) => assert_eq!(count, 0),
        }
    }

    #[test]
    fn test_removexattr_unknown_inode_fails() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let name = CString::new("user.test").unwrap();
        let result = fs.removexattr(&ctx, 99999, &name);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_removexattr_on_root() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let name = CString::new("user.test").unwrap();

        // In mock mode, xattr doesn't exist, so removexattr returns NotFound
        let result = fs.removexattr(&ctx, ROOT_INODE, &name);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    // === Rename Tests ===

    #[test]
    fn test_rename_unknown_source_fails() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let old_name = CString::new("nonexistent").unwrap();
        let new_name = CString::new("target").unwrap();

        // Unknown source inode should fail
        let result = fs.rename(&ctx, 99999, &old_name, ROOT_INODE, &new_name, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_rename_directory() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        // Create a directory
        let old_name = CString::new("olddir").unwrap();
        let entry = fs.mkdir(&ctx, ROOT_INODE, &old_name, 0o755, 0o022).unwrap();

        // Rename it - in mock mode, exists() returns None so rename fails
        // because the source path is not found in KV store
        let new_name = CString::new("newdir").unwrap();
        let result = fs.rename(&ctx, ROOT_INODE, &old_name, ROOT_INODE, &new_name, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);

        // But the inode itself should still be accessible via getattr
        let (attr, _) = fs.getattr(&ctx, entry.inode, None).unwrap();
        assert_eq!(attr.st_ino, entry.inode);
    }

    // === releasedir and fsyncdir Tests ===

    #[test]
    fn test_releasedir_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.releasedir(&ctx, ROOT_INODE, 0, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fsyncdir_succeeds() {
        let fs = AspenFs::new_mock(1000, 1000);
        let ctx = mock_context();

        let result = fs.fsyncdir(&ctx, ROOT_INODE, false, 0);
        assert!(result.is_ok());
    }

    // === In-Memory Backend Tests ===

    #[test]
    fn test_in_memory_kv_round_trip() {
        let fs = AspenFs::new_in_memory(1000, 1000);

        // Write should persist
        fs.kv_write("test/key", b"hello world").unwrap();

        // Read should return the written value
        let result = fs.kv_read("test/key").unwrap();
        assert_eq!(result, Some(b"hello world".to_vec()));

        // exists() should detect the file
        let entry_type = fs.exists("test/key").unwrap();
        assert_eq!(entry_type, Some(EntryType::File));

        // exists() should detect parent as directory
        let entry_type = fs.exists("test").unwrap();
        assert_eq!(entry_type, Some(EntryType::Directory));

        // Scan should find the key
        let keys = fs.kv_scan("test/").unwrap();
        assert_eq!(keys, vec!["test/key"]);

        // Delete should remove it
        fs.kv_delete("test/key").unwrap();
        let result = fs.kv_read("test/key").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_in_memory_lookup_file() {
        let fs = AspenFs::new_in_memory(1000, 1000);
        let ctx = mock_context();

        // Write a key to simulate a file
        fs.kv_write("myfile.txt", b"content").unwrap();

        // Lookup should find it and report correct size
        let name = CString::new("myfile.txt").unwrap();
        let entry = fs.lookup(&ctx, ROOT_INODE, &name).unwrap();
        assert_eq!(entry.attr.st_size, 7); // "content".len()
        assert_eq!(entry.attr.st_mode, DEFAULT_FILE_MODE);
    }

    #[test]
    fn test_in_memory_symlink_round_trip() {
        let fs = AspenFs::new_in_memory(1000, 1000);
        let ctx = mock_context();

        // Create symlink
        let name = CString::new("mylink").unwrap();
        let target = CString::new("/target/path").unwrap();
        let entry = fs.symlink(&ctx, &target, ROOT_INODE, &name).unwrap();
        assert_eq!(entry.attr.st_mode, DEFAULT_SYMLINK_MODE);

        // Readlink should return the target
        let link_target = fs.readlink(&ctx, entry.inode).unwrap();
        assert_eq!(link_target, b"/target/path");
    }

    #[test]
    fn test_in_memory_create_write_read() {
        use fuse_backend_rs::abi::fuse_abi::CreateIn;
        use fuse_backend_rs::api::filesystem::FileSystem;

        let fs = AspenFs::new_in_memory(1000, 1000);
        let ctx = mock_context();

        // Create a file under root
        let name = CString::new("data.bin").unwrap();
        // SAFETY: CreateIn is a C struct where all fields are integers
        let args: CreateIn = unsafe { std::mem::zeroed() };
        let (entry, _, _, _) = fs.create(&ctx, ROOT_INODE, &name, args).unwrap();
        assert_eq!(entry.attr.st_size, 0);

        // Verify the file exists in KV
        let value = fs.kv_read("data.bin").unwrap();
        assert_eq!(value, Some(vec![])); // create writes empty value
    }

    #[test]
    fn test_in_memory_readdir_lists_children() {
        use fuse_backend_rs::api::filesystem::DirEntry;
        use fuse_backend_rs::api::filesystem::FileSystem;

        let fs = AspenFs::new_in_memory(1000, 1000);
        let ctx = mock_context();

        // Write files to simulate directory contents
        fs.kv_write("alpha", b"a").unwrap();
        fs.kv_write("beta", b"b").unwrap();
        fs.kv_write("sub/nested", b"n").unwrap();

        // readdir on root should list alpha, beta, sub (direct children only)
        let mut entries = Vec::new();
        fs.readdir(&ctx, ROOT_INODE, 0, u32::MAX, 0, &mut |entry: DirEntry| {
            entries.push(String::from_utf8_lossy(entry.name).to_string());
            Ok(entry.name.len() + 24) // approximate dirent size
        })
        .unwrap();

        // Should have ".", "..", "alpha", "beta", "sub"
        assert!(entries.contains(&".".to_string()));
        assert!(entries.contains(&"..".to_string()));
        assert!(entries.contains(&"alpha".to_string()));
        assert!(entries.contains(&"beta".to_string()));
        assert!(entries.contains(&"sub".to_string()));
    }

    #[test]
    fn test_in_memory_xattr_round_trip() {
        use fuse_backend_rs::api::filesystem::FileSystem;
        use fuse_backend_rs::api::filesystem::GetxattrReply;

        let fs = AspenFs::new_in_memory(1000, 1000);
        let ctx = mock_context();

        // Set xattr on root
        let attr_name = CString::new("user.test").unwrap();
        fs.setxattr(&ctx, ROOT_INODE, &attr_name, b"my-value", 0).unwrap();

        // Get xattr should return the value
        let reply = fs.getxattr(&ctx, ROOT_INODE, &attr_name, 256).unwrap();
        match reply {
            GetxattrReply::Value(v) => assert_eq!(v, b"my-value"),
            _ => panic!("expected GetxattrReply::Value"),
        }

        // Remove xattr
        fs.removexattr(&ctx, ROOT_INODE, &attr_name).unwrap();

        // Get xattr should now fail
        let result = fs.getxattr(&ctx, ROOT_INODE, &attr_name, 256);
        assert!(result.is_err());
    }

    // === Constants Tests ===

    #[test]
    fn test_symlink_mode_bits() {
        // 0o120777 = symlink (0o120000) + rwxrwxrwx (0o777)
        assert_eq!(DEFAULT_SYMLINK_MODE & 0o170000, 0o120000); // Symlink type
        assert_eq!(DEFAULT_SYMLINK_MODE & 0o777, 0o777); // Permissions
    }

    #[test]
    fn test_xattr_constants() {
        assert_eq!(SYMLINK_SUFFIX, ".symlink");
        assert_eq!(XATTR_PREFIX, ".xattr.");
        assert_eq!(MAX_XATTR_NAME_SIZE, 255);
        assert_eq!(MAX_XATTR_VALUE_SIZE, 64 * 1024);
        assert_eq!(MAX_XATTRS_PER_FILE, 100);
    }
}
