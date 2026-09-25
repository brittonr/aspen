
/// The profile, tool, identity, finding, and non-claim checks of one structural audit scan, in
/// receipt order.
fn audit_checks(input: &AstGrepScanInput) -> Vec<ReceiptCheck> {
    let profile_validation = validate_ast_grep_profile(&input.profile);
    let known_rule_ids =
        input.profile.rules.iter().map(|rule| rule.id.as_str()).collect::<std::collections::BTreeSet<_>>();
    let is_findings_known = input.findings.iter().all(|finding| known_rule_ids.contains(finding.rule_id.as_str()));
    let blocking_rule_ids = input
        .profile
        .rules
        .iter()
        .filter(|rule| rule.posture == RulePosture::Blocking)
        .map(|rule| rule.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let is_blocking_findings_absent =
        input.findings.iter().all(|finding| !blocking_rule_ids.contains(finding.rule_id.as_str()));
    let is_non_claims_bound = has_all_required_non_claims(&input.profile.non_claims);

    let mut checks = Vec::new();
    checks.push(check(
        "ast-grep-profile-valid",
        profile_validation.valid,
        diagnostics_or("profile satisfies runtime-authority audit contract", &profile_validation.diagnostics),
    ));
    checks.push(check(
        "ast-grep-tool-bound",
        input.ast_grep_version.starts_with(AST_GREP_TOOL_PREFIX),
        format!("tool version must start with {AST_GREP_TOOL_PREFIX}"),
    ));
    checks.push(check(
        "rule-bundle-blake3-bound",
        is_content_ref(&input.rule_bundle_hash),
        "rule bundle identity must be a BLAKE3 content ref".to_string(),
    ));
    checks.push(check(
        "scan-scope-blake3-bound",
        is_content_ref(&input.scan_scope_hash),
        "scan scope identity must be a BLAKE3 content ref".to_string(),
    ));
    checks.push(check(
        "evidence-gate-run-bound",
        is_content_ref(&input.evidence_gate_run_ref),
        "runtime or evidence-gate run identity must be a content ref".to_string(),
    ));
    checks.push(check(
        "findings-reference-known-rules",
        is_findings_known,
        "all structural findings must reference declared inventory rules".to_string(),
    ));
    checks.push(check(
        "blocking-findings-absent",
        is_blocking_findings_absent,
        "blocking structural rules must have no findings in converted scopes".to_string(),
    ));
    checks.push(check(
        "findings-are-structural-candidates",
        true,
        "findings are candidate structural evidence only".to_string(),
    ));
    checks.push(check(
        "non-claims-bound",
        is_non_claims_bound,
        "receipt must bind authority, replay, sealed-repro, UCAN, distributed-safety, and release non-claims"
            .to_string(),
    ));
    checks
}

pub fn requires_fresh_scan(receipt: &AstGrepAuditReceipt, current_rule_bundle_hash: &str) -> bool {
    // r[impl aspen.ast_grep_runtime_authority_audits.identity]
    receipt.rule_bundle_hash != current_rule_bundle_hash
}

fn required_surfaces() -> Vec<AuditSurface> {
    vec![
        surface("core-runtime", &["src/runtime/**/*.rs", "src/node/runtime.rs"]),
        surface("node-control", &["src/node/**/*.rs", "src/cli/ops/node/**/*.rs"]),
        surface("effect-handlers", &["src/effects/**/*.rs", "src/resources/**/*.rs"]),
        surface("plugin-host", &["src/plugin/**/*.rs", "docs/plugin-extension-contracts/**/*.ncl"]),
        surface("sealed-repro", &["src/harness/**/*.rs", "src/cli/runtime/repro/**/*.rs"]),
        surface("iroh-transport", &["src/iroh/**/*.rs", "src/node/iroh.rs"]),
        surface("policy-evidence-gates", &["src/evidence/**/*.rs", "cairn-policy/**/*.ncl"]),
        surface("operator-workflow", &["src/operator/**/*.rs", "docs/production-*.ncl"]),
        surface("local-store-adapters", &[
            "src/artifacts/parts/mod/p*/body.rs",
            "src/chunk/parts/store/p*/body.rs",
            "src/retention/parts/mod/p*/body.rs",
            "src/remote/parts/dataspace/p*/body.rs",
            "src/iroh/parts/exchange/p*/body.rs",
        ]),
        surface("test-workspace-shells", &[
            "src/test/support.rs",
            "src/main/tests/ops/helpers.rs",
            "tests/parts/cliharness/p013/body.rs",
            "selected converted unit-test helper pages",
        ]),
        surface("filesystem-materializers", &[
            "src/cli/runtime/repro/bundle*.rs",
            "src/retention/parts/mod/p028/body.rs",
            "src/cli/ops/dogfood/{archive,io}.rs",
            "src/operator/parts/dogfood/p008/body.rs",
        ]),
    ]
}

fn inventory_rules() -> Vec<AuditRule> {
    vec![
        inventory_rule("ambient-filesystem-call", "ambient-filesystem", "std::fs::*"),
        inventory_rule("ambient-process-command", "ambient-process", "std::process::Command::new"),
        inventory_rule("ambient-network-bind", "ambient-network", "std::net::TcpListener::bind"),
        inventory_rule("ambient-clock-now", "ambient-clock", "std::time::SystemTime::now"),
        inventory_rule("ambient-random-thread-rng", "ambient-random", "rand::thread_rng"),
        inventory_rule("credential-env-var", "credential-access", "std::env::var"),
        inventory_rule("plugin-dynamic-load", "plugin-loading", "libloading::Library::new"),
        inventory_rule("unsafe-block", "unsafe-hotspot", "unsafe block"),
        inventory_rule("panic-bypass", "panic-hotspot", "panic!"),
        inventory_rule("direct-authority-bypass", "direct-authority-bypass", "AuthorityBypass::admit"),
        blocking_rule(
            "store-ambient-filesystem-call",
            "ambient-filesystem",
            "std::fs/fs child calls and ambient root reacquisition in converted stores",
            STORE_POSITIVE_FIXTURE_PATH,
            STORE_NEGATIVE_FIXTURE_PATH,
        ),
        blocking_rule(
            "test-ambient-temp-workspace",
            "ambient-filesystem",
            "predictable ambient temp roots and broad prefix cleanup in converted test helpers",
            TEST_WORKSPACE_POSITIVE_FIXTURE_PATH,
            TEST_WORKSPACE_NEGATIVE_FIXTURE_PATH,
        ),
        blocking_rule(
            "materialization-ambient-output",
            "ambient-filesystem",
            "ambient descendant I/O and generic archive unpack in converted materializers",
            MATERIALIZATION_POSITIVE_FIXTURE_PATH,
            MATERIALIZATION_NEGATIVE_FIXTURE_PATH,
        ),
    ]
}

fn surface(id: &str, scan_scopes: &[&str]) -> AuditSurface {
    AuditSurface {
        id: id.to_string(),
        scan_scopes: scan_scopes.iter().map(|scope| (*scope).to_string()).collect(),
    }
}

fn inventory_rule(id: &str, category: &str, pattern_summary: &str) -> AuditRule {
    AuditRule {
        id: id.to_string(),
        category: category.to_string(),
        pattern_summary: pattern_summary.to_string(),
        posture: RulePosture::Inventory,
        positive_fixture: Some(POSITIVE_FIXTURE_PATH.to_string()),
        negative_fixture: Some(NEGATIVE_FIXTURE_PATH.to_string()),
    }
}

fn blocking_rule(
    id: &str,
    category: &str,
    pattern_summary: &str,
    positive_fixture: &str,
    negative_fixture: &str,
) -> AuditRule {
    // r[impl molten.chunk_store.cap_std_regression_gate]
    // r[impl molten.testing.cap_std_regression_gate]
    // r[impl molten.filesystem_materialization.regression_gate]
    AuditRule {
        id: id.to_string(),
        category: category.to_string(),
        pattern_summary: pattern_summary.to_string(),
        posture: RulePosture::Blocking,
        positive_fixture: Some(positive_fixture.to_string()),
        negative_fixture: Some(negative_fixture.to_string()),
    }
}

fn required_non_claims() -> Vec<String> {
    REQUIRED_NON_CLAIMS.iter().map(|non_claim| (*non_claim).to_string()).collect()
}

fn require_items<'a>(
    label: &str,
    required: &[&str],
    actual: impl Iterator<Item = &'a str>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let actual = actual.collect::<std::collections::BTreeSet<_>>();
    for required_item in required {
        if !actual.contains(required_item) {
            diagnostics.push_item(format!("missing ast-grep audit {label}: {required_item}"));
        }
    }
}

fn has_all_required_non_claims(non_claims: &[String]) -> bool {
    let non_claims = non_claims.iter().map(String::as_str).collect::<std::collections::BTreeSet<_>>();
    REQUIRED_NON_CLAIMS.iter().all(|required| non_claims.contains(required))
}

fn finding_rule_ids(findings: &[AstGrepFinding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| finding.rule_id.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn diagnostics_or(success: &str, diagnostics: &[String]) -> String {
    if diagnostics.is_empty() {
        success.to_string()
    } else {
        diagnostics.join("; ")
    }
}

fn check(name: &str, passed: bool, diagnostic: String) -> ReceiptCheck {
    ReceiptCheck {
        name: name.to_string(),
        passed,
        diagnostic,
    }
}

fn is_content_ref(value: &str) -> bool {
    value.starts_with(CONTENT_REF_PREFIX) && value.len() > CONTENT_REF_PREFIX.len()
}
