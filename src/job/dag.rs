use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use preserves::IOValue;
use preserves::Record;
use preserves::Value;

use crate::artifacts;
use crate::authority;
use crate::chunk_store;
use crate::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE;
use crate::error::MoltenError;
use crate::error::Result;
use crate::eval_cache;
use crate::ledger;
use crate::octet_gate;
use crate::preserves_rail::JOB_ADMISSION_PLAN_SCHEMA;
use crate::preserves_rail::JOB_ADMISSION_RECEIPT_SCHEMA;
use crate::preserves_rail::JOB_ADMISSION_REQUEST_SCHEMA;
use crate::preserves_rail::JOB_DAG_EDGE_SCHEMA;
use crate::preserves_rail::JOB_DAG_NODE_SCHEMA;
use crate::preserves_rail::JOB_DAG_OUTPUT_REQUEST_SCHEMA;
use crate::preserves_rail::JOB_DAG_RECEIPT_SCHEMA;
use crate::preserves_rail::JOB_DAG_SCHEMA;
use crate::preserves_rail::JOB_EXECUTION_RECEIPT_SCHEMA;
use crate::preserves_rail::JOB_EXECUTION_REQUEST_SCHEMA;
use crate::preserves_rail::JOB_FUSION_PLAN_SCHEMA;
use crate::preserves_rail::JOB_FUSION_RECEIPT_SCHEMA;
use crate::preserves_rail::JOB_PLAN_RECEIPT_SCHEMA;
use crate::preserves_rail::JOB_PLAN_SCHEMA;
use crate::preserves_rail::JOB_PROFILE_RECEIPT_SCHEMA;
use crate::preserves_rail::JOB_PROFILE_SCHEMA;
use crate::preserves_rail::JOB_REF_RECEIPT_SCHEMA;
use crate::preserves_rail::JOB_REF_STATUS_SCHEMA;
use crate::preserves_rail::JOB_REF_SUBMISSION_SCHEMA;
use crate::preserves_rail::JOB_STAGE_OPERATION_SCHEMA;
use crate::preserves_rail::JOB_SYNC_PLAN_SCHEMA;
use crate::preserves_rail::JOB_SYNC_RECEIPT_SCHEMA;
use crate::preserves_rail::JOB_SYNC_REQUEST_SCHEMA;
use crate::preserves_rail::JOB_WORKER_ASSIGNMENT_SCHEMA;
use crate::preserves_rail::JOB_WORKER_RECEIPT_SCHEMA;
use crate::preserves_rail::JOB_WORKER_REQUEST_SCHEMA;
use crate::preserves_rail::JOB_WORKER_RESULT_SCHEMA;
use crate::preserves_rail::JOB_WORKER_SCHEDULE_RECEIPT_SCHEMA;
use crate::preserves_rail::JOB_WORKER_STATUS_SCHEMA;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_text;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;
use crate::remote_dataspace;
use crate::resources;
use crate::typed_storage;

pub const JOB_ARTIFACT_KIND: &str = "job-dag";
pub const JOB_CACHE_OPERATION: &str = "job-stage";
pub const JOB_TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAX_JOB_NODES: usize = 256;
const MAX_JOB_EDGES: usize = 4_096;
const MAX_JOB_ROOTS: usize = MAX_JOB_NODES;
const MAX_JOB_REFS: usize = 4_096;
const MAX_JOB_PORTS: usize = 64;
const MAX_JOB_CHECKS: usize = 256;
const MAX_JOB_STAGE_VALUES: usize = 4_096;
const MAX_JOB_INLINE_BYTES: u64 = 4_096;

const _: () = assert!(MAX_JOB_NODES > 0);
const _: () = assert!(MAX_JOB_ROOTS <= MAX_JOB_NODES);
const _: () = assert!(MAX_JOB_EDGES >= MAX_JOB_NODES);
const _: () = assert!(MAX_JOB_STAGE_VALUES >= MAX_JOB_NODES);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobDag {
    pub job_ref: String,
    pub version: String,
    pub nodes: Vec<JobNode>,
    pub edges: Vec<JobEdge>,
    pub output_roots: Vec<String>,
    pub schema_refs: Vec<String>,
    pub effect_manifest_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobNode {
    pub id: String,
    pub kind: String,
    pub stage_artifact_ref: Option<String>,
    pub input_ports: Vec<String>,
    pub output_ports: Vec<String>,
    pub config: IOValue,
    pub effect_manifest_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobEdge {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
    pub schema_ref: Option<String>,
    pub partitioning: String,
    pub materialization: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobOutputRequest {
    pub request_ref: String,
    pub dag_ref: String,
    pub roots: Vec<String>,
    pub materialization: String,
    pub policy_refs: Vec<String>,
    pub handler_profile_ref: Option<String>,
    pub seed_config_ref: Option<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobInstall {
    pub job_ref: String,
    pub artifact_ref: String,
    pub decision: String,
    pub receipt_value: IOValue,
    pub artifact_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRunOptions<'a> {
    pub registry_root: &'a Path,
    pub storage_root: &'a Path,
    pub cache_root: &'a Path,
    pub chunk_root: &'a Path,
    pub ledger_root: Option<&'a Path>,
    pub output_request: Option<IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRun {
    pub job_ref: String,
    pub request_ref: String,
    pub stage_receipt_refs: Vec<String>,
    pub output_refs: Vec<String>,
    pub output_value: IOValue,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobStageRun {
    pub node_id: String,
    pub output_values: Vec<IOValue>,
    pub output_refs: Vec<String>,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrellisExecutionPlan {
    order_ids: Vec<String>,
    node_index: BTreeMap<String, usize>,
    dependency_indices: BTreeMap<String, Vec<u64>>,
}

struct PlanMapping {
    node_ids: Vec<String>,
    node_index: BTreeMap<String, usize>,
    edges: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub job_ref: Option<String>,
    pub request_ref: Option<String>,
    pub stage_id: Option<String>,
    pub input_refs: Vec<String>,
    pub output_refs: Vec<String>,
    pub cache_ref: Option<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobPlan {
    pub plan_ref: String,
    pub job_ref: String,
    pub request_ref: String,
    pub stage_order: Vec<String>,
    pub value: IOValue,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobProfile {
    pub profile_ref: String,
    pub job_ref: String,
    pub request_ref: String,
    pub stage_count: u64,
    pub edge_count: u64,
    pub materialization_boundaries: u64,
    pub value: IOValue,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobFusionPreview {
    pub fusion_ref: String,
    pub job_ref: String,
    pub request_ref: String,
    pub chains: Vec<Vec<String>>,
    pub value: IOValue,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSyncRequest {
    pub request_ref: String,
    pub job_ref: String,
    pub stage_ids: Vec<String>,
    pub target_peer: String,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSyncPlan {
    pub plan_ref: String,
    pub request: JobSyncRequest,
    pub root_refs: Vec<String>,
    pub closure_refs: Vec<String>,
    pub missing_refs: Vec<String>,
    pub value: IOValue,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct SyncLoopbackInput<'a> {
    pub source_registry: &'a Path,
    pub target_registry: &'a Path,
    pub request_value: &'a IOValue,
    pub provenance_values: &'a [IOValue],
    pub build_verification_values: &'a [IOValue],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSyncLoopback {
    pub receipt_ref: String,
    pub plan: JobSyncPlan,
    pub decision: String,
    pub installed_refs: Vec<String>,
    pub already_present_refs: Vec<String>,
    pub provenance_receipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobAdmissionRequest {
    pub request_ref: String,
    pub job_ref: String,
    pub sync_ref: String,
    pub stage_ids: Vec<String>,
    pub target_peer: String,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobAdmissionStageVerdict {
    pub stage_id: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobAdmissionPlan {
    pub plan_ref: String,
    pub request: JobAdmissionRequest,
    pub closure_refs: Vec<String>,
    pub stage_order: Vec<String>,
    pub stage_verdicts: Vec<JobAdmissionStageVerdict>,
    pub authority_receipt_refs: Vec<String>,
    pub source_gate_validation_refs: Vec<String>,
    pub resource_verdict: String,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobAdmissionLoopback {
    pub receipt_ref: String,
    pub plan: JobAdmissionPlan,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobAdmissionReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub job_ref: String,
    pub request_ref: String,
    pub plan_ref: String,
    pub sync_ref: String,
    pub target_peer: String,
    pub closure_refs: Vec<String>,
    pub stage_order: Vec<String>,
    pub authority_receipt_refs: Vec<String>,
    pub source_gate_validation_refs: Vec<String>,
    pub resource_verdict: String,
    pub diagnostics: Vec<String>,
    pub refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobExecutionRequest {
    pub request_ref: String,
    pub job_ref: String,
    pub admission_ref: String,
    pub target_peer: String,
    pub stage_ids: Vec<String>,
    pub storage_profile_ref: String,
    pub cache_profile_ref: String,
    pub chunk_profile_ref: String,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobExecutionLoopback {
    pub receipt_ref: String,
    pub request: JobExecutionRequest,
    pub admission: JobAdmissionReceipt,
    pub run: Option<JobRun>,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobContentRef {
    pub content_ref: String,
    pub size: u64,
    pub format: String,
    pub schema_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobRefJobSubmission {
    pub submission_ref: String,
    pub job_id: String,
    pub operation_id: String,
    pub executable: JobContentRef,
    pub inputs: Vec<JobContentRef>,
    pub output_mode: String,
    pub input_schema_refs: Vec<String>,
    pub output_schema_refs: Vec<String>,
    pub effect_manifest_refs: Vec<String>,
    pub handler_profile: String,
    pub authority_context_ref: String,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobRefJobExecution {
    pub submission: BlobRefJobSubmission,
    pub decision: String,
    pub status_values: Vec<IOValue>,
    pub output_manifest_ref: Option<String>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BlobRefJobSubmissionValueInput<'a> {
    pub job_id: &'a str,
    pub operation_id: &'a str,
    pub executable: JobContentRef,
    pub inputs: Vec<JobContentRef>,
    pub output_mode: &'a str,
    pub input_schema_refs: &'a [String],
    pub output_schema_refs: &'a [String],
    pub effect_manifest_refs: &'a [String],
    pub handler_profile: &'a str,
    pub authority_context_ref: &'a str,
    pub policy_refs: &'a [String],
    pub provenance_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct BlobRefJobExecuteInput<'a> {
    pub chunk_root: &'a Path,
    pub submission_value: &'a IOValue,
    pub ledger_root: Option<&'a Path>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobWorkerRequest {
    pub request_ref: String,
    pub job_ref: String,
    pub target_peer: String,
    pub stage_ids: Vec<String>,
    pub sync_ref: String,
    pub admission_ref: String,
    pub execution_request_ref: String,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub peer_bootstrap_refs: Vec<String>,
    pub node_identity_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobWorkerResult {
    pub result_ref: String,
    pub decision: String,
    pub job_ref: String,
    pub target_peer: String,
    pub execution_receipt_ref: Option<String>,
    pub output_refs: Vec<String>,
    pub stage_receipt_refs: Vec<(String, String)>,
    pub resource_receipt_refs: Vec<String>,
    pub delivery_log_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobWorkerReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub job_ref: Option<String>,
    pub request_ref: Option<String>,
    pub assignment_ref: String,
    pub status_refs: Vec<String>,
    pub result_ref: String,
    pub execution_receipt_ref: Option<String>,
    pub delivery_log_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobWorkerScheduleReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub job_ref: String,
    pub request_ref: String,
    pub queue_key: String,
    pub lease_key: String,
    pub worker_session: String,
    pub coordination_report_ref: String,
    pub token_ref: Option<String>,
    pub worker_receipt_ref: Option<String>,
    pub result_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobWorkerExecution {
    pub request: Option<JobWorkerRequest>,
    pub assignment_value: IOValue,
    pub status_values: Vec<IOValue>,
    pub result: JobWorkerResult,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub execution: Option<JobExecutionLoopback>,
}

#[derive(Debug, Clone, Copy)]
pub struct JobWorkerRequestValueInput<'a> {
    pub job_ref: &'a str,
    pub target_peer: &'a str,
    pub stage_ids: &'a [String],
    pub sync_ref: &'a str,
    pub admission_ref: &'a str,
    pub execution_request_ref: &'a str,
    pub authority_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub peer_bootstrap_refs: &'a [String],
    pub node_identity_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct JobWorkerScheduleReceiptValueInput<'a> {
    pub operation: &'a str,
    pub decision: &'a str,
    pub job_ref: &'a str,
    pub request_ref: &'a str,
    pub queue_key: &'a str,
    pub lease_key: &'a str,
    pub worker_session: &'a str,
    pub coordination_report_ref: &'a str,
    pub enqueue_receipt_ref: Option<&'a str>,
    pub enqueue_duplicate_receipt_ref: Option<&'a str>,
    pub dequeue_receipt_ref: Option<&'a str>,
    pub lease_receipt_ref: Option<&'a str>,
    pub release_receipt_ref: Option<&'a str>,
    pub token_ref: Option<&'a str>,
    pub worker_receipt_ref: Option<&'a str>,
    pub result_ref: Option<&'a str>,
    pub diagnostics: &'a [String],
    pub refs: &'a [String],
    pub checks: &'a [(&'a str, &'a str)],
}

#[derive(Debug, Clone, Copy)]
pub struct JobWorkerEnvelopeInput<'a> {
    pub from_peer: &'a str,
    pub from_actor: &'a str,
    pub to_peer: &'a str,
    pub topic: &'a str,
    pub request_value: &'a IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct JobWorkerExecuteInput<'a> {
    pub target_registry: &'a Path,
    pub storage_root: &'a Path,
    pub cache_root: &'a Path,
    pub chunk_root: &'a Path,
    pub delivery: &'a remote_dataspace::RemoteDataspaceDelivery,
    pub delivery_log: Option<&'a remote_dataspace::RemoteDeliveryLog>,
    pub admission_receipt_value: &'a IOValue,
    pub execution_request_value: &'a IOValue,
    pub ledger_root: Option<&'a Path>,
}

#[derive(Debug, Clone)]
pub struct NodeValueInput<'a> {
    pub id: &'a str,
    pub kind: &'a str,
    pub stage_artifact_ref: Option<&'a str>,
    pub input_ports: &'a [String],
    pub output_ports: &'a [String],
    pub config: IOValue,
    pub effect_manifest_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct EdgeValueInput<'a> {
    pub from_node: &'a str,
    pub from_port: &'a str,
    pub to_node: &'a str,
    pub to_port: &'a str,
    pub schema_ref: Option<&'a str>,
    pub partitioning: &'a str,
    pub materialization: &'a str,
}

#[derive(Debug, Clone)]
pub struct DagValueInput<'a> {
    pub nodes: Vec<IOValue>,
    pub edges: Vec<IOValue>,
    pub output_roots: &'a [String],
    pub schema_refs: &'a [String],
    pub effect_manifest_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct OutputRequestValueInput<'a> {
    pub dag_ref: &'a str,
    pub roots: &'a [String],
    pub materialization: &'a str,
    pub policy_refs: &'a [String],
    pub handler_profile_ref: Option<&'a str>,
    pub seed_config_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct SyncRequestValueInput<'a> {
    pub job_ref: &'a str,
    pub stage_ids: &'a [String],
    pub target_peer: &'a str,
    pub policy_refs: &'a [String],
    pub capability_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct AdmissionRequestValueInput<'a> {
    pub job_ref: &'a str,
    pub sync_ref: &'a str,
    pub stage_ids: &'a [String],
    pub target_peer: &'a str,
    pub policy_refs: &'a [String],
    pub capability_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub resource_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ExecutionRequestValueInput<'a> {
    pub job_ref: &'a str,
    pub admission_ref: &'a str,
    pub stage_ids: &'a [String],
    pub target_peer: &'a str,
    pub storage_profile_ref: &'a str,
    pub cache_profile_ref: &'a str,
    pub chunk_profile_ref: &'a str,
    pub policy_refs: &'a [String],
    pub capability_refs: &'a [String],
    pub resource_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ExecutionLoopbackInput<'a> {
    pub target_registry: &'a Path,
    pub storage_root: &'a Path,
    pub cache_root: &'a Path,
    pub chunk_root: &'a Path,
    pub admission_receipt_value: &'a IOValue,
    pub request_value: &'a IOValue,
}

struct ExecutionReceiptValueInput<'a> {
    decision: &'a str,
    request: &'a JobExecutionRequest,
    admission: &'a JobAdmissionReceipt,
    stage_receipt_refs: &'a [String],
    output_refs: &'a [String],
    run_receipt_refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct BlobRefReceiptValueInput<'a> {
    decision: &'a str,
    submission: &'a BlobRefJobSubmission,
    status_refs: &'a [String],
    verify_refs: &'a [String],
    fetch_refs: &'a [String],
    pin_refs: &'a [String],
    cleanup_refs: &'a [String],
    output_manifest_ref: Option<&'a str>,
    output_put_ref: Option<&'a str>,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

#[derive(Debug, Clone, Copy)]
struct Preflight {
    has_policy: bool,
    has_provenance: bool,
    has_effect_manifest: bool,
    has_supported_output_mode: bool,
    has_supported_handler: bool,
}

struct FetchOutcome {
    content_refs: Vec<JobContentRef>,
    input_bytes: Vec<Vec<u8>>,
    verify_refs: Vec<String>,
    fetch_refs: Vec<String>,
    pin_refs: Vec<String>,
    diagnostics: Vec<String>,
    is_content_verified: bool,
}

struct OutputOutcome {
    output_manifest_ref: String,
    output_put_ref: String,
    verify_ref: String,
    pin_ref: String,
    status_values: Vec<IOValue>,
}

struct FinishInput<'a> {
    ledger_root: Option<&'a Path>,
    submission: BlobRefJobSubmission,
    status_values: Vec<IOValue>,
    verify_refs: Vec<String>,
    fetch_refs: Vec<String>,
    pin_refs: Vec<String>,
    cleanup_refs: Vec<String>,
    output_manifest_ref: Option<String>,
    output_put_ref: Option<String>,
    diagnostics: Vec<String>,
    preflight: Preflight,
    is_content_verified: bool,
    has_preliminary_pass: bool,
}

struct WorkerResultValueInput<'a> {
    decision: &'a str,
    request: &'a JobWorkerRequest,
    execution_receipt_ref: Option<&'a str>,
    output_refs: &'a [String],
    stage_receipt_refs: &'a [(String, String)],
    resource_receipt_refs: &'a [String],
    delivery_log_ref: Option<&'a str>,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct WorkerStatusValueInput<'a> {
    request: &'a JobWorkerRequest,
    delivery: &'a remote_dataspace::RemoteDataspaceDelivery,
    state: &'a str,
    execution_receipt_ref: Option<&'a str>,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct WorkerReceiptValueInput<'a> {
    decision: &'a str,
    request: Option<&'a JobWorkerRequest>,
    assignment_ref: &'a str,
    status_refs: &'a [String],
    result_ref: &'a str,
    execution_receipt_ref: Option<&'a str>,
    delivery_log_ref: Option<&'a str>,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct AnalysisReceiptValueInput<'a> {
    label: &'static str,
    schema: &'static str,
    operation: &'static str,
    job_ref: &'a str,
    request_ref: &'a str,
    artifact_ref: &'a str,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

pub fn job_node_value(input: NodeValueInput<'_>) -> Result<IOValue> {
    validate_node_id(input.id)?;
    validate_stage_kind(input.kind)?;
    if let Some(stage_artifact_ref) = input.stage_artifact_ref {
        validate_ref(stage_artifact_ref, "job stage artifact ref")?;
    }
    validate_refs(input.effect_manifest_refs, "job node effect manifest ref")?;
    validate_refs(input.policy_refs, "job node policy ref")?;
    validate_refs(input.evidence_refs, "job node evidence ref")?;
    reject_mobile_closure_config(&input.config)?;
    Ok(record("job-node-v1", vec![
        string(JOB_DAG_NODE_SCHEMA),
        record("id", vec![string(input.id)]),
        record("kind", vec![string(input.kind)]),
        record("stage-artifact", vec![optional_ref_value(input.stage_artifact_ref)]),
        record("inputs", vec![ports_sequence(input.input_ports)]),
        record("outputs", vec![ports_sequence(input.output_ports)]),
        record("config", vec![input.config]),
        record("effects", vec![refs_sequence(&sorted_unique(input.effect_manifest_refs))]),
        record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        checks_value(&[
            "stage-artifact-not-closure",
            "bounded-stage-kind",
            "explicit-effect-boundary",
        ]),
    ]))
}

pub fn job_edge_value(input: EdgeValueInput<'_>) -> Result<IOValue> {
    validate_node_id(input.from_node)?;
    validate_node_id(input.to_node)?;
    validate_non_empty(input.from_port, "job edge from port")?;
    validate_non_empty(input.to_port, "job edge to port")?;
    if let Some(schema_ref) = input.schema_ref {
        validate_ref(schema_ref, "job edge schema ref")?;
    }
    validate_partitioning(input.partitioning)?;
    validate_materialization(input.materialization)?;
    Ok(record("job-edge-v1", vec![
        string(JOB_DAG_EDGE_SCHEMA),
        record("from", vec![string(input.from_node), string(input.from_port)]),
        record("to", vec![string(input.to_node), string(input.to_port)]),
        record("schema", vec![optional_ref_value(input.schema_ref)]),
        record("partitioning", vec![string(input.partitioning)]),
        record("materialization", vec![string(input.materialization)]),
        checks_value(&["schema-bound", "canonical-edge", "explicit-materialization"]),
    ]))
}

pub fn job_dag_value(input: DagValueInput<'_>) -> Result<IOValue> {
    validate_refs(input.schema_refs, "job schema ref")?;
    validate_refs(input.effect_manifest_refs, "job effect manifest ref")?;
    validate_refs(input.policy_refs, "job policy ref")?;
    validate_refs(input.evidence_refs, "job evidence ref")?;
    Ok(record("job-dag-v1", vec![
        string(JOB_DAG_SCHEMA),
        record("version", vec![string("v1")]),
        record("nodes", vec![sequence(input.nodes)]),
        record("edges", vec![sequence(input.edges)]),
        record("outputs", vec![sequence(input.output_roots.iter().map(string).collect())]),
        record("schemas", vec![refs_sequence(&sorted_unique(input.schema_refs))]),
        record("effect-manifests", vec![refs_sequence(&sorted_unique(input.effect_manifest_refs))]),
        record("policies", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        checks_value(&[
            "canonical-dag",
            "no-name-identity",
            "stage-artifacts-explicit",
            "deterministic-local-profile",
        ]),
    ]))
}

pub fn job_output_request_value(input: OutputRequestValueInput<'_>) -> Result<IOValue> {
    validate_ref(input.dag_ref, "job output request dag ref")?;
    for root in input.roots {
        validate_node_id(root)?;
    }
    validate_request_materialization(input.materialization)?;
    validate_refs(input.policy_refs, "job output request policy ref")?;
    if let Some(handler_profile_ref) = input.handler_profile_ref {
        validate_ref(handler_profile_ref, "job output request handler profile ref")?;
    }
    if let Some(seed_config_ref) = input.seed_config_ref {
        validate_ref(seed_config_ref, "job output request seed config ref")?;
    }
    Ok(record("job-output-request-v1", vec![
        string(JOB_DAG_OUTPUT_REQUEST_SCHEMA),
        record("dag", vec![string(input.dag_ref)]),
        record("roots", vec![sequence(input.roots.iter().map(string).collect())]),
        record("materialization", vec![string(input.materialization)]),
        record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        record("handler-profile", vec![optional_ref_value(input.handler_profile_ref)]),
        record("seed-config", vec![optional_ref_value(input.seed_config_ref)]),
        checks_value(&["request-ref-bound", "full-ref-identity", "deterministic-inputs-bound"]),
    ]))
}

pub fn builtin_stage_operation_value(operation: &str) -> Result<IOValue> {
    validate_stage_operation(operation)?;
    Ok(record("job-stage-operation-v1", vec![
        string(JOB_STAGE_OPERATION_SCHEMA),
        record("operation", vec![string(operation)]),
        checks_value(&["bounded-built-in", "no-mobile-closure", "canonical-operation"]),
    ]))
}

pub fn builtin_stage_operation_ref(operation: &str) -> Result<String> {
    canonical_hash(&builtin_stage_operation_value(operation)?)
}

pub fn job_sync_request_value(input: SyncRequestValueInput<'_>) -> Result<IOValue> {
    validate_ref(input.job_ref, "job sync request job ref")?;
    for stage_id in input.stage_ids {
        validate_node_id(stage_id)?;
    }
    validate_non_empty(input.target_peer, "job sync target peer")?;
    validate_refs(input.policy_refs, "job sync policy ref")?;
    validate_refs(input.capability_refs, "job sync capability ref")?;
    validate_refs(input.evidence_refs, "job sync evidence ref")?;
    Ok(record("job-sync-request-v1", vec![
        string(JOB_SYNC_REQUEST_SCHEMA),
        record("job", vec![string(input.job_ref)]),
        record("stages", vec![sequence(input.stage_ids.iter().map(string).collect())]),
        record("target-peer", vec![string(input.target_peer)]),
        record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        record("capability", vec![refs_sequence(&sorted_unique(input.capability_refs))]),
        record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        checks_value(&["transport-neutral", "no-execution", "full-ref-identity"]),
    ]))
}

pub fn parse_job_sync_request_value(value: &IOValue) -> Result<JobSyncRequest> {
    let fields = value
        .collect_simple_record("job-sync-request-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-sync-request-v1 ...>"))?;
    require_schema(&fields[0], JOB_SYNC_REQUEST_SCHEMA, "job sync request")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "no-execution", "job sync request")?;
    Ok(JobSyncRequest {
        request_ref: canonical_hash(value)?,
        job_ref: record_ref(&fields[1], "job")?,
        stage_ids: record_node_id_sequence(&fields[2], "stages")?,
        target_peer: record_string(&fields[3], "target-peer")?,
        policy_refs: record_ref_sequence(&fields[4], "policy")?,
        capability_refs: record_ref_sequence(&fields[5], "capability")?,
        evidence_refs: record_ref_sequence(&fields[6], "evidence")?,
        value: value.clone(),
    })
}

pub fn job_admission_request_value(input: AdmissionRequestValueInput<'_>) -> Result<IOValue> {
    validate_ref(input.job_ref, "job admission request job ref")?;
    validate_ref(input.sync_ref, "job admission request sync ref")?;
    for stage_id in input.stage_ids {
        validate_node_id(stage_id)?;
    }
    validate_non_empty(input.target_peer, "job admission target peer")?;
    validate_refs(input.policy_refs, "job admission policy ref")?;
    validate_refs(input.capability_refs, "job admission capability ref")?;
    validate_refs(input.evidence_refs, "job admission evidence ref")?;
    validate_refs(input.resource_refs, "job admission resource ref")?;
    Ok(record("job-admission-request-v1", vec![
        string(JOB_ADMISSION_REQUEST_SCHEMA),
        record("job", vec![string(input.job_ref)]),
        record("sync", vec![string(input.sync_ref)]),
        record("stages", vec![sequence(input.stage_ids.iter().map(string).collect())]),
        record("target-peer", vec![string(input.target_peer)]),
        record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        record("capability", vec![refs_sequence(&sorted_unique(input.capability_refs))]),
        record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        record("resource", vec![refs_sequence(&sorted_unique(input.resource_refs))]),
        checks_value(&["target-side-admission", "no-execution", "full-ref-identity"]),
    ]))
}

pub fn parse_job_admission_request_value(value: &IOValue) -> Result<JobAdmissionRequest> {
    let fields = value
        .collect_simple_record("job-admission-request-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-admission-request-v1 ...>"))?;
    require_schema(&fields[0], JOB_ADMISSION_REQUEST_SCHEMA, "job admission request")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "no-execution", "job admission request")?;
    Ok(JobAdmissionRequest {
        request_ref: canonical_hash(value)?,
        job_ref: record_ref(&fields[1], "job")?,
        sync_ref: record_ref(&fields[2], "sync")?,
        stage_ids: record_node_id_sequence(&fields[3], "stages")?,
        target_peer: record_string(&fields[4], "target-peer")?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        capability_refs: record_ref_sequence(&fields[6], "capability")?,
        evidence_refs: record_ref_sequence(&fields[7], "evidence")?,
        resource_refs: record_ref_sequence(&fields[8], "resource")?,
        value: value.clone(),
    })
}

pub fn job_execution_request_value(input: ExecutionRequestValueInput<'_>) -> Result<IOValue> {
    validate_ref(input.job_ref, "job execution request job ref")?;
    validate_ref(input.admission_ref, "job execution admission receipt ref")?;
    for stage_id in input.stage_ids {
        validate_node_id(stage_id)?;
    }
    validate_non_empty(input.target_peer, "job execution target peer")?;
    validate_ref(input.storage_profile_ref, "job execution storage profile ref")?;
    validate_ref(input.cache_profile_ref, "job execution cache profile ref")?;
    validate_ref(input.chunk_profile_ref, "job execution chunk profile ref")?;
    validate_refs(input.policy_refs, "job execution policy ref")?;
    validate_refs(input.capability_refs, "job execution capability ref")?;
    validate_refs(input.resource_refs, "job execution resource ref")?;
    Ok(record("job-execution-request-v1", vec![
        string(JOB_EXECUTION_REQUEST_SCHEMA),
        record("job", vec![string(input.job_ref)]),
        record("admission", vec![string(input.admission_ref)]),
        record("target-peer", vec![string(input.target_peer)]),
        record("stages", vec![sequence(input.stage_ids.iter().map(string).collect())]),
        record("storage", vec![string(input.storage_profile_ref)]),
        record("cache", vec![string(input.cache_profile_ref)]),
        record("chunks", vec![string(input.chunk_profile_ref)]),
        record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        record("capability", vec![refs_sequence(&sorted_unique(input.capability_refs))]),
        record("resource", vec![refs_sequence(&sorted_unique(input.resource_refs))]),
        checks_value(&[
            "admission-required",
            "target-state-only",
            "no-source-registry",
            "full-ref-identity",
        ]),
    ]))
}

pub fn parse_job_execution_request_value(value: &IOValue) -> Result<JobExecutionRequest> {
    let fields = value
        .collect_simple_record("job-execution-request-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-execution-request-v1 ...>"))?;
    require_schema(&fields[0], JOB_EXECUTION_REQUEST_SCHEMA, "job execution request")?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "admission-required", "job execution request")?;
    require_check(&checks, "target-state-only", "job execution request")?;
    require_check(&checks, "no-source-registry", "job execution request")?;
    Ok(JobExecutionRequest {
        request_ref: canonical_hash(value)?,
        job_ref: record_ref(&fields[1], "job")?,
        admission_ref: record_ref(&fields[2], "admission")?,
        target_peer: record_string(&fields[3], "target-peer")?,
        stage_ids: record_node_id_sequence(&fields[4], "stages")?,
        storage_profile_ref: record_ref(&fields[5], "storage")?,
        cache_profile_ref: record_ref(&fields[6], "cache")?,
        chunk_profile_ref: record_ref(&fields[7], "chunks")?,
        policy_refs: record_ref_sequence(&fields[8], "policy")?,
        capability_refs: record_ref_sequence(&fields[9], "capability")?,
        resource_refs: record_ref_sequence(&fields[10], "resource")?,
        value: value.clone(),
    })
}

pub fn job_content_ref_value(content: &JobContentRef) -> Result<IOValue> {
    validate_job_content_ref(content, "job content ref")?;
    Ok(record("job-content-ref", vec![
        record("content-ref", vec![string(&content.content_ref)]),
        record("size", vec![u64_value(content.size)]),
        record("format", vec![string(&content.format)]),
        record("schema", vec![optional_ref_value(content.schema_ref.as_deref())]),
    ]))
}

pub fn job_ref_submission_value(input: BlobRefJobSubmissionValueInput<'_>) -> Result<IOValue> {
    validate_blob_ref_submission_input(&input)?;
    let input_values = input.inputs.iter().map(job_content_ref_value).collect::<Result<Vec<_>>>()?;
    Ok(record("job-ref-submission-v1", vec![
        string(JOB_REF_SUBMISSION_SCHEMA),
        record("job-id", vec![string(input.job_id)]),
        record("operation-id", vec![string(input.operation_id)]),
        record("executable", vec![job_content_ref_value(&input.executable)?]),
        record("inputs", vec![sequence(input_values)]),
        record("output-mode", vec![string(input.output_mode)]),
        record("input-schemas", vec![refs_sequence(&sorted_unique(input.input_schema_refs))]),
        record("output-schemas", vec![refs_sequence(&sorted_unique(input.output_schema_refs))]),
        record("effects", vec![refs_sequence(&sorted_unique(input.effect_manifest_refs))]),
        record("handler-profile", vec![string(input.handler_profile)]),
        record("authority", vec![string(input.authority_context_ref)]),
        record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        record("provenance", vec![refs_sequence(&sorted_unique(input.provenance_refs))]),
        record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        checks_value(&[
            "content-refs-only",
            "no-inline-large-bytes",
            "handler-profile-declared",
            "authority-context-declared",
            "full-ref-identity",
        ]),
    ]))
}

pub fn parse_job_ref_submission_value(value: &IOValue) -> Result<BlobRefJobSubmission> {
    reject_blob_ref_job_inline_tokens(value)?;
    let fields = value
        .collect_simple_record("job-ref-submission-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-ref-submission-v1 ...>"))?;
    require_schema(&fields[0], JOB_REF_SUBMISSION_SCHEMA, "job ref submission")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "content-refs-only", "job ref submission")?;
    require_check(&checks, "no-inline-large-bytes", "job ref submission")?;
    let executable = parse_job_content_ref_record(&fields[3], "executable")?;
    let input_values = record_sequence_values(&fields[4], "inputs")?;
    let mut inputs = Vec::with_capacity(input_values.len());
    for input_value in &input_values {
        push_bounded(
            &mut inputs,
            parse_job_content_ref_value(input_value)?,
            MAX_JOB_REFS,
            "job ref submission inputs",
        )?;
    }
    let submission = BlobRefJobSubmission {
        submission_ref: canonical_hash(value)?,
        job_id: record_string(&fields[1], "job-id")?,
        operation_id: record_ref(&fields[2], "operation-id")?,
        executable,
        inputs,
        output_mode: record_string(&fields[5], "output-mode")?,
        input_schema_refs: record_ref_sequence(&fields[6], "input-schemas")?,
        output_schema_refs: record_ref_sequence(&fields[7], "output-schemas")?,
        effect_manifest_refs: record_ref_sequence(&fields[8], "effects")?,
        handler_profile: record_string(&fields[9], "handler-profile")?,
        authority_context_ref: record_ref(&fields[10], "authority")?,
        policy_refs: record_ref_sequence(&fields[11], "policy")?,
        provenance_refs: record_ref_sequence(&fields[12], "provenance")?,
        evidence_refs: record_ref_sequence(&fields[13], "evidence")?,
        value: value.clone(),
    };
    validate_blob_ref_submission(&submission)?;
    Ok(submission)
}

pub fn execute_blob_ref_job(input: BlobRefJobExecuteInput<'_>) -> Result<BlobRefJobExecution> {
    let submission = parse_job_ref_submission_value(input.submission_value)?;
    let (preflight, mut diagnostics) = preflight(&submission)?;
    let mut status_values = vec![blob_ref_job_status_value(&submission, "queued", &[], &[(
        "submission-valid",
        "pass",
    )])?];
    push_bounded(
        &mut status_values,
        blob_ref_job_status_value(&submission, "fetching", &[], &[("content-fetch-started", "pass")])?,
        MAX_JOB_REFS,
        "job ref status values",
    )?;
    let FetchOutcome {
        content_refs,
        input_bytes,
        mut verify_refs,
        fetch_refs,
        mut pin_refs,
        diagnostics: fetch_diagnostics,
        is_content_verified,
    } = fetch_content(input.chunk_root, &submission)?;
    for diagnostic in fetch_diagnostics {
        push_bounded(&mut diagnostics, diagnostic, MAX_JOB_REFS, "job ref diagnostics")?;
    }
    let has_preliminary_pass = preflight.has_policy
        && preflight.has_provenance
        && preflight.has_effect_manifest
        && preflight.has_supported_output_mode
        && preflight.has_supported_handler
        && is_content_verified;
    let (output_manifest_ref, output_put_ref) = if has_preliminary_pass {
        let output = run_output(input.chunk_root, &submission, &input_bytes)?;
        push_bounded(&mut verify_refs, output.verify_ref, MAX_JOB_REFS, "job ref verify refs")?;
        push_bounded(&mut pin_refs, output.pin_ref, MAX_JOB_REFS, "job ref pin refs")?;
        for status_value in output.status_values {
            push_bounded(&mut status_values, status_value, MAX_JOB_REFS, "job ref status values")?;
        }
        (Some(output.output_manifest_ref), Some(output.output_put_ref))
    } else {
        (None, None)
    };
    let cleanup_refs = cleanup_content(input.chunk_root, &content_refs)?;
    finish_run(FinishInput {
        ledger_root: input.ledger_root,
        submission,
        status_values,
        verify_refs,
        fetch_refs,
        pin_refs,
        cleanup_refs,
        output_manifest_ref,
        output_put_ref,
        diagnostics,
        preflight,
        is_content_verified,
        has_preliminary_pass,
    })
}

pub fn job_worker_request_value(input: JobWorkerRequestValueInput<'_>) -> Result<IOValue> {
    validate_ref(input.job_ref, "job worker request job ref")?;
    validate_non_empty(input.target_peer, "job worker target peer")?;
    for stage_id in input.stage_ids {
        validate_node_id(stage_id)?;
    }
    validate_ref(input.sync_ref, "job worker sync receipt ref")?;
    validate_ref(input.admission_ref, "job worker admission receipt ref")?;
    validate_ref(input.execution_request_ref, "job worker execution request ref")?;
    validate_refs(input.authority_refs, "job worker authority ref")?;
    validate_refs(input.resource_refs, "job worker resource ref")?;
    validate_refs(input.peer_bootstrap_refs, "job worker peer bootstrap ref")?;
    validate_refs(input.node_identity_refs, "job worker node identity ref")?;
    validate_refs(input.evidence_refs, "job worker evidence ref")?;
    Ok(record("job-worker-request-v1", vec![
        string(JOB_WORKER_REQUEST_SCHEMA),
        record("job", vec![string(input.job_ref)]),
        record("target-peer", vec![string(input.target_peer)]),
        record("stages", vec![sequence(input.stage_ids.iter().map(string).collect())]),
        record("sync", vec![string(input.sync_ref)]),
        record("admission", vec![string(input.admission_ref)]),
        record("execution-request", vec![string(input.execution_request_ref)]),
        record("authority", vec![refs_sequence(&sorted_unique(input.authority_refs))]),
        record("resource", vec![refs_sequence(&sorted_unique(input.resource_refs))]),
        record("peer-bootstrap", vec![refs_sequence(&sorted_unique(input.peer_bootstrap_refs))]),
        record("node-identity", vec![refs_sequence(&sorted_unique(input.node_identity_refs))]),
        record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        checks_value(&[
            "target-admission-required",
            "loopback-execution-required",
            "remote-dataspace-carrier",
            "transport-is-not-authority",
            "target-state-only",
            "no-mobile-closures",
            "full-ref-identity",
        ]),
    ]))
}

pub fn parse_job_worker_request_value(value: &IOValue) -> Result<JobWorkerRequest> {
    reject_worker_ambient_tokens(value)?;
    let fields = value
        .collect_simple_record("job-worker-request-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-worker-request-v1 ...>"))?;
    require_schema(&fields[0], JOB_WORKER_REQUEST_SCHEMA, "job worker request")?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "target-admission-required", "job worker request")?;
    require_check(&checks, "loopback-execution-required", "job worker request")?;
    require_check(&checks, "remote-dataspace-carrier", "job worker request")?;
    require_check(&checks, "transport-is-not-authority", "job worker request")?;
    require_check(&checks, "target-state-only", "job worker request")?;
    require_check(&checks, "no-mobile-closures", "job worker request")?;
    Ok(JobWorkerRequest {
        request_ref: canonical_hash(value)?,
        job_ref: record_ref(&fields[1], "job")?,
        target_peer: record_string(&fields[2], "target-peer")?,
        stage_ids: record_node_id_sequence(&fields[3], "stages")?,
        sync_ref: record_ref(&fields[4], "sync")?,
        admission_ref: record_ref(&fields[5], "admission")?,
        execution_request_ref: record_ref(&fields[6], "execution-request")?,
        authority_refs: record_ref_sequence(&fields[7], "authority")?,
        resource_refs: record_ref_sequence(&fields[8], "resource")?,
        peer_bootstrap_refs: record_ref_sequence(&fields[9], "peer-bootstrap")?,
        node_identity_refs: record_ref_sequence(&fields[10], "node-identity")?,
        evidence_refs: record_ref_sequence(&fields[11], "evidence")?,
        value: value.clone(),
    })
}

pub fn job_worker_envelope(input: JobWorkerEnvelopeInput<'_>) -> Result<remote_dataspace::RemoteDataspaceEnvelope> {
    let request = parse_job_worker_request_value(input.request_value)?;
    if input.to_peer != request.target_peer {
        return Err(MoltenError::invalid_harness(format!(
            "job worker envelope target {} does not match request target {}",
            input.to_peer, request.target_peer
        )));
    }
    remote_dataspace::build_envelope(remote_dataspace::RemoteDataspaceEnvelopeInput {
        from_peer: input.from_peer.to_string(),
        from_actor: input.from_actor.to_string(),
        to_peer: input.to_peer.to_string(),
        topic: input.topic.to_string(),
        operation: remote_dataspace::RemoteDataspaceOperation::Message,
        payload: input.request_value.clone(),
        content_refs: Vec::new(),
        capability_refs: request.authority_refs.clone(),
        evidence_refs: sorted_unique(&request.evidence_refs),
    })
}

pub fn execute_worker_delivery(input: JobWorkerExecuteInput<'_>) -> Result<JobWorkerExecution> {
    let request = parse_job_worker_request_value(&input.delivery.envelope.payload)?;
    let assignment_value = job_worker_assignment_value(&request, input.delivery)?;
    let assignment_ref = canonical_hash(&assignment_value)?;
    let delivery = collect_delivery_checks(&input, &request)?;
    let run = run_worker_delivery(&input, &request, &delivery)?;
    finish_worker_delivery(FinishDeliveryInput {
        input,
        request,
        assignment_value,
        assignment_ref,
        delivery,
        run,
    })
}

pub fn live_unrecorded_worker_result(input: JobWorkerExecuteInput<'_>) -> Result<JobWorkerExecution> {
    let without_log = JobWorkerExecuteInput {
        delivery_log: None,
        ..input
    };
    execute_worker_delivery(without_log)
}

pub fn parse_job_worker_result_value(value: &IOValue) -> Result<JobWorkerResult> {
    let fields = value
        .collect_simple_record("job-worker-result-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-worker-result-v1 ...>"))?;
    require_schema(&fields[0], JOB_WORKER_RESULT_SCHEMA, "job worker result")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_worker_decision(&decision)?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "canonical-result", "job worker result")?;
    Ok(JobWorkerResult {
        result_ref: canonical_hash(value)?,
        decision,
        job_ref: record_ref(&fields[2], "job")?,
        target_peer: record_string(&fields[3], "target-peer")?,
        execution_receipt_ref: record_optional_ref(&fields[4], "execution-receipt")?,
        output_refs: record_ref_sequence(&fields[5], "outputs")?,
        stage_receipt_refs: record_stage_receipt_sequence(&fields[6], "stage-receipts")?,
        resource_receipt_refs: record_ref_sequence(&fields[7], "resource")?,
        delivery_log_ref: record_optional_ref(&fields[8], "delivery-log")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn parse_job_worker_receipt_value(value: &IOValue) -> Result<JobWorkerReceipt> {
    let fields = value
        .collect_simple_record("job-worker-receipt-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-worker-receipt-v1 ...>"))?;
    require_schema(&fields[0], JOB_WORKER_RECEIPT_SCHEMA, "job worker receipt")?;
    let operation = record_string(&fields[1], "operation")?;
    if operation != "worker-execute" {
        return Err(MoltenError::invalid_harness(format!("unsupported job worker receipt operation {operation}")));
    }
    let decision = record_string(&fields[2], "decision")?;
    validate_worker_decision(&decision)?;
    require_check(&parse_checks(&fields[12])?, "canonical-receipt", "job worker receipt")?;
    Ok(JobWorkerReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        job_ref: record_optional_ref(&fields[3], "job")?,
        request_ref: record_optional_ref(&fields[4], "request")?,
        assignment_ref: record_ref(&fields[5], "assignment")?,
        status_refs: record_ref_sequence(&fields[6], "status")?,
        result_ref: record_ref(&fields[7], "result")?,
        execution_receipt_ref: record_optional_ref(&fields[8], "execution-receipt")?,
        delivery_log_ref: record_optional_ref(&fields[9], "delivery-log")?,
        diagnostics: record_string_sequence(&fields[10], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn parse_job_worker_schedule_receipt_value(value: &IOValue) -> Result<JobWorkerScheduleReceipt> {
    let fields = value
        .collect_simple_record("job-worker-schedule-receipt-v1", Some(20))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-worker-schedule-receipt-v1 ...>"))?;
    require_schema(&fields[0], JOB_WORKER_SCHEDULE_RECEIPT_SCHEMA, "job worker schedule receipt")?;
    let operation = record_string(&fields[1], "operation")?;
    if operation != "worker-schedule-local" {
        return Err(MoltenError::invalid_harness(format!("unsupported job worker schedule operation {operation}")));
    }
    let decision = record_string(&fields[2], "decision")?;
    validate_decision(&decision)?;
    require_check(&parse_checks(&fields[19])?, "canonical-receipt", "job worker schedule receipt")?;
    Ok(JobWorkerScheduleReceipt {
        receipt_ref: canonical_hash(value)?,
        operation,
        decision,
        job_ref: record_ref(&fields[3], "job")?,
        request_ref: record_ref(&fields[4], "request")?,
        queue_key: record_string(&fields[5], "queue-key")?,
        lease_key: record_string(&fields[6], "lease-key")?,
        worker_session: record_string(&fields[7], "worker-session")?,
        coordination_report_ref: record_ref(&fields[8], "coordination-report")?,
        token_ref: record_optional_ref(&fields[14], "token")?,
        worker_receipt_ref: record_optional_ref(&fields[15], "worker-receipt")?,
        result_ref: record_optional_ref(&fields[16], "result")?,
        diagnostics: record_string_sequence(&fields[17], "diagnostics")?,
        refs: record_ref_sequence(&fields[18], "refs")?,
        value: value.clone(),
    })
}

pub fn parse_blob_ref_job_receipt_value(value: &IOValue) -> Result<JobReceipt> {
    let fields = value
        .collect_simple_record("job-ref-receipt-v1", Some(18))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-ref-receipt-v1 ...>"))?;
    require_schema(&fields[0], JOB_REF_RECEIPT_SCHEMA, "job ref receipt")?;
    let checks = parse_checks(&fields[17])?;
    require_check(&checks, "content-refs-only", "job ref receipt")?;
    require_check(&checks, "no-inline-large-bytes", "job ref receipt")?;
    Ok(JobReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        job_ref: Some(record_string(&fields[4], "job-id")?),
        request_ref: Some(record_ref(&fields[3], "submission")?),
        stage_id: None,
        input_refs: record_ref_sequence(&fields[7], "inputs")?,
        output_refs: record_optional_ref(&fields[13], "output")?.into_iter().collect(),
        cache_ref: record_optional_ref(&fields[14], "output-put")?,
        checks,
        value: value.clone(),
    })
}

pub fn parse_job_dag_value(value: &IOValue) -> Result<JobDag> {
    let fields = value
        .collect_simple_record("job-dag-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-dag-v1 ...>"))?;
    require_schema(&fields[0], JOB_DAG_SCHEMA, "job dag")?;
    let version = record_string(&fields[1], "version")?;
    if version != "v1" {
        return Err(MoltenError::invalid_harness(format!("unsupported job dag version {version}")));
    }
    let nodes = parse_node_sequence(&fields[2])?;
    if nodes.is_empty() {
        return Err(MoltenError::invalid_harness("job dag requires at least one node"));
    }
    let mut node_ids = BTreeSet::new();
    for node in &nodes {
        if !node_ids.insert(node.id.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate job node id {}", node.id)));
        }
    }
    let edges = parse_edge_sequence(&fields[3])?;
    for edge in &edges {
        if !node_ids.contains(&edge.from_node) {
            return Err(MoltenError::invalid_harness(format!("job edge from unknown node {}", edge.from_node)));
        }
        if !node_ids.contains(&edge.to_node) {
            return Err(MoltenError::invalid_harness(format!("job edge to unknown node {}", edge.to_node)));
        }
    }
    let output_roots = record_node_id_sequence(&fields[4], "outputs")?;
    for root in &output_roots {
        if !node_ids.contains(root) {
            return Err(MoltenError::invalid_harness(format!("job output root {root} is not a node")));
        }
    }
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "canonical-dag", "job dag")?;
    require_check(&checks, "no-name-identity", "job dag")?;
    validate_topology(&nodes, &edges)?;
    Ok(JobDag {
        job_ref: canonical_hash(value)?,
        version,
        nodes,
        edges,
        output_roots,
        schema_refs: record_ref_sequence(&fields[5], "schemas")?,
        effect_manifest_refs: record_ref_sequence(&fields[6], "effect-manifests")?,
        policy_refs: record_ref_sequence(&fields[7], "policies")?,
        evidence_refs: record_ref_sequence(&fields[8], "evidence")?,
        value: value.clone(),
    })
}

pub fn parse_job_output_request_value(value: &IOValue, expected_dag_ref: &str) -> Result<JobOutputRequest> {
    let fields = value
        .collect_simple_record("job-output-request-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-output-request-v1 ...>"))?;
    require_schema(&fields[0], JOB_DAG_OUTPUT_REQUEST_SCHEMA, "job output request")?;
    let dag_ref = record_ref(&fields[1], "dag")?;
    if dag_ref != expected_dag_ref {
        return Err(MoltenError::invalid_harness(format!(
            "job output request dag ref {dag_ref} does not match job {expected_dag_ref}"
        )));
    }
    let roots = record_node_id_sequence(&fields[2], "roots")?;
    let materialization = record_string(&fields[3], "materialization")?;
    validate_request_materialization(&materialization)?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "request-ref-bound", "job output request")?;
    Ok(JobOutputRequest {
        request_ref: canonical_hash(value)?,
        dag_ref,
        roots,
        materialization,
        policy_refs: record_ref_sequence(&fields[4], "policy")?,
        handler_profile_ref: record_optional_ref(&fields[5], "handler-profile")?,
        seed_config_ref: record_optional_ref(&fields[6], "seed-config")?,
        value: value.clone(),
    })
}

pub fn parse_job_receipt(value: &IOValue) -> Result<JobReceipt> {
    let fields = value
        .collect_simple_record("job-dag-receipt-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-dag-receipt-v1 ...>"))?;
    require_schema(&fields[0], JOB_DAG_RECEIPT_SCHEMA, "job dag receipt")?;
    let checks = parse_checks(&fields[13])?;
    require_check(&checks, "canonical-receipt", "job dag receipt")?;
    Ok(JobReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        job_ref: record_optional_ref(&fields[3], "job")?,
        request_ref: record_optional_ref(&fields[4], "request")?,
        stage_id: record_optional_string(&fields[5], "stage")?,
        input_refs: record_ref_sequence(&fields[6], "inputs")?,
        output_refs: record_ref_sequence(&fields[7], "outputs")?,
        cache_ref: record_optional_ref(&fields[8], "cache")?,
        checks,
        value: value.clone(),
    })
}

pub fn parse_job_admission_receipt_value(value: &IOValue) -> Result<JobAdmissionReceipt> {
    let fields = value
        .collect_simple_record("job-admission-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-admission-receipt-v1 ...>"))?;
    require_schema(&fields[0], JOB_ADMISSION_RECEIPT_SCHEMA, "job admission receipt")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "canonical-receipt", "job admission receipt")?;
    Ok(JobAdmissionReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        job_ref: record_ref(&fields[3], "job")?,
        request_ref: record_ref(&fields[4], "request")?,
        plan_ref: record_ref(&fields[5], "artifact")?,
        sync_ref: record_ref(&fields[6], "sync")?,
        target_peer: record_string(&fields[7], "target-peer")?,
        closure_refs: record_ref_sequence(&fields[8], "closure")?,
        stage_order: record_node_id_sequence(&fields[9], "stages")?,
        authority_receipt_refs: record_ref_sequence(&fields[10], "authority")?,
        source_gate_validation_refs: Vec::new(),
        resource_verdict: record_string(&fields[11], "resource-verdict")?,
        diagnostics: record_string_sequence(&fields[12], "diagnostics")?,
        refs: record_ref_sequence(&fields[13], "refs")?,
        checks,
        value: value.clone(),
    })
}

pub fn install_job_dag(registry_root: &Path, value: &IOValue) -> Result<JobInstall> {
    let dag = parse_job_dag_value(value)?;
    let stage_deps = dag.nodes.iter().filter_map(|node| node.stage_artifact_ref.clone()).collect::<Vec<_>>();
    let install = artifacts::install_artifact(registry_root, &artifacts::ArtifactInstallInput {
        kind: JOB_ARTIFACT_KIND.to_string(),
        payload: dag.value.clone(),
        schema_refs: dag.schema_refs.clone(),
        dependency_refs: sorted_unique(&stage_deps),
        effect_manifest_ref: None,
        policy_refs: if dag.policy_refs.is_empty() {
            vec![local_ref("job-install-policy", &dag.job_ref)?]
        } else {
            dag.policy_refs.clone()
        },
        evidence_refs: if dag.evidence_refs.is_empty() {
            vec![local_ref("job-install-evidence", &dag.job_ref)?]
        } else {
            dag.evidence_refs.clone()
        },
        installer_ref: local_ref("job-installer", &dag.job_ref)?,
        capability_refs: vec![local_ref("job-install-capability", &dag.job_ref)?],
    })?;
    let artifact_receipt_ref = canonical_hash(&install.receipt_value)?;
    let evidence_refs = vec![artifact_receipt_ref];
    let diagnostics = install
        .missing_dependencies
        .iter()
        .map(|reference| format!("missing stage dependency {reference}"))
        .collect::<Vec<_>>();
    let receipt_value = job_receipt_value(JobReceiptInput {
        operation: "install",
        decision: &install.decision,
        job_ref: Some(&dag.job_ref),
        request_ref: None,
        stage_id: None,
        input_refs: &stage_deps,
        output_refs: std::slice::from_ref(&install.artifact_ref),
        cache_ref: None,
        effect_refs: &[],
        policy_refs: &install.artifact.policy_refs,
        evidence_refs: &evidence_refs,
        diagnostics: &diagnostics,
        checks: &[
            ("canonical-dag", "pass"),
            ("no-mobile-closures", "pass"),
            ("artifact-registry-install", if install.decision == "pass" { "pass" } else { "fail" }),
        ],
    })?;
    Ok(JobInstall {
        job_ref: dag.job_ref,
        artifact_ref: install.artifact_ref,
        decision: install.decision,
        receipt_value,
        artifact_receipt_value: install.receipt_value,
    })
}

pub fn job_artifact_ref(registry_root: &Path, job_ref: &str) -> Result<String> {
    validate_ref(job_ref, "job artifact lookup ref")?;
    for artifact in artifacts::list_artifacts(registry_root, Some(JOB_ARTIFACT_KIND))? {
        let payload = artifacts::read_payload(registry_root, &artifact.artifact_ref)?;
        let dag = parse_job_dag_value(&payload)?;
        if dag.job_ref == job_ref || artifact.artifact_ref == job_ref {
            return Ok(artifact.artifact_ref);
        }
    }
    Err(MoltenError::invalid_harness(format!("job artifact {job_ref} not found in registry")))
}

pub fn read_job_dag(registry_root: &Path, reference: &str) -> Result<JobDag> {
    if validate_ref(reference, "job ref").is_ok() {
        if let Ok(payload) = artifacts::read_payload(registry_root, reference)
            && let Ok(dag) = parse_job_dag_value(&payload)
        {
            return Ok(dag);
        }
        for artifact in artifacts::list_artifacts(registry_root, Some(JOB_ARTIFACT_KIND))? {
            let payload = artifacts::read_payload(registry_root, &artifact.artifact_ref)?;
            let dag = parse_job_dag_value(&payload)?;
            if dag.job_ref == reference || artifact.artifact_ref == reference {
                return Ok(dag);
            }
        }
    }
    Err(MoltenError::invalid_harness(format!("job dag {reference} not found in registry")))
}

pub fn read_job_dag_file_or_registry(registry_root: &Path, spec: &str) -> Result<JobDag> {
    let path = Path::new(spec);
    if path.exists() {
        let text = fs::read_to_string(path).map_err(MoltenError::from)?;
        let value = parse_text(&text)?;
        parse_job_dag_value(&value)
    } else {
        read_job_dag(registry_root, spec)
    }
}

pub fn run_job_dag_value(value: &IOValue, options: &JobRunOptions<'_>) -> Result<JobRun> {
    let dag = parse_job_dag_value(value)?;
    run_job_dag(&dag, options)
}

pub fn run_job_dag(dag: &JobDag, options: &JobRunOptions<'_>) -> Result<JobRun> {
    let request = if let Some(output_request) = options.output_request.as_ref() {
        parse_job_output_request_value(output_request, &dag.job_ref)?
    } else {
        default_output_request(dag)?
    };
    ensure_count_at_most(dag.nodes.len(), MAX_JOB_NODES, "job run nodes")?;
    ensure_count_at_most(dag.edges.len(), MAX_JOB_EDGES, "job run edges")?;
    let plan = trellis_execution_plan(&dag.nodes, &dag.edges)?;
    let stages = run_stages(dag, &request, &plan, options)?;
    let finish = complete_run(CompleteInput {
        dag,
        request: &request,
        plan: &plan,
        outputs_by_index: &stages.outputs_by_index,
        output_refs_by_index: &stages.output_refs_by_index,
        stage_receipt_refs: &stages.receipt_refs,
    })?;
    if let Some(ledger_root) = options.ledger_root {
        ledger::import_artifact(ledger_root, &finish.receipt_value)?;
    }
    Ok(JobRun {
        job_ref: dag.job_ref.clone(),
        request_ref: request.request_ref,
        stage_receipt_refs: stages.receipt_refs,
        output_refs: finish.output_refs,
        output_value: finish.output_value,
        receipt_value: finish.receipt_value,
    })
}

struct RunStages {
    receipt_refs: Vec<String>,
    outputs_by_index: Vec<Option<Vec<IOValue>>>,
    output_refs_by_index: Vec<Option<Vec<String>>>,
}

fn run_stages(
    dag: &JobDag,
    request: &JobOutputRequest,
    plan: &TrellisExecutionPlan,
    options: &JobRunOptions<'_>,
) -> Result<RunStages> {
    let mut completed_indices = Vec::with_capacity(plan.order_ids.len());
    let mut outputs_by_index: Vec<Option<Vec<IOValue>>> = vec![None; dag.nodes.len()];
    let mut output_refs_by_index: Vec<Option<Vec<String>>> = vec![None; dag.nodes.len()];
    let mut receipt_refs = Vec::with_capacity(plan.order_ids.len());
    for node_id in &plan.order_ids {
        let deps = plan.dependency_indices.get(node_id).cloned().unwrap_or_default();
        if !trellis::job_dag::all_deps_satisfied(&deps, &completed_indices)
            || trellis::job_dag::unsatisfied_count(&deps, &completed_indices) != 0
        {
            return Err(MoltenError::invalid_harness(format!(
                "trellis dependency readiness failed for job node {node_id}"
            )));
        }
        let node = find_job_node(&dag.nodes, node_id)?;
        let inputs = gather_inputs(node, &dag.edges, &outputs_by_index, &plan.node_index)?;
        let stage = run_stage_with_cache(dag, request, node, &inputs, options)?;
        let receipt_ref = canonical_hash(&stage.receipt_value)?;
        if let Some(ledger_root) = options.ledger_root {
            ledger::import_artifact(ledger_root, &stage.receipt_value)?;
        }
        ensure_count_at_most(stage.output_refs.len(), MAX_JOB_REFS, "job stage output refs")?;
        ensure_count_at_most(stage.output_values.len(), MAX_JOB_STAGE_VALUES, "job stage output values")?;
        push_bounded(&mut receipt_refs, receipt_ref, MAX_JOB_NODES, "job stage receipt refs")?;
        let node_index = *plan
            .node_index
            .get(node_id)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis node index missing for {node_id}")))?;
        let output_refs_slot = output_refs_by_index.get_mut(node_index).ok_or_else(|| {
            MoltenError::invalid_harness(format!("job output refs index {node_index} outside node set"))
        })?;
        *output_refs_slot = Some(stage.output_refs.clone());
        let output_slot = outputs_by_index
            .get_mut(node_index)
            .ok_or_else(|| MoltenError::invalid_harness(format!("job output index {node_index} outside node set")))?;
        *output_slot = Some(stage.output_values);
        push_bounded(
            &mut completed_indices,
            usize_to_u64(node_index, "trellis completed node index")?,
            MAX_JOB_NODES,
            "trellis completed node indices",
        )?;
    }
    Ok(RunStages {
        receipt_refs,
        outputs_by_index,
        output_refs_by_index,
    })
}

struct RunFinish {
    output_refs: Vec<String>,
    output_value: IOValue,
    receipt_value: IOValue,
}

struct CompleteInput<'a> {
    dag: &'a JobDag,
    request: &'a JobOutputRequest,
    plan: &'a TrellisExecutionPlan,
    outputs_by_index: &'a [Option<Vec<IOValue>>],
    output_refs_by_index: &'a [Option<Vec<String>>],
    stage_receipt_refs: &'a [String],
}

fn complete_run(input: CompleteInput<'_>) -> Result<RunFinish> {
    let dag = input.dag;
    let request = input.request;
    let stage_receipt_refs = input.stage_receipt_refs;
    let roots = if request.roots.is_empty() {
        sink_nodes(dag)?
    } else {
        request.roots.clone()
    };
    ensure_count_at_most(roots.len(), MAX_JOB_ROOTS, "job output roots")?;
    let mut final_values = Vec::with_capacity(roots.len());
    let mut final_refs = Vec::with_capacity(roots.len());
    for root in roots {
        let root_index =
            *input.plan.node_index.get(&root).ok_or_else(|| {
                MoltenError::invalid_harness(format!("job output root {root} missing from node index"))
            })?;
        let values = input
            .outputs_by_index
            .get(root_index)
            .and_then(Option::as_ref)
            .ok_or_else(|| MoltenError::invalid_harness(format!("job output root {root} was not executed")))?;
        extend_cloned_bounded(&mut final_values, values, MAX_JOB_STAGE_VALUES, "job final values")?;
        if let Some(refs) = input.output_refs_by_index.get(root_index).and_then(Option::as_ref) {
            extend_cloned_bounded(&mut final_refs, refs, MAX_JOB_REFS, "job final refs")?;
        }
    }
    let output_value = sequence(final_values.clone());
    if final_refs.is_empty() {
        push_bounded(&mut final_refs, canonical_hash(&output_value)?, MAX_JOB_REFS, "job final refs")?;
    }
    let evidence_count = checked_count_sum(
        dag.evidence_refs.len(),
        stage_receipt_refs.len(),
        MAX_JOB_REFS,
        "job receipt evidence refs",
    )?;
    let mut evidence_refs = Vec::with_capacity(evidence_count);
    extend_cloned_bounded(&mut evidence_refs, &dag.evidence_refs, MAX_JOB_REFS, "job receipt evidence refs")?;
    extend_cloned_bounded(&mut evidence_refs, stage_receipt_refs, MAX_JOB_REFS, "job receipt evidence refs")?;
    let receipt_value = job_receipt_value(JobReceiptInput {
        operation: "run",
        decision: "pass",
        job_ref: Some(&dag.job_ref),
        request_ref: Some(&request.request_ref),
        stage_id: None,
        input_refs: stage_receipt_refs,
        output_refs: &final_refs,
        cache_ref: None,
        effect_refs: &[],
        policy_refs: &combined_policy_refs(dag, request, None),
        evidence_refs: &evidence_refs,
        diagnostics: &[],
        checks: &[
            ("deterministic-topological-order", "pass"),
            ("trellis-topo-order", "pass"),
            ("trellis-deps-ready", "pass"),
            ("stage-receipts-bound", "pass"),
            ("output-refs-bound", "pass"),
        ],
    })?;
    Ok(RunFinish {
        output_refs: final_refs,
        output_value,
        receipt_value,
    })
}

fn stage_plan_values(dag: &JobDag, plan: &TrellisExecutionPlan) -> Result<Vec<IOValue>> {
    let mut node_map = BTreeMap::new();
    for node in &dag.nodes {
        insert_bounded(&mut node_map, node.id.clone(), node, MAX_JOB_NODES, "job plan node map")?;
    }
    let mut stage_values = Vec::with_capacity(plan.order_ids.len());
    for node_id in &plan.order_ids {
        let node = node_map
            .get(node_id)
            .ok_or_else(|| MoltenError::invalid_harness(format!("job plan missing node {node_id}")))?;
        let index = *plan
            .node_index
            .get(node_id)
            .ok_or_else(|| MoltenError::invalid_harness(format!("job plan missing index for {node_id}")))?;
        let deps = dependency_ids(plan, node_id)?;
        push_bounded(
            &mut stage_values,
            record("job-stage-plan-v1", vec![
                record("id", vec![string(node_id)]),
                record("trellis-index", vec![u64_value(usize_to_u64(index, "job plan trellis index")?)]),
                record("dependencies", vec![sequence(deps.iter().map(string).collect())]),
                record("placement", vec![string("local")]),
                record("cache-projection", vec![string(if node.kind == "materialize" {
                    "not-cacheable"
                } else {
                    "eligible"
                })]),
                record("policy", vec![refs_sequence(&node.policy_refs)]),
                record("resources", vec![sequence(Vec::new())]),
                checks_value(&["trellis-dependencies-bound", "placement-is-proposal", "local-only-plan"]),
            ]),
            MAX_JOB_STAGE_VALUES,
            "job plan stage values",
        )?;
    }
    Ok(stage_values)
}

pub fn plan_job_dag(dag: &JobDag, output_request: Option<&IOValue>) -> Result<JobPlan> {
    let request = request_for_analysis(dag, output_request)?;
    let plan = trellis_execution_plan(&dag.nodes, &dag.edges)?;
    let stage_values = stage_plan_values(dag, &plan)?;
    let value = record("job-plan-v1", vec![
        string(JOB_PLAN_SCHEMA),
        record("job", vec![string(&dag.job_ref)]),
        record("request", vec![string(&request.request_ref)]),
        record("stage-order", vec![sequence(plan.order_ids.iter().map(string).collect())]),
        record("stages", vec![sequence(stage_values)]),
        record("policy", vec![refs_sequence(&combined_policy_refs(dag, &request, None))]),
        checks_value(&[
            "trellis-topo-order",
            "trellis-deps-ready",
            "canonical-node-index-map",
            "placement-proposals-only",
        ]),
    ]);
    let plan_ref = canonical_hash(&value)?;
    let receipt_value = analysis_receipt_value(AnalysisReceiptValueInput {
        label: "job-plan-receipt-v1",
        schema: JOB_PLAN_RECEIPT_SCHEMA,
        operation: "plan",
        job_ref: &dag.job_ref,
        request_ref: &request.request_ref,
        artifact_ref: &plan_ref,
        diagnostics: &[],
        checks: &[
            ("trellis-topo-order", "pass"),
            ("trellis-deps-ready", "pass"),
            ("canonical-plan-ref", "pass"),
        ],
    })?;
    Ok(JobPlan {
        plan_ref,
        job_ref: dag.job_ref.clone(),
        request_ref: request.request_ref,
        stage_order: plan.order_ids,
        value,
        receipt_value,
    })
}

struct StageProfiles {
    config_bytes: u64,
    values: Vec<IOValue>,
}

fn stage_profile_values(dag: &JobDag, plan: &TrellisExecutionPlan, cache_entries: usize) -> Result<StageProfiles> {
    let mut config_bytes = 0_u64;
    let mut values = Vec::with_capacity(plan.order_ids.len());
    for node_id in &plan.order_ids {
        let node = dag
            .nodes
            .iter()
            .find(|candidate| candidate.id == *node_id)
            .ok_or_else(|| MoltenError::invalid_harness(format!("job profile missing node {node_id}")))?;
        let bytes = usize_to_u64(canonical_bytes(&node.config)?.len(), "job profile config bytes")?;
        config_bytes = config_bytes
            .checked_add(bytes)
            .ok_or_else(|| MoltenError::invalid_harness("job profile estimated config bytes overflowed"))?;
        push_bounded(
            &mut values,
            record("job-stage-profile-v1", vec![
                record("id", vec![string(node_id)]),
                record("kind", vec![string(&node.kind)]),
                record("estimated-config-bytes", vec![u64_value(bytes)]),
                record("cache-projection", vec![string(if node.kind == "materialize" {
                    "not-cacheable"
                } else if cache_entries == 0 {
                    "projected-miss"
                } else {
                    "candidate-hit-or-miss"
                })]),
                checks_value(&["deterministic-estimate", "no-wall-clock-time"]),
            ]),
            MAX_JOB_STAGE_VALUES,
            "job profile stage values",
        )?;
    }
    Ok(StageProfiles { config_bytes, values })
}

pub fn profile_job_dag(
    dag: &JobDag,
    output_request: Option<&IOValue>,
    cache_root: Option<&Path>,
) -> Result<JobProfile> {
    let request = request_for_analysis(dag, output_request)?;
    let plan = trellis_execution_plan(&dag.nodes, &dag.edges)?;
    let cache_entries = if let Some(cache_root) = cache_root {
        eval_cache::list(cache_root, &eval_cache::EvalCacheListFilter {
            operation: Some(JOB_CACHE_OPERATION.to_string()),
            ..eval_cache::EvalCacheListFilter::default()
        })?
        .len()
    } else {
        0
    };
    let profile_stages = stage_profile_values(dag, &plan, cache_entries)?;
    let materialization_boundaries = usize_to_u64(
        dag.edges.iter().filter(|edge| edge.materialization != "stream").count()
            + dag.nodes.iter().filter(|node| node.kind == "materialize").count(),
        "job profile materialization boundary count",
    )?;
    let stage_count = usize_to_u64(dag.nodes.len(), "job profile stage count")?;
    let edge_count = usize_to_u64(dag.edges.len(), "job profile edge count")?;
    let value = record("job-profile-v1", vec![
        string(JOB_PROFILE_SCHEMA),
        record("job", vec![string(&dag.job_ref)]),
        record("request", vec![string(&request.request_ref)]),
        record("stage-count", vec![u64_value(stage_count)]),
        record("edge-count", vec![u64_value(edge_count)]),
        record("materialization-boundaries", vec![u64_value(materialization_boundaries)]),
        record("estimated-bytes", vec![
            record("config", vec![u64_value(profile_stages.config_bytes)]),
            record("known-cache-entries", vec![u64_value(usize_to_u64(cache_entries, "job cache entry count")?)]),
        ]),
        record("stages", vec![sequence(profile_stages.values)]),
        checks_value(&[
            "deterministic-profile",
            "no-wall-clock-time",
            "cache-projection-only",
            "trellis-order-bound",
        ]),
    ]);
    let profile_ref = canonical_hash(&value)?;
    let receipt_value = analysis_receipt_value(AnalysisReceiptValueInput {
        label: "job-profile-receipt-v1",
        schema: JOB_PROFILE_RECEIPT_SCHEMA,
        operation: "profile",
        job_ref: &dag.job_ref,
        request_ref: &request.request_ref,
        artifact_ref: &profile_ref,
        diagnostics: &[],
        checks: &[
            ("deterministic-profile", "pass"),
            ("no-wall-clock-time", "pass"),
            ("cache-projection-only", "pass"),
        ],
    })?;
    Ok(JobProfile {
        profile_ref,
        job_ref: dag.job_ref.clone(),
        request_ref: request.request_ref,
        stage_count,
        edge_count,
        materialization_boundaries,
        value,
        receipt_value,
    })
}

pub fn sync_plan_value(source_registry: &Path, target_registry: &Path, request_value: &IOValue) -> Result<JobSyncPlan> {
    let request = parse_job_sync_request_value(request_value)?;
    let dag = read_job_dag(source_registry, &request.job_ref)?;
    let roots = sync_roots(source_registry, &dag, &request)?;
    let closure = artifacts::dependency_closure(source_registry, &roots)?;
    if !closure.missing_refs.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "job sync source dependency closure missing refs: {}",
            closure.missing_refs.join(",")
        )));
    }
    let mut missing_refs = Vec::new();
    for artifact_ref in &closure.closure_refs {
        match artifacts::read_artifact(target_registry, artifact_ref) {
            Ok(_) => {}
            Err(_) => push_bounded(&mut missing_refs, artifact_ref.clone(), MAX_JOB_REFS, "job sync missing refs")?,
        }
    }
    let value = record("job-sync-plan-v1", vec![
        string(JOB_SYNC_PLAN_SCHEMA),
        record("request", vec![string(&request.request_ref)]),
        record("job", vec![string(&request.job_ref)]),
        record("target-peer", vec![string(&request.target_peer)]),
        record("roots", vec![refs_sequence(&roots)]),
        record("closure", vec![refs_sequence(&closure.closure_refs)]),
        record("missing", vec![refs_sequence(&missing_refs)]),
        record("stages", vec![sequence(request.stage_ids.iter().map(string).collect())]),
        checks_value(&[
            "dependency-closure",
            "hash-verify-before-install",
            "transport-neutral",
            "no-execution",
            "no-mobile-closures",
        ]),
    ]);
    let plan_ref = canonical_hash(&value)?;
    let receipt_value = analysis_receipt_value(AnalysisReceiptValueInput {
        label: "job-sync-receipt-v1",
        schema: JOB_SYNC_RECEIPT_SCHEMA,
        operation: "sync-plan",
        job_ref: &request.job_ref,
        request_ref: &request.request_ref,
        artifact_ref: &plan_ref,
        diagnostics: &[],
        checks: &[
            ("dependency-closure", "pass"),
            ("missing-set-computed", "pass"),
            ("no-execution", "pass"),
        ],
    })?;
    Ok(JobSyncPlan {
        plan_ref,
        request,
        root_refs: roots,
        closure_refs: closure.closure_refs,
        missing_refs,
        value,
        receipt_value,
    })
}

struct SyncInstallCandidate {
    artifact_ref: String,
    source: artifacts::ArtifactRecord,
    payload: IOValue,
}

struct CandidateSelection {
    install_candidates: Vec<SyncInstallCandidate>,
    already_present_refs: Vec<String>,
    provenance_receipt_refs: Vec<String>,
    diagnostics: Vec<String>,
}

struct ReceiptInput<'a> {
    plan: &'a JobSyncPlan,
    decision: &'a str,
    installed_refs: &'a [String],
    already_present_refs: &'a [String],
    provenance_receipt_refs: &'a [String],
    diagnostics: &'a [String],
}

fn collect_candidates(
    input: &SyncLoopbackInput<'_>,
    plan: &JobSyncPlan,
    ordered_refs: Vec<String>,
) -> Result<CandidateSelection> {
    let missing = plan.missing_refs.iter().cloned().collect::<BTreeSet<_>>();
    let mut install_candidates = Vec::new();
    let mut already_present_refs = Vec::new();
    let mut provenance_receipt_refs = Vec::new();
    let mut diagnostics = Vec::new();
    for artifact_ref in ordered_refs {
        if !missing.contains(&artifact_ref) {
            push_bounded(&mut already_present_refs, artifact_ref, MAX_JOB_REFS, "job sync already-present refs")?;
            continue;
        }
        let source = artifacts::read_artifact(input.source_registry, &artifact_ref)?;
        let payload = artifacts::read_payload(input.source_registry, &artifact_ref)?;
        let provenance = crate::provenance::evaluate_provenance(&crate::provenance::ProvenanceEvaluationInput {
            operation: "remote-sync-install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: input.provenance_values,
            build_verification_values: input.build_verification_values,
            prior_diagnostics: &[],
        })?;
        push_bounded(
            &mut provenance_receipt_refs,
            provenance.receipt_ref.clone(),
            MAX_JOB_REFS,
            "job sync provenance receipt refs",
        )?;
        if provenance.decision == "pass" {
            push_bounded(
                &mut install_candidates,
                SyncInstallCandidate {
                    artifact_ref,
                    source,
                    payload,
                },
                MAX_JOB_REFS,
                "job sync install candidates",
            )?;
        } else {
            push_bounded(
                &mut diagnostics,
                format!("job sync provenance denied artifact {} with receipt {}", artifact_ref, provenance.receipt_ref),
                MAX_JOB_REFS,
                "job sync diagnostics",
            )?;
            for diagnostic in provenance.diagnostics {
                push_bounded(&mut diagnostics, diagnostic, MAX_JOB_REFS, "job sync diagnostics")?;
            }
        }
    }
    Ok(CandidateSelection {
        install_candidates,
        already_present_refs,
        provenance_receipt_refs,
        diagnostics,
    })
}

fn apply_candidates(
    target_registry: &Path,
    request: &JobSyncRequest,
    candidates: Vec<SyncInstallCandidate>,
) -> Result<Vec<String>> {
    let mut installed_refs = Vec::new();
    for candidate in candidates {
        let installed = artifacts::install_artifact(target_registry, &artifacts::ArtifactInstallInput {
            kind: candidate.source.kind.clone(),
            payload: candidate.payload,
            schema_refs: candidate.source.schema_refs.clone(),
            dependency_refs: candidate.source.dependency_refs.clone(),
            effect_manifest_ref: candidate.source.effect_manifest_ref.clone(),
            policy_refs: candidate.source.policy_refs.clone(),
            evidence_refs: candidate.source.evidence_refs.clone(),
            installer_ref: local_ref("job-sync-installer", &request.request_ref)?,
            capability_refs: if request.capability_refs.is_empty() {
                vec![local_ref("job-sync-capability", &request.request_ref)?]
            } else {
                request.capability_refs.clone()
            },
        })?;
        if installed.decision != "pass" || installed.artifact_ref != candidate.artifact_ref {
            return Err(MoltenError::invalid_harness(format!(
                "job sync install mismatch for {}: decision={} installed={}",
                candidate.artifact_ref, installed.decision, installed.artifact_ref
            )));
        }
        let target = artifacts::read_artifact(target_registry, &candidate.artifact_ref)?;
        if target.value != candidate.source.value {
            return Err(MoltenError::invalid_harness(format!(
                "job sync target artifact {} differs from source",
                candidate.artifact_ref
            )));
        }
        push_bounded(&mut installed_refs, candidate.artifact_ref, MAX_JOB_REFS, "job sync installed refs")?;
    }
    Ok(installed_refs)
}

fn loopback_receipt(input: ReceiptInput<'_>) -> Result<IOValue> {
    let mut refs = input.plan.closure_refs.clone();
    extend_cloned_bounded(&mut refs, input.installed_refs, MAX_JOB_REFS, "job sync refs")?;
    extend_cloned_bounded(&mut refs, input.already_present_refs, MAX_JOB_REFS, "job sync refs")?;
    extend_cloned_bounded(&mut refs, input.provenance_receipt_refs, MAX_JOB_REFS, "job sync refs")?;
    push_bounded(&mut refs, input.plan.plan_ref.clone(), MAX_JOB_REFS, "job sync refs")?;
    let is_clean = input.diagnostics.is_empty();
    Ok(record("job-sync-receipt-v1", vec![
        string(JOB_SYNC_RECEIPT_SCHEMA),
        record("operation", vec![string("sync-loopback")]),
        record("decision", vec![string(input.decision)]),
        record("job", vec![string(&input.plan.request.job_ref)]),
        record("request", vec![string(&input.plan.request.request_ref)]),
        record("artifact", vec![string(&input.plan.plan_ref)]),
        record("installed", vec![refs_sequence(input.installed_refs)]),
        record("already-present", vec![refs_sequence(input.already_present_refs)]),
        record("provenance", vec![refs_sequence(input.provenance_receipt_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&[
            ("hash-verify-before-install", status(is_clean)),
            ("provenance-before-install", status(is_clean)),
            ("dependency-closure", "pass"),
            ("loopback-transfer", status(is_clean)),
            ("no-execution", "pass"),
            ("no-mobile-closures", "pass"),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn sync_loopback(input: SyncLoopbackInput<'_>) -> Result<JobSyncLoopback> {
    let plan = sync_plan_value(input.source_registry, input.target_registry, input.request_value)?;
    let ordered_refs = sync_install_order(input.source_registry, &plan.root_refs)?;
    let CandidateSelection {
        install_candidates,
        already_present_refs,
        provenance_receipt_refs,
        diagnostics,
    } = collect_candidates(&input, &plan, ordered_refs)?;
    let installed_refs = if diagnostics.is_empty() {
        apply_candidates(input.target_registry, &plan.request, install_candidates)?
    } else {
        Vec::new()
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = loopback_receipt(ReceiptInput {
        plan: &plan,
        decision,
        installed_refs: &installed_refs,
        already_present_refs: &already_present_refs,
        provenance_receipt_refs: &provenance_receipt_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(JobSyncLoopback {
        receipt_ref,
        plan,
        decision: decision.to_string(),
        installed_refs,
        already_present_refs,
        provenance_receipt_refs,
        diagnostics,
        receipt_value,
    })
}

pub fn admission_plan_value(target_registry: &Path, request_value: &IOValue) -> Result<JobAdmissionPlan> {
    let request = parse_job_admission_request_value(request_value)?;
    let mut diagnostics = Vec::new();
    let mut closure_refs = Vec::new();
    let mut stage_order = Vec::new();
    let mut stage_verdicts = Vec::new();
    let has_target_closure;
    let mut has_valid_topology = true;
    let mut has_executable_artifacts = true;

    let has_explicit_authority = explicit_admission_authority(&request, &mut diagnostics);
    let (has_capability_authority, authority_receipt_refs) =
        capability_contexts_admit(target_registry, &request, &mut diagnostics)?;
    let has_admission_authority = has_explicit_authority && has_capability_authority;
    let has_sync_evidence = sync_evidence_bound(&request, &mut diagnostics);
    let (has_source_gate_evidence, source_gate_validation_refs) =
        source_gate_evidence_bound(target_registry, &request, &mut diagnostics)?;

    match read_job_dag(target_registry, &request.job_ref) {
        Ok(dag) => {
            let selected = selected_stage_set(&dag, &request.stage_ids, &mut diagnostics, &mut stage_verdicts)?;
            if !diagnostics.is_empty()
                && request.stage_ids.iter().any(|stage_id| !dag.nodes.iter().any(|node| node.id == *stage_id))
            {
                has_valid_topology = false;
            }
            match trellis_execution_plan(&dag.nodes, &dag.edges) {
                Ok(plan) => {
                    let node_map = dag.nodes.iter().map(|node| (node.id.clone(), node)).collect::<BTreeMap<_, _>>();
                    let mut completed_indices = Vec::new();
                    for node_id in &plan.order_ids {
                        if !selected.contains(node_id) {
                            continue;
                        }
                        let mut stage_diagnostics = Vec::new();
                        let deps = plan.dependency_indices.get(node_id).cloned().unwrap_or_default();
                        if !trellis::job_dag::all_deps_satisfied(&deps, &completed_indices)
                            || trellis::job_dag::unsatisfied_count(&deps, &completed_indices) != 0
                        {
                            has_valid_topology = false;
                            stage_diagnostics.push(format!("unsatisfied selected-stage dependencies for {node_id}"));
                        }
                        let node = node_map.get(node_id).ok_or_else(|| {
                            MoltenError::invalid_harness(format!("job admission missing node {node_id}"))
                        })?;
                        if node.stage_artifact_ref.is_none() {
                            has_executable_artifacts = false;
                            stage_diagnostics
                                .push(format!("stage {node_id} lacks artifact-backed executable operation"));
                        }
                        if !stage_diagnostics.is_empty() {
                            diagnostics.extend(stage_diagnostics.iter().cloned());
                        }
                        push_bounded(
                            &mut stage_verdicts,
                            JobAdmissionStageVerdict {
                                stage_id: node_id.clone(),
                                decision: if stage_diagnostics.is_empty() { "pass" } else { "deny" }.to_string(),
                                diagnostics: stage_diagnostics,
                            },
                            MAX_JOB_NODES,
                            "job admission stage verdicts",
                        )?;
                        let node_index = *plan.node_index.get(node_id).ok_or_else(|| {
                            MoltenError::invalid_harness(format!("job admission missing trellis index for {node_id}"))
                        })?;
                        push_bounded(
                            &mut completed_indices,
                            usize_to_u64(node_index, "job admission completed node index")?,
                            MAX_JOB_NODES,
                            "job admission completed node indices",
                        )?;
                        push_bounded(&mut stage_order, node_id.clone(), MAX_JOB_NODES, "job admission stage order")?;
                    }
                }
                Err(error) => {
                    has_valid_topology = false;
                    diagnostics.push(format!("trellis topology denied: {error}"));
                }
            }
            let (closure_is_complete, target_closure_refs, target_closure_diagnostics) =
                target_closure_state(target_registry, &dag, &selected)?;
            has_target_closure = closure_is_complete;
            closure_refs = target_closure_refs;
            diagnostics.extend(target_closure_diagnostics);
        }
        Err(error) => {
            has_target_closure = false;
            has_valid_topology = false;
            has_executable_artifacts = false;
            diagnostics.push(format!("target job not available: {error}"));
        }
    }

    let has_resource_profile = resource_profile_admits(&request, &stage_order, &mut diagnostics)?;
    let decision = if has_target_closure
        && has_valid_topology
        && has_executable_artifacts
        && has_admission_authority
        && has_sync_evidence
        && has_source_gate_evidence
        && has_resource_profile
    {
        "pass"
    } else {
        "deny"
    };
    let resource_verdict = if has_resource_profile { "pass" } else { "deny" }.to_string();
    let stage_values = stage_verdicts
        .iter()
        .map(|verdict| {
            record("stage", vec![
                string(&verdict.stage_id),
                string(&verdict.decision),
                record("diagnostics", vec![sequence(verdict.diagnostics.iter().map(string).collect())]),
            ])
        })
        .collect::<Vec<_>>();
    let checks = [
        ("target-closure-present", status(has_target_closure)),
        ("trellis-topology", status(has_valid_topology)),
        ("executable-artifact-gate", status(has_executable_artifacts)),
        ("explicit-authority", status(has_explicit_authority)),
        ("capability-authority-context", status(has_capability_authority)),
        ("resource-profile", status(has_resource_profile)),
        ("sync-evidence-bound", status(has_sync_evidence)),
        ("strict-octet-source-gate-bound", status(has_source_gate_evidence)),
        ("no-execution", "pass"),
    ];
    let value = record("job-admission-plan-v1", vec![
        string(JOB_ADMISSION_PLAN_SCHEMA),
        record("request", vec![string(&request.request_ref)]),
        record("job", vec![string(&request.job_ref)]),
        record("sync", vec![string(&request.sync_ref)]),
        record("target-peer", vec![string(&request.target_peer)]),
        record("stages", vec![sequence(request.stage_ids.iter().map(string).collect())]),
        record("closure", vec![refs_sequence(&closure_refs)]),
        record("topology", vec![sequence(stage_order.iter().map(string).collect())]),
        record("stage-verdicts", vec![sequence(stage_values)]),
        record("authority", vec![refs_sequence(&authority_receipt_refs)]),
        record("resource-verdict", vec![string(&resource_verdict)]),
        record("decision", vec![string(decision)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(&checks),
    ]);
    let plan_ref = canonical_hash(&value)?;
    let receipt_value = job_admission_receipt_value(JobAdmissionReceiptValueInput {
        operation: "admit-plan",
        decision,
        request: &request,
        plan_ref: &plan_ref,
        closure_refs: &closure_refs,
        stage_order: &stage_order,
        authority_receipt_refs: &authority_receipt_refs,
        source_gate_validation_refs: &source_gate_validation_refs,
        resource_verdict: &resource_verdict,
        diagnostics: &diagnostics,
        checks: &checks,
    })?;
    Ok(JobAdmissionPlan {
        plan_ref,
        request,
        closure_refs,
        stage_order,
        stage_verdicts,
        authority_receipt_refs,
        source_gate_validation_refs,
        resource_verdict,
        decision: decision.to_string(),
        diagnostics,
        value,
        receipt_value,
    })
}

pub fn admission_loopback(target_registry: &Path, request_value: &IOValue) -> Result<JobAdmissionLoopback> {
    let plan = admission_plan_value(target_registry, request_value)?;
    let checks = [
        ("target-closure-present", status(plan.decision == "pass" || !plan.closure_refs.is_empty())),
        ("trellis-topology", status(plan.stage_verdicts.iter().all(|verdict| verdict.decision == "pass"))),
        (
            "executable-artifact-gate",
            status(plan.stage_verdicts.iter().all(|verdict| verdict.decision == "pass")),
        ),
        (
            "explicit-authority",
            status(!plan.request.policy_refs.is_empty() && !plan.request.capability_refs.is_empty()),
        ),
        (
            "capability-authority-context",
            status(!plan.authority_receipt_refs.is_empty() && plan.decision == "pass"),
        ),
        ("resource-profile", status(plan.resource_verdict == "pass")),
        (
            "sync-evidence-bound",
            status(plan.request.evidence_refs.iter().any(|reference| reference == &plan.request.sync_ref)),
        ),
        (
            "strict-octet-source-gate-bound",
            status(plan.request.evidence_refs.iter().any(|reference| reference != &plan.request.sync_ref)),
        ),
        ("loopback-admission", "pass"),
        ("no-execution", "pass"),
    ];
    let receipt_value = job_admission_receipt_value(JobAdmissionReceiptValueInput {
        operation: "admit-loopback",
        decision: &plan.decision,
        request: &plan.request,
        plan_ref: &plan.plan_ref,
        closure_refs: &plan.closure_refs,
        stage_order: &plan.stage_order,
        authority_receipt_refs: &plan.authority_receipt_refs,
        source_gate_validation_refs: &plan.source_gate_validation_refs,
        resource_verdict: &plan.resource_verdict,
        diagnostics: &plan.diagnostics,
        checks: &checks,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(JobAdmissionLoopback {
        receipt_ref,
        plan,
        receipt_value,
    })
}

pub fn missing_admission_execution_receipt_value(request_value: &IOValue, diagnostic: &str) -> Result<IOValue> {
    let request = parse_job_execution_request_value(request_value)?;
    let sync_ref = local_ref("missing-execution-sync", &request.admission_ref)?;
    let mut refs = vec![
        request.job_ref.clone(),
        request.request_ref.clone(),
        request.admission_ref.clone(),
        sync_ref.clone(),
    ];
    refs.extend(request.policy_refs.iter().cloned());
    refs.extend(request.capability_refs.iter().cloned());
    refs.extend(request.resource_refs.iter().cloned());
    Ok(record("job-execution-receipt-v1", vec![
        string(JOB_EXECUTION_RECEIPT_SCHEMA),
        record("operation", vec![string("execute-loopback")]),
        record("decision", vec![string("deny")]),
        record("job", vec![string(&request.job_ref)]),
        record("request", vec![string(&request.request_ref)]),
        record("admission", vec![string(&request.admission_ref)]),
        record("sync", vec![string(&sync_ref)]),
        record("target-peer", vec![string(&request.target_peer)]),
        record("closure", vec![refs_sequence(&[])]),
        record("authority", vec![refs_sequence(&[])]),
        record("stages", vec![sequence(Vec::new())]),
        record("outputs", vec![refs_sequence(&[])]),
        record("run", vec![refs_sequence(&[])]),
        record("diagnostics", vec![sequence(vec![string(diagnostic)])]),
        record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&[
            ("admission-required", "fail"),
            ("admission-readable", "fail"),
            ("no-stage-execution-on-deny", "pass"),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn execution_loopback(input: ExecutionLoopbackInput<'_>) -> Result<JobExecutionLoopback> {
    let request = parse_job_execution_request_value(input.request_value)?;
    let admission = parse_job_admission_receipt_value(input.admission_receipt_value)?;
    let mut diagnostics = Vec::new();
    let mut checks = Vec::new();

    let has_matching_admission_ref = admission.receipt_ref == request.admission_ref;
    push_check(&mut checks, "admission-ref-binding", has_matching_admission_ref);
    if !has_matching_admission_ref {
        diagnostics.push(format!(
            "job execution request admission ref {} does not match receipt {}",
            request.admission_ref, admission.receipt_ref
        ));
    }

    let is_admission_pass = admission.decision == "pass";
    push_check(&mut checks, "admission-pass", is_admission_pass);
    if !is_admission_pass {
        diagnostics.push(format!("job execution admission decision is {}", admission.decision));
    }

    let has_matching_job_ref = admission.job_ref == request.job_ref;
    push_check(&mut checks, "job-ref-binding", has_matching_job_ref);
    if !has_matching_job_ref {
        diagnostics.push(format!(
            "job execution request job {} does not match admission job {}",
            request.job_ref, admission.job_ref
        ));
    }

    let has_matching_target_peer = admission.target_peer == request.target_peer;
    push_check(&mut checks, "target-peer-binding", has_matching_target_peer);
    if !has_matching_target_peer {
        diagnostics.push(format!(
            "job execution target peer {} does not match admission target peer {}",
            request.target_peer, admission.target_peer
        ));
    }

    let required_admission_checks = [
        "target-closure-present",
        "trellis-topology",
        "executable-artifact-gate",
        "capability-authority-context",
        "resource-profile",
        "sync-evidence-bound",
        "strict-octet-source-gate-bound",
        "no-execution",
    ];
    let has_required_admission_checks = required_admission_checks
        .iter()
        .all(|required| admission.checks.iter().any(|check| check == *required));
    push_check(&mut checks, "admission-checkset", has_required_admission_checks);
    if !has_required_admission_checks {
        diagnostics.push("job execution admission receipt is missing required target-side checks".to_string());
    }

    let has_authority_receipts = !admission.authority_receipt_refs.is_empty();
    push_check(&mut checks, "authority-receipt-binding", has_authority_receipts);
    if !has_authority_receipts {
        diagnostics.push("job execution admission has no authority receipt refs".to_string());
    }

    let has_resource_profile = admission.resource_verdict == "pass";
    push_check(&mut checks, "resource-profile-binding", has_resource_profile);
    if !has_resource_profile {
        diagnostics.push(format!("job execution resource verdict is {}", admission.resource_verdict));
    }

    let dag = match read_job_dag(input.target_registry, &request.job_ref) {
        Ok(dag) => dag,
        Err(error) => {
            diagnostics.push(format!("target job unavailable before execution: {error}"));
            let receipt_value = job_execution_receipt_value(ExecutionReceiptValueInput {
                decision: "deny",
                request: &request,
                admission: &admission,
                stage_receipt_refs: &[],
                output_refs: &[],
                run_receipt_refs: &[],
                diagnostics: &diagnostics,
                checks: &checks_with_extra(&checks, &[
                    ("target-job-present", "fail"),
                    ("no-stage-execution-on-deny", "pass"),
                ]),
            })?;
            let receipt_ref = canonical_hash(&receipt_value)?;
            return Ok(JobExecutionLoopback {
                receipt_ref,
                request,
                admission,
                run: None,
                decision: "deny".to_string(),
                diagnostics,
                receipt_value,
            });
        }
    };
    push_check(&mut checks, "target-job-present", true);

    let stage_order = if request.stage_ids.is_empty() {
        admission.stage_order.clone()
    } else {
        request.stage_ids.clone()
    };
    let has_selected_stage_binding = stage_order == admission.stage_order;
    push_check(&mut checks, "selected-stage-binding", has_selected_stage_binding);
    if !has_selected_stage_binding {
        diagnostics.push("job execution selected stages do not match admission stage order".to_string());
    }

    let full_stage_order = trellis_execution_plan(&dag.nodes, &dag.edges)?.order_ids;
    let has_full_stage_selection = stage_order == full_stage_order;
    push_check(&mut checks, "selected-stages-full-target", has_full_stage_selection);
    if !has_full_stage_selection {
        diagnostics
            .push("job execution loopback currently requires admitted stages to cover the full target DAG".to_string());
    }

    let closure = recompute_execution_closure(input.target_registry, &dag, &stage_order);
    match closure {
        Ok(closure_refs) => {
            let has_recomputed_closure = sorted_unique(&closure_refs) == sorted_unique(&admission.closure_refs);
            push_check(&mut checks, "target-closure-recomputed", has_recomputed_closure);
            if !has_recomputed_closure {
                diagnostics.push("job execution recomputed target closure diverges from admission closure".to_string());
            }
        }
        Err(error) => {
            push_check(&mut checks, "target-closure-recomputed", false);
            diagnostics.push(format!("job execution target closure recompute failed: {error}"));
        }
    }

    let has_request_ref_bindings = refs_are_bound_in_admission(&request.policy_refs, &admission.refs)
        && refs_are_bound_in_admission(&request.capability_refs, &admission.refs)
        && refs_are_bound_in_admission(&request.resource_refs, &admission.refs);
    push_check(&mut checks, "request-ref-binding", has_request_ref_bindings);
    if !has_request_ref_bindings {
        diagnostics
            .push("job execution request policy/capability/resource refs are not all bound by admission".to_string());
    }

    let decision = if checks.iter().all(|(_, status)| *status == "pass") {
        "pass"
    } else {
        "deny"
    };
    if decision == "deny" {
        let receipt_value = job_execution_receipt_value(ExecutionReceiptValueInput {
            decision: "deny",
            request: &request,
            admission: &admission,
            stage_receipt_refs: &[],
            output_refs: &[],
            run_receipt_refs: &[],
            diagnostics: &diagnostics,
            checks: &checks_with_extra(&checks, &[("no-stage-execution-on-deny", "pass")]),
        })?;
        let receipt_ref = canonical_hash(&receipt_value)?;
        return Ok(JobExecutionLoopback {
            receipt_ref,
            request,
            admission,
            run: None,
            decision: decision.to_string(),
            diagnostics,
            receipt_value,
        });
    }

    let run = run_job_dag(&dag, &JobRunOptions {
        registry_root: input.target_registry,
        storage_root: input.storage_root,
        cache_root: input.cache_root,
        chunk_root: input.chunk_root,
        ledger_root: None,
        output_request: None,
    })?;
    let receipt_value = job_execution_receipt_value(ExecutionReceiptValueInput {
        decision: "pass",
        request: &request,
        admission: &admission,
        stage_receipt_refs: &run.stage_receipt_refs,
        output_refs: &run.output_refs,
        run_receipt_refs: &[canonical_hash(&run.receipt_value)?],
        diagnostics: &diagnostics,
        checks: &checks_with_extra(&checks, &[
            ("executed-on-target-state", "pass"),
            ("stage-receipts-bound", "pass"),
            ("output-refs-bound", "pass"),
        ]),
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(JobExecutionLoopback {
        receipt_ref,
        request,
        admission,
        run: Some(run),
        decision: "pass".to_string(),
        diagnostics,
        receipt_value,
    })
}

pub fn fusion_preview_job_dag(dag: &JobDag, output_request: Option<&IOValue>) -> Result<JobFusionPreview> {
    let request = request_for_analysis(dag, output_request)?;
    let plan = trellis_execution_plan(&dag.nodes, &dag.edges)?;
    let (chain_values, chains) = adjacent_chains(dag, &plan.order_ids)?;
    let value = record("job-fusion-plan-v1", vec![
        string(JOB_FUSION_PLAN_SCHEMA),
        record("job", vec![string(&dag.job_ref)]),
        record("request", vec![string(&request.request_ref)]),
        record("chains", vec![sequence(chain_values)]),
        checks_value(&[
            "trellis-order-bound",
            "no-reduce-materialize-fusion",
            "effect-policy-boundaries-preserved",
            "fusion-is-preview-only",
        ]),
    ]);
    let fusion_ref = canonical_hash(&value)?;
    let receipt_value = analysis_receipt_value(AnalysisReceiptValueInput {
        label: "job-fusion-receipt-v1",
        schema: JOB_FUSION_RECEIPT_SCHEMA,
        operation: "fusion-preview",
        job_ref: &dag.job_ref,
        request_ref: &request.request_ref,
        artifact_ref: &fusion_ref,
        diagnostics: &[],
        checks: &[
            ("trellis-order-bound", "pass"),
            ("effect-policy-boundaries-preserved", "pass"),
            ("fusion-preview-only", "pass"),
        ],
    })?;
    Ok(JobFusionPreview {
        fusion_ref,
        job_ref: dag.job_ref.clone(),
        request_ref: request.request_ref,
        chains,
        value,
        receipt_value,
    })
}

fn adjacent_chains(dag: &JobDag, order_ids: &[String]) -> Result<(Vec<IOValue>, Vec<Vec<String>>)> {
    let positions = order_ids
        .iter()
        .enumerate()
        .map(|(index, node_id)| (node_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let node_map = dag.nodes.iter().map(|node| (node.id.clone(), node)).collect::<BTreeMap<_, _>>();
    let mut edges = dag.edges.iter().collect::<Vec<_>>();
    edges.sort_by(|left, right| fusion_edge_sort_key(&positions, left).cmp(&fusion_edge_sort_key(&positions, right)));

    let mut chain_values = Vec::new();
    let mut chains = Vec::new();
    for edge in edges {
        let from = node_map
            .get(&edge.from_node)
            .ok_or_else(|| MoltenError::invalid_harness(format!("fusion edge from missing node {}", edge.from_node)))?;
        let to = node_map
            .get(&edge.to_node)
            .ok_or_else(|| MoltenError::invalid_harness(format!("fusion edge to missing node {}", edge.to_node)))?;
        if fusion_edge_safe(from, to, edge) {
            let chain = vec![from.id.clone(), to.id.clone()];
            push_bounded(&mut chain_values, adjacent_chain_value(&chain), MAX_JOB_EDGES, "job fusion chain values")?;
            push_bounded(&mut chains, chain, MAX_JOB_EDGES, "job fusion chains")?;
        }
    }
    Ok((chain_values, chains))
}

fn adjacent_chain_value(chain: &[String]) -> IOValue {
    record("job-fusion-chain-v1", vec![
        record("stages", vec![sequence(chain.iter().map(string).collect())]),
        record("reason", vec![string("pure-adjacent-map-filter")]),
        checks_value(&[
            "trellis-adjacent-order",
            "no-materialization-boundary",
            "no-effect-policy-boundary",
            "schema-boundary-preserved",
        ]),
    ])
}

fn selected_stage_set(
    dag: &JobDag,
    requested: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    stage_verdicts: &mut impl crate::bounded::VecSink<JobAdmissionStageVerdict>,
) -> Result<BTreeSet<String>> {
    let known = dag.nodes.iter().map(|node| node.id.clone()).collect::<BTreeSet<_>>();
    if requested.is_empty() {
        return Ok(known);
    }
    let mut selected = BTreeSet::new();
    for stage_id in requested {
        if known.contains(stage_id) {
            selected.insert(stage_id.clone());
        } else {
            let diagnostic = format!("unknown selected stage {stage_id}");
            push_bounded(diagnostics, diagnostic.clone(), MAX_JOB_REFS, "job admission diagnostics")?;
            push_bounded(
                stage_verdicts,
                JobAdmissionStageVerdict {
                    stage_id: stage_id.clone(),
                    decision: "deny".to_string(),
                    diagnostics: vec![diagnostic],
                },
                MAX_JOB_NODES,
                "job admission stage verdicts",
            )?;
        }
    }
    Ok(selected)
}

fn admission_roots(target_registry: &Path, dag: &JobDag, selected: &BTreeSet<String>) -> Result<Vec<String>> {
    let mut roots = vec![job_artifact_ref(target_registry, &dag.job_ref)?];
    for node in &dag.nodes {
        if selected.contains(&node.id)
            && let Some(stage_artifact_ref) = node.stage_artifact_ref.as_ref()
        {
            push_bounded(&mut roots, stage_artifact_ref.clone(), MAX_JOB_REFS, "job admission roots")?;
        }
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn target_closure_state(
    target_registry: &Path,
    dag: &JobDag,
    selected: &BTreeSet<String>,
) -> Result<(bool, Vec<String>, Vec<String>)> {
    let roots = match admission_roots(target_registry, dag, selected) {
        Ok(roots) => roots,
        Err(error) => return Ok((false, Vec::new(), vec![format!("target closure roots denied: {error}")])),
    };
    let closure = match artifacts::dependency_closure(target_registry, &roots) {
        Ok(closure) => closure,
        Err(error) => return Ok((false, Vec::new(), vec![format!("target closure computation failed: {error}")])),
    };

    let mut has_target_closure = true;
    let closure_refs = closure.closure_refs;
    let diagnostic_capacity = closure.missing_refs.len().saturating_add(closure_refs.len());
    let mut diagnostics = Vec::with_capacity(diagnostic_capacity);
    if !closure.missing_refs.is_empty() {
        has_target_closure = false;
        diagnostics.extend(closure.missing_refs.iter().map(|missing| format!("target closure missing {missing}")));
    }
    for artifact_ref in &closure_refs {
        if let Some(diagnostic) = target_closure_artifact_diagnostic(target_registry, artifact_ref) {
            has_target_closure = false;
            diagnostics.push(diagnostic);
        }
    }
    Ok((has_target_closure, closure_refs, diagnostics))
}

fn target_closure_artifact_diagnostic(target_registry: &Path, artifact_ref: &str) -> Option<String> {
    match artifacts::read_artifact(target_registry, artifact_ref) {
        Ok(artifact) if artifact.artifact_ref == artifact_ref => None,
        Ok(artifact) => Some(format!("target artifact key {artifact_ref} contains envelope {}", artifact.artifact_ref)),
        Err(error) => Some(format!("target artifact {artifact_ref} unreadable: {error}")),
    }
}

fn explicit_admission_authority(
    request: &JobAdmissionRequest,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> bool {
    let mut has_explicit_authority = true;
    if request.policy_refs.is_empty() {
        has_explicit_authority = false;
        diagnostics.push_item("job admission missing explicit policy refs".to_string());
    }
    if request.capability_refs.is_empty() {
        has_explicit_authority = false;
        diagnostics.push_item("job admission missing explicit capability refs".to_string());
    }
    if request.evidence_refs.is_empty() {
        has_explicit_authority = false;
        diagnostics.push_item("job admission missing explicit evidence refs".to_string());
    }
    if request.resource_refs.is_empty() {
        has_explicit_authority = false;
        diagnostics.push_item("job admission missing explicit resource refs".to_string());
    }
    has_explicit_authority
}

fn capability_contexts_admit(
    target_registry: &Path,
    request: &JobAdmissionRequest,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<(bool, Vec<String>)> {
    if request.capability_refs.is_empty() {
        return Ok((false, Vec::new()));
    }
    let mut has_passing_authority = false;
    let mut receipt_refs = Vec::new();
    for capability_ref in &request.capability_refs {
        match authority_context_value_for_ref(target_registry, capability_ref)? {
            Some(context_value) => {
                let admission = authority::admit_authority(&context_value, "job:execute", &request.job_ref, 0, &[])?;
                push_bounded(
                    &mut receipt_refs,
                    admission.receipt.receipt_ref.clone(),
                    MAX_JOB_REFS,
                    "job admission authority receipt refs",
                )?;
                if admission.decision == "pass" {
                    has_passing_authority = true;
                } else {
                    diagnostics.push_item(format!(
                        "authority context {capability_ref} denied job:execute for {}",
                        request.job_ref
                    ));
                }
            }
            None => diagnostics.push_item(format!(
                "capability ref {capability_ref} is not an authority-context artifact in target registry"
            )),
        }
    }
    if !has_passing_authority {
        diagnostics.push_item(format!("no authority context admits job:execute for {}", request.job_ref));
    }
    Ok((has_passing_authority, receipt_refs))
}

fn authority_context_value_for_ref(target_registry: &Path, context_ref: &str) -> Result<Option<IOValue>> {
    validate_ref(context_ref, "job admission authority context ref")?;
    for artifact in artifacts::list_artifacts(target_registry, None)? {
        let payload = artifacts::read_payload(target_registry, &artifact.artifact_ref)?;
        if let Ok(context) = authority::parse_authority_context(&payload)
            && context.context_ref == context_ref
        {
            return Ok(Some(payload));
        }
    }
    Ok(None)
}

fn sync_evidence_bound(request: &JobAdmissionRequest, diagnostics: &mut impl crate::bounded::VecSink<String>) -> bool {
    if request.evidence_refs.iter().any(|reference| reference == &request.sync_ref) {
        true
    } else {
        diagnostics.push_item(format!("job admission evidence refs do not bind sync evidence {}", request.sync_ref));
        false
    }
}

fn source_gate_evidence_bound(
    target_registry: &Path,
    request: &JobAdmissionRequest,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<(bool, Vec<String>)> {
    let candidates = request
        .evidence_refs
        .iter()
        .filter(|reference| *reference != &request.sync_ref)
        .cloned()
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        diagnostics.push_item("job admission missing strict Octet source gate evidence ref".to_string());
        return Ok((false, Vec::new()));
    }
    let mut validation_refs = Vec::new();
    let mut has_passing_source_gate = false;
    for candidate in candidates {
        match source_gate_value_for_ref(target_registry, &candidate)? {
            Some(value) => {
                let validation = octet_gate::validate_octet_source_gate(&octet_gate::OctetSourceGateValidationInput {
                    consumer: "job-remote-admission".to_string(),
                    subject_ref: request.job_ref.clone(),
                    gate_receipt_value: Some(value),
                    source_scope: Vec::new(),
                })?;
                push_bounded(
                    &mut validation_refs,
                    validation.validation_ref.clone(),
                    MAX_JOB_REFS,
                    "job admission source gate validation refs",
                )?;
                if validation.decision == "pass" {
                    has_passing_source_gate = true;
                } else {
                    diagnostics.push_item(format!(
                        "strict Octet source gate {candidate} denied validation {}",
                        validation.validation_ref
                    ));
                }
            }
            None => diagnostics.push_item(format!(
                "strict Octet source gate evidence {candidate} is not available as a target artifact payload"
            )),
        }
    }
    if !has_passing_source_gate {
        diagnostics.push_item("job admission found no passing strict Octet source gate validation".to_string());
    }
    Ok((has_passing_source_gate, validation_refs))
}

fn source_gate_value_for_ref(target_registry: &Path, gate_ref: &str) -> Result<Option<IOValue>> {
    validate_ref(gate_ref, "job admission source gate ref")?;
    if let Ok(value) = artifacts::read_payload(target_registry, gate_ref) {
        return Ok(Some(value));
    }
    for artifact in artifacts::list_artifacts(target_registry, None)? {
        let payload = artifacts::read_payload(target_registry, &artifact.artifact_ref)?;
        if canonical_hash(&payload)? == gate_ref {
            return Ok(Some(payload));
        }
    }
    Ok(None)
}

fn resource_profile_admits(
    request: &JobAdmissionRequest,
    stage_order: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<bool> {
    if request.resource_refs.is_empty() {
        return Ok(false);
    }
    let stages = stage_order.iter().map(|stage| (stage.as_str(), 1_u64)).collect::<Vec<_>>();
    let planned = resources::plan_job_stages(&stages, usize_to_u64(request.resource_refs.len(), "job resource refs")?)?;
    if planned.len() == stage_order.len() {
        Ok(true)
    } else {
        diagnostics.push_item(format!(
            "job admission resource refs admit {} of {} selected stages",
            planned.len(),
            stage_order.len()
        ));
        Ok(false)
    }
}

struct JobAdmissionReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    request: &'a JobAdmissionRequest,
    plan_ref: &'a str,
    closure_refs: &'a [String],
    stage_order: &'a [String],
    authority_receipt_refs: &'a [String],
    source_gate_validation_refs: &'a [String],
    resource_verdict: &'a str,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

fn job_admission_receipt_value(input: JobAdmissionReceiptValueInput<'_>) -> Result<IOValue> {
    validate_non_empty(input.operation, "job admission receipt operation")?;
    validate_decision(input.decision)?;
    validate_ref(input.plan_ref, "job admission receipt plan ref")?;
    validate_refs(input.closure_refs, "job admission receipt closure ref")?;
    validate_refs(input.authority_receipt_refs, "job admission authority receipt ref")?;
    validate_refs(input.source_gate_validation_refs, "job admission source gate validation ref")?;
    for stage_id in input.stage_order {
        validate_node_id(stage_id)?;
    }
    validate_decision(input.resource_verdict)?;
    let mut refs = vec![
        input.request.job_ref.clone(),
        input.request.sync_ref.clone(),
        input.request.request_ref.clone(),
        input.plan_ref.to_string(),
    ];
    refs.extend(input.closure_refs.iter().cloned());
    refs.extend(input.authority_receipt_refs.iter().cloned());
    refs.extend(input.source_gate_validation_refs.iter().cloned());
    refs.extend(input.request.policy_refs.iter().cloned());
    refs.extend(input.request.capability_refs.iter().cloned());
    refs.extend(input.request.evidence_refs.iter().cloned());
    refs.extend(input.request.resource_refs.iter().cloned());
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(record("job-admission-receipt-v1", vec![
        string(JOB_ADMISSION_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("job", vec![string(&input.request.job_ref)]),
        record("request", vec![string(&input.request.request_ref)]),
        record("artifact", vec![string(input.plan_ref)]),
        record("sync", vec![string(&input.request.sync_ref)]),
        record("target-peer", vec![string(&input.request.target_peer)]),
        record("closure", vec![refs_sequence(input.closure_refs)]),
        record("stages", vec![sequence(input.stage_order.iter().map(string).collect())]),
        record("authority", vec![refs_sequence(input.authority_receipt_refs)]),
        record("resource-verdict", vec![string(input.resource_verdict)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

fn job_execution_receipt_value(input: ExecutionReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_refs(input.stage_receipt_refs, "job execution stage receipt ref")?;
    validate_refs(input.output_refs, "job execution output ref")?;
    validate_refs(input.run_receipt_refs, "job execution run receipt ref")?;
    let stage_values = input
        .admission
        .stage_order
        .iter()
        .enumerate()
        .map(|(index, stage_id)| {
            record("stage", vec![
                string(stage_id),
                optional_ref_value(input.stage_receipt_refs.get(index).map(String::as_str)),
            ])
        })
        .collect::<Vec<_>>();
    let mut refs = vec![
        input.request.job_ref.clone(),
        input.request.request_ref.clone(),
        input.request.admission_ref.clone(),
        input.admission.sync_ref.clone(),
        input.admission.plan_ref.clone(),
    ];
    refs.extend(input.admission.closure_refs.iter().cloned());
    refs.extend(input.admission.authority_receipt_refs.iter().cloned());
    refs.extend(input.stage_receipt_refs.iter().cloned());
    refs.extend(input.output_refs.iter().cloned());
    refs.extend(input.run_receipt_refs.iter().cloned());
    refs.extend(input.request.policy_refs.iter().cloned());
    refs.extend(input.request.capability_refs.iter().cloned());
    refs.extend(input.request.resource_refs.iter().cloned());
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(record("job-execution-receipt-v1", vec![
        string(JOB_EXECUTION_RECEIPT_SCHEMA),
        record("operation", vec![string("execute-loopback")]),
        record("decision", vec![string(input.decision)]),
        record("job", vec![string(&input.request.job_ref)]),
        record("request", vec![string(&input.request.request_ref)]),
        record("admission", vec![string(&input.request.admission_ref)]),
        record("sync", vec![string(&input.admission.sync_ref)]),
        record("target-peer", vec![string(&input.request.target_peer)]),
        record("closure", vec![refs_sequence(&input.admission.closure_refs)]),
        record("authority", vec![refs_sequence(&input.admission.authority_receipt_refs)]),
        record("stages", vec![sequence(stage_values)]),
        record("outputs", vec![refs_sequence(input.output_refs)]),
        record("run", vec![refs_sequence(input.run_receipt_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

fn job_worker_assignment_value(
    request: &JobWorkerRequest,
    delivery: &remote_dataspace::RemoteDataspaceDelivery,
) -> Result<IOValue> {
    Ok(record("job-worker-assignment-v1", vec![
        string(JOB_WORKER_ASSIGNMENT_SCHEMA),
        record("request", vec![string(&request.request_ref)]),
        record("job", vec![string(&request.job_ref)]),
        record("target-peer", vec![string(&request.target_peer)]),
        record("stages", vec![sequence(request.stage_ids.iter().map(string).collect())]),
        record("from-peer", vec![string(&delivery.envelope.from_peer)]),
        record("delivery-envelope", vec![string(&delivery.envelope.envelope_ref)]),
        record("operation-ref", vec![string(&delivery.envelope.operation_ref)]),
        record("execution-request", vec![string(&request.execution_request_ref)]),
        checks_value(&[
            "request-assigned-to-target",
            "remote-dataspace-envelope-bound",
            "delivery-operation-ref-bound",
            "transport-is-not-authority",
        ]),
    ]))
}

fn job_worker_status_value(input: WorkerStatusValueInput<'_>) -> Result<IOValue> {
    validate_worker_state(input.state)?;
    let mut refs = vec![
        input.request.request_ref.clone(),
        input.delivery.envelope.envelope_ref.clone(),
        input.delivery.envelope.operation_ref.clone(),
    ];
    if let Some(execution_receipt_ref) = input.execution_receipt_ref {
        push_bounded(&mut refs, execution_receipt_ref.to_string(), MAX_JOB_REFS, "job worker status refs")?;
    }
    let mut status_checks = input.checks.to_vec();
    status_checks.push(("canonical-status", "pass"));
    status_checks.push(("delivery-operation-ref-bound", "pass"));
    Ok(record("job-worker-status-v1", vec![
        string(JOB_WORKER_STATUS_SCHEMA),
        record("request", vec![string(&input.request.request_ref)]),
        record("job", vec![string(&input.request.job_ref)]),
        record("target-peer", vec![string(&input.request.target_peer)]),
        record("state", vec![string(input.state)]),
        record("delivery-envelope", vec![string(&input.delivery.envelope.envelope_ref)]),
        record("operation-ref", vec![string(&input.delivery.envelope.operation_ref)]),
        record("execution-receipt", vec![optional_ref_value(input.execution_receipt_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&status_checks),
    ]))
}

fn job_worker_result_value(input: WorkerResultValueInput<'_>) -> Result<IOValue> {
    validate_worker_decision(input.decision)?;
    validate_refs(input.output_refs, "job worker output ref")?;
    validate_stage_receipt_refs(input.stage_receipt_refs)?;
    validate_refs(input.resource_receipt_refs, "job worker resource receipt ref")?;
    if let Some(delivery_log_ref) = input.delivery_log_ref {
        validate_ref(delivery_log_ref, "job worker delivery log ref")?;
    }
    let mut refs = vec![
        input.request.request_ref.clone(),
        input.request.job_ref.clone(),
        input.request.sync_ref.clone(),
        input.request.admission_ref.clone(),
        input.request.execution_request_ref.clone(),
    ];
    if let Some(execution_receipt_ref) = input.execution_receipt_ref {
        push_bounded(&mut refs, execution_receipt_ref.to_string(), MAX_JOB_REFS, "job worker result refs")?;
    }
    extend_cloned_bounded(&mut refs, input.output_refs, MAX_JOB_REFS, "job worker result refs")?;
    for (_, receipt_ref) in input.stage_receipt_refs {
        push_bounded(&mut refs, receipt_ref.clone(), MAX_JOB_REFS, "job worker result refs")?;
    }
    extend_cloned_bounded(&mut refs, input.resource_receipt_refs, MAX_JOB_REFS, "job worker result refs")?;
    if let Some(delivery_log_ref) = input.delivery_log_ref {
        push_bounded(&mut refs, delivery_log_ref.to_string(), MAX_JOB_REFS, "job worker result refs")?;
    }
    let stage_values = input
        .stage_receipt_refs
        .iter()
        .map(|(stage_id, receipt_ref)| record("stage", vec![string(stage_id), string(receipt_ref)]))
        .collect::<Vec<_>>();
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-result", "pass"));
    Ok(record("job-worker-result-v1", vec![
        string(JOB_WORKER_RESULT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("job", vec![string(&input.request.job_ref)]),
        record("target-peer", vec![string(&input.request.target_peer)]),
        record("execution-receipt", vec![optional_ref_value(input.execution_receipt_ref)]),
        record("outputs", vec![refs_sequence(input.output_refs)]),
        record("stage-receipts", vec![sequence(stage_values)]),
        record("resource", vec![refs_sequence(input.resource_receipt_refs)]),
        record("delivery-log", vec![optional_ref_value(input.delivery_log_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

fn job_worker_receipt_value(input: WorkerReceiptValueInput<'_>) -> Result<IOValue> {
    validate_worker_decision(input.decision)?;
    validate_ref(input.assignment_ref, "job worker assignment ref")?;
    validate_refs(input.status_refs, "job worker status ref")?;
    validate_ref(input.result_ref, "job worker result ref")?;
    if let Some(execution_receipt_ref) = input.execution_receipt_ref {
        validate_ref(execution_receipt_ref, "job worker execution receipt ref")?;
    }
    if let Some(delivery_log_ref) = input.delivery_log_ref {
        validate_ref(delivery_log_ref, "job worker delivery log ref")?;
    }
    let request_ref = input.request.map(|request| request.request_ref.as_str());
    let job_ref = input.request.map(|request| request.job_ref.as_str());
    let mut refs = vec![input.assignment_ref.to_string(), input.result_ref.to_string()];
    extend_cloned_bounded(&mut refs, input.status_refs, MAX_JOB_REFS, "job worker receipt refs")?;
    if let Some(request) = input.request {
        push_bounded(&mut refs, request.request_ref.clone(), MAX_JOB_REFS, "job worker receipt refs")?;
        push_bounded(&mut refs, request.job_ref.clone(), MAX_JOB_REFS, "job worker receipt refs")?;
    }
    if let Some(execution_receipt_ref) = input.execution_receipt_ref {
        push_bounded(&mut refs, execution_receipt_ref.to_string(), MAX_JOB_REFS, "job worker receipt refs")?;
    }
    if let Some(delivery_log_ref) = input.delivery_log_ref {
        push_bounded(&mut refs, delivery_log_ref.to_string(), MAX_JOB_REFS, "job worker receipt refs")?;
    }
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(record("job-worker-receipt-v1", vec![
        string(JOB_WORKER_RECEIPT_SCHEMA),
        record("operation", vec![string("worker-execute")]),
        record("decision", vec![string(input.decision)]),
        record("job", vec![optional_ref_value(job_ref)]),
        record("request", vec![optional_ref_value(request_ref)]),
        record("assignment", vec![string(input.assignment_ref)]),
        record("status", vec![refs_sequence(input.status_refs)]),
        record("result", vec![string(input.result_ref)]),
        record("execution-receipt", vec![optional_ref_value(input.execution_receipt_ref)]),
        record("delivery-log", vec![optional_ref_value(input.delivery_log_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

pub fn job_worker_schedule_receipt_value(input: JobWorkerScheduleReceiptValueInput<'_>) -> Result<IOValue> {
    validate_non_empty(input.operation, "job worker schedule operation")?;
    validate_decision(input.decision)?;
    validate_ref(input.job_ref, "job worker schedule job ref")?;
    validate_ref(input.request_ref, "job worker schedule request ref")?;
    validate_non_empty(input.queue_key, "job worker schedule queue key")?;
    validate_non_empty(input.lease_key, "job worker schedule lease key")?;
    validate_non_empty(input.worker_session, "job worker schedule worker session")?;
    validate_ref(input.coordination_report_ref, "job worker schedule coordination report ref")?;
    validate_present_refs(&optional_refs(&input))?;
    validate_refs(input.refs, "job worker schedule refs")?;
    ensure_count_at_most(input.diagnostics.len(), MAX_JOB_REFS, "job worker schedule diagnostics")?;
    let refs = collected_refs(&input)?;
    let checks = checked_pairs(&input);
    Ok(record("job-worker-schedule-receipt-v1", vec![
        string(JOB_WORKER_SCHEDULE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("job", vec![string(input.job_ref)]),
        record("request", vec![string(input.request_ref)]),
        record("queue-key", vec![string(input.queue_key)]),
        record("lease-key", vec![string(input.lease_key)]),
        record("worker-session", vec![string(input.worker_session)]),
        record("coordination-report", vec![string(input.coordination_report_ref)]),
        record("enqueue", vec![optional_ref_value(input.enqueue_receipt_ref)]),
        record("enqueue-duplicate", vec![optional_ref_value(input.enqueue_duplicate_receipt_ref)]),
        record("dequeue", vec![optional_ref_value(input.dequeue_receipt_ref)]),
        record("lease", vec![optional_ref_value(input.lease_receipt_ref)]),
        record("release", vec![optional_ref_value(input.release_receipt_ref)]),
        record("token", vec![optional_ref_value(input.token_ref)]),
        record("worker-receipt", vec![optional_ref_value(input.worker_receipt_ref)]),
        record("result", vec![optional_ref_value(input.result_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

fn optional_refs<'a>(input: &JobWorkerScheduleReceiptValueInput<'a>) -> [(&'static str, Option<&'a str>); 8] {
    [
        ("enqueue receipt", input.enqueue_receipt_ref),
        ("enqueue duplicate receipt", input.enqueue_duplicate_receipt_ref),
        ("dequeue receipt", input.dequeue_receipt_ref),
        ("lease receipt", input.lease_receipt_ref),
        ("release receipt", input.release_receipt_ref),
        ("token", input.token_ref),
        ("worker receipt", input.worker_receipt_ref),
        ("result", input.result_ref),
    ]
}

fn validate_present_refs(pairs: &[(&str, Option<&str>)]) -> Result<()> {
    for (label, reference) in pairs {
        if let Some(reference) = reference {
            validate_ref(reference, &format!("job worker schedule {label} ref"))?;
        }
    }
    Ok(())
}

fn collected_refs(input: &JobWorkerScheduleReceiptValueInput<'_>) -> Result<Vec<String>> {
    let mut refs = vec![
        input.job_ref.to_string(),
        input.request_ref.to_string(),
        input.coordination_report_ref.to_string(),
    ];
    for (_, reference) in optional_refs(input) {
        if let Some(reference) = reference {
            push_bounded(&mut refs, reference.to_string(), MAX_JOB_REFS, "job worker schedule refs")?;
        }
    }
    extend_cloned_bounded(&mut refs, input.refs, MAX_JOB_REFS, "job worker schedule refs")?;
    Ok(refs)
}

fn checked_pairs<'a>(input: &JobWorkerScheduleReceiptValueInput<'a>) -> Vec<(&'a str, &'a str)> {
    let mut checks = input.checks.to_vec();
    checks.push(("coordination-queue-bound", "pass"));
    checks.push(("coordination-lease-bound", status(input.token_ref.is_some())));
    checks.push(("transport-is-not-authority", "pass"));
    checks.push(("canonical-receipt", "pass"));
    checks
}

fn preflight(submission: &BlobRefJobSubmission) -> Result<(Preflight, Vec<String>)> {
    let preflight = Preflight {
        has_policy: !submission.policy_refs.is_empty(),
        has_provenance: !submission.provenance_refs.is_empty(),
        has_effect_manifest: !submission.effect_manifest_refs.is_empty(),
        has_supported_output_mode: submission.output_mode == "chunk-manifest",
        has_supported_handler: submission.handler_profile == "local-echo-v1",
    };
    let mut diagnostics = Vec::new();
    if !preflight.has_policy {
        push_bounded(
            &mut diagnostics,
            "job ref submission missing policy refs".to_string(),
            MAX_JOB_REFS,
            "job ref diagnostics",
        )?;
    }
    if !preflight.has_provenance {
        push_bounded(
            &mut diagnostics,
            "job ref submission missing executable provenance refs".to_string(),
            MAX_JOB_REFS,
            "job ref diagnostics",
        )?;
    }
    if !preflight.has_effect_manifest {
        push_bounded(
            &mut diagnostics,
            "job ref submission missing effect manifest refs".to_string(),
            MAX_JOB_REFS,
            "job ref diagnostics",
        )?;
    }
    if !preflight.has_supported_output_mode {
        push_bounded(
            &mut diagnostics,
            format!("unsupported job ref output mode {}", submission.output_mode),
            MAX_JOB_REFS,
            "job ref diagnostics",
        )?;
    }
    if !preflight.has_supported_handler {
        push_bounded(
            &mut diagnostics,
            format!("unsupported job ref handler profile {}", submission.handler_profile),
            MAX_JOB_REFS,
            "job ref diagnostics",
        )?;
    }
    Ok((preflight, diagnostics))
}

fn fetch_content(chunk_root: &Path, submission: &BlobRefJobSubmission) -> Result<FetchOutcome> {
    let mut content_refs = vec![submission.executable.clone()];
    extend_cloned_bounded(&mut content_refs, &submission.inputs, MAX_JOB_REFS, "job ref content refs")?;
    let mut input_bytes = Vec::new();
    let mut verify_refs = Vec::new();
    let mut fetch_refs = Vec::new();
    let mut pin_refs = Vec::new();
    let mut diagnostics = Vec::new();
    let mut is_content_verified = true;
    for (content_index, content) in content_refs.iter().enumerate() {
        let fetched = fetch_blob_ref_job_content(chunk_root, content, &mut verify_refs, &mut fetch_refs, &mut pin_refs);
        match fetched {
            Ok(bytes) => {
                if content_index > 0 {
                    push_bounded(&mut input_bytes, bytes, MAX_JOB_REFS, "job ref input byte sets")?;
                }
            }
            Err(error) => {
                is_content_verified = false;
                push_bounded(&mut diagnostics, error.to_string(), MAX_JOB_REFS, "job ref diagnostics")?;
            }
        }
    }
    Ok(FetchOutcome {
        content_refs,
        input_bytes,
        verify_refs,
        fetch_refs,
        pin_refs,
        diagnostics,
        is_content_verified,
    })
}

fn run_output(chunk_root: &Path, submission: &BlobRefJobSubmission, input_bytes: &[Vec<u8>]) -> Result<OutputOutcome> {
    let mut status_values = Vec::new();
    push_bounded(
        &mut status_values,
        blob_ref_job_status_value(submission, "running", &[], &[("content-verified-before-run", "pass")])?,
        MAX_JOB_REFS,
        "job ref status values",
    )?;
    let output_bytes = run_blob_ref_job_handler(submission, input_bytes)?;
    let put = chunk_store::put_bytes(chunk_root, "job-ref-result", &output_bytes, DEFAULT_FIXED_V1_CHUNK_SIZE)?;
    let output_put_ref = canonical_hash(&put.receipt_value)?;
    let output_verify = chunk_store::verify_manifest(chunk_root, &put.manifest_ref)?;
    let verify_ref = canonical_hash(&output_verify.receipt_value)?;
    let output_pin = chunk_store::pin_manifest(chunk_root, &put.manifest_ref)?;
    let pin_ref = canonical_hash(&output_pin.receipt_value)?;
    let output_manifest_ref = put.manifest_ref.clone();
    push_bounded(
        &mut status_values,
        blob_ref_job_status_value(submission, "result-ready", std::slice::from_ref(&output_manifest_ref), &[(
            "output-content-ref",
            "pass",
        )])?,
        MAX_JOB_REFS,
        "job ref status values",
    )?;
    Ok(OutputOutcome {
        output_manifest_ref,
        output_put_ref,
        verify_ref,
        pin_ref,
        status_values,
    })
}

fn cleanup_content(chunk_root: &Path, content_refs: &[JobContentRef]) -> Result<Vec<String>> {
    let mut cleanup_refs = Vec::new();
    for content in content_refs {
        if let Ok(unpin) = chunk_store::unpin_manifest(chunk_root, &content.content_ref) {
            push_bounded(
                &mut cleanup_refs,
                canonical_hash(&unpin.receipt_value)?,
                MAX_JOB_REFS,
                "job ref cleanup refs",
            )?;
        }
    }
    Ok(cleanup_refs)
}

fn finish_run(input: FinishInput<'_>) -> Result<BlobRefJobExecution> {
    let FinishInput {
        ledger_root,
        submission,
        mut status_values,
        verify_refs,
        fetch_refs,
        pin_refs,
        cleanup_refs,
        output_manifest_ref,
        output_put_ref,
        diagnostics,
        preflight,
        is_content_verified,
        has_preliminary_pass,
    } = input;
    let final_decision = if has_preliminary_pass { "pass" } else { "deny" };
    let final_state = if has_preliminary_pass { "complete" } else { "failed" };
    push_bounded(
        &mut status_values,
        blob_ref_job_status_value(&submission, final_state, output_manifest_ref.as_slice(), &[(
            "terminal-status",
            "pass",
        )])?,
        MAX_JOB_REFS,
        "job ref status values",
    )?;
    let status_refs = status_values.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?;
    let receipt_checks = final_checks(
        preflight,
        is_content_verified,
        output_manifest_ref.is_some(),
        !pin_refs.is_empty(),
        !cleanup_refs.is_empty(),
    );
    let receipt_value = blob_ref_job_receipt_value(BlobRefReceiptValueInput {
        decision: final_decision,
        submission: &submission,
        status_refs: &status_refs,
        verify_refs: &verify_refs,
        fetch_refs: &fetch_refs,
        pin_refs: &pin_refs,
        cleanup_refs: &cleanup_refs,
        output_manifest_ref: output_manifest_ref.as_deref(),
        output_put_ref: output_put_ref.as_deref(),
        diagnostics: &diagnostics,
        checks: &receipt_checks,
    })?;
    let receipt_ref = canonical_hash(&receipt_value)?;
    if let Some(ledger_root) = ledger_root {
        import_blob_ref_job_artifacts(ledger_root, &status_values, &receipt_value)?;
    }
    Ok(BlobRefJobExecution {
        submission,
        decision: final_decision.to_string(),
        status_values,
        output_manifest_ref,
        receipt_ref,
        receipt_value,
        diagnostics,
    })
}

fn final_checks(
    preflight: Preflight,
    is_content_verified: bool,
    has_output: bool,
    has_pins: bool,
    has_cleanup: bool,
) -> [(&'static str, &'static str); 10] {
    [
        ("content-refs-only", "pass"),
        ("no-inline-large-bytes", "pass"),
        ("content-verification-before-run", status(is_content_verified)),
        ("provenance-policy", status(preflight.has_policy && preflight.has_provenance)),
        ("effect-admission-policy", status(preflight.has_effect_manifest)),
        ("local-worker-handler", status(preflight.has_supported_handler)),
        ("output-content-ref", status(has_output)),
        ("retention-pins", status(has_pins)),
        ("cleanup-receipts", status(has_cleanup)),
        ("job-dag-ref-integration", "pass"),
    ]
}

fn blob_ref_job_status_value(
    submission: &BlobRefJobSubmission,
    state: &str,
    output_refs: &[String],
    checks: &[(&str, &str)],
) -> Result<IOValue> {
    validate_blob_ref_state(state)?;
    validate_refs(output_refs, "job ref status output ref")?;
    let mut refs = vec![submission.submission_ref.clone(), submission.operation_id.clone()];
    push_bounded(&mut refs, submission.executable.content_ref.clone(), MAX_JOB_REFS, "job ref status refs")?;
    extend_cloned_bounded(&mut refs, output_refs, MAX_JOB_REFS, "job ref status refs")?;
    let mut status_checks = checks.to_vec();
    status_checks.push(("canonical-status", "pass"));
    Ok(record("job-ref-status-v1", vec![
        string(JOB_REF_STATUS_SCHEMA),
        record("submission", vec![string(&submission.submission_ref)]),
        record("job-id", vec![string(&submission.job_id)]),
        record("operation-id", vec![string(&submission.operation_id)]),
        record("state", vec![string(state)]),
        record("outputs", vec![refs_sequence(output_refs)]),
        record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&status_checks),
    ]))
}

fn blob_ref_job_receipt_value(input: BlobRefReceiptValueInput<'_>) -> Result<IOValue> {
    validate_worker_decision(input.decision)?;
    validate_refs(input.status_refs, "job ref receipt status ref")?;
    validate_refs(input.verify_refs, "job ref receipt verify ref")?;
    validate_refs(input.fetch_refs, "job ref receipt fetch ref")?;
    validate_refs(input.pin_refs, "job ref receipt pin ref")?;
    validate_refs(input.cleanup_refs, "job ref receipt cleanup ref")?;
    if let Some(output_manifest_ref) = input.output_manifest_ref {
        validate_ref(output_manifest_ref, "job ref receipt output manifest ref")?;
    }
    if let Some(output_put_ref) = input.output_put_ref {
        validate_ref(output_put_ref, "job ref receipt output put ref")?;
    }
    let mut refs = vec![
        input.submission.submission_ref.clone(),
        input.submission.operation_id.clone(),
        input.submission.executable.content_ref.clone(),
        input.submission.authority_context_ref.clone(),
    ];
    for content in &input.submission.inputs {
        push_bounded(&mut refs, content.content_ref.clone(), MAX_JOB_REFS, "job ref receipt refs")?;
    }
    extend_cloned_bounded(&mut refs, input.status_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, input.verify_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, input.fetch_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, input.pin_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, input.cleanup_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, &input.submission.policy_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, &input.submission.provenance_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    extend_cloned_bounded(&mut refs, &input.submission.evidence_refs, MAX_JOB_REFS, "job ref receipt refs")?;
    if let Some(output_manifest_ref) = input.output_manifest_ref {
        push_bounded(&mut refs, output_manifest_ref.to_string(), MAX_JOB_REFS, "job ref receipt refs")?;
    }
    if let Some(output_put_ref) = input.output_put_ref {
        push_bounded(&mut refs, output_put_ref.to_string(), MAX_JOB_REFS, "job ref receipt refs")?;
    }
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(record("job-ref-receipt-v1", vec![
        string(JOB_REF_RECEIPT_SCHEMA),
        record("operation", vec![string("blob-ref-worker-execute")]),
        record("decision", vec![string(input.decision)]),
        record("submission", vec![string(&input.submission.submission_ref)]),
        record("job-id", vec![string(&input.submission.job_id)]),
        record("operation-id", vec![string(&input.submission.operation_id)]),
        record("executable", vec![string(&input.submission.executable.content_ref)]),
        record("inputs", vec![refs_sequence(
            &input.submission.inputs.iter().map(|content| content.content_ref.clone()).collect::<Vec<_>>(),
        )]),
        record("status", vec![refs_sequence(input.status_refs)]),
        record("verify", vec![refs_sequence(input.verify_refs)]),
        record("fetch", vec![refs_sequence(input.fetch_refs)]),
        record("pins", vec![refs_sequence(input.pin_refs)]),
        record("cleanup", vec![refs_sequence(input.cleanup_refs)]),
        record("output", vec![optional_ref_value(input.output_manifest_ref)]),
        record("output-put", vec![optional_ref_value(input.output_put_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("refs", vec![refs_sequence(&sorted_unique(&refs))]),
        checks_value_from_pairs(&checks),
    ]))
}

fn fetch_blob_ref_job_content(
    chunk_root: &Path,
    content: &JobContentRef,
    verify_refs: &mut impl crate::bounded::VecSink<String>,
    fetch_refs: &mut impl crate::bounded::VecSink<String>,
    pin_refs: &mut impl crate::bounded::VecSink<String>,
) -> Result<Vec<u8>> {
    let manifest = chunk_store::read_manifest(chunk_root, &content.content_ref)?;
    if manifest.total_len != content.size {
        return Err(MoltenError::invalid_harness(format!(
            "job content {} size hint {} does not match manifest length {}",
            content.content_ref, content.size, manifest.total_len
        )));
    }
    let verify = chunk_store::verify_manifest(chunk_root, &content.content_ref)?;
    push_bounded(verify_refs, canonical_hash(&verify.receipt_value)?, MAX_JOB_REFS, "job ref verify refs")?;
    let read = chunk_store::read_object(chunk_root, &content.content_ref)?;
    push_bounded(fetch_refs, canonical_hash(&read.receipt_value)?, MAX_JOB_REFS, "job ref fetch refs")?;
    let pin = chunk_store::pin_manifest(chunk_root, &content.content_ref)?;
    push_bounded(pin_refs, canonical_hash(&pin.receipt_value)?, MAX_JOB_REFS, "job ref pin refs")?;
    Ok(read.bytes)
}

fn run_blob_ref_job_handler(submission: &BlobRefJobSubmission, input_bytes: &[Vec<u8>]) -> Result<Vec<u8>> {
    match submission.handler_profile.as_str() {
        "local-echo-v1" => {
            let total_len = input_bytes.iter().try_fold(0usize, |sum, bytes| {
                checked_count_sum(sum, bytes.len(), MAX_JOB_STAGE_VALUES * MAX_JOB_STAGE_VALUES, "job ref output bytes")
            })?;
            let mut output = Vec::with_capacity(total_len);
            for bytes in input_bytes {
                output.extend_from_slice(bytes);
            }
            Ok(output)
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported job ref handler profile {other}"))),
    }
}

fn import_blob_ref_job_artifacts(ledger_root: &Path, statuses: &[IOValue], receipt_value: &IOValue) -> Result<()> {
    for status_value in statuses {
        ledger::import_artifact(ledger_root, status_value)?;
    }
    ledger::import_artifact(ledger_root, receipt_value)?;
    Ok(())
}

fn push_check(checks: &mut impl crate::bounded::VecSink<(&'static str, &'static str)>, name: &'static str, ok: bool) {
    checks.push_item((name, status(ok)));
}

fn checks_with_extra(
    checks: &[(&'static str, &'static str)],
    extra: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    let mut merged = checks.to_vec();
    merged.extend_from_slice(extra);
    merged
}

struct DeliveryChecks {
    diagnostics: Vec<String>,
    checks: Vec<(&'static str, &'static str)>,
    delivery_log_ref: Option<String>,
    has_recorded_delivery: bool,
    is_preliminary_pass: bool,
}

struct WorkerRun {
    status_values: Vec<IOValue>,
    execution: Option<JobExecutionLoopback>,
}

struct WorkerOutputs {
    execution_receipt_ref: Option<String>,
    output_refs: Vec<String>,
    stage_receipt_refs: Vec<(String, String)>,
    resource_receipt_refs: Vec<String>,
    is_execution_pass: bool,
}

struct FinishDeliveryInput<'a> {
    input: JobWorkerExecuteInput<'a>,
    request: JobWorkerRequest,
    assignment_value: IOValue,
    assignment_ref: String,
    delivery: DeliveryChecks,
    run: WorkerRun,
}

#[derive(Default)]
struct DeliveryCheckBuffers {
    diagnostics: Vec<String>,
    checks: Vec<(&'static str, &'static str)>,
}

impl DeliveryCheckBuffers {
    fn push(&mut self, name: &'static str, ok: bool) {
        push_check(&mut self.checks, name, ok);
    }

    fn note(&mut self, value: impl Into<String>) {
        self.diagnostics.push(value.into());
    }

    fn finish(self, delivery_log_ref: Option<String>, has_recorded_delivery: bool) -> DeliveryChecks {
        let is_preliminary_pass = self
            .checks
            .iter()
            .filter(|(name, _)| *name != "recorded-delivery-log")
            .all(|(_, value)| *value == "pass");
        DeliveryChecks {
            diagnostics: self.diagnostics,
            checks: self.checks,
            delivery_log_ref,
            has_recorded_delivery,
            is_preliminary_pass,
        }
    }
}

fn collect_delivery_checks(input: &JobWorkerExecuteInput<'_>, request: &JobWorkerRequest) -> Result<DeliveryChecks> {
    let mut buffers = DeliveryCheckBuffers::default();
    let (delivery_log_ref, has_recorded_delivery) = push_delivery_checks(input, request, &mut buffers);
    let (execution_request, admission) = push_input_checks(input, request, &mut buffers)?;
    push_binding_checks(request, &execution_request, &admission, &mut buffers);
    push_authority_checks(request, &admission, &mut buffers);
    push_target_state_check(request, &execution_request, &mut buffers)?;
    Ok(buffers.finish(delivery_log_ref, has_recorded_delivery))
}

fn run_worker_delivery(
    input: &JobWorkerExecuteInput<'_>,
    request: &JobWorkerRequest,
    delivery: &DeliveryChecks,
) -> Result<WorkerRun> {
    let mut status_values = vec![job_worker_status_value(WorkerStatusValueInput {
        request,
        delivery: input.delivery,
        state: "received",
        execution_receipt_ref: None,
        diagnostics: &delivery.diagnostics,
        checks: &[("request-envelope-bound", status(delivery.is_preliminary_pass))],
    })?];

    let execution = if delivery.is_preliminary_pass {
        let running_status = job_worker_status_value(WorkerStatusValueInput {
            request,
            delivery: input.delivery,
            state: "running",
            execution_receipt_ref: None,
            diagnostics: &[],
            checks: &[("target-side-admission-verified", "pass")],
        })?;
        push_bounded(&mut status_values, running_status, MAX_JOB_REFS, "job worker statuses")?;
        Some(execution_loopback(ExecutionLoopbackInput {
            target_registry: input.target_registry,
            storage_root: input.storage_root,
            cache_root: input.cache_root,
            chunk_root: input.chunk_root,
            admission_receipt_value: input.admission_receipt_value,
            request_value: input.execution_request_value,
        })?)
    } else {
        None
    };
    Ok(WorkerRun {
        status_values,
        execution,
    })
}

fn finish_worker_delivery(mut input: FinishDeliveryInput<'_>) -> Result<JobWorkerExecution> {
    let mut diagnostics = input.delivery.diagnostics.clone();
    let outputs = worker_outputs(&input.request, input.run.execution.as_ref(), &mut diagnostics)?;
    let final_decision = final_worker_decision(outputs.is_execution_pass, input.delivery.has_recorded_delivery);
    let final_status = final_worker_status(FinalStatusInput {
        delivery: input.input.delivery,
        request: &input.request,
        diagnostics: &diagnostics,
        outputs: &outputs,
        is_preliminary_pass: input.delivery.is_preliminary_pass,
        final_decision,
    })?;
    push_bounded(&mut input.run.status_values, final_status, MAX_JOB_REFS, "job worker statuses")?;

    let result_checks = checks_with_extra(&input.delivery.checks, &[
        ("loopback-execution-verifier", status(input.run.execution.is_some())),
        ("executed-on-target-state", status(outputs.is_execution_pass)),
        ("result-output-binding", status(final_decision != "pass" || !outputs.output_refs.is_empty())),
        ("resource-accounting", status(!input.request.resource_refs.is_empty())),
        ("live-unrecorded-diagnostic", status(final_decision != "non-replayable")),
    ]);
    let result = worker_result(&input, &outputs, &diagnostics, &result_checks)?;
    let receipt = worker_receipt(WorkerReceiptInput {
        input: &input,
        status_values: &input.run.status_values,
        outputs: &outputs,
        diagnostics: &diagnostics,
        result: &result,
        result_checks: &result_checks,
    })?;
    if let Some(ledger_root) = input.input.ledger_root {
        import_worker_artifacts(
            ledger_root,
            &input.assignment_value,
            &input.run.status_values,
            &result.value,
            &receipt.receipt_value,
        )?;
    }
    Ok(JobWorkerExecution {
        request: Some(input.request),
        assignment_value: input.assignment_value,
        status_values: input.run.status_values,
        result,
        receipt_ref: receipt.receipt_ref,
        receipt_value: receipt.receipt_value,
        execution: input.run.execution,
    })
}

fn worker_outputs(
    request: &JobWorkerRequest,
    execution: Option<&JobExecutionLoopback>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<WorkerOutputs> {
    let execution_receipt_ref = execution.map(|execution| execution.receipt_ref.clone());
    let mut output_refs = Vec::new();
    let mut stage_receipt_refs = Vec::new();
    let mut resource_receipt_refs = request.resource_refs.clone();
    push_bounded(
        &mut resource_receipt_refs,
        local_ref("job-worker-resource-accounting", &request.request_ref)?,
        MAX_JOB_REFS,
        "job worker resource receipt refs",
    )?;
    if let Some(execution) = execution {
        if let Some(run) = execution.run.as_ref() {
            output_refs = run.output_refs.clone();
            stage_receipt_refs = worker_stage_receipts(&execution.admission.stage_order, &run.stage_receipt_refs)?;
        }
        diagnostics.extend_cloned_items(&execution.diagnostics);
    }
    Ok(WorkerOutputs {
        execution_receipt_ref,
        output_refs,
        stage_receipt_refs,
        resource_receipt_refs,
        is_execution_pass: execution.is_some_and(|execution| execution.decision == "pass"),
    })
}

fn final_worker_decision(is_execution_pass: bool, has_recorded_delivery: bool) -> &'static str {
    if is_execution_pass && has_recorded_delivery {
        "pass"
    } else if is_execution_pass {
        "non-replayable"
    } else {
        "deny"
    }
}

fn final_worker_status(input: FinalStatusInput<'_>) -> Result<IOValue> {
    let final_state = match input.final_decision {
        "pass" => "completed",
        "non-replayable" => "non-replayable",
        _ => "denied",
    };
    job_worker_status_value(WorkerStatusValueInput {
        request: input.request,
        delivery: input.delivery,
        state: final_state,
        execution_receipt_ref: input.outputs.execution_receipt_ref.as_deref(),
        diagnostics: input.diagnostics,
        checks: &[(
            "no-stage-execution-on-deny",
            status(!input.is_preliminary_pass || input.outputs.is_execution_pass),
        )],
    })
}

struct WorkerReceipt {
    receipt_ref: String,
    receipt_value: IOValue,
}

struct FinalStatusInput<'a> {
    delivery: &'a remote_dataspace::RemoteDataspaceDelivery,
    request: &'a JobWorkerRequest,
    diagnostics: &'a [String],
    outputs: &'a WorkerOutputs,
    is_preliminary_pass: bool,
    final_decision: &'a str,
}

struct WorkerReceiptInput<'a> {
    input: &'a FinishDeliveryInput<'a>,
    status_values: &'a [IOValue],
    outputs: &'a WorkerOutputs,
    diagnostics: &'a [String],
    result: &'a JobWorkerResult,
    result_checks: &'a [(&'static str, &'static str)],
}

fn worker_result(
    input: &FinishDeliveryInput<'_>,
    outputs: &WorkerOutputs,
    diagnostics: &[String],
    result_checks: &[(&'static str, &'static str)],
) -> Result<JobWorkerResult> {
    let final_decision = final_worker_decision(outputs.is_execution_pass, input.delivery.has_recorded_delivery);
    let result_value = job_worker_result_value(WorkerResultValueInput {
        decision: final_decision,
        request: &input.request,
        execution_receipt_ref: outputs.execution_receipt_ref.as_deref(),
        output_refs: &outputs.output_refs,
        stage_receipt_refs: &outputs.stage_receipt_refs,
        resource_receipt_refs: &outputs.resource_receipt_refs,
        delivery_log_ref: input.delivery.delivery_log_ref.as_deref(),
        diagnostics,
        checks: result_checks,
    })?;
    parse_job_worker_result_value(&result_value)
}

fn worker_receipt(input: WorkerReceiptInput<'_>) -> Result<WorkerReceipt> {
    let final_decision =
        final_worker_decision(input.outputs.is_execution_pass, input.input.delivery.has_recorded_delivery);
    let status_refs = input.status_values.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?;
    let receipt_checks = checks_with_extra(input.result_checks, &[
        ("assignment-bound", "pass"),
        ("status-log-bound", "pass"),
        ("worker-result-bound", "pass"),
        ("transport-is-not-authority", "pass"),
    ]);
    let receipt_value = job_worker_receipt_value(WorkerReceiptValueInput {
        decision: final_decision,
        request: Some(&input.input.request),
        assignment_ref: &input.input.assignment_ref,
        status_refs: &status_refs,
        result_ref: &input.result.result_ref,
        execution_receipt_ref: input.outputs.execution_receipt_ref.as_deref(),
        delivery_log_ref: input.input.delivery.delivery_log_ref.as_deref(),
        diagnostics: input.diagnostics,
        checks: &receipt_checks,
    })?;
    Ok(WorkerReceipt {
        receipt_ref: canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

fn push_delivery_checks(
    input: &JobWorkerExecuteInput<'_>,
    request: &JobWorkerRequest,
    buffers: &mut DeliveryCheckBuffers,
) -> (Option<String>, bool) {
    let has_message_operation =
        input.delivery.envelope.operation == remote_dataspace::RemoteDataspaceOperation::Message;
    buffers.push("remote-dataspace-message", has_message_operation);
    if !has_message_operation {
        buffers.note("job worker request was not delivered as a remote dataspace message");
    }

    let has_target_binding = input.delivery.envelope.to_peer == request.target_peer;
    buffers.push("target-peer-binding", has_target_binding);
    if !has_target_binding {
        buffers.note(format!(
            "job worker envelope target {} does not match request target {}",
            input.delivery.envelope.to_peer, request.target_peer
        ));
    }

    let delivery_log_ref = input.delivery_log.map(|log| log.log_ref.clone());
    let has_recorded_delivery = input.delivery_log.is_some_and(|log| {
        log.replayable
            && log.entries.iter().any(|entry| entry.envelope.envelope_ref == input.delivery.envelope.envelope_ref)
    });
    buffers.push("recorded-delivery-log", has_recorded_delivery);
    if !has_recorded_delivery {
        buffers.note("job worker delivery log is missing, non-replayable, or does not bind request envelope");
    }
    (delivery_log_ref, has_recorded_delivery)
}

fn push_input_checks(
    input: &JobWorkerExecuteInput<'_>,
    request: &JobWorkerRequest,
    buffers: &mut DeliveryCheckBuffers,
) -> Result<(JobExecutionRequest, JobAdmissionReceipt)> {
    let execution_request_ref = canonical_hash(input.execution_request_value)?;
    let has_execution_request_ref = execution_request_ref == request.execution_request_ref;
    buffers.push("execution-request-ref-binding", has_execution_request_ref);
    if !has_execution_request_ref {
        buffers.note(format!(
            "job worker execution request hashes to {execution_request_ref}, expected {}",
            request.execution_request_ref
        ));
    }
    let execution_request = parse_job_execution_request_value(input.execution_request_value)?;

    let admission_ref = canonical_hash(input.admission_receipt_value)?;
    let has_admission_ref = admission_ref == request.admission_ref;
    buffers.push("admission-ref-binding", has_admission_ref);
    if !has_admission_ref {
        buffers.note(format!(
            "job worker admission receipt hashes to {admission_ref}, expected {}",
            request.admission_ref
        ));
    }
    let admission = parse_job_admission_receipt_value(input.admission_receipt_value)?;
    Ok((execution_request, admission))
}

fn push_binding_checks(
    request: &JobWorkerRequest,
    execution_request: &JobExecutionRequest,
    admission: &JobAdmissionReceipt,
    buffers: &mut DeliveryCheckBuffers,
) {
    let has_job_binding = request.job_ref == execution_request.job_ref && request.job_ref == admission.job_ref;
    buffers.push("job-ref-binding", has_job_binding);
    if !has_job_binding {
        buffers.note("job worker request, execution request, and admission job refs diverge");
    }

    let has_sync_binding = request.sync_ref == admission.sync_ref;
    buffers.push("sync-ref-binding", has_sync_binding);
    if !has_sync_binding {
        buffers.note(format!(
            "job worker sync ref {} does not match admission sync {}",
            request.sync_ref, admission.sync_ref
        ));
    }

    let has_execution_admission_binding = execution_request.admission_ref == request.admission_ref;
    buffers.push("execution-admission-ref-binding", has_execution_admission_binding);
    if !has_execution_admission_binding {
        buffers.note("job worker execution request does not bind worker admission ref");
    }

    let is_stage_binding = if request.stage_ids.is_empty() {
        execution_request.stage_ids == admission.stage_order
    } else {
        request.stage_ids == execution_request.stage_ids && request.stage_ids == admission.stage_order
    };
    buffers.push("selected-stage-binding", is_stage_binding);
    if !is_stage_binding {
        buffers.note("job worker selected stages do not match execution/admission stage order");
    }

    let has_evidence_refs = request.evidence_refs.iter().any(|reference| reference == &request.sync_ref)
        && request.evidence_refs.iter().any(|reference| reference == &request.admission_ref)
        && request.evidence_refs.iter().any(|reference| reference == &request.execution_request_ref);
    buffers.push("sync-admission-execution-evidence", has_evidence_refs);
    if !has_evidence_refs {
        buffers.note("job worker evidence refs must bind sync, admission, and execution request refs");
    }

    let has_transport_not_authority = !request.peer_bootstrap_refs.is_empty() && !request.node_identity_refs.is_empty();
    buffers.push("peer-bootstrap-node-identity-binding", has_transport_not_authority);
    if !has_transport_not_authority {
        buffers.note("job worker requires peer bootstrap and node identity evidence separate from transport");
    }
}

fn push_authority_checks(
    request: &JobWorkerRequest,
    admission: &JobAdmissionReceipt,
    buffers: &mut DeliveryCheckBuffers,
) {
    let has_authority = !request.authority_refs.is_empty()
        && !admission.authority_receipt_refs.is_empty()
        && refs_are_bound_in_admission(&request.authority_refs, &admission.refs);
    buffers.push("authority-binding", has_authority);
    if !has_authority {
        buffers.note("job worker missing explicit authority refs admitted for job:execute");
    }

    let has_resource = !request.resource_refs.is_empty()
        && admission.resource_verdict == "pass"
        && refs_are_bound_in_admission(&request.resource_refs, &admission.refs);
    buffers.push("resource-binding", has_resource);
    if !has_resource {
        buffers.note("job worker missing admitted resource refs");
    }
}

fn push_target_state_check(
    request: &JobWorkerRequest,
    execution_request: &JobExecutionRequest,
    buffers: &mut DeliveryCheckBuffers,
) -> Result<()> {
    let has_target_state_only = execution_request.target_peer == request.target_peer
        && !to_text(&request.value)?.contains("<source-registry")
        && !to_text(&execution_request.value)?.contains("<source-registry");
    buffers.push("target-state-only", has_target_state_only);
    if !has_target_state_only {
        buffers.note("job worker execution request must run from target roots only");
    }
    Ok(())
}

fn refs_are_bound_in_admission(refs: &[String], admission_refs: &[String]) -> bool {
    refs.iter().all(|reference| admission_refs.iter().any(|admission_ref| admission_ref == reference))
}

fn recompute_execution_closure(target_registry: &Path, dag: &JobDag, stage_order: &[String]) -> Result<Vec<String>> {
    let selected = stage_order.iter().cloned().collect::<BTreeSet<_>>();
    let roots = admission_roots(target_registry, dag, &selected)?;
    let closure = artifacts::dependency_closure(target_registry, &roots)?;
    if !closure.missing_refs.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "job execution target closure missing refs: {}",
            closure.missing_refs.join(",")
        )));
    }
    Ok(closure.closure_refs)
}

fn status(ok: bool) -> &'static str {
    if ok { "pass" } else { "fail" }
}

fn sync_roots(source_registry: &Path, dag: &JobDag, request: &JobSyncRequest) -> Result<Vec<String>> {
    let mut roots = vec![job_artifact_ref(source_registry, &dag.job_ref)?];
    let selected = request.stage_ids.iter().cloned().collect::<BTreeSet<_>>();
    for node in &dag.nodes {
        if (selected.is_empty() || selected.contains(&node.id))
            && let Some(stage_artifact_ref) = node.stage_artifact_ref.as_ref()
        {
            push_bounded(&mut roots, stage_artifact_ref.clone(), MAX_JOB_REFS, "job sync roots")?;
        }
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn sync_install_order(source_registry: &Path, roots: &[String]) -> Result<Vec<String>> {
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for root in roots {
        sync_install_order_visit(source_registry, root, &mut visited, &mut order)?;
    }
    Ok(order)
}

fn sync_install_order_visit(
    source_registry: &Path,
    artifact_ref: &str,
    visited: &mut BTreeSet<String>,
    order: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    validate_ref(artifact_ref, "job sync artifact ref")?;
    let mut pending = Vec::with_capacity(1);
    push_bounded(&mut pending, (artifact_ref.to_string(), false), MAX_JOB_REFS, "job sync install order frames")?;
    while let Some((current_ref, is_exit_frame)) = pending.pop() {
        validate_ref(&current_ref, "job sync artifact ref")?;
        if is_exit_frame {
            push_bounded(order, current_ref, MAX_JOB_REFS, "job sync install order")?;
            continue;
        }
        let is_first_visit = visited.insert(current_ref.clone());
        if is_first_visit {
            let artifact = artifacts::read_artifact(source_registry, &current_ref)?;
            push_bounded(&mut pending, (current_ref, true), MAX_JOB_REFS, "job sync install order frames")?;
            for dependency_ref in artifact.dependency_refs.iter().rev() {
                push_bounded(
                    &mut pending,
                    (dependency_ref.clone(), false),
                    MAX_JOB_REFS,
                    "job sync install order frames",
                )?;
            }
        }
    }
    Ok(())
}

fn request_for_analysis(dag: &JobDag, output_request: Option<&IOValue>) -> Result<JobOutputRequest> {
    if let Some(output_request) = output_request {
        parse_job_output_request_value(output_request, &dag.job_ref)
    } else {
        default_output_request(dag)
    }
}

fn dependency_ids(plan: &TrellisExecutionPlan, node_id: &str) -> Result<Vec<String>> {
    let deps = plan.dependency_indices.get(node_id).cloned().unwrap_or_default();
    let mut ids = Vec::with_capacity(deps.len());
    for dep in deps {
        let dep_index = usize::try_from(dep).map_err(|error| {
            MoltenError::invalid_harness(format!("trellis dependency index cannot convert to usize: {error}"))
        })?;
        let dep_id = plan
            .node_index
            .iter()
            .find_map(|(id, index)| (*index == dep_index).then_some(id.clone()))
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis dependency index {dep_index} has no node")))?;
        ids.push(dep_id);
    }
    ids.sort();
    Ok(ids)
}

fn fusion_edge_sort_key<'a>(
    positions: &BTreeMap<String, usize>,
    edge: &'a JobEdge,
) -> (bool, usize, &'a String, &'a String) {
    match positions.get(&edge.from_node) {
        Some(position) => (false, *position, &edge.from_node, &edge.to_node),
        None => (true, 0, &edge.from_node, &edge.to_node),
    }
}

fn fusion_edge_safe(from: &JobNode, to: &JobNode, edge: &JobEdge) -> bool {
    matches!(from.kind.as_str(), "map" | "filter")
        && matches!(to.kind.as_str(), "map" | "filter")
        && edge.schema_ref.is_none()
        && edge.materialization == "stream"
        && from.effect_manifest_refs.is_empty()
        && to.effect_manifest_refs.is_empty()
        && from.policy_refs.is_empty()
        && to.policy_refs.is_empty()
}

fn analysis_receipt_value(input: AnalysisReceiptValueInput<'_>) -> Result<IOValue> {
    validate_ref(input.job_ref, "job analysis receipt job ref")?;
    validate_ref(input.request_ref, "job analysis receipt request ref")?;
    validate_ref(input.artifact_ref, "job analysis receipt artifact ref")?;
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(record(input.label, vec![
        string(input.schema),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string("pass")]),
        record("job", vec![string(input.job_ref)]),
        record("request", vec![string(input.request_ref)]),
        record("artifact", vec![string(input.artifact_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(&checks),
    ]))
}

pub fn receipt_summary(value: &IOValue) -> Result<String> {
    if let Ok(receipt) = parse_job_worker_schedule_receipt_value(value) {
        return Ok(format!(
            "job worker schedule decision={} job={} request={} queue={} lease={} token={} worker={} result={} diagnostics={}",
            receipt.decision,
            receipt.job_ref,
            receipt.request_ref,
            receipt.queue_key,
            receipt.lease_key,
            receipt.token_ref.unwrap_or_else(|| "-".to_string()),
            receipt.worker_receipt_ref.unwrap_or_else(|| "-".to_string()),
            receipt.result_ref.unwrap_or_else(|| "-".to_string()),
            receipt.diagnostics.join(";")
        ));
    }
    if let Ok(receipt) = parse_job_worker_receipt_value(value) {
        return Ok(format!(
            "job worker receipt decision={} job={} request={} result={} status={} diagnostics={}",
            receipt.decision,
            receipt.job_ref.unwrap_or_else(|| "-".to_string()),
            receipt.request_ref.unwrap_or_else(|| "-".to_string()),
            receipt.result_ref,
            receipt.status_refs.len(),
            receipt.diagnostics.join(";")
        ));
    }
    if let Ok(result) = parse_job_worker_result_value(value) {
        return Ok(format!(
            "job worker result decision={} job={} target={} execution={} outputs={} diagnostics={}",
            result.decision,
            result.job_ref,
            result.target_peer,
            result.execution_receipt_ref.unwrap_or_else(|| "-".to_string()),
            result.output_refs.len(),
            result.diagnostics.join(";")
        ));
    }
    let receipt = parse_job_receipt(value).or_else(|_| parse_blob_ref_job_receipt_value(value))?;
    Ok(format!(
        "job receipt operation={} decision={} job={} request={} stage={} outputs={}",
        receipt.operation,
        receipt.decision,
        receipt.job_ref.unwrap_or_else(|| "-".to_string()),
        receipt.request_ref.unwrap_or_else(|| "-".to_string()),
        receipt.stage_id.unwrap_or_else(|| "-".to_string()),
        receipt.output_refs.len()
    ))
}

pub fn dag_summary(dag: &JobDag) -> String {
    format!(
        "job dag {} nodes={} edges={} outputs={}",
        dag.job_ref,
        dag.nodes.len(),
        dag.edges.len(),
        dag.output_roots.join(",")
    )
}

struct StageMemo<'a> {
    dag: &'a JobDag,
    request: &'a JobOutputRequest,
    node: &'a JobNode,
    inputs: &'a [IOValue],
    cache_root: &'a Path,
    key_input: &'a eval_cache::EvalCacheKeyInput,
    key_ref: &'a str,
}

fn run_stage_with_cache(
    dag: &JobDag,
    request: &JobOutputRequest,
    node: &JobNode,
    inputs: &[IOValue],
    options: &JobRunOptions<'_>,
) -> Result<JobStageRun> {
    let is_cacheable = node.kind != "materialize";
    let key_input = stage_cache_key_input(dag, request, node, inputs)?;
    let key_value = eval_cache::eval_cache_key_value(&key_input)?;
    let key = eval_cache::parse_eval_cache_key(&key_value)?;
    let memo = StageMemo {
        dag,
        request,
        node,
        inputs,
        cache_root: options.cache_root,
        key_input: &key_input,
        key_ref: &key.key_ref,
    };
    if is_cacheable && let Some(hit) = stage_memo_hit(&memo)? {
        return Ok(hit);
    }
    let stage = execute_stage(dag, request, node, inputs, options)?;
    if is_cacheable {
        stage_memo_store(&memo, stage)
    } else {
        Ok(stage)
    }
}

fn stage_memo_hit(input: &StageMemo<'_>) -> Result<Option<JobStageRun>> {
    let current_policy_refs = combined_policy_refs(input.dag, input.request, Some(input.node));
    if let Ok(hit) = eval_cache::get(input.cache_root, input.key_ref, &eval_cache::EvalCacheGetInput {
        current_policy_refs,
        current_capability_refs: Vec::new(),
        current_revocation_refs: Vec::new(),
        semantic: true,
    }) && let Some(output) = hit.output
    {
        let output_values = parse_cached_stage_output(&output)?;
        let output_refs = refs_for_values(&output_values)?;
        let cache_ref = canonical_hash(&hit.receipt_value)?;
        let receipt_value = job_receipt_value(JobReceiptInput {
            operation: "memo-hit",
            decision: "pass",
            job_ref: Some(&input.dag.job_ref),
            request_ref: Some(&input.request.request_ref),
            stage_id: Some(&input.node.id),
            input_refs: &refs_for_values(input.inputs)?,
            output_refs: &output_refs,
            cache_ref: Some(&cache_ref),
            effect_refs: &[],
            policy_refs: &combined_policy_refs(input.dag, input.request, Some(input.node)),
            evidence_refs: std::slice::from_ref(&cache_ref),
            diagnostics: &[],
            checks: &[
                ("eval-cache-hit", "pass"),
                ("memo-key-bound", "pass"),
                ("policy-current-revalidation", "pass"),
            ],
        })?;
        return Ok(Some(JobStageRun {
            node_id: input.node.id.clone(),
            output_values,
            output_refs,
            receipt_value,
        }));
    }
    Ok(None)
}

fn stage_memo_store(input: &StageMemo<'_>, stage: JobStageRun) -> Result<JobStageRun> {
    let stage_output = sequence(stage.output_values.clone());
    let policy_refs = combined_policy_refs(input.dag, input.request, Some(input.node));
    let tier = if policy_refs.is_empty() {
        eval_cache::TIER_PURE
    } else {
        eval_cache::TIER_POLICY_CURRENT
    };
    let cache_put = eval_cache::put(input.cache_root, input.key_input, &eval_cache::EvalCacheValueInput {
        tier: tier.to_string(),
        status: eval_cache::STATUS_PASS.to_string(),
        output: Some(stage_output),
        dependency_refs: input.key_input.dependency_refs.clone(),
        policy_refs: policy_refs.clone(),
        evidence_refs: vec![canonical_hash(&stage.receipt_value)?],
        diagnostics: Vec::new(),
    })?;
    let cache_ref = canonical_hash(&cache_put.receipt_value)?;
    let stage_receipt_ref = canonical_hash(&stage.receipt_value)?;
    let evidence_refs = vec![stage_receipt_ref, cache_ref.clone()];
    let receipt_value = job_receipt_value(JobReceiptInput {
        operation: "stage",
        decision: "pass",
        job_ref: Some(&input.dag.job_ref),
        request_ref: Some(&input.request.request_ref),
        stage_id: Some(&input.node.id),
        input_refs: &refs_for_values(input.inputs)?,
        output_refs: &stage.output_refs,
        cache_ref: Some(&cache_ref),
        effect_refs: &[],
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
        diagnostics: &[],
        checks: &[
            ("eval-cache-miss", "pass"),
            ("stage-executed", "pass"),
            ("memo-key-bound", "pass"),
        ],
    })?;
    Ok(JobStageRun { receipt_value, ..stage })
}

fn execute_stage(
    dag: &JobDag,
    request: &JobOutputRequest,
    node: &JobNode,
    inputs: &[IOValue],
    options: &JobRunOptions<'_>,
) -> Result<JobStageRun> {
    let mut effects = Vec::new();
    let output_values = match node.kind.as_str() {
        "source" => execute_source(node, options, &mut effects)?,
        "map" => execute_map(node, inputs)?,
        "filter" => execute_filter(node, inputs)?,
        "reduce" => execute_reduce(node, inputs)?,
        "materialize" => execute_materialize(node, inputs, options, &mut effects)?,
        _ => return Err(MoltenError::invalid_harness(format!("unsupported job stage kind {}", node.kind))),
    };
    let output_refs = refs_for_values(&output_values)?;
    let receipt_value = job_receipt_value(JobReceiptInput {
        operation: if node.kind == "materialize" {
            "materialize"
        } else {
            "stage"
        },
        decision: "pass",
        job_ref: Some(&dag.job_ref),
        request_ref: Some(&request.request_ref),
        stage_id: Some(&node.id),
        input_refs: &refs_for_values(inputs)?,
        output_refs: &output_refs,
        cache_ref: None,
        effect_refs: &effects,
        policy_refs: &combined_policy_refs(dag, request, Some(node)),
        evidence_refs: &node.evidence_refs,
        diagnostics: &[],
        checks: &[
            ("deterministic-stage", "pass"),
            ("explicit-effect-boundary", "pass"),
            ("no-mobile-closures", "pass"),
        ],
    })?;
    Ok(JobStageRun {
        node_id: node.id.clone(),
        output_values,
        output_refs,
        receipt_value,
    })
}

fn execute_source(
    node: &JobNode,
    options: &JobRunOptions<'_>,
    effects: &mut impl crate::bounded::VecSink<String>,
) -> Result<Vec<IOValue>> {
    let source = simple_record(&node.config, "source", 1)?;
    let payload = value_to_iovalue(&source[0]);
    if let Some(values) = payload.collect_simple_record("values", Some(1)) {
        return sequence_items(&values[0], "source values");
    }
    if let Some(value) = payload.collect_simple_record("value", Some(1)) {
        return Ok(vec![value_to_iovalue(&value[0])]);
    }
    if let Some(typed) = payload.collect_simple_record("typed-storage", Some(3)) {
        let namespace = required_string(&typed[0], "source typed storage namespace")?;
        let key = required_string(&typed[1], "source typed storage key")?;
        let schema_ref = parse_optional_ref_value(&typed[2])?;
        let admission = typed_storage::TypedStorageAdmission::local_fixture(&format!("job:{}:{}", namespace, key));
        let get = typed_storage::get_value(options.storage_root, &namespace, &key, schema_ref.as_deref(), &admission)?;
        effects.push_item(canonical_hash(&get.receipt_value)?);
        return Ok(vec![get.value]);
    }
    if let Some(chunk) = payload.collect_simple_record("chunk-manifest", Some(1)) {
        let manifest_ref = required_ref(&chunk[0], "source chunk manifest ref")?;
        let read = chunk_store::read_object(options.chunk_root, &manifest_ref)?;
        effects.push_item(canonical_hash(&read.receipt_value)?);
        let value = crate::preserves_rail::parse_canonical_bytes(&read.bytes)?;
        if let Some(items) = value.collect_sequence() {
            ensure_count_at_most(items.len(), MAX_JOB_STAGE_VALUES, "source chunk values")?;
            let mut output = Vec::with_capacity(items.len());
            for item in items.iter() {
                push_bounded(&mut output, value_to_iovalue(item), MAX_JOB_STAGE_VALUES, "source chunk values")?;
            }
            return Ok(output);
        }
        return Ok(vec![value]);
    }
    Err(MoltenError::invalid_harness(
        "unsupported source config; expected <source <values [...]>>, <source <value ...>>, <source <typed-storage ...>>, or <source <chunk-manifest ...>>",
    ))
}

fn execute_map(node: &JobNode, inputs: &[IOValue]) -> Result<Vec<IOValue>> {
    let op = stage_operation(&node.config)?;
    ensure_count_at_most(inputs.len(), MAX_JOB_STAGE_VALUES, "map input values")?;
    let mut output = Vec::with_capacity(inputs.len());
    for value in inputs {
        push_bounded(&mut output, apply_map_op(&op, value)?, MAX_JOB_STAGE_VALUES, "map output values")?;
    }
    Ok(output)
}

fn execute_filter(node: &JobNode, inputs: &[IOValue]) -> Result<Vec<IOValue>> {
    let op = stage_operation(&node.config)?;
    let mut output = Vec::new();
    for value in inputs {
        if apply_filter_op(&op, value)? {
            push_bounded(&mut output, value.clone(), MAX_JOB_STAGE_VALUES, "filter output values")?;
        }
    }
    Ok(output)
}

fn execute_reduce(node: &JobNode, inputs: &[IOValue]) -> Result<Vec<IOValue>> {
    let op = stage_operation(&node.config)?;
    match op.name.as_str() {
        "count" => Ok(vec![u64_value(inputs.len() as u64)]),
        "sum-u64" | "sum-integers" => {
            let mut sum = 0_u64;
            for value in inputs {
                sum = sum
                    .checked_add(required_u64_value(value, "sum-u64 input")?)
                    .ok_or_else(|| MoltenError::invalid_harness("sum-u64 reducer overflowed u64"))?;
            }
            Ok(vec![u64_value(sum)])
        }
        "concat-lists" => {
            let mut values = Vec::new();
            for value in inputs {
                if let Some(items) = value.collect_sequence() {
                    for item in items.iter() {
                        push_bounded(
                            &mut values,
                            value_to_iovalue(item),
                            MAX_JOB_STAGE_VALUES,
                            "concat-list output values",
                        )?;
                    }
                } else {
                    return Err(MoltenError::invalid_harness("concat-lists reducer requires sequence inputs"));
                }
            }
            Ok(vec![sequence(values)])
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported reduce operation {other}"))),
    }
}

fn execute_materialize(
    node: &JobNode,
    inputs: &[IOValue],
    options: &JobRunOptions<'_>,
    effects: &mut impl crate::bounded::VecSink<String>,
) -> Result<Vec<IOValue>> {
    let config = materialize_config(&node.config)?;
    let value = sequence(inputs.to_vec());
    match config.kind.as_str() {
        "inline" => Ok(vec![value]),
        "typed-storage" => {
            let namespace = config
                .namespace
                .ok_or_else(|| MoltenError::invalid_harness("typed-storage materialization requires namespace"))?;
            let key = config
                .key
                .ok_or_else(|| MoltenError::invalid_harness("typed-storage materialization requires key"))?;
            let admission = typed_storage::TypedStorageAdmission::local_fixture(&format!("job:{namespace}:{key}"));
            let put = typed_storage::put_value(options.storage_root, &typed_storage::TypedStoragePutInput {
                namespace,
                key,
                schema_ref: None,
                value,
                producer_ref: local_ref("job-materialize-producer", &node.id)?,
                policy_refs: node.policy_refs.clone(),
                evidence_refs: node.evidence_refs.clone(),
                admission,
            })?;
            effects.push_item(canonical_hash(&put.receipt_value)?);
            Ok(vec![put.typed_ref_value])
        }
        "chunk-manifest" => {
            let bytes = canonical_bytes(&value)?;
            let put =
                chunk_store::put_bytes(options.chunk_root, "job-materialization", &bytes, DEFAULT_FIXED_V1_CHUNK_SIZE)?;
            effects.push_item(canonical_hash(&put.receipt_value)?);
            Ok(vec![record("chunk-manifest-ref", vec![string(&put.manifest_ref)])])
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported materialization kind {other}"))),
    }
}

#[derive(Debug, Clone)]
struct StageOperation {
    name: String,
    argument: Option<IOValue>,
}

fn stage_operation(config: &IOValue) -> Result<StageOperation> {
    if let Ok(fields) = simple_record(config, "op", 2) {
        let name = required_string(&fields[0], "stage operation")?;
        validate_stage_operation(&name)?;
        return Ok(StageOperation {
            name,
            argument: Some(value_to_iovalue(&fields[1])),
        });
    }
    let fields = simple_record(config, "op", 1)?;
    let name = required_string(&fields[0], "stage operation")?;
    validate_stage_operation(&name)?;
    Ok(StageOperation { name, argument: None })
}

fn apply_map_op(op: &StageOperation, value: &IOValue) -> Result<IOValue> {
    match op.name.as_str() {
        "identity" => Ok(value.clone()),
        "wrap" | "tag-record" => {
            let label = op
                .argument
                .as_ref()
                .ok_or_else(|| MoltenError::invalid_harness("wrap operation requires label argument"))?
                .as_string()
                .map(|value| value.into_owned())
                .ok_or_else(|| MoltenError::invalid_harness("wrap operation label must be a string"))?;
            Ok(record("wrapped", vec![string(&label), value.clone()]))
        }
        "project-field" => {
            let label = op
                .argument
                .as_ref()
                .ok_or_else(|| MoltenError::invalid_harness("project-field operation requires label argument"))?
                .as_string()
                .map(|value| value.into_owned())
                .ok_or_else(|| MoltenError::invalid_harness("project-field label must be a string"))?;
            if let Some(fields) = value.collect_simple_record(&label, Some(1)) {
                Ok(value_to_iovalue(&fields[0]))
            } else {
                Err(MoltenError::invalid_harness(format!("project-field did not match record label {label}")))
            }
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported map operation {other}"))),
    }
}

fn apply_filter_op(op: &StageOperation, value: &IOValue) -> Result<bool> {
    match op.name.as_str() {
        "keep-all" => Ok(true),
        "drop-all" => Ok(false),
        "equals" => Ok(op.argument.as_ref().is_some_and(|expected| expected == value)),
        "match-record" => {
            let label = op
                .argument
                .as_ref()
                .ok_or_else(|| MoltenError::invalid_harness("match-record operation requires label argument"))?
                .as_string()
                .map(|value| value.into_owned())
                .ok_or_else(|| MoltenError::invalid_harness("match-record label must be a string"))?;
            Ok(value.collect_simple_record(&label, None).is_some())
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported filter operation {other}"))),
    }
}

#[derive(Debug, Clone)]
struct MaterializeConfig {
    kind: String,
    namespace: Option<String>,
    key: Option<String>,
}

fn materialize_config(config: &IOValue) -> Result<MaterializeConfig> {
    if let Ok(fields) = simple_record(config, "materialize", 3) {
        let kind = required_string(&fields[0], "materialize kind")?;
        validate_request_materialization(&kind)?;
        return Ok(MaterializeConfig {
            kind,
            namespace: Some(required_string(&fields[1], "materialize namespace")?),
            key: Some(required_string(&fields[2], "materialize key")?),
        });
    }
    let fields = simple_record(config, "materialize", 1)?;
    let kind = required_string(&fields[0], "materialize kind")?;
    validate_request_materialization(&kind)?;
    Ok(MaterializeConfig {
        kind,
        namespace: None,
        key: None,
    })
}

fn default_output_request(dag: &JobDag) -> Result<JobOutputRequest> {
    let roots = if dag.output_roots.is_empty() {
        sink_nodes(dag)?
    } else {
        dag.output_roots.clone()
    };
    let value = job_output_request_value(OutputRequestValueInput {
        dag_ref: &dag.job_ref,
        roots: &roots,
        materialization: "inline",
        policy_refs: &dag.policy_refs,
        handler_profile_ref: None,
        seed_config_ref: None,
    })?;
    parse_job_output_request_value(&value, &dag.job_ref)
}

fn stage_cache_key_input(
    dag: &JobDag,
    request: &JobOutputRequest,
    node: &JobNode,
    inputs: &[IOValue],
) -> Result<eval_cache::EvalCacheKeyInput> {
    let input_refs = refs_for_values(inputs)?;
    let stage_artifact_ref = stage_artifact_or_builtin_ref(node)?;
    let dependency_capacity = 3usize
        .saturating_add(input_refs.len())
        .saturating_add(node.stage_artifact_ref.iter().count())
        .saturating_add(node.effect_manifest_refs.len())
        .saturating_add(node.evidence_refs.len());
    ensure_count_at_most(dependency_capacity, MAX_JOB_REFS, "job stage dependency refs")?;
    let mut dependency_refs = Vec::with_capacity(dependency_capacity);
    dependency_refs.push(dag.job_ref.clone());
    dependency_refs.push(request.request_ref.clone());
    dependency_refs.push(stage_artifact_ref.clone());
    dependency_refs.extend(input_refs);
    if let Some(stage_artifact_ref) = node.stage_artifact_ref.as_ref() {
        dependency_refs.push(stage_artifact_ref.clone());
    }
    dependency_refs.extend(node.effect_manifest_refs.iter().cloned());
    dependency_refs.extend(node.evidence_refs.iter().cloned());
    let dependency_refs = sorted_unique(&dependency_refs);
    let dependency_closure_hash =
        canonical_hash(&record("job-stage-dependency-closure", vec![refs_sequence(&dependency_refs)]))?;
    let input_ref = canonical_hash(&record("job-stage-input-v1", vec![
        record("job", vec![string(&dag.job_ref)]),
        record("request", vec![string(&request.request_ref)]),
        record("stage", vec![string(&node.id)]),
        record("stage-artifact", vec![string(&stage_artifact_ref)]),
        record("inputs", vec![sequence(inputs.to_vec())]),
        record("config", vec![node.config.clone()]),
    ]))?;
    let assumption_capacity = dag
        .schema_refs
        .len()
        .saturating_add(node.effect_manifest_refs.len())
        .saturating_add(request.seed_config_ref.iter().count())
        .saturating_add(1);
    ensure_count_at_most(assumption_capacity, MAX_JOB_REFS, "job stage assumption refs")?;
    let mut assumptions = Vec::with_capacity(assumption_capacity);
    assumptions.extend(dag.schema_refs.iter().cloned());
    assumptions.extend(node.effect_manifest_refs.iter().cloned());
    assumptions.extend(request.seed_config_ref.iter().cloned());
    assumptions.push(canonical_hash(&node.config)?);
    Ok(eval_cache::EvalCacheKeyInput {
        operation: JOB_CACHE_OPERATION.to_string(),
        version: "v1".to_string(),
        input_ref,
        dependency_closure_hash,
        dependency_refs,
        handler_profile_ref: request.handler_profile_ref.clone(),
        policy_refs: combined_policy_refs(dag, request, Some(node)),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: job_tool_ref()?,
        tool_version: JOB_TOOL_VERSION.to_string(),
        assumption_refs: sorted_unique(&assumptions),
    })
}

fn stage_artifact_or_builtin_ref(node: &JobNode) -> Result<String> {
    if let Some(stage_artifact_ref) = node.stage_artifact_ref.as_ref() {
        return Ok(stage_artifact_ref.clone());
    }
    match node.kind.as_str() {
        "source" => builtin_stage_operation_ref("source"),
        "materialize" => builtin_stage_operation_ref("materialize"),
        "map" | "filter" | "reduce" => builtin_stage_operation_ref(&stage_operation(&node.config)?.name),
        other => Err(MoltenError::invalid_harness(format!("unsupported stage kind {other}"))),
    }
}

fn job_tool_ref() -> Result<String> {
    canonical_hash(&record("job-dag-tool-v1", vec![string("molten-job-dag"), string(JOB_TOOL_VERSION)]))
}

struct JobReceiptInput<'a> {
    operation: &'a str,
    decision: &'a str,
    job_ref: Option<&'a str>,
    request_ref: Option<&'a str>,
    stage_id: Option<&'a str>,
    input_refs: &'a [String],
    output_refs: &'a [String],
    cache_ref: Option<&'a str>,
    effect_refs: &'a [String],
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

fn job_receipt_value(input: JobReceiptInput<'_>) -> Result<IOValue> {
    validate_receipt_operation(input.operation)?;
    validate_decision(input.decision)?;
    if let Some(job_ref) = input.job_ref {
        validate_ref(job_ref, "job receipt job ref")?;
    }
    if let Some(request_ref) = input.request_ref {
        validate_ref(request_ref, "job receipt request ref")?;
    }
    if let Some(stage_id) = input.stage_id {
        validate_node_id(stage_id)?;
    }
    validate_refs(input.input_refs, "job receipt input ref")?;
    validate_refs(input.output_refs, "job receipt output ref")?;
    if let Some(cache_ref) = input.cache_ref {
        validate_ref(cache_ref, "job receipt cache ref")?;
    }
    validate_refs(input.effect_refs, "job receipt effect ref")?;
    validate_refs(input.policy_refs, "job receipt policy ref")?;
    validate_refs(input.evidence_refs, "job receipt evidence ref")?;
    let mut checks = input.checks.to_vec();
    checks.push(("canonical-receipt", "pass"));
    Ok(record("job-dag-receipt-v1", vec![
        string(JOB_DAG_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("job", vec![optional_ref_value(input.job_ref)]),
        record("request", vec![optional_ref_value(input.request_ref)]),
        record("stage", vec![optional_string_value(input.stage_id)]),
        record("inputs", vec![refs_sequence(&sorted_unique(input.input_refs))]),
        record("outputs", vec![refs_sequence(&sorted_unique(input.output_refs))]),
        record("cache", vec![optional_ref_value(input.cache_ref)]),
        record("effects", vec![refs_sequence(&sorted_unique(input.effect_refs))]),
        record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(&checks),
    ]))
}

fn parse_node_sequence(value: &Value<IOValue>) -> Result<Vec<JobNode>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, "nodes", 1)?;
    let items = required_sequence(&record[0], "job nodes")?;
    ensure_count_at_most(items.len(), MAX_JOB_NODES, "job nodes")?;
    let mut nodes = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut nodes, parse_job_node_value(&value_to_iovalue(item))?, MAX_JOB_NODES, "job nodes")?;
    }
    Ok(nodes)
}

fn parse_job_node_value(value: &IOValue) -> Result<JobNode> {
    let fields = value
        .collect_simple_record("job-node-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-node-v1 ...>"))?;
    require_schema(&fields[0], JOB_DAG_NODE_SCHEMA, "job node")?;
    let id = record_string(&fields[1], "id")?;
    validate_node_id(&id)?;
    let kind = record_string(&fields[2], "kind")?;
    validate_stage_kind(&kind)?;
    let stage_artifact_ref = record_optional_ref(&fields[3], "stage-artifact")?;
    let input_ports = record_port_sequence(&fields[4], "inputs")?;
    let output_ports = record_port_sequence(&fields[5], "outputs")?;
    let config = record_iovalue(&fields[6], "config")?;
    reject_mobile_closure_config(&config)?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "stage-artifact-not-closure", "job node")?;
    Ok(JobNode {
        id,
        kind,
        stage_artifact_ref,
        input_ports,
        output_ports,
        config,
        effect_manifest_refs: record_ref_sequence(&fields[7], "effects")?,
        policy_refs: record_ref_sequence(&fields[8], "policy")?,
        evidence_refs: record_ref_sequence(&fields[9], "evidence")?,
        checks,
    })
}

fn parse_edge_sequence(value: &Value<IOValue>) -> Result<Vec<JobEdge>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, "edges", 1)?;
    let items = required_sequence(&record[0], "job edges")?;
    ensure_count_at_most(items.len(), MAX_JOB_EDGES, "job edges")?;
    let mut edges = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut edges, parse_job_edge_value(&value_to_iovalue(item))?, MAX_JOB_EDGES, "job edges")?;
    }
    Ok(edges)
}

fn parse_job_edge_value(value: &IOValue) -> Result<JobEdge> {
    let fields = value
        .collect_simple_record("job-edge-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-edge-v1 ...>"))?;
    require_schema(&fields[0], JOB_DAG_EDGE_SCHEMA, "job edge")?;
    let from = value_to_iovalue(&fields[1]);
    let from_fields = simple_record(&from, "from", 2)?;
    let to = value_to_iovalue(&fields[2]);
    let to_fields = simple_record(&to, "to", 2)?;
    let partitioning = record_string(&fields[4], "partitioning")?;
    let materialization = record_string(&fields[5], "materialization")?;
    validate_partitioning(&partitioning)?;
    validate_materialization(&materialization)?;
    Ok(JobEdge {
        from_node: required_string(&from_fields[0], "edge from node")?,
        from_port: required_string(&from_fields[1], "edge from port")?,
        to_node: required_string(&to_fields[0], "edge to node")?,
        to_port: required_string(&to_fields[1], "edge to port")?,
        schema_ref: record_optional_ref(&fields[3], "schema")?,
        partitioning,
        materialization,
    })
}

fn validate_topology(nodes: &[JobNode], edges: &[JobEdge]) -> Result<()> {
    execution_order(nodes, edges).map(|_| ())
}

fn execution_order(nodes: &[JobNode], edges: &[JobEdge]) -> Result<Vec<String>> {
    Ok(trellis_execution_plan(nodes, edges)?.order_ids)
}

fn trellis_execution_plan(nodes: &[JobNode], edges: &[JobEdge]) -> Result<TrellisExecutionPlan> {
    let mapping = plan_mapping(nodes, edges)?;
    if mapping.node_ids.len().checked_add(mapping.edges.len()).is_none() {
        return Err(MoltenError::invalid_harness("job dag trellis mapping exceeds topo-sort size precondition"));
    }
    let order_ids = plan_order_ids(&mapping.edges, &mapping.node_ids)?;
    let dependency_indices = plan_dependency_indices(&mapping.edges, &mapping.node_ids)?;
    Ok(TrellisExecutionPlan {
        order_ids,
        node_index: mapping.node_index,
        dependency_indices,
    })
}

fn plan_mapping(nodes: &[JobNode], edges: &[JobEdge]) -> Result<PlanMapping> {
    ensure_count_at_most(nodes.len(), MAX_JOB_NODES, "trellis nodes")?;
    ensure_count_at_most(edges.len(), MAX_JOB_EDGES, "trellis edges")?;
    let mut node_ids = Vec::with_capacity(nodes.len());
    for node in nodes {
        push_bounded(&mut node_ids, node.id.clone(), MAX_JOB_NODES, "trellis node ids")?;
    }
    node_ids.sort();
    node_ids.dedup();
    if node_ids.len() != nodes.len() {
        return Err(MoltenError::invalid_harness("job dag has duplicate node ids before trellis mapping"));
    }
    let mut node_index = BTreeMap::new();
    for (index, node) in node_ids.iter().enumerate() {
        insert_bounded(&mut node_index, node.clone(), index, MAX_JOB_NODES, "trellis node index")?;
    }
    let mut mapped_edges = Vec::with_capacity(edges.len());
    for edge in edges {
        let from = *node_index.get(&edge.from_node).ok_or_else(|| {
            MoltenError::invalid_harness(format!("trellis edge from unknown node {}", edge.from_node))
        })?;
        let to = *node_index
            .get(&edge.to_node)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis edge to unknown node {}", edge.to_node)))?;
        push_bounded(&mut mapped_edges, (from, to), MAX_JOB_EDGES, "trellis edges")?;
    }
    mapped_edges.sort();
    Ok(PlanMapping {
        node_ids,
        node_index,
        edges: mapped_edges,
    })
}

fn plan_order_ids(edges: &[(usize, usize)], node_ids: &[String]) -> Result<Vec<String>> {
    let Some(order_indices) = trellis::topo_sort::topo_sort(edges, node_ids.len()) else {
        return Err(MoltenError::invalid_harness("trellis topo_sort rejected cyclic job dag"));
    };
    if !trellis::topo_sort::is_topo_order(edges, node_ids.len(), &order_indices) {
        return Err(MoltenError::invalid_harness("trellis topo_sort produced invalid job order"));
    }
    let mut order_ids = Vec::with_capacity(order_indices.len());
    for index in &order_indices {
        let node_id = node_ids
            .get(*index)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis topo index {index} outside node set")))?;
        push_bounded(&mut order_ids, node_id.clone(), MAX_JOB_NODES, "trellis order ids")?;
    }
    Ok(order_ids)
}

fn plan_dependency_indices(edges: &[(usize, usize)], node_ids: &[String]) -> Result<BTreeMap<String, Vec<u64>>> {
    let incoming_counts = trellis_incoming_counts(edges, node_ids.len())?;
    let mut dependency_indices = BTreeMap::new();
    for (index, node_id) in node_ids.iter().enumerate() {
        insert_bounded(
            &mut dependency_indices,
            node_id.clone(),
            Vec::with_capacity(incoming_counts[index]),
            MAX_JOB_NODES,
            "trellis dependency index",
        )?;
    }
    for (from, to) in edges {
        let to_node = node_ids
            .get(*to)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis dependency index {to} outside node set")))?;
        let dependency_values = dependency_indices
            .get_mut(to_node)
            .ok_or_else(|| MoltenError::invalid_harness(format!("dependency vector missing for {to_node}")))?;
        push_bounded(
            dependency_values,
            usize_to_u64(*from, "trellis dependency index")?,
            MAX_JOB_EDGES,
            "trellis dependency refs",
        )?;
    }
    for deps in dependency_indices.values_mut() {
        deps.sort();
        deps.dedup();
    }
    Ok(dependency_indices)
}

fn trellis_incoming_counts(trellis_edges: &[(usize, usize)], node_count: usize) -> Result<Vec<usize>> {
    ensure_count_at_most(node_count, MAX_JOB_NODES, "trellis incoming nodes")?;
    let mut counts = vec![0usize; node_count];
    for (_, to) in trellis_edges {
        let count = counts
            .get_mut(*to)
            .ok_or_else(|| MoltenError::invalid_harness(format!("trellis edge target {to} outside node set")))?;
        *count = count
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("trellis incoming edge count overflow"))?;
        ensure_count_at_most(*count, MAX_JOB_EDGES, "trellis incoming edges")?;
    }
    Ok(counts)
}

fn find_job_node<'a>(nodes: &'a [JobNode], node_id: &str) -> Result<&'a JobNode> {
    for node in nodes {
        if node.id == node_id {
            return Ok(node);
        }
    }
    Err(MoltenError::invalid_harness(format!("job node {node_id} missing from node set")))
}

fn gather_inputs(
    node: &JobNode,
    edges: &[JobEdge],
    outputs_by_index: &[Option<Vec<IOValue>>],
    node_index: &BTreeMap<String, usize>,
) -> Result<Vec<IOValue>> {
    ensure_count_at_most(edges.len(), MAX_JOB_EDGES, "job input edges")?;
    let mut incoming = Vec::with_capacity(edges.len());
    for edge in edges {
        if edge.to_node == node.id {
            push_bounded(&mut incoming, edge, MAX_JOB_EDGES, "job incoming edges")?;
        }
    }
    incoming.sort_by(|left, right| {
        (&left.to_port, &left.from_node, &left.from_port).cmp(&(&right.to_port, &right.from_node, &right.from_port))
    });
    let mut value_count = 0usize;
    for edge in &incoming {
        let from_values = indexed_stage_outputs(outputs_by_index, node_index, &edge.from_node)?;
        value_count = checked_count_sum(value_count, from_values.len(), MAX_JOB_STAGE_VALUES, "job input values")?;
    }
    let mut values = Vec::with_capacity(value_count);
    for edge in incoming {
        let from_values = indexed_stage_outputs(outputs_by_index, node_index, &edge.from_node)?;
        extend_cloned_bounded(&mut values, from_values, MAX_JOB_STAGE_VALUES, "job input values")?;
    }
    Ok(values)
}

fn indexed_stage_outputs<'a>(
    outputs_by_index: &'a [Option<Vec<IOValue>>],
    node_index: &BTreeMap<String, usize>,
    node_id: &str,
) -> Result<&'a Vec<IOValue>> {
    let from_index = *node_index
        .get(node_id)
        .ok_or_else(|| MoltenError::invalid_harness(format!("job edge input from {node_id} lacks node index")))?;
    outputs_by_index
        .get(from_index)
        .and_then(Option::as_ref)
        .ok_or_else(|| MoltenError::invalid_harness(format!("job edge input from {node_id} not available")))
}

fn sink_nodes(dag: &JobDag) -> Result<Vec<String>> {
    ensure_count_at_most(dag.nodes.len(), MAX_JOB_NODES, "job sink nodes")?;
    ensure_count_at_most(dag.edges.len(), MAX_JOB_EDGES, "job sink edges")?;
    let mut from = BTreeSet::new();
    for edge in &dag.edges {
        from.insert(edge.from_node.clone());
    }
    let mut sinks = Vec::with_capacity(dag.nodes.len());
    for node in &dag.nodes {
        if !from.contains(&node.id) {
            push_bounded(&mut sinks, node.id.clone(), MAX_JOB_NODES, "job sink nodes")?;
        }
    }
    if sinks.is_empty() {
        for node in &dag.nodes {
            push_bounded(&mut sinks, node.id.clone(), MAX_JOB_NODES, "job sink nodes")?;
        }
    }
    sinks.sort();
    Ok(sinks)
}

fn refs_for_values(values: &[IOValue]) -> Result<Vec<String>> {
    ensure_count_at_most(values.len(), MAX_JOB_STAGE_VALUES, "job values to hash")?;
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        push_bounded(&mut refs, canonical_hash(value)?, MAX_JOB_REFS, "job value refs")?;
    }
    Ok(refs)
}

fn parse_cached_stage_output(value: &IOValue) -> Result<Vec<IOValue>> {
    if let Some(items) = value.collect_sequence() {
        ensure_count_at_most(items.len(), MAX_JOB_STAGE_VALUES, "cached job stage output")?;
        let mut values = Vec::with_capacity(items.len());
        for item in items.iter() {
            push_bounded(&mut values, value_to_iovalue(item), MAX_JOB_STAGE_VALUES, "cached job stage output")?;
        }
        Ok(values)
    } else {
        Err(MoltenError::invalid_harness("cached job stage output must be a sequence"))
    }
}

fn combined_policy_refs(dag: &JobDag, request: &JobOutputRequest, node: Option<&JobNode>) -> Vec<String> {
    let node_policy_count = node.map_or(0, |node| node.policy_refs.len());
    let capacity = dag
        .policy_refs
        .len()
        .saturating_add(request.policy_refs.len())
        .saturating_add(node_policy_count)
        .min(MAX_JOB_REFS);
    let mut refs = Vec::with_capacity(capacity);
    refs.extend(dag.policy_refs.iter().cloned());
    refs.extend(request.policy_refs.iter().cloned());
    if let Some(node) = node {
        refs.extend(node.policy_refs.iter().cloned());
    }
    sorted_unique(&refs)
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_optional_string(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_string_value(&record[0])
}

fn record_iovalue(value: &Value<IOValue>, label: &str) -> Result<IOValue> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    Ok(value_to_iovalue(&record[0]))
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn record_node_id_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_JOB_NODES, label)?;
    let mut ids = Vec::with_capacity(items.len());
    for item in items.iter() {
        let id = required_string(item, label)?;
        validate_node_id(&id)?;
        push_bounded(&mut ids, id, MAX_JOB_NODES, label)?;
    }
    Ok(ids)
}

fn record_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_JOB_REFS, label)?;
    let mut strings = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut strings, required_string(item, label)?, MAX_JOB_REFS, label)?;
    }
    Ok(strings)
}

fn record_port_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_JOB_PORTS, label)?;
    let mut ports = Vec::with_capacity(items.len());
    for item in items.iter() {
        let port = value_to_iovalue(item);
        let fields = simple_record(&port, "port", 2)?;
        let name = required_string(&fields[0], "port name")?;
        validate_non_empty(&name, "port name")?;
        push_bounded(&mut ports, name, MAX_JOB_PORTS, label)?;
    }
    Ok(ports)
}

fn parse_ref_sequence_value(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    ensure_count_at_most(items.len(), MAX_JOB_REFS, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut refs, required_ref(item, label)?, MAX_JOB_REFS, label)?;
    }
    Ok(refs)
}

fn sequence_items(value: &Value<IOValue>, label: &str) -> Result<Vec<IOValue>> {
    let items = required_sequence(value, label)?;
    ensure_count_at_most(items.len(), MAX_JOB_STAGE_VALUES, label)?;
    let mut values = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut values, value_to_iovalue(item), MAX_JOB_STAGE_VALUES, label)?;
    }
    Ok(values)
}

fn simple_record<'a>(
    value: &'a IOValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IOValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IOValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IOValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &Value<IOValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn required_u64_value(value: &IOValue, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn usize_to_u64(value: usize, field: &str) -> Result<u64> {
    u64::try_from(value).map_err(|error| MoltenError::invalid_harness(format!("{field} out of range: {error}")))
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    let total = left
        .checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    Ok(total)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    checked_count_sum(values.item_count(), 1, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn extend_cloned_bounded<T: Clone>(
    values: &mut impl crate::bounded::VecSink<T>,
    incoming: &[T],
    maximum: usize,
    label: &str,
) -> Result<()> {
    let final_count = checked_count_sum(values.item_count(), incoming.len(), maximum, label)?;
    values.reserve_items(final_count.saturating_sub(values.item_count()));
    values.extend_cloned_items(incoming);
    Ok(())
}

fn insert_bounded<K: Ord, V>(
    values: &mut BTreeMap<K, V>,
    key: K,
    value: V,
    maximum: usize,
    label: &str,
) -> Result<Option<V>> {
    if !values.contains_key(&key) {
        checked_count_sum(values.len(), 1, maximum, label)?;
    }
    Ok(values.insert(key, value))
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_string_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn parse_optional_string_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_string(&some[0], "optional string").map(Some);
    }
    required_string(value, "optional string").map(Some)
}

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn ports_sequence(ports: &[String]) -> IOValue {
    sequence(ports.iter().map(|port| record("port", vec![string(port), record("none", Vec::new())])).collect())
}

fn checks_value(names: &[&str]) -> IOValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "checks")?;
    ensure_count_at_most(items.len(), MAX_JOB_CHECKS, "checks")?;
    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("job check {name} has status {status}")));
        }
        parsed.push(name);
    }
    Ok(parsed)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &Value<IOValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn record_sequence_values(value: &Value<IOValue>, label: &str) -> Result<Vec<IOValue>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_JOB_REFS, label)?;
    let mut values = Vec::with_capacity(items.len());
    for item in items.iter() {
        push_bounded(&mut values, value_to_iovalue(item), MAX_JOB_REFS, label)?;
    }
    Ok(values)
}

fn parse_job_content_ref_record(value: &Value<IOValue>, label: &str) -> Result<JobContentRef> {
    parse_job_content_ref_value(&record_iovalue(value, label)?)
}

fn parse_job_content_ref_value(value: &IOValue) -> Result<JobContentRef> {
    let fields = simple_record(value, "job-content-ref", 4)?;
    let size_value = record_iovalue(&fields[1], "size")?;
    let content = JobContentRef {
        content_ref: record_ref(&fields[0], "content-ref")?,
        size: required_u64_value(&size_value, "job content size")?,
        format: record_string(&fields[2], "format")?,
        schema_ref: record_optional_ref(&fields[3], "schema")?,
    };
    validate_job_content_ref(&content, "job content ref")?;
    Ok(content)
}

fn validate_blob_ref_submission_input(input: &BlobRefJobSubmissionValueInput<'_>) -> Result<()> {
    validate_non_empty(input.job_id, "job ref submission job id")?;
    validate_ref(input.operation_id, "job ref submission operation id")?;
    validate_job_content_ref(&input.executable, "job ref executable")?;
    ensure_count_at_most(input.inputs.len(), MAX_JOB_REFS, "job ref submission inputs")?;
    for content in &input.inputs {
        validate_job_content_ref(content, "job ref input")?;
    }
    validate_output_mode(input.output_mode)?;
    validate_blob_ref_handler_profile(input.handler_profile)?;
    validate_ref(input.authority_context_ref, "job ref authority context ref")?;
    validate_refs(input.input_schema_refs, "job ref input schema ref")?;
    validate_refs(input.output_schema_refs, "job ref output schema ref")?;
    validate_refs(input.effect_manifest_refs, "job ref effect manifest ref")?;
    validate_refs(input.policy_refs, "job ref policy ref")?;
    validate_refs(input.provenance_refs, "job ref provenance ref")?;
    validate_refs(input.evidence_refs, "job ref evidence ref")?;
    Ok(())
}

fn validate_blob_ref_submission(submission: &BlobRefJobSubmission) -> Result<()> {
    validate_non_empty(&submission.job_id, "job ref submission job id")?;
    validate_ref(&submission.operation_id, "job ref submission operation id")?;
    validate_job_content_ref(&submission.executable, "job ref executable")?;
    ensure_count_at_most(submission.inputs.len(), MAX_JOB_REFS, "job ref submission inputs")?;
    for content in &submission.inputs {
        validate_job_content_ref(content, "job ref input")?;
    }
    validate_output_mode(&submission.output_mode)?;
    validate_blob_ref_handler_profile(&submission.handler_profile)?;
    validate_ref(&submission.authority_context_ref, "job ref authority context ref")?;
    validate_refs(&submission.input_schema_refs, "job ref input schema ref")?;
    validate_refs(&submission.output_schema_refs, "job ref output schema ref")?;
    validate_refs(&submission.effect_manifest_refs, "job ref effect manifest ref")?;
    validate_refs(&submission.policy_refs, "job ref policy ref")?;
    validate_refs(&submission.provenance_refs, "job ref provenance ref")?;
    validate_refs(&submission.evidence_refs, "job ref evidence ref")?;
    Ok(())
}

fn validate_job_content_ref(content: &JobContentRef, field: &str) -> Result<()> {
    if content.size > MAX_JOB_INLINE_BYTES && content.content_ref.is_empty() {
        return Err(MoltenError::invalid_harness("large job content must use a content ref"));
    }
    validate_ref(&content.content_ref, field)?;
    validate_non_empty(&content.format, "job content format")?;
    if let Some(schema_ref) = content.schema_ref.as_ref() {
        validate_ref(schema_ref, "job content schema ref")?;
    }
    Ok(())
}

fn validate_output_mode(output_mode: &str) -> Result<()> {
    if output_mode == "chunk-manifest" {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job ref output mode {output_mode}")))
    }
}

fn validate_blob_ref_handler_profile(handler_profile: &str) -> Result<()> {
    if handler_profile == "local-echo-v1" {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job ref handler profile {handler_profile}")))
    }
}

fn validate_blob_ref_state(state: &str) -> Result<()> {
    if matches!(state, "queued" | "fetching" | "running" | "result-ready" | "complete" | "failed" | "cancelled") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job ref status state {state}")))
    }
}

fn reject_blob_ref_job_inline_tokens(value: &IOValue) -> Result<()> {
    let text = to_text(value)?;
    for token in ["inline-bytes", "inline-executable", "inline-dataset"] {
        if text.contains(token) {
            return Err(MoltenError::invalid_harness(format!(
                "job ref submission must use content refs, found inline token {token}"
            )));
        }
    }
    Ok(())
}

fn validate_stage_kind(kind: &str) -> Result<()> {
    if matches!(kind, "source" | "map" | "filter" | "reduce" | "materialize") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job stage kind {kind}")))
    }
}

fn validate_stage_operation(operation: &str) -> Result<()> {
    if matches!(
        operation,
        "source"
            | "materialize"
            | "identity"
            | "wrap"
            | "tag-record"
            | "project-field"
            | "keep-all"
            | "drop-all"
            | "equals"
            | "match-record"
            | "count"
            | "sum-u64"
            | "sum-integers"
            | "concat-lists"
    ) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job stage operation {operation}")))
    }
}

fn validate_partitioning(partitioning: &str) -> Result<()> {
    if matches!(partitioning, "single" | "partitioned") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job edge partitioning {partitioning}")))
    }
}

fn validate_materialization(materialization: &str) -> Result<()> {
    if matches!(materialization, "stream" | "typed-ref" | "content-ref") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job edge materialization {materialization}")))
    }
}

fn worker_stage_receipts(stage_order: &[String], receipt_refs: &[String]) -> Result<Vec<(String, String)>> {
    ensure_count_at_most(stage_order.len(), MAX_JOB_NODES, "job worker stage receipts")?;
    ensure_count_at_most(receipt_refs.len(), MAX_JOB_NODES, "job worker stage receipts")?;
    let mut stage_receipts = Vec::with_capacity(stage_order.len().min(receipt_refs.len()));
    for (stage_id, receipt_ref) in stage_order.iter().zip(receipt_refs.iter()) {
        validate_node_id(stage_id)?;
        validate_ref(receipt_ref, "job worker stage receipt ref")?;
        push_bounded(
            &mut stage_receipts,
            (stage_id.clone(), receipt_ref.clone()),
            MAX_JOB_NODES,
            "job worker stage receipts",
        )?;
    }
    Ok(stage_receipts)
}

fn validate_stage_receipt_refs(stage_receipts: &[(String, String)]) -> Result<()> {
    ensure_count_at_most(stage_receipts.len(), MAX_JOB_NODES, "job worker stage receipts")?;
    for (stage_id, receipt_ref) in stage_receipts {
        validate_node_id(stage_id)?;
        validate_ref(receipt_ref, "job worker stage receipt ref")?;
    }
    Ok(())
}

fn record_stage_receipt_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    ensure_count_at_most(items.len(), MAX_JOB_NODES, label)?;
    let mut stage_receipts = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let stage = simple_record(&item, "stage", 2)?;
        let stage_id = required_string(&stage[0], "job worker stage id")?;
        validate_node_id(&stage_id)?;
        let receipt_ref = required_ref(&stage[1], "job worker stage receipt ref")?;
        push_bounded(&mut stage_receipts, (stage_id, receipt_ref), MAX_JOB_NODES, label)?;
    }
    Ok(stage_receipts)
}

fn import_worker_artifacts(
    ledger_root: &Path,
    assignment_value: &IOValue,
    status_values: &[IOValue],
    result_value: &IOValue,
    receipt_value: &IOValue,
) -> Result<()> {
    ledger::import_artifact(ledger_root, assignment_value)?;
    for status_value in status_values {
        ledger::import_artifact(ledger_root, status_value)?;
    }
    ledger::import_artifact(ledger_root, result_value)?;
    ledger::import_artifact(ledger_root, receipt_value)?;
    Ok(())
}

fn reject_worker_ambient_tokens(value: &IOValue) -> Result<()> {
    let text = to_text(value)?;
    let banned = [
        "<raw-closure",
        "<closure",
        "<host-path",
        "<source-path",
        "<source-registry",
        "<process-command",
        "<command",
        "<env",
        "<environment",
        "<source-text",
    ];
    if let Some(token) = banned.iter().find(|token| text.contains(**token)) {
        Err(MoltenError::invalid_harness(format!("job worker request contains mobile/ambient token {token}")))
    } else {
        Ok(())
    }
}

fn validate_request_materialization(materialization: &str) -> Result<()> {
    if matches!(materialization, "inline" | "typed-storage" | "chunk-manifest") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job output materialization {materialization}")))
    }
}

fn validate_receipt_operation(operation: &str) -> Result<()> {
    if matches!(operation, "install" | "run" | "stage" | "memo-hit" | "memo-miss" | "materialize" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job receipt operation {operation}")))
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job receipt decision {decision}")))
    }
}

fn validate_worker_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny" | "non-replayable") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job worker decision {decision}")))
    }
}

fn validate_worker_state(state: &str) -> Result<()> {
    if matches!(state, "received" | "running" | "completed" | "denied" | "non-replayable") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported job worker state {state}")))
    }
}

fn validate_node_id(id: &str) -> Result<()> {
    validate_non_empty(id, "job node id")?;
    if id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "job node id {id} must use ascii alphanumeric, '-', '_' or '.'"
        )))
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

fn reject_mobile_closure_config(config: &IOValue) -> Result<()> {
    let text = to_text(config)?;
    let banned = [
        "<closure",
        "<raw-closure",
        "<host-path",
        "<process-command",
        "<command",
        "<env",
        "<environment",
        "<source-text",
    ];
    if let Some(token) = banned.iter().find(|token| text.contains(**token)) {
        Err(MoltenError::invalid_harness(format!("job stage config contains mobile/ambient token {token}")))
    } else {
        Ok(())
    }
}

fn local_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("job-dag-local-ref", vec![string(kind), string(label)]))
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<BTreeSet<_>>().into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use hegel::TestCase;
    use hegel::generators;

    use super::*;

    fn test_node_value(
        id: &str,
        kind: &str,
        input_ports: &[String],
        output_ports: &[String],
        config: IOValue,
    ) -> Result<IOValue> {
        job_node_value(NodeValueInput {
            id,
            kind,
            stage_artifact_ref: None,
            input_ports,
            output_ports,
            config,
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
    }

    fn stream_edge_value(from_node: &str, to_node: &str) -> Result<IOValue> {
        job_edge_value(EdgeValueInput {
            from_node,
            from_port: "out",
            to_node,
            to_port: "in",
            schema_ref: None,
            partitioning: "single",
            materialization: "stream",
        })
    }

    fn test_dag_value(nodes: Vec<IOValue>, edges: Vec<IOValue>, output_roots: &[String]) -> Result<IOValue> {
        job_dag_value(DagValueInput {
            nodes,
            edges,
            output_roots,
            schema_refs: &[],
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
    }

    #[test]
    fn dag_identity_is_stable_and_ignores_names() {
        let dag = fixture_dag("identity");
        let parsed = parse_job_dag_value(&dag).expect("parse dag");
        let reparsed =
            parse_job_dag_value(&parse_text(&to_text(&dag).expect("text")).expect("text parse")).expect("reparse");
        assert_eq!(parsed.job_ref, reparsed.job_ref);
        let changed = fixture_dag("count");
        let changed = parse_job_dag_value(&changed).expect("changed parse");
        assert_ne!(parsed.job_ref, changed.job_ref);
    }

    #[test]
    fn local_pipeline_runs_and_memoizes() {
        let root = temp_dir("job-pipeline");
        let registry = root.join("registry");
        let storage = root.join("storage");
        let cache = root.join("cache");
        let chunks = root.join("chunks");
        let ledger = root.join("ledger");
        let dag_value = pipeline_dag().expect("dag");
        let install = install_job_dag(&registry, &dag_value).expect("install");
        assert_eq!(install.decision, "pass");
        let dag = read_job_dag(&registry, &install.job_ref).expect("read dag");
        let options = JobRunOptions {
            registry_root: &registry,
            storage_root: &storage,
            cache_root: &cache,
            chunk_root: &chunks,
            ledger_root: Some(&ledger),
            output_request: None,
        };
        let first = run_job_dag(&dag, &options).expect("first run");
        let second = run_job_dag(&dag, &options).expect("second run");
        assert_eq!(first.output_refs, second.output_refs);
        let second_text = to_text(&second.receipt_value).expect("receipt text");
        assert!(["memo-hit", "stage-receipts-bound"].iter().any(|needle| second_text.contains(needle)));
        let output_text = to_text(&second.output_value).expect("output text");
        assert!(output_text.contains("wrapped"));
    }

    #[test]
    fn blob_ref_job_submission_worker_verifies_and_outputs_manifest() {
        let root = temp_dir("job-ref-worker");
        let chunks = root.join("chunks");
        let ledger = root.join("ledger");
        let executable = chunk_store::put_bytes(&chunks, "job-executable", b"echo", DEFAULT_FIXED_V1_CHUNK_SIZE)
            .expect("put executable");
        let input =
            chunk_store::put_bytes(&chunks, "job-input", b"hello", DEFAULT_FIXED_V1_CHUNK_SIZE).expect("put input");
        let operation_id = local_ref("job-ref-operation", "one").expect("operation id");
        let policy_ref = local_ref("job-ref-policy", "one").expect("policy ref");
        let provenance_ref = local_ref("job-ref-provenance", "one").expect("provenance ref");
        let effect_ref = local_ref("job-ref-effect", "one").expect("effect ref");
        let authority_ref = local_ref("job-ref-authority", "one").expect("authority ref");
        let submission_value = job_ref_submission_value(BlobRefJobSubmissionValueInput {
            job_id: "job-ref-worker",
            operation_id: &operation_id,
            executable: JobContentRef {
                content_ref: executable.manifest_ref.clone(),
                size: executable.total_len,
                format: "elf-executable".to_string(),
                schema_ref: None,
            },
            inputs: vec![JobContentRef {
                content_ref: input.manifest_ref.clone(),
                size: input.total_len,
                format: "bytes".to_string(),
                schema_ref: None,
            }],
            output_mode: "chunk-manifest",
            input_schema_refs: &[],
            output_schema_refs: &[],
            effect_manifest_refs: std::slice::from_ref(&effect_ref),
            handler_profile: "local-echo-v1",
            authority_context_ref: &authority_ref,
            policy_refs: std::slice::from_ref(&policy_ref),
            provenance_refs: std::slice::from_ref(&provenance_ref),
            evidence_refs: &[],
        })
        .expect("submission value");
        let parsed_submission = parse_job_ref_submission_value(&submission_value).expect("parse submission");
        assert_eq!(parsed_submission.inputs.len(), 1);
        let executed = execute_blob_ref_job(BlobRefJobExecuteInput {
            chunk_root: &chunks,
            submission_value: &submission_value,
            ledger_root: Some(&ledger),
        })
        .expect("execute blob ref job");
        assert_eq!(executed.decision, "pass");
        let output_ref = executed.output_manifest_ref.as_deref().expect("output ref");
        let output = chunk_store::read_object(&chunks, output_ref).expect("read output");
        assert_eq!(output.bytes, b"hello");
        let receipt = parse_blob_ref_job_receipt_value(&executed.receipt_value).expect("parse receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.output_refs, vec![output_ref.to_string()]);
        assert!(
            crate::ledger::list_artifacts(&ledger)
                .expect("ledger artifacts")
                .iter()
                .any(|artifact| artifact.artifact_kind == "job-ref-receipt")
        );
    }

    #[test]
    fn blob_ref_job_submission_denies_missing_ref_before_run() {
        let root = temp_dir("job-ref-missing");
        let chunks = root.join("chunks");
        let executable = chunk_store::put_bytes(&chunks, "job-executable", b"echo", DEFAULT_FIXED_V1_CHUNK_SIZE)
            .expect("put executable");
        let operation_id = local_ref("job-ref-operation", "missing").expect("operation id");
        let policy_ref = local_ref("job-ref-policy", "missing").expect("policy ref");
        let provenance_ref = local_ref("job-ref-provenance", "missing").expect("provenance ref");
        let effect_ref = local_ref("job-ref-effect", "missing").expect("effect ref");
        let authority_ref = local_ref("job-ref-authority", "missing").expect("authority ref");
        let missing_ref = local_ref("job-ref-missing-input", "missing").expect("missing input ref");
        let submission_value = job_ref_submission_value(BlobRefJobSubmissionValueInput {
            job_id: "job-ref-missing",
            operation_id: &operation_id,
            executable: JobContentRef {
                content_ref: executable.manifest_ref.clone(),
                size: executable.total_len,
                format: "elf-executable".to_string(),
                schema_ref: None,
            },
            inputs: vec![JobContentRef {
                content_ref: missing_ref,
                size: 5,
                format: "bytes".to_string(),
                schema_ref: None,
            }],
            output_mode: "chunk-manifest",
            input_schema_refs: &[],
            output_schema_refs: &[],
            effect_manifest_refs: std::slice::from_ref(&effect_ref),
            handler_profile: "local-echo-v1",
            authority_context_ref: &authority_ref,
            policy_refs: std::slice::from_ref(&policy_ref),
            provenance_refs: std::slice::from_ref(&provenance_ref),
            evidence_refs: &[],
        })
        .expect("submission value");
        let executed = execute_blob_ref_job(BlobRefJobExecuteInput {
            chunk_root: &chunks,
            submission_value: &submission_value,
            ledger_root: None,
        })
        .expect("deny execution still emits receipt");
        assert_eq!(executed.decision, "deny");
        assert!(executed.output_manifest_ref.is_none());
        assert!(!executed.diagnostics.is_empty());
        let receipt = parse_blob_ref_job_receipt_value(&executed.receipt_value).expect("parse deny receipt");
        assert_eq!(receipt.decision, "deny");
    }

    #[test]
    fn blob_ref_job_submission_rejects_malformed_content_refs() {
        let operation_id = local_ref("job-ref-operation", "malformed").expect("operation id");
        let authority_ref = local_ref("job-ref-authority", "malformed").expect("authority ref");
        for invalid in [
            "blake3:fixture",
            "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
        ] {
            let error = job_ref_submission_value(BlobRefJobSubmissionValueInput {
                job_id: "job-ref-malformed",
                operation_id: &operation_id,
                executable: JobContentRef {
                    content_ref: invalid.to_string(),
                    size: 4,
                    format: "elf-executable".to_string(),
                    schema_ref: None,
                },
                inputs: Vec::new(),
                output_mode: "chunk-manifest",
                input_schema_refs: &[],
                output_schema_refs: &[],
                effect_manifest_refs: &[],
                handler_profile: "local-echo-v1",
                authority_context_ref: &authority_ref,
                policy_refs: &[],
                provenance_refs: &[],
                evidence_refs: &[],
            })
            .expect_err("malformed executable ref denied");
            assert!(error.to_string().contains("canonical blake3 content ref"), "unexpected error: {error}");
        }
    }

    #[test]
    fn blob_ref_job_submission_rejects_inline_large_bytes() {
        let operation_id = local_ref("job-ref-operation", "inline").expect("operation id");
        let authority_ref = local_ref("job-ref-authority", "inline").expect("authority ref");
        let value = record("job-ref-submission-v1", vec![
            string(JOB_REF_SUBMISSION_SCHEMA),
            record("job-id", vec![string("job-ref-inline")]),
            record("operation-id", vec![string(&operation_id)]),
            record("executable", vec![record("inline-bytes", vec![string("not-a-content-ref")])]),
            record("inputs", vec![sequence(vec![])]),
            record("output-mode", vec![string("chunk-manifest")]),
            record("input-schemas", vec![refs_sequence(&[])]),
            record("output-schemas", vec![refs_sequence(&[])]),
            record("effects", vec![refs_sequence(&[])]),
            record("handler-profile", vec![string("local-echo-v1")]),
            record("authority", vec![string(&authority_ref)]),
            record("policy", vec![refs_sequence(&[])]),
            record("provenance", vec![refs_sequence(&[])]),
            record("evidence", vec![refs_sequence(&[])]),
            checks_value(&["content-refs-only", "no-inline-large-bytes"]),
        ]);
        assert!(
            parse_job_ref_submission_value(&value)
                .expect_err("inline bytes rejected")
                .to_string()
                .contains("inline")
        );
    }

    #[test]
    fn reduce_records_deterministic_output() {
        let root = temp_dir("job-reduce");
        let registry = root.join("registry");
        let storage = root.join("storage");
        let cache = root.join("cache");
        let chunks = root.join("chunks");
        let source = test_node_value(
            "source",
            "source",
            &[],
            &["out".to_string()],
            record("source", vec![record("values", vec![sequence(vec![
                u64_value(1),
                u64_value(2),
                u64_value(3),
            ])])]),
        )
        .expect("source");
        let reduce = test_node_value(
            "sum",
            "reduce",
            &["in".to_string()],
            &["out".to_string()],
            record("op", vec![string("sum-u64")]),
        )
        .expect("reduce");
        let edge = stream_edge_value("source", "sum").expect("edge");
        let dag_value = test_dag_value(vec![source, reduce], vec![edge], &["sum".to_string()]).expect("dag");
        let dag = parse_job_dag_value(&dag_value).expect("parse dag");
        let options = JobRunOptions {
            registry_root: &registry,
            storage_root: &storage,
            cache_root: &cache,
            chunk_root: &chunks,
            ledger_root: None,
            output_request: None,
        };
        let run = run_job_dag(&dag, &options).expect("run");
        assert_eq!(to_text(&run.output_value).expect("output"), "[6]");
    }

    #[test]
    fn trellis_topology_orders_by_canonical_indices_and_rejects_cycles() {
        let a = test_node_value(
            "a",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            record("op", vec![string("identity")]),
        )
        .expect("a");
        let b = test_node_value(
            "b",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            record("op", vec![string("identity")]),
        )
        .expect("b");
        let a_node = parse_job_node_value(&a).expect("parse a");
        let b_node = parse_job_node_value(&b).expect("parse b");
        let ordered = execution_order(&[b_node, a_node], &[]).expect("independent order");
        assert_eq!(ordered, vec!["a".to_string(), "b".to_string()]);
        let ab = stream_edge_value("a", "b").expect("ab");
        let ba = stream_edge_value("b", "a").expect("ba");
        let cyclic = test_dag_value(vec![a, b], vec![ab, ba], &["b".to_string()]).expect("cyclic dag value");
        assert!(parse_job_dag_value(&cyclic).expect_err("cycle rejected").to_string().contains("trellis"));
    }

    #[test]
    fn sync_plan_and_loopback_copy_dependency_closure_without_execution() {
        let case = copy_case();
        let sync_ref = assert_copy(&case);
        let flow = passing_flow(&case, sync_ref);
        assert_target_run(&case, &flow);
        assert_wrong_peer_execution(&case, &flow);
        assert_other_reference_denial(&case, &flow);
        assert_stale_closure_execution(&case, &flow);
        assert_missing_admission_inputs(&case, &flow);
        assert_unsatisfied_stage_denial(&case, &flow);
    }

    struct CopyArtifacts {
        base: artifacts::ArtifactInstall,
        source_stage: artifacts::ArtifactInstall,
        stage: artifacts::ArtifactInstall,
    }

    struct CopyCase {
        root: PathBuf,
        source: PathBuf,
        target: PathBuf,
        base: artifacts::ArtifactInstall,
        source_stage: artifacts::ArtifactInstall,
        stage: artifacts::ArtifactInstall,
        installed_job: JobInstall,
        request: IOValue,
    }

    struct CopyFlow {
        sync_ref: String,
        source_gate_ref: String,
        authority_context_ref: String,
        admission: JobAdmissionLoopback,
        admission_ref: String,
    }

    fn install_case_artifact(
        registry: &Path,
        kind: &str,
        payload: IOValue,
        dependency_refs: Vec<String>,
        label: &str,
    ) -> artifacts::ArtifactInstall {
        artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
            kind: kind.to_string(),
            payload,
            schema_refs: vec![test_ref("schema")],
            dependency_refs,
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect(label)
    }

    fn copy_artifacts(source: &Path) -> CopyArtifacts {
        let base =
            install_case_artifact(source, "schema", record("schema", vec![string("base")]), Vec::new(), "install base");
        let source_stage = install_case_artifact(
            source,
            "stage",
            builtin_stage_operation_value("source").expect("source stage op"),
            Vec::new(),
            "install source stage",
        );
        let stage = install_case_artifact(
            source,
            "stage",
            builtin_stage_operation_value("identity").expect("stage op"),
            vec![base.artifact_ref.clone()],
            "install stage",
        );
        CopyArtifacts {
            base,
            source_stage,
            stage,
        }
    }

    fn copy_case() -> CopyCase {
        let root = temp_dir("job-sync");
        let source = root.join("source");
        let target = root.join("target");
        let CopyArtifacts {
            base,
            source_stage,
            stage,
        } = copy_artifacts(&source);
        let source_node = job_node_value(NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: Some(&source_stage.artifact_ref),
            input_ports: &[],
            output_ports: &["out".to_string()],
            config: record("source", vec![record("values", vec![sequence(vec![string("x")])])]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("source node");
        let map = job_node_value(NodeValueInput {
            id: "map",
            kind: "map",
            stage_artifact_ref: Some(&stage.artifact_ref),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: record("op", vec![string("identity")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("map node");
        let edge = stream_edge_value("source", "map").expect("edge");
        let dag_value = test_dag_value(vec![source_node, map], vec![edge], &["map".to_string()]).expect("dag value");
        let installed_job = install_job_dag(&source, &dag_value).expect("install job");
        let request = job_sync_request_value(SyncRequestValueInput {
            job_ref: &installed_job.job_ref,
            stage_ids: &[],
            target_peer: "peer:loopback",
            policy_refs: &[test_ref("sync-policy")],
            capability_refs: &[test_ref("sync-capability")],
            evidence_refs: &[test_ref("sync-evidence")],
        })
        .expect("sync request");
        CopyCase {
            root,
            source,
            target,
            base,
            source_stage,
            stage,
            installed_job,
            request,
        }
    }

    fn assert_copy(case: &CopyCase) -> String {
        let plan = sync_plan_value(&case.source, &case.target, &case.request).expect("sync plan");
        assert!(plan.missing_refs.contains(&case.base.artifact_ref));
        assert!(plan.missing_refs.contains(&case.stage.artifact_ref));
        let denied = sync_loopback(SyncLoopbackInput {
            source_registry: &case.source,
            target_registry: &case.target,
            request_value: &case.request,
            provenance_values: &[],
            build_verification_values: &[],
        })
        .expect("sync without provenance emits deny receipt");
        assert_eq!(denied.decision, "deny");
        assert!(denied.installed_refs.is_empty());
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing provenance")));
        assert!(artifacts::list_artifacts(&case.target, None).expect("target artifacts").is_empty());
        let sync_provenance = reviewed_provenance_values(&[
            case.base.artifact_ref.clone(),
            case.source_stage.artifact_ref.clone(),
            case.stage.artifact_ref.clone(),
            case.installed_job.artifact_ref.clone(),
        ]);
        let synced = sync_loopback(SyncLoopbackInput {
            source_registry: &case.source,
            target_registry: &case.target,
            request_value: &case.request,
            provenance_values: &sync_provenance,
            build_verification_values: &[],
        })
        .expect("sync loopback");
        assert!(synced.installed_refs.contains(&case.base.artifact_ref));
        assert!(synced.installed_refs.contains(&case.source_stage.artifact_ref));
        assert!(synced.installed_refs.contains(&case.stage.artifact_ref));
        assert_eq!(
            artifacts::read_artifact(&case.target, &case.base.artifact_ref).expect("target base").value,
            case.base.artifact.value
        );
        let second = sync_loopback(SyncLoopbackInput {
            source_registry: &case.source,
            target_registry: &case.target,
            request_value: &case.request,
            provenance_values: &sync_provenance,
            build_verification_values: &[],
        })
        .expect("sync no-op");
        assert!(second.installed_refs.is_empty());
        assert!(second.already_present_refs.contains(&case.base.artifact_ref));
        assert!(to_text(&second.receipt_value).expect("receipt text").contains("no-execution"));
        canonical_hash(&synced.receipt_value).expect("sync receipt ref")
    }

    fn passing_flow(case: &CopyCase, sync_ref: String) -> CopyFlow {
        let authority_context_ref = install_job_execute_authority_context(&case.target, &case.installed_job.job_ref);
        let source_gate_ref = install_clean_octet_gate(&case.target);
        let admission_request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            sync_ref: &sync_ref,
            stage_ids: &[],
            target_peer: "peer:loopback",
            policy_refs: &[test_ref("admit-policy")],
            capability_refs: std::slice::from_ref(&authority_context_ref),
            evidence_refs: &[sync_ref.clone(), source_gate_ref.clone()],
            resource_refs: &[test_ref("resource-1"), test_ref("resource-2")],
        })
        .expect("admission request");
        let admission = admission_loopback(&case.target, &admission_request).expect("admission loopback");
        assert_eq!(admission.plan.decision, "pass");
        assert!(to_text(&admission.receipt_value).expect("admission receipt").contains("no-execution"));
        let admission_ref = canonical_hash(&admission.receipt_value).expect("admission ref");
        CopyFlow {
            sync_ref,
            source_gate_ref,
            authority_context_ref,
            admission,
            admission_ref,
        }
    }

    fn passing_execution(case: &CopyCase, flow: &CopyFlow) -> (IOValue, JobExecutionLoopback) {
        let request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            admission_ref: &flow.admission_ref,
            stage_ids: &flow.admission.plan.stage_order,
            target_peer: "peer:loopback",
            storage_profile_ref: &test_ref("storage-profile"),
            cache_profile_ref: &test_ref("cache-profile"),
            chunk_profile_ref: &test_ref("chunk-profile"),
            policy_refs: &[test_ref("admit-policy")],
            capability_refs: std::slice::from_ref(&flow.authority_context_ref),
            resource_refs: &[test_ref("resource-1"), test_ref("resource-2")],
        })
        .expect("execution request");
        let execution = execution_loopback(ExecutionLoopbackInput {
            target_registry: &case.target,
            storage_root: &case.root.join("storage"),
            cache_root: &case.root.join("cache"),
            chunk_root: &case.root.join("chunks"),
            admission_receipt_value: &flow.admission.receipt_value,
            request_value: &request,
        })
        .expect("execution loopback");
        (request, execution)
    }

    fn assert_target_run(case: &CopyCase, flow: &CopyFlow) {
        let (request, execution) = passing_execution(case, flow);
        assert_eq!(execution.decision, "pass");
        assert!(to_text(&execution.run.as_ref().expect("run").output_value).expect("execution output").contains("x"));
        let equivalent_source_run = run_job_dag(
            &read_job_dag(&case.source, &case.installed_job.job_ref).expect("source job"),
            &JobRunOptions {
                registry_root: &case.source,
                storage_root: &case.root.join("source-storage"),
                cache_root: &case.root.join("source-cache"),
                chunk_root: &case.root.join("source-chunks"),
                ledger_root: None,
                output_request: None,
            },
        )
        .expect("equivalent source run");
        assert_eq!(execution.run.as_ref().expect("execution run").output_value, equivalent_source_run.output_value);
        assert_eq!(crate::ledger::artifact_kind(&request), "job-execution-request");
        assert_eq!(crate::ledger::artifact_kind(&execution.receipt_value), "job-execution-receipt");
        let execution_text = to_text(&execution.receipt_value).expect("execution receipt");
        assert!(execution_text.contains("job-execution-receipt-v1"));
        assert!(execution_text.contains("executed-on-target-state"));
        assert!(execution_text.contains(&flow.admission.plan.request.sync_ref));
        assert!(execution_text.contains(&flow.admission.plan.authority_receipt_refs[0]));
        assert!(execution_text.contains(&test_ref("resource-1")));
        assert!(execution_text.contains(&execution.run.as_ref().expect("execution run refs").stage_receipt_refs[0]));
        assert!(execution_text.contains(&execution.run.as_ref().expect("execution output refs").output_refs[0]));
    }

    fn assert_wrong_peer_execution(case: &CopyCase, flow: &CopyFlow) {
        let request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            admission_ref: &flow.admission_ref,
            stage_ids: &flow.admission.plan.stage_order,
            target_peer: "peer:other",
            storage_profile_ref: &test_ref("storage-profile"),
            cache_profile_ref: &test_ref("cache-profile"),
            chunk_profile_ref: &test_ref("chunk-profile"),
            policy_refs: &[test_ref("admit-policy")],
            capability_refs: std::slice::from_ref(&flow.authority_context_ref),
            resource_refs: &[test_ref("resource-1"), test_ref("resource-2")],
        })
        .expect("wrong peer request");
        let denied = execution_loopback(ExecutionLoopbackInput {
            target_registry: &case.target,
            storage_root: &case.root.join("storage-deny"),
            cache_root: &case.root.join("cache-deny"),
            chunk_root: &case.root.join("chunks-deny"),
            admission_receipt_value: &flow.admission.receipt_value,
            request_value: &request,
        })
        .expect("denied execution receipt");
        assert_eq!(denied.decision, "deny");
        assert!(denied.run.is_none());
        assert!(
            to_text(&denied.receipt_value)
                .expect("denied execution receipt")
                .contains("no-stage-execution-on-deny")
        );
    }

    fn assert_other_reference_denial(case: &CopyCase, flow: &CopyFlow) {
        let request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &test_ref("other-job"),
            admission_ref: &flow.admission_ref,
            stage_ids: &flow.admission.plan.stage_order,
            target_peer: "peer:loopback",
            storage_profile_ref: &test_ref("storage-profile"),
            cache_profile_ref: &test_ref("cache-profile"),
            chunk_profile_ref: &test_ref("chunk-profile"),
            policy_refs: &[],
            capability_refs: &[],
            resource_refs: &[],
        })
        .expect("wrong job request");
        let denied = execution_loopback(ExecutionLoopbackInput {
            target_registry: &case.target,
            storage_root: &case.root.join("storage-wrong-job"),
            cache_root: &case.root.join("cache-wrong-job"),
            chunk_root: &case.root.join("chunks-wrong-job"),
            admission_receipt_value: &flow.admission.receipt_value,
            request_value: &request,
        })
        .expect("wrong job execution denial");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("does not match admission job")));
    }

    fn assert_stale_closure_execution(case: &CopyCase, flow: &CopyFlow) {
        let stale_ref = test_ref("stale-stage-artifact");
        let tampered_admission = parse_text(&to_text(&flow.admission.receipt_value).expect("admission text").replacen(
            &case.stage.artifact_ref,
            &stale_ref,
            1,
        ))
        .expect("tampered admission parse");
        let tampered_admission_ref = canonical_hash(&tampered_admission).expect("tampered admission ref");
        let request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            admission_ref: &tampered_admission_ref,
            stage_ids: &flow.admission.plan.stage_order,
            target_peer: "peer:loopback",
            storage_profile_ref: &test_ref("storage-profile"),
            cache_profile_ref: &test_ref("cache-profile"),
            chunk_profile_ref: &test_ref("chunk-profile"),
            policy_refs: &[],
            capability_refs: &[],
            resource_refs: &[],
        })
        .expect("stale closure request");
        let denied = execution_loopback(ExecutionLoopbackInput {
            target_registry: &case.target,
            storage_root: &case.root.join("storage-stale"),
            cache_root: &case.root.join("cache-stale"),
            chunk_root: &case.root.join("chunks-stale"),
            admission_receipt_value: &tampered_admission,
            request_value: &request,
        })
        .expect("stale closure denial");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("closure diverges")));
    }

    fn assert_missing_admission_inputs(case: &CopyCase, flow: &CopyFlow) {
        let request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            sync_ref: &flow.sync_ref,
            stage_ids: &[],
            target_peer: "peer:loopback",
            policy_refs: &[],
            capability_refs: &[],
            evidence_refs: &[],
            resource_refs: &[],
        })
        .expect("missing authority request");
        let denied = admission_plan_value(&case.target, &request).expect("authority denial");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("policy")));
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("strict Octet source gate")));
        let admission = admission_loopback(&case.target, &request).expect("denied admission receipt");
        let admission_ref = canonical_hash(&admission.receipt_value).expect("denied admission ref");
        let execution_request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            admission_ref: &admission_ref,
            stage_ids: &admission.plan.stage_order,
            target_peer: "peer:loopback",
            storage_profile_ref: &test_ref("storage-profile"),
            cache_profile_ref: &test_ref("cache-profile"),
            chunk_profile_ref: &test_ref("chunk-profile"),
            policy_refs: &[],
            capability_refs: &[],
            resource_refs: &[],
        })
        .expect("denied execution request");
        let denied_by_admission = execution_loopback(ExecutionLoopbackInput {
            target_registry: &case.target,
            storage_root: &case.root.join("storage-denied-admission"),
            cache_root: &case.root.join("cache-denied-admission"),
            chunk_root: &case.root.join("chunks-denied-admission"),
            admission_receipt_value: &admission.receipt_value,
            request_value: &execution_request,
        })
        .expect("denied admission execution receipt");
        assert_eq!(denied_by_admission.decision, "deny");
        assert!(denied_by_admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("admission decision")));
    }

    fn assert_unsatisfied_stage_denial(case: &CopyCase, flow: &CopyFlow) {
        let request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            sync_ref: &flow.sync_ref,
            stage_ids: &["map".to_string()],
            target_peer: "peer:loopback",
            policy_refs: &[test_ref("admit-policy")],
            capability_refs: std::slice::from_ref(&flow.authority_context_ref),
            evidence_refs: &[flow.sync_ref.clone(), flow.source_gate_ref.clone()],
            resource_refs: &[test_ref("resource-1")],
        })
        .expect("unsatisfied request");
        let denied = admission_plan_value(&case.target, &request).expect("unsatisfied denial");
        assert_eq!(denied.decision, "deny");
        assert!(
            denied
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("unsatisfied selected-stage dependencies"))
        );
    }

    #[test]
    fn iroh_worker_recorded_loopback_executes_and_imports_results() {
        let fixture = worker_fixture("job-worker-pass");
        let worker = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("worker-storage"),
            cache_root: &fixture.root.join("worker-cache"),
            chunk_root: &fixture.root.join("worker-chunks"),
            delivery: &fixture.delivery,
            delivery_log: Some(&fixture.delivery_log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: Some(&fixture.ledger),
        })
        .expect("worker execute");
        assert_eq!(worker.result.decision, "pass", "{:?}", worker.result.diagnostics);
        assert!(worker.execution.as_ref().expect("execution").run.is_some());
        let source_run = run_job_dag(
            &read_job_dag(&fixture.source, &fixture.installed_job.job_ref).expect("source job"),
            &JobRunOptions {
                registry_root: &fixture.source,
                storage_root: &fixture.root.join("source-storage"),
                cache_root: &fixture.root.join("source-cache"),
                chunk_root: &fixture.root.join("source-chunks"),
                ledger_root: None,
                output_request: None,
            },
        )
        .expect("source equivalent run");
        assert_eq!(
            worker.execution.as_ref().expect("execution").run.as_ref().expect("run").output_value,
            source_run.output_value
        );
        assert_eq!(crate::ledger::artifact_kind(&fixture.worker_request), "job-worker-request");
        assert_eq!(crate::ledger::artifact_kind(&worker.result.value), "job-worker-result");
        assert_eq!(crate::ledger::artifact_kind(&worker.receipt_value), "job-worker-receipt");
        let kinds = crate::ledger::list_artifacts(&fixture.ledger)
            .expect("worker ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<BTreeSet<_>>();
        assert!(kinds.contains("job-worker-assignment"));
        assert!(kinds.contains("job-worker-status"));
        assert!(kinds.contains("job-worker-result"));
        assert!(kinds.contains("job-worker-receipt"));
        let receipt_text = to_text(&worker.receipt_value).expect("worker receipt text");
        let result_text = to_text(&worker.result.value).expect("worker result text");
        assert!(receipt_text.contains("transport-is-not-authority"));
        assert!(receipt_text.contains("recorded-delivery-log"));
        assert!(result_text.contains(&fixture.sync_ref));
        assert!(result_text.contains(&fixture.execution_request_ref));
    }

    #[test]
    fn worker_denies_missing_authority_stale_sync_target_mismatch_and_missing_artifact() {
        let fixture = worker_fixture("job-worker-deny");

        assert_missing_authority(&fixture);
        assert_missing_admission(&fixture);
        assert_denied_admission(&fixture);
        assert_stale_sync(&fixture);
        assert_target_mismatch(&fixture);
        assert_missing_artifact(&fixture);
    }

    fn assert_missing_authority(fixture: &WorkerFixture) {
        let request = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            target_peer: "peer:b",
            stage_ids: &fixture.admission.plan.stage_order,
            sync_ref: &fixture.sync_ref,
            admission_ref: &fixture.admission_ref,
            execution_request_ref: &fixture.execution_request_ref,
            authority_refs: &[],
            resource_refs: &fixture.resource_refs,
            peer_bootstrap_refs: std::slice::from_ref(&fixture.peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&fixture.node_identity_ref),
            evidence_refs: &fixture.evidence_refs,
        })
        .expect("missing authority worker request");
        let (delivery, log) =
            deliver_worker_request(&fixture.root.join("missing-authority-transport"), &request, "peer:b", true);
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("missing-authority-storage"),
            cache_root: &fixture.root.join("missing-authority-cache"),
            chunk_root: &fixture.root.join("missing-authority-chunks"),
            delivery: &delivery,
            delivery_log: Some(&log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("missing authority denial");
        assert_eq!(denied.result.decision, "deny");
        assert!(denied.execution.is_none());
        assert!(denied.result.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority")));
    }

    fn assert_missing_admission(fixture: &WorkerFixture) {
        let missing_admission_ref = test_ref("missing-worker-admission");
        let request = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            target_peer: "peer:b",
            stage_ids: &fixture.admission.plan.stage_order,
            sync_ref: &fixture.sync_ref,
            admission_ref: &missing_admission_ref,
            execution_request_ref: &fixture.execution_request_ref,
            authority_refs: std::slice::from_ref(&fixture.authority_context_ref),
            resource_refs: &fixture.resource_refs,
            peer_bootstrap_refs: std::slice::from_ref(&fixture.peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&fixture.node_identity_ref),
            evidence_refs: &[
                fixture.sync_ref.clone(),
                missing_admission_ref.clone(),
                fixture.execution_request_ref.clone(),
            ],
        })
        .expect("missing admission worker request");
        let (delivery, log) =
            deliver_worker_request(&fixture.root.join("missing-admission-transport"), &request, "peer:b", true);
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("missing-admission-storage"),
            cache_root: &fixture.root.join("missing-admission-cache"),
            chunk_root: &fixture.root.join("missing-admission-chunks"),
            delivery: &delivery,
            delivery_log: Some(&log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("missing admission denial");
        assert_eq!(denied.result.decision, "deny");
        assert!(denied.execution.is_none());
        assert!(denied.result.diagnostics.iter().any(|diagnostic| diagnostic.contains("admission receipt hashes")));
    }

    fn assert_denied_admission(fixture: &WorkerFixture) {
        let request_value = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            sync_ref: &fixture.sync_ref,
            stage_ids: &[],
            target_peer: "peer:b",
            policy_refs: &[],
            capability_refs: &[],
            evidence_refs: &[],
            resource_refs: &[],
        })
        .expect("denied admission request");
        let admission = admission_loopback(&fixture.target, &request_value).expect("denied admission");
        assert_eq!(admission.plan.decision, "deny");
        let admission_ref = canonical_hash(&admission.receipt_value).expect("denied admission ref");
        let execution_request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            admission_ref: &admission_ref,
            stage_ids: &admission.plan.stage_order,
            target_peer: "peer:b",
            storage_profile_ref: &test_ref("denied-worker-storage-profile"),
            cache_profile_ref: &test_ref("denied-worker-cache-profile"),
            chunk_profile_ref: &test_ref("denied-worker-chunk-profile"),
            policy_refs: &[],
            capability_refs: &[],
            resource_refs: &[],
        })
        .expect("denied execution request");
        let execution_request_ref = canonical_hash(&execution_request).expect("denied execution ref");
        let worker_request = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            target_peer: "peer:b",
            stage_ids: &admission.plan.stage_order,
            sync_ref: &fixture.sync_ref,
            admission_ref: &admission_ref,
            execution_request_ref: &execution_request_ref,
            authority_refs: &[],
            resource_refs: &[],
            peer_bootstrap_refs: std::slice::from_ref(&fixture.peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&fixture.node_identity_ref),
            evidence_refs: &[
                fixture.sync_ref.clone(),
                admission_ref.clone(),
                execution_request_ref.clone(),
            ],
        })
        .expect("denied worker request");
        let (delivery, log) =
            deliver_worker_request(&fixture.root.join("denied-admission-transport"), &worker_request, "peer:b", true);
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("denied-admission-storage"),
            cache_root: &fixture.root.join("denied-admission-cache"),
            chunk_root: &fixture.root.join("denied-admission-chunks"),
            delivery: &delivery,
            delivery_log: Some(&log),
            admission_receipt_value: &admission.receipt_value,
            execution_request_value: &execution_request,
            ledger_root: None,
        })
        .expect("denied worker");
        assert_eq!(denied.result.decision, "deny");
        assert!(denied.execution.is_none());
    }

    fn assert_stale_sync(fixture: &WorkerFixture) {
        let stale_sync = test_ref("stale-sync");
        let request = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            target_peer: "peer:b",
            stage_ids: &fixture.admission.plan.stage_order,
            sync_ref: &stale_sync,
            admission_ref: &fixture.admission_ref,
            execution_request_ref: &fixture.execution_request_ref,
            authority_refs: std::slice::from_ref(&fixture.authority_context_ref),
            resource_refs: &fixture.resource_refs,
            peer_bootstrap_refs: std::slice::from_ref(&fixture.peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&fixture.node_identity_ref),
            evidence_refs: &[
                stale_sync.clone(),
                fixture.admission_ref.clone(),
                fixture.execution_request_ref.clone(),
            ],
        })
        .expect("stale sync request");
        let (delivery, log) =
            deliver_worker_request(&fixture.root.join("stale-sync-transport"), &request, "peer:b", true);
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("stale-storage"),
            cache_root: &fixture.root.join("stale-cache"),
            chunk_root: &fixture.root.join("stale-chunks"),
            delivery: &delivery,
            delivery_log: Some(&log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("stale denial");
        assert_eq!(denied.result.decision, "deny");
        assert!(denied.execution.is_none());
        assert!(denied.result.diagnostics.iter().any(|diagnostic| diagnostic.contains("sync ref")));
    }

    fn assert_target_mismatch(fixture: &WorkerFixture) {
        let envelope = crate::remote_dataspace::build_envelope(crate::remote_dataspace::RemoteDataspaceEnvelopeInput {
            from_peer: "peer:a".to_string(),
            from_actor: "source-worker".to_string(),
            to_peer: "peer:c".to_string(),
            topic: "molten.job.worker".to_string(),
            operation: crate::remote_dataspace::RemoteDataspaceOperation::Message,
            payload: fixture.worker_request.clone(),
            content_refs: Vec::new(),
            capability_refs: vec![fixture.authority_context_ref.clone()],
            evidence_refs: fixture.evidence_refs.clone(),
        })
        .expect("target mismatch envelope");
        crate::remote_dataspace::publish_local_gossip(
            &fixture.root.join("target-mismatch-transport"),
            &envelope,
            "peer:a",
        )
        .expect("publish mismatch");
        let delivery = crate::remote_dataspace::deliver_local_gossip(
            &fixture.root.join("target-mismatch-transport"),
            "molten.job.worker",
            &envelope.envelope_ref,
            "peer:c",
        )
        .expect("deliver mismatch");
        let log = crate::remote_dataspace::delivery_log(std::slice::from_ref(&delivery), true)
            .expect("mismatch delivery log");
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("target-mismatch-storage"),
            cache_root: &fixture.root.join("target-mismatch-cache"),
            chunk_root: &fixture.root.join("target-mismatch-chunks"),
            delivery: &delivery,
            delivery_log: Some(&log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("target mismatch denial");
        assert_eq!(denied.result.decision, "deny");
        assert!(denied.execution.is_none());
    }

    fn assert_missing_artifact(fixture: &WorkerFixture) {
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.root.join("empty-target"),
            storage_root: &fixture.root.join("missing-artifact-storage"),
            cache_root: &fixture.root.join("missing-artifact-cache"),
            chunk_root: &fixture.root.join("missing-artifact-chunks"),
            delivery: &fixture.delivery,
            delivery_log: Some(&fixture.delivery_log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("missing target artifact denial");
        assert_eq!(denied.result.decision, "deny", "{:?}", denied.result.diagnostics);
        assert!(
            denied.execution.as_ref().is_some_and(|execution| execution.run.is_none()),
            "{:?}",
            denied.result.diagnostics
        );
    }

    #[test]
    fn live_unrecorded_worker_run_is_diagnostic_only() {
        let fixture = worker_fixture("job-worker-live");
        let live = live_unrecorded_worker_result(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("live-storage"),
            cache_root: &fixture.root.join("live-cache"),
            chunk_root: &fixture.root.join("live-chunks"),
            delivery: &fixture.delivery,
            delivery_log: Some(&fixture.delivery_log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("live diagnostic worker");
        assert_eq!(live.result.decision, "non-replayable", "{:?}", live.result.diagnostics);
        assert!(live.execution.as_ref().is_some_and(|execution| execution.decision == "pass"));
        assert!(live.result.diagnostics.iter().any(|diagnostic| diagnostic.contains("delivery log")));
    }

    #[hegel::test(test_cases = 4)]
    fn hegel_worker_request_identity_recorded_replay_and_no_source_state(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(10_000));
        let job_ref = test_ref(&format!("worker-job-{salt}"));
        let sync_ref = test_ref(&format!("worker-sync-{salt}"));
        let admission_ref = test_ref(&format!("worker-admission-{salt}"));
        let execution_request_ref = test_ref(&format!("worker-exec-{salt}"));
        let authority_ref = test_ref(&format!("worker-authority-{salt}"));
        let resource_ref = test_ref(&format!("worker-resource-{salt}"));
        let peer_bootstrap_ref = test_ref(&format!("worker-bootstrap-{salt}"));
        let node_identity_ref = test_ref(&format!("worker-node-{salt}"));
        let evidence = vec![sync_ref.clone(), admission_ref.clone(), execution_request_ref.clone()];
        let first = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &job_ref,
            target_peer: "peer:b",
            stage_ids: &["stage".to_string()],
            sync_ref: &sync_ref,
            admission_ref: &admission_ref,
            execution_request_ref: &execution_request_ref,
            authority_refs: std::slice::from_ref(&authority_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            peer_bootstrap_refs: std::slice::from_ref(&peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&node_identity_ref),
            evidence_refs: &evidence,
        })
        .expect("first request");
        let second = parse_text(&to_text(&first).expect("request text")).expect("reparse request");
        let parsed = parse_job_worker_request_value(&second).expect("parse worker request");
        assert_eq!(canonical_hash(&first).expect("first ref"), parsed.request_ref);
        let request_text = to_text(&first).expect("worker request text");
        assert!(!request_text.contains("source-registry"));
        assert!(request_text.contains("target-state-only"));
        assert!(request_text.contains(&sync_ref));
    }

    #[hegel::test(test_cases = 4)]
    fn hegel_blob_ref_submission_rejects_inline_tokens_and_records_pin_lifecycle(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(10_000));
        let token_selector = tc.draw(generators::integers::<u64>().min_value(0).max_value(2));
        let token = match token_selector {
            0 => "inline-bytes",
            1 => "inline-executable",
            _ => "inline-dataset",
        };
        let operation_id = test_ref(&format!("job-ref-hegel-operation-{salt}"));
        let authority_ref = test_ref(&format!("job-ref-hegel-authority-{salt}"));
        let inline_value = record("job-ref-submission-v1", vec![
            string(JOB_REF_SUBMISSION_SCHEMA),
            record("job-id", vec![string(format!("job-ref-hegel-{salt}"))]),
            record("operation-id", vec![string(&operation_id)]),
            record("executable", vec![record(token, vec![string("bytes")])]),
            record("inputs", vec![sequence(vec![])]),
            record("output-mode", vec![string("chunk-manifest")]),
            record("input-schemas", vec![refs_sequence(&[])]),
            record("output-schemas", vec![refs_sequence(&[])]),
            record("effects", vec![refs_sequence(&[])]),
            record("handler-profile", vec![string("local-echo-v1")]),
            record("authority", vec![string(&authority_ref)]),
            record("policy", vec![refs_sequence(&[])]),
            record("provenance", vec![refs_sequence(&[])]),
            record("evidence", vec![refs_sequence(&[])]),
            checks_value(&["content-refs-only", "no-inline-large-bytes"]),
        ]);
        assert!(parse_job_ref_submission_value(&inline_value).is_err());

        let root = temp_dir(&format!("job-ref-hegel-{salt}"));
        let chunks = root.join("chunks");
        let executable = chunk_store::put_bytes(&chunks, "job-executable", b"echo", DEFAULT_FIXED_V1_CHUNK_SIZE)
            .expect("put executable");
        let input_bytes = format!("input-{salt}");
        let input = chunk_store::put_bytes(&chunks, "job-input", input_bytes.as_bytes(), DEFAULT_FIXED_V1_CHUNK_SIZE)
            .expect("put input");
        let policy_ref = test_ref(&format!("job-ref-hegel-policy-{salt}"));
        let provenance_ref = test_ref(&format!("job-ref-hegel-provenance-{salt}"));
        let effect_ref = test_ref(&format!("job-ref-hegel-effect-{salt}"));
        let submission_value = job_ref_submission_value(BlobRefJobSubmissionValueInput {
            job_id: &format!("job-ref-hegel-{salt}"),
            operation_id: &operation_id,
            executable: JobContentRef {
                content_ref: executable.manifest_ref.clone(),
                size: executable.total_len,
                format: "elf-executable".to_string(),
                schema_ref: None,
            },
            inputs: vec![JobContentRef {
                content_ref: input.manifest_ref.clone(),
                size: input.total_len,
                format: "bytes".to_string(),
                schema_ref: None,
            }],
            output_mode: "chunk-manifest",
            input_schema_refs: &[],
            output_schema_refs: &[],
            effect_manifest_refs: std::slice::from_ref(&effect_ref),
            handler_profile: "local-echo-v1",
            authority_context_ref: &authority_ref,
            policy_refs: std::slice::from_ref(&policy_ref),
            provenance_refs: std::slice::from_ref(&provenance_ref),
            evidence_refs: &[],
        })
        .expect("submission");
        let executed = execute_blob_ref_job(BlobRefJobExecuteInput {
            chunk_root: &chunks,
            submission_value: &submission_value,
            ledger_root: None,
        })
        .expect("execute");
        assert_eq!(executed.decision, "pass");
        let receipt_text = to_text(&executed.receipt_value).expect("receipt text");
        assert!(receipt_text.contains("content-verification-before-run"));
        assert!(receipt_text.contains("retention-pins"));
        assert!(receipt_text.contains("cleanup-receipts"));
    }

    #[test]
    fn remote_admission_denies_missing_targets_and_non_artifact_stages() {
        let root = temp_dir("job-admit-deny");
        let registry = root.join("registry");
        let dag_value = pipeline_dag().expect("dag");
        let installed = install_job_dag(&registry, &dag_value).expect("install dag");
        let sync_ref = test_ref("sync-receipt");
        let request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &installed.job_ref,
            sync_ref: &sync_ref,
            stage_ids: &[],
            target_peer: "peer:loopback",
            policy_refs: &[test_ref("policy")],
            capability_refs: &[test_ref("capability")],
            evidence_refs: &[sync_ref.clone(), test_ref("octet-gate")],
            resource_refs: &[
                test_ref("resource-a"),
                test_ref("resource-b"),
                test_ref("resource-c"),
                test_ref("resource-d"),
            ],
        })
        .expect("request");
        let denied = admission_plan_value(&registry, &request).expect("admission denial");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("artifact-backed executable")));

        let missing = admission_plan_value(&root.join("empty-target"), &request).expect("missing target denial");
        assert_eq!(missing.decision, "deny");
        assert!(missing.diagnostics.iter().any(|diagnostic| diagnostic.contains("target job not available")));
    }

    #[test]
    fn planning_profile_and_fusion_preview_are_canonical_and_conservative() {
        let root = temp_dir("job-planning");
        let dag_value = pipeline_dag().expect("dag");
        let dag = parse_job_dag_value(&dag_value).expect("parse dag");
        let plan = plan_job_dag(&dag, None).expect("plan");
        assert_eq!(plan.stage_order.first(), Some(&"source".to_string()));
        assert!(to_text(&plan.value).expect("plan text").contains("trellis-topo-order"));
        let profile = profile_job_dag(&dag, None, Some(&root.join("cache"))).expect("profile");
        assert_eq!(profile.stage_count, 4);
        assert!(to_text(&profile.value).expect("profile text").contains("no-wall-clock-time"));
        let fusion = fusion_preview_job_dag(&dag, None).expect("fusion");
        assert!(fusion.chains.iter().any(|chain| chain == &vec!["filter".to_string(), "map".to_string()]));

        let policy_ref = test_ref("fusion-policy");
        let left = job_node_value(NodeValueInput {
            id: "left",
            kind: "map",
            stage_artifact_ref: None,
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: record("op", vec![string("identity")]),
            effect_manifest_refs: &[],
            policy_refs: std::slice::from_ref(&policy_ref),
            evidence_refs: &[],
        })
        .expect("left");
        let right = test_node_value(
            "right",
            "filter",
            &["in".to_string()],
            &["out".to_string()],
            record("op", vec![string("keep-all")]),
        )
        .expect("right");
        let edge = stream_edge_value("left", "right").expect("edge");
        let boundary = test_dag_value(vec![left, right], vec![edge], &["right".to_string()]).expect("boundary dag");
        let boundary = parse_job_dag_value(&boundary).expect("boundary parse");
        assert!(fusion_preview_job_dag(&boundary, None).expect("boundary fusion").chains.is_empty());
    }

    #[test]
    fn raw_closure_config_denies_before_execution() {
        let source = test_node_value(
            "source",
            "source",
            &[],
            &["out".to_string()],
            record("source", vec![record("values", vec![sequence(vec![string("ok")])])]),
        )
        .expect("source");
        let bad = test_node_value(
            "bad",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            record("host-path", vec![string("/bin/echo")]),
        );
        assert!(bad.expect_err("bad config").to_string().contains("mobile/ambient"));
        let edge = stream_edge_value("source", "bad").expect("edge");
        let bad_node = record("job-node-v1", vec![
            string(JOB_DAG_NODE_SCHEMA),
            record("id", vec![string("bad")]),
            record("kind", vec![string("map")]),
            record("stage-artifact", vec![record("none", Vec::new())]),
            record("inputs", vec![ports_sequence(&["in".to_string()])]),
            record("outputs", vec![ports_sequence(&["out".to_string()])]),
            record("config", vec![record("host-path", vec![string("/bin/echo")])]),
            record("effects", vec![sequence(Vec::new())]),
            record("policy", vec![sequence(Vec::new())]),
            record("evidence", vec![sequence(Vec::new())]),
            checks_value(&["stage-artifact-not-closure"]),
        ]);
        let dag = test_dag_value(vec![source, bad_node], vec![edge], &["bad".to_string()]).expect("dag");
        assert!(parse_job_dag_value(&dag).expect_err("parse rejects").to_string().contains("mobile/ambient"));
    }

    #[hegel::test(test_cases = 10)]
    fn hegel_dag_hash_and_memo_key_are_stable(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let dag = fixture_dag(if salt.is_multiple_of(2) { "identity" } else { "count" });
        let first = parse_job_dag_value(&dag).expect("first");
        let second =
            parse_job_dag_value(&parse_text(&to_text(&dag).expect("text")).expect("parse text")).expect("second");
        assert_eq!(first.job_ref, second.job_ref);
        assert_eq!(
            execution_order(&first.nodes, &first.edges).expect("order"),
            execution_order(&second.nodes, &second.edges).expect("order")
        );
    }

    fn pipeline_dag() -> Result<IOValue> {
        let source = test_node_value(
            "source",
            "source",
            &[],
            &["out".to_string()],
            record("source", vec![record("values", vec![sequence(vec![
                record("keep", vec![string("a")]),
                record("drop", vec![string("b")]),
                record("keep", vec![string("c")]),
            ])])]),
        )?;
        let filter = test_node_value(
            "filter",
            "filter",
            &["in".to_string()],
            &["out".to_string()],
            record("op", vec![string("match-record"), string("keep")]),
        )?;
        let map = test_node_value(
            "map",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            record("op", vec![string("wrap"), string("item")]),
        )?;
        let materialize = test_node_value(
            "out",
            "materialize",
            &["in".to_string()],
            &["out".to_string()],
            record("materialize", vec![string("inline")]),
        )?;
        let e1 = stream_edge_value("source", "filter")?;
        let e2 = stream_edge_value("filter", "map")?;
        let e3 = stream_edge_value("map", "out")?;
        test_dag_value(vec![source, filter, map, materialize], vec![e1, e2, e3], &["out".to_string()])
    }

    fn fixture_dag(operation: &str) -> IOValue {
        let source = test_node_value(
            "source",
            "source",
            &[],
            &["out".to_string()],
            record("source", vec![record("values", vec![sequence(vec![string("a"), string("b")])])]),
        )
        .expect("source");
        let stage_kind = if operation == "count" { "reduce" } else { "map" };
        let stage = test_node_value(
            "stage",
            stage_kind,
            &["in".to_string()],
            &["out".to_string()],
            record("op", vec![string(operation)]),
        )
        .expect("stage");
        let edge = stream_edge_value("source", "stage").expect("edge");
        test_dag_value(vec![source, stage], vec![edge], &["stage".to_string()]).expect("dag")
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("job-dag-test-ref", vec![string(label)])).expect("test ref")
    }

    fn reviewed_provenance_values(artifact_refs: &[String]) -> Vec<IOValue> {
        artifact_refs
            .iter()
            .map(|artifact_ref| {
                crate::provenance::synthetic_reviewed_provenance_record(artifact_ref).expect("reviewed provenance")
            })
            .collect()
    }

    fn install_clean_octet_gate(registry: &Path) -> String {
        let gate_value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean octet gate fixture");
        let gate_ref = canonical_hash(&gate_value).expect("octet gate ref");
        let install = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
            kind: "octet-gate-receipt".to_string(),
            payload: gate_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("octet-policy")],
            evidence_refs: vec![test_ref("octet-evidence")],
            installer_ref: test_ref("octet-installer"),
            capability_refs: vec![test_ref("octet-capability")],
        })
        .expect("install octet gate");
        assert_eq!(install.decision, "pass");
        gate_ref
    }

    fn install_job_execute_authority_context(registry: &Path, job_ref: &str) -> String {
        let subject_ref = test_ref("target-peer-subject");
        let context_value = authority::authority_context_value(authority::ContextValueInput {
            subject_ref: &subject_ref,
            capabilities: &[authority::AuthorityCapability {
                capability: "job:execute".to_string(),
                scope: job_ref.to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[test_ref("authority-policy")],
            evidence_refs: &[test_ref("authority-evidence")],
        })
        .expect("authority context");
        let context_ref = canonical_hash(&context_value).expect("authority context ref");
        let install = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
            kind: "authority-context".to_string(),
            payload: context_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("authority-policy")],
            evidence_refs: vec![test_ref("authority-evidence")],
            installer_ref: test_ref("authority-installer"),
            capability_refs: vec![test_ref("authority-install-capability")],
        })
        .expect("install authority context");
        assert_eq!(install.decision, "pass");
        context_ref
    }

    struct WorkerFixture {
        root: PathBuf,
        source: PathBuf,
        target: PathBuf,
        ledger: PathBuf,
        installed_job: JobInstall,
        admission: JobAdmissionLoopback,
        sync_ref: String,
        admission_ref: String,
        execution_request_ref: String,
        execution_request: IOValue,
        authority_context_ref: String,
        resource_refs: Vec<String>,
        peer_bootstrap_ref: String,
        node_identity_ref: String,
        evidence_refs: Vec<String>,
        worker_request: IOValue,
        delivery: crate::remote_dataspace::RemoteDataspaceDelivery,
        delivery_log: crate::remote_dataspace::RemoteDeliveryLog,
    }

    struct SeedArtifacts {
        base: artifacts::ArtifactInstall,
        source_stage: artifacts::ArtifactInstall,
        map_stage: artifacts::ArtifactInstall,
    }

    struct FlowParts {
        authority_context_ref: String,
        resource_refs: Vec<String>,
        admission: JobAdmissionLoopback,
        admission_ref: String,
        execution_request: IOValue,
        execution_request_ref: String,
    }

    struct RequestParts {
        peer_bootstrap_ref: String,
        node_identity_ref: String,
        evidence_refs: Vec<String>,
        worker_request: IOValue,
    }

    fn seed_artifacts(source: &Path) -> SeedArtifacts {
        let base = artifacts::install_artifact(source, &artifacts::ArtifactInstallInput {
            kind: "schema".to_string(),
            payload: record("schema", vec![string("worker-base")]),
            schema_refs: vec![test_ref("worker-schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("worker-policy")],
            evidence_refs: vec![test_ref("worker-evidence")],
            installer_ref: test_ref("worker-installer"),
            capability_refs: vec![test_ref("worker-capability")],
        })
        .expect("install worker base");
        let source_stage = artifacts::install_artifact(source, &artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: builtin_stage_operation_value("source").expect("source operation"),
            schema_refs: vec![test_ref("worker-stage-schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("worker-stage-policy")],
            evidence_refs: vec![test_ref("worker-stage-evidence")],
            installer_ref: test_ref("worker-stage-installer"),
            capability_refs: vec![test_ref("worker-stage-capability")],
        })
        .expect("install worker source stage");
        let map_stage = artifacts::install_artifact(source, &artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: builtin_stage_operation_value("identity").expect("identity operation"),
            schema_refs: vec![test_ref("worker-stage-schema")],
            dependency_refs: vec![base.artifact_ref.clone()],
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("worker-stage-policy")],
            evidence_refs: vec![test_ref("worker-stage-evidence")],
            installer_ref: test_ref("worker-stage-installer"),
            capability_refs: vec![test_ref("worker-stage-capability")],
        })
        .expect("install worker map stage");
        SeedArtifacts {
            base,
            source_stage,
            map_stage,
        }
    }

    fn seed_graph(source: &Path, seed: &SeedArtifacts) -> JobInstall {
        let source_node = job_node_value(NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: Some(&seed.source_stage.artifact_ref),
            input_ports: &[],
            output_ports: &["out".to_string()],
            config: record("source", vec![record("values", vec![sequence(vec![string("remote-x")])])]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("worker source node");
        let map_node = job_node_value(NodeValueInput {
            id: "map",
            kind: "map",
            stage_artifact_ref: Some(&seed.map_stage.artifact_ref),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: record("op", vec![string("identity")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("worker map node");
        let edge = stream_edge_value("source", "map").expect("worker edge");
        let dag_value =
            test_dag_value(vec![source_node, map_node], vec![edge], &["map".to_string()]).expect("worker dag value");
        install_job_dag(source, &dag_value).expect("install worker job")
    }

    fn synced_ref(source: &Path, target: &Path, installed: &JobInstall, seed: &SeedArtifacts) -> String {
        let sync_request = job_sync_request_value(SyncRequestValueInput {
            job_ref: &installed.job_ref,
            stage_ids: &[],
            target_peer: "peer:b",
            policy_refs: &[test_ref("worker-sync-policy")],
            capability_refs: &[test_ref("worker-sync-capability")],
            evidence_refs: &[test_ref("worker-sync-evidence")],
        })
        .expect("worker sync request");
        let sync_provenance = reviewed_provenance_values(&[
            seed.base.artifact_ref.clone(),
            seed.source_stage.artifact_ref.clone(),
            seed.map_stage.artifact_ref.clone(),
            installed.artifact_ref.clone(),
        ]);
        let synced = sync_loopback(SyncLoopbackInput {
            source_registry: source,
            target_registry: target,
            request_value: &sync_request,
            provenance_values: &sync_provenance,
            build_verification_values: &[],
        })
        .expect("worker sync loopback");
        canonical_hash(&synced.receipt_value).expect("worker sync ref")
    }

    fn flow_parts(target: &Path, installed: &JobInstall, sync_ref: &str) -> FlowParts {
        let authority_context_ref = install_job_execute_authority_context(target, &installed.job_ref);
        let source_gate_ref = install_clean_octet_gate(target);
        let resource_refs = vec![test_ref("worker-resource-a"), test_ref("worker-resource-b")];
        let admission_request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &installed.job_ref,
            sync_ref,
            stage_ids: &[],
            target_peer: "peer:b",
            policy_refs: &[test_ref("worker-admission-policy")],
            capability_refs: std::slice::from_ref(&authority_context_ref),
            evidence_refs: &[sync_ref.to_string(), source_gate_ref],
            resource_refs: &resource_refs,
        })
        .expect("worker admission request");
        let admission = admission_loopback(target, &admission_request).expect("worker admission");
        assert_eq!(admission.plan.decision, "pass");
        let admission_ref = canonical_hash(&admission.receipt_value).expect("worker admission ref");
        let execution_request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &installed.job_ref,
            admission_ref: &admission_ref,
            stage_ids: &admission.plan.stage_order,
            target_peer: "peer:b",
            storage_profile_ref: &test_ref("worker-storage-profile"),
            cache_profile_ref: &test_ref("worker-cache-profile"),
            chunk_profile_ref: &test_ref("worker-chunk-profile"),
            policy_refs: &[test_ref("worker-admission-policy")],
            capability_refs: std::slice::from_ref(&authority_context_ref),
            resource_refs: &resource_refs,
        })
        .expect("worker execution request");
        let execution_request_ref = canonical_hash(&execution_request).expect("worker execution request ref");
        FlowParts {
            authority_context_ref,
            resource_refs,
            admission,
            admission_ref,
            execution_request,
            execution_request_ref,
        }
    }

    fn request_parts(installed: &JobInstall, sync_ref: &str, flow: &FlowParts) -> RequestParts {
        let peer_bootstrap_ref = test_ref("worker-peer-bootstrap");
        let node_identity_ref = test_ref("worker-node-identity");
        let evidence_refs = vec![
            sync_ref.to_string(),
            flow.admission_ref.clone(),
            flow.execution_request_ref.clone(),
            peer_bootstrap_ref.clone(),
            node_identity_ref.clone(),
        ];
        let worker_request = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &installed.job_ref,
            target_peer: "peer:b",
            stage_ids: &flow.admission.plan.stage_order,
            sync_ref,
            admission_ref: &flow.admission_ref,
            execution_request_ref: &flow.execution_request_ref,
            authority_refs: std::slice::from_ref(&flow.authority_context_ref),
            resource_refs: &flow.resource_refs,
            peer_bootstrap_refs: std::slice::from_ref(&peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&node_identity_ref),
            evidence_refs: &evidence_refs,
        })
        .expect("worker request");
        RequestParts {
            peer_bootstrap_ref,
            node_identity_ref,
            evidence_refs,
            worker_request,
        }
    }

    fn worker_fixture(name: &str) -> WorkerFixture {
        let root = temp_dir(name);
        let source = root.join("source");
        let target = root.join("target");
        let ledger = root.join("worker-ledger");
        let seed = seed_artifacts(&source);
        let installed_job = seed_graph(&source, &seed);
        let sync_ref = synced_ref(&source, &target, &installed_job, &seed);
        let flow = flow_parts(&target, &installed_job, &sync_ref);
        let request = request_parts(&installed_job, &sync_ref, &flow);
        let (delivery, delivery_log) =
            deliver_worker_request(&root.join("transport"), &request.worker_request, "peer:b", true);
        let FlowParts {
            authority_context_ref,
            resource_refs,
            admission,
            admission_ref,
            execution_request,
            execution_request_ref,
        } = flow;
        let RequestParts {
            peer_bootstrap_ref,
            node_identity_ref,
            evidence_refs,
            worker_request,
        } = request;
        WorkerFixture {
            root,
            source,
            target,
            ledger,
            installed_job,
            admission,
            sync_ref,
            admission_ref,
            execution_request_ref,
            execution_request,
            authority_context_ref,
            resource_refs,
            peer_bootstrap_ref,
            node_identity_ref,
            evidence_refs,
            worker_request,
            delivery,
            delivery_log,
        }
    }

    fn deliver_worker_request(
        transport_root: &Path,
        request_value: &IOValue,
        target_peer: &str,
        replayable: bool,
    ) -> (crate::remote_dataspace::RemoteDataspaceDelivery, crate::remote_dataspace::RemoteDeliveryLog) {
        let envelope = job_worker_envelope(JobWorkerEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "source-worker",
            to_peer: target_peer,
            topic: "molten.job.worker",
            request_value,
        })
        .expect("worker envelope");
        crate::remote_dataspace::publish_local_gossip(transport_root, &envelope, "peer:a")
            .expect("publish worker request");
        let delivery = crate::remote_dataspace::deliver_local_gossip(
            transport_root,
            "molten.job.worker",
            &envelope.envelope_ref,
            target_peer,
        )
        .expect("deliver worker request");
        let delivery_log = crate::remote_dataspace::delivery_log(std::slice::from_ref(&delivery), replayable)
            .expect("worker delivery log");
        (delivery, delivery_log)
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
