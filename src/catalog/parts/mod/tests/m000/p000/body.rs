    use super::*;

    #[test]
    fn summaries_include_registry_names_dependencies_and_ledger_classification() {
        let dir = temp_dir("catalog-summary");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let schema_ref = test_ref("schema");
        let base =
            install_fixture(&registry, "schema", parse_text("<schema \"base\">").expect("schema"), &[], &[schema_ref]);
        let dependent = install_fixture(
            &registry,
            "doc",
            parse_text("<doc \"hello\">").expect("doc"),
            std::slice::from_ref(&base.artifact_ref),
            &[],
        );
        crate::artifacts::set_name_pointer(&registry, &crate::artifacts::SetNamePointerInput {
            pointer_kind: "name",
            name: "docs/main",
            artifact_ref: &dependent.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("set name");
        crate::ledger::import_artifact(&ledger_root, &dependent.artifact.value).expect("ledger import");
        let listed = list(&registry, Some(&ledger_root), &ListInput {
            kind: Some("doc".to_string()),
            visibility: VisibilityInput::default(),
        })
        .expect("catalog list");
        assert_eq!(listed.items.len(), 1);
        let text = to_text(&listed.items[0]).expect("render summary");
        assert!(["docs/main", "catalog-summary-v1"].iter().any(|needle| text.contains(needle)));
        assert!(text.contains(&base.artifact_ref));
        assert!(text.contains("ledger-kind:artifact-registry-artifact"));
    }

    #[test]
    fn chunk_store_exposes_availability_dedup_and_pins() {
        let dir = temp_dir("catalog-chunk-store");
        let chunks = dir.join("chunks");
        let first = crate::chunk_store::put_bytes(&chunks, "artifact", b"aaaabbbb", 4).expect("first chunk put");
        let second = crate::chunk_store::put_bytes(&chunks, "artifact", b"aaaacccc", 4).expect("second chunk put");
        crate::chunk_store::pin_manifest(&chunks, &first.manifest_ref).expect("pin manifest");
        crate::chunk_store::pin_chunk(&chunks, &first.chunk_refs[0]).expect("pin chunk");
        let result = chunk_store(&chunks, &ChunkStoreInput {
            visibility: VisibilityInput::default(),
        })
        .expect("catalog chunk store");
        assert_eq!(result.decision, "pass");
        let text = result.items.iter().map(|item| to_text(item).expect("render chunk catalog")).collect::<String>();
        assert!(text.contains("chunk-store-catalog-v1"));
        assert!(text.contains("chunk-manifest-catalog-v1"));
        assert!(text.contains("chunk-store:dedup"));
        assert!(text.contains("chunk-store-pin:pinned"));
        assert!(text.contains("chunk-store-availability:complete"));
        assert!(text.contains(&second.manifest_ref));
        assert!(text.contains(&first.chunk_refs[0]));

        let hidden = chunk_store(&chunks, &ChunkStoreInput {
            visibility: VisibilityInput {
                hidden_refs: vec![first.chunk_refs[0].clone()],
                ..VisibilityInput::default()
            },
        })
        .expect("catalog chunk store with hidden chunk");
        let hidden_text = hidden
            .items
            .iter()
            .map(|item| to_text(item).expect("render hidden chunk catalog"))
            .collect::<String>();
        assert!(!hidden_text.contains(&first.chunk_refs[0]));
        assert!(hidden_text.contains("redaction"));
    }

    #[test]
    fn search_filters_schema_dependency_receipt_decision_text_and_visibility() {
        let dir = temp_dir("catalog-search");
        let registry = dir.join("registry");
        let schema_ref = test_ref("schema-search");
        let base = install_fixture(
            &registry,
            "schema",
            parse_text("<schema \"search\">").expect("schema"),
            &[],
            std::slice::from_ref(&schema_ref),
        );
        let receipt_payload = record("rewrite-receipt-v1", vec![
            string("molten.rewrite.receipt.v1"),
            record("operation", vec![string("apply")]),
            record("decision", vec![string("pass")]),
            record("subject", vec![string(test_ref("subject"))]),
            record("refs", vec![sequence(Vec::new())]),
            record("diagnostics", vec![sequence(Vec::new())]),
            record("tool", vec![string("test")]),
            checks_value(&["canonical-receipt"]),
        ]);
        let receipt =
            install_fixture(&registry, "receipt", receipt_payload, std::slice::from_ref(&base.artifact_ref), &[]);
        let found = search(&registry, None, &SearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![
                Filter::ArtifactKind("receipt".to_string()),
                Filter::DependencyRef(base.artifact_ref.clone()),
                Filter::ReceiptDecision("pass".to_string()),
                Filter::Text("apply".to_string()),
            ],
            visibility: VisibilityInput::default(),
        })
        .expect("search receipt");
        assert_eq!(found.items.len(), 1);
        let hidden = search(&registry, None, &SearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![Filter::Text("apply".to_string())],
            visibility: VisibilityInput {
                hidden_refs: vec![receipt.artifact_ref],
                ..VisibilityInput::default()
            },
        })
        .expect("hidden search");
        assert!(hidden.items.is_empty());
    }

    #[test]
    fn semantic_search_covers_transcript_upgrade_and_receipt_views() {
        let dir = temp_dir("catalog-unison-views");
        let registry = dir.join("registry");
        let transcript_ref = test_ref("transcript");
        let transcript_receipt = record("transcript-run-receipt-v1", vec![
            string(crate::preserves_rail::TRANSCRIPT_RUN_RECEIPT_SCHEMA),
            record("operation", vec![string("run")]),
            record("decision", vec![string("pass")]),
            record("transcript", vec![string(&transcript_ref)]),
            record("mode", vec![string("check")]),
            record("outcomes", vec![sequence(Vec::new())]),
            record("output", vec![record("none", Vec::new())]),
            record("refs", vec![sequence(vec![string(&transcript_ref)])]),
            record("diagnostics", vec![sequence(Vec::new())]),
            record("outcome-values", vec![sequence(Vec::new())]),
            checks_value(&["canonical-run"]),
        ]);
        let transcript_artifact = install_fixture(&registry, "transcript-run-receipt", transcript_receipt, &[], &[]);
        let upgrade_receipt = record("upgrade-receipt-v1", vec![
            string(crate::preserves_rail::UPGRADE_RECEIPT_SCHEMA),
            record("operation", vec![string("session-create")]),
            record("decision", vec![string("pass")]),
            record("session", vec![string("session-catalog")]),
            record("plan", vec![string(test_ref("plan"))]),
            record("task", vec![record("none", Vec::new())]),
            record("refs", vec![sequence(Vec::new())]),
            checks_value(&["canonical-receipt"]),
        ]);
        let upgrade_artifact = install_fixture(&registry, "upgrade-receipt", upgrade_receipt, &[], &[]);
        let transcript = search(&registry, None, &SearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![Filter::TranscriptStatus("pass".to_string())],
            visibility: VisibilityInput::default(),
        })
        .expect("transcript search");
        assert_eq!(transcript.items.len(), 1);
        assert!(
            to_text(&transcript.value)
                .expect("transcript result text")
                .contains(&transcript_artifact.artifact_ref)
        );
        let upgrade = search(&registry, None, &SearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![Filter::UpgradeStatus("pass".to_string())],
            visibility: VisibilityInput::default(),
        })
        .expect("upgrade search");
        assert_eq!(upgrade.items.len(), 1);
        assert!(to_text(&upgrade.value).expect("upgrade result text").contains(&upgrade_artifact.artifact_ref));
        let receipt_view = receipts(&registry, None, &GraphInput {
            reference: transcript_artifact.artifact_ref,
            transitive: false,
            visibility: VisibilityInput::default(),
        })
        .expect("receipt view");
        assert!(!receipt_view.items.is_empty());
    }

    struct Values {
        verify: IoValue,
        divergence: IoValue,
        rollup: IoValue,
        index: IoValue,
    }

    fn verify_value(expected_report_ref: &str, actual_report_ref: &str, final_state_ref: &str) -> IoValue {
        record("deterministic-replay-verify-v1", vec![
            string(crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA),
            string("pass"),
            record("expected-report-ref", vec![string(expected_report_ref)]),
            record("actual-report-ref", vec![string(actual_report_ref)]),
            record("final-state-ref", vec![string(final_state_ref)]),
            record("divergence", vec![string("none")]),
            checks_value(&["report-replayed", "final-state-bound", "no-divergence"]),
        ])
    }

    fn divergence_value() -> IoValue {
        record("deterministic-first-divergence-v1", vec![
            string(crate::preserves_rail::DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA),
            record("kind", vec![string("effect-response")]),
            record("turn-id", vec![string("turn:1")]),
            record("actor-id", vec![string("actor:helper")]),
            record("log-position", vec![string("0")]),
            record("handler-profile-ref", vec![string(test_ref("handler-profile"))]),
            record("expected-ref", vec![string(test_ref("expected-effect"))]),
            record("actual-ref", vec![string(test_ref("actual-effect"))]),
            sequence(vec![string("safe-canonical-refs-only")]),
        ])
    }

    fn seed_values(expected_report_ref: &str, actual_report_ref: &str, final_state_ref: &str) -> Values {
        let verify = verify_value(expected_report_ref, actual_report_ref, final_state_ref);
        let verify_ref = canonical_hash(&verify).expect("verify ref");
        let rollup =
            crate::deterministic_replay::rollup_replay_receipts(&[crate::deterministic_replay::ReplayRollupInput {
                expected_ref: Some(verify_ref.clone()),
                value: verify.clone(),
            }])
            .expect("replay rollup");
        let index = crate::deterministic_replay::index_replay_evidence(&[
            crate::deterministic_replay::ReplayIndexInput {
                expected_ref: Some(verify_ref),
                value: verify.clone(),
            },
            crate::deterministic_replay::ReplayIndexInput {
                expected_ref: Some(rollup.rollup_ref.clone()),
                value: rollup.value.clone(),
            },
        ])
        .expect("replay index");
        Values {
            verify,
            divergence: divergence_value(),
            rollup: rollup.value,
            index: index.value,
        }
    }

    fn import_values(ledger_root: &std::path::Path, values: &Values) {
        let verify_import = crate::ledger::import_artifact(ledger_root, &values.verify).expect("import replay verify");
        crate::ledger::import_artifact(ledger_root, &values.divergence).expect("import first divergence");
        let rollup_import = crate::ledger::import_artifact(ledger_root, &values.rollup).expect("import replay rollup");
        let index_import = crate::ledger::import_artifact(ledger_root, &values.index).expect("import replay index");
        assert_eq!(verify_import.artifact_kind, "deterministic-replay-verify-receipt");
        assert_eq!(rollup_import.artifact_kind, "deterministic-replay-rollup");
        assert_eq!(index_import.artifact_kind, "deterministic-replay-index");
    }

    fn assert_found_text(
        registry: &std::path::Path,
        ledger_root: &std::path::Path,
        filters: Vec<Filter>,
        needles: &[&str],
    ) {
        let found = search(registry, Some(ledger_root), &SearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters,
            visibility: VisibilityInput::default(),
        })
        .expect("catalog search");
        assert_eq!(found.items.len(), 1);
        let text = to_text(&found.value).expect("catalog result text");
        for needle in needles {
            assert!(text.contains(needle), "missing catalog text {needle}");
        }
    }
