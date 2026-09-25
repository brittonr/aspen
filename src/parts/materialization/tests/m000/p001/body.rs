
    #[test]
    fn archive_round_trip_is_bounded_and_rejects_duplicates_and_special_entries() {
        // r[verify molten.filesystem_materialization.archive_members]
        let policy = policy(ReplacementPolicy::NoReplace);
        let payloads = payloads();
        let archive_bytes = write_archive(Vec::new(), &policy, &payloads).expect("write archive");
        let verified = verify_archive(std::io::Cursor::new(archive_bytes.clone()), &policy).expect("verify archive");
        assert_eq!(verified.plan, plan_payloads(&policy, &payloads).expect("expected plan"));
        let mut corrupt_header = archive_bytes;
        corrupt_header[0] ^= 1;
        assert!(verify_archive(std::io::Cursor::new(corrupt_header), &policy).is_err());

        let duplicate_bytes = {
            let mut builder = tar::Builder::new(Vec::new());
            append_test_entry(&mut builder, "same", b"one", tar::EntryType::Regular);
            append_test_entry(&mut builder, "same", b"two", tar::EntryType::Regular);
            builder.into_inner().expect("duplicate archive")
        };
        assert!(verify_archive(std::io::Cursor::new(duplicate_bytes), &policy).is_err());

        let symlink_bytes = {
            let mut builder = tar::Builder::new(Vec::new());
            append_test_entry(&mut builder, "link", b"target", tar::EntryType::Symlink);
            builder.into_inner().expect("symlink archive")
        };
        assert!(verify_archive(std::io::Cursor::new(symlink_bytes), &policy).is_err());

        let traversal_bytes = {
            let mut builder = tar::Builder::new(Vec::new());
            append_raw_test_entry(&mut builder, b"../escape", b"escape", tar::EntryType::Regular);
            builder.into_inner().expect("traversal archive")
        };
        assert!(verify_archive(std::io::Cursor::new(traversal_bytes), &policy).is_err());

        const TINY_MAX_MEMBERS: u64 = 1;
        const TINY_MAX_MEMBER_BYTES: u64 = 4;
        const TINY_MAX_TOTAL_BYTES: u64 = 4;
        const TINY_MAX_PATH_BYTES: u64 = 128;
        let tiny = MaterializationPolicy::bounded("tiny-archive-v1", ReplacementPolicy::NoReplace)
            .expect("tiny policy")
            .with_bounds(TINY_MAX_MEMBERS, TINY_MAX_MEMBER_BYTES, TINY_MAX_TOTAL_BYTES, TINY_MAX_PATH_BYTES)
            .expect("tiny bounds");
        let oversized_bytes = {
            let mut builder = tar::Builder::new(Vec::new());
            append_test_entry(&mut builder, "large", b"large", tar::EntryType::Regular);
            builder.into_inner().expect("oversized archive")
        };
        assert!(verify_archive(std::io::Cursor::new(oversized_bytes), &tiny).is_err());

        let too_many_bytes = {
            let mut builder = tar::Builder::new(Vec::new());
            append_test_entry(&mut builder, "one", b"1", tar::EntryType::Regular);
            append_test_entry(&mut builder, "two", b"2", tar::EntryType::Regular);
            builder.into_inner().expect("too-many archive")
        };
        assert!(verify_archive(std::io::Cursor::new(too_many_bytes), &tiny).is_err());
    }

    fn append_test_entry(builder: &mut tar::Builder<Vec<u8>>, path: &str, bytes: &[u8], entry_type: tar::EntryType) {
        let mut header = test_header(bytes, entry_type);
        builder.append_data(&mut header, path, std::io::Cursor::new(bytes)).expect("append test entry");
    }

    fn append_raw_test_entry(
        builder: &mut tar::Builder<Vec<u8>>,
        path: &[u8],
        bytes: &[u8],
        entry_type: tar::EntryType,
    ) {
        let mut header = test_header(bytes, entry_type);
        header.as_mut_bytes()[..path.len()].copy_from_slice(path);
        header.set_cksum();
        builder.append(&header, std::io::Cursor::new(bytes)).expect("append raw test entry");
    }

    fn test_header(bytes: &[u8], entry_type: tar::EntryType) -> tar::Header {
        let mut header = tar::Header::new_gnu();
        header.set_size(u64::try_from(bytes.len()).expect("test entry size"));
        header.set_entry_type(entry_type);
        if entry_type.is_symlink() {
            header.set_link_name("target").expect("link name");
        }
        header.set_mode(ARCHIVE_READ_ONLY_MODE);
        header.set_cksum();
        header
    }
