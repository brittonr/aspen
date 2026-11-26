#[test]
fn fixed_v1_chunking_has_stable_manifest_identity() {
    // r[verify molten.chunk_store.cap_std_conversion_validation]
    let root_path = super::temp_dir("chunk-stable");
    let root = crate::chunk_store::CapabilityChunkRoot::open(&root_path).expect("open chunk capability root");
    let bytes = b"abcdefghij0123456789";
    let first = crate::chunk_store::put_bytes_with_root(&root, "artifact", bytes, 4).expect("put first");
    let second = crate::chunk_store::put_bytes_with_root(&root, "artifact", bytes, 4).expect("put second");
    assert_eq!(first.manifest_ref, second.manifest_ref);
    assert_eq!(second.dedup_hits, first.chunk_refs.len());
    let different_chunk_size =
        crate::chunk_store::put_bytes_with_root(&root, "artifact", bytes, 5).expect("put different size");
    assert_ne!(first.manifest_ref, different_chunk_size.manifest_ref);
    let different_bytes = crate::chunk_store::put_bytes_with_root(&root, "artifact", b"abcdefghij012345678X", 4)
        .expect("put different bytes");
    assert_ne!(first.manifest_ref, different_bytes.manifest_ref);
}

// Each case creates three isolated workspaces and runs put, duplicate put, range read, sync, pin,
// and GC through durable storage, about 10-20 s per case on the shared CI builder (3 cases took 62
// s under parallel load). That cost is fixed per case: bounding the input to at most eight chunks
// did not change it. Hegel's TooSlow check (fewer than 10 valid cases in 30 s) was therefore
// inherent. The run is bounded explicitly instead: 2 derandomized cases with TooSlow suppressed.
// Two cases cannot be relied on to draw a multi-chunk object, so every case stores at least two
// chunks and also reads the range that spans the first chunk boundary.
#[hegel::test(
    test_cases = 2,
    derandomize = true,
    database = None,
    suppress_health_check = [hegel::HealthCheck::TooSlow]
)]
fn hegel_chunk_store_determinism_range_resumable_and_no_dangling(tc: hegel::TestCase) {
    let chunk_size = tc.draw(hegel::generators::integers::<usize>().min_value(1).max_value(16));
    let bytes = tc.draw(hegel::generators::binary().min_size(chunk_size + 1).max_size(96));
    let root = super::temp_dir("chunk-hegel-root");
    let duplicate_root = super::temp_dir("chunk-hegel-duplicate");
    let sync_dest = super::temp_dir("chunk-hegel-sync-dest");

    let first = crate::chunk_store::put_bytes(&root, "artifact", &bytes, chunk_size as u64).expect("put first");
    let duplicate =
        crate::chunk_store::put_bytes(&duplicate_root, "artifact", &bytes, chunk_size as u64).expect("put duplicate");
    assert_eq!(first.manifest_ref, duplicate.manifest_ref);
    assert!(first.chunk_refs.len() >= 2);
    assert_eq!(crate::chunk_store::read_object(&root, &first.manifest_ref).expect("read full").bytes, bytes);

    let offset = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(bytes.len()));
    let max_len = bytes.len().saturating_sub(offset);
    let length = tc.draw(hegel::generators::integers::<usize>().min_value(0).max_value(max_len));
    let range =
        crate::chunk_store::range_read(&root, &first.manifest_ref, offset as u64, length as u64).expect("range read");
    assert_eq!(range.bytes, bytes[offset..offset + length]);
    let spanning =
        crate::chunk_store::range_read(&root, &first.manifest_ref, (chunk_size - 1) as u64, 2).expect("boundary range");
    assert_eq!(spanning.bytes, bytes[chunk_size - 1..chunk_size + 1]);

    let sync = crate::chunk_store::sync_missing_chunks(&root, &sync_dest, &first.manifest_ref).expect("sync missing");
    assert_eq!(sync.missing_before.len(), first.chunk_refs.len());
    assert_eq!(crate::chunk_store::read_object(&sync_dest, &first.manifest_ref).expect("read synced").bytes, bytes);
    let repeat = crate::chunk_store::sync_missing_chunks(&root, &sync_dest, &first.manifest_ref).expect("repeat sync");
    assert!(repeat.missing_before.is_empty());
    assert!(repeat.fetched_chunks.is_empty());
    assert!(
        crate::chunk_store::missing_chunks(&sync_dest, &first.manifest_ref)
            .expect("missing after sync")
            .is_empty()
    );

    crate::chunk_store::pin_manifest(&root, &first.manifest_ref).expect("pin manifest");
    let retention_evidence = super::retention_evidence(&root, "hegel-pinned");
    crate::chunk_store::gc(&root, crate::chunk_store::ChunkGcInput {
        dry_run: false,
        retention_evidence: &retention_evidence,
        apply_refs: &[],
    })
    .expect("gc pinned root");
    assert_eq!(crate::chunk_store::read_object(&root, &first.manifest_ref).expect("read after gc").bytes, bytes);
    for chunk_ref in &first.chunk_refs {
        assert!(super::test_chunk_path(&root, chunk_ref).expect("chunk path").exists());
    }
}

#[test]
fn chunks_deduplicate_across_objects_and_verify_ranges() {
    let root = super::temp_dir("chunk-dedup");
    let first = crate::chunk_store::put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put first");
    let second = crate::chunk_store::put_bytes(&root, "snapshot", b"aaaabbbbdddd", 4).expect("put second");
    assert_eq!(second.dedup_hits, 2);
    assert_eq!(crate::chunk_store::list_chunk_refs(&root).expect("list chunks").len(), 4);
    let read = crate::chunk_store::read_object(&root, &first.manifest_ref).expect("read object");
    assert_eq!(read.bytes, b"aaaabbbbcccc");
    let range = crate::chunk_store::range_read(&root, &first.manifest_ref, 2, 8).expect("range read");
    assert_eq!(range.bytes, b"aabbbbcc");
    crate::chunk_store::verify_manifest(&root, &first.manifest_ref).expect("verify first");
    crate::chunk_store::verify_manifest(&root, &second.manifest_ref).expect("verify second");
}

#[test]
fn chunk_availability_core_checks_index_and_partial_fetch_repair() {
    let root = super::temp_dir("chunk-availability-core");
    let put = crate::chunk_store::put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put source");
    let manifest = crate::chunk_store::read_manifest(&root, &put.manifest_ref).expect("read manifest");
    let available = put.chunk_refs.clone();
    let repaired = crate::chunk_store::evaluate_chunk_availability(crate::chunk_store::ChunkAvailabilityInput {
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
    let stale_index = crate::chunk_store::evaluate_chunk_availability(crate::chunk_store::ChunkAvailabilityInput {
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
fn verification_rejects_corrupted_missing_or_tampered_chunks() {
    let root = super::temp_dir("chunk-corrupt");
    let put = crate::chunk_store::put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put");
    let manifest = crate::chunk_store::read_manifest(&root, &put.manifest_ref).expect("read manifest");
    std::fs::write(super::test_chunk_path(&root, &manifest.chunks[1].chunk_ref).expect("chunk path"), b"zzzz")
        .expect("corrupt");
    let error = crate::chunk_store::verify_manifest(&root, &put.manifest_ref).expect_err("corruption fails");
    assert!(error.to_string().contains("chunk hash mismatch"));

    std::fs::remove_dir_all(root.join("chunks")).expect("remove chunks");
    std::fs::create_dir_all(root.join("chunks")).expect("recreate chunks");
    let put = crate::chunk_store::put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put after corruption");
    let manifest = crate::chunk_store::read_manifest(&root, &put.manifest_ref).expect("read manifest");
    std::fs::remove_file(super::test_chunk_path(&root, &manifest.chunks[0].chunk_ref).expect("chunk path"))
        .expect("remove chunk");
    let missing = crate::chunk_store::missing_chunks(&root, &put.manifest_ref).expect("missing chunks");
    assert_eq!(missing, vec![manifest.chunks[0].chunk_ref.clone()]);
    let error = crate::chunk_store::read_object(&root, &put.manifest_ref).expect_err("missing chunk fails");
    assert!(error.to_string().contains("does not exist"), "{error}");
}

#[test]
fn manifest_parser_rejects_invalid_content_refs() {
    let root = super::temp_dir("chunk-invalid-refs");
    let put = crate::chunk_store::put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put");
    let manifest = crate::chunk_store::read_manifest(&root, &put.manifest_ref).expect("read manifest");
    let chunk_size = usize::try_from(manifest.chunk_size).expect("test chunk size fits usize");
    let chunk_values = manifest
        .chunks
        .iter()
        .map(|chunk| {
            crate::blocks::wire::encode::chunk_ref_value(
                &chunk.chunk_ref,
                chunk.length,
                chunk_size,
                &manifest.transforms,
            )
        })
        .collect::<Vec<_>>();
    let uppercase_metadata_ref = manifest.metadata_ref.to_ascii_uppercase();
    let uppercase_manifest_value =
        manifest_value_with(&manifest, &uppercase_metadata_ref, &manifest.policy_refs, &chunk_values);
    assert!(
        crate::chunk_store::parse_manifest_value(&uppercase_manifest_value, None)
            .expect_err("uppercase metadata ref is rejected")
            .to_string()
            .contains("metadata-ref is invalid")
    );

    let non_blake3_policy_refs = vec!["sha256:abc123".to_string()];
    let non_blake3_manifest_value =
        manifest_value_with(&manifest, &manifest.metadata_ref, &non_blake3_policy_refs, &chunk_values);
    assert!(
        crate::chunk_store::parse_manifest_value(&non_blake3_manifest_value, None)
            .expect_err("non-blake3 policy ref is rejected")
            .to_string()
            .contains("policy-ref is invalid")
    );

    let mut invalid_chunk_values = chunk_values.clone();
    invalid_chunk_values[0] = crate::blocks::wire::encode::chunk_ref_value(
        "blake3:not-hex",
        manifest.chunks[0].length,
        chunk_size,
        &manifest.transforms,
    );
    let malformed_chunk_manifest_value =
        manifest_value_with(&manifest, &manifest.metadata_ref, &manifest.policy_refs, &invalid_chunk_values);
    assert!(
        crate::chunk_store::parse_manifest_value(&malformed_chunk_manifest_value, None)
            .expect_err("malformed chunk ref is rejected")
            .to_string()
            .contains("chunk ref hash is invalid")
    );
}

/// Re-encodes `manifest` with the given metadata ref, policy refs, and chunk values.
fn manifest_value_with(
    manifest: &crate::chunk_store::ChunkManifest,
    metadata_ref: &str,
    policy_refs: &[String],
    chunks: &[preserves::IOValue],
) -> preserves::IOValue {
    crate::blocks::wire::encode::manifest_value(&crate::blocks::wire::encode::ChunkManifestValueInput {
        object_kind: &manifest.object_kind,
        total_len: manifest.total_len,
        chunk_size: manifest.chunk_size,
        transforms: &manifest.transforms,
        metadata_ref,
        policy_refs,
        chunks,
        root_ref: &manifest.root_ref,
        evidence_refs: &manifest.evidence_refs,
    })
}

#[test]
fn manifest_text_roundtrip_keeps_identity() {
    let root = super::temp_dir("chunk-roundtrip");
    let put = crate::chunk_store::put_bytes(&root, "artifact", b"abcdef", 3).expect("put");
    let rendered = crate::preserves_rail::to_text(&put.manifest_value).expect("render manifest");
    let reparsed = crate::preserves_rail::parse_text(&rendered).expect("parse manifest");
    let parsed =
        crate::chunk_store::parse_manifest_value(&reparsed, Some(&put.manifest_ref)).expect("parse manifest value");
    assert_eq!(parsed.chunks.len(), 2);
}
