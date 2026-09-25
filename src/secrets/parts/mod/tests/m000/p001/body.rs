
    #[test]
    fn ledger_catalog_and_mcp_classify_confidentiality_artifacts() {
        let run = run_secrets_fixture().expect("fixture");
        assert_eq!(crate::ledger::artifact_kind(&run.secret.value), "secret-ref");
        assert_eq!(crate::ledger::artifact_kind(&run.encrypted.value), "encrypted-ref");
        assert_eq!(crate::ledger::artifact_kind(&run.marker.value), "redaction-marker");
        assert_eq!(crate::ledger::artifact_kind(&run.transform.value), "redaction-transform-receipt");
        let root = temp_dir("secrets-ledger");
        let registry = root.join("registry");
        let ledger_root = root.join("ledger");
        std::fs::create_dir_all(&registry).expect("registry");
        crate::ledger::import_artifact(&ledger_root, &run.secret.value).expect("import");
        let list = crate::catalog::list(&registry, Some(&ledger_root), &crate::catalog::ListInput {
            kind: Some("secret-ref".to_string()),
            visibility: crate::catalog::VisibilityInput::default(),
        })
        .expect("list");
        assert_eq!(list.items.len(), 1);
        let request = crate::catalog_mcp::mcp_request_value("catalog.view", vec![record("reference", vec![string(
            &run.secret.secret_ref,
        )])])
        .expect("mcp request");
        let response = crate::catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("mcp call");
        assert_eq!(response.decision, "pass");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_redaction_stability_no_plaintext_and_authority_monotonicity(tc: TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(1_000_000));
        let payload = record("secret", vec![string(format!("payload-{salt}"))]);
        let first = redacted_view(&payload, None).expect("first redaction");
        let second = redacted_view(&payload, None).expect("second redaction");
        assert_eq!(first.value, second.value);
        let redacted = to_text(&first.value).expect("redacted text");
        assert!(!redacted.contains(&format!("payload-{salt}")));
        let secret_ref = fixture_ref(&format!("secret-{salt}"));
        let commitment_ref = fixture_ref(&format!("commitment-{salt}"));
        let denied = parse_reveal_receipt(
            &reveal_receipt_value(&RevealReceiptInput {
                secret_ref: secret_ref.clone(),
                encrypted_ref: None,
                requester_ref: fixture_ref("requester"),
                purpose: "debug".to_string(),
                plaintext_ref: Some(fixture_ref("plain")),
                commitment_ref: commitment_ref.clone(),
                authority_refs: Vec::new(),
                policy_refs: vec![fixture_ref("policy")],
                resource_refs: vec![fixture_ref("resource")],
                effect_handle_refs: vec![fixture_ref("effect")],
                revocation_refs: Vec::new(),
            })
            .expect("deny reveal value"),
        )
        .expect("deny reveal");
        let admitted = parse_reveal_receipt(
            &reveal_receipt_value(&RevealReceiptInput {
                secret_ref,
                encrypted_ref: None,
                requester_ref: fixture_ref("requester"),
                purpose: "debug".to_string(),
                plaintext_ref: Some(fixture_ref("plain")),
                commitment_ref,
                authority_refs: vec![fixture_ref("authority")],
                policy_refs: vec![fixture_ref("policy")],
                resource_refs: vec![fixture_ref("resource")],
                effect_handle_refs: vec![fixture_ref("effect")],
                revocation_refs: Vec::new(),
            })
            .expect("admit reveal value"),
        )
        .expect("admit reveal");
        assert_eq!(denied.decision, "deny");
        assert_eq!(admitted.decision, "pass");
    }
