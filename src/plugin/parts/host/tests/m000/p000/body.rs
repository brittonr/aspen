    use super::*;

    type ListInput = crate::catalog::ListInput;
    type VisibilityInput = crate::catalog::VisibilityInput;

    const TEST_GRANT_VALID_UNTIL_TURN: u64 = 16;
    const TEST_GRANT_EXPIRED_TURN: u64 = TEST_GRANT_VALID_UNTIL_TURN + 1;
    const TEST_GRANT_MAX_DELEGATION_DEPTH: u64 = 2;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn manifest_value_for_artifact(artifact_ref: &str) -> IoValue {
        let lifecycle_callbacks = string_vec(&["init", "start", "health", "stop", "remove"]);
        let effect_refs = vec![test_ref("effect")];
        let hostcall_refs = vec![storage_read_hostcall_ref().expect("hostcall ref")];
        let schema_refs = vec![test_ref("schema")];
        let policy_refs = vec![test_ref("policy")];
        let resource_refs = vec![test_ref("resource")];
        let supply_refs = vec![test_ref("supply")];
        plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:test",
            artifact_ref,
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &lifecycle_callbacks,
            effect_manifest_refs: &effect_refs,
            hostcall_refs: &hostcall_refs,
            schema_refs: &schema_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            supply_chain_refs: &supply_refs,
            extension_contract_refs: &[],
        })
        .expect("manifest")
    }

    #[test]
    fn plugin_fixture_runs_lifecycle_and_upgrade() {
        let dir = temp_dir("plugin-fixture");
        let run = minimal_plugin_fixture(&dir).expect("minimal plugin fixture");
        assert_eq!(run.decision, PLUGIN_DECISION_PASS);
        crate::preserves_rail::validate_content_ref(&run.manifest_ref).expect("manifest ref is canonical");
        crate::preserves_rail::validate_content_ref(&run.install_receipt_ref)
            .expect("install receipt ref is canonical");
        assert!(plugin_summary(&run.report_value).expect("summary").contains("plugin fixture report"));
        assert!(run.evidence_values.len() >= 10);
    }

    #[test]
    fn raw_host_path_missing_artifact_and_stale_provenance_deny() {
        let malformed = parse_text(
            "<plugin-manifest-v1 \"molten.plugin.manifest.v1\" \
             <plugin-id \"plugin:path\"> <artifact \"/usr/bin/plugin\"> <abi \"molten.plugin.host-abi.v1\"> \
             <lifecycle [\"start\"]> <effects []> <hostcalls []> <schemas []> <policy []> <resource []> \
             <supply-chain []> <checks [<check \"artifact-backed\" \"fail\"> <check \"no-ambient-authority\" \"pass\">]>>",
        )
        .expect("parse malformed manifest");
        assert!(parse_plugin_manifest(&malformed).is_err());

        let dir = temp_dir("plugin-deny");
        let registry = dir.join("registry");
        let artifact = crate::artifacts::install_artifact(&registry, &crate::artifacts::ArtifactInstallInput {
            kind: "plugin-executor".to_string(),
            payload: record("plugin", vec![string("x")]),
            schema_refs: vec![test_ref("schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: Some(test_ref("effect")),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("supply")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install artifact");
        let manifest = manifest_value_for_artifact(&artifact.artifact_ref);
        let install = install_plugin(&registry, &manifest).expect("install plugin");
        assert_eq!(install.decision, PLUGIN_DECISION_PASS);
        let permission = plugin_permission_receipt_value(&PermissionReviewInput {
            manifest_value: &manifest,
            authority_refs: &[test_ref("authority")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            effect_receipt_refs: &[test_ref("effect-receipt")],
            supply_chain_refs: &[test_ref("stale-supply")],
        })
        .expect("permission receipt");
        let parsed = parse_plugin_permission_receipt(&permission).expect("parse permission");
        assert_eq!(parsed.decision, PLUGIN_DECISION_DENY);
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("supply-chain")));
    }

    #[test]
    fn ambient_hostcall_failed_health_and_cleanup_are_receipted() {
        let manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:ambient",
            artifact_ref: &test_ref("artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &[
                "start".to_string(),
                "health".to_string(),
                "stop".to_string(),
                "remove".to_string(),
            ],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("storage hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
            extension_contract_refs: &[],
        })
        .expect("manifest");
        let denied_hostcall = plugin_hostcall_receipt_value(&HostcallReceiptInput {
            manifest_value: &manifest,
            operation: "network.open",
            hostcall_ref: &network_open_hostcall_ref().expect("network hostcall"),
            executor_receipt_ref: &test_ref("executor"),
            effect_receipt_ref: &test_ref("effect-receipt"),
            authority_refs: &[test_ref("authority")],
            capability_grants: &[],
            resource_refs: &[test_ref("resource")],
            extension_contracts: &[],
            input_schema_ref: None,
            output_schema_ref: None,
            evaluation_turn: PLUGIN_INITIAL_TURN,
        })
        .expect("hostcall receipt");
        let denied = parse_plugin_hostcall_receipt(&denied_hostcall).expect("parse hostcall");
        assert_eq!(denied.decision, PLUGIN_DECISION_DENY);
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("ambient")));
        let health = plugin_health_receipt_value(&HealthReceiptInput {
            manifest_value: &manifest,
            lifecycle_receipt_ref: &test_ref("start"),
            service_refs: &[test_ref("service")],
            health_status: "failed",
            diagnostics: &["probe failed".to_string()],
        })
        .expect("health receipt");
        assert_eq!(parse_plugin_health_receipt(&health).expect("parse health").decision, PLUGIN_DECISION_DENY);
        let incomplete_removal = plugin_removal_receipt_value(&RemovalReceiptInput {
            manifest_value: &manifest,
            lifecycle_receipt_ref: &test_ref("remove"),
            owned_service_refs: &[test_ref("service")],
            assertion_refs: &[],
            handle_refs: &[],
            catalog_entry_refs: &[],
            diagnostics: &[],
        })
        .expect("removal receipt");
        assert_eq!(
            parse_plugin_removal_receipt(&incomplete_removal)
                .expect("parse removal")
                .decision,
            PLUGIN_DECISION_DENY
        );
    }

    #[test]
    fn hostcall_operation_ref_binding_accepts_matching_and_denies_mismatch() {
        let manifest = manifest_value_for_artifact(&test_ref("artifact-binding"));
        let storage_ref = storage_read_hostcall_ref().expect("storage hostcall ref");
        let pass = plugin_hostcall_receipt_value(&HostcallReceiptInput {
            manifest_value: &manifest,
            operation: "storage.read",
            hostcall_ref: &storage_ref,
            executor_receipt_ref: &test_ref("executor"),
            effect_receipt_ref: &test_ref("effect-receipt"),
            authority_refs: &[test_ref("authority")],
            capability_grants: &[],
            resource_refs: &[test_ref("resource")],
            extension_contracts: &[],
            input_schema_ref: None,
            output_schema_ref: None,
            evaluation_turn: PLUGIN_INITIAL_TURN,
        })
        .expect("matching hostcall receipt");
        assert_eq!(parse_plugin_hostcall_receipt(&pass).expect("parse pass").decision, PLUGIN_DECISION_PASS);

        let mismatch = plugin_hostcall_receipt_value(&HostcallReceiptInput {
            manifest_value: &manifest,
            operation: "network.open",
            hostcall_ref: &storage_ref,
            executor_receipt_ref: &test_ref("executor"),
            effect_receipt_ref: &test_ref("effect-receipt"),
            authority_refs: &[test_ref("authority")],
            capability_grants: &[],
            resource_refs: &[test_ref("resource")],
            extension_contracts: &[],
            input_schema_ref: None,
            output_schema_ref: None,
            evaluation_turn: PLUGIN_INITIAL_TURN,
        })
        .expect("mismatched hostcall receipt");
        let parsed = parse_plugin_hostcall_receipt(&mismatch).expect("parse mismatch");
        assert_eq!(parsed.decision, PLUGIN_DECISION_DENY);
        assert!(parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("operation/ref binding mismatch")));
    }

    #[test]
    fn forged_pass_and_empty_deny_receipts_are_rejected() {
        let manifest = parse_plugin_manifest(&manifest_value_for_artifact(&test_ref("artifact-forged"))).expect("manifest");
        let forged_pass = record("plugin-hostcall-receipt-v1", vec![
            string(crate::preserves_rail::PLUGIN_HOSTCALL_RECEIPT_SCHEMA),
            record("decision", vec![string(PLUGIN_DECISION_PASS)]),
            record("plugin", vec![string(&manifest.plugin_ref)]),
            record("manifest", vec![string(&manifest.manifest_ref)]),
            record("operation", vec![string("storage.read")]),
            record("hostcall", vec![string(storage_read_hostcall_ref().expect("hostcall"))]),
            record("executor", vec![string(test_ref("executor"))]),
            record("effect", vec![string(test_ref("effect"))]),
            record("authority", vec![refs_sequence(&[test_ref("authority")])]),
            record("capability-grants", vec![refs_sequence(&Vec::<String>::new())]),
            record("resource", vec![refs_sequence(&[test_ref("resource")])]),
            record("evaluation-turn", vec![u64_value(PLUGIN_INITIAL_TURN)]),
            record("diagnostics", vec![strings_sequence(&Vec::<String>::new())]),
            checks_value(&[
                ("declared-hostcall", PLUGIN_CHECK_FAIL),
                ("operation-ref-bound", PLUGIN_DECISION_PASS),
                ("effect-handle-boundary", PLUGIN_DECISION_PASS),
                ("capability-grant-match", PLUGIN_DECISION_PASS),
            ]),
        ]);
        assert!(parse_plugin_hostcall_receipt(&forged_pass).is_err());

        let empty_deny = record("plugin-hostcall-receipt-v1", vec![
            string(crate::preserves_rail::PLUGIN_HOSTCALL_RECEIPT_SCHEMA),
            record("decision", vec![string(PLUGIN_DECISION_DENY)]),
            record("plugin", vec![string(&manifest.plugin_ref)]),
            record("manifest", vec![string(&manifest.manifest_ref)]),
            record("operation", vec![string("storage.read")]),
            record("hostcall", vec![string(storage_read_hostcall_ref().expect("hostcall"))]),
            record("executor", vec![string(test_ref("executor"))]),
            record("effect", vec![string(test_ref("effect"))]),
            record("authority", vec![refs_sequence(&[test_ref("authority")])]),
            record("capability-grants", vec![refs_sequence(&Vec::<String>::new())]),
            record("resource", vec![refs_sequence(&[test_ref("resource")])]),
            record("evaluation-turn", vec![u64_value(PLUGIN_INITIAL_TURN)]),
            record("diagnostics", vec![strings_sequence(&Vec::<String>::new())]),
            checks_value(&[
                ("declared-hostcall", PLUGIN_DECISION_PASS),
                ("operation-ref-bound", PLUGIN_DECISION_PASS),
                ("effect-handle-boundary", PLUGIN_DECISION_PASS),
                ("capability-grant-match", PLUGIN_DECISION_PASS),
            ]),
        ]);
        assert!(parse_plugin_hostcall_receipt(&empty_deny).is_err());
    }
