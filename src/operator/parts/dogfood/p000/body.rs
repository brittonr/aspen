type IoValue = preserves::IOValue;
type Path = std::path::Path;
type Record<T> = preserves::Record<T>;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type SignedReceiptKey = crate::evidence::SignedReceiptKey;
type SignedReceiptKeyRevocation = crate::evidence::SignedReceiptKeyRevocation;
type VerifySignedReceiptKeyringPolicy<'a> = crate::evidence::VerifySignedReceiptKeyringPolicy<'a>;
type VerifySignedReceiptPolicy<'a> = crate::evidence::VerifySignedReceiptPolicy<'a>;

fn verify_signed_receipt_with_policy(
    value: &IoValue,
    policy: &VerifySignedReceiptPolicy<'_>,
) -> Result<crate::evidence::SignedReceipt> {
    crate::evidence::verify_signed_receipt_with_policy(value, policy)
}

fn verify_signed_receipt_with_keyring_policy(
    value: &IoValue,
    policy: &VerifySignedReceiptKeyringPolicy<'_>,
) -> Result<crate::evidence::SignedReceiptWithKey> {
    crate::evidence::verify_signed_receipt_with_keyring_policy(value, policy)
}

pub const RELEASE_EVIDENCE_SIGNING_PURPOSE: &str = "release-evidence";
pub const RELEASE_PROMOTION_SIGNING_PURPOSE: &str = "release-promotion";

const LOCAL_NODE_WORKFLOW_ID: &str = "dogfood:local-node";
const DOGFOOD_HARNESS_SUITE: &str = r#"<harness-suite-v1 "molten.harness.suite.v1" "dogfood-repro" 3
  <budget-v1 "molten.harness.budget.v1" <limits 32 8 128 65536>>
  <actor-registry-v1 "molten.harness.actor-registry.v1" [
    <actor "producer" "native">
  ]>
  <capabilities-v1 "molten.harness.capabilities.v1" [
    <grant "producer" "assert" #f "dogfood.ready">
  ]>
  [<assert "producer" "dogfood.ready">]>"#;

const MAX_OPERATOR_STEPS: usize = 64;
const MAX_OPERATOR_REFS: usize = 4096;
const MAX_OPERATOR_DIAGNOSTICS: usize = 256;
const _: () = assert!(MAX_OPERATOR_STEPS > 0);
const _: () = assert!(MAX_OPERATOR_REFS > MAX_OPERATOR_STEPS);
const _: () = assert!(MAX_OPERATOR_DIAGNOSTICS > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorStepInput<'a> {
    pub name: &'a str,
    pub request_ref: Option<&'a str>,
    pub receipt_ref: Option<&'a str>,
    pub decision: &'a str,
    pub replay_status: &'a str,
    pub mandatory: bool,
    pub artifact_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorCheckpointInput<'a> {
    pub workflow_id: &'a str,
    pub sequence: u64,
    pub step_ref: &'a str,
    pub request_ref: Option<&'a str>,
    pub receipt_ref: Option<&'a str>,
    pub result_ref: Option<&'a str>,
    pub state_root_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorWorkflowInput<'a> {
    pub workflow_id: &'a str,
    pub steps: &'a [IoValue],
    pub policy_refs: &'a [String],
    pub capability_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub replay_profile: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DogfoodReportInput<'a> {
    pub workflow_value: &'a IoValue,
    pub checkpoint_values: &'a [IoValue],
    pub gate_receipt_refs: &'a [String],
    pub repro_bundle_refs: &'a [String],
    pub final_state_ref: &'a str,
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseGateInput<'a> {
    pub report_value: &'a IoValue,
    pub node_startup_ref: &'a str,
    pub node_shutdown_ref: &'a str,
    pub harness_gate_refs: &'a [String],
    pub catalog_query_refs: &'a [String],
    pub repro_verify_refs: &'a [String],
    pub replay_index_refs: &'a [String],
    pub gc_refs: &'a [String],
    pub validation_command_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseGateReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub report_ref: String,
    pub startup_ref: String,
    pub shutdown_ref: String,
    pub harness_gate_refs: Vec<String>,
    pub catalog_query_refs: Vec<String>,
    pub repro_verify_refs: Vec<String>,
    pub replay_index_refs: Vec<String>,
    pub gc_refs: Vec<String>,
    pub validation_command_refs: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixDogfoodEvidenceInput<'a> {
    pub output_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixDogfoodEvidence {
    pub evidence_ref: String,
    pub output_path: String,
    pub output_path_ref: String,
    pub report_ref: String,
    pub release_gate_ref: String,
    pub replay_verify_ref: String,
    pub replay_index_ref: String,
    pub summary_ref: String,
    pub nextest_marker_ref: String,
    pub nextest_check_path: String,
    pub file_refs: Vec<(String, String)>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixDogfoodVerifyInput<'a> {
    pub output_path: &'a Path,
    pub evidence_value: &'a IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NixDogfoodVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub evidence_ref: String,
    pub output_path_ref: String,
    pub report_ref: String,
    pub release_gate_ref: String,
    pub replay_verify_ref: String,
    pub replay_index_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceBundleInput<'a> {
    pub output_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceBundle {
    pub bundle_ref: String,
    pub output_path: String,
    pub output_path_ref: String,
    pub report_ref: String,
    pub release_gate_ref: String,
    pub replay_verify_ref: String,
    pub replay_index_ref: String,
    pub nix_evidence_ref: String,
    pub nix_verify_ref: String,
    pub summary_ref: String,
    pub nextest_marker_ref: String,
    pub nextest_check_path: String,
    pub member_refs: Vec<(String, String)>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceBundleVerifyInput<'a> {
    pub output_path: &'a Path,
    pub bundle_value: &'a IoValue,
    pub signed_member_values: &'a [IoValue],
    pub signed_purpose: &'a str,
    pub signed_trust_root: &'a str,
    pub signed_key: &'a str,
    pub signed_keys: &'a [SignedReceiptKey],
    pub signed_key_revocations: &'a [SignedReceiptKeyRevocation],
    pub signed_key_ref: Option<&'a str>,
    pub signed_key_id: Option<&'a str>,
    pub signed_signer: Option<&'a str>,
    pub is_signed_members_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceBundleVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub bundle_ref: String,
    pub output_path_ref: String,
    pub report_ref: String,
    pub release_gate_ref: String,
    pub replay_verify_ref: String,
    pub replay_index_ref: String,
    pub nix_evidence_ref: String,
    pub nix_verify_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePromotionGateInput<'a> {
    pub output_path: &'a Path,
    pub bundle_verify_value: &'a IoValue,
    pub source_evidence: &'a str,
    pub octet_evidence: &'a str,
    pub cairn_evidence: &'a str,
    pub signed_keys: &'a [SignedReceiptKey],
    pub signed_key_revocations: &'a [SignedReceiptKeyRevocation],
    pub signed_trust_root: &'a str,
    pub signed_signer: Option<&'a str>,
    pub signed_key_ref: Option<&'a str>,
    pub signed_key_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePromotionGateReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub bundle_verify_ref: String,
    pub bundle_ref: String,
    pub output_path_ref: String,
    pub selected_key_ref: String,
    pub source_ref: String,
    pub octet_ref: String,
    pub cairn_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

pub struct ReleasePromotionSummaryInput<'a> {
    pub output_path: &'a Path,
    pub signed_keys: &'a [SignedReceiptKey],
    pub signed_key_revocations: &'a [SignedReceiptKeyRevocation],
    pub signed_trust_root: &'a str,
    pub signed_signer: Option<&'a str>,
    pub signed_key_ref: Option<&'a str>,
    pub signed_key_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePromotionSummary {
    pub summary_ref: String,
    pub decision: String,
    pub promotion_ref: String,
    pub signed_envelope_ref: String,
    pub signed_subject_ref: String,
    pub signed_key_ref: String,
    pub bundle_verify_ref: String,
    pub source_ref: String,
    pub octet_ref: String,
    pub cairn_ref: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseExportManifestInput<'a> {
    pub output_path: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseExportManifest {
    pub manifest_ref: String,
    pub output_path_ref: String,
    pub promotion_summary_ref: String,
    pub member_refs: Vec<(String, String)>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseExportVerifyInput<'a> {
    pub manifest_value: Option<&'a IoValue>,
    pub member_refs: &'a [(String, String)],
    pub archive_diagnostics: &'a [String],
}
