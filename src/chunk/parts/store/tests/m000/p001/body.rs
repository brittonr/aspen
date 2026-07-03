
    #[test]
    fn redb_index_tracks_rebuild_pins_missing_chunks_and_partial_fetches() {
        let root = temp_dir("chunk-index");
        let put = put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put");
        let status = index_status(&root).expect("index status after put");
        assert_eq!(status.manifests, 1);
        assert_eq!(status.chunks, 2);
        assert_eq!(status.available_chunks, 2);
        assert_eq!(status.missing_chunks, 0);
        assert_eq!(status.receipts, 1);

        pin_manifest(&root, &put.manifest_ref).expect("pin manifest");
        let status = index_status(&root).expect("index status after pin");
        assert_eq!(status.manifest_pins, 1);
        let rebuild = rebuild_index(&root).expect("rebuild index");
        assert_eq!(rebuild.status.manifests, 1);
        assert_eq!(rebuild.status.chunks, 2);
        assert_eq!(rebuild.status.manifest_pins, 1);
        assert_eq!(rebuild.status.receipts, 3);

        let manifest = read_manifest(&root, &put.manifest_ref).expect("read manifest");
        fs::remove_file(chunk_path(&root, &manifest.chunks[0].chunk_ref).expect("chunk path")).expect("remove chunk");
        let missing = missing_chunks(&root, &put.manifest_ref).expect("missing chunks");
        assert_eq!(missing, vec![manifest.chunks[0].chunk_ref.clone()]);
        let status = index_status(&root).expect("index status after missing scan");
        assert_eq!(status.available_chunks, 1);
        assert_eq!(status.missing_chunks, 1);

        let source = temp_dir("chunk-index-source");
        let dest = temp_dir("chunk-index-dest");
        let source_put = put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
        put_bytes(&dest, "artifact", b"aaaa", 4).expect("seed dest");
        let sync = sync_missing_chunks(&source, &dest, &source_put.manifest_ref).expect("sync");
        assert_eq!(sync.missing_before.len(), 2);
        assert_eq!(sync.fetched_chunks.len(), 2);
        let status = index_status(&dest).expect("dest index status");
        assert_eq!(status.partial_fetches, 1);
        assert_eq!(status.missing_chunks, 0);
        assert_eq!(status.available_chunks, 3);
    }

    #[test]
    fn receipt_index_covers_pass_denial_dedup_and_tombstone_evidence() {
        let root = temp_dir("chunk-receipts");
        let put = put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put");
        put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("dedup put");
        verify_manifest(&root, &put.manifest_ref).expect("verify");
        read_object(&root, &put.manifest_ref).expect("fetch");
        range_read(&root, &put.manifest_ref, 1, 5).expect("range");
        pin_manifest(&root, &put.manifest_ref).expect("pin");
        unpin_manifest(&root, &put.manifest_ref).expect("unpin");
        let retention_evidence = retention_evidence(&root, "receipt-index");
        let apply_refs =
            gc_apply_refs(&root, std::slice::from_ref(&put.manifest_ref), &put.chunk_refs, &retention_evidence);
        gc(&root, ChunkStoreGcInput {
            dry_run: false,
            retention_evidence: &retention_evidence,
            apply_refs: &apply_refs,
        })
        .expect("gc");

        let before_rebuild = list_receipt_refs(&root).expect("list receipts");
        let receipts = before_rebuild
            .iter()
            .map(|receipt_ref| read_receipt(&root, receipt_ref).expect("read receipt"))
            .collect::<Vec<_>>();
        for receipt in &receipts {
            assert_eq!(canonical_hash(&receipt.value).expect("receipt ref"), receipt.receipt_ref);
            parse_receipt_value(&receipt.value, Some(&receipt.receipt_ref)).expect("validate receipt");
        }
        let operations = receipts.iter().map(|receipt| receipt.operation.as_str()).collect::<OrderedSet<_>>();
        for expected in [
            "manifest-create",
            "dedup-hit",
            "chunk-verify",
            "fetch",
            "range-read",
            "pin",
            "unpin",
            "gc",
            "tombstone",
        ] {
            assert!(operations.contains(expected), "missing receipt operation {expected}");
        }

        rebuild_index(&root).expect("rebuild preserves receipt table");
        let after_rebuild = list_receipt_refs(&root).expect("list receipts after rebuild");
        for receipt_ref in before_rebuild {
            assert!(after_rebuild.contains(&receipt_ref), "receipt {receipt_ref} survived rebuild");
        }

        let denial_root = temp_dir("chunk-denial-receipts");
        let denial_put = put_bytes(&denial_root, "artifact", b"aaaabbbb", 4).expect("put denial fixture");
        let denial_manifest = read_manifest(&denial_root, &denial_put.manifest_ref).expect("read denial manifest");
        fs::write(chunk_path(&denial_root, &denial_manifest.chunks[0].chunk_ref).expect("chunk path"), b"zzzz")
            .expect("corrupt chunk");
        verify_manifest(&denial_root, &denial_put.manifest_ref).expect_err("corrupt verify denied");
        range_read(&denial_root, &denial_put.manifest_ref, 99, 1).expect_err("range denied");
        let missing_chunk_ref =
            canonical_hash(&record("chunk-test-ref", vec![string("missing-pin")])).expect("missing pin ref");
        pin_chunk(&denial_root, &missing_chunk_ref).expect_err("missing chunk pin denied");
        let denials = list_receipt_refs(&denial_root)
            .expect("list denial receipts")
            .iter()
            .map(|receipt_ref| read_receipt(&denial_root, receipt_ref).expect("read denial receipt"))
            .filter(|receipt| receipt.decision == "deny")
            .collect::<Vec<_>>();
        assert!(denials.iter().any(|receipt| receipt.operation == "chunk-verify"));
        assert!(denials.iter().any(|receipt| receipt.operation == "range-read"));
        assert!(denials.iter().any(|receipt| receipt.operation == "pin"));
    }

    #[test]
    fn confidentiality_and_transform_modes_fail_closed_until_supported() {
        assert_confidential_write_denials();
        let (root, transformed_manifest_ref) = write_unsupported_manifest();
        assert_unsupported_transform_denials(&root, &transformed_manifest_ref);
    }

    #[test]
    fn manifest_parser_rejects_invalid_content_refs() {
        let root = temp_dir("chunk-invalid-refs");
        let put = put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put");
        let manifest = read_manifest(&root, &put.manifest_ref).expect("read manifest");
        let chunk_size = usize::try_from(manifest.chunk_size).expect("test chunk size fits usize");
        let chunk_values = manifest
            .chunks
            .iter()
            .map(|chunk| chunk_ref_value(&chunk.chunk_ref, chunk.length, chunk_size, &manifest.transforms))
            .collect::<Vec<_>>();
        let uppercase_metadata_ref = manifest.metadata_ref.to_ascii_uppercase();
        let uppercase_manifest_value = manifest_value(&ChunkManifestValueInput {
            object_kind: &manifest.object_kind,
            total_len: manifest.total_len,
            chunk_size: manifest.chunk_size,
            transforms: &manifest.transforms,
            metadata_ref: &uppercase_metadata_ref,
            policy_refs: &manifest.policy_refs,
            chunks: &chunk_values,
            root_ref: &manifest.root_ref,
            evidence_refs: &manifest.evidence_refs,
        });
        assert!(
            parse_manifest_value(&uppercase_manifest_value, None)
                .expect_err("uppercase metadata ref is rejected")
                .to_string()
                .contains("metadata-ref is invalid")
        );

        let non_blake3_policy_refs = vec!["sha256:abc123".to_string()];
        let non_blake3_manifest_value = manifest_value(&ChunkManifestValueInput {
            object_kind: &manifest.object_kind,
            total_len: manifest.total_len,
            chunk_size: manifest.chunk_size,
            transforms: &manifest.transforms,
            metadata_ref: &manifest.metadata_ref,
            policy_refs: &non_blake3_policy_refs,
            chunks: &chunk_values,
            root_ref: &manifest.root_ref,
            evidence_refs: &manifest.evidence_refs,
        });
        assert!(
            parse_manifest_value(&non_blake3_manifest_value, None)
                .expect_err("non-blake3 policy ref is rejected")
                .to_string()
                .contains("policy-ref is invalid")
        );

        let mut invalid_chunk_values = chunk_values.clone();
        invalid_chunk_values[0] = chunk_ref_value(
            "blake3:not-hex",
            manifest.chunks[0].length,
            chunk_size,
            &manifest.transforms,
        );
        let malformed_chunk_manifest_value = manifest_value(&ChunkManifestValueInput {
            object_kind: &manifest.object_kind,
            total_len: manifest.total_len,
            chunk_size: manifest.chunk_size,
            transforms: &manifest.transforms,
            metadata_ref: &manifest.metadata_ref,
            policy_refs: &manifest.policy_refs,
            chunks: &invalid_chunk_values,
            root_ref: &manifest.root_ref,
            evidence_refs: &manifest.evidence_refs,
        });
        assert!(
            parse_manifest_value(&malformed_chunk_manifest_value, None)
                .expect_err("malformed chunk ref is rejected")
                .to_string()
                .contains("chunk ref hash is invalid")
        );
    }

    fn assert_confidential_write_denials() {
        let confidential_root = temp_dir("chunk-confidential-deny");
        let metadata = record("chunk-metadata-v1", vec![record("object-kind", vec![string("artifact")])]);
        let mut confidential_without_commitment = ChunkTransforms::public_plaintext();
        confidential_without_commitment.confidentiality = "confidential".to_string();
        let error = put_bytes_with_transforms(&PutBytesWithTransformsInput {
            root: &confidential_root,
            object_kind: "artifact",
            bytes: b"secret bytes",
            chunk_size: 4,
            metadata: &metadata,
            policy_refs: &[],
            transforms: &confidential_without_commitment,
        })
        .expect_err("confidential write without commitment is denied");
        assert!(error.to_string().contains("protected commitment"));
        let denial_receipts = list_receipt_refs(&confidential_root)
            .expect("list confidential receipts")
            .iter()
            .map(|receipt_ref| read_receipt(&confidential_root, receipt_ref).expect("read receipt"))
            .filter(|receipt| receipt.decision == "deny" && receipt.operation == "manifest-create")
            .collect::<Vec<_>>();
        assert_eq!(denial_receipts.len(), 1);
        let protected_commitment_ref = content_ref_from_bytes(b"protected-commitment-fixture");
        let protected_shape = ChunkTransforms::confidential_protected(&protected_commitment_ref);
        let protected_error = put_bytes_with_transforms(&PutBytesWithTransformsInput {
            root: &confidential_root,
            object_kind: "artifact",
            bytes: b"secret bytes",
            chunk_size: 4,
            metadata: &metadata,
            policy_refs: &[],
            transforms: &protected_shape,
        })
        .expect_err("protected confidential writes are denied until encryption exists");
        assert!(protected_error.to_string().contains("protected encryption implementation"));
    }

    fn write_unsupported_manifest() -> (std::path::PathBuf, String) {
        let root = temp_dir("chunk-transform-unsupported");
        let put = put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put public");
        let public_manifest = read_manifest(&root, &put.manifest_ref).expect("read manifest");
        let unsupported = ChunkTransforms {
            compression: "zstd-placeholder".to_string(),
            encryption: "none".to_string(),
            ordering: "compress".to_string(),
            confidentiality: "public".to_string(),
            protected_commitment_ref: None,
        };
        let transformed_chunks = public_manifest
            .chunks
            .iter()
            .map(|chunk| ChunkRef {
                chunk_ref: chunk.chunk_ref.clone(),
                length: chunk.length,
                domain: chunk.domain.clone(),
                chunker: chunk.chunker.clone(),
                transforms: unsupported.clone(),
            })
            .collect::<Vec<_>>();
        let transformed_chunk_values = transformed_chunks
            .iter()
            .map(|chunk| {
                chunk_ref_value(
                    &chunk.chunk_ref,
                    chunk.length,
                    usize::try_from(public_manifest.chunk_size).expect("test chunk size fits usize"),
                    &unsupported,
                )
            })
            .collect::<Vec<_>>();
        let transformed_root_ref = chunk_root_ref(&transformed_chunks).expect("chunk root");
        let transformed_manifest_value = manifest_value(&ChunkManifestValueInput {
            object_kind: &public_manifest.object_kind,
            total_len: public_manifest.total_len,
            chunk_size: public_manifest.chunk_size,
            transforms: &unsupported,
            metadata_ref: &public_manifest.metadata_ref,
            policy_refs: &public_manifest.policy_refs,
            chunks: &transformed_chunk_values,
            root_ref: &transformed_root_ref,
            evidence_refs: &public_manifest.evidence_refs,
        });
        let transformed_manifest_ref = canonical_hash(&transformed_manifest_value).expect("manifest ref");
        fs::write(
            manifest_path(&root, &transformed_manifest_ref).expect("manifest path"),
            canonical_bytes(&transformed_manifest_value).expect("manifest bytes"),
        )
        .expect("write transformed manifest");
        let parsed = read_manifest(&root, &transformed_manifest_ref).expect("parse transformed manifest");
        assert_eq!(parsed.transforms, unsupported);
        (root, transformed_manifest_ref)
    }

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
