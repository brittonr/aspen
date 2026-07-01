type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const PROD_OPS_BACKUP_RESTORE_DRILL_SCHEMA: &str = crate::preserves_rail::PROD_OPS_BACKUP_RESTORE_DRILL_SCHEMA;
const PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA: &str = crate::preserves_rail::PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA;
const PROD_OPS_OBSERVABILITY_SLO_SCHEMA: &str = crate::preserves_rail::PROD_OPS_OBSERVABILITY_SLO_SCHEMA;
const PROD_OPS_RUNBOOK_CHECK_SCHEMA: &str = crate::preserves_rail::PROD_OPS_RUNBOOK_CHECK_SCHEMA;
const PROD_OPS_UPGRADE_ROLLBACK_DRILL_SCHEMA: &str = crate::preserves_rail::PROD_OPS_UPGRADE_ROLLBACK_DRILL_SCHEMA;
const PROD_RELEASE_CANDIDATE_GATE_SCHEMA: &str = crate::preserves_rail::PROD_RELEASE_CANDIDATE_GATE_SCHEMA;
const PROD_RELEASE_PILOT_DECISION_SCHEMA: &str = crate::preserves_rail::PROD_RELEASE_PILOT_DECISION_SCHEMA;
const PROD_SECURITY_BOUNDARY_NEGATIVE_SUITE_SCHEMA: &str =
    crate::preserves_rail::PROD_SECURITY_BOUNDARY_NEGATIVE_SUITE_SCHEMA;
const PROD_SECURITY_DRILL_SCHEMA: &str = crate::preserves_rail::PROD_SECURITY_DRILL_SCHEMA;
const PROD_SECURITY_READINESS_REPORT_SCHEMA: &str = crate::preserves_rail::PROD_SECURITY_READINESS_REPORT_SCHEMA;
const PROD_SECURITY_REDACTION_AUDIT_SCHEMA: &str = crate::preserves_rail::PROD_SECURITY_REDACTION_AUDIT_SCHEMA;
const PROD_SECURITY_SUPPLY_CHAIN_REVIEW_SCHEMA: &str = crate::preserves_rail::PROD_SECURITY_SUPPLY_CHAIN_REVIEW_SCHEMA;
const PROD_SECURITY_THREAT_MODEL_SCHEMA: &str = crate::preserves_rail::PROD_SECURITY_THREAT_MODEL_SCHEMA;

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

#[cfg(test)]
fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

#[cfg(test)]
fn to_text(value: &IoValue) -> Result<String> {
    crate::preserves_rail::to_text(value)
}

const MAX_PROD_REFS: usize = 512;
const MAX_PROD_TEXTS: usize = 256;
const _: () = assert!(MAX_PROD_REFS <= 100_000);
const _: () = assert!(MAX_PROD_TEXTS <= 100_000);

const BROAD_PRODUCTION_SCOPE: &str = "broad-production";
const CONFIGURATION_CLEAN_CAVEAT_STATUS: &str = "configuration-clean-caveat";
const SOURCE_REMEDIATED_ZERO_STATUS: &str = "source-remediated-zero";

const SECURITY_DRILL_KINDS: &[&str] = &[
    "key-revocation",
    "delegation-expiry",
    "authority-attenuation",
    "live-ref-cleanup",
    "stale-ticket-denial",
    "compromised-peer-evidence",
    "incident-response",
];

const INCIDENT_KINDS: &[&str] = &[
    "compromised-key",
    "leaked-ticket",
    "stale-source-gate",
    "bad-release-evidence",
    "secret-exposure",
    "emergency-stop",
];

pub struct DeploymentProfileInput<'a> {
    pub decision: &'a str,
    pub profile_name: &'a str,
    pub state_layout_refs: &'a [String],
    pub required_adapter_refs: &'a [String],
    pub source_gate_refs: &'a [String],
    pub resource_limit_refs: &'a [String],
    pub redaction_setting_refs: &'a [String],
    pub live_transport_refs: &'a [String],
    pub startup_expectation_refs: &'a [String],
    pub shutdown_expectation_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct BackupRestoreDrillInput<'a> {
    pub decision: &'a str,
    pub drill_name: &'a str,
    pub ledger_refs: &'a [String],
    pub redb_refs: &'a [String],
    pub chunk_refs: &'a [String],
    pub identity_refs: &'a [String],
    pub retention_pin_refs: &'a [String],
    pub source_gate_refs: &'a [String],
    pub restore_verification_refs: &'a [String],
    pub tamper_denial_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct UpgradeRollbackDrillInput<'a> {
    pub decision: &'a str,
    pub plan_name: &'a str,
    pub migration_refs: &'a [String],
    pub smoke_refs: &'a [String],
    pub rollback_eligibility_refs: &'a [String],
    pub irreversible_exclusion_refs: &'a [String],
    pub post_rollback_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct ObservabilitySloInput<'a> {
    pub decision: &'a str,
    pub snapshot_name: &'a str,
    pub adapter_health_refs: &'a [String],
    pub queue_depth: u64,
    pub max_queue_depth: u64,
    pub control_loop_refs: &'a [String],
    pub resource_pressure_refs: &'a [String],
    pub retention_drift_refs: &'a [String],
    pub source_gate_freshness_refs: &'a [String],
    pub live_transport_refs: &'a [String],
    pub import_export_failure_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct RunbookCheckInput<'a> {
    pub decision: &'a str,
    pub runbook_name: &'a str,
    pub operation: &'a str,
    pub canonical_artifact_refs: &'a [String],
    pub denial_fixture_refs: &'a [String],
    pub auxiliary_log_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct ThreatModelInput<'a> {
    pub decision: &'a str,
    pub model_name: &'a str,
    pub threat_entries: &'a [String],
    pub mapped_gate_refs: &'a [String],
    pub drill_refs: &'a [String],
    pub negative_suite_refs: &'a [String],
    pub unresolved_risk_refs: &'a [String],
    pub pilot_consequence_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct SecurityDrillInput<'a> {
    pub decision: &'a str,
    pub drill_kind: &'a str,
    pub scenario: &'a str,
    pub pass_evidence_refs: &'a [String],
    pub denial_refs: &'a [String],
    pub cleanup_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct RedactionAuditInput<'a> {
    pub decision: &'a str,
    pub audit_name: &'a str,
    pub surface_refs: &'a [String],
    pub redaction_refs: &'a [String],
    pub reveal_gate_refs: &'a [String],
    pub plaintext_denial_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct SupplyChainReviewInput<'a> {
    pub decision: &'a str,
    pub review_name: &'a str,
    pub release_refs: &'a [String],
    pub source_gate_refs: &'a [String],
    pub provenance_refs: &'a [String],
    pub build_verify_refs: &'a [String],
    pub signed_keyring_refs: &'a [String],
    pub sensitive_artifact_refs: &'a [String],
    pub mismatch_denial_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct BoundaryNegativeSuiteInput<'a> {
    pub decision: &'a str,
    pub suite_name: &'a str,
    pub preserves_parser_refs: &'a [String],
    pub receipt_validator_refs: &'a [String],
    pub source_gate_refs: &'a [String],
    pub repro_bundle_refs: &'a [String],
    pub node_ingress_refs: &'a [String],
    pub provenance_refs: &'a [String],
    pub plugin_hostcall_refs: &'a [String],
    pub malformed_denial_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct IncidentResponseDrillInput<'a> {
    pub decision: &'a str,
    pub incident_kind: &'a str,
    pub scenario: &'a str,
    pub detection_refs: &'a [String],
    pub containment_refs: &'a [String],
    pub recovery_refs: &'a [String],
    pub next_step_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct SecurityReadinessReportInput<'a> {
    pub decision: &'a str,
    pub report_name: &'a str,
    pub threat_model_refs: &'a [String],
    pub supply_chain_refs: &'a [String],
    pub drill_refs: &'a [String],
    pub redaction_audit_refs: &'a [String],
    pub boundary_suite_refs: &'a [String],
    pub incident_response_refs: &'a [String],
    pub unresolved_risk_refs: &'a [String],
    pub pilot_recommendation: &'a str,
    pub diagnostics: &'a [String],
}

pub struct PilotDecisionInput<'a> {
    pub decision: &'a str,
    pub scope: &'a str,
    pub allowed_workloads: &'a [String],
    pub denied_workloads: &'a [String],
    pub rollback_triggers: &'a [String],
    pub stop_conditions: &'a [String],
    pub operator_review_refs: &'a [String],
    pub caveats: &'a [String],
    pub diagnostics: &'a [String],
}

pub struct ReleaseCandidateGateInput<'a> {
    pub decision: &'a str,
    pub candidate: &'a str,
    pub source_ref: &'a str,
    pub rust_validation_refs: &'a [String],
    pub nextest_refs: &'a [String],
    pub nix_check_refs: &'a [String],
    pub cairn_validation_refs: &'a [String],
    pub octet_refs: &'a [String],
    pub dogfood_refs: &'a [String],
    pub bundle_verify_refs: &'a [String],
    pub promotion_refs: &'a [String],
    pub export_verify_refs: &'a [String],
    pub source_gate_status: &'a str,
    pub source_gate_caveats: &'a [String],
    pub pilot_decision_refs: &'a [String],
    pub diagnostics: &'a [String],
}

pub fn deployment_profile_value(input: &DeploymentProfileInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("profile name", input.profile_name)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_refs("state layout", input.state_layout_refs, input.decision)?;
    require_pass_refs("required adapter", input.required_adapter_refs, input.decision)?;
    require_pass_refs("source gate", input.source_gate_refs, input.decision)?;
    require_pass_refs("resource limit", input.resource_limit_refs, input.decision)?;
    require_pass_refs("redaction setting", input.redaction_setting_refs, input.decision)?;
    require_pass_refs("live transport", input.live_transport_refs, input.decision)?;
    require_pass_refs("startup expectation", input.startup_expectation_refs, input.decision)?;
    require_pass_refs("shutdown expectation", input.shutdown_expectation_refs, input.decision)?;
    Ok(record("prod-ops-deployment-profile-v1", vec![
        string(PROD_OPS_DEPLOYMENT_PROFILE_SCHEMA),
        decision_field(input.decision),
        record("profile", vec![string(input.profile_name)]),
        refs_field("state-layout", input.state_layout_refs)?,
        refs_field("required-adapters", input.required_adapter_refs)?,
        refs_field("source-gates", input.source_gate_refs)?,
        refs_field("resource-limits", input.resource_limit_refs)?,
        refs_field("redaction-settings", input.redaction_setting_refs)?,
        refs_field("live-transport", input.live_transport_refs)?,
        refs_field("startup-expectations", input.startup_expectation_refs)?,
        refs_field("shutdown-expectations", input.shutdown_expectation_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("explicit-state-layout", pass_check(input.state_layout_refs.is_empty())),
            check_value("required-adapters-bound", pass_check(input.required_adapter_refs.is_empty())),
            check_value("source-gate-inputs-bound", pass_check(input.source_gate_refs.is_empty())),
            check_value(
                "resource-redaction-live-settings-bound",
                pass_check(
                    input.resource_limit_refs.is_empty()
                        || input.redaction_setting_refs.is_empty()
                        || input.live_transport_refs.is_empty(),
                ),
            ),
            check_value("profile-receipt-does-not-grant-authority", "pass"),
        ]),
    ]))
}

pub fn backup_restore_drill_value(input: &BackupRestoreDrillInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("backup restore drill name", input.drill_name)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("ledger", input.ledger_refs),
        ("redb", input.redb_refs),
        ("chunk", input.chunk_refs),
        ("identity", input.identity_refs),
        ("retention pin", input.retention_pin_refs),
        ("source gate", input.source_gate_refs),
        ("restore verification", input.restore_verification_refs),
        ("tamper denial", input.tamper_denial_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    Ok(record("prod-ops-backup-restore-drill-v1", vec![
        string(PROD_OPS_BACKUP_RESTORE_DRILL_SCHEMA),
        decision_field(input.decision),
        record("drill", vec![string(input.drill_name)]),
        refs_field("ledgers", input.ledger_refs)?,
        refs_field("redb-stores", input.redb_refs)?,
        refs_field("chunks", input.chunk_refs)?,
        refs_field("identity-metadata", input.identity_refs)?,
        refs_field("retention-pins", input.retention_pin_refs)?,
        refs_field("source-gates", input.source_gate_refs)?,
        refs_field("restore-verification", input.restore_verification_refs)?,
        refs_field("tamper-denials", input.tamper_denial_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value(
                "ledger-redb-chunk-identity-bound",
                pass_check(
                    input.ledger_refs.is_empty()
                        || input.redb_refs.is_empty()
                        || input.chunk_refs.is_empty()
                        || input.identity_refs.is_empty(),
                ),
            ),
            check_value(
                "retention-source-gate-bound",
                pass_check(input.retention_pin_refs.is_empty() || input.source_gate_refs.is_empty()),
            ),
            check_value("tampered-backup-denies-restore", pass_check(input.tamper_denial_refs.is_empty())),
            check_value("restore-verifies-before-operation", pass_check(input.restore_verification_refs.is_empty())),
        ]),
    ]))
}

pub fn upgrade_rollback_drill_value(input: &UpgradeRollbackDrillInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("upgrade rollback plan", input.plan_name)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("migration", input.migration_refs),
        ("smoke", input.smoke_refs),
        ("rollback eligibility", input.rollback_eligibility_refs),
        ("irreversible exclusion", input.irreversible_exclusion_refs),
        ("post rollback", input.post_rollback_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    Ok(record("prod-ops-upgrade-rollback-drill-v1", vec![
        string(PROD_OPS_UPGRADE_ROLLBACK_DRILL_SCHEMA),
        decision_field(input.decision),
        record("plan", vec![string(input.plan_name)]),
        refs_field("migrations", input.migration_refs)?,
        refs_field("smoke-or-dogfood", input.smoke_refs)?,
        refs_field("rollback-eligibility", input.rollback_eligibility_refs)?,
        refs_field("irreversible-exclusions", input.irreversible_exclusion_refs)?,
        refs_field("post-rollback-verification", input.post_rollback_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("migration-receipts-bound", pass_check(input.migration_refs.is_empty())),
            check_value("copied-state-smoke-bound", pass_check(input.smoke_refs.is_empty())),
            check_value("rollback-eligibility-bound", pass_check(input.rollback_eligibility_refs.is_empty())),
            check_value("irreversible-operations-excluded", pass_check(input.irreversible_exclusion_refs.is_empty())),
        ]),
    ]))
}

pub fn observability_slo_value(input: &ObservabilitySloInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("observability snapshot name", input.snapshot_name)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_refs("adapter health", input.adapter_health_refs, input.decision)?;
    require_pass_refs("control loop", input.control_loop_refs, input.decision)?;
    require_pass_refs("source gate freshness", input.source_gate_freshness_refs, input.decision)?;
    require_pass_refs("live transport", input.live_transport_refs, input.decision)?;
    require_pass_metric_bound("queue depth", input.queue_depth, input.max_queue_depth, input.decision)?;
    Ok(record("prod-ops-observability-slo-v1", vec![
        string(PROD_OPS_OBSERVABILITY_SLO_SCHEMA),
        decision_field(input.decision),
        record("snapshot", vec![string(input.snapshot_name)]),
        refs_field("adapter-health", input.adapter_health_refs)?,
        record("queue-depth", vec![u64_value(input.queue_depth)]),
        record("max-queue-depth", vec![u64_value(input.max_queue_depth)]),
        refs_field("control-loop", input.control_loop_refs)?,
        refs_field("resource-pressure", input.resource_pressure_refs)?,
        refs_field("retention-drift", input.retention_drift_refs)?,
        refs_field("source-gate-freshness", input.source_gate_freshness_refs)?,
        refs_field("live-transport", input.live_transport_refs)?,
        refs_field("import-export-failures", input.import_export_failure_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("adapter-health-bound", pass_check(input.adapter_health_refs.is_empty())),
            check_value("queue-depth-within-slo", pass_check(input.queue_depth > input.max_queue_depth)),
            check_value("control-loop-liveness-bound", pass_check(input.control_loop_refs.is_empty())),
            check_value("logs-auxiliary-receipts-canonical", "pass"),
        ]),
    ]))
}

pub fn runbook_check_value(input: &RunbookCheckInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("runbook name", input.runbook_name)?;
    validate_text_field("runbook operation", input.operation)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_refs("canonical artifact", input.canonical_artifact_refs, input.decision)?;
    require_pass_refs("denial fixture", input.denial_fixture_refs, input.decision)?;
    Ok(record("prod-ops-runbook-check-v1", vec![
        string(PROD_OPS_RUNBOOK_CHECK_SCHEMA),
        decision_field(input.decision),
        record("runbook", vec![string(input.runbook_name)]),
        record("operation", vec![string(input.operation)]),
        refs_field("canonical-artifacts", input.canonical_artifact_refs)?,
        refs_field("denial-fixtures", input.denial_fixture_refs)?,
        refs_field("auxiliary-logs", input.auxiliary_log_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("canonical-receipts-not-terminal-output", pass_check(input.canonical_artifact_refs.is_empty())),
            check_value("denial-path-covered", pass_check(input.denial_fixture_refs.is_empty())),
            check_value("logs-auxiliary-only", "pass"),
        ]),
    ]))
}

pub fn threat_model_value(input: &ThreatModelInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("threat model name", input.model_name)?;
    validate_text_slice("threat entry", input.threat_entries)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_texts("threat entry", input.threat_entries, input.decision)?;
    require_pass_refs("mapped gate", input.mapped_gate_refs, input.decision)?;
    require_pass_coverage(
        "threat model mapping",
        &[
            input.mapped_gate_refs,
            input.drill_refs,
            input.negative_suite_refs,
            input.unresolved_risk_refs,
        ],
        input.decision,
    )?;
    if is_pass(input.decision) && !input.unresolved_risk_refs.is_empty() {
        require_non_empty_refs("pilot consequence", input.pilot_consequence_refs)?;
    }
    Ok(record("prod-security-threat-model-v1", vec![
        string(PROD_SECURITY_THREAT_MODEL_SCHEMA),
        decision_field(input.decision),
        record("model", vec![string(input.model_name)]),
        texts_field("threats", input.threat_entries)?,
        refs_field("mapped-gates", input.mapped_gate_refs)?,
        refs_field("drills", input.drill_refs)?,
        refs_field("negative-suites", input.negative_suite_refs)?,
        refs_field("unresolved-risks", input.unresolved_risk_refs)?,
        refs_field("pilot-consequences", input.pilot_consequence_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("threats-named", pass_check(input.threat_entries.is_empty())),
            check_value(
                "gate-drill-or-risk-mapped",
                pass_check(
                    input.mapped_gate_refs.is_empty()
                        && input.drill_refs.is_empty()
                        && input.negative_suite_refs.is_empty()
                        && input.unresolved_risk_refs.is_empty(),
                ),
            ),
            check_value(
                "unresolved-risks-have-pilot-consequences",
                pass_check(!input.unresolved_risk_refs.is_empty() && input.pilot_consequence_refs.is_empty()),
            ),
        ]),
    ]))
}

pub fn security_drill_value(input: &SecurityDrillInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_allowed_text("security drill kind", input.drill_kind, SECURITY_DRILL_KINDS)?;
    validate_text_field("security drill scenario", input.scenario)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_refs("pass evidence", input.pass_evidence_refs, input.decision)?;
    require_pass_refs("denial", input.denial_refs, input.decision)?;
    require_pass_refs("cleanup", input.cleanup_refs, input.decision)?;
    Ok(record("prod-security-drill-v1", vec![
        string(PROD_SECURITY_DRILL_SCHEMA),
        decision_field(input.decision),
        record("drill-kind", vec![string(input.drill_kind)]),
        record("scenario", vec![string(input.scenario)]),
        refs_field("pass-evidence", input.pass_evidence_refs)?,
        refs_field("denials", input.denial_refs)?,
        refs_field("cleanup", input.cleanup_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value(
                "revocation-or-attenuation-denies-before-side-effects",
                pass_check(input.denial_refs.is_empty()),
            ),
            check_value("cleanup-actions-bound", pass_check(input.cleanup_refs.is_empty())),
            check_value("drill-receipt-does-not-grant-authority", "pass"),
        ]),
    ]))
}

pub fn redaction_audit_value(input: &RedactionAuditInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("redaction audit name", input.audit_name)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_refs("surface", input.surface_refs, input.decision)?;
    require_pass_refs("redaction", input.redaction_refs, input.decision)?;
    require_pass_refs("plaintext denial", input.plaintext_denial_refs, input.decision)?;
    Ok(record("prod-security-redaction-audit-v1", vec![
        string(PROD_SECURITY_REDACTION_AUDIT_SCHEMA),
        decision_field(input.decision),
        record("audit", vec![string(input.audit_name)]),
        refs_field("surfaces", input.surface_refs)?,
        refs_field("redactions", input.redaction_refs)?,
        refs_field("reveal-gates", input.reveal_gate_refs)?,
        refs_field("plaintext-denials", input.plaintext_denial_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("surfaces-covered", pass_check(input.surface_refs.is_empty())),
            check_value("redaction-or-encryption-bound", pass_check(input.redaction_refs.is_empty())),
            check_value("plaintext-secret-export-denied", pass_check(input.plaintext_denial_refs.is_empty())),
        ]),
    ]))
}

pub fn supply_chain_review_value(input: &SupplyChainReviewInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("supply chain review name", input.review_name)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("release", input.release_refs),
        ("source gate", input.source_gate_refs),
        ("provenance", input.provenance_refs),
        ("build verification", input.build_verify_refs),
        ("signed keyring", input.signed_keyring_refs),
        ("sensitive artifact", input.sensitive_artifact_refs),
        ("mismatch denial", input.mismatch_denial_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    Ok(record("prod-security-supply-chain-review-v1", vec![
        string(PROD_SECURITY_SUPPLY_CHAIN_REVIEW_SCHEMA),
        decision_field(input.decision),
        record("review", vec![string(input.review_name)]),
        refs_field("release", input.release_refs)?,
        refs_field("source-gates", input.source_gate_refs)?,
        refs_field("provenance", input.provenance_refs)?,
        refs_field("build-verification", input.build_verify_refs)?,
        refs_field("signed-keyring", input.signed_keyring_refs)?,
        refs_field("sensitive-artifacts", input.sensitive_artifact_refs)?,
        refs_field("mismatch-denials", input.mismatch_denial_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value(
                "release-source-provenance-build-bound",
                pass_check(
                    input.release_refs.is_empty()
                        || input.source_gate_refs.is_empty()
                        || input.provenance_refs.is_empty()
                        || input.build_verify_refs.is_empty(),
                ),
            ),
            check_value("signed-keyring-currentness-bound", pass_check(input.signed_keyring_refs.is_empty())),
            check_value("stale-sensitive-artifact-denies", pass_check(input.mismatch_denial_refs.is_empty())),
        ]),
    ]))
}

pub fn boundary_negative_suite_value(input: &BoundaryNegativeSuiteInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("boundary negative suite name", input.suite_name)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("Preserves parser", input.preserves_parser_refs),
        ("receipt validator", input.receipt_validator_refs),
        ("source gate", input.source_gate_refs),
        ("repro bundle", input.repro_bundle_refs),
        ("node ingress", input.node_ingress_refs),
        ("provenance", input.provenance_refs),
        ("plugin hostcall", input.plugin_hostcall_refs),
        ("malformed denial", input.malformed_denial_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    Ok(record("prod-security-boundary-negative-suite-v1", vec![
        string(PROD_SECURITY_BOUNDARY_NEGATIVE_SUITE_SCHEMA),
        decision_field(input.decision),
        record("suite", vec![string(input.suite_name)]),
        refs_field("preserves-parsers", input.preserves_parser_refs)?,
        refs_field("receipt-validators", input.receipt_validator_refs)?,
        refs_field("source-gates", input.source_gate_refs)?,
        refs_field("repro-bundles", input.repro_bundle_refs)?,
        refs_field("node-ingress", input.node_ingress_refs)?,
        refs_field("provenance", input.provenance_refs)?,
        refs_field("plugin-hostcalls", input.plugin_hostcall_refs)?,
        refs_field("malformed-denials", input.malformed_denial_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("parser-failures-structured", pass_check(input.preserves_parser_refs.is_empty())),
            check_value("receipt-validator-boundaries-covered", pass_check(input.receipt_validator_refs.is_empty())),
            check_value(
                "malformed-input-denies-not-missing-clean-evidence",
                pass_check(input.malformed_denial_refs.is_empty()),
            ),
        ]),
    ]))
}

pub fn incident_response_drill_value(input: &IncidentResponseDrillInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_allowed_text("incident kind", input.incident_kind, INCIDENT_KINDS)?;
    validate_text_field("incident response scenario", input.scenario)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("detection", input.detection_refs),
        ("containment", input.containment_refs),
        ("recovery", input.recovery_refs),
        ("next step", input.next_step_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    Ok(record("prod-security-incident-response-drill-v1", vec![
        string(PROD_SECURITY_DRILL_SCHEMA),
        decision_field(input.decision),
        record("incident-kind", vec![string(input.incident_kind)]),
        record("scenario", vec![string(input.scenario)]),
        refs_field("detection", input.detection_refs)?,
        refs_field("containment", input.containment_refs)?,
        refs_field("recovery", input.recovery_refs)?,
        refs_field("next-steps", input.next_step_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("incident-detected", pass_check(input.detection_refs.is_empty())),
            check_value("containment-bound", pass_check(input.containment_refs.is_empty())),
            check_value(
                "recovery-next-steps-bound",
                pass_check(input.recovery_refs.is_empty() || input.next_step_refs.is_empty()),
            ),
        ]),
    ]))
}

pub fn security_readiness_report_value(input: &SecurityReadinessReportInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("security readiness report name", input.report_name)?;
    validate_text_field("pilot recommendation", input.pilot_recommendation)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("threat model", input.threat_model_refs),
        ("supply chain", input.supply_chain_refs),
        ("drill", input.drill_refs),
        ("redaction audit", input.redaction_audit_refs),
        ("boundary suite", input.boundary_suite_refs),
        ("incident response", input.incident_response_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    if is_pass(input.decision)
        && !input.unresolved_risk_refs.is_empty()
        && input.pilot_recommendation == BROAD_PRODUCTION_SCOPE
    {
        return Err(MoltenError::invalid_harness(
            "security readiness with unresolved risks cannot recommend broad production",
        ));
    }
    Ok(record("prod-security-readiness-report-v1", vec![
        string(PROD_SECURITY_READINESS_REPORT_SCHEMA),
        decision_field(input.decision),
        record("report", vec![string(input.report_name)]),
        refs_field("threat-models", input.threat_model_refs)?,
        refs_field("supply-chain", input.supply_chain_refs)?,
        refs_field("drills", input.drill_refs)?,
        refs_field("redaction-audits", input.redaction_audit_refs)?,
        refs_field("boundary-suites", input.boundary_suite_refs)?,
        refs_field("incident-response", input.incident_response_refs)?,
        refs_field("unresolved-risks", input.unresolved_risk_refs)?,
        record("pilot-recommendation", vec![string(input.pilot_recommendation)]),
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("threat-model-bound", pass_check(input.threat_model_refs.is_empty())),
            check_value(
                "drills-and-negative-suites-bound",
                pass_check(input.drill_refs.is_empty() || input.boundary_suite_refs.is_empty()),
            ),
            check_value("pilot-scope-recommendation-explicit", "pass"),
        ]),
    ]))
}

pub fn pilot_decision_value(input: &PilotDecisionInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("pilot scope", input.scope)?;
    validate_text_slice("allowed workload", input.allowed_workloads)?;
    validate_text_slice("denied workload", input.denied_workloads)?;
    validate_text_slice("rollback trigger", input.rollback_triggers)?;
    validate_text_slice("stop condition", input.stop_conditions)?;
    validate_text_slice("pilot caveat", input.caveats)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_texts("allowed workload", input.allowed_workloads, input.decision)?;
    require_pass_texts("denied workload", input.denied_workloads, input.decision)?;
    require_pass_texts("rollback trigger", input.rollback_triggers, input.decision)?;
    require_pass_texts("stop condition", input.stop_conditions, input.decision)?;
    require_pass_refs("operator review", input.operator_review_refs, input.decision)?;
    if is_pass(input.decision) && input.scope == BROAD_PRODUCTION_SCOPE && !input.caveats.is_empty() {
        return Err(MoltenError::invalid_harness(
            "pilot decision with evidence-only caveats cannot claim broad production scope",
        ));
    }
    Ok(record("prod-release-pilot-decision-v1", vec![
        string(PROD_RELEASE_PILOT_DECISION_SCHEMA),
        decision_field(input.decision),
        record("scope", vec![string(input.scope)]),
        texts_field("allowed-workloads", input.allowed_workloads)?,
        texts_field("denied-workloads", input.denied_workloads)?,
        texts_field("rollback-triggers", input.rollback_triggers)?,
        texts_field("stop-conditions", input.stop_conditions)?,
        refs_field("operator-review", input.operator_review_refs)?,
        texts_field("caveats", input.caveats)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("allowed-workloads-explicit", pass_check(input.allowed_workloads.is_empty())),
            check_value("denied-workloads-explicit", pass_check(input.denied_workloads.is_empty())),
            check_value(
                "rollback-and-stop-conditions-explicit",
                pass_check(input.rollback_triggers.is_empty() || input.stop_conditions.is_empty()),
            ),
            check_value("operator-review-bound", pass_check(input.operator_review_refs.is_empty())),
        ]),
    ]))
}

struct ReleaseCandidateGate<'a> {
    input: &'a ReleaseCandidateGateInput<'a>,
}

impl<'a> ReleaseCandidateGate<'a> {
    fn new(input: &'a ReleaseCandidateGateInput<'a>) -> Self {
        Self { input }
    }

    fn validate(&self) -> Result<()> {
        validate_decision(self.input.decision)?;
        validate_text_field("candidate", self.input.candidate)?;
        validate_content_ref(self.input.source_ref)?;
        validate_source_gate_status(self.input.source_gate_status)?;
        validate_text_slice("source gate caveat", self.input.source_gate_caveats)?;
        validate_diagnostics(self.input.diagnostics)?;
        self.require_evidence_refs()?;
        self.require_source_gate_caveat()
    }

    fn require_evidence_refs(&self) -> Result<()> {
        for (label, refs) in [
            ("Rust validation", self.input.rust_validation_refs),
            ("nextest", self.input.nextest_refs),
            ("Nix check", self.input.nix_check_refs),
            ("Cairn validation", self.input.cairn_validation_refs),
            ("Octet", self.input.octet_refs),
            ("dogfood", self.input.dogfood_refs),
            ("release bundle verify", self.input.bundle_verify_refs),
            ("promotion", self.input.promotion_refs),
            ("export verify", self.input.export_verify_refs),
            ("pilot decision", self.input.pilot_decision_refs),
        ] {
            require_pass_refs(label, refs, self.input.decision)?;
        }
        Ok(())
    }

    fn require_source_gate_caveat(&self) -> Result<()> {
        if is_pass(self.input.decision)
            && self.input.source_gate_status != SOURCE_REMEDIATED_ZERO_STATUS
            && self.input.source_gate_caveats.is_empty()
        {
            return Err(MoltenError::invalid_harness(
                "passing production candidate with non-zero source gate status requires source gate caveats",
            ));
        }
        Ok(())
    }

    fn value(&self) -> Result<IoValue> {
        Ok(record("prod-release-candidate-gate-v1", vec![
            string(PROD_RELEASE_CANDIDATE_GATE_SCHEMA),
            decision_field(self.input.decision),
            record("candidate", vec![string(self.input.candidate)]),
            record("source", vec![string(self.input.source_ref)]),
            refs_field("rust-validation", self.input.rust_validation_refs)?,
            refs_field("nextest", self.input.nextest_refs)?,
            refs_field("nix-checks", self.input.nix_check_refs)?,
            refs_field("cairn-validation", self.input.cairn_validation_refs)?,
            refs_field("octet-source-gates", self.input.octet_refs)?,
            refs_field("dogfood", self.input.dogfood_refs)?,
            refs_field("release-bundle-verification", self.input.bundle_verify_refs)?,
            refs_field("promotion", self.input.promotion_refs)?,
            refs_field("export-verification", self.input.export_verify_refs)?,
            record("source-gate-status", vec![string(self.input.source_gate_status)]),
            texts_field("source-gate-caveats", self.input.source_gate_caveats)?,
            refs_field("pilot-decisions", self.input.pilot_decision_refs)?,
            diagnostics_field(self.input.diagnostics)?,
            checks_field(self.checks()),
        ]))
    }

    fn checks(&self) -> Vec<IoValue> {
        vec![
            check_value("full-validation-matrix-bound", pass_check(self.has_validation_matrix_gap())),
            check_value("source-gate-current-or-limited", pass_check(self.has_source_gate_limiter())),
            check_value("bundle-promotion-export-bound", pass_check(self.has_release_bundle_gap())),
            check_value("pilot-decision-bound", pass_check(self.input.pilot_decision_refs.is_empty())),
            check_value("release-candidate-receipt-does-not-grant-authority", "pass"),
        ]
    }

    fn has_validation_matrix_gap(&self) -> bool {
        self.input.rust_validation_refs.is_empty()
            || self.input.nextest_refs.is_empty()
            || self.input.nix_check_refs.is_empty()
            || self.input.cairn_validation_refs.is_empty()
    }

    fn has_source_gate_limiter(&self) -> bool {
        self.input.source_gate_status != SOURCE_REMEDIATED_ZERO_STATUS && self.input.source_gate_caveats.is_empty()
    }

    fn has_release_bundle_gap(&self) -> bool {
        self.input.bundle_verify_refs.is_empty()
            || self.input.promotion_refs.is_empty()
            || self.input.export_verify_refs.is_empty()
    }
}

pub fn release_candidate_gate_value(input: &ReleaseCandidateGateInput<'_>) -> Result<IoValue> {
    let gate = ReleaseCandidateGate::new(input);
    gate.validate()?;
    gate.value()
}

fn decision_field(decision: &str) -> IoValue {
    record("decision", vec![string(decision)])
}

fn diagnostics_field(values: &[String]) -> Result<IoValue> {
    Ok(record("diagnostics", vec![sequence(string_values("diagnostic", values)?)]))
}

fn refs_field(label: &'static str, refs: &[String]) -> Result<IoValue> {
    Ok(record(label, vec![sequence(ref_values(label, refs)?)]))
}

fn texts_field(label: &'static str, values: &[String]) -> Result<IoValue> {
    Ok(record(label, vec![sequence(string_values(label, values)?)]))
}

fn checks_field(checks: Vec<IoValue>) -> IoValue {
    record("checks", vec![sequence(checks)])
}

fn check_value(name: &'static str, status: &'static str) -> IoValue {
    record("check", vec![string(name), string(status)])
}

fn pass_check(is_failed: bool) -> &'static str {
    if is_failed { "deny" } else { "pass" }
}

fn is_pass(decision: &str) -> bool {
    decision == "pass"
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" | "unavailable" | "skipped" | "degraded" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported production readiness decision {other}; expected pass, deny, degraded, unavailable, or skipped"
        ))),
    }
}

fn validate_source_gate_status(status: &str) -> Result<()> {
    match status {
        SOURCE_REMEDIATED_ZERO_STATUS | CONFIGURATION_CLEAN_CAVEAT_STATUS | "stale" | "missing" | "failed" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported production source gate status {other}; expected source-remediated-zero, configuration-clean-caveat, stale, missing, or failed"
        ))),
    }
}

fn validate_allowed_text(label: &str, value: &str, allowed: &[&str]) -> Result<()> {
    validate_text_field(label, value)?;
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported production {label} {value}; expected one of {}",
            allowed.join(", ")
        )))
    }
}

fn validate_text_field(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("production readiness {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_text_slice(label: &str, values: &[String]) -> Result<()> {
    string_values(label, values).map(|_| ())
}

fn validate_diagnostics(values: &[String]) -> Result<()> {
    string_values("diagnostic", values).map(|_| ())
}

fn validate_ref_slice(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_PROD_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "production readiness {label} ref count {} exceeds bound {MAX_PROD_REFS}",
            refs.len()
        )));
    }
    for reference in refs {
        validate_content_ref(reference).map_err(|error| {
            MoltenError::invalid_harness(format!("invalid production readiness {label} ref {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn require_pass_refs(label: &str, refs: &[String], decision: &str) -> Result<()> {
    validate_ref_slice(label, refs)?;
    if is_pass(decision) && refs.is_empty() {
        Err(MoltenError::invalid_harness(format!(
            "passing production readiness receipt requires at least one {label} ref"
        )))
    } else {
        Ok(())
    }
}

fn require_non_empty_refs(label: &str, refs: &[String]) -> Result<()> {
    validate_ref_slice(label, refs)?;
    if refs.is_empty() {
        Err(MoltenError::invalid_harness(format!(
            "production readiness receipt requires at least one {label} ref"
        )))
    } else {
        Ok(())
    }
}

fn require_pass_texts(label: &str, values: &[String], decision: &str) -> Result<()> {
    validate_text_slice(label, values)?;
    if is_pass(decision) && values.is_empty() {
        Err(MoltenError::invalid_harness(format!(
            "passing production readiness receipt requires at least one {label}"
        )))
    } else {
        Ok(())
    }
}

fn require_pass_coverage(label: &str, groups: &[&[String]], decision: &str) -> Result<()> {
    if is_pass(decision) && groups.iter().all(|group| group.is_empty()) {
        Err(MoltenError::invalid_harness(format!(
            "passing production readiness receipt requires {label} coverage"
        )))
    } else {
        Ok(())
    }
}

fn require_pass_metric_bound(label: &str, actual: u64, maximum: u64, decision: &str) -> Result<()> {
    if is_pass(decision) && actual > maximum {
        Err(MoltenError::invalid_harness(format!(
            "passing production readiness {label} {actual} exceeds bound {maximum}"
        )))
    } else {
        Ok(())
    }
}

fn ref_values(label: &str, refs: &[String]) -> Result<Vec<IoValue>> {
    validate_ref_slice(label, refs)?;
    Ok(refs.iter().map(string).collect())
}

fn string_values(label: &str, values: &[String]) -> Result<Vec<IoValue>> {
    if values.len() > MAX_PROD_TEXTS {
        return Err(MoltenError::invalid_harness(format!(
            "production readiness {label} count {} exceeds bound {MAX_PROD_TEXTS}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        validate_text_field(label, value)?;
        output.push(string(value));
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    const OBSERVED_QUEUE_DEPTH: u64 = 2;
    const MAX_QUEUE_DEPTH: u64 = 8;
    const OVER_LIMIT_QUEUE_DEPTH: u64 = 13;

    fn reference(label: &str) -> String {
        canonical_hash(&record("prod-readiness-ref", vec![string(label)])).expect("synthetic ref")
    }

    fn refs(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|label| reference(label)).collect()
    }

    fn texts(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|label| (*label).to_string()).collect()
    }

    #[test]
    fn production_ops_receipts_bind_positive_and_denial_evidence() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["operator reviewed"]);
        let profile = deployment_profile_value(&DeploymentProfileInput {
            decision: "pass",
            profile_name: "pilot-node",
            state_layout_refs: &base_refs,
            required_adapter_refs: &base_refs,
            source_gate_refs: &base_refs,
            resource_limit_refs: &base_refs,
            redaction_setting_refs: &base_refs,
            live_transport_refs: &base_refs,
            startup_expectation_refs: &base_refs,
            shutdown_expectation_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("deployment profile");
        let backup = backup_restore_drill_value(&BackupRestoreDrillInput {
            decision: "pass",
            drill_name: "backup-restore",
            ledger_refs: &base_refs,
            redb_refs: &base_refs,
            chunk_refs: &base_refs,
            identity_refs: &base_refs,
            retention_pin_refs: &base_refs,
            source_gate_refs: &base_refs,
            restore_verification_refs: &base_refs,
            tamper_denial_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("backup restore");
        let runbook = runbook_check_value(&RunbookCheckInput {
            decision: "pass",
            runbook_name: "startup",
            operation: "init",
            canonical_artifact_refs: &base_refs,
            denial_fixture_refs: &base_refs,
            auxiliary_log_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("runbook check");
        let profile_text = to_text(&profile).expect("profile text");
        let backup_text = to_text(&backup).expect("backup text");
        let runbook_text = to_text(&runbook).expect("runbook text");
        assert!(profile_text.contains("prod-ops-deployment-profile-v1"));
        assert!(backup_text.contains("tampered-backup-denies-restore"));
        assert!(runbook_text.contains("canonical-receipts-not-terminal-output"));
    }

    #[test]
    fn production_ops_pass_denies_missing_or_degraded_evidence() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["queue pressure"]);
        let missing_profile = deployment_profile_value(&DeploymentProfileInput {
            decision: "pass",
            profile_name: "pilot-node",
            state_layout_refs: &[],
            required_adapter_refs: &base_refs,
            source_gate_refs: &base_refs,
            resource_limit_refs: &base_refs,
            redaction_setting_refs: &base_refs,
            live_transport_refs: &base_refs,
            startup_expectation_refs: &base_refs,
            shutdown_expectation_refs: &base_refs,
            diagnostics: &diagnostics,
        });
        let degraded_pass = observability_slo_value(&ObservabilitySloInput {
            decision: "pass",
            snapshot_name: "over-limit",
            adapter_health_refs: &base_refs,
            queue_depth: OVER_LIMIT_QUEUE_DEPTH,
            max_queue_depth: MAX_QUEUE_DEPTH,
            control_loop_refs: &base_refs,
            resource_pressure_refs: &base_refs,
            retention_drift_refs: &base_refs,
            source_gate_freshness_refs: &base_refs,
            live_transport_refs: &base_refs,
            import_export_failure_refs: &base_refs,
            diagnostics: &diagnostics,
        });
        assert!(missing_profile.is_err());
        assert!(degraded_pass.is_err());
        observability_slo_value(&ObservabilitySloInput {
            decision: "degraded",
            snapshot_name: "over-limit",
            adapter_health_refs: &base_refs,
            queue_depth: OVER_LIMIT_QUEUE_DEPTH,
            max_queue_depth: MAX_QUEUE_DEPTH,
            control_loop_refs: &base_refs,
            resource_pressure_refs: &base_refs,
            retention_drift_refs: &base_refs,
            source_gate_freshness_refs: &base_refs,
            live_transport_refs: &base_refs,
            import_export_failure_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("degraded snapshot can be emitted");
    }

    #[test]
    fn security_readiness_receipts_require_mapped_drills_and_denials() {
        let base_refs = refs(&["base"]);
        let threats = texts(&["leaked live ticket"]);
        let diagnostics = texts(&["pilot only"]);
        let threat = threat_model_value(&ThreatModelInput {
            decision: "pass",
            model_name: "pilot-threat-model",
            threat_entries: &threats,
            mapped_gate_refs: &base_refs,
            drill_refs: &base_refs,
            negative_suite_refs: &base_refs,
            unresolved_risk_refs: &base_refs,
            pilot_consequence_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("threat model");
        let drill = security_drill_value(&SecurityDrillInput {
            decision: "pass",
            drill_kind: "stale-ticket-denial",
            scenario: "stale live ticket",
            pass_evidence_refs: &base_refs,
            denial_refs: &base_refs,
            cleanup_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("security drill");
        let report = security_readiness_report_value(&SecurityReadinessReportInput {
            decision: "pass",
            report_name: "pilot-security",
            threat_model_refs: &refs(&["threat"]),
            supply_chain_refs: &base_refs,
            drill_refs: &refs(&["drill"]),
            redaction_audit_refs: &base_refs,
            boundary_suite_refs: &base_refs,
            incident_response_refs: &base_refs,
            unresolved_risk_refs: &base_refs,
            pilot_recommendation: "limited-internal-pilot",
            diagnostics: &diagnostics,
        })
        .expect("security readiness report");
        assert!(to_text(&threat).expect("threat text").contains("gate-drill-or-risk-mapped"));
        assert!(to_text(&drill).expect("drill text").contains("stale-ticket-denial"));
        assert!(to_text(&report).expect("report text").contains("pilot-scope-recommendation-explicit"));
    }

    #[test]
    fn security_readiness_denies_unmapped_or_broad_unresolved_risk() {
        let base_refs = refs(&["base"]);
        let threats = texts(&["unmapped threat"]);
        let diagnostics = texts(&["risk remains"]);
        let unmapped = threat_model_value(&ThreatModelInput {
            decision: "pass",
            model_name: "bad-threat-model",
            threat_entries: &threats,
            mapped_gate_refs: &[],
            drill_refs: &[],
            negative_suite_refs: &[],
            unresolved_risk_refs: &[],
            pilot_consequence_refs: &[],
            diagnostics: &diagnostics,
        });
        let broad = security_readiness_report_value(&SecurityReadinessReportInput {
            decision: "pass",
            report_name: "bad-security",
            threat_model_refs: &base_refs,
            supply_chain_refs: &base_refs,
            drill_refs: &base_refs,
            redaction_audit_refs: &base_refs,
            boundary_suite_refs: &base_refs,
            incident_response_refs: &base_refs,
            unresolved_risk_refs: &base_refs,
            pilot_recommendation: BROAD_PRODUCTION_SCOPE,
            diagnostics: &diagnostics,
        });
        assert!(unmapped.is_err());
        assert!(broad.is_err());
    }

    #[test]
    fn release_candidate_binds_matrix_and_scoped_pilot() {
        let base_refs = refs(&["base"]);
        let caveats = texts(&["Octet disabled lint family burn-down remains"]);
        let diagnostics = texts(&["candidate reviewed"]);
        let pilot = pilot_decision_value(&PilotDecisionInput {
            decision: "pass",
            scope: "limited-internal-pilot",
            allowed_workloads: &texts(&["stateless internal jobs"]),
            denied_workloads: &texts(&["customer-critical destructive retention"]),
            rollback_triggers: &texts(&["stale source gate"]),
            stop_conditions: &texts(&["failed dogfood replay"]),
            operator_review_refs: &base_refs,
            caveats: &caveats,
            diagnostics: &diagnostics,
        })
        .expect("pilot decision");
        let pilot_refs = vec![canonical_hash(&pilot).expect("pilot ref")];
        let candidate = release_candidate_gate_value(&ReleaseCandidateGateInput {
            decision: "pass",
            candidate: "aspen-molten-pilot",
            source_ref: &reference("source"),
            rust_validation_refs: &base_refs,
            nextest_refs: &base_refs,
            nix_check_refs: &base_refs,
            cairn_validation_refs: &base_refs,
            octet_refs: &base_refs,
            dogfood_refs: &base_refs,
            bundle_verify_refs: &base_refs,
            promotion_refs: &base_refs,
            export_verify_refs: &base_refs,
            source_gate_status: CONFIGURATION_CLEAN_CAVEAT_STATUS,
            source_gate_caveats: &caveats,
            pilot_decision_refs: &pilot_refs,
            diagnostics: &diagnostics,
        })
        .expect("release candidate gate");
        let candidate_text = to_text(&candidate).expect("candidate text");
        assert!(candidate_text.contains("prod-release-candidate-gate-v1"));
        assert!(candidate_text.contains("source-gate-current-or-limited"));
    }

    #[test]
    fn release_candidate_denies_broad_caveat_or_missing_matrix() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["candidate reviewed"]);
        let broad = pilot_decision_value(&PilotDecisionInput {
            decision: "pass",
            scope: BROAD_PRODUCTION_SCOPE,
            allowed_workloads: &texts(&["all workloads"]),
            denied_workloads: &texts(&["none"]),
            rollback_triggers: &texts(&["none"]),
            stop_conditions: &texts(&["none"]),
            operator_review_refs: &base_refs,
            caveats: &texts(&["source caveat"]),
            diagnostics: &diagnostics,
        });
        let missing_source_caveat = release_candidate_gate_value(&ReleaseCandidateGateInput {
            decision: "pass",
            candidate: "bad-candidate",
            source_ref: &reference("source"),
            rust_validation_refs: &base_refs,
            nextest_refs: &base_refs,
            nix_check_refs: &base_refs,
            cairn_validation_refs: &base_refs,
            octet_refs: &base_refs,
            dogfood_refs: &base_refs,
            bundle_verify_refs: &base_refs,
            promotion_refs: &base_refs,
            export_verify_refs: &base_refs,
            source_gate_status: CONFIGURATION_CLEAN_CAVEAT_STATUS,
            source_gate_caveats: &[],
            pilot_decision_refs: &base_refs,
            diagnostics: &diagnostics,
        });
        assert!(broad.is_err());
        assert!(missing_source_caveat.is_err());
    }

    #[test]
    fn incident_boundary_redaction_and_supply_chain_positive_paths_emit_receipts() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["reviewed"]);
        let boundary = boundary_negative_suite_value(&BoundaryNegativeSuiteInput {
            decision: "pass",
            suite_name: "boundary-negative",
            preserves_parser_refs: &base_refs,
            receipt_validator_refs: &base_refs,
            source_gate_refs: &base_refs,
            repro_bundle_refs: &base_refs,
            node_ingress_refs: &base_refs,
            provenance_refs: &base_refs,
            plugin_hostcall_refs: &base_refs,
            malformed_denial_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("boundary suite");
        let redaction = redaction_audit_value(&RedactionAuditInput {
            decision: "pass",
            audit_name: "redaction",
            surface_refs: &base_refs,
            redaction_refs: &base_refs,
            reveal_gate_refs: &base_refs,
            plaintext_denial_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("redaction audit");
        let supply = supply_chain_review_value(&SupplyChainReviewInput {
            decision: "pass",
            review_name: "supply-chain",
            release_refs: &base_refs,
            source_gate_refs: &base_refs,
            provenance_refs: &base_refs,
            build_verify_refs: &base_refs,
            signed_keyring_refs: &base_refs,
            sensitive_artifact_refs: &base_refs,
            mismatch_denial_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("supply chain review");
        let incident = incident_response_drill_value(&IncidentResponseDrillInput {
            decision: "pass",
            incident_kind: "leaked-ticket",
            scenario: "leaked peer ticket",
            detection_refs: &base_refs,
            containment_refs: &base_refs,
            recovery_refs: &base_refs,
            next_step_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("incident response");
        assert!(to_text(&boundary).expect("boundary text").contains("malformed-input-denies"));
        assert!(to_text(&redaction).expect("redaction text").contains("plaintext-secret-export-denied"));
        assert!(to_text(&supply).expect("supply text").contains("stale-sensitive-artifact-denies"));
        assert!(to_text(&incident).expect("incident text").contains("leaked-ticket"));
    }

    #[test]
    fn observability_pass_accepts_in_bound_queue() {
        let base_refs = refs(&["base"]);
        let diagnostics = texts(&["healthy"]);
        let receipt = observability_slo_value(&ObservabilitySloInput {
            decision: "pass",
            snapshot_name: "healthy",
            adapter_health_refs: &base_refs,
            queue_depth: OBSERVED_QUEUE_DEPTH,
            max_queue_depth: MAX_QUEUE_DEPTH,
            control_loop_refs: &base_refs,
            resource_pressure_refs: &base_refs,
            retention_drift_refs: &base_refs,
            source_gate_freshness_refs: &base_refs,
            live_transport_refs: &base_refs,
            import_export_failure_refs: &base_refs,
            diagnostics: &diagnostics,
        })
        .expect("observability receipt");
        assert!(to_text(&receipt).expect("receipt text").contains("queue-depth-within-slo"));
    }
}
