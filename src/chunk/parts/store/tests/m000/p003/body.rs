
    const IROH_ADOPTION_TEST_CHUNK_SIZE: u64 = 4;
    const IROH_ADOPTION_RANGE_OFFSET: u64 = 0;
    const IROH_ADOPTION_RANGE_LENGTH: u64 = 4;

    fn adoption_chunk_ref(label: &str) -> String {
        content_ref_from_bytes(label.as_bytes())
    }

    fn adoption_manifest() -> (std::path::PathBuf, ChunkManifest, Vec<(String, Vec<u8>)>) {
        let root = temp_dir("chunk-iroh-adoption");
        let body = b"abcdefgh";
        let put = put_bytes(&root, "artifact", body, IROH_ADOPTION_TEST_CHUNK_SIZE).expect("put bytes");
        let manifest = parse_manifest_value(&put.manifest_value, Some(&put.manifest_ref)).expect("manifest");
        let chunk_size = usize::try_from(IROH_ADOPTION_TEST_CHUNK_SIZE).expect("chunk size fits usize");
        let bytes = manifest
            .chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| {
                let start = index * chunk_size;
                let end = (start + chunk_size).min(body.len());
                (chunk.chunk_ref.clone(), body[start..end].to_vec())
            })
            .collect::<Vec<_>>();
        (root, manifest, bytes)
    }

    #[test]
    fn chunk_traversal_sync_plans_partitioned_missing_fetches() {
        let (_root, manifest, _bytes) = adoption_manifest();
        let present = vec![manifest.chunks[0].chunk_ref.clone()];
        let peers = vec!["peer:a".to_string(), "peer:b".to_string()];
        let plan = plan_chunk_traversal_sync(&ChunkTraversalSyncInput {
            manifest: &manifest,
            verified_present_refs: &present,
            candidate_peers: &peers,
            strategy: CHUNK_SYNC_PARTITIONED_LEAF,
        })
        .expect("chunk sync plan");
        assert_eq!(plan.decision, "pass");
        let expected_missing_count = manifest.chunks.len() - present.len();
        assert_eq!(plan.already_present_refs, present);
        assert_eq!(plan.missing_refs.len(), expected_missing_count);
        assert_eq!(plan.fetch_effects.len(), plan.missing_refs.len());

        let accepted = validate_chunk_sync_response(&ChunkSyncResponseInput {
            plan: &plan,
            manifest_ref: &manifest.manifest_ref,
            returned_refs: &plan.missing_refs,
        })
        .expect("chunk response pass");
        assert_eq!(accepted.decision, "pass");

        let unexpected = adoption_chunk_ref("unexpected-chunk");
        let denied = validate_chunk_sync_response(&ChunkSyncResponseInput {
            plan: &plan,
            manifest_ref: &manifest.manifest_ref,
            returned_refs: &[plan.missing_refs[0].clone(), unexpected],
        })
        .expect("chunk response deny");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("unexpected")));
    }

    #[test]
    fn remote_byte_source_hint_verifies_range_before_exposure() {
        let (_root, manifest, bytes) = adoption_manifest();
        let outboard_ref = adoption_chunk_ref("outboard");
        let evidence_ref = adoption_chunk_ref("remote-hint-evidence");
        let hint = remote_byte_source_hint(&RemoteByteSourceHintInput {
            manifest_ref: &manifest.manifest_ref,
            location: "https://example.invalid/object",
            outboard_ref: &outboard_ref,
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("remote hint");
        let pass = verify_remote_range_readback(&RemoteRangeReadbackInput {
            hint: &hint,
            manifest: &manifest,
            offset: IROH_ADOPTION_RANGE_OFFSET,
            length: IROH_ADOPTION_RANGE_LENGTH,
            chunk_bytes: &bytes,
        })
        .expect("remote range pass");
        assert_eq!(pass.decision, "pass");
        assert!(!pass.bytes.is_empty());

        let mut corrupt = bytes.clone();
        corrupt[0].1 = b"xxxx".to_vec();
        let denied = verify_remote_range_readback(&RemoteRangeReadbackInput {
            hint: &hint,
            manifest: &manifest,
            offset: IROH_ADOPTION_RANGE_OFFSET,
            length: IROH_ADOPTION_RANGE_LENGTH,
            chunk_bytes: &corrupt,
        })
        .expect("remote range deny");
        assert_eq!(denied.decision, "deny");
        assert!(denied.bytes.is_empty());
    }
