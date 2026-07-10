use std::collections::BTreeSet;

const AST_GREP_PROFILE_ID: &str = "runtime-authority";
const AST_GREP_TOOL_PREFIX: &str = "ast-grep ";
const CONTENT_REF_PREFIX: &str = "blake3:";
const RECEIPT_DECISION_EVIDENCE_ONLY: &str = "evidence-only";
const RECEIPT_DECISION_INVALID: &str = "invalid";
const CLAIM_SCOPE_STRUCTURAL_HYGIENE: &str = "structural-hygiene-only";
const POSITIVE_FIXTURE_PATH: &str = "tools/ast-grep/runtime-authority/fixtures/positive/inventory_candidates.rs";
const NEGATIVE_FIXTURE_PATH: &str = "tools/ast-grep/runtime-authority/fixtures/negative/allowed_shell_effects.rs";

const REQUIRED_SURFACES: &[&str] = &[
    "core-runtime",
    "node-control",
    "effect-handlers",
    "plugin-host",
    "sealed-repro",
    "iroh-transport",
    "policy-evidence-gates",
    "operator-workflow",
];

const REQUIRED_INVENTORY_CATEGORIES: &[&str] = &[
    "ambient-filesystem",
    "ambient-process",
    "ambient-network",
    "ambient-clock",
    "ambient-random",
    "credential-access",
    "plugin-loading",
    "unsafe-hotspot",
    "panic-hotspot",
    "direct-authority-bypass",
];

const REQUIRED_NON_CLAIMS: &[&str] = &[
    "not-runtime-authority-admission",
    "not-replay-correctness-proof",
    "not-sealed-repro-correctness-proof",
    "not-ucan-authorization-proof",
    "not-distributed-safety-proof",
    "not-release-readiness-proof",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstGrepAuditProfile {
    pub id: String,
    pub surfaces: Vec<AuditSurface>,
    pub rules: Vec<AuditRule>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditSurface {
    pub id: String,
    pub scan_scopes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulePosture {
    Inventory,
    Warning,
    Blocking,
}

impl RulePosture {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inventory => "inventory",
            Self::Warning => "warning",
            Self::Blocking => "blocking",
        }
    }

    fn requires_fixtures(self) -> bool {
        matches!(self, Self::Warning | Self::Blocking)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRule {
    pub id: String,
    pub category: String,
    pub pattern_summary: String,
    pub posture: RulePosture,
    pub positive_fixture: Option<String>,
    pub negative_fixture: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditProfileValidation {
    pub valid: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstGrepFinding {
    pub rule_id: String,
    pub surface: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstGrepScanInput {
    pub profile: AstGrepAuditProfile,
    pub ast_grep_version: String,
    pub rule_bundle_hash: String,
    pub scan_scope_hash: String,
    pub evidence_gate_run_ref: String,
    pub findings: Vec<AstGrepFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstGrepAuditReceipt {
    pub profile_id: String,
    pub ast_grep_version: String,
    pub rule_bundle_hash: String,
    pub scan_scope_hash: String,
    pub evidence_gate_run_ref: String,
    pub decision: String,
    pub claim_scope: String,
    pub finding_count: usize,
    pub finding_rule_ids: Vec<String>,
    pub non_claims: Vec<String>,
    pub checks: Vec<ReceiptCheck>,
}

impl AstGrepAuditReceipt {
    pub fn valid(&self) -> bool {
        self.checks.iter().all(|check| check.passed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptCheck {
    pub name: String,
    pub passed: bool,
    pub diagnostic: String,
}

pub fn runtime_authority_profile() -> AstGrepAuditProfile {
    // r[impl aspen.ast_grep_runtime_authority_audits.profile]
    // r[impl aspen.ast_grep_runtime_authority_audits.inventory]
    AstGrepAuditProfile {
        id: AST_GREP_PROFILE_ID.to_string(),
        surfaces: required_surfaces(),
        rules: inventory_rules(),
        non_claims: required_non_claims(),
    }
}

pub fn validate_ast_grep_profile(profile: &AstGrepAuditProfile) -> AuditProfileValidation {
    // r[impl aspen.ast_grep_runtime_authority_audits.profile]
    // r[impl aspen.ast_grep_runtime_authority_audits.fixtures]
    let mut diagnostics = Vec::new();
    if profile.id != AST_GREP_PROFILE_ID {
        diagnostics.push(format!("ast-grep audit profile id must be {AST_GREP_PROFILE_ID}, got {}", profile.id));
    }

    require_items(
        "surface",
        REQUIRED_SURFACES,
        profile.surfaces.iter().map(|surface| surface.id.as_str()),
        &mut diagnostics,
    );
    require_items(
        "inventory category",
        REQUIRED_INVENTORY_CATEGORIES,
        profile.rules.iter().map(|rule| rule.category.as_str()),
        &mut diagnostics,
    );
    require_items("non-claim", REQUIRED_NON_CLAIMS, profile.non_claims.iter().map(String::as_str), &mut diagnostics);

    for rule in &profile.rules {
        if rule.posture.requires_fixtures() && (rule.positive_fixture.is_none() || rule.negative_fixture.is_none()) {
            diagnostics.push(format!(
                "rule {} cannot become {} without positive and negative fixtures",
                rule.id,
                rule.posture.as_str()
            ));
        }
    }

    AuditProfileValidation {
        valid: diagnostics.is_empty(),
        diagnostics,
    }
}

pub fn rule_bundle_hash(profile: &AstGrepAuditProfile) -> String {
    // r[impl aspen.ast_grep_runtime_authority_audits.identity]
    let mut lines = vec![format!("profile:{}", profile.id)];
    for surface in &profile.surfaces {
        lines.push(format!("surface:{}", surface.id));
        for scope in &surface.scan_scopes {
            lines.push(format!("scope:{}:{scope}", surface.id));
        }
    }
    for rule in &profile.rules {
        lines.push(format!(
            "rule:{}:{}:{}:{}:{}:{}",
            rule.id,
            rule.category,
            rule.pattern_summary,
            rule.posture.as_str(),
            rule.positive_fixture.as_deref().unwrap_or("none"),
            rule.negative_fixture.as_deref().unwrap_or("none")
        ));
    }
    for non_claim in &profile.non_claims {
        lines.push(format!("non-claim:{non_claim}"));
    }
    lines.sort();
    let digest = blake3::hash(lines.join("\n").as_bytes()).to_hex();
    format!("{CONTENT_REF_PREFIX}{digest}")
}

pub fn scan_scope_hash(surface_ids: &[String]) -> String {
    // r[impl aspen.ast_grep_runtime_authority_audits.identity]
    let mut sorted = surface_ids.to_vec();
    sorted.sort();
    let digest = blake3::hash(sorted.join("\n").as_bytes()).to_hex();
    format!("{CONTENT_REF_PREFIX}{digest}")
}

pub fn build_ast_grep_audit_receipt(input: AstGrepScanInput) -> AstGrepAuditReceipt {
    // r[impl aspen.ast_grep_runtime_authority_audits.identity]
    // r[impl aspen.ast_grep_runtime_authority_audits.evidence_gates]
    let profile_validation = validate_ast_grep_profile(&input.profile);
    let known_rule_ids = input.profile.rules.iter().map(|rule| rule.id.as_str()).collect::<BTreeSet<_>>();
    let finding_rule_ids = finding_rule_ids(&input.findings);
    let findings_known = input.findings.iter().all(|finding| known_rule_ids.contains(finding.rule_id.as_str()));
    let non_claims_bound = has_all_required_non_claims(&input.profile.non_claims);

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
        findings_known,
        "all structural findings must reference declared inventory rules".to_string(),
    ));
    checks.push(check(
        "findings-are-structural-candidates",
        true,
        "findings are candidate structural evidence only".to_string(),
    ));
    checks.push(check(
        "non-claims-bound",
        non_claims_bound,
        "receipt must bind authority, replay, sealed-repro, UCAN, distributed-safety, and release non-claims"
            .to_string(),
    ));

    let valid = checks.iter().all(|candidate| candidate.passed);
    AstGrepAuditReceipt {
        profile_id: input.profile.id,
        ast_grep_version: input.ast_grep_version,
        rule_bundle_hash: input.rule_bundle_hash,
        scan_scope_hash: input.scan_scope_hash,
        evidence_gate_run_ref: input.evidence_gate_run_ref,
        decision: if valid {
            RECEIPT_DECISION_EVIDENCE_ONLY.to_string()
        } else {
            RECEIPT_DECISION_INVALID.to_string()
        },
        claim_scope: CLAIM_SCOPE_STRUCTURAL_HYGIENE.to_string(),
        finding_count: input.findings.len(),
        finding_rule_ids,
        non_claims: input.profile.non_claims,
        checks,
    }
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

fn required_non_claims() -> Vec<String> {
    REQUIRED_NON_CLAIMS.iter().map(|non_claim| (*non_claim).to_string()).collect()
}

fn require_items<'a>(
    label: &str,
    required: &[&str],
    actual: impl Iterator<Item = &'a str>,
    diagnostics: &mut Vec<String>,
) {
    let actual = actual.collect::<BTreeSet<_>>();
    for required_item in required {
        if !actual.contains(required_item) {
            diagnostics.push(format!("missing ast-grep audit {label}: {required_item}"));
        }
    }
}

fn has_all_required_non_claims(non_claims: &[String]) -> bool {
    let non_claims = non_claims.iter().map(String::as_str).collect::<BTreeSet<_>>();
    REQUIRED_NON_CLAIMS.iter().all(|required| non_claims.contains(required))
}

fn finding_rule_ids(findings: &[AstGrepFinding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| finding.rule_id.clone())
        .collect::<BTreeSet<_>>()
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
        assert!(profile.rules.iter().all(|rule| rule.posture == RulePosture::Inventory));
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
        let profile = runtime_authority_profile();
        let surface_ids = profile.surfaces.iter().map(|surface| surface.id.clone()).collect::<Vec<_>>();
        let scope_hash = scan_scope_hash(&surface_ids);

        assert!(is_content_ref(&scope_hash));
        assert!(profile.rules.iter().all(|rule| rule.positive_fixture.as_deref() == Some(POSITIVE_FIXTURE_PATH)));
        assert!(profile.rules.iter().all(|rule| rule.negative_fixture.as_deref() == Some(NEGATIVE_FIXTURE_PATH)));
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
