
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
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput { health: Some(&failed_health), upgrade: Some(&fixture.upgrade), ..fixture_lifecycle_input(&fixture, PluginLifecycleEvaluationKind::UpgradeRequest) })
        .expect("evaluate lifecycle state");
        assert_eq!(decision.decision, PLUGIN_DECISION_DENY);
        assert!(!decision.side_effect_authorized);
        assert_eq!(decision.prior_state, PluginLifecycleState::Degraded);
        assert_eq!(decision.event, PluginLifecycleEvent::Upgrade);
        assert!(decision
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PLUGIN_LIFECYCLE_HEALTH_FAILED));
    }

    #[test]
    fn plugin_lifecycle_state_core_denies_hostcall_after_removal() {
        let fixture = lifecycle_proof_fixture("plugin-lifecycle-removal-deny");
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput { hostcall: Some(&fixture.hostcall), removal: Some(&fixture.removal), ..fixture_lifecycle_input(&fixture, PluginLifecycleEvaluationKind::HostcallRequest) })
        .expect("evaluate lifecycle state");
        assert_eq!(decision.decision, PLUGIN_DECISION_DENY);
        assert!(!decision.side_effect_authorized);
        assert!(decision.authority_closed);
        assert_eq!(decision.prior_state, PluginLifecycleState::Removed);
        assert_eq!(decision.event, PluginLifecycleEvent::Hostcall);
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
        let decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput { removal: Some(&incomplete_removal), ..fixture_lifecycle_input(&fixture, PluginLifecycleEvaluationKind::RemovalRequest) })
        .expect("evaluate lifecycle state");
        assert_eq!(decision.decision, PLUGIN_DECISION_DENY);
        assert!(!decision.side_effect_authorized);
        assert_eq!(decision.event, PluginLifecycleEvent::Remove);
        assert_eq!(decision.next_state, decision.prior_state);
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

    /// Lifecycle input with the fixture's install, permission, start, and health receipts and no later receipts.
    fn fixture_lifecycle_input(
        fixture: &LifecycleProofFixture,
        evaluation_kind: PluginLifecycleEvaluationKind,
    ) -> PluginLifecycleStateInput<'_> {
        PluginLifecycleStateInput {
            evaluation_kind,
            manifest: &fixture.manifest,
            install: Some(&fixture.install),
            permission: Some(&fixture.permission),
            activation: Some(&fixture.start),
            hostcall: None,
            health: Some(&fixture.health),
            removal: None,
            upgrade: None,
            negotiation: None,
            compatibility: None,
            recovery_receipt_ref: None,
        }
    }

    fn hostcall_receipt_with_grant(
        manifest_value: &IoValue,
        contract: &PluginExtensionContract,
        descriptor: &PluginHostcallDescriptor,
        grant: &PluginCapabilityGrant,
        evaluation_turn: u64,
    ) -> PluginHostcallReceipt {
        let value = plugin_hostcall_receipt_value(&HostcallReceiptInput {
            manifest_value,
            operation: &descriptor.operation,
            hostcall_ref: &descriptor.descriptor_ref,
            executor_receipt_ref: &test_ref("executor"),
            effect_receipt_ref: &test_ref("effect-receipt"),
            authority_refs: &descriptor.authority_refs,
            capability_grants: std::slice::from_ref(grant),
            resource_refs: &descriptor.resource_refs,
            extension_contracts: std::slice::from_ref(contract),
            input_schema_ref: Some(&descriptor.input_schema_ref),
            output_schema_ref: Some(&descriptor.output_schema_ref),
            evaluation_turn,
        })
        .expect("hostcall receipt with grant");
        parse_plugin_hostcall_receipt(&value).expect("parse hostcall receipt with grant")
    }

    fn matching_capability_grant_fixture(
        manifest_value: &IoValue,
        contract: &PluginExtensionContract,
        descriptor: &PluginHostcallDescriptor,
        revoked: bool,
    ) -> PluginCapabilityGrant {
        capability_grant_fixture(
            manifest_value,
            contract,
            descriptor,
            &descriptor.operation,
            &descriptor.resource_refs,
            revoked,
            TEST_GRANT_VALID_UNTIL_TURN,
            PLUGIN_INITIAL_TURN,
            TEST_GRANT_MAX_DELEGATION_DEPTH,
        )
    }

    fn capability_grant_fixture(
        manifest_value: &IoValue,
        contract: &PluginExtensionContract,
        descriptor: &PluginHostcallDescriptor,
        operation: &str,
        resource_refs: &[String],
        revoked: bool,
        valid_until_turn: u64,
        current_delegation_depth: u64,
        max_delegation_depth: u64,
    ) -> PluginCapabilityGrant {
        let manifest = parse_plugin_manifest(manifest_value).expect("parse grant manifest");
        let effect_receipt_refs = vec![test_ref("effect-receipt")];
        let issuer_ref = test_ref("grant-issuer");
        let proof_refs = vec![test_ref("grant-proof")];
        let budget_refs = vec![test_ref("grant-budget")];
        let revocation_refs = if revoked {
            vec![test_ref("grant-revocation")]
        } else {
            Vec::new()
        };
        let resource_scope = resource_refs.first().expect("grant resource scope");
        let attenuation = PluginCapabilityGrantAttenuationInput {
            delegated_scope: resource_scope,
            current_delegation_depth,
            max_delegation_depth,
            budget_refs: &budget_refs,
            valid_from_turn: PLUGIN_INITIAL_TURN,
            valid_until_turn,
        };
        let value = plugin_capability_grant_value(&PluginCapabilityGrantInput {
            plugin_ref: &manifest.plugin_ref,
            plugin_id: &manifest.plugin_id,
            manifest_ref: &manifest.manifest_ref,
            extension_contract_ref: Some(&contract.contract_ref),
            hostcall_descriptor_ref: &descriptor.descriptor_ref,
            operation,
            input_schema_ref: &descriptor.input_schema_ref,
            output_schema_ref: &descriptor.output_schema_ref,
            resource_refs,
            resource_scope,
            effect_manifest_refs: &descriptor.effect_manifest_refs,
            effect_receipt_refs: &effect_receipt_refs,
            policy_refs: &manifest.policy_refs,
            issuer_ref: &issuer_ref,
            proof_refs: &proof_refs,
            attenuation,
            revocation_refs: &revocation_refs,
            revoked,
            replay_class: &descriptor.replay_class,
        })
        .expect("capability grant value");
        parse_plugin_capability_grant(&value).expect("parse capability grant")
    }
