
    /// A tampered materialization receipt and a tampered bundled plan are each denied on verify.
    fn assert_tampered_bundle_denied(bundle_dir: &std::path::Path, plan_ref: &str) {
        let materialization_receipt_path = bundle_dir.join(CANDIDATE_BUNDLE_MATERIALIZATION_RECEIPT);
        let materialization_receipt_value =
            read_store_value(&materialization_receipt_path).expect("read candidate materialization receipt");
        let materialization_receipt = crate::materialization::parse_materialization_receipt(
            &materialization_receipt_value,
        )
        .expect("parse candidate materialization receipt");
        assert!(materialization_receipt.valid());
        write_store_value(
            &materialization_receipt_path,
            &crate::preserves_rail::record("tampered", Vec::new()),
        )
        .expect("tamper candidate materialization receipt");
        let invalid_receipt = verify_candidate_bundle(CandidateBundleVerifyInput {
            bundle_dir,
        })
        .expect("verify invalid candidate materialization receipt");
        assert_eq!(invalid_receipt.decision, "deny");
        assert!(invalid_receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.starts_with("retention-bundle-materialization-receipt-invalid:")));
        write_store_value(&materialization_receipt_path, &materialization_receipt_value)
            .expect("restore candidate materialization receipt");
        let tampered_path = bundle_dir
            .join("artifacts/gc-plans")
            .join(format!("{}.preserves", ref_file_name(plan_ref).expect("plan file name")));
        write_store_value(
            &tampered_path,
            &crate::preserves_rail::record("tampered", vec![crate::preserves_rail::string("plan")]),
        )
        .expect("tamper bundle plan");
        let tampered = verify_candidate_bundle(CandidateBundleVerifyInput {
            bundle_dir,
        })
        .expect("verify tampered retention candidate bundle");
        assert_eq!(tampered.decision, "deny");
        assert!(
            tampered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("retention-bundle-tampered-file:gc-plans"))
        );
    }
