#[test]
fn gc_preserves_pinned_manifest_chunks_and_removes_unpinned_content() {
    let root = super::temp_dir("chunk-gc");
    let pinned = crate::chunk_store::put_bytes(&root, "artifact", b"aaaabbbbcccc", 4).expect("put pinned");
    let unpinned = crate::chunk_store::put_bytes(&root, "artifact", b"dddd", 4).expect("put unpinned");
    crate::chunk_store::pin_manifest(&root, &pinned.manifest_ref).expect("pin manifest");
    let retention_evidence = super::retention_evidence(&root, "gc-remove");
    let apply_refs = super::gc_apply_refs(
        &root,
        std::slice::from_ref(&unpinned.manifest_ref),
        &unpinned.chunk_refs,
        &retention_evidence,
    );
    let gc = crate::chunk_store::gc(&root, crate::chunk_store::ChunkGcInput {
        dry_run: false,
        retention_evidence: &retention_evidence,
        apply_refs: &apply_refs,
    })
    .expect("gc");
    assert!(gc.removed_manifests.contains(&unpinned.manifest_ref));
    assert!(gc.removed_chunks.contains(&unpinned.chunk_refs[0]));
    crate::chunk_store::read_object(&root, &pinned.manifest_ref).expect("pinned object remains readable");
    assert!(crate::chunk_store::read_manifest(&root, &unpinned.manifest_ref).is_err());
}

#[test]
fn chunk_gc_requires_retention_pass_before_removal() {
    let root = super::temp_dir("chunk-retention-gc");
    let put = crate::chunk_store::put_bytes(&root, "artifact", b"retained", 4).expect("put retained");
    let owner_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("chunk-test-ref", vec![
        crate::preserves_rail::string("owner"),
    ]))
    .expect("owner ref");
    let policy_refs = vec![
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("chunk-test-ref", vec![
            crate::preserves_rail::string("policy"),
        ]))
        .expect("policy ref"),
    ];
    let evidence_refs = vec![
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("chunk-test-ref", vec![
            crate::preserves_rail::string("evidence"),
        ]))
        .expect("evidence ref"),
    ];
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
    let retention_evidence = super::retention_evidence(&root, "retention-pin");
    let gc = crate::chunk_store::gc(&root, crate::chunk_store::ChunkGcInput {
        dry_run: false,
        retention_evidence: &retention_evidence,
        apply_refs: &[],
    })
    .expect("gc");
    assert_eq!(gc.decision, "deny");
    assert!(gc.removed_manifests.is_empty());
    assert!(gc.removed_chunks.is_empty());
    assert!(!gc.retention_receipt_refs.is_empty());
    crate::chunk_store::read_object(&root, &put.manifest_ref).expect("retained object remains readable");
}

#[test]
fn chunk_gc_denies_apply_refs_for_the_wrong_object_scope() {
    let root = super::temp_dir("chunk-retention-wrong-apply");
    let protected = crate::chunk_store::put_bytes(&root, "artifact", b"protected", 4).expect("put protected");
    let wrong = crate::chunk_store::put_bytes(&root, "artifact", b"wrong", 4).expect("put wrong");
    let retention_evidence = super::retention_evidence(&root, "wrong-apply");
    let wrong_apply_refs =
        super::gc_apply_refs(&root, std::slice::from_ref(&wrong.manifest_ref), &[], &retention_evidence);
    let gc = crate::chunk_store::gc(&root, crate::chunk_store::ChunkGcInput {
        dry_run: false,
        retention_evidence: &retention_evidence,
        apply_refs: &wrong_apply_refs,
    })
    .expect("gc denied by wrong apply");
    assert_eq!(gc.decision, "deny");
    assert!(gc.removed_manifests.is_empty());
    assert!(gc.removed_chunks.is_empty());
    let receipt_text = crate::preserves_rail::to_text(&gc.receipt_value).expect("gc receipt text");
    assert!(receipt_text.contains("retention-gc-execute-apply-scope-mismatch"), "{receipt_text}");
    crate::chunk_store::read_object(&root, &protected.manifest_ref).expect("protected object remains readable");
    crate::chunk_store::read_object(&root, &wrong.manifest_ref).expect("wrong-scope object remains readable");
}

#[test]
fn chunk_gc_denies_incomplete_reference_index_and_remote_uncertainty() {
    let root = super::temp_dir("chunk-retention-incomplete-remote");
    let put = crate::chunk_store::put_bytes(&root, "artifact", b"remote", 3).expect("put remote-retained");
    let mut retention_evidence = super::retention_evidence(&root, "incomplete-remote");
    retention_evidence.remote_refs = vec![super::chunk_test_ref("remote", "incomplete-remote")];
    retention_evidence.is_reference_index_complete = false;
    let gc = crate::chunk_store::gc(&root, crate::chunk_store::ChunkGcInput {
        dry_run: false,
        retention_evidence: &retention_evidence,
        apply_refs: &[],
    })
    .expect("gc denied");
    assert_eq!(gc.decision, "deny");
    assert!(gc.removed_manifests.is_empty());
    assert!(gc.removed_chunks.is_empty());
    crate::chunk_store::read_object(&root, &put.manifest_ref).expect("remote-uncertain object remains readable");
}
