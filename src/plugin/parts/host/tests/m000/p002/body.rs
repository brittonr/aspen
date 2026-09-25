
    /// A grant whose attenuation names no budget refs cannot be encoded.
    fn assert_empty_budget_grant_rejected(
        manifest_value: &IoValue,
        contract: &PluginExtensionContract,
        descriptor: &PluginHostcallDescriptor,
    ) {
        let empty_budget_refs = Vec::new();
        let empty_budget_attenuation = PluginCapabilityGrantAttenuationInput {
            delegated_scope: &descriptor.resource_refs[0],
            current_delegation_depth: PLUGIN_INITIAL_TURN,
            max_delegation_depth: TEST_GRANT_MAX_DELEGATION_DEPTH,
            budget_refs: &empty_budget_refs,
            valid_from_turn: PLUGIN_INITIAL_TURN,
            valid_until_turn: TEST_GRANT_VALID_UNTIL_TURN,
        };
        let manifest = parse_plugin_manifest(manifest_value).expect("parse manifest for budget negative");
        let empty_budget_value = plugin_capability_grant_value(&PluginCapabilityGrantInput {
            plugin_ref: &manifest.plugin_ref,
            plugin_id: &manifest.plugin_id,
            manifest_ref: &manifest.manifest_ref,
            extension_contract_ref: Some(&contract.contract_ref),
            hostcall_descriptor_ref: &descriptor.descriptor_ref,
            operation: &descriptor.operation,
            input_schema_ref: &descriptor.input_schema_ref,
            output_schema_ref: &descriptor.output_schema_ref,
            resource_refs: &descriptor.resource_refs,
            resource_scope: &descriptor.resource_refs[0],
            effect_manifest_refs: &descriptor.effect_manifest_refs,
            effect_receipt_refs: &[test_ref("effect-receipt")],
            policy_refs: &manifest.policy_refs,
            issuer_ref: &test_ref("grant-issuer"),
            proof_refs: &[test_ref("grant-proof")],
            attenuation: empty_budget_attenuation,
            revocation_refs: &[],
            revoked: false,
            replay_class: &descriptor.replay_class,
        });
        assert!(empty_budget_value.is_err());
    }

    #[test]
    fn extension_negotiation_denies_missing_required_and_allows_optional_omission() {
        let contract = storage_extension_contract(PLUGIN_PROFILE_PRODUCTION, "1.0.0");
        let manifest = manifest_with_extension_refs(std::slice::from_ref(&contract.contract_ref), &test_ref("effect-extension"));
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
        let manifest = manifest_with_extension_refs(std::slice::from_ref(&contract.contract_ref), &test_ref("effect-extension"));
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
        let old_manifest = manifest_with_extension_refs(std::slice::from_ref(&old_contract.contract_ref), &test_ref("effect-extension"));
        let new_manifest = manifest_with_extension_refs(std::slice::from_ref(&new_contract.contract_ref), &test_ref("effect-extension"));
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
            std::slice::from_ref(&downgrade_contract.contract_ref),
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
            std::slice::from_ref(&removed_contract.contract_ref),
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
            std::slice::from_ref(&contract.contract_ref),
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
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput { hostcall: Some(&fixture.hostcall), removal: Some(&fixture.removal), upgrade: Some(&fixture.upgrade), ..fixture_lifecycle_input(&fixture, PluginLifecycleEvaluationKind::CompleteTrace) })
        .expect("evaluate lifecycle state");
        assert_eq!(decision.decision, PLUGIN_DECISION_PASS);
        assert!(decision.side_effect_authorized);
        assert!(decision.authority_closed);
        assert_eq!(decision.prior_state, PluginLifecycleState::ManifestDeclared);
        assert_eq!(decision.event, PluginLifecycleEvent::CompleteTrace);
        assert_eq!(decision.next_state, PluginLifecycleState::Upgraded);
        assert_eq!(decision.side_effect_class, "trace-replay");
        assert!(decision.guard_refs.contains(&fixture.install.receipt_ref));
        assert!(to_text(&decision.value)
            .expect("render FSM decision")
            .contains("reviewed-transition-table"));
        assert!(decision.diagnostics.is_empty());
    }

    #[test]
    fn plugin_lifecycle_state_core_denies_hostcall_before_permission() {
        let fixture = lifecycle_proof_fixture("plugin-lifecycle-permission-deny");
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput { permission: None, hostcall: Some(&fixture.hostcall), ..fixture_lifecycle_input(&fixture, PluginLifecycleEvaluationKind::HostcallRequest) })
        .expect("evaluate lifecycle state");
        assert_eq!(decision.decision, PLUGIN_DECISION_DENY);
        assert!(!decision.side_effect_authorized);
        assert_eq!(decision.event, PluginLifecycleEvent::Hostcall);
        assert_eq!(decision.next_state, decision.prior_state);
        assert_eq!(decision.side_effect_class, PLUGIN_LIFECYCLE_SIDE_EFFECT_NONE);
        assert!(decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PLUGIN_LIFECYCLE_PERMISSION_MISSING));
    }
