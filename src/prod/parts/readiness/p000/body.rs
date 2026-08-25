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
const PRODUCTION_PROFILE_SCHEMA_VERSION: u64 = 1;
const PRODUCTION_PROFILE_SOURCE_LANGUAGE: &str = "nickel";

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
    pub schema_id: &'a str,
    pub schema_version: u64,
    pub source_language: &'a str,
    pub profile_identity: &'a str,
    pub profile_ref: &'a str,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateEvidenceBinding<'a> {
    pub artifact_ref: &'a str,
    pub source_ref: &'a str,
}

pub struct ReleaseCandidateGateInput<'a> {
    pub decision: &'a str,
    pub candidate: &'a str,
    pub source_ref: &'a str,
    pub rust_validation_evidence: &'a [CandidateEvidenceBinding<'a>],
    pub nextest_evidence: &'a [CandidateEvidenceBinding<'a>],
    pub nix_check_evidence: &'a [CandidateEvidenceBinding<'a>],
    pub cairn_validation_evidence: &'a [CandidateEvidenceBinding<'a>],
    pub octet_evidence: &'a [CandidateEvidenceBinding<'a>],
    pub dogfood_evidence: &'a [CandidateEvidenceBinding<'a>],
    pub bundle_verify_evidence: &'a [CandidateEvidenceBinding<'a>],
    pub promotion_evidence: &'a [CandidateEvidenceBinding<'a>],
    pub export_verify_evidence: &'a [CandidateEvidenceBinding<'a>],
    pub source_gate_status: &'a str,
    pub source_gate_caveats: &'a [String],
    pub pilot_decision_evidence: &'a [CandidateEvidenceBinding<'a>],
    pub diagnostics: &'a [String],
}

pub fn deployment_profile_value(input: &DeploymentProfileInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("profile name", input.profile_name)?;
    validate_profile_metadata(input)?;
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
        record("schema-id", vec![string(input.schema_id)]),
        record("schema-version", vec![u64_value(input.schema_version)]),
        record("source-language", vec![string(input.source_language)]),
        record("profile-identity", vec![string(input.profile_identity)]),
        record("profile-ref", vec![string(input.profile_ref)]),
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
            check_value("profile-metadata-bound", "pass"),
            check_value("profile-receipt-does-not-grant-authority", "pass"),
            check_value("profile-metadata-does-not-grant-subsystem-trust", "pass"),
        ]),
    ]))
}
