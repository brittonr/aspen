type FilePath = std::path::Path;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;
type Record<T> = preserves::Record<T>;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

const DEFAULT_FIXED_V1_CHUNK_SIZE: u64 = crate::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE;

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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobNode {
    pub id: String,
    pub kind: String,
    pub stage_artifact_ref: Option<String>,
    pub input_ports: Vec<String>,
    pub output_ports: Vec<String>,
    pub config: IoValue,
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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobInstall {
    pub job_ref: String,
    pub artifact_ref: String,
    pub decision: String,
    pub receipt_value: IoValue,
    pub artifact_receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRunOptions<'a> {
    pub registry_root: &'a FilePath,
    pub storage_root: &'a FilePath,
    pub cache_root: &'a FilePath,
    pub chunk_root: &'a FilePath,
    pub ledger_root: Option<&'a FilePath>,
    pub output_request: Option<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRun {
    pub job_ref: String,
    pub request_ref: String,
    pub stage_receipt_refs: Vec<String>,
    pub output_refs: Vec<String>,
    pub output_value: IoValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobStageRun {
    pub node_id: String,
    pub output_values: Vec<IoValue>,
    pub output_refs: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrellisExecutionPlan {
    order_ids: Vec<String>,
    node_index: OrderedMap<String, usize>,
    dependency_indices: OrderedMap<String, Vec<u64>>,
}

struct PlanMapping {
    node_ids: Vec<String>,
    node_index: OrderedMap<String, usize>,
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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobPlan {
    pub plan_ref: String,
    pub job_ref: String,
    pub request_ref: String,
    pub stage_order: Vec<String>,
    pub value: IoValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobProfile {
    pub profile_ref: String,
    pub job_ref: String,
    pub request_ref: String,
    pub stage_count: u64,
    pub edge_count: u64,
    pub materialization_boundaries: u64,
    pub value: IoValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobFusionPreview {
    pub fusion_ref: String,
    pub job_ref: String,
    pub request_ref: String,
    pub chains: Vec<Vec<String>>,
    pub value: IoValue,
    pub receipt_value: IoValue,
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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSyncPlan {
    pub plan_ref: String,
    pub request: JobSyncRequest,
    pub root_refs: Vec<String>,
    pub closure_refs: Vec<String>,
    pub missing_refs: Vec<String>,
    pub value: IoValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct SyncLoopbackInput<'a> {
    pub source_registry: &'a FilePath,
    pub target_registry: &'a FilePath,
    pub request_value: &'a IoValue,
    pub provenance_values: &'a [IoValue],
    pub build_verification_values: &'a [IoValue],
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
    pub receipt_value: IoValue,
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
    pub value: IoValue,
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
    pub value: IoValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobAdmissionLoopback {
    pub receipt_ref: String,
    pub plan: JobAdmissionPlan,
    pub receipt_value: IoValue,
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
    pub value: IoValue,
}
