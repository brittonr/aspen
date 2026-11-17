    #[test]
    fn bounded_read_enforces_exact_bytes_and_rechecks_content_identity() {
        let workspace = crate::test_support::process_workspace("ledger-bounded-read").expect("owned workspace");
        let root = open_capability_ledger_root(workspace.as_ref()).expect("open ledger");
        let artifact = crate::preserves_rail::record("bounded-read-control", vec![crate::preserves_rail::string("original")]);
        let bytes = crate::preserves_rail::canonical_bytes(&artifact).expect("canonical bytes");
        let imported = import_artifact_with_root(&root, &artifact).expect("import artifact");
        let maximum = u64::try_from(bytes.len()).expect("fixture size");
        assert_eq!(read_artifact_bounded_with_root(&root, &imported.artifact_ref, maximum).expect("exact limit"), artifact);
        assert!(read_artifact_bounded_with_root(&root, &imported.artifact_ref, maximum - 1).is_err());
        let changed = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::string("changed")).expect("changed bytes");
        root.root().write(&content_store_path(&imported.artifact_ref).expect("content locator"), &changed).expect("adversarial rewrite");
        assert!(read_artifact_bounded_with_root(&root, &imported.artifact_ref, maximum).is_err());
    }

    #[test]
    fn bounded_read_denies_deep_packed_values_before_tree_allocation() {
        let workspace = crate::test_support::process_workspace("ledger-bounded-depth").expect("owned workspace");
        let root = open_capability_ledger_root(workspace.as_ref()).expect("open ledger");
        let depth = crate::preserves_rail::packed_bounds::MAX_DEPTH + 1;
        let mut bytes = vec![u8::from(preserves::packed::constants::Tag::Sequence); depth];
        bytes.push(u8::from(preserves::packed::constants::Tag::True));
        bytes.extend(std::iter::repeat_n(u8::from(preserves::packed::constants::Tag::End), depth));
        let reference = crate::preserves_rail::content_ref_from_bytes(&bytes);
        root.root().write(&content_store_path(&reference).expect("content locator"), &bytes).expect("adversarial packed value");
        assert!(read_artifact_bounded_with_root(&root, &reference, u64::try_from(bytes.len()).expect("fixture size")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn bounded_read_denies_symlink_and_fifo_members_without_waiting_for_a_writer() {
        let workspace = crate::test_support::process_workspace("ledger-bounded-kind").expect("owned workspace");
        let root = open_capability_ledger_root(workspace.as_ref()).expect("open ledger");
        let artifact = crate::preserves_rail::string("file-kind-control");
        let imported = import_artifact_with_root(&root, &artifact).expect("import artifact");
        let path = content_store_path(&imported.artifact_ref).expect("content locator");
        let directory = root.root().try_clone_dir().expect("retained capability");
        directory.rename(path.as_path(), &directory, "original").expect("retain original");
        directory.symlink("../original", path.as_path()).expect("hostile symlink");
        assert!(read_artifact_bounded_with_root(&root, &imported.artifact_ref, 1_024).is_err());
        directory.remove_file(path.as_path()).expect("remove owned symlink");
        rustix::fs::mkfifoat(&directory, path.as_path(), rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR).expect("owned FIFO");
        assert!(read_artifact_bounded_with_root(&root, &imported.artifact_ref, 1_024).is_err());
    }
