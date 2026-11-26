#[test]
fn sync_fetches_only_missing_chunks_and_preserves_manifest_identity() {
    let source = super::temp_dir("chunk-sync-source");
    let dest = super::temp_dir("chunk-sync-dest");
    let source_put = crate::chunk_store::put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
    let _dest_seed = crate::chunk_store::put_bytes(&dest, "artifact", b"aaaabbbb", 4).expect("seed destination");
    let sync =
        crate::chunk_store::sync_missing_chunks(&source, &dest, &source_put.manifest_ref).expect("sync missing chunks");
    assert_eq!(sync.manifest_ref, source_put.manifest_ref);
    assert_eq!(sync.fetched_chunks, vec![source_put.chunk_refs[2].clone()]);
    assert_eq!(
        crate::chunk_store::read_object(&dest, &source_put.manifest_ref).expect("read synced").bytes,
        b"aaaabbbbcccc"
    );
    let repeat =
        crate::chunk_store::sync_missing_chunks(&source, &dest, &source_put.manifest_ref).expect("repeat sync");
    assert!(repeat.fetched_chunks.is_empty());
    assert!(repeat.missing_before.is_empty());
}

#[test]
fn iroh_adapter_publishes_and_fetches_missing_verified_chunks() {
    let source = super::temp_dir("chunk-iroh-source");
    let dest = super::temp_dir("chunk-iroh-dest");
    let iroh = super::temp_dir("chunk-iroh-blobs");
    let source_put = crate::chunk_store::put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
    let published =
        crate::chunk_store::publish_iroh_blobs(&source, &iroh, &source_put.manifest_ref, "node:test").expect("publish");
    assert_eq!(published.manifest_ref, source_put.manifest_ref);
    assert_eq!(published.manifest_blob_ref, source_put.manifest_ref);
    assert_eq!(published.chunk_blob_refs.len(), source_put.chunk_refs.len());
    let _dest_seed = crate::chunk_store::put_bytes(&dest, "artifact", b"aaaa", 4).expect("seed destination");
    let fetched = crate::chunk_store::fetch_iroh_blobs(
        &iroh,
        &dest,
        &published.ticket,
        Some(&source_put.manifest_ref),
        "peer:test",
    )
    .expect("fetch");
    assert_eq!(fetched.manifest_ref, source_put.manifest_ref);
    assert_eq!(fetched.missing_before.len(), 2);
    assert_eq!(fetched.fetched_chunks, source_put.chunk_refs[1..].to_vec());
    assert_eq!(
        crate::chunk_store::read_object(&dest, &source_put.manifest_ref).expect("read fetched").bytes,
        b"aaaabbbbcccc"
    );
    let repeat = crate::chunk_store::fetch_iroh_blobs(
        &iroh,
        &dest,
        &published.ticket,
        Some(&source_put.manifest_ref),
        "peer:test",
    )
    .expect("repeat fetch");
    assert!(repeat.missing_before.is_empty());
    assert!(repeat.fetched_chunks.is_empty());
    let receipts = crate::chunk_store::list_receipt_refs(&dest)
        .expect("list receipts")
        .iter()
        .map(|receipt_ref| crate::chunk_store::read_receipt(&dest, receipt_ref).expect("read receipt"))
        .collect::<Vec<_>>();
    let has_pass_fetch_receipt = receipts
        .iter()
        .filter(|receipt| receipt.operation == "iroh-fetch")
        .any(|receipt| receipt.decision == "pass");
    assert!(has_pass_fetch_receipt);
    let wrong =
        crate::chunk_store::fetch_iroh_blobs(&iroh, &dest, &published.ticket, Some("blake3:deadbeef"), "peer:test")
            .expect_err("wrong expected manifest fails");
    assert!(wrong.to_string().contains("expected blake3:deadbeef"));
}

#[test]
fn lineage_chains_bind_manifest_publication_fetch_and_scope() {
    let source = super::temp_dir("chunk-lineage-source");
    let dest = super::temp_dir("chunk-lineage-dest");
    let iroh = super::temp_dir("chunk-lineage-iroh");
    let source_put = crate::chunk_store::put_bytes(&source, "artifact", b"aaaabbbbcccc", 4).expect("put source");
    let published =
        crate::chunk_store::publish_iroh_blobs(&source, &iroh, &source_put.manifest_ref, "node:test").expect("publish");
    let source_lineage =
        crate::chunk_store::build_chunk_lineage(&source, &source_put.manifest_ref).expect("source lineage");
    assert_eq!(source_lineage.manifest_ref, source_put.manifest_ref);
    assert!(source_lineage.receipt_refs.len() >= 2);
    crate::chunk_store::parse_chunk_lineage_value(&source_lineage.value).expect("parse source lineage");
    let source_text = crate::preserves_rail::to_text(&source_lineage.value).expect("render source lineage");
    assert!(source_text.contains("chunk-lineage"));
    assert!(source_text.contains("iroh-publish"));
    assert!(source_text.contains("lineage-no-global-head"));

    let fetched = crate::chunk_store::fetch_iroh_blobs(
        &iroh,
        &dest,
        &published.ticket,
        Some(&source_put.manifest_ref),
        "peer:test",
    )
    .expect("fetch");
    let dest_lineage = crate::chunk_store::build_chunk_lineage(&dest, &fetched.manifest_ref).expect("dest lineage");
    assert_eq!(dest_lineage.manifest_ref, source_put.manifest_ref);
    crate::chunk_store::parse_chunk_lineage_value(&dest_lineage.value).expect("parse dest lineage");
    assert!(
        crate::preserves_rail::to_text(&dest_lineage.value)
            .expect("render dest lineage")
            .contains("iroh-fetch")
    );

    let manifest = crate::chunk_store::read_manifest(&source, &source_put.manifest_ref).expect("read manifest");
    let wrong_root = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("wrong-root", vec![
        crate::preserves_rail::string("lineage"),
    ]))
    .expect("wrong root ref");
    let tampered_root = crate::preserves_rail::parse_text(&source_text.replacen(&manifest.root_ref, &wrong_root, 1))
        .expect("parse tampered root lineage");
    let error = crate::chunk_store::parse_chunk_lineage_value(&tampered_root).expect_err("tampered root fails");
    assert!(["root", "scope"].iter().any(|needle| error.to_string().contains(needle)), "{error}");

    let tampered_ticket =
        crate::preserves_rail::parse_text(&source_text.replacen("iroh-local-chunk", "iroh-tampered-chunk", 1))
            .expect("parse tampered ticket lineage");
    let error = crate::chunk_store::parse_chunk_lineage_value(&tampered_ticket).expect_err("tampered ticket fails");
    assert!(["payload", "receipt"].iter().any(|needle| error.to_string().contains(needle)), "{error}");

    let other_put = crate::chunk_store::put_bytes(&source, "artifact", b"different", 4).expect("put other");
    let other_lineage =
        crate::chunk_store::build_chunk_lineage(&source, &other_put.manifest_ref).expect("other lineage");
    assert_ne!(source_lineage.manifest_ref, other_lineage.manifest_ref);
    assert_ne!(source_lineage.link_refs.last(), other_lineage.link_refs.last());
}
