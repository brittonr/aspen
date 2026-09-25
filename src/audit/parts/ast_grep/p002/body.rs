
#[cfg(test)]
mod tests {
    use super::*;

    const AST_GREP_VERSION_FIXTURE: &str = "ast-grep 0.39.5";
    const RUN_REF_FIXTURE: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
    const SCOPE_REF_FIXTURE: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
    const STALE_RULE_BUNDLE_REF: &str = "blake3:3333333333333333333333333333333333333333333333333333333333333333";

    #[test]
    fn profile_declares_required_structural_surfaces_and_inventory_rules() {
        // r[verify aspen.ast_grep_runtime_authority_audits.profile]
        // r[verify aspen.ast_grep_runtime_authority_audits.inventory]
        let profile = runtime_authority_profile();
        let validation = validate_ast_grep_profile(&profile);

        assert!(validation.valid, "{:?}", validation.diagnostics);
        assert!(
            profile
                .rules
                .iter()
                .any(|rule| { rule.id == "store-ambient-filesystem-call" && rule.posture == RulePosture::Blocking })
        );
        assert!(profile.rules.iter().any(|rule| rule.posture == RulePosture::Inventory));
        assert!(has_all_required_non_claims(&profile.non_claims));
    }

    #[test]
    fn warning_or_blocking_rule_requires_positive_and_negative_fixtures() {
        // r[verify aspen.ast_grep_runtime_authority_audits.fixtures]
        let mut profile = runtime_authority_profile();
        profile.rules.push(AuditRule {
            id: "unproven-blocking-rule".to_string(),
            category: "ambient-filesystem".to_string(),
            pattern_summary: "std::fs::remove_file".to_string(),
            posture: RulePosture::Blocking,
            positive_fixture: Some(POSITIVE_FIXTURE_PATH.to_string()),
            negative_fixture: None,
        });

        let validation = validate_ast_grep_profile(&profile);

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
        let profile = runtime_authority_profile();
        let surface_ids = profile.surfaces.iter().map(|surface| surface.id.clone()).collect::<Vec<_>>();
        let scope_hash = scan_scope_hash(&surface_ids);

        assert!(is_content_ref(&scope_hash));
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
    }

    #[test]
    fn receipt_binds_tool_identity_scope_findings_and_non_claims() {
        // r[verify aspen.ast_grep_runtime_authority_audits.identity]
        // r[verify aspen.ast_grep_runtime_authority_audits.evidence_gates]
        let profile = runtime_authority_profile();
        let rule_bundle_hash = rule_bundle_hash(&profile);
        let receipt = build_ast_grep_audit_receipt(AstGrepScanInput {
            profile,
            ast_grep_version: AST_GREP_VERSION_FIXTURE.to_string(),
            rule_bundle_hash,
            scan_scope_hash: SCOPE_REF_FIXTURE.to_string(),
            evidence_gate_run_ref: RUN_REF_FIXTURE.to_string(),
            findings: vec![AstGrepFinding {
                rule_id: "ambient-filesystem-call".to_string(),
                surface: "policy-evidence-gates".to_string(),
                path: "src/cli/evidence/gate/io.rs".to_string(),
                message: "direct filesystem call is shell-owned inventory evidence".to_string(),
            }],
        });

        assert!(receipt.valid(), "{:?}", receipt.checks);
        assert_eq!(receipt.decision, RECEIPT_DECISION_EVIDENCE_ONLY);
        assert_eq!(receipt.claim_scope, CLAIM_SCOPE_STRUCTURAL_HYGIENE);
        assert_eq!(receipt.finding_count, 1);
        assert_eq!(receipt.finding_rule_ids, vec!["ambient-filesystem-call"]);
        assert!(receipt.non_claims.iter().any(|claim| claim == "not-runtime-authority-admission"));
        assert!(receipt.non_claims.iter().any(|claim| claim == "not-release-readiness-proof"));
    }

    #[test]
    fn blocking_store_finding_invalidates_receipt_without_overclaiming_runtime_safety() {
        // r[verify molten.chunk_store.cap_std_regression_gate]
        let profile = runtime_authority_profile();
        let rule_bundle_hash = rule_bundle_hash(&profile);
        let receipt = build_ast_grep_audit_receipt(AstGrepScanInput {
            profile,
            ast_grep_version: AST_GREP_VERSION_FIXTURE.to_string(),
            rule_bundle_hash,
            scan_scope_hash: SCOPE_REF_FIXTURE.to_string(),
            evidence_gate_run_ref: RUN_REF_FIXTURE.to_string(),
            findings: vec![AstGrepFinding {
                rule_id: "store-ambient-filesystem-call".to_string(),
                surface: "local-store-adapters".to_string(),
                path: "src/chunk/parts/store/p001/body.rs".to_string(),
                message: "ambient child read in converted store".to_string(),
            }],
        });

        assert!(!receipt.valid());
        assert_eq!(receipt.decision, RECEIPT_DECISION_INVALID);
        assert_eq!(receipt.claim_scope, CLAIM_SCOPE_STRUCTURAL_HYGIENE);
        assert!(receipt.checks.iter().any(|check| check.name == "blocking-findings-absent" && !check.passed));
    }

    #[test]
    fn blocking_materialization_finding_invalidates_structural_receipt() {
        // r[verify molten.filesystem_materialization.regression_gate]
        let profile = runtime_authority_profile();
        let rule_bundle_hash = rule_bundle_hash(&profile);
        let receipt = build_ast_grep_audit_receipt(AstGrepScanInput {
            profile,
            ast_grep_version: AST_GREP_VERSION_FIXTURE.to_string(),
            rule_bundle_hash,
            scan_scope_hash: SCOPE_REF_FIXTURE.to_string(),
            evidence_gate_run_ref: RUN_REF_FIXTURE.to_string(),
            findings: vec![AstGrepFinding {
                rule_id: "materialization-ambient-output".to_string(),
                surface: "filesystem-materializers".to_string(),
                path: "src/cli/runtime/repro/bundle.rs".to_string(),
                message: "ambient descendant write in converted materializer".to_string(),
            }],
        });

        assert!(!receipt.valid());
        assert_eq!(receipt.decision, RECEIPT_DECISION_INVALID);
        assert_eq!(receipt.claim_scope, CLAIM_SCOPE_STRUCTURAL_HYGIENE);
        assert!(receipt.checks.iter().any(|check| check.name == "blocking-findings-absent" && !check.passed));
    }

    #[test]
    fn changed_rule_bundle_requires_fresh_scan_receipt() {
        // r[verify aspen.ast_grep_runtime_authority_audits.identity]
        let profile = runtime_authority_profile();
        let current_rule_bundle_hash = rule_bundle_hash(&profile);
        let receipt = build_ast_grep_audit_receipt(AstGrepScanInput {
            profile,
            ast_grep_version: AST_GREP_VERSION_FIXTURE.to_string(),
            rule_bundle_hash: STALE_RULE_BUNDLE_REF.to_string(),
            scan_scope_hash: SCOPE_REF_FIXTURE.to_string(),
            evidence_gate_run_ref: RUN_REF_FIXTURE.to_string(),
            findings: Vec::new(),
        });

        assert!(receipt.valid());
        assert!(requires_fresh_scan(&receipt, &current_rule_bundle_hash));
    }

    #[test]
    fn receipt_rejects_unknown_finding_rule_without_overclaiming_authority() {
        // r[verify aspen.ast_grep_runtime_authority_audits.fixtures]
        // r[verify aspen.ast_grep_runtime_authority_audits.evidence_gates]
        let profile = runtime_authority_profile();
        let rule_bundle_hash = rule_bundle_hash(&profile);
        let receipt = build_ast_grep_audit_receipt(AstGrepScanInput {
            profile,
            ast_grep_version: AST_GREP_VERSION_FIXTURE.to_string(),
            rule_bundle_hash,
            scan_scope_hash: SCOPE_REF_FIXTURE.to_string(),
            evidence_gate_run_ref: RUN_REF_FIXTURE.to_string(),
            findings: vec![AstGrepFinding {
                rule_id: "undeclared-rule".to_string(),
                surface: "plugin-host".to_string(),
                path: "src/plugin/host.rs".to_string(),
                message: "unknown structural finding".to_string(),
            }],
        });

        assert!(!receipt.valid());
        assert_eq!(receipt.decision, RECEIPT_DECISION_INVALID);
        assert_eq!(receipt.claim_scope, CLAIM_SCOPE_STRUCTURAL_HYGIENE);
        assert!(receipt.checks.iter().any(|check| check.name == "findings-reference-known-rules" && !check.passed));
    }
}
