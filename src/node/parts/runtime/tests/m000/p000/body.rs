    use super::*;

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("node-runtime-test-ref", vec![string(label)])).expect("test ref")
    }

    fn clean_source_gate() -> (String, IoValue) {
        let value =
            crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean octet gate fixture");
        let reference = canonical_hash(&value).expect("octet gate ref");
        (reference, value)
    }

    fn required_adapter_bindings_scrambled() -> Vec<NodeAdapterBinding> {
        [
            "jobs",
            "ledger",
            "catalog-mcp",
            "control",
            "cache",
            "coordination",
            "registry",
            "remote-dataspace",
            "plugin-host",
            "chunks",
            "services",
            "storage",
        ]
        .iter()
        .map(|name| node_adapter_binding(name, &test_ref(&format!("{name}-profile"))).expect("adapter"))
        .collect()
    }

    fn test_node_config_value(adapters: &[NodeAdapterBinding]) -> IoValue {
        let identity_ref = test_ref("node-id");
        let state_root_ref = test_ref("state-root");
        let policy_refs = vec![test_ref("policy")];
        let capability_refs = vec![test_ref("capability")];
        let resource_refs = vec![test_ref("resource")];
        let effect_profile_refs = vec![test_ref("effects")];
        node_config_value(&ConfigValueInput {
            identity_ref: &identity_ref,
            state_root_ref: &state_root_ref,
            adapters,
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            resource_refs: &resource_refs,
            effect_profile_refs: &effect_profile_refs,
        })
        .expect("node config")
    }

    #[test]
    fn node_config_requires_explicit_state_and_adapters() {
        let adapters = vec![node_adapter_binding("ledger", &test_ref("ledger-profile")).expect("adapter")];
        let value = test_node_config_value(&adapters);
        let config = parse_node_config(&value).expect("parse config");
        assert_eq!(config.adapters[0].name, "ledger");
        assert_eq!(crate::ledger::artifact_kind(&value), "node-config");
        let identity_ref = test_ref("node-id");
        let state_root_ref = test_ref("state-root");
        assert!(
            node_config_value(&ConfigValueInput {
                identity_ref: &identity_ref,
                state_root_ref: "./state",
                adapters: &adapters,
                policy_refs: &[],
                capability_refs: &[],
                resource_refs: &[],
                effect_profile_refs: &[],
            })
            .is_err()
        );
        assert!(
            node_config_value(&ConfigValueInput {
                identity_ref: &identity_ref,
                state_root_ref: &state_root_ref,
                adapters: &[],
                policy_refs: &[],
                capability_refs: &[],
                resource_refs: &[],
                effect_profile_refs: &[],
            })
            .is_err()
        );
    }

    #[test]
    fn startup_receipt_binds_config_identity_and_adapter_receipts() {
        let adapters = vec![
            node_adapter_binding("ledger", &test_ref("ledger-profile")).expect("ledger"),
            node_adapter_binding("registry", &test_ref("registry-profile")).expect("registry"),
        ];
        let config_value = test_node_config_value(&adapters);
        let config = parse_node_config(&config_value).expect("parse config");
        let adapter_receipts = adapters
            .iter()
            .map(|adapter| {
                let receipt = node_adapter_receipt_value("start", "pass", adapter, &[]).expect("adapter receipt");
                NodeAdapterReceiptRef {
                    name: adapter.name.clone(),
                    receipt_ref: canonical_hash(&receipt).expect("adapter receipt ref"),
                }
            })
            .collect::<Vec<_>>();
        let identity_receipt_ref = test_ref("identity-receipt");
        let source_gate_receipt_refs = vec![test_ref("octet-gate-receipt")];
        let source_gate_validation_refs = vec![test_ref("octet-source-gate-validation")];
        let capability_receipt_refs = vec![test_ref("capability-receipt")];
        let resource_receipt_refs = vec![test_ref("resource-receipt")];
        let version_refs = vec![test_ref("version")];
        let receipt_value = node_startup_receipt_value(&StartupReceiptValueInput {
            decision: "pass",
            config: &config,
            identity_receipt_ref: &identity_receipt_ref,
            adapter_receipts: &adapter_receipts,
            source_gate_receipt_refs: &source_gate_receipt_refs,
            source_gate_validation_refs: &source_gate_validation_refs,
            capability_receipt_refs: &capability_receipt_refs,
            resource_receipt_refs: &resource_receipt_refs,
            version_refs: &version_refs,
            diagnostics: &[],
        })
        .expect("startup receipt");
        let receipt = parse_node_startup_receipt(&receipt_value).expect("parse startup");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.config_ref, config.config_ref);
        assert_eq!(receipt.source_gate_receipt_refs, vec![test_ref("octet-gate-receipt")]);
        assert_eq!(receipt.source_gate_validation_refs, vec![test_ref("octet-source-gate-validation")]);
        assert_eq!(crate::ledger::artifact_kind(&receipt_value), "node-startup-receipt");
    }

    #[test]
    fn adapter_lifecycle_receipts_cover_verify_deny_and_shutdown_decisions() {
        let adapter = node_adapter_binding("ledger", &test_ref("ledger-profile")).expect("adapter");
        for (operation, decision) in [("verify", "pass"), ("deny", "deny"), ("shutdown", "pass")] {
            let index_refs = vec![test_ref("index")];
            let resource_refs = vec![test_ref("resource")];
            let receipt = node_adapter_lifecycle_receipt_value(&AdapterLifecycleReceiptInput {
                operation,
                decision,
                adapter: &adapter,
                index_receipt_refs: &index_refs,
                resource_receipt_refs: &resource_refs,
                diagnostics: &[],
            })
            .expect("adapter lifecycle receipt");
            let text = crate::preserves_rail::to_text(&receipt).expect("receipt text");
            assert!(text.contains(operation));
            assert!(text.contains(decision));
            assert_eq!(crate::ledger::artifact_kind(&receipt), "node-adapter-receipt");
        }
    }

    #[test]
    fn control_request_and_receipt_bind_authority_resource_and_subreceipts() {
        let target_ref = test_ref("target");
        let payload_ref = test_ref("payload");
        let authority_refs = vec![test_ref("authority")];
        let policy_refs = vec![test_ref("policy")];
        let resource_refs = vec![test_ref("resource")];
        let request_value = control_request_value(&ControlRequestValueInput {
            operation: "install",
            target_ref: Some(&target_ref),
            payload_ref: Some(&payload_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("control request");
        let request = parse_control_request(&request_value).expect("parse control request");
        let startup_ref = test_ref("startup");
        let authority_receipt_refs = vec![test_ref("authority-receipt")];
        let resource_receipt_refs = vec![test_ref("resource-receipt")];
        let subreceipt_refs = vec![test_ref("artifact-install-receipt")];
        let receipt_value = control_receipt_value(&ControlReceiptValueInput {
            decision: "pass",
            request: &request,
            startup_receipt_ref: &startup_ref,
            authority_receipt_refs: &authority_receipt_refs,
            resource_receipt_refs: &resource_receipt_refs,
            subreceipt_refs: &subreceipt_refs,
            diagnostics: &[],
        })
        .expect("control receipt");
        let receipt = parse_control_receipt(&receipt_value).expect("parse control receipt");

        assert_eq!(request.operation, "install");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.request_ref, request.request_ref);
        assert_eq!(crate::ledger::artifact_kind(&request_value), "node-control-request");
        assert_eq!(crate::ledger::artifact_kind(&receipt_value), "node-control-receipt");
    }

    #[test]
    fn control_request_rejects_short_fixture_ref_shape() {
        let authority_refs = vec![test_ref("authority")];
        let policy_refs = vec![test_ref("policy")];
        let resource_refs = vec![test_ref("resource")];
        let error = control_request_value(&ControlRequestValueInput {
            operation: "install",
            target_ref: Some("blake3:target-fixture"),
            payload_ref: Some("blake3:payload-fixture"),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect_err("short fixture refs are rejected");

        assert!(error.to_string().contains("canonical blake3 content ref"));
    }

    #[test]
    fn control_denial_is_canonical_when_authority_or_resource_evidence_is_missing() {
        let payload_ref = test_ref("payload");
        let request_value = control_request_value(&ControlRequestValueInput {
            operation: "gate",
            target_ref: None,
            payload_ref: Some(&payload_ref),
            authority_refs: &[],
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("request");
        let request = parse_control_request(&request_value).expect("parse request");
        let receipt_value =
            control_deny_receipt_value(&request, &test_ref("startup"), "missing authority/resource evidence")
                .expect("deny receipt");
        let receipt = parse_control_receipt(&receipt_value).expect("parse receipt");
        let text = crate::preserves_rail::to_text(&receipt_value).expect("receipt text");

        assert_eq!(receipt.decision, "deny");
        assert!(text.contains("authority-gated"));
        assert!(text.contains("resource-gated"));
        assert!(text.contains("missing authority/resource evidence"));
    }

    #[test]
    fn shutdown_receipt_binds_drain_index_and_adapter_close_evidence() {
        let adapter = NodeAdapterReceiptRef {
            name: "ledger".to_string(),
            receipt_ref: test_ref("ledger-shutdown"),
        };
        let drained = vec![test_ref("job-drained")];
        let index = vec![test_ref("index-persisted")];
        let startup_ref = test_ref("startup");
        let receipt_value = node_shutdown_receipt_value(&ShutdownReceiptValueInput {
            decision: "pass",
            startup_receipt_ref: &startup_ref,
            adapter_receipts: std::slice::from_ref(&adapter),
            drained_job_refs: &drained,
            index_receipt_refs: &index,
            diagnostics: &[],
        })
        .expect("shutdown receipt");
        let receipt = parse_node_shutdown_receipt(&receipt_value).expect("parse shutdown receipt");
        let text = crate::preserves_rail::to_text(&receipt_value).expect("shutdown text");

        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.adapters, vec![adapter]);
        assert_eq!(receipt.drained_job_refs, drained);
        assert_eq!(receipt.index_receipt_refs, index);
        assert!(text.contains("graceful-shutdown"));
        assert_eq!(crate::ledger::artifact_kind(&receipt_value), "node-shutdown-receipt");
    }
