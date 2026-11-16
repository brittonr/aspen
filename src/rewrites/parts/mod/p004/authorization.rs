    // r[verify molten.local_rewrite.upgrade_hook_authority]
    // r[verify molten.upgrades.authenticated_source_gate]
    #[test]
    fn apply_receipt_builds_upgrade_plan_from_authenticated_source_gate() {
        let (root, input) = rewrite_upgrade_fixture("rewrite-upgrade-hook");
        let source_gate = crate::node_daemon::AuthenticatedGate::fixture().expect("authenticated gate fixture");
        let refs = UpgradeRefs::new();
        let (applied, plan) = apply_with_upgrade_plan(&root, &input, &refs.input(&source_gate)).expect("upgrade rewrite");
        let parsed = crate::upgrades::parse_upgrade_plan(&plan).expect("parse upgrade plan");
        assert_eq!(parsed.tasks[0].kind, "install-artifact");
        assert!(parsed.checks.as_slice().iter().any(|check| check == "strict-octet-source-gate-bound"));
        assert_eq!(parsed.initiator_ref, refs.initiator);
        assert_eq!(parsed.capability_refs, refs.capabilities);
        assert_eq!(parsed.policy_refs, refs.policies);
        let bound = [source_gate.binding_ref(), source_gate.validation_ref()];
        let evidence = parsed.evidence_refs.as_slice();
        assert!(bound.iter().all(|reference| evidence.iter().any(|item| item.as_str() == *reference)));
        // Beyond the rewrite and binding refs, the plan binds its own strict source-gate validation ref.
        let inputs = sorted_unique_refs(&[
            applied.preview.plan_ref.clone(),
            canonical_hash(&applied.preview.receipt_value).expect("preview receipt ref"),
            canonical_hash(&applied.receipt_value).expect("apply receipt ref"),
            source_gate.binding_ref().to_string(),
            source_gate.validation_ref().to_string(),
        ]);
        assert_eq!(parsed.evidence_refs.len(), inputs.len() + 1);
    }

    // r[verify molten.upgrades.authenticated_source_gate]
    #[test]
    fn upgrade_plan_rejects_denied_or_fixture_source_gate_content() {
        let (root, input) = rewrite_upgrade_fixture("rewrite-upgrade-denied-gate");
        let authenticated = crate::node_daemon::AuthenticatedGate::fixture().expect("authenticated gate fixture");
        let refs = UpgradeRefs::new();
        let denied_value = parse_text(
            &to_text(authenticated.gate_value())
                .expect("gate text")
                .replacen("<decision \"pass\">", "<decision \"deny\">", 1),
        )
        .expect("denied gate");
        let fixture = crate::octet_gate::UnqualifiedFixtureGate::new("rewrite-test").expect("fixture gate");
        for gate_value in [denied_value, fixture.value().clone()] {
            let gate = authenticated.clone().with_gate_value_for_tests(gate_value).expect("replace gate");
            let error = apply_with_upgrade_plan(&root, &input, &refs.input(&gate)).expect_err("gate denied");
            assert!(error.to_string().contains("source gate validation failed"), "{error}");
            assert_no_upgrade_replacement(&root);
        }
    }

    // r[verify molten.upgrades.authenticated_source_gate]
    #[test]
    fn stale_authenticated_source_gate_is_denied_before_replacement_installation() {
        let (root, input) = rewrite_upgrade_fixture("rewrite-upgrade-stale-gate");
        let authenticated = crate::node_daemon::AuthenticatedGate::fixture().expect("authenticated gate fixture");
        let fields = authenticated.gate_value().collect_simple_record("octet-gate-receipt-v1", Some(15))
            .expect("gate receipt");
        let metadata = fields[11].collect_simple_record("metadata", Some(3)).expect("metadata");
        let config = metadata[0].collect_simple_record("config-hash", Some(1)).expect("config hash");
        let some = config[0].collect_simple_record("some", Some(1)).expect("present config hash");
        let config_hash = some[0].as_string().expect("config hash string");
        let stale_hash = format!("b3:{}", blake3::hash(b"different-workspace-config").to_hex());
        let stale_value = parse_text(
            &to_text(authenticated.gate_value()).expect("gate text")
                .replacen(&format!("\"{config_hash}\""), &format!("\"{stale_hash}\""), 1),
        ).expect("stale produced gate");
        crate::octet_gate::validate_produced_octet_gate_receipt(&stale_value)
            .expect("produced gate admission intentionally does not check workspace freshness");
        let stale = authenticated.with_gate_value_for_tests(stale_value).expect("stale authenticated fixture");
        let refs = UpgradeRefs::new();
        let error = apply_with_upgrade_plan(&root, &input, &refs.input(&stale)).expect_err("stale workspace denied");
        assert!(error.to_string().contains("config hash"), "{error}");
        assert_no_upgrade_replacement(&root);
    }

    // r[verify molten.local_rewrite.upgrade_hook_authority]
    #[test]
    fn upgrade_plan_requires_explicit_operator_refs() {
        let (root, rewrite_input) = rewrite_upgrade_fixture("rewrite-upgrade-refs");
        let source_gate = crate::node_daemon::AuthenticatedGate::fixture().expect("authenticated gate fixture");
        let refs = UpgradeRefs::new();
        let complete = refs.input(&source_gate);
        for input in [
            RewriteUpgradePlanInput { capability_refs: &[], ..complete },
            RewriteUpgradePlanInput { policy_refs: &[], ..complete },
            RewriteUpgradePlanInput { initiator_ref: "", ..complete },
            RewriteUpgradePlanInput { initiator_ref: "operator", ..complete },
        ] {
            apply_with_upgrade_plan(&root, &rewrite_input, &input).expect_err("missing or malformed authority refs denied");
            assert_no_upgrade_replacement(&root);
        }
    }

    fn assert_no_upgrade_replacement(root: &Path) {
        let replacements = find(root, &query(RewritePattern::StringEquals("new".to_string())))
            .expect("query replacement artifacts");
        assert!(replacements.matches.is_empty(), "refused upgrade must not install replacement artifacts");
    }

    struct UpgradeRefs {
        initiator: String,
        capabilities: Vec<String>,
        policies: Vec<String>,
    }

    impl UpgradeRefs {
        fn new() -> Self {
            Self {
                initiator: test_ref("initiator"),
                capabilities: vec![test_ref("upgrade-capability")],
                policies: vec![test_ref("upgrade-policy")],
            }
        }

        fn input<'a>(
            &'a self,
            source_gate: &'a crate::node_daemon::AuthenticatedGate,
        ) -> RewriteUpgradePlanInput<'a> {
            RewriteUpgradePlanInput {
                session_id: "rewrite-session",
                initiator_ref: &self.initiator,
                capability_refs: &self.capabilities,
                policy_refs: &self.policies,
                source_gate,
            }
        }
    }