#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_root_reads_and_writes_valid_relative_paths() {
        // r[verify molten.chunk_store.cap_std_boundary.tests.positive]
        let root_path = temp_dir("cap-std-positive");
        let root = ChunkStoreRoot::open(&root_path).expect("open chunk root");
        let artifact_root = ArtifactStoreRoot::open(&root_path.join("artifact")).expect("open artifact root");
        let retention_root = RetentionStoreRoot::open(&root_path.join("retention")).expect("open retention root");
        let dataspace_root = DataspaceStoreRoot::open(&root_path.join("dataspace")).expect("open dataspace root");
        let path = LocalStorePath::parse("chunks/ok.bin").expect("relative path");
        root.root().write(&path, b"chunk-bytes").expect("write under root");
        assert_eq!(root.root().read(&path).expect("read under root"), b"chunk-bytes");
        artifact_root
            .root()
            .write(&LocalStorePath::parse("objects/value.preserves").expect("artifact path"), b"artifact")
            .expect("write artifact");
        retention_root
            .root()
            .write(&LocalStorePath::parse("pins/pin.preserves").expect("retention path"), b"pin")
            .expect("write retention");
        dataspace_root
            .root()
            .write(&LocalStorePath::parse("envelopes/message.preserves").expect("dataspace path"), b"message")
            .expect("write dataspace");
        let chunks_path = LocalStorePath::parse("chunks").expect("chunks path");
        let names = root.root().list_file_names(&chunks_path).expect("list chunks");
        assert_eq!(names, vec!["ok.bin".to_string()]);
        let derived = ExchangeStoreRoot::open_chunk_subdir(&root, &chunks_path).expect("derive exchange subroot");
        assert_eq!(derived.root().kind(), LocalStoreKind::Exchange);
        assert_eq!(
            derived
                .root()
                .read(&LocalStorePath::parse("ok.bin").expect("derived path"))
                .expect("read through derived root"),
            b"chunk-bytes"
        );
    }

    #[test]
    fn capability_roots_deny_invalid_local_locators_and_missing_authority() {
        // r[verify molten.chunk_store.cap_std_boundary.tests.negative]
        assert!(
            LocalStorePath::parse("../escape")
                .expect_err("parent traversal denied")
                .to_string()
                .contains("parent")
        );
        assert!(LocalStorePath::parse("/tmp/escape").expect_err("absolute denied").to_string().contains("relative"));
        assert!(
            LocalStorePath::parse("iroh://peer/blob")
                .expect_err("remote locator denied")
                .to_string()
                .contains("local filesystem")
        );
        assert!(
            LocalStorePath::parse("blake3:abc")
                .expect_err("content ref denied")
                .to_string()
                .contains("local filesystem")
        );
        assert!(
            LocalStorePath::parse(r"C:\escape")
                .expect_err("drive-prefixed path denied")
                .to_string()
                .contains("platform-prefixed")
        );
        assert!(
            LocalStorePath::parse(r"\\server\share")
                .expect_err("UNC-prefixed path denied")
                .to_string()
                .contains("platform-prefixed")
        );
        let missing_parent =
            crate::test_support::process_workspace("cap_std_missing_root").expect("create missing-root workspace");
        let missing_root = missing_parent.join("missing");
        let error = RetentionStoreRoot::open_existing(&missing_root).expect_err("missing root denied");
        assert!(error.to_string().contains("No such") || error.to_string().contains("not found"));
    }

    #[cfg(unix)]
    #[test]
    fn capability_root_denies_symlink_escape() {
        // r[verify molten.chunk_store.cap_std_boundary.tests.negative]
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let root_path = temp_dir("cap-std-symlink");
        let outside_path = temp_dir("cap-std-outside");
        std::fs::write(outside_path.join("secret.txt"), b"secret").expect("outside secret");
        std::fs::create_dir(root_path.join("entries")).expect("create entry directory");
        std::os::unix::fs::symlink(outside_path.join("secret.txt"), root_path.join("entries/escape-link"))
            .expect("create symlink");
        std::os::unix::fs::symlink(&outside_path, root_path.join("entries/escape-directory"))
            .expect("create intermediate symlink");
        std::fs::write(root_path.join("entries/replace-me"), b"inside").expect("replacement fixture");
        let root = ExchangeStoreRoot::open(&root_path).expect("open exchange root");
        let error = root
            .root()
            .read(&LocalStorePath::parse("entries/escape-link").expect("symlink path"))
            .expect_err("symlink escape denied");
        assert!(
            error.to_string().contains("outside")
                || error.to_string().contains("symlink")
                || error.to_string().contains("permission")
                || error.to_string().contains("regular file")
        );
        let intermediate_error = root
            .root()
            .read(&LocalStorePath::parse("entries/escape-directory/secret.txt").expect("intermediate path"))
            .expect_err("intermediate symlink escape denied");
        assert!(!intermediate_error.to_string().is_empty());
        let entries = root
            .root()
            .list_entries(&LocalStorePath::parse("entries").expect("entry directory"))
            .expect("list logical entries");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "escape-directory");
        assert_eq!(entries[0].kind, LocalStoreEntryKind::Symlink);
        assert_eq!(entries[1].name, "escape-link");
        assert_eq!(entries[1].kind, LocalStoreEntryKind::Symlink);
        std::fs::remove_file(root_path.join("entries/replace-me")).expect("remove enumerated file");
        std::os::unix::fs::symlink(outside_path.join("secret.txt"), root_path.join("entries/replace-me"))
            .expect("replace enumerated file with symlink");
        let replacement_error = root
            .root()
            .read(&LocalStorePath::parse("entries/replace-me").expect("replacement path"))
            .expect_err("replacement symlink escape denied");
        assert!(!replacement_error.to_string().is_empty());
    }

    #[test]
    fn typed_roots_do_not_substitute_same_relative_handle_across_authorities() {
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let artifact_path = temp_dir("cap-std-artifact-authority");
        let chunk_path = temp_dir("cap-std-chunk-authority");
        let artifact_root = ArtifactStoreRoot::open(&artifact_path).expect("open artifact root");
        let chunk_root = ChunkStoreRoot::open(&chunk_path).expect("open chunk root");
        let locator = LocalStorePath::parse("shared/value.bin").expect("shared locator");
        artifact_root.root().write(&locator, b"artifact").expect("write artifact authority");
        chunk_root.root().write(&locator, b"chunk").expect("write chunk authority");

        assert_eq!(artifact_root.root().read(&locator).expect("read artifact authority"), b"artifact");
        assert_eq!(chunk_root.root().read(&locator).expect("read chunk authority"), b"chunk");
        assert_ne!(artifact_root.root().kind(), chunk_root.root().kind());
    }

    #[cfg(unix)]
    #[test]
    fn database_handle_rejects_symlink_and_non_regular_leaf() {
        // r[verify molten.chunk_store.cap_std_backend_handles]
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let root_path = temp_dir("cap-std-database-leaf");
        let outside_path = temp_dir("cap-std-database-outside");
        let outside_file = outside_path.join("outside.redb");
        std::fs::write(&outside_file, b"outside").expect("outside database fixture");
        std::os::unix::fs::symlink(&outside_file, root_path.join("symlink.redb")).expect("database symlink fixture");
        std::fs::create_dir(root_path.join("directory.redb")).expect("database directory fixture");
        let root = ChunkStoreRoot::open(&root_path).expect("open chunk root");

        let symlink_error = root
            .root()
            .open_database_file(&LocalStorePath::parse("symlink.redb").expect("symlink path"))
            .expect_err("database symlink denied");
        assert!(symlink_error.to_string().contains("regular file"));
        let directory_error = root
            .root()
            .open_database_file(&LocalStorePath::parse("directory.redb").expect("directory path"))
            .expect_err("database directory denied");
        assert!(directory_error.to_string().contains("regular file"));
    }

    #[cfg(unix)]
    #[test]
    fn open_capability_survives_ambient_root_replacement_without_switching_authority() {
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let root_path = temp_dir("cap-std-root-replacement");
        let moved_path = root_path.with_extension("moved");
        let root = RetentionStoreRoot::open(&root_path).expect("open retention root");
        std::fs::rename(&root_path, &moved_path).expect("move opened root");
        std::fs::create_dir(&root_path).expect("replace ambient root path");
        std::fs::write(root_path.join("replacement.txt"), b"replacement").expect("write replacement root");

        let logical_path = LocalStorePath::parse("bound.txt").expect("bound path");
        root.root().write(&logical_path, b"bound").expect("write through opened capability");
        assert_eq!(std::fs::read(moved_path.join("bound.txt")).expect("read moved root"), b"bound");
        assert!(!root_path.join("bound.txt").exists());
        assert_eq!(std::fs::read(root_path.join("replacement.txt")).expect("read replacement"), b"replacement");
    }

    fn temp_dir(label: &str) -> crate::test_support::ProcessWorkspace {
        crate::test_support::process_workspace(label).expect("create isolated local-store workspace")
    }
}
