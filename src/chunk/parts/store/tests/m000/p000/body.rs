    use super::*;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    #[test]
    fn fixed_v1_chunking_has_stable_manifest_identity() {
        let root = temp_dir("chunk-stable");
        let bytes = b"abcdefghij0123456789";
        let first = put_bytes(&root, "artifact", bytes, 4).expect("put first");
        let second = put_bytes(&root, "artifact", bytes, 4).expect("put second");
        assert_eq!(first.manifest_ref, second.manifest_ref);
        assert_eq!(second.dedup_hits, first.chunk_refs.len());
        let different_chunk_size = put_bytes(&root, "artifact", bytes, 5).expect("put different size");
        assert_ne!(first.manifest_ref, different_chunk_size.manifest_ref);
        let different_bytes = put_bytes(&root, "artifact", b"abcdefghij012345678X", 4).expect("put different bytes");
        assert_ne!(first.manifest_ref, different_bytes.manifest_ref);
    }

    #[hegel::test(test_cases = 32)]
    fn hegel_chunk_store_determinism_range_resumable_and_no_dangling(tc: hegel::TestCase) {
        let bytes = tc.draw(hegel::generators::binary().max_size(96));
        let chunk_size = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(16));
        let root = temp_dir("chunk-hegel-root");
        let duplicate_root = temp_dir("chunk-hegel-duplicate");
        let sync_dest = temp_dir("chunk-hegel-sync-dest");

        let first = put_bytes(&root, "artifact", &bytes, chunk_size).expect("put first");
        let duplicate = put_bytes(&duplicate_root, "artifact", &bytes, chunk_size).expect("put duplicate");
        assert_eq!(first.manifest_ref, duplicate.manifest_ref);
        assert_eq!(read_object(&root, &first.manifest_ref).expect("read full").bytes, bytes);

        let offset = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(bytes.len()));
        let max_len = bytes.len().saturating_sub(offset);
        let length = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(max_len));
        let range = range_read(&root, &first.manifest_ref, offset as u64, length as u64).expect("range read");
        assert_eq!(range.bytes, bytes[offset..offset + length]);

        let sync = sync_missing_chunks(&root, &sync_dest, &first.manifest_ref).expect("sync missing");
        assert_eq!(sync.missing_before.len(), first.chunk_refs.len());
        assert_eq!(read_object(&sync_dest, &first.manifest_ref).expect("read synced").bytes, bytes);
        let repeat = sync_missing_chunks(&root, &sync_dest, &first.manifest_ref).expect("repeat sync");
        assert!(repeat.missing_before.is_empty());
        assert!(repeat.fetched_chunks.is_empty());
        assert!(missing_chunks(&sync_dest, &first.manifest_ref).expect("missing after sync").is_empty());

        pin_manifest(&root, &first.manifest_ref).expect("pin manifest");
        let retention_evidence = retention_evidence(&root, "hegel-pinned");
        gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc pinned root");
        assert_eq!(read_object(&root, &first.manifest_ref).expect("read after gc").bytes, bytes);
        for chunk_ref in &first.chunk_refs {
            assert!(chunk_path(&root, chunk_ref).expect("chunk path").exists());
        }
    }

    #[test]
    fn chunks_deduplicate_across_objects_and_verify_ranges() {
        let root = temp_dir("chunk-dedup");
        let first = put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put first");
        let second = put_bytes(&root, "snapshot", b"aaaabbbbdddd", 4).expect("put second");
        assert_eq!(second.dedup_hits, 2);
        assert_eq!(list_chunk_refs(&root).expect("list chunks").len(), 4);
        let read = read_object(&root, &first.manifest_ref).expect("read object");
        assert_eq!(read.bytes, b"aaaabbbbcccc");
        let range = range_read(&root, &first.manifest_ref, 2, 8).expect("range read");
        assert_eq!(range.bytes, b"aabbbbcc");
        verify_manifest(&root, &first.manifest_ref).expect("verify first");
        verify_manifest(&root, &second.manifest_ref).expect("verify second");
    }

    #[test]
    fn sync_fetches_only_missing_chunks_and_preserves_manifest_identity() {
        let source = temp_dir("chunk-sync-source");
        let dest = temp_dir("chunk-sync-dest");
        let source_put = put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
        let _dest_seed = put_bytes(&dest, "artifact", b"aaaabbbb", 4).expect("seed destination");
        let sync = sync_missing_chunks(&source, &dest, &source_put.manifest_ref).expect("sync missing chunks");
        assert_eq!(sync.manifest_ref, source_put.manifest_ref);
        assert_eq!(sync.fetched_chunks, vec![source_put.chunk_refs[2].clone()]);
        assert_eq!(read_object(&dest, &source_put.manifest_ref).expect("read synced").bytes, b"aaaabbbbcccc");
        let repeat = sync_missing_chunks(&source, &dest, &source_put.manifest_ref).expect("repeat sync");
        assert!(repeat.fetched_chunks.is_empty());
        assert!(repeat.missing_before.is_empty());
    }

    #[test]
    fn chunk_availability_core_checks_index_and_partial_fetch_repair() {
        let root = temp_dir("chunk-availability-core");
        let put = put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put source");
        let manifest = read_manifest(&root, &put.manifest_ref).expect("read manifest");
        let available = put.chunk_refs.clone();
        let repaired = evaluate_chunk_availability(ChunkAvailabilityInput {
            manifest: &manifest,
            available_chunk_refs: &available,
            missing_chunk_refs: &[],
            indexed_available_refs: &available,
            indexed_missing_refs: &[],
            partial_fetch_missing_refs: std::slice::from_ref(&put.chunk_refs[2]),
            partial_fetch_fetched_refs: std::slice::from_ref(&put.chunk_refs[2]),
        });
        assert_eq!(repaired.decision, "pass");

        let missing = vec![put.chunk_refs[2].clone()];
        let mut incomplete_available = put.chunk_refs.clone();
        incomplete_available.pop();
        let stale_index = evaluate_chunk_availability(ChunkAvailabilityInput {
            manifest: &manifest,
            available_chunk_refs: &incomplete_available,
            missing_chunk_refs: &missing,
            indexed_available_refs: &available,
            indexed_missing_refs: &[],
            partial_fetch_missing_refs: &missing,
            partial_fetch_fetched_refs: &missing,
        });
        assert_eq!(stale_index.decision, "deny");
        assert!(stale_index.diagnostics.iter().any(|value| value == "chunk-missing"));
        assert!(stale_index.diagnostics.iter().any(|value| value == "index-availability-mismatch"));
        assert!(stale_index.diagnostics.iter().any(|value| value == "partial-fetch-repair-incomplete"));
    }

    #[test]
    fn iroh_adapter_publishes_and_fetches_missing_verified_chunks() {
        let source = temp_dir("chunk-iroh-source");
        let dest = temp_dir("chunk-iroh-dest");
        let iroh = temp_dir("chunk-iroh-blobs");
        let source_put = put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
        let published = publish_iroh_blobs(&source, &iroh, &source_put.manifest_ref, "node:test").expect("publish");
        assert_eq!(published.manifest_ref, source_put.manifest_ref);
        assert_eq!(published.manifest_blob_ref, source_put.manifest_ref);
        assert_eq!(published.chunk_blob_refs.len(), source_put.chunk_refs.len());
        let _dest_seed = put_bytes(&dest, "artifact", b"aaaa", 4).expect("seed destination");
        let fetched = fetch_iroh_blobs(&iroh, &dest, &published.ticket, Some(&source_put.manifest_ref), "peer:test")
            .expect("fetch");
        assert_eq!(fetched.manifest_ref, source_put.manifest_ref);
        assert_eq!(fetched.missing_before.len(), 2);
        assert_eq!(fetched.fetched_chunks, source_put.chunk_refs[1..].to_vec());
        assert_eq!(read_object(&dest, &source_put.manifest_ref).expect("read fetched").bytes, b"aaaabbbbcccc");
        let repeat = fetch_iroh_blobs(&iroh, &dest, &published.ticket, Some(&source_put.manifest_ref), "peer:test")
            .expect("repeat fetch");
        assert!(repeat.missing_before.is_empty());
        assert!(repeat.fetched_chunks.is_empty());
        let receipts = list_receipt_refs(&dest)
            .expect("list receipts")
            .iter()
            .map(|receipt_ref| read_receipt(&dest, receipt_ref).expect("read receipt"))
            .collect::<Vec<_>>();
        let has_pass_fetch_receipt = receipts
            .iter()
            .filter(|receipt| receipt.operation == "iroh-fetch")
            .any(|receipt| receipt.decision == "pass");
        assert!(has_pass_fetch_receipt);
        let wrong = fetch_iroh_blobs(&iroh, &dest, &published.ticket, Some("blake3:deadbeef"), "peer:test")
            .expect_err("wrong expected manifest fails");
        assert!(wrong.to_string().contains("expected blake3:deadbeef"));
    }

    #[test]
    fn lineage_chains_bind_manifest_publication_fetch_and_scope() {
        let source = temp_dir("chunk-lineage-source");
        let dest = temp_dir("chunk-lineage-dest");
        let iroh = temp_dir("chunk-lineage-iroh");
        let source_put = put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
        let published = publish_iroh_blobs(&source, &iroh, &source_put.manifest_ref, "node:test").expect("publish");
        let source_lineage = build_chunk_lineage(&source, &source_put.manifest_ref).expect("source lineage");
        assert_eq!(source_lineage.manifest_ref, source_put.manifest_ref);
        assert!(source_lineage.receipt_refs.len() >= 2);
        parse_chunk_lineage_value(&source_lineage.value).expect("parse source lineage");
        let source_text = to_text(&source_lineage.value).expect("render source lineage");
        assert!(source_text.contains("chunk-lineage"));
        assert!(source_text.contains("iroh-publish"));
        assert!(source_text.contains("lineage-no-global-head"));

        let fetched = fetch_iroh_blobs(&iroh, &dest, &published.ticket, Some(&source_put.manifest_ref), "peer:test")
            .expect("fetch");
        let dest_lineage = build_chunk_lineage(&dest, &fetched.manifest_ref).expect("dest lineage");
        assert_eq!(dest_lineage.manifest_ref, source_put.manifest_ref);
        parse_chunk_lineage_value(&dest_lineage.value).expect("parse dest lineage");
        assert!(to_text(&dest_lineage.value).expect("render dest lineage").contains("iroh-fetch"));

        let manifest = read_manifest(&source, &source_put.manifest_ref).expect("read manifest");
        let wrong_root = canonical_hash(&record("wrong-root", vec![string("lineage")])).expect("wrong root ref");
        let tampered_root =
            parse_text(&source_text.replacen(&manifest.root_ref, &wrong_root, 1)).expect("parse tampered root lineage");
        let error = parse_chunk_lineage_value(&tampered_root).expect_err("tampered root fails");
        assert!(["root", "scope"].iter().any(|needle| error.to_string().contains(needle)), "{error}");

        let tampered_ticket = parse_text(&source_text.replacen("iroh-local-chunk", "iroh-tampered-chunk", 1))
            .expect("parse tampered ticket lineage");
        let error = parse_chunk_lineage_value(&tampered_ticket).expect_err("tampered ticket fails");
        assert!(["payload", "receipt"].iter().any(|needle| error.to_string().contains(needle)), "{error}");

        let other_put = put_bytes(&source, "artifact", b"different", 4).expect("put other");
        let other_lineage = build_chunk_lineage(&source, &other_put.manifest_ref).expect("other lineage");
        assert_ne!(source_lineage.manifest_ref, other_lineage.manifest_ref);
        assert_ne!(source_lineage.link_refs.last(), other_lineage.link_refs.last());
    }

    #[test]
    fn verification_rejects_corrupted_missing_or_tampered_chunks() {
        let root = temp_dir("chunk-corrupt");
        let put = put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put");
        let manifest = read_manifest(&root, &put.manifest_ref).expect("read manifest");
        fs::write(chunk_path(&root, &manifest.chunks[1].chunk_ref).expect("chunk path"), b"zzzz").expect("corrupt");
        let error = verify_manifest(&root, &put.manifest_ref).expect_err("corruption fails");
        assert!(error.to_string().contains("chunk hash mismatch"));

        fs::remove_dir_all(root.join("chunks")).expect("remove chunks");
        fs::create_dir_all(root.join("chunks")).expect("recreate chunks");
        let put = put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put after corruption");
        let manifest = read_manifest(&root, &put.manifest_ref).expect("read manifest");
        fs::remove_file(chunk_path(&root, &manifest.chunks[0].chunk_ref).expect("chunk path")).expect("remove chunk");
        let missing = missing_chunks(&root, &put.manifest_ref).expect("missing chunks");
        assert_eq!(missing, vec![manifest.chunks[0].chunk_ref.clone()]);
        let error = read_object(&root, &put.manifest_ref).expect_err("missing chunk fails");
        assert!(["No such file", "io error"].iter().any(|needle| error.to_string().contains(needle)));
    }

    #[test]
    fn gc_preserves_pinned_manifest_chunks_and_removes_unpinned_content() {
        let root = temp_dir("chunk-gc");
        let pinned = put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put pinned");
        let unpinned = put_bytes(&root, "artifact", b"dddd", 4).expect("put unpinned");
        pin_manifest(&root, &pinned.manifest_ref).expect("pin manifest");
        let retention_evidence = retention_evidence(&root, "gc-remove");
        let apply_refs = gc_apply_refs(
            &root,
            std::slice::from_ref(&unpinned.manifest_ref),
            &unpinned.chunk_refs,
            &retention_evidence,
        );
        let gc = gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &apply_refs,
        })
        .expect("gc");
        assert!(gc.removed_manifests.contains(&unpinned.manifest_ref));
        assert!(gc.removed_chunks.contains(&unpinned.chunk_refs[0]));
        read_object(&root, &pinned.manifest_ref).expect("pinned object remains readable");
        assert!(read_manifest(&root, &unpinned.manifest_ref).is_err());
    }

    #[test]
    fn chunk_gc_requires_retention_pass_before_removal() {
        let root = temp_dir("chunk-retention-gc");
        let put = put_bytes(&root, "artifact", b"retained", 4).expect("put retained");
        let owner_ref = canonical_hash(&record("chunk-test-ref", vec![string("owner")])).expect("owner ref");
        let policy_refs = vec![canonical_hash(&record("chunk-test-ref", vec![string("policy")])).expect("policy ref")];
        let evidence_refs =
            vec![canonical_hash(&record("chunk-test-ref", vec![string("evidence")])).expect("evidence ref")];
        crate::retention::pin_object(&root, crate::retention::PinInput {
            object_ref: put.manifest_ref.clone(),
            object_kind: "chunk-manifest".to_string(),
            retention_class: crate::retention::CLASS_PUBLIC_ARTIFACT.to_string(),
            source: crate::retention::SOURCE_OPERATOR_HOLD.to_string(),
            reason: "operator hold".to_string(),
            owner_ref,
            expiry_ref: None,
            policy_refs,
            evidence_refs,
            has_authority: true,
        })
        .expect("retention pin");
        let retention_evidence = retention_evidence(&root, "retention-pin");
        let gc = gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_manifests.is_empty());
        assert!(gc.removed_chunks.is_empty());
        assert!(!gc.retention_receipt_refs.is_empty());
        read_object(&root, &put.manifest_ref).expect("retained object remains readable");
    }

    #[test]
    fn chunk_gc_denies_apply_refs_for_the_wrong_object_scope() {
        let root = temp_dir("chunk-retention-wrong-apply");
        let protected = put_bytes(&root, "artifact", b"protected", 4).expect("put protected");
        let wrong = put_bytes(&root, "artifact", b"wrong", 4).expect("put wrong");
        let retention_evidence = retention_evidence(&root, "wrong-apply");
        let wrong_apply_refs = gc_apply_refs(
            &root,
            std::slice::from_ref(&wrong.manifest_ref),
            &[],
            &retention_evidence,
        );
        let gc = gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &wrong_apply_refs,
        })
        .expect("gc denied by wrong apply");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_manifests.is_empty());
        assert!(gc.removed_chunks.is_empty());
        let receipt_text = to_text(&gc.receipt_value).expect("gc receipt text");
        assert!(receipt_text.contains("retention-gc-execute-apply-scope-mismatch"), "{receipt_text}");
        read_object(&root, &protected.manifest_ref).expect("protected object remains readable");
        read_object(&root, &wrong.manifest_ref).expect("wrong-scope object remains readable");
    }

    #[test]
    fn chunk_gc_denies_incomplete_reference_index_and_remote_uncertainty() {
        let root = temp_dir("chunk-retention-incomplete-remote");
        let put = put_bytes(&root, "artifact", b"remote", 3).expect("put remote-retained");
        let mut retention_evidence = retention_evidence(&root, "incomplete-remote");
        retention_evidence.remote_refs = vec![chunk_test_ref("remote", "incomplete-remote")];
        retention_evidence.is_reference_index_complete = false;
        let gc = gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &[],
        })
        .expect("gc denied");
        assert_eq!(gc.decision, "deny");
        assert!(gc.removed_manifests.is_empty());
        assert!(gc.removed_chunks.is_empty());
        read_object(&root, &put.manifest_ref).expect("remote-uncertain object remains readable");
    }
