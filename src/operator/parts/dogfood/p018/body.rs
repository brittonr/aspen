
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseWorkflowStateInput<'a> {
    pub required_stage: &'a str,
    pub dogfood_report_ref: Option<&'a str>,
    pub dogfood_report_decision: &'a str,
    pub release_gate_ref: Option<&'a str>,
    pub bundle_ref: Option<&'a str>,
    pub bundle_verify_ref: Option<&'a str>,
    pub bundle_verify_decision: &'a str,
    pub signed_member_refs: &'a [String],
    pub required_signed_member_refs: &'a [String],
    pub promotion_ref: Option<&'a str>,
    pub promotion_decision: &'a str,
    pub signed_promotion_ref: Option<&'a str>,
    pub signed_promotion_subject_ref: Option<&'a str>,
    pub summary_ref: Option<&'a str>,
    pub summary_decision: &'a str,
    pub summary_promotion_ref: Option<&'a str>,
    pub export_manifest_ref: Option<&'a str>,
    pub export_manifest_summary_ref: Option<&'a str>,
    pub export_verify_ref: Option<&'a str>,
    pub export_verify_decision: &'a str,
    pub export_verify_manifest_ref: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseWorkflowStateDecision {
    pub decision: String,
    pub completed_stages: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseEvidenceBoundaryInput<'a> {
    pub operation: &'a str,
    pub release_receipt_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub provenance_refs: &'a [String],
    pub source_gate_refs: &'a [String],
    pub retention_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub transport_refs: &'a [String],
    pub destructive_operation_refs: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseEvidenceBoundaryDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
}
