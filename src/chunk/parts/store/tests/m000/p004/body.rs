
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
