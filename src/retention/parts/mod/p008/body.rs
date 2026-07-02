
struct AdmissionRefsResult {
    diagnostics: Vec<String>,
    admitted_refs: Vec<String>,
    remote_refs: Vec<String>,
}

struct RemoteClearanceRefsInput<'a> {
    root: &'a Path,
    refs: &'a [String],
    scope: &'a AdmissionScope<'a>,
    required_remote_refs: &'a [String],
    required_peer_refs: &'a [String],
    policy_refs: &'a [String],
    authority_refs: &'a [String],
}

struct RemoteClearanceRefsResult {
    diagnostics: Vec<String>,
    admitted_refs: Vec<String>,
    remote_refs: Vec<String>,
    peer_refs: Vec<String>,
}

pub struct GcPlanValueInput<'a> {
    decision: &'a str,
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    requester_ref: Option<&'a str>,
    index: &'a ReferenceIndex,
    evidence_value: &'a IoValue,
    gates: &'a [PlanGate],
    diagnostics: &'a [String],
}

struct ApplyValueInput<'a> {
    decision: &'a str,
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    requester_ref: Option<&'a str>,
    plan_ref: &'a str,
    recomputed_plan_ref: &'a str,
    retention_receipt_ref: Option<&'a str>,
    tombstone_ref: Option<&'a str>,
    admission_refs: &'a [String],
    diagnostics: &'a [String],
}

struct ExecutionGateValueInput<'a> {
    decision: &'a str,
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    apply_ref: Option<&'a str>,
    plan_ref: Option<&'a str>,
    recomputed_plan_ref: Option<&'a str>,
    retention_receipt_ref: Option<&'a str>,
    tombstone_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

struct AuditValueInput<'a> {
    decision: &'a str,
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    plan_ref: Option<&'a str>,
    plan_decision: &'a str,
    apply_ref: Option<&'a str>,
    apply_decision: &'a str,
    execution_ref: &'a str,
    execution_decision: &'a str,
    retention_receipt_ref: Option<&'a str>,
    retention_receipt_decision: &'a str,
    tombstone_ref: Option<&'a str>,
    tombstone_status: &'a str,
    diagnostics: &'a [String],
}

struct CandidateExplainValueInput<'a> {
    object_ref: &'a str,
    object_kind: Option<&'a str>,
    retention_class: Option<&'a str>,
    action: Option<&'a str>,
    subsystem: Option<&'a str>,
    pin_refs: &'a [String],
    admission_refs: &'a [String],
    remote_clearance_refs: &'a [String],
    remote_clearance_import_refs: &'a [String],
    gc_plan_refs: &'a [String],
    gc_apply_refs: &'a [String],
    gc_execution_refs: &'a [String],
    gc_audit_refs: &'a [String],
    retention_receipt_refs: &'a [String],
    tombstone_refs: &'a [String],
    diagnostics: &'a [String],
}

struct MatchRefs {
    pin_refs: Vec<String>,
    admission_refs: Vec<String>,
    remote_clearance_refs: Vec<String>,
    remote_clearance_import_refs: Vec<String>,
    gc_plan_refs: Vec<String>,
    gc_apply_refs: Vec<String>,
    gc_execution_refs: Vec<String>,
    gc_audit_refs: Vec<String>,
    retention_receipt_refs: Vec<String>,
    tombstone_refs: Vec<String>,
}

struct CandidateFilter<'a> {
    object_ref: &'a str,
    object_kind: Option<&'a str>,
    retention_class: Option<&'a str>,
    action: Option<&'a str>,
    subsystem: Option<&'a str>,
}

struct CandidateBundleValueInput<'a> {
    explain: &'a CandidateExplain,
    artifact_refs: &'a [String],
    diagnostics: &'a [String],
}

struct CandidateBundleProfileValueInput<'a> {
    profile: CandidateBundleExportProfile,
    decision: &'a str,
    bundle_ref: &'a str,
    marker_refs: &'a [String],
    diagnostics: &'a [String],
}

struct BundleArtifactGroupInput<'a> {
    root: &'a Path,
    bundle_dir: &'a Path,
    dir_name: &'a str,
    refs: &'a [String],
    read: fn(&Path, &str) -> Result<IoValue>,
}

struct GroupSpec<'a> {
    dir_name: &'static str,
    refs: &'a [String],
    read: fn(&Path, &str) -> Result<IoValue>,
}

struct CandidateBundleVerifyValueInput<'a> {
    bundle: &'a CandidateBundle,
    decision: &'a str,
    file_refs: &'a [String],
    diagnostics: &'a [String],
}

struct BundleVerifyGroupInput<'a> {
    bundle_dir: &'a Path,
    dir_name: &'a str,
    refs: &'a [String],
    parse: fn(&IoValue) -> Result<()>,
}

struct Group<'a> {
    dir_name: &'a str,
    refs: &'a [String],
    parse: fn(&IoValue) -> Result<()>,
}

struct BundleArtifactGroupScanInput<'a> {
    group_dir: &'a Path,
    dir_name: &'a str,
    expected_refs: &'a OrderedSet<String>,
}

struct AuditScope<'a> {
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
}

struct GcAuditScope<'a> {
    subsystem: &'a str,
    retention: AuditScope<'a>,
}

struct AuditFacts {
    apply_decision: String,
    plan_ref: Option<String>,
    plan_decision: String,
    retention_receipt_decision: String,
    tombstone_status: String,
    diagnostics: Vec<String>,
}

struct ApplyStatus {
    decision: String,
    plan_ref: Option<String>,
    diagnostics: Vec<String>,
}

struct PlanStatus {
    decision: String,
    diagnostics: Vec<String>,
}

struct ReceiptStatus {
    decision: String,
    diagnostics: Vec<String>,
}

struct TombstoneStatus {
    status: String,
    diagnostics: Vec<String>,
}

struct GateAdmissions {
    policy: AdmissionRefsResult,
    authority: AdmissionRefsResult,
    supporting: AdmissionRefsResult,
    reference_index: AdmissionRefsResult,
    remote_gc: AdmissionRefsResult,
}

struct GateInputs<'a> {
    input: &'a GcPlanInput<'a>,
    policy: AdmissionRefsResult,
    authority: AdmissionRefsResult,
    supporting: AdmissionRefsResult,
    reference_index: AdmissionRefsResult,
    remote_gc: AdmissionRefsResult,
    remote_clearance: RemoteClearanceRefsResult,
    has_delete_authority: bool,
    has_remote_gc_clearance: bool,
}

struct PlanGateBuildInput<'a> {
    name: &'a str,
    is_required: bool,
    required_refs: &'a [String],
    admitted_refs: &'a [String],
    diagnostics: Vec<String>,
}

struct LocalGateInput<'a> {
    input: &'a GcPlanInput<'a>,
    index: &'a ReferenceIndex,
    has_delete_authority: bool,
    has_remote_gc_clearance: bool,
}

struct MissingDiagnosticInput<'a> {
    diagnostics: &'a [String],
    is_missing: bool,
    missing_diagnostic: &'a str,
}

struct RemoteClearanceGateInput<'a> {
    diagnostics: &'a [String],
    has_missing_refs: bool,
    has_missing_peers: bool,
}

struct LiveControlRequestInput<'a> {
    target_ref: &'a str,
    payload_ref: Option<&'a str>,
    authority_refs: &'a [String],
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    evidence_refs: &'a [String],
}

fn remote_clearance_live_control_request_value(input: &LiveControlRequestInput<'_>) -> Result<(String, IoValue)> {
    let value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "gate",
        target_ref: Some(input.target_ref),
        payload_ref: input.payload_ref,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
    })?;
    let reference = crate::preserves_rail::canonical_hash(&value)?;
    Ok((reference, value))
}
