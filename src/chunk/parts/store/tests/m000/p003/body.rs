
    const IROH_ADOPTION_TEST_CHUNK_SIZE: u64 = 4;
    const IROH_ADOPTION_RANGE_OFFSET: u64 = 0;
    const IROH_ADOPTION_RANGE_LENGTH: u64 = 4;

    fn adoption_chunk_ref(label: &str) -> String {
        content_ref_from_bytes(label.as_bytes())
    }

    fn adoption_manifest() -> (crate::test_support::ProcessWorkspace, ChunkManifest, Vec<(String, Vec<u8>)>) {
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

    #[test]
    fn chunk_manifest_count_preflight_bounds_remote_readback() {
        let (_root, mut manifest, bytes) = adoption_manifest();
        let chunk = manifest.chunks[0].clone();
        manifest.chunks.resize(MAX_CHUNK_STORE_CHUNKS, chunk);
        manifest.total_len = manifest.chunk_size * MAX_CHUNK_STORE_CHUNKS as u64;
        let hint = remote_byte_source_hint(&RemoteByteSourceHintInput {
            manifest_ref: &manifest.manifest_ref,
            location: "https://example.invalid/object",
            outboard_ref: &adoption_chunk_ref("outboard"),
            evidence_refs: &[],
        })
        .expect("remote hint");
        let admitted = verify_remote_range_readback(&RemoteRangeReadbackInput {
            hint: &hint,
            manifest: &manifest,
            offset: 0,
            length: IROH_ADOPTION_RANGE_LENGTH,
            chunk_bytes: &bytes[..1],
        })
        .expect("exact-limit manifest is admitted");
        assert_eq!(admitted.decision, "pass");
        assert_eq!(admitted.bytes, b"abcd");

        let chunk = manifest.chunks[0].clone();
        manifest.chunks.push(chunk);
        let error = match verify_remote_range_readback(&RemoteRangeReadbackInput {
            hint: &hint,
            manifest: &manifest,
            offset: 0,
            length: IROH_ADOPTION_RANGE_LENGTH,
            chunk_bytes: &bytes[..1],
        }) {
            Ok(_) => panic!("oversized caller-built manifest must be denied before range collection"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("chunk manifest chunks count 65537 exceeds maximum 65536"));
    }

    #[test]
    fn chunk_manifest_count_preflight_bounds_traversal_before_strategy() {
        let (_root, mut manifest, _bytes) = adoption_manifest();
        let peers = ["peer:a".to_string()];
        let original_ref = std::mem::replace(&mut manifest.chunks[0].chunk_ref, "invalid".to_string());
        let error = match plan_chunk_traversal_sync(&ChunkTraversalSyncInput {
            manifest: &manifest,
            verified_present_refs: &[],
            candidate_peers: &peers,
            strategy: "unsupported",
        }) {
            Ok(_) => panic!("admitted malformed chunk ref must be denied before strategy"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("content ref must start with"));
        manifest.chunks[0].chunk_ref = original_ref;
        let chunk = manifest.chunks[0].clone();
        manifest.chunks.resize(MAX_CHUNK_STORE_CHUNKS, chunk);
        let error = match plan_chunk_traversal_sync(&ChunkTraversalSyncInput {
            manifest: &manifest,
            verified_present_refs: &[],
            candidate_peers: &peers,
            strategy: "unsupported",
        }) {
            Ok(_) => panic!("unsupported strategy must be denied at the admitted count"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("unsupported chunk traversal sync strategy"));

        let chunk = manifest.chunks[0].clone();
        manifest.chunks.push(chunk);
        let error = match plan_chunk_traversal_sync(&ChunkTraversalSyncInput {
            manifest: &manifest,
            verified_present_refs: &[],
            candidate_peers: &peers,
            strategy: "unsupported",
        }) {
            Ok(_) => panic!("oversized caller-built manifest must be denied before strategy or allocation"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("chunk manifest chunks count 65537 exceeds maximum 65536"));
    }

    #[test]
    fn remote_readback_preflights_supplied_chunk_count() {
        let (_root, manifest, _bytes) = adoption_manifest();
        let hint = remote_byte_source_hint(&RemoteByteSourceHintInput {
            manifest_ref: &manifest.manifest_ref,
            location: "https://example.invalid/object",
            outboard_ref: &adoption_chunk_ref("outboard"),
            evidence_refs: &[],
        })
        .expect("remote hint");
        let irrelevant = adoption_chunk_ref("irrelevant");
        let mut supplied = vec![(irrelevant.clone(), Vec::new()); MAX_CHUNK_STORE_CHUNKS - 1];
        supplied.push((manifest.chunks[0].chunk_ref.clone(), b"abcd".to_vec()));
        let exact = verify_remote_range_readback(&RemoteRangeReadbackInput {
            hint: &hint,
            manifest: &manifest,
            offset: 0,
            length: IROH_ADOPTION_RANGE_LENGTH,
            chunk_bytes: &supplied,
        })
        .expect("exactly bounded supplied tuples");
        assert_eq!(exact.decision, "pass");
        assert_eq!(exact.bytes, b"abcd");
        let range_error = match verify_remote_range_readback(&RemoteRangeReadbackInput {
            hint: &hint,
            manifest: &manifest,
            offset: u64::MAX,
            length: IROH_ADOPTION_RANGE_LENGTH,
            chunk_bytes: &supplied,
        }) {
            Ok(_) => panic!("range overflow must precede admitted tuple lookup"),
            Err(error) => error,
        };
        assert!(range_error.to_string().contains("remote byte-source range end overflow"));

        supplied.push((irrelevant, Vec::new()));
        let error = match verify_remote_range_readback(&RemoteRangeReadbackInput {
            hint: &hint,
            manifest: &manifest,
            offset: 0,
            length: IROH_ADOPTION_RANGE_LENGTH,
            chunk_bytes: &supplied,
        }) {
            Ok(_) => panic!("65,537 supplied tuples must be rejected before lookup"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("remote byte-source supplied chunks count 65537 exceeds maximum 65536"));
    }

    #[test]
    fn remote_readback_uses_last_supplied_duplicate_for_verification() {
        let (_root, manifest, _bytes) = adoption_manifest();
        let hint = remote_byte_source_hint(&RemoteByteSourceHintInput {
            manifest_ref: &manifest.manifest_ref,
            location: "https://example.invalid/object",
            outboard_ref: &adoption_chunk_ref("outboard"),
            evidence_refs: &[],
        })
        .expect("remote hint");
        let required_ref = manifest.chunks[0].chunk_ref.clone();
        let mut supplied = [(required_ref.clone(), b"abcd".to_vec()), (required_ref, b"xxxx".to_vec())];
        let denied = verify_remote_range_readback(&RemoteRangeReadbackInput {
            hint: &hint,
            manifest: &manifest,
            offset: 0,
            length: IROH_ADOPTION_RANGE_LENGTH,
            chunk_bytes: &supplied,
        })
        .expect("last duplicate is corrupt");
        assert_eq!(denied.decision, "deny");
        assert!(denied.bytes.is_empty());
        supplied.swap(0, 1);
        let admitted = verify_remote_range_readback(&RemoteRangeReadbackInput {
            hint: &hint,
            manifest: &manifest,
            offset: 0,
            length: IROH_ADOPTION_RANGE_LENGTH,
            chunk_bytes: &supplied,
        })
        .expect("last duplicate is verified");
        assert_eq!(admitted.decision, "pass");
        assert_eq!(admitted.bytes, b"abcd");
    }
