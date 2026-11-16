
    #[test]
    fn restart_health_receipt_requires_shutdown_indexes_and_no_open_jobs() {
        let adapters = required_adapter_bindings_scrambled();
        let config_value = test_node_config_value(&adapters);
        let (source_gate_ref, source_gate_value) = clean_source_gate();
        let started = start_node_runtime(&NodeRuntimeStartInput {
            config_value,
            identity_receipt_ref: test_ref("identity-receipt"),
            index_receipt_refs: vec![test_ref("startup-index")],
            source_gate_receipt_refs: vec![source_gate_ref],
            source_gate_receipt_values: vec![source_gate_value],
            profile_metadata_refs: vec![test_ref("profile-metadata")],
            capability_receipt_refs: vec![test_ref("capability-receipt")],
            resource_receipt_refs: vec![test_ref("resource-receipt")],
            version_refs: vec![test_ref("version")],
        })
        .expect("start runtime");
        let shutdown_index_refs = vec![test_ref("shutdown-index")];
        let shutdown = node_shutdown_receipt_value(&ShutdownReceiptValueInput {
            decision: "pass",
            startup_receipt_ref: &started.startup_receipt.receipt_ref,
            adapter_receipts: &started.adapter_receipts,
            drained_job_refs: &[],
            index_receipt_refs: &shutdown_index_refs,
            diagnostics: &[],
        })
        .expect("shutdown");
        let shutdown = parse_node_shutdown_receipt(&shutdown).expect("parse shutdown");
        let restart_index_refs = vec![test_ref("restart-index")];
        let head_refs = vec![test_ref("chain-head")];
        let healthy = node_restart_health_receipt_value(&RestartHealthReceiptValueInput {
            startup_receipt: &started.startup_receipt,
            shutdown_receipt_ref: Some(&shutdown.receipt_ref),
            index_receipt_refs: &restart_index_refs,
            head_refs: &head_refs,
            open_job_refs: &[],
            diagnostics: &[],
        })
        .expect("healthy restart");
        let health = parse_node_health_receipt(&healthy).expect("parse health");
        assert_eq!(health.decision, "pass");
        assert_eq!(health.replay_status, "eligible");
        assert_eq!(crate::ledger::artifact_kind(&healthy), "node-health-receipt");

        let unhealthy_head_refs = vec![test_ref("chain-head")];
        let open_job_refs = vec![test_ref("open-job")];
        let unhealthy = node_restart_health_receipt_value(&RestartHealthReceiptValueInput {
            startup_receipt: &started.startup_receipt,
            shutdown_receipt_ref: None,
            index_receipt_refs: &[],
            head_refs: &unhealthy_head_refs,
            open_job_refs: &open_job_refs,
            diagnostics: &[],
        })
        .expect("unhealthy restart");
        let unhealthy = parse_node_health_receipt(&unhealthy).expect("parse unhealthy");
        assert_eq!(unhealthy.decision, "deny");
        assert_eq!(unhealthy.replay_status, "ineligible");
        assert!(
            unhealthy
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("previous shutdown receipt missing"))
        );
        assert!(unhealthy.diagnostics.iter().any(|diagnostic| diagnostic.contains("adapter indexes not verified")));
        assert!(unhealthy.diagnostics.iter().any(|diagnostic| diagnostic.contains("open jobs")));
    }

    #[test]
    fn runtime_start_orders_adapters_and_binds_index_and_resource_receipts() {
        let adapters = required_adapter_bindings_scrambled();
        let config_value = test_node_config_value(&adapters);
        let index_ref = test_ref("index-verify");
        let resource_ref = test_ref("resource-profile");
        let (source_gate_ref, source_gate_value) = clean_source_gate();
        let started = start_node_runtime(&NodeRuntimeStartInput {
            config_value,
            identity_receipt_ref: test_ref("identity-receipt"),
            index_receipt_refs: vec![index_ref.clone()],
            source_gate_receipt_refs: vec![source_gate_ref],
            source_gate_receipt_values: vec![source_gate_value],
            profile_metadata_refs: vec![test_ref("profile-metadata")],
            capability_receipt_refs: vec![test_ref("capability-receipt")],
            resource_receipt_refs: vec![resource_ref.clone()],
            version_refs: vec![test_ref("version")],
        })
        .expect("start runtime");

        assert_eq!(started.decision, "pass");
        assert_eq!(
            started.adapter_receipts.iter().map(|receipt| receipt.name.as_str()).collect::<Vec<_>>(),
            REQUIRED_RUNTIME_ADAPTERS
        );
        let adapter_text = crate::preserves_rail::to_text(&started.adapter_receipt_values[0]).expect("adapter text");
        assert!(adapter_text.contains(&index_ref));
        assert!(adapter_text.contains(&resource_ref));
        assert_eq!(started.startup_receipt.decision, "pass");
    }

    #[test]
    fn runtime_start_denies_missing_index_resource_or_required_adapters() {
        let adapters = vec![node_adapter_binding("ledger", &test_ref("ledger-profile")).expect("ledger")];
        let config_value = test_node_config_value(&adapters);
        let started = start_node_runtime(&NodeRuntimeStartInput {
            config_value,
            identity_receipt_ref: test_ref("identity-receipt"),
            index_receipt_refs: Vec::new(),
            source_gate_receipt_refs: Vec::new(),
            source_gate_receipt_values: Vec::new(),
            profile_metadata_refs: Vec::new(),
            capability_receipt_refs: vec![test_ref("capability-receipt")],
            resource_receipt_refs: Vec::new(),
            version_refs: vec![test_ref("version")],
        })
        .expect("deny runtime start");

        assert_eq!(started.decision, "deny");
        assert!(
            started
                .startup_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("missing required node runtime adapters"))
        );
        assert!(
            started
                .startup_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("index verification"))
        );
        assert!(
            started
                .startup_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("strict Octet source gate"))
        );
        assert!(started.startup_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource profile")));
        assert!(started.startup_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("profile metadata")));
    }

    #[test]
    fn runtime_start_rejects_tampered_profile_metadata_ref() {
        let adapters = required_adapter_bindings_scrambled();
        let config_value = test_node_config_value(&adapters);
        let (source_gate_ref, source_gate_value) = clean_source_gate();
        let started = start_node_runtime(&NodeRuntimeStartInput {
            config_value,
            identity_receipt_ref: test_ref("identity-receipt"),
            index_receipt_refs: vec![test_ref("startup-index")],
            source_gate_receipt_refs: vec![source_gate_ref],
            source_gate_receipt_values: vec![source_gate_value],
            profile_metadata_refs: vec!["not-a-content-ref".to_string()],
            capability_receipt_refs: vec![test_ref("capability-receipt")],
            resource_receipt_refs: vec![test_ref("resource-receipt")],
            version_refs: vec![test_ref("version")],
        });

        assert!(started.is_err());
    }

    fn start_input_with_gate(gate_ref: String, gate_value: IoValue) -> NodeRuntimeStartInput {
        NodeRuntimeStartInput {
            config_value: test_node_config_value(&required_adapter_bindings_scrambled()),
            identity_receipt_ref: test_ref("identity-receipt"),
            index_receipt_refs: vec![test_ref("startup-index")],
            source_gate_receipt_refs: vec![gate_ref],
            source_gate_receipt_values: vec![gate_value],
            profile_metadata_refs: vec![test_ref("profile-metadata")],
            capability_receipt_refs: vec![test_ref("capability-receipt")],
            resource_receipt_refs: vec![test_ref("resource-receipt")],
            version_refs: vec![test_ref("version")],
        }
    }

    // r[verify molten.octet_source_gate_receipt_validation.spec.unqualified_fixture_denied]
    // r[verify molten.operator_dogfood_node_workflow.spec.unqualified_fixture_gate]
    #[test]
    fn fixture_gate_is_denied_by_scan_startup_and_labelled_in_fixture_startup() {
        let fixture = crate::octet_gate::UnqualifiedFixtureGate::new("dogfood:local-node").expect("fixture gate");
        let input = start_input_with_gate(fixture.gate_ref().to_string(), fixture.value().clone());

        let scanned = start_node_runtime(&input).expect("scan start");
        assert_eq!(scanned.decision, "deny");
        assert!(scanned.startup_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("source gate")));

        let started = start_node_runtime_with_unqualified_fixture_gate(&input, &fixture).expect("fixture start");
        assert_eq!(started.decision, "pass");
        assert_eq!(started.startup_receipt.source_gate_validation_refs, vec![fixture.validation_ref().to_string()]);
        let receipt = crate::preserves_rail::to_text(&started.startup_receipt.value).expect("startup text");
        assert!(receipt.contains(r#"<check "strict-octet-source-gate-bound" "fail">"#), "{receipt}");
        assert!(receipt.contains(r#"<check "unqualified-fixture-source-gate" "pass">"#), "{receipt}");

        let (clean_ref, clean_value) = clean_source_gate();
        let strict = start_node_runtime(&start_input_with_gate(clean_ref.clone(), clean_value.clone())).expect("start");
        let strict_text = crate::preserves_rail::to_text(&strict.startup_receipt.value).expect("startup text");
        assert!(strict_text.contains(r#"<check "strict-octet-source-gate-bound" "pass">"#), "{strict_text}");
        assert!(!strict_text.contains("unqualified-fixture-source-gate"));
        start_node_runtime_with_unqualified_fixture_gate(&start_input_with_gate(clean_ref, clean_value), &fixture)
            .expect_err("fixture mode only binds the fixture value");
    }
