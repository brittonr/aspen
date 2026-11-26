
pub fn requires_fresh_scan(receipt: &AuditReceipt, current_rule_bundle_hash: &str) -> bool {
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
            "src/chunk/store.rs",
            "src/chunk/store/**/*.rs",
            "src/retention/parts/mod/p*/body.rs",
            "src/remote/parts/dataspace/p*/body.rs",
            "src/iroh/parts/exchange/p*/body.rs",
        ]),
        surface("test-workspace-shells", &[
            "src/test/support.rs",
            "src/test/parts/support/p*/body.rs",
            "src/main/tests/ops/helpers.rs",
            "src/local_store.rs",
            "src/chunk/store/tests/mod.rs",
            "src/retention/parts/mod/tests/m000/p000/body.rs",
            "src/retention/parts/mod/tests/m000/p009/body.rs",
            "src/remote/parts/dataspace/tests/m000/p001/body.rs",
            "src/iroh/parts/exchange/tests/m000/p001/body.rs",
            "src/evidence/parts/chain/tests/m000/p003/body.rs",
            "src/node/daemon/tests/lifecycle/startup.rs",
            "src/node/daemon/tests/lifecycle/mailbox.rs",
            "src/node/daemon/tests/lifecycle/looping.rs",
            "src/node/daemon/tests/services/loopbacks.rs",
            "src/node/daemon/tests/services/supervisors.rs",
            "tests/parts/cliharness/p000/body.rs",
            "tests/parts/cliharness/p013/body.rs",
        ]),
        surface("node-state-adapters", &[
            "src/node/daemon.rs",
            "src/node/daemon/**/*.rs",
            "src/node/ingress/*.rs",
            "src/node/parts/identity/p001/body.rs",
            "src/node/parts/identity/p004/body.rs",
            "src/job/parts/dag/p009/body.rs",
            "src/job/parts/dag/p017/body.rs",
            "src/job/parts/dag/p030/body.rs",
            "src/job/parts/dag/p033/body.rs",
        ]),
        surface("filesystem-materializers", &[
            "src/cli/runtime/repro/bundle.rs",
            "src/cli/runtime/repro/bundle/unpack.rs",
            "src/retention/parts/mod/p028/body.rs",
            "src/retention/parts/mod/p041/body.rs",
            "src/cli/ops/dogfood/{archive,io}.rs",
            "src/operator/parts/dogfood/p008/body.rs",
            "src/operator/parts/dogfood/p021/body.rs",
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
        blocking_rule(
            "node-state-ambient-filesystem-call",
            "ambient-filesystem",
            "ambient descendant I/O in converted node-state daemon, identity, and target-job paths",
            NODE_STATE_POSITIVE_FIXTURE_PATH,
            NODE_STATE_NEGATIVE_FIXTURE_PATH,
        ),
        blocking_rule(
            "node-state-root-reacquisition",
            "direct-authority-bypass",
            "ambient DirectoryRoot/Scope reopening in inner node and target-job paths",
            NODE_STATE_POSITIVE_FIXTURE_PATH,
            NODE_STATE_CARRIED_NEGATIVE_FIXTURE_PATH,
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
        rule_path: rule_path(id),
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
    // r[impl molten.node.cap_std_regression_gate]
    AuditRule {
        id: id.to_string(),
        category: category.to_string(),
        pattern_summary: pattern_summary.to_string(),
        posture: RulePosture::Blocking,
        rule_path: rule_path(id),
        positive_fixture: Some(positive_fixture.to_string()),
        negative_fixture: Some(negative_fixture.to_string()),
    }
}

fn rule_path(id: &str) -> String {
    format!("{RULE_DIRECTORY}/{id}.{RULE_FILE_EXTENSION}")
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

fn finding_rule_ids(findings: &[Finding]) -> Vec<String> {
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

fn check(name: &str, diagnostics: Vec<String>, success: &str) -> ReceiptCheck {
    ReceiptCheck {
        name: name.to_string(),
        passed: diagnostics.is_empty(),
        diagnostic: diagnostics_or(success, &diagnostics),
    }
}

fn is_content_ref(value: &str) -> bool {
    value.strip_prefix(CONTENT_REF_PREFIX).is_some_and(|digest| {
        digest.len() == BLAKE3_HEX_DIGEST_LEN && digest.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    })
}
