
    fn snapshot_test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(label)).expect("snapshot test ref")
    }

    fn snapshot_state_with_readable(readable: Vec<String>) -> RuntimeSnapshotAuthorityState {
        RuntimeSnapshotAuthorityState {
            snapshot_ref: snapshot_test_ref("label-snapshot"),
            admitted_authority_refs: vec![snapshot_test_ref("label-admitted")],
            claimed_authority_refs: vec![snapshot_test_ref("label-admitted")],
            requested_assertion_refs: readable.clone(),
            readable_assertion_refs: readable,
            redacted_assertion_refs: Vec::new(),
        }
    }

    #[test]
    fn snapshot_receipt_labels_readable_assertions_against_the_admitted_set() {
        // r[verify molten.runtime_spine.snapshot_receipt_check_label]
        let admitted = evaluate_snapshot_authority(&snapshot_state_with_readable(vec![snapshot_test_ref(
            "label-admitted",
        )]))
        .expect("snapshot authority predicate");
        assert!(admitted.is_allowed);
        assert_eq!(admitted.receipt.predicate, "molten.trellis-runtime.snapshot-authority.v2");
        let readable_label = SnapshotAuthorityCheck::ReadableAssertionsSubsetAdmitted.label();
        assert_eq!(readable_label, "readable-assertions-subset-admitted");
        assert!(admitted.receipt.checks.iter().any(|check| check == readable_label));
        assert!(!admitted.receipt.checks.iter().any(|check| check == "readable-assertions-subset-claimed"));
        for check in &admitted.receipt.checks {
            parse_snapshot_authority_check(&admitted.receipt.predicate, check).expect("v2 receipt check parses");
        }
    }

    #[test]
    fn snapshot_receipt_fails_the_admitted_check_for_an_unadmitted_readable_assertion() {
        // r[verify molten.runtime_spine.snapshot_receipt_check_label]
        let readable = sorted_refs(vec![snapshot_test_ref("label-admitted"), snapshot_test_ref("label-outside")]);
        let denied = evaluate_snapshot_authority(&snapshot_state_with_readable(readable)).expect("denied snapshot");
        assert!(!denied.is_allowed);
        assert_eq!(denied.receipt.decision, PredicateDecision::Deny);
        assert_eq!(denied.receipt.diagnostics, vec!["snapshot-readable-assertion-not-authorized".to_string()]);
        let readable_label = SnapshotAuthorityCheck::ReadableAssertionsSubsetAdmitted.label();
        assert!(denied.receipt.checks.iter().any(|check| check == readable_label));
    }

    #[test]
    fn legacy_snapshot_receipt_label_parses_only_as_an_alias_in_v1_receipts() {
        // r[verify molten.runtime_spine.snapshot_receipt_check_label]
        let v1 = "molten.trellis-runtime.snapshot-authority.v1";
        let v2 = "molten.trellis-runtime.snapshot-authority.v2";
        let legacy = "readable-assertions-subset-claimed";
        assert_eq!(
            parse_snapshot_authority_check(v1, legacy).expect("v1 alias parses"),
            SnapshotAuthorityCheck::ReadableAssertionsSubsetAdmitted
        );
        assert!(parse_snapshot_authority_check(v2, legacy).is_err());
        assert_eq!(
            parse_snapshot_authority_check(v2, "readable-assertions-subset-admitted").expect("v2 label parses"),
            SnapshotAuthorityCheck::ReadableAssertionsSubsetAdmitted
        );
        assert!(parse_snapshot_authority_check("molten.trellis-runtime.object-authority.v1", legacy).is_err());
        assert!(parse_snapshot_authority_check(v1, "readable-assertions-subset-anything").is_err());
    }
