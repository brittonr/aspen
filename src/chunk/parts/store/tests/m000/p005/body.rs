
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
