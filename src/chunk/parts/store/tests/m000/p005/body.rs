
    fn assert_unsupported_transform_denials(root: &std::path::Path, transformed_manifest_ref: &str) {
        assert!(
            verify_manifest(root, transformed_manifest_ref)
                .expect_err("verify rejects unsupported transform")
                .to_string()
                .contains("unsupported chunk-store transform")
        );
        assert!(
            read_object(root, transformed_manifest_ref)
                .expect_err("read rejects unsupported transform")
                .to_string()
                .contains("unsupported chunk-store transform")
        );
        assert!(
            range_read(root, transformed_manifest_ref, 0, 1)
                .expect_err("range rejects unsupported transform")
                .to_string()
                .contains("unsupported chunk-store transform")
        );
        let transform_denials = list_receipt_refs(root)
            .expect("list transform receipts")
            .iter()
            .map(|receipt_ref| read_receipt(root, receipt_ref).expect("read transform receipt"))
            .filter(|receipt| receipt.decision == "deny")
            .collect::<Vec<_>>();
        assert!(transform_denials.iter().any(|receipt| receipt.operation == "chunk-verify"));
        assert!(transform_denials.iter().any(|receipt| receipt.operation == "fetch"));
        assert!(transform_denials.iter().any(|receipt| receipt.operation == "range-read"));
    }

    #[test]
    fn manifest_text_roundtrip_keeps_identity() {
        let root = temp_dir("chunk-roundtrip");
        let put = put_bytes(&root, "artifact", b"abcdef", 3).expect("put");
        let rendered = to_text(&put.manifest_value).expect("render manifest");
        let reparsed = crate::preserves_rail::parse_text(&rendered).expect("parse manifest");
        let parsed = parse_manifest_value(&reparsed, Some(&put.manifest_ref)).expect("parse manifest value");
        assert_eq!(parsed.chunks.len(), 2);
    }

    // Each case creates three isolated workspaces and runs put, duplicate put, range read, sync, pin, and
    // GC through durable storage, about 10-20 s per case on the shared CI builder (3 cases took 62 s under
    // parallel load). That cost is fixed per case: bounding the input to at most eight chunks did not
    // change it. Hegel's TooSlow check (fewer than 10 valid cases in 30 s) was therefore inherent. The run
    // is bounded explicitly instead: 2 derandomized cases with TooSlow suppressed. Two cases cannot be
    // relied on to draw a multi-chunk object, so every case stores at least two chunks and also reads the
    // range that spans the first chunk boundary.
    #[hegel::test(
        test_cases = 2,
        derandomize = true,
        database = None,
        suppress_health_check = [hegel::HealthCheck::TooSlow]
    )]
    fn hegel_chunk_store_determinism_range_resumable_and_no_dangling(tc: hegel::TestCase) {
        let chunk_size = tc.draw(hegel::generators::integers::<usize>().min_value(1).max_value(16));
        let bytes = tc.draw(hegel::generators::binary().min_size(chunk_size + 1).max_size(96));
        let root = temp_dir("chunk-hegel-root");
        let duplicate_root = temp_dir("chunk-hegel-duplicate");
        let sync_dest = temp_dir("chunk-hegel-sync-dest");

        let first = put_bytes(&root, "artifact", &bytes, chunk_size as u64).expect("put first");
        let duplicate = put_bytes(&duplicate_root, "artifact", &bytes, chunk_size as u64).expect("put duplicate");
        assert_eq!(first.manifest_ref, duplicate.manifest_ref);
        assert!(first.chunk_refs.len() >= 2);
        assert_eq!(read_object(&root, &first.manifest_ref).expect("read full").bytes, bytes);

        let offset = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(bytes.len()));
        let max_len = bytes.len().saturating_sub(offset);
        let length = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(max_len));
        let range = range_read(&root, &first.manifest_ref, offset as u64, length as u64).expect("range read");
        assert_eq!(range.bytes, bytes[offset..offset + length]);
        let spanning = range_read(&root, &first.manifest_ref, (chunk_size - 1) as u64, 2).expect("boundary range");
        assert_eq!(spanning.bytes, bytes[chunk_size - 1..chunk_size + 1]);

        let sync = sync_missing_chunks(&root, &sync_dest, &first.manifest_ref).expect("sync missing");
        assert_eq!(sync.missing_before.len(), first.chunk_refs.len());
        assert_eq!(read_object(&sync_dest, &first.manifest_ref).expect("read synced").bytes, bytes);
        let repeat = sync_missing_chunks(&root, &sync_dest, &first.manifest_ref).expect("repeat sync");
        assert!(repeat.missing_before.is_empty());
        assert!(repeat.fetched_chunks.is_empty());
        assert!(missing_chunks(&sync_dest, &first.manifest_ref).expect("missing after sync").is_empty());

        pin_manifest(&root, &first.manifest_ref).expect("pin manifest");
        let retention_evidence = retention_evidence(&root, "hegel-pinned");
        gc(&root, ChunkGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc pinned root");
        assert_eq!(read_object(&root, &first.manifest_ref).expect("read after gc").bytes, bytes);
        for chunk_ref in &first.chunk_refs {
            assert!(test_chunk_path(&root, chunk_ref).expect("chunk path").exists());
        }
    }
