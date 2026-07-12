
struct ApplyOutcome {
    retention_receipt_ref: Option<String>,
    tombstone_ref: Option<String>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct GcExecutionGateInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub subsystem: &'a str,
    pub action: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub apply_ref: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcExecutionGate {
    pub execution_ref: String,
    pub decision: String,
    pub subsystem: String,
    pub action: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub apply_ref: Option<String>,
    pub plan_ref: Option<String>,
    pub recomputed_plan_ref: Option<String>,
    pub retention_receipt_ref: Option<String>,
    pub tombstone_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct GcAuditInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub execution_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcAudit {
    pub audit_ref: String,
    pub decision: String,
    pub subsystem: String,
    pub action: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub plan_ref: Option<String>,
    pub plan_decision: String,
    pub apply_ref: Option<String>,
    pub apply_decision: String,
    pub execution_ref: String,
    pub execution_decision: String,
    pub retention_receipt_ref: Option<String>,
    pub retention_receipt_decision: String,
    pub tombstone_ref: Option<String>,
    pub tombstone_status: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct RetentionGcLifecycleInput<'a> {
    pub plan: Option<&'a GcPlan>,
    pub apply: Option<&'a GcApply>,
    pub execution: Option<&'a GcExecutionGate>,
    pub audit: Option<&'a GcAudit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionGcLifecycleDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct CandidateExplainInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub object_ref: &'a str,
    pub object_kind: Option<&'a str>,
    pub retention_class: Option<&'a str>,
    pub action: Option<&'a str>,
    pub subsystem: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateExplain {
    pub explain_ref: String,
    pub object_ref: String,
    pub object_kind: Option<String>,
    pub retention_class: Option<String>,
    pub action: Option<String>,
    pub subsystem: Option<String>,
    pub pin_refs: Vec<String>,
    pub admission_refs: Vec<String>,
    pub remote_clearance_refs: Vec<String>,
    pub remote_clearance_import_refs: Vec<String>,
    pub gc_plan_refs: Vec<String>,
    pub gc_apply_refs: Vec<String>,
    pub gc_execution_refs: Vec<String>,
    pub gc_audit_refs: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub tombstone_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateBundleExportProfile {
    Internal,
    Public,
    Diagnostic,
}

impl CandidateBundleExportProfile {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "internal" => Ok(Self::Internal),
            "public" => Ok(Self::Public),
            "diagnostic" => Ok(Self::Diagnostic),
            _ => Err(MoltenError::invalid_harness(format!(
                "unsupported retention bundle export profile {value}; expected internal, public, or diagnostic"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Internal => "internal",
            Self::Public => "public",
            Self::Diagnostic => "diagnostic",
        }
    }

    fn loss_classification(self) -> &'static str {
        match self {
            Self::Internal => "local-full-fidelity",
            Self::Public => "deny-sensitive",
            Self::Diagnostic => "diagnostic-redacted-view",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CandidateBundleExportInput<'a, Root: ?Sized = Path> {
    pub root: &'a Root,
    pub explain_value: &'a IoValue,
    pub out: &'a Path,
    pub profile: CandidateBundleExportProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateBundle {
    pub bundle_ref: String,
    pub explain_ref: String,
    pub object_ref: String,
    pub object_kind: Option<String>,
    pub retention_class: Option<String>,
    pub action: Option<String>,
    pub subsystem: Option<String>,
    pub gc_plan_refs: Vec<String>,
    pub gc_apply_refs: Vec<String>,
    pub gc_execution_refs: Vec<String>,
    pub gc_audit_refs: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub tombstone_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateBundleProfile {
    pub profile_ref: String,
    pub decision: String,
    pub profile: String,
    pub loss_classification: String,
    pub bundle_ref: String,
    pub marker_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct CandidateBundleVerifyInput<'a, Root: ?Sized = Path> {
    pub bundle_dir: &'a Root,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateBundleVerify {
    pub verify_ref: String,
    pub decision: String,
    pub bundle_ref: String,
    pub explain_ref: String,
    pub object_ref: String,
    pub object_kind: Option<String>,
    pub retention_class: Option<String>,
    pub action: Option<String>,
    pub subsystem: Option<String>,
    pub artifact_refs: Vec<String>,
    pub file_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceAdmissionInput<'a> {
    pub kind: &'a str,
    pub decision: &'a str,
    pub requester_ref: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub bound_refs: &'a [String],
    pub retained_refs: &'a [String],
    pub remote_refs: &'a [String],
    pub is_reference_index_complete: bool,
    pub is_current: bool,
    pub revoked_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceAdmission {
    pub admission_ref: String,
    pub kind: String,
    pub decision: String,
    pub requester_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub action: String,
    pub bound_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub remote_refs: Vec<String>,
    pub is_reference_index_complete: bool,
    pub is_current: bool,
    pub revoked_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearanceInput<'a> {
    pub decision: &'a str,
    pub requester_ref: &'a str,
    pub peer_ref: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub remote_ref: &'a str,
    pub policy_ref: &'a str,
    pub authority_ref: &'a str,
    pub evidence_refs: &'a [String],
    pub retained_refs: &'a [String],
    pub is_current: bool,
    pub revoked_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearance {
    pub clearance_ref: String,
    pub decision: String,
    pub requester_ref: String,
    pub peer_ref: String,
    pub object_ref: String,
    pub object_kind: String,
    pub retention_class: String,
    pub action: String,
    pub remote_ref: String,
    pub policy_ref: String,
    pub authority_ref: String,
    pub evidence_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub is_current: bool,
    pub revoked_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteGcClearanceRequestInput<'a> {
    pub requester_ref: &'a str,
    pub peer_ref: &'a str,
    pub object_ref: &'a str,
    pub object_kind: &'a str,
    pub retention_class: &'a str,
    pub action: &'a str,
    pub remote_ref: &'a str,
    pub policy_ref: &'a str,
    pub authority_ref: &'a str,
    pub evidence_refs: &'a [String],
}
