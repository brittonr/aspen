
    #[test]
    fn classifies_generic_replay_receipts_and_divergence() {
        let dir = temp_dir("catalog-replay");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let expected_report_ref = test_ref("expected-report");
        let actual_report_ref = test_ref("actual-report");
        let final_state_ref = test_ref("final-state");
        let values = seed_values(&expected_report_ref, &actual_report_ref, &final_state_ref);
        import_values(&ledger_root, &values);

        assert_found_text(
            &registry,
            &ledger_root,
            vec![
                Filter::LedgerKind("deterministic-replay-verify-receipt".to_string()),
                Filter::ReceiptDecision("pass".to_string()),
                Filter::Text(format!("replay-final-state:{final_state_ref}")),
            ],
            &[
                "deterministic-replay:verify",
                expected_report_ref.as_str(),
                actual_report_ref.as_str(),
                final_state_ref.as_str(),
            ],
        );
        assert_found_text(
            &registry,
            &ledger_root,
            vec![Filter::Text("replay-divergence:effect-response".to_string())],
            &["deterministic-replay:first-divergence"],
        );
        assert_found_text(
            &registry,
            &ledger_root,
            vec![
                Filter::LedgerKind("deterministic-replay-rollup".to_string()),
                Filter::Text("replay-rollup-decision:pass".to_string()),
                Filter::Text("replay-rollup-total:1".to_string()),
            ],
            &["deterministic-replay:rollup"],
        );
        assert_found_text(
            &registry,
            &ledger_root,
            vec![
                Filter::LedgerKind("deterministic-replay-index".to_string()),
                Filter::Text("replay-index-decision:pass".to_string()),
                Filter::Text("replay-index-rollups:1".to_string()),
                Filter::Text(format!("replay-index-final-state:{final_state_ref}")),
            ],
            &["deterministic-replay:index"],
        );
    }

    #[test]
    fn classifies_provenance_records_receipts_and_build_evidence() {
        let dir = temp_dir("catalog-provenance");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let artifact_ref = test_ref("provenance-artifact");
        let record = crate::provenance::synthetic_reviewed_record(&artifact_ref).expect("record");
        let evaluation = crate::provenance::evaluate(&crate::provenance::EvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&record),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("evaluate provenance");
        crate::ledger::import_artifact(&ledger_root, &record).expect("import record");
        crate::ledger::import_artifact(&ledger_root, &evaluation.receipt_value).expect("import receipt");
        let found = search(&registry, Some(&ledger_root), &SearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![Filter::Text("provenance-trust-state:reviewed".to_string())],
            visibility: VisibilityInput::default(),
        })
        .expect("provenance search");
        assert!(!found.items.is_empty());
        let text = to_text(&found.value).expect("provenance result text");
        assert!(text.contains("provenance:record"));
        assert!(text.contains("provenance:receipt"));
    }

    #[test]
    fn gc_chain_artifacts_are_catalog_searchable() {
        let dir = temp_dir("catalog-retention-gc");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let retention_root = dir.join("retention");
        let fixture = gc_case(&retention_root, "catalog-retention-gc", "ledger-gc");
        crate::ledger::import_artifact(&ledger_root, &fixture.plan.value).expect("import plan");
        crate::ledger::import_artifact(&ledger_root, &fixture.apply.value).expect("import apply");
        crate::ledger::import_artifact(&ledger_root, &fixture.execution.value).expect("import execution");
        crate::ledger::import_artifact(&ledger_root, &fixture.audit.value).expect("import audit");

        let found = search(&registry, Some(&ledger_root), &SearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![
                Filter::Text(format!("retention-gc-object:{}", fixture.object_ref)),
                Filter::Text("retention-gc-subsystem:ledger-gc".to_string()),
            ],
            visibility: VisibilityInput::default(),
        })
        .expect("search retention GC chain");
        assert_eq!(found.items.len(), 4);
        let text = to_text(&found.value).expect("retention GC catalog text");
        assert!(text.contains("retention-gc:plan"));
        assert!(text.contains("retention-gc:apply"));
        assert!(text.contains("retention-gc:execute"));
        assert!(text.contains("retention-gc:audit"));
        assert!(text.contains(&fixture.plan.plan_ref));
        assert!(text.contains(&fixture.apply.apply_ref));
        assert!(text.contains(&fixture.execution.execution_ref));

        let audit = search(&registry, Some(&ledger_root), &SearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![Filter::LedgerKind("retention-gc-audit".to_string())],
            visibility: VisibilityInput::default(),
        })
        .expect("search retention GC audit by ledger kind");
        assert_eq!(audit.items.len(), 1);
        assert!(to_text(&audit.value).expect("audit search text").contains("retention-gc:audit"));
    }

    #[test]
    fn short_id_resolution_denies_too_short_ambiguous_and_hidden_candidates() {
        let dir = temp_dir("catalog-short");
        let registry = dir.join("registry");
        let mut refs_by_first_hex = Vec::<(char, String)>::with_capacity(32);
        let mut ambiguous_pair = None;
        for index in 0..32 {
            let installed =
                install_fixture(&registry, "doc", parse_text(&format!("<doc {index}>")).expect("doc"), &[], &[]);
            let first_hex = installed.artifact_ref.as_bytes()[7] as char;
            if let Some((_, existing_ref)) = refs_by_first_hex.iter().find(|(hex, _)| *hex == first_hex) {
                ambiguous_pair = Some((existing_ref.clone(), installed.artifact_ref.clone()));
                break;
            }
            refs_by_first_hex.push((first_hex, installed.artifact_ref));
        }
        let (first_ref, second_ref) = ambiguous_pair.expect("fixture collision within hex alphabet");
        let shared_prefix = first_ref[7..8].to_string();
        let too_short = resolve_short_id(&registry, None, &ShortIdInput {
            prefix: shared_prefix.clone(),
            min_length: DEFAULT_SHORT_ID_MIN_LENGTH,
            visibility: VisibilityInput::default(),
        })
        .expect("too short resolution receipt");
        assert_eq!(too_short.decision, "deny");
        let ambiguous = resolve_short_id(&registry, None, &ShortIdInput {
            prefix: shared_prefix.clone(),
            min_length: 0,
            visibility: VisibilityInput::default(),
        })
        .expect("ambiguous resolution receipt");
        assert_eq!(ambiguous.decision, "deny");
        assert!(ambiguous.candidates.len() >= 2);
        let visible = resolve_short_id(&registry, None, &ShortIdInput {
            prefix: shared_prefix,
            min_length: 0,
            visibility: VisibilityInput {
                hidden_refs: vec![second_ref],
                ..VisibilityInput::default()
            },
        })
        .expect("hidden candidate filtered");
        assert_eq!(visible.full_ref.as_deref(), Some(first_ref.as_str()));
    }

    #[test]
    fn short_id_resolution_rejects_malformed_ref_shapes_and_uppercase_prefixes() {
        let dir = temp_dir("catalog-short-canonical");
        let registry = dir.join("registry");
        let artifact = install_fixture(&registry, "doc", parse_text("<doc \"canonical\">").expect("doc"), &[], &[]);

        let malformed_ref = resolve_short_id(&registry, None, &ShortIdInput {
            prefix: "blake3:".to_string(),
            min_length: 0,
            visibility: VisibilityInput::default(),
        })
        .expect("malformed ref resolution receipt");
        assert_eq!(malformed_ref.decision, "deny");
        assert!(malformed_ref.candidates.is_empty());
        assert!(to_text(&malformed_ref.value).expect("malformed ref text").contains("malformed full content ref"));

        let uppercase = resolve_short_id(&registry, None, &ShortIdInput {
            prefix: "ABCDEF".to_string(),
            min_length: 0,
            visibility: VisibilityInput::default(),
        })
        .expect("uppercase resolution receipt");
        assert_eq!(uppercase.decision, "deny");
        assert!(uppercase.candidates.is_empty());
        assert!(to_text(&uppercase.value).expect("uppercase text").contains("lowercase hex"));

        let hidden_only = resolve_short_id(&registry, None, &ShortIdInput {
            prefix: artifact.artifact_ref[7..19].to_string(),
            min_length: 0,
            visibility: VisibilityInput {
                hidden_refs: vec![artifact.artifact_ref.clone()],
                ..VisibilityInput::default()
            },
        })
        .expect("hidden-only resolution receipt");
        assert_eq!(hidden_only.decision, "deny");
        assert!(hidden_only.candidates.is_empty());
        assert_eq!(hidden_only.full_ref, None);

        let full_ref = resolve_short_id(&registry, None, &ShortIdInput {
            prefix: artifact.artifact_ref.clone(),
            min_length: DEFAULT_SHORT_ID_MIN_LENGTH,
            visibility: VisibilityInput::default(),
        })
        .expect("full ref resolution receipt");
        assert_eq!(full_ref.decision, "pass");
        assert_eq!(full_ref.full_ref, Some(artifact.artifact_ref));
    }

    #[test]
    fn view_redacts_sensitive_payloads_before_rendering() {
        let dir = temp_dir("catalog-redact");
        let registry = dir.join("registry");
        let secret =
            install_fixture(&registry, "doc", parse_text("<doc <secret \"do-not-render\">>").expect("secret"), &[], &[
            ]);
        let viewed = view(&registry, None, &ViewInput {
            reference: secret.artifact_ref,
            include_payload: true,
            redacted: true,
            visibility: VisibilityInput::default(),
        })
        .expect("view redacted");
        let text = to_text(&viewed.value).expect("render view");
        assert!(text.contains("redaction-marker-v1"));
        assert!(!text.contains("do-not-render"));
    }
