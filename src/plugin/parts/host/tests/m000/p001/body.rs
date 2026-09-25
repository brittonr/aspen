
    #[test]
    fn stale_manifest_receipts_deny_lifecycle_use() {
        let fixture = lifecycle_proof_fixture("plugin-stale-manifest");
        let stale_manifest_ref = test_ref("stale-manifest");
        let stale_hostcall = PluginHostcallReceipt {
            manifest_ref: stale_manifest_ref.clone(),
            ..fixture.hostcall.clone()
        };
        let hostcall_decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput { hostcall: Some(&stale_hostcall), ..fixture_lifecycle_input(&fixture, PluginLifecycleEvaluationKind::HostcallRequest) })
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
        let health_decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput { health: Some(&stale_health), upgrade: Some(&fixture.upgrade), ..fixture_lifecycle_input(&fixture, PluginLifecycleEvaluationKind::UpgradeRequest) })
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
        let removal_decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput { removal: Some(&stale_removal), ..fixture_lifecycle_input(&fixture, PluginLifecycleEvaluationKind::RemovalRequest) })
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
        let manifest = manifest_with_extension_refs(std::slice::from_ref(&contract.contract_ref), &test_ref("effect-extension"));
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

        let valid_grant_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/plugin-extension-contracts/storage.grant.preserves"
        ));
        let valid_grant_value = parse_text(valid_grant_source).expect("parse checked-in grant export");
        let valid_grant = parse_plugin_capability_grant(&valid_grant_value).expect("validate checked-in grant export");
        assert_eq!(valid_grant.plugin_id, "plugin:storage");

        let invalid_grant_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/docs/plugin-extension-contracts/storage-missing-proof.grant.preserves"
        ));
        let invalid_grant_value = parse_text(invalid_grant_source).expect("parse invalid checked-in grant export");
        assert!(parse_plugin_capability_grant(&invalid_grant_value).is_err());
    }

    #[test]
    fn contract_aware_hostcall_requires_descriptor_specific_evidence() {
        let contract = storage_extension_contract(PLUGIN_PROFILE_PRODUCTION, "1.0.0");
        let descriptor = &contract.hostcall_descriptors[0];
        let manifest_value = manifest_value_with_extension_refs(
            std::slice::from_ref(&contract.contract_ref),
            &descriptor.effect_manifest_refs[0],
        );
        let generic_deny = plugin_hostcall_receipt_value(&HostcallReceiptInput {
            manifest_value: &manifest_value,
            operation: &descriptor.operation,
            hostcall_ref: &descriptor.descriptor_ref,
            executor_receipt_ref: &test_ref("executor"),
            effect_receipt_ref: &test_ref("effect-receipt"),
            authority_refs: &[test_ref("unrelated-authority")],
            capability_grants: &[],
            resource_refs: &[test_ref("unrelated-resource")],
            extension_contracts: std::slice::from_ref(&contract),
            input_schema_ref: Some(&descriptor.input_schema_ref),
            output_schema_ref: Some(&descriptor.output_schema_ref),
            evaluation_turn: PLUGIN_INITIAL_TURN,
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
            capability_grants: &[matching_capability_grant_fixture(&manifest_value, &contract, descriptor, false)],
            resource_refs: &descriptor.resource_refs,
            extension_contracts: std::slice::from_ref(&contract),
            input_schema_ref: Some(&descriptor.input_schema_ref),
            output_schema_ref: Some(&descriptor.output_schema_ref),
            evaluation_turn: PLUGIN_INITIAL_TURN,
        })
        .expect("descriptor-specific hostcall pass");
        assert_eq!(parse_plugin_hostcall_receipt(&pass).expect("parse pass").decision, PLUGIN_DECISION_PASS);
    }

    #[test]
    fn capability_grants_parse_classify_and_deny_mismatches() {
        let contract = storage_extension_contract(PLUGIN_PROFILE_PRODUCTION, "1.0.0");
        let descriptor = &contract.hostcall_descriptors[0];
        let manifest_value = manifest_value_with_extension_refs(
            std::slice::from_ref(&contract.contract_ref),
            &descriptor.effect_manifest_refs[0],
        );
        let matching_grant = matching_capability_grant_fixture(&manifest_value, &contract, descriptor, false);
        assert_eq!(matching_grant.typed_ref.as_str(), matching_grant.grant_ref);
        assert_eq!(
            classify_plugin_reference_value(&matching_grant.value),
            PluginReferenceRole::CapabilityGrant
        );
        assert_eq!(crate::ledger::artifact_kind(&matching_grant.value), "plugin-capability-grant");
        assert!(plugin_summary(&matching_grant.value)
            .expect("grant summary")
            .contains("plugin capability grant"));
        let assert_denied = |grant: &PluginCapabilityGrant, evaluation_turn: u64, expected_diagnostic: &str| {
            let receipt = hostcall_receipt_with_grant(&manifest_value, &contract, descriptor, grant, evaluation_turn);
            assert_eq!(receipt.decision, PLUGIN_DECISION_DENY);
            assert!(
                receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains(expected_diagnostic)),
                "{expected_diagnostic}"
            );
        };
        let grant_with = |operation: &str, resource_refs: &[String], valid_until_turn: u64, delegation_depth: u64| {
            capability_grant_fixture(
                &manifest_value,
                &contract,
                descriptor,
                operation,
                resource_refs,
                false,
                valid_until_turn,
                delegation_depth,
                TEST_GRANT_MAX_DELEGATION_DEPTH,
            )
        };

        let mut wrong_manifest_grant = matching_grant.clone();
        wrong_manifest_grant.manifest_ref = test_ref("wrong-manifest");
        assert_denied(&wrong_manifest_grant, PLUGIN_INITIAL_TURN, "wrong-manifest");
        let mut wrong_descriptor_grant = matching_grant.clone();
        wrong_descriptor_grant.hostcall_descriptor_ref = test_ref("wrong-descriptor");
        assert_denied(&wrong_descriptor_grant, PLUGIN_INITIAL_TURN, "wrong-descriptor");
        let mut wrong_schema_grant = matching_grant.clone();
        wrong_schema_grant.input_schema_ref = test_ref("wrong-schema");
        assert_denied(&wrong_schema_grant, PLUGIN_INITIAL_TURN, "wrong-schema");

        assert_empty_budget_grant_rejected(&manifest_value, &contract, descriptor);

        let wrong_operation_grant =
            grant_with("storage.write", &descriptor.resource_refs, TEST_GRANT_VALID_UNTIL_TURN, PLUGIN_INITIAL_TURN);
        assert_denied(&wrong_operation_grant, PLUGIN_INITIAL_TURN, "wrong-operation");
        let wrong_resource_refs = vec![test_ref("wrong-resource")];
        let wrong_resource_grant =
            grant_with(&descriptor.operation, &wrong_resource_refs, TEST_GRANT_VALID_UNTIL_TURN, PLUGIN_INITIAL_TURN);
        assert_denied(&wrong_resource_grant, PLUGIN_INITIAL_TURN, "wrong-resource");
        let over_delegated_grant = grant_with(
            &descriptor.operation,
            &descriptor.resource_refs,
            TEST_GRANT_VALID_UNTIL_TURN,
            TEST_GRANT_MAX_DELEGATION_DEPTH + 1,
        );
        assert_denied(&over_delegated_grant, PLUGIN_INITIAL_TURN, "over-delegated");
        let revoked_grant = matching_capability_grant_fixture(&manifest_value, &contract, descriptor, true);
        assert_denied(&revoked_grant, PLUGIN_INITIAL_TURN, "revoked");
        let expired_grant =
            grant_with(&descriptor.operation, &descriptor.resource_refs, PLUGIN_INITIAL_TURN, PLUGIN_INITIAL_TURN);
        assert_denied(&expired_grant, TEST_GRANT_EXPIRED_TURN, "expired");
    }
