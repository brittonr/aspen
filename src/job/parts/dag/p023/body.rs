
const REMOTE_EXECUTION_CLOSURE_DESCRIPTOR_SCHEMA: &str = "molten.remote-execution.closure-descriptor.v1";
const REMOTE_EXECUTION_REQUEST_SCHEMA: &str = "molten.remote-execution.request.v1";
const REMOTE_EXECUTION_CLOSURE_PLAN_SCHEMA: &str = "molten.remote-execution.closure-plan.v1";
const REMOTE_EXECUTION_ADMISSION_RECEIPT_SCHEMA: &str = "molten.remote-execution.admission-receipt.v1";

const REMOTE_EXECUTION_CLOSURE_DESCRIPTOR_FIELDS: usize = 10;
const REMOTE_EXECUTION_REQUEST_FIELDS: usize = 15;
const REMOTE_EXECUTION_CLOSURE_PLAN_FIELDS: usize = 8;
const REMOTE_EXECUTION_ADMISSION_RECEIPT_FIELDS: usize = 12;
const MAX_REMOTE_EXECUTION_DIAGNOSTICS: usize = 128;

const _: () = assert!(MAX_REMOTE_EXECUTION_DIAGNOSTICS > 0);
const _: () = assert!(MAX_REMOTE_EXECUTION_DIAGNOSTICS <= MAX_JOB_CHECKS);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteExecutionClosureDescriptorInput {
    pub root_artifact_ref: String,
    pub dependency_refs: Vec<String>,
    pub closure_digest_ref: Option<String>,
    pub artifact_kind: String,
    pub size_bound_ref: String,
    pub effect_manifest_ref: String,
    pub handler_profile: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub replay_nonce_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteExecutionClosureDescriptor {
    pub descriptor_ref: String,
    pub root_artifact_ref: String,
    pub dependency_refs: Vec<String>,
    pub closure_digest_ref: Option<String>,
    pub artifact_kind: String,
    pub size_bound_ref: String,
    pub effect_manifest_ref: String,
    pub handler_profile: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub replay_nonce_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteExecutionRequestInput {
    pub execution_id: String,
    pub root_artifact_ref: String,
    pub closure_descriptor: IoValue,
    pub entrypoint_id: String,
    pub argument: IoValue,
    pub effect_manifest_ref: String,
    pub handler_profile: String,
    pub capability_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub reply_route_ref: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteExecutionRequest {
    pub request_ref: String,
    pub execution_id: String,
    pub root_artifact_ref: String,
    pub closure_descriptor: RemoteExecutionClosureDescriptor,
    pub entrypoint_id: String,
    pub argument: IoValue,
    pub effect_manifest_ref: String,
    pub handler_profile: String,
    pub capability_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub reply_route_ref: String,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteExecutionClosurePlanInput {
    pub closure_descriptor: IoValue,
    pub receiver_present_refs: Vec<String>,
    pub sender_payload_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteExecutionClosurePlan {
    pub plan_ref: String,
    pub root_artifact_ref: String,
    pub dependency_refs: Vec<String>,
    pub already_present_refs: Vec<String>,
    pub missing_refs: Vec<String>,
    pub selected_fetch_refs: Vec<String>,
    pub sender_extra_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteExecutionAdmissionInput {
    pub request: IoValue,
    pub closure_plan: IoValue,
    pub fetched_refs: Vec<String>,
    pub verified_artifact_refs: Vec<String>,
    pub admitted_capability_refs: Vec<String>,
    pub handler_profile_admission_ref: String,
    pub local_policy_refs: Vec<String>,
    pub provenance_receipt_refs: Vec<String>,
    pub source_gate_receipt_refs: Vec<String>,
    pub resource_receipt_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteExecutionAdmissionReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub request_ref: String,
    pub execution_id: String,
    pub root_artifact_ref: String,
    pub closure_plan_ref: String,
    pub fetched_refs: Vec<String>,
    pub verified_artifact_refs: Vec<String>,
    pub handler_profile_admission_ref: String,
    pub diagnostics: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

struct RemoteExecutionClosurePlanValueInput<'a> {
    root_artifact_ref: &'a str,
    dependency_refs: &'a [String],
    already_present_refs: &'a [String],
    missing_refs: &'a [String],
    selected_fetch_refs: &'a [String],
    sender_extra_refs: &'a [String],
    diagnostics: &'a [String],
}

struct RemoteExecutionAdmissionValueInput<'a> {
    decision: &'a str,
    request_ref: &'a str,
    execution_id: &'a str,
    root_artifact_ref: &'a str,
    closure_plan_ref: &'a str,
    fetched_refs: &'a [String],
    verified_artifact_refs: &'a [String],
    handler_profile_admission_ref: &'a str,
    diagnostics: &'a [String],
    evidence_refs: &'a [String],
}
