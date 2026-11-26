#[test]
fn confidentiality_and_transform_modes_fail_closed_until_supported() {
    assert_confidential_write_denials();
    let (root, transformed_manifest_ref) = write_unsupported_manifest();
    assert_unsupported_transform_denials(&root, &transformed_manifest_ref);
}

fn assert_confidential_write_denials() {
    let confidential_root = super::temp_dir("chunk-confidential-deny");
    let metadata =
        crate::preserves_rail::record("chunk-metadata-v1", vec![crate::preserves_rail::record("object-kind", vec![
            crate::preserves_rail::string("artifact"),
        ])]);
    let mut confidential_without_commitment = crate::chunk_store::ChunkTransforms::public_plaintext();
    confidential_without_commitment.confidentiality = "confidential".to_string();
    let error = crate::chunk_store::put_bytes_with_transforms(&crate::chunk_store::PutBytesWithTransformsInput {
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
    let denial_receipts = crate::chunk_store::list_receipt_refs(&confidential_root)
        .expect("list confidential receipts")
        .iter()
        .map(|receipt_ref| crate::chunk_store::read_receipt(&confidential_root, receipt_ref).expect("read receipt"))
        .filter(|receipt| receipt.decision == "deny" && receipt.operation == "manifest-create")
        .collect::<Vec<_>>();
    assert_eq!(denial_receipts.len(), 1);
    let protected_commitment_ref = crate::preserves_rail::content_ref_from_bytes(b"protected-commitment-fixture");
    let protected_shape = crate::chunk_store::ChunkTransforms::confidential_protected(&protected_commitment_ref);
    let protected_error =
        crate::chunk_store::put_bytes_with_transforms(&crate::chunk_store::PutBytesWithTransformsInput {
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

fn write_unsupported_manifest() -> (crate::test_support::ProcessWorkspace, String) {
    let root = super::temp_dir("chunk-transform-unsupported");
    let put = crate::chunk_store::put_bytes(&root, "artifact", b"aaaabbbb", 4).expect("put public");
    let public_manifest = crate::chunk_store::read_manifest(&root, &put.manifest_ref).expect("read manifest");
    let unsupported = crate::chunk_store::ChunkTransforms {
        compression: "zstd-placeholder".to_string(),
        encryption: "none".to_string(),
        ordering: "compress".to_string(),
        confidentiality: "public".to_string(),
        protected_commitment_ref: None,
    };
    let transformed_chunks = public_manifest
        .chunks
        .iter()
        .map(|chunk| crate::chunk_store::ChunkRef {
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
            crate::blocks::wire::encode::chunk_ref_value(
                &chunk.chunk_ref,
                chunk.length,
                usize::try_from(public_manifest.chunk_size).expect("test chunk size fits usize"),
                &unsupported,
            )
        })
        .collect::<Vec<_>>();
    let transformed_root_ref = crate::blocks::wire::encode::chunk_root_ref(&transformed_chunks).expect("chunk root");
    let transformed_manifest_value =
        crate::blocks::wire::encode::manifest_value(&crate::blocks::wire::encode::ChunkManifestValueInput {
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
    let transformed_manifest_ref =
        crate::preserves_rail::canonical_hash(&transformed_manifest_value).expect("manifest ref");
    std::fs::write(
        super::test_manifest_path(&root, &transformed_manifest_ref).expect("manifest path"),
        crate::preserves_rail::canonical_bytes(&transformed_manifest_value).expect("manifest bytes"),
    )
    .expect("write transformed manifest");
    let parsed =
        crate::chunk_store::read_manifest(&root, &transformed_manifest_ref).expect("parse transformed manifest");
    assert_eq!(parsed.transforms, unsupported);
    (root, transformed_manifest_ref)
}

fn assert_unsupported_transform_denials(root: &std::path::Path, transformed_manifest_ref: &str) {
    assert!(
        crate::chunk_store::verify_manifest(root, transformed_manifest_ref)
            .expect_err("verify rejects unsupported transform")
            .to_string()
            .contains("unsupported chunk-store transform")
    );
    assert!(
        crate::chunk_store::read_object(root, transformed_manifest_ref)
            .expect_err("read rejects unsupported transform")
            .to_string()
            .contains("unsupported chunk-store transform")
    );
    assert!(
        crate::chunk_store::range_read(root, transformed_manifest_ref, 0, 1)
            .expect_err("range rejects unsupported transform")
            .to_string()
            .contains("unsupported chunk-store transform")
    );
    let transform_denials = crate::chunk_store::list_receipt_refs(root)
        .expect("list transform receipts")
        .iter()
        .map(|receipt_ref| crate::chunk_store::read_receipt(root, receipt_ref).expect("read transform receipt"))
        .filter(|receipt| receipt.decision == "deny")
        .collect::<Vec<_>>();
    assert!(transform_denials.iter().any(|receipt| receipt.operation == "chunk-verify"));
    assert!(transform_denials.iter().any(|receipt| receipt.operation == "fetch"));
    assert!(transform_denials.iter().any(|receipt| receipt.operation == "range-read"));
}
