    // Scan-scope, fixture, and receipt-check tests; helpers live in `src/audit/parts/ast_grep/tests/p000/body.rs`.
    #[test]
    fn scan_scope_hash_binds_exact_source_inventory_and_bytes() {
        // r[verify aspen.ast_grep_runtime_authority_audits.validation]
        // r[verify molten.evidence_gates.static_audit_content_bound_hashes]
        let daemon = ContentSource {
            path: "src/node/parts/daemon/p020/body.rs",
            bytes: b"fn carried(root: &DirectoryRoot) {}\n",
        };
        let ingress = ContentSource {
            path: "src/node/ingress/mod.rs",
            bytes: b"fn ingress() {}\n",
        };
        let baseline = scan_scope_hash(&[daemon, ingress]).expect("scan scope hash");
        assert!(is_content_ref(&baseline));
        assert_eq!(scan_scope_hash(&[ingress, daemon]).expect("reordered scan scope"), baseline);

        let edited = ContentSource {
            bytes: b"fn carried(root: &DirectoryRoot) { DirectoryRoot::open(path); }\n",
            ..daemon
        };
        assert_ne!(scan_scope_hash(&[edited, ingress]).expect("edited scan scope"), baseline);
        let renamed = ContentSource {
            path: "src/node/parts/daemon/p021/body.rs",
            ..daemon
        };
        assert_ne!(scan_scope_hash(&[renamed, ingress]).expect("renamed scan scope"), baseline);
        assert_ne!(scan_scope_hash(&[daemon]).expect("narrowed scan scope"), baseline);

        assert!(scan_scope_hash(&[]).is_err());
        assert!(scan_scope_hash(&[daemon, daemon]).is_err());
        for path in ["/abs/src/lib.rs", "src/../lib.rs", "src//lib.rs", "./src/lib.rs", ""] {
            let source = ContentSource { path, bytes: b"" };
            assert!(scan_scope_hash(&[source]).is_err(), "{path:?}");
        }
    }

    #[test]
    fn warning_or_blocking_rule_requires_positive_and_negative_fixtures() {
        // r[verify aspen.ast_grep_runtime_authority_audits.fixtures]
        let mut profile = profile();
        profile.rules.push(AuditRule {
            id: "unproven-blocking-rule".to_string(),
            category: "ambient-filesystem".to_string(),
            pattern_summary: "std::fs::remove_file".to_string(),
            posture: RulePosture::Blocking,
            rule_path: rule_path("unproven-blocking-rule"),
            positive_fixture: Some(POSITIVE_FIXTURE_PATH.to_string()),
            negative_fixture: None,
        });

        let validation = validate_profile(&profile);

        assert!(!validation.valid);
        assert!(
            validation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("positive and negative fixtures"))
        );
    }

    #[test]
    fn validation_evidence_names_rule_fixtures_and_scan_scope() {
        // r[verify aspen.ast_grep_runtime_authority_audits.validation]
        // r[verify molten.testing.cap_std_regression_gate]
        // r[verify molten.filesystem_materialization.regression_gate]
        // r[verify molten.node.cap_std_regression_gate]
        let profile = profile();

        assert!(profile.rules.iter().filter(|rule| rule.posture == RulePosture::Inventory).all(|rule| {
            rule.positive_fixture.as_deref() == Some(POSITIVE_FIXTURE_PATH)
                && rule.negative_fixture.as_deref() == Some(NEGATIVE_FIXTURE_PATH)
        }));
        let store_rule = profile
            .rules
            .iter()
            .find(|rule| rule.id == "store-ambient-filesystem-call")
            .expect("store blocking rule");
        assert_eq!(store_rule.positive_fixture.as_deref(), Some(STORE_POSITIVE_FIXTURE_PATH));
        assert_eq!(store_rule.negative_fixture.as_deref(), Some(STORE_NEGATIVE_FIXTURE_PATH));
        let workspace_rule = profile
            .rules
            .iter()
            .find(|rule| rule.id == "test-ambient-temp-workspace")
            .expect("test workspace blocking rule");
        assert_eq!(workspace_rule.positive_fixture.as_deref(), Some(TEST_WORKSPACE_POSITIVE_FIXTURE_PATH));
        assert_eq!(workspace_rule.negative_fixture.as_deref(), Some(TEST_WORKSPACE_NEGATIVE_FIXTURE_PATH));
        let materialization_rule = profile
            .rules
            .iter()
            .find(|rule| rule.id == "materialization-ambient-output")
            .expect("materialization blocking rule");
        assert_eq!(materialization_rule.positive_fixture.as_deref(), Some(MATERIALIZATION_POSITIVE_FIXTURE_PATH));
        assert_eq!(materialization_rule.negative_fixture.as_deref(), Some(MATERIALIZATION_NEGATIVE_FIXTURE_PATH));
        let node_state_rule = profile
            .rules
            .iter()
            .find(|rule| rule.id == "node-state-root-reacquisition")
            .expect("node-state root reacquisition rule");
        assert_eq!(node_state_rule.positive_fixture.as_deref(), Some(NODE_STATE_POSITIVE_FIXTURE_PATH));
        assert_eq!(node_state_rule.negative_fixture.as_deref(), Some(NODE_STATE_CARRIED_NEGATIVE_FIXTURE_PATH));
    }

    #[test]
    fn receipt_binds_tool_identity_scope_findings_and_non_claims() {
        // r[verify aspen.ast_grep_runtime_authority_audits.identity]
        // r[verify aspen.ast_grep_runtime_authority_audits.evidence_gates]
        let profile = profile();
        let rule_bundle_hash = default_rule_bundle_hash(&profile);
        let receipt = build_audit_receipt(scan_input(profile, rule_bundle_hash, vec![finding(
            "ambient-filesystem-call",
            "policy-evidence-gates",
            "src/cli/evidence/gate/io.rs",
        )]));

        assert!(receipt.valid(), "{:?}", receipt.checks);
        assert_eq!(receipt.decision, RECEIPT_DECISION_EVIDENCE_ONLY);
        assert_eq!(receipt.claim_scope, CLAIM_SCOPE_STRUCTURAL_HYGIENE);
        assert_eq!(receipt.finding_count, 1);
        assert_eq!(receipt.finding_rule_ids, vec!["ambient-filesystem-call"]);
        assert!(receipt.non_claims.iter().any(|claim| claim == "not-runtime-authority-admission"));
        assert!(receipt.non_claims.iter().any(|claim| claim == "not-release-readiness-proof"));
    }

    #[test]
    fn receipt_check_results_are_derived_from_their_diagnostics() {
        // r[verify molten.evidence_gates.static_audit_content_bound_hashes]
        // r[verify aspen.ast_grep_runtime_authority_audits.evidence_gates]
        let profile = profile();
        let rule_bundle_hash = default_rule_bundle_hash(&profile);
        let receipt = build_audit_receipt(scan_input(profile.clone(), rule_bundle_hash.clone(), vec![finding(
            "ambient-filesystem-call",
            "invented-surface",
            "/etc/passwd",
        )]));

        assert!(!receipt.valid());
        assert_eq!(receipt.decision, RECEIPT_DECISION_INVALID);
        let candidates = failed_check(&receipt, "findings-are-structural-candidates");
        assert!(candidates.diagnostic.contains("undeclared surface invented-surface"), "{}", candidates.diagnostic);
        assert!(candidates.diagnostic.contains("not repository-relative"), "{}", candidates.diagnostic);

        let mut stripped = profile;
        stripped.non_claims.retain(|claim| claim != "not-release-readiness-proof");
        let mut input = scan_input(stripped, rule_bundle_hash, Vec::new());
        input.scan_scope_hash = "blake3:not-a-digest".to_string();
        let receipt = build_audit_receipt(input);
        let non_claims = failed_check(&receipt, "non-claims-bound");
        assert!(non_claims.diagnostic.contains("not-release-readiness-proof"), "{}", non_claims.diagnostic);
        assert!(failed_check(&receipt, "scan-scope-blake3-bound").diagnostic.contains("blake3:not-a-digest"));
        let failed = receipt.checks.iter().filter(|check| !check.passed).map(|check| check.name.as_str()).collect::<Vec<_>>();
        // Stripping a non-claim changes the profile metadata, so the supplied rule-bundle identity no longer
        // matches the measured bundle either.
        assert_eq!(failed, vec![
            "ast-grep-profile-valid",
            "rule-bundle-blake3-bound",
            "scan-scope-blake3-bound",
            "non-claims-bound",
        ]);
    }

    #[test]
    fn blocking_store_finding_invalidates_receipt_without_overclaiming_safety() {
        // r[verify molten.chunk_store.cap_std_regression_gate]
        let profile = profile();
        let rule_bundle_hash = default_rule_bundle_hash(&profile);
        let receipt = build_audit_receipt(scan_input(profile, rule_bundle_hash, vec![finding(
            "store-ambient-filesystem-call",
            "local-store-adapters",
            "src/chunk/parts/store/p001/body.rs",
        )]));

        assert!(!receipt.valid());
        assert_eq!(receipt.decision, RECEIPT_DECISION_INVALID);
        assert_eq!(receipt.claim_scope, CLAIM_SCOPE_STRUCTURAL_HYGIENE);
        assert!(receipt.checks.iter().any(|check| check.name == "blocking-findings-absent" && !check.passed));
    }

    #[test]
    fn blocking_materialization_finding_invalidates_structural_receipt() {
        // r[verify molten.filesystem_materialization.regression_gate]
        let profile = profile();
        let rule_bundle_hash = default_rule_bundle_hash(&profile);
        let receipt = build_audit_receipt(scan_input(profile, rule_bundle_hash, vec![finding(
            "materialization-ambient-output",
            "filesystem-materializers",
            "src/cli/runtime/repro/bundle.rs",
        )]));

        assert!(!receipt.valid());
        assert_eq!(receipt.decision, RECEIPT_DECISION_INVALID);
        assert_eq!(receipt.claim_scope, CLAIM_SCOPE_STRUCTURAL_HYGIENE);
        assert!(receipt.checks.iter().any(|check| check.name == "blocking-findings-absent" && !check.passed));
    }

    #[test]
    fn changed_rule_bundle_requires_fresh_scan_receipt() {
        // r[verify aspen.ast_grep_runtime_authority_audits.identity]
        let profile = profile();
        let measured = default_rule_bundle_hash(&profile);
        let receipt = build_audit_receipt(scan_input(profile, measured.clone(), Vec::new()));

        assert!(receipt.valid(), "{:?}", receipt.checks);
        assert!(!requires_fresh_scan(&receipt, &measured));
        assert!(requires_fresh_scan(&receipt, STALE_RULE_BUNDLE_REF));
    }

    #[test]
    fn receipt_rejects_identities_that_do_not_match_the_measured_bytes() {
        // r[verify molten.evidence_gates.static_audit_content_bound_hashes]
        // r[verify aspen.ast_grep_runtime_authority_audits.identity]
        let profile = profile();
        let measured = default_rule_bundle_hash(&profile);
        let stale = build_audit_receipt(scan_input(profile.clone(), STALE_RULE_BUNDLE_REF.to_string(), Vec::new()));
        assert!(!stale.valid());
        let rule = failed_check(&stale, "rule-bundle-blake3-bound");
        assert!(rule.diagnostic.contains(&format!("does not match measured {measured}")), "{}", rule.diagnostic);

        let tampered = [ContentSource {
            bytes: b"fn gate_io() { std::fs::read(\"x\"); }\n",
            ..SCANNED_SOURCES[0]
        }];
        let mut input = scan_input(profile.clone(), measured.clone(), Vec::new());
        input.scan_scope_hash = scan_scope_hash(&tampered).expect("tampered scope");
        let receipt = build_audit_receipt(input);
        assert!(!receipt.valid());
        assert!(failed_check(&receipt, "scan-scope-blake3-bound").diagnostic.contains("does not match measured"));

        let rule_sources = rule_sources(DEFAULT_PROFILE_RULE_FILES);
        let unmeasured = super::build_audit_receipt(scan_input(profile, measured, Vec::new()), &ScanMeasurement {
            rule_sources: &rule_sources[1..],
            scanned_sources: SCANNED_SOURCES,
        });
        assert!(failed_check(&unmeasured, "rule-bundle-blake3-bound").diagnostic.contains("cannot be measured"));
    }

    #[test]
    fn findings_outside_the_measured_scan_invalidate_the_receipt() {
        // r[verify molten.evidence_gates.static_audit_content_bound_hashes]
        let profile = profile();
        let rule_bundle_hash = default_rule_bundle_hash(&profile);
        let receipt = build_audit_receipt(scan_input(profile, rule_bundle_hash, vec![finding(
            "ambient-filesystem-call",
            "policy-evidence-gates",
            "src/cli/evidence/gate/unscanned.rs",
        )]));
        assert!(!receipt.valid());
        let candidates = failed_check(&receipt, "findings-are-structural-candidates");
        assert!(candidates.diagnostic.contains("outside the measured scan"), "{}", candidates.diagnostic);
    }

    #[test]
    fn receipt_rejects_unknown_finding_rule_without_overclaiming() {
        // r[verify aspen.ast_grep_runtime_authority_audits.fixtures]
        // r[verify aspen.ast_grep_runtime_authority_audits.evidence_gates]
        let profile = profile();
        let rule_bundle_hash = default_rule_bundle_hash(&profile);
        let receipt = build_audit_receipt(scan_input(profile, rule_bundle_hash, vec![finding(
            "undeclared-rule",
            "plugin-host",
            "src/plugin/host.rs",
        )]));

        assert!(!receipt.valid());
        assert_eq!(receipt.decision, RECEIPT_DECISION_INVALID);
        assert_eq!(receipt.claim_scope, CLAIM_SCOPE_STRUCTURAL_HYGIENE);
        assert!(receipt.checks.iter().any(|check| check.name == "findings-reference-known-rules" && !check.passed));
    }
