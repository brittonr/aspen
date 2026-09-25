const AST_GREP_PROFILE_ID: &str = "runtime-authority";
const AST_GREP_TOOL_PREFIX: &str = "ast-grep ";
const CONTENT_REF_PREFIX: &str = "blake3:";
const RECEIPT_DECISION_EVIDENCE_ONLY: &str = "evidence-only";
const RECEIPT_DECISION_INVALID: &str = "invalid";
const CLAIM_SCOPE_STRUCTURAL_HYGIENE: &str = "structural-hygiene-only";
const POSITIVE_FIXTURE_PATH: &str = "tools/ast-grep/runtime-authority/fixtures/positive/inventory_candidates.rs";
const NEGATIVE_FIXTURE_PATH: &str = "tools/ast-grep/runtime-authority/fixtures/negative/allowed_shell_effects.rs";
const STORE_POSITIVE_FIXTURE_PATH: &str =
    "tools/ast-grep/runtime-authority/fixtures/positive/store_ambient_filesystem_calls.rs";
const STORE_NEGATIVE_FIXTURE_PATH: &str =
    "tools/ast-grep/runtime-authority/fixtures/negative/store_capability_shells.rs";
const TEST_WORKSPACE_POSITIVE_FIXTURE_PATH: &str =
    "tools/ast-grep/runtime-authority/fixtures/positive/test_ambient_temp_workspace.rs";
const TEST_WORKSPACE_NEGATIVE_FIXTURE_PATH: &str =
    "tools/ast-grep/runtime-authority/fixtures/negative/test_capability_workspace.rs";
const MATERIALIZATION_POSITIVE_FIXTURE_PATH: &str =
    "tools/ast-grep/runtime-authority/fixtures/positive/materialization_ambient_output.rs";
const MATERIALIZATION_NEGATIVE_FIXTURE_PATH: &str =
    "tools/ast-grep/runtime-authority/fixtures/negative/materialization_capability_shell.rs";

const REQUIRED_SURFACES: &[&str] = &[
    "core-runtime",
    "node-control",
    "effect-handlers",
    "plugin-host",
    "sealed-repro",
    "iroh-transport",
    "policy-evidence-gates",
    "operator-workflow",
    "local-store-adapters",
    "test-workspace-shells",
    "filesystem-materializers",
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

    diagnostics.extend(
        profile
            .rules
            .iter()
            .filter(|rule| {
                rule.posture.requires_fixtures() && (rule.positive_fixture.is_none() || rule.negative_fixture.is_none())
            })
            .map(|rule| {
                format!(
                    "rule {} cannot become {} without positive and negative fixtures",
                    rule.id,
                    rule.posture.as_str()
                )
            }),
    );

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
    let finding_rule_ids = finding_rule_ids(&input.findings);
    let checks = audit_checks(&input);
    let is_valid = checks.iter().all(|candidate| candidate.passed);
    AstGrepAuditReceipt {
        profile_id: input.profile.id,
        ast_grep_version: input.ast_grep_version,
        rule_bundle_hash: input.rule_bundle_hash,
        scan_scope_hash: input.scan_scope_hash,
        evidence_gate_run_ref: input.evidence_gate_run_ref,
        decision: if is_valid {
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
