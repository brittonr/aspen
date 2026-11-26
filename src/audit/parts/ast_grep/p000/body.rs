const AST_GREP_PROFILE_ID: &str = "runtime-authority";
const AST_GREP_TOOL_PREFIX: &str = "ast-grep ";
const CONTENT_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_DIGEST_LEN: usize = 64;
const FINDING_SHAPE_DIAGNOSTICS_MAX: usize = 3;
const RECEIPT_DECISION_EVIDENCE_ONLY: &str = "evidence-only";
const RECEIPT_DECISION_INVALID: &str = "invalid";
const CLAIM_SCOPE_STRUCTURAL_HYGIENE: &str = "structural-hygiene-only";
const RULE_DIRECTORY: &str = "tools/ast-grep/runtime-authority/rules";
const RULE_FILE_EXTENSION: &str = "yml";
const MAX_CONTENT_SOURCES: usize = 65_536;
const NODE_STATE_POSITIVE_FIXTURE_PATH: &str =
    "tools/ast-grep/runtime-authority/fixtures/positive/node_state_ambient_authority.rs";
const NODE_STATE_NEGATIVE_FIXTURE_PATH: &str =
    "tools/ast-grep/runtime-authority/fixtures/negative/node_state_capability_shell.rs";
const NODE_STATE_CARRIED_NEGATIVE_FIXTURE_PATH: &str =
    "tools/ast-grep/runtime-authority/fixtures/negative/node_state_carried_authority.rs";
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
    "node-state-adapters",
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
pub struct AuditProfile {
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
    /// Repository-relative YAML rule file whose bytes `rule_bundle_hash` binds.
    pub rule_path: String,
    pub positive_fixture: Option<String>,
    pub negative_fixture: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditProfileValidation {
    pub valid: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule_id: String,
    pub surface: String,
    pub path: String,
    pub message: String,
}

/// The bytes the audit shell actually read for one scan: every declared YAML rule file and every scanned source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanMeasurement<'a> {
    pub rule_sources: &'a [ContentSource<'a>],
    pub scanned_sources: &'a [ContentSource<'a>],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanInput {
    pub profile: AuditProfile,
    pub ast_grep_version: String,
    pub rule_bundle_hash: String,
    pub scan_scope_hash: String,
    pub evidence_gate_run_ref: String,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReceipt {
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

impl AuditReceipt {
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

pub fn profile() -> AuditProfile {
    // r[impl aspen.ast_grep_runtime_authority_audits.profile]
    // r[impl aspen.ast_grep_runtime_authority_audits.inventory]
    AuditProfile {
        id: AST_GREP_PROFILE_ID.to_string(),
        surfaces: required_surfaces(),
        rules: inventory_rules(),
        non_claims: required_non_claims(),
    }
}

pub fn validate_profile(profile: &AuditProfile) -> AuditProfileValidation {
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

/// Builds a receipt whose supplied rule-bundle and scan-scope identities are recomputed from `measurement`;
/// a supplied identity that differs from the measured bytes, or a finding outside the scanned inventory,
/// invalidates the receipt.
pub fn build_audit_receipt(input: ScanInput, measurement: &ScanMeasurement<'_>) -> AuditReceipt {
    // r[impl aspen.ast_grep_runtime_authority_audits.identity]
    // r[impl aspen.ast_grep_runtime_authority_audits.evidence_gates]
    let finding_rule_ids = finding_rule_ids(&input.findings);
    let checks = audit_checks(&input, measurement);
    let is_valid = checks.iter().all(|candidate| candidate.passed);
    AuditReceipt {
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
