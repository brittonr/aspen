#[test]
fn redb_index_tracks_rebuild_pins_missing_chunks_and_partial_fetches() {
    let root = super::temp_dir("chunk-index");
    let put = crate::chunk_store::put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put");
    let status = crate::chunk_store::index_status(&root).expect("index status after put");
    assert_eq!(status.manifests, 1);
    assert_eq!(status.chunks, 2);
    assert_eq!(status.available_chunks, 2);
    assert_eq!(status.missing_chunks, 0);
    assert_eq!(status.receipts, 1);

    crate::chunk_store::pin_manifest(&root, &put.manifest_ref).expect("pin manifest");
    let status = crate::chunk_store::index_status(&root).expect("index status after pin");
    assert_eq!(status.manifest_pins, 1);
    let rebuild = crate::chunk_store::rebuild_index(&root).expect("rebuild index");
    assert_eq!(rebuild.status.manifests, 1);
    assert_eq!(rebuild.status.chunks, 2);
    assert_eq!(rebuild.status.manifest_pins, 1);
    assert_eq!(rebuild.status.receipts, 3);

    let manifest = crate::chunk_store::read_manifest(&root, &put.manifest_ref).expect("read manifest");
    std::fs::remove_file(super::test_chunk_path(&root, &manifest.chunks[0].chunk_ref).expect("chunk path"))
        .expect("remove chunk");
    let missing = crate::chunk_store::missing_chunks(&root, &put.manifest_ref).expect("missing chunks");
    assert_eq!(missing, vec![manifest.chunks[0].chunk_ref.clone()]);
    let status = crate::chunk_store::index_status(&root).expect("index status after missing scan");
    assert_eq!(status.available_chunks, 1);
    assert_eq!(status.missing_chunks, 1);

    let source = super::temp_dir("chunk-index-source");
    let dest = super::temp_dir("chunk-index-dest");
    let source_put = crate::chunk_store::put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
    crate::chunk_store::put_bytes(&dest, "artifact", b"aaaa", 4).expect("seed dest");
    let sync = crate::chunk_store::sync_missing_chunks(&source, &dest, &source_put.manifest_ref).expect("sync");
    assert_eq!(sync.missing_before.len(), 2);
    assert_eq!(sync.fetched_chunks.len(), 2);
    let status = crate::chunk_store::index_status(&dest).expect("dest index status");
    assert_eq!(status.partial_fetches, 1);
    assert_eq!(status.missing_chunks, 0);
    assert_eq!(status.available_chunks, 3);
}

#[test]
fn receipt_index_covers_pass_denial_dedup_and_tombstone_evidence() {
    let root = super::temp_dir("chunk-receipts");
    let put = crate::chunk_store::put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put");
    crate::chunk_store::put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("dedup put");
    crate::chunk_store::verify_manifest(&root, &put.manifest_ref).expect("verify");
    crate::chunk_store::read_object(&root, &put.manifest_ref).expect("fetch");
    crate::chunk_store::range_read(&root, &put.manifest_ref, 1, 5).expect("range");
    crate::chunk_store::pin_manifest(&root, &put.manifest_ref).expect("pin");
    crate::chunk_store::unpin_manifest(&root, &put.manifest_ref).expect("unpin");
    let retention_evidence = super::retention_evidence(&root, "receipt-index");
    let apply_refs =
        super::gc_apply_refs(&root, std::slice::from_ref(&put.manifest_ref), &put.chunk_refs, &retention_evidence);
    crate::chunk_store::gc(&root, crate::chunk_store::ChunkGcInput {
        dry_run: false,
        retention_evidence: &retention_evidence,
        apply_refs: &apply_refs,
    })
    .expect("gc");

    let before_rebuild = crate::chunk_store::list_receipt_refs(&root).expect("list receipts");
    let receipts = before_rebuild
        .iter()
        .map(|receipt_ref| crate::chunk_store::read_receipt(&root, receipt_ref).expect("read receipt"))
        .collect::<Vec<_>>();
    for receipt in &receipts {
        assert_eq!(crate::preserves_rail::canonical_hash(&receipt.value).expect("receipt ref"), receipt.receipt_ref);
        crate::chunk_store::parse_receipt_value(&receipt.value, Some(&receipt.receipt_ref)).expect("validate receipt");
    }
    let operations =
        receipts.iter().map(|receipt| receipt.operation.as_str()).collect::<std::collections::BTreeSet<_>>();
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

    crate::chunk_store::rebuild_index(&root).expect("rebuild preserves receipt table");
    let after_rebuild = crate::chunk_store::list_receipt_refs(&root).expect("list receipts after rebuild");
    for receipt_ref in before_rebuild {
        assert!(after_rebuild.contains(&receipt_ref), "receipt {receipt_ref} survived rebuild");
    }
    assert_denial_receipts_are_indexed();
}

/// Corrupt verify, out-of-bounds range, and missing-chunk pin requests each index a deny receipt.
fn assert_denial_receipts_are_indexed() {
    let denial_root = super::temp_dir("chunk-denial-receipts");
    let denial_put =
        crate::chunk_store::put_bytes(&denial_root, "artifact", b"aaaabbbb", 4).expect("put denial fixture");
    let denial_manifest =
        crate::chunk_store::read_manifest(&denial_root, &denial_put.manifest_ref).expect("read denial manifest");
    std::fs::write(
        super::test_chunk_path(&denial_root, &denial_manifest.chunks[0].chunk_ref).expect("chunk path"),
        b"zzzz",
    )
    .expect("corrupt chunk");
    crate::chunk_store::verify_manifest(&denial_root, &denial_put.manifest_ref).expect_err("corrupt verify denied");
    crate::chunk_store::range_read(&denial_root, &denial_put.manifest_ref, 99, 1).expect_err("range denied");
    let missing_chunk_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("chunk-test-ref", vec![
            crate::preserves_rail::string("missing-pin"),
        ]))
        .expect("missing pin ref");
    crate::chunk_store::pin_chunk(&denial_root, &missing_chunk_ref).expect_err("missing chunk pin denied");
    let denials = crate::chunk_store::list_receipt_refs(&denial_root)
        .expect("list denial receipts")
        .iter()
        .map(|receipt_ref| crate::chunk_store::read_receipt(&denial_root, receipt_ref).expect("read denial receipt"))
        .filter(|receipt| receipt.decision == "deny")
        .collect::<Vec<_>>();
    assert!(denials.iter().any(|receipt| receipt.operation == "chunk-verify"));
    assert!(denials.iter().any(|receipt| receipt.operation == "range-read"));
    assert!(denials.iter().any(|receipt| receipt.operation == "pin"));
}
