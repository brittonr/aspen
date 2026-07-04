
#[cfg(test)]
mod tests {
    use super::*;

    type ListInput = crate::catalog::ListInput;
    type VisibilityInput = crate::catalog::VisibilityInput;

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
            resource_refs: &[test_ref("resource")],
            extension_contracts: &[],
            input_schema_ref: None,
            output_schema_ref: None,
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
            resource_refs: &[test_ref("resource")],
            extension_contracts: &[],
            input_schema_ref: None,
            output_schema_ref: None,
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
            resource_refs: &[test_ref("resource")],
            extension_contracts: &[],
            input_schema_ref: None,
            output_schema_ref: None,
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
            record("resource", vec![refs_sequence(&[test_ref("resource")])]),
            record("diagnostics", vec![strings_sequence(&Vec::<String>::new())]),
            checks_value(&[
                ("declared-hostcall", PLUGIN_CHECK_FAIL),
                ("operation-ref-bound", PLUGIN_DECISION_PASS),
                ("effect-handle-boundary", PLUGIN_DECISION_PASS),
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
            record("resource", vec![refs_sequence(&[test_ref("resource")])]),
            record("diagnostics", vec![strings_sequence(&Vec::<String>::new())]),
            checks_value(&[
                ("declared-hostcall", PLUGIN_DECISION_PASS),
                ("operation-ref-bound", PLUGIN_DECISION_PASS),
                ("effect-handle-boundary", PLUGIN_DECISION_PASS),
            ]),
        ]);
        assert!(parse_plugin_hostcall_receipt(&empty_deny).is_err());
    }

    #[test]
    fn stale_manifest_receipts_deny_lifecycle_use() {
        let fixture = lifecycle_proof_fixture("plugin-stale-manifest");
        let stale_manifest_ref = test_ref("stale-manifest");
        let stale_hostcall = PluginHostcallReceipt {
            manifest_ref: stale_manifest_ref.clone(),
            ..fixture.hostcall.clone()
        };
        let hostcall_decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
            evaluation_kind: PluginLifecycleEvaluationKind::HostcallRequest,
            manifest: &fixture.manifest,
            install: Some(&fixture.install),
            permission: Some(&fixture.permission),
            activation: Some(&fixture.start),
            hostcall: Some(&stale_hostcall),
            health: Some(&fixture.health),
            removal: None,
            upgrade: None,
            negotiation: None,
            compatibility: None,
            recovery_receipt_ref: None,
        })
        .expect("evaluate stale hostcall");
        assert_eq!(hostcall_decision.decision, PLUGIN_DECISION_DENY);
        assert!(hostcall_decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PLUGIN_LIFECYCLE_HOSTCALL_BINDING_MISMATCH));

        let stale_health = PluginHealthReceipt {
            manifest_ref: stale_manifest_ref.clone(),
            ..fixture.health.clone()
        };
        let health_decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
            evaluation_kind: PluginLifecycleEvaluationKind::UpgradeRequest,
            manifest: &fixture.manifest,
            install: Some(&fixture.install),
            permission: Some(&fixture.permission),
            activation: Some(&fixture.start),
            hostcall: None,
            health: Some(&stale_health),
            removal: None,
            upgrade: Some(&fixture.upgrade),
            negotiation: None,
            compatibility: None,
            recovery_receipt_ref: None,
        })
        .expect("evaluate stale health");
        assert_eq!(health_decision.decision, PLUGIN_DECISION_DENY);
        assert!(health_decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PLUGIN_LIFECYCLE_HEALTH_FAILED));

        let stale_removal = PluginRemovalReceipt {
            manifest_ref: stale_manifest_ref,
            ..fixture.removal.clone()
        };
        let removal_decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
            evaluation_kind: PluginLifecycleEvaluationKind::RemovalRequest,
            manifest: &fixture.manifest,
            install: Some(&fixture.install),
            permission: Some(&fixture.permission),
            activation: Some(&fixture.start),
            hostcall: None,
            health: Some(&fixture.health),
            removal: Some(&stale_removal),
            upgrade: None,
            negotiation: None,
            compatibility: None,
            recovery_receipt_ref: None,
        })
        .expect("evaluate stale removal");
        assert_eq!(removal_decision.decision, PLUGIN_DECISION_DENY);
        assert!(removal_decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PLUGIN_LIFECYCLE_REMOVAL_BINDING_MISMATCH));
    }

    #[test]
    fn host_abi_result_and_upgrade_compatibility_are_canonical() {
        let payload_ref = test_ref("payload");
        let result = plugin_host_abi_result_value(&HostAbiResultInput {
            status: "ok",
            payload_ref: Some(&payload_ref),
            error: None,
        })
        .expect("ABI result");
        assert!(to_text(&result).expect("render result").contains("plugin-host-abi-result-v1"));
        let old_manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:upgrade",
            artifact_ref: &test_ref("old-artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string()],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
            extension_contract_refs: &[],
        })
        .expect("old manifest");
        let new_manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:upgrade",
            artifact_ref: &test_ref("new-artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string()],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema"), test_ref("schema-extra")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
            extension_contract_refs: &[],
        })
        .expect("new manifest");
        let upgrade = plugin_upgrade_receipt_value(&UpgradeReceiptInput {
            old_manifest_value: &old_manifest,
            new_manifest_value: &new_manifest,
            rollback_ref: &test_ref("rollback"),
            cleanup_refs: &[test_ref("cleanup")],
            diagnostics: &[],
        })
        .expect("upgrade receipt");
        assert_eq!(parse_plugin_upgrade_receipt(&upgrade).expect("parse upgrade").decision, PLUGIN_DECISION_PASS);
    }

    #[test]
    fn extension_contract_artifacts_parse_and_are_classified() {
        let contract = storage_extension_contract(PLUGIN_PROFILE_PRODUCTION, "1.0.0");
        assert_eq!(crate::ledger::artifact_kind(&contract.value), "plugin-extension-contract");
        assert_eq!(contract.extension_id, "plugin-extension:storage");
        assert!(contract.production_profile);
        let manifest = manifest_with_extension_refs(&[contract.contract_ref.clone()], &test_ref("effect-extension"));
        assert!(manifest.extension_contract_refs.contains(&contract.contract_ref));
    }

    #[test]
    fn checked_in_nickel_exported_contract_fixtures_validate() {
        let valid_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/plugin-extension-contracts/storage.contract.preserves"
        ));
        let valid_value = parse_text(valid_source).expect("parse checked-in contract export");
        let valid_contract = parse_plugin_extension_contract(&valid_value).expect("validate checked-in contract export");
        assert_eq!(valid_contract.extension_id, "plugin-extension:storage");

        let invalid_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/plugin-extension-contracts/storage-missing-schema.contract.preserves"
        ));
        let invalid_value = parse_text(invalid_source).expect("parse invalid checked-in export");
        assert!(parse_plugin_extension_contract(&invalid_value).is_err());
    }

    #[test]
    fn contract_aware_hostcall_requires_descriptor_specific_evidence() {
        let contract = storage_extension_contract(PLUGIN_PROFILE_PRODUCTION, "1.0.0");
        let descriptor = &contract.hostcall_descriptors[0];
        let manifest_value = manifest_value_with_extension_refs(
            &[contract.contract_ref.clone()],
            &descriptor.effect_manifest_refs[0],
        );
        let generic_deny = plugin_hostcall_receipt_value(&HostcallReceiptInput {
            manifest_value: &manifest_value,
            operation: &descriptor.operation,
            hostcall_ref: &descriptor.descriptor_ref,
            executor_receipt_ref: &test_ref("executor"),
            effect_receipt_ref: &test_ref("effect-receipt"),
            authority_refs: &[test_ref("unrelated-authority")],
            resource_refs: &[test_ref("unrelated-resource")],
            extension_contracts: std::slice::from_ref(&contract),
            input_schema_ref: Some(&descriptor.input_schema_ref),
            output_schema_ref: Some(&descriptor.output_schema_ref),
        })
        .expect("generic authority denial receipt");
        let denied = parse_plugin_hostcall_receipt(&generic_deny).expect("parse generic deny");
        assert_eq!(denied.decision, PLUGIN_DECISION_DENY);
        assert!(denied
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("descriptor-specific")));

        let pass = plugin_hostcall_receipt_value(&HostcallReceiptInput {
            manifest_value: &manifest_value,
            operation: &descriptor.operation,
            hostcall_ref: &descriptor.descriptor_ref,
            executor_receipt_ref: &test_ref("executor"),
            effect_receipt_ref: &test_ref("effect-receipt"),
            authority_refs: &descriptor.authority_refs,
            resource_refs: &descriptor.resource_refs,
            extension_contracts: std::slice::from_ref(&contract),
            input_schema_ref: Some(&descriptor.input_schema_ref),
            output_schema_ref: Some(&descriptor.output_schema_ref),
        })
        .expect("descriptor-specific hostcall pass");
        assert_eq!(parse_plugin_hostcall_receipt(&pass).expect("parse pass").decision, PLUGIN_DECISION_PASS);
    }

    #[test]
    fn extension_negotiation_denies_missing_required_and_allows_optional_omission() {
        let contract = storage_extension_contract(PLUGIN_PROFILE_PRODUCTION, "1.0.0");
        let manifest = manifest_with_extension_refs(&[contract.contract_ref.clone()], &test_ref("effect-extension"));
        let missing = plugin_extension_negotiation_receipt_value(&PluginExtensionNegotiationInput {
            manifest: &manifest,
            required_contract_refs: std::slice::from_ref(&contract.contract_ref),
            optional_contract_refs: &[],
            host_supported_contract_refs: &[],
            host_feature_snapshot_ref: &test_ref("host-features"),
            extension_contracts: std::slice::from_ref(&contract),
            production_profile: true,
            allow_optional_omission: true,
        })
        .expect("missing required negotiation");
        assert_eq!(
            parse_plugin_extension_negotiation_receipt(&missing)
                .expect("parse missing required")
                .decision,
            PLUGIN_DECISION_DENY
        );

        let optional = plugin_extension_negotiation_receipt_value(&PluginExtensionNegotiationInput {
            manifest: &manifest,
            required_contract_refs: &[],
            optional_contract_refs: std::slice::from_ref(&contract.contract_ref),
            host_supported_contract_refs: &[],
            host_feature_snapshot_ref: &test_ref("host-features"),
            extension_contracts: std::slice::from_ref(&contract),
            production_profile: true,
            allow_optional_omission: true,
        })
        .expect("optional omission negotiation");
        assert_eq!(
            parse_plugin_extension_negotiation_receipt(&optional)
                .expect("parse optional omission")
                .decision,
            PLUGIN_DECISION_PASS
        );
    }

    #[test]
    fn production_negotiation_denies_diagnostic_only_conformance() {
        let contract = storage_extension_contract(PLUGIN_PROFILE_DEVELOPMENT, "1.0.0");
        let manifest = manifest_with_extension_refs(&[contract.contract_ref.clone()], &test_ref("effect-extension"));
        let receipt = plugin_extension_negotiation_receipt_value(&PluginExtensionNegotiationInput {
            manifest: &manifest,
            required_contract_refs: std::slice::from_ref(&contract.contract_ref),
            optional_contract_refs: &[],
            host_supported_contract_refs: std::slice::from_ref(&contract.contract_ref),
            host_feature_snapshot_ref: &test_ref("host-features"),
            extension_contracts: std::slice::from_ref(&contract),
            production_profile: true,
            allow_optional_omission: true,
        })
        .expect("production conformance denial");
        let parsed = parse_plugin_extension_negotiation_receipt(&receipt).expect("parse conformance denial");
        assert_eq!(parsed.decision, PLUGIN_DECISION_DENY);
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("production conformance")));
    }

    #[test]
    fn extension_compatibility_passes_upgrade_and_denies_downgrade_or_removed_hostcall() {
        let old_contract = storage_extension_contract(PLUGIN_PROFILE_PRODUCTION, "1.0.0");
        let new_contract = storage_extension_contract(PLUGIN_PROFILE_PRODUCTION, "1.1.0");
        let old_manifest = manifest_with_extension_refs(&[old_contract.contract_ref.clone()], &test_ref("effect-extension"));
        let new_manifest = manifest_with_extension_refs(&[new_contract.contract_ref.clone()], &test_ref("effect-extension"));
        let pass = plugin_extension_compatibility_receipt_value(&PluginExtensionCompatibilityInput {
            old_manifest: &old_manifest,
            new_manifest: &new_manifest,
            old_contracts: std::slice::from_ref(&old_contract),
            new_contracts: std::slice::from_ref(&new_contract),
            migration_refs: &[],
            rollback_ref: &test_ref("rollback"),
            cleanup_refs: &[test_ref("cleanup")],
            production_profile: true,
        })
        .expect("compatible extension upgrade");
        assert_eq!(
            parse_plugin_extension_compatibility_receipt(&pass)
                .expect("parse compatible upgrade")
                .decision,
            PLUGIN_DECISION_PASS
        );

        let downgrade_contract = storage_extension_contract(PLUGIN_PROFILE_PRODUCTION, "0.9.0");
        let downgrade_manifest = manifest_with_extension_refs(
            &[downgrade_contract.contract_ref.clone()],
            &test_ref("effect-extension"),
        );
        let downgrade = plugin_extension_compatibility_receipt_value(&PluginExtensionCompatibilityInput {
            old_manifest: &old_manifest,
            new_manifest: &downgrade_manifest,
            old_contracts: std::slice::from_ref(&old_contract),
            new_contracts: std::slice::from_ref(&downgrade_contract),
            migration_refs: &[],
            rollback_ref: &test_ref("rollback"),
            cleanup_refs: &[test_ref("cleanup")],
            production_profile: true,
        })
        .expect("downgrade receipt");
        assert_eq!(
            parse_plugin_extension_compatibility_receipt(&downgrade)
                .expect("parse downgrade")
                .decision,
            PLUGIN_DECISION_DENY
        );

        let removed_contract = storage_extension_contract_without_hostcall(PLUGIN_PROFILE_PRODUCTION, "1.1.0");
        let removed_manifest = manifest_with_extension_refs(
            &[removed_contract.contract_ref.clone()],
            &test_ref("effect-extension"),
        );
        let removed = plugin_extension_compatibility_receipt_value(&PluginExtensionCompatibilityInput {
            old_manifest: &old_manifest,
            new_manifest: &removed_manifest,
            old_contracts: std::slice::from_ref(&old_contract),
            new_contracts: std::slice::from_ref(&removed_contract),
            migration_refs: &[],
            rollback_ref: &test_ref("rollback"),
            cleanup_refs: &[test_ref("cleanup")],
            production_profile: true,
        })
        .expect("removed hostcall receipt");
        let removed_parsed = parse_plugin_extension_compatibility_receipt(&removed).expect("parse removed hostcall");
        assert_eq!(removed_parsed.decision, PLUGIN_DECISION_DENY);
        assert!(removed_parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("removes required hostcall")));
    }

    #[test]
    fn lifecycle_requires_extension_negotiation_before_activation() {
        let contract = storage_extension_contract(PLUGIN_PROFILE_PRODUCTION, "1.0.0");
        let manifest_value = manifest_value_with_extension_refs(
            &[contract.contract_ref.clone()],
            &test_ref("effect-extension"),
        );
        let manifest = parse_plugin_manifest(&manifest_value).expect("manifest");
        let install = PluginInstallReceipt {
            receipt_ref: test_ref("install"),
            decision: PLUGIN_DECISION_PASS.to_string(),
            plugin_ref: manifest.plugin_ref.clone(),
            manifest_ref: manifest.manifest_ref.clone(),
            artifact_ref: manifest.artifact_ref.clone(),
            diagnostics: Vec::new(),
            value: record("test", Vec::new()),
        };
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
            evaluation_kind: PluginLifecycleEvaluationKind::ActivationRequest,
            manifest: &manifest,
            install: Some(&install),
            permission: None,
            activation: None,
            hostcall: None,
            health: None,
            removal: None,
            upgrade: None,
            negotiation: None,
            compatibility: None,
            recovery_receipt_ref: None,
        })
        .expect("evaluate activation without negotiation");
        assert_eq!(decision.decision, PLUGIN_DECISION_DENY);
        assert!(decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PLUGIN_LIFECYCLE_NEGOTIATION_MISSING));
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_plugin_artifacts() {
        let dir = temp_dir("plugin-catalog");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:catalog",
            artifact_ref: &test_ref("artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string()],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
            extension_contract_refs: &[],
        })
        .expect("manifest");
        let imported = crate::ledger::import_artifact(&ledger_root, &manifest).expect("ledger import");
        assert_eq!(imported.artifact_kind, "plugin-manifest");
        let listed = crate::catalog::list(&registry, Some(&ledger_root), &ListInput {
            kind: Some("plugin-manifest".to_string()),
            visibility: VisibilityInput::default(),
        })
        .expect("catalog list plugin manifest");
        assert_eq!(listed.items.len(), 1);
        let rendered = to_text(&listed.value).expect("render catalog result");
        assert!(rendered.contains("ledger-kind:plugin-manifest"));
        let request = crate::catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string(
            "plugin-manifest",
        )])])
        .expect("MCP request");
        let mcp = crate::catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("MCP list plugin manifest");
        assert_eq!(mcp.decision, PLUGIN_DECISION_PASS);
        assert!(to_text(&mcp.response_value).expect("render MCP response").contains("plugin-manifest"));
    }

    #[test]
    fn plugin_lifecycle_state_core_accepts_complete_ordered_trace() {
        let fixture = lifecycle_proof_fixture("plugin-lifecycle-complete");
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
            evaluation_kind: PluginLifecycleEvaluationKind::CompleteTrace,
            manifest: &fixture.manifest,
            install: Some(&fixture.install),
            permission: Some(&fixture.permission),
            activation: Some(&fixture.start),
            hostcall: Some(&fixture.hostcall),
            health: Some(&fixture.health),
            removal: Some(&fixture.removal),
            upgrade: Some(&fixture.upgrade),
            negotiation: None,
            compatibility: None,
            recovery_receipt_ref: None,
        })
        .expect("evaluate lifecycle state");
        assert_eq!(decision.decision, PLUGIN_DECISION_PASS);
        assert!(decision.side_effect_authorized);
        assert!(decision.authority_closed);
        assert!(decision.diagnostics.is_empty());
    }

    #[test]
    fn plugin_lifecycle_state_core_denies_hostcall_before_permission() {
        let fixture = lifecycle_proof_fixture("plugin-lifecycle-permission-deny");
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
            evaluation_kind: PluginLifecycleEvaluationKind::HostcallRequest,
            manifest: &fixture.manifest,
            install: Some(&fixture.install),
            permission: None,
            activation: Some(&fixture.start),
            hostcall: Some(&fixture.hostcall),
            health: Some(&fixture.health),
            removal: None,
            upgrade: None,
            negotiation: None,
            compatibility: None,
            recovery_receipt_ref: None,
        })
        .expect("evaluate lifecycle state");
        assert_eq!(decision.decision, PLUGIN_DECISION_DENY);
        assert!(!decision.side_effect_authorized);
        assert!(decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PLUGIN_LIFECYCLE_PERMISSION_MISSING));
    }

    #[test]
    fn plugin_lifecycle_state_core_denies_failed_health_upgrade() {
        let fixture = lifecycle_proof_fixture("plugin-lifecycle-health-deny");
        let failed_health_value = plugin_health_receipt_value(&HealthReceiptInput {
            manifest_value: &fixture.manifest_value,
            lifecycle_receipt_ref: &fixture.start.receipt_ref,
            service_refs: &[test_ref("service")],
            health_status: "failed",
            diagnostics: &["probe failed".to_string()],
        })
        .expect("failed health receipt");
        let failed_health = parse_plugin_health_receipt(&failed_health_value).expect("parse failed health");
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
            evaluation_kind: PluginLifecycleEvaluationKind::UpgradeRequest,
            manifest: &fixture.manifest,
            install: Some(&fixture.install),
            permission: Some(&fixture.permission),
            activation: Some(&fixture.start),
            hostcall: None,
            health: Some(&failed_health),
            removal: None,
            upgrade: Some(&fixture.upgrade),
            negotiation: None,
            compatibility: None,
            recovery_receipt_ref: None,
        })
        .expect("evaluate lifecycle state");
        assert_eq!(decision.decision, PLUGIN_DECISION_DENY);
        assert!(!decision.side_effect_authorized);
        assert!(decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PLUGIN_LIFECYCLE_HEALTH_FAILED));
    }

    #[test]
    fn plugin_lifecycle_state_core_denies_hostcall_after_removal() {
        let fixture = lifecycle_proof_fixture("plugin-lifecycle-removal-deny");
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
            evaluation_kind: PluginLifecycleEvaluationKind::HostcallRequest,
            manifest: &fixture.manifest,
            install: Some(&fixture.install),
            permission: Some(&fixture.permission),
            activation: Some(&fixture.start),
            hostcall: Some(&fixture.hostcall),
            health: Some(&fixture.health),
            removal: Some(&fixture.removal),
            upgrade: None,
            negotiation: None,
            compatibility: None,
            recovery_receipt_ref: None,
        })
        .expect("evaluate lifecycle state");
        assert_eq!(decision.decision, PLUGIN_DECISION_DENY);
        assert!(!decision.side_effect_authorized);
        assert!(decision.authority_closed);
        assert!(decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PLUGIN_LIFECYCLE_AUTHORITY_CLOSED));
    }

    #[test]
    fn plugin_lifecycle_state_core_denies_incomplete_cleanup() {
        let fixture = lifecycle_proof_fixture("plugin-lifecycle-cleanup-deny");
        let incomplete_removal_value = plugin_removal_receipt_value(&RemovalReceiptInput {
            manifest_value: &fixture.manifest_value,
            lifecycle_receipt_ref: &fixture.start.receipt_ref,
            owned_service_refs: &[test_ref("service")],
            assertion_refs: &[],
            handle_refs: &[],
            catalog_entry_refs: &[],
            diagnostics: &[],
        })
        .expect("incomplete removal receipt");
        let incomplete_removal = parse_plugin_removal_receipt(&incomplete_removal_value)
            .expect("parse incomplete removal receipt");
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
            evaluation_kind: PluginLifecycleEvaluationKind::RemovalRequest,
            manifest: &fixture.manifest,
            install: Some(&fixture.install),
            permission: Some(&fixture.permission),
            activation: Some(&fixture.start),
            hostcall: None,
            health: Some(&fixture.health),
            removal: Some(&incomplete_removal),
            upgrade: None,
            negotiation: None,
            compatibility: None,
            recovery_receipt_ref: None,
        })
        .expect("evaluate lifecycle state");
        assert_eq!(decision.decision, PLUGIN_DECISION_DENY);
        assert!(!decision.side_effect_authorized);
        assert!(decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PLUGIN_LIFECYCLE_REMOVAL_FAILED));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_plugin_lifecycle_refs_are_deterministic_and_authority_gated(tc: hegel::TestCase) {
        let callback_count = tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(4));
        let callback_count = usize::try_from(callback_count).expect("bounded callback count");
        let callbacks = ["init", "start", "health", "stop"]
            .iter()
            .take(callback_count)
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();
        let artifact_ref = test_ref("artifact-property");
        let value = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:property",
            artifact_ref: &artifact_ref,
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &callbacks,
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
            extension_contract_refs: &[],
        })
        .expect("manifest");
        let first_ref = canonical_hash(&value).expect("first ref");
        let rendered = to_text(&value).expect("render manifest");
        let reparsed = parse_text(&rendered).expect("parse rendered manifest");
        assert_eq!(first_ref, canonical_hash(&reparsed).expect("second ref"));
        let permission = plugin_permission_receipt_value(&PermissionReviewInput {
            manifest_value: &value,
            authority_refs: &[],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            effect_receipt_refs: &[test_ref("effect-receipt")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("permission receipt");
        assert_eq!(parse_plugin_permission_receipt(&permission).expect("parse permission").decision, PLUGIN_DECISION_DENY);
    }

    struct LifecycleProofFixture {
        manifest_value: IoValue,
        manifest: PluginManifest,
        install: PluginInstallReceipt,
        permission: PluginPermissionReceipt,
        start: PluginLifecycleReceipt,
        hostcall: PluginHostcallReceipt,
        health: PluginHealthReceipt,
        removal: PluginRemovalReceipt,
        upgrade: PluginUpgradeReceipt,
    }

    fn lifecycle_proof_fixture(label: &str) -> LifecycleProofFixture {
        let dir = temp_dir(label);
        let registry = dir.join("registry");
        let seed = seed_refs().expect("seed refs");
        let manifest_value = executor_manifest(&registry, &seed, label).expect("executor manifest");
        let manifest = parse_plugin_manifest(&manifest_value).expect("parse manifest");
        let install = install_plugin(&registry, &manifest_value).expect("install plugin");
        let permission = permission_step(&manifest_value, &seed).expect("permission step");
        let start = life_step("start", &manifest_value, &permission.receipt_ref, &seed).expect("start lifecycle");
        let hostcall = call_step(&manifest_value, &seed).expect("hostcall step");
        let service_ref = plugin_ref("service-supervision").expect("service ref");
        let health = health_step(&manifest_value, &start.receipt_ref, &service_ref).expect("health step");
        let removal = removal_step(&manifest_value, &start.receipt_ref, &service_ref).expect("removal step");
        let upgraded_manifest_value = executor_manifest(&registry, &seed, &format!("{label}-upgrade"))
            .expect("upgraded manifest");
        let upgrade = upgrade_step(&manifest_value, &upgraded_manifest_value, &removal.receipt_ref)
            .expect("upgrade step");
        LifecycleProofFixture {
            manifest_value,
            manifest,
            install,
            permission,
            start,
            hostcall,
            health,
            removal,
            upgrade,
        }
    }

    fn storage_extension_contract(profile: &str, version: &str) -> PluginExtensionContract {
        let authority_refs = vec![test_ref("storage-read-authority")];
        let resource_refs = vec![test_ref("storage-resource")];
        let effect_refs = vec![test_ref("effect-extension")];
        let error_refs = vec![test_ref("storage-error")];
        let descriptor_ref = test_ref("extension-storage-descriptor");
        let input_schema_ref = test_ref("storage-input-schema");
        let output_schema_ref = test_ref("storage-output-schema");
        let descriptor = PluginHostcallDescriptorInput {
            operation: "storage.read",
            descriptor_ref: &descriptor_ref,
            input_schema_ref: &input_schema_ref,
            output_schema_ref: &output_schema_ref,
            authority_refs: &authority_refs,
            resource_refs: &resource_refs,
            effect_manifest_refs: &effect_refs,
            replay_class: "idempotent",
            error_class_refs: &error_refs,
        };
        contract_from_descriptors(profile, version, &[descriptor])
    }

    fn storage_extension_contract_without_hostcall(profile: &str, version: &str) -> PluginExtensionContract {
        let authority_refs = vec![test_ref("other-authority")];
        let resource_refs = vec![test_ref("other-resource")];
        let effect_refs = vec![test_ref("effect-extension")];
        let error_refs = vec![test_ref("other-error")];
        let descriptor_ref = test_ref("other-descriptor");
        let input_schema_ref = test_ref("other-input-schema");
        let output_schema_ref = test_ref("other-output-schema");
        let descriptor = PluginHostcallDescriptorInput {
            operation: "storage.write",
            descriptor_ref: &descriptor_ref,
            input_schema_ref: &input_schema_ref,
            output_schema_ref: &output_schema_ref,
            authority_refs: &authority_refs,
            resource_refs: &resource_refs,
            effect_manifest_refs: &effect_refs,
            replay_class: "idempotent",
            error_class_refs: &error_refs,
        };
        contract_from_descriptors(profile, version, &[descriptor])
    }

    fn contract_from_descriptors(
        profile: &str,
        version: &str,
        descriptors: &[PluginHostcallDescriptorInput<'_>],
    ) -> PluginExtensionContract {
        let lifecycle = vec!["start".to_string(), "health".to_string()];
        let policy_refs = vec![test_ref("extension-policy")];
        let supply_refs = vec![test_ref("extension-supply")];
        let positive_suite_ref = test_ref("extension-positive-suite");
        let negative_suite_ref = test_ref("extension-negative-suite");
        let property_suite_ref = test_ref("extension-property-suite");
        let conformance = PluginExtensionConformanceInput {
            positive_suite_ref: &positive_suite_ref,
            negative_suite_ref: &negative_suite_ref,
            property_suite_ref: &property_suite_ref,
        };
        let value = plugin_extension_contract_value(&PluginExtensionContractInput {
            extension_id: "plugin-extension:storage",
            version,
            compatible_host_abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &lifecycle,
            hostcall_descriptors: descriptors,
            conformance,
            policy_refs: &policy_refs,
            supply_chain_refs: &supply_refs,
            production_profile: profile == PLUGIN_PROFILE_PRODUCTION,
        })
        .expect("extension contract value");
        parse_plugin_extension_contract(&value).expect("parse extension contract")
    }

    fn manifest_value_with_extension_refs(extension_contract_refs: &[String], effect_ref: &str) -> IoValue {
        plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:extension",
            artifact_ref: &test_ref("extension-artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string(), "health".to_string(), "remove".to_string()],
            effect_manifest_refs: &[effect_ref.to_string()],
            hostcall_refs: &[storage_read_hostcall_ref().expect("primitive hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
            extension_contract_refs,
        })
        .expect("manifest with extension refs")
    }

    fn manifest_with_extension_refs(extension_contract_refs: &[String], effect_ref: &str) -> PluginManifest {
        parse_plugin_manifest(&manifest_value_with_extension_refs(extension_contract_refs, effect_ref))
            .expect("parse extension manifest")
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
