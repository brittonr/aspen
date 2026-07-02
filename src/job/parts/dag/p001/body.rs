
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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobExecutionLoopback {
    pub receipt_ref: String,
    pub request: JobExecutionRequest,
    pub admission: JobAdmissionReceipt,
    pub run: Option<JobRun>,
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_value: IoValue,
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
    pub context_ref: String,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobRefJobExecution {
    pub submission: BlobRefJobSubmission,
    pub decision: String,
    pub status_values: Vec<IoValue>,
    pub output_manifest_ref: Option<String>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
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
    pub context_ref: &'a str,
    pub policy_refs: &'a [String],
    pub provenance_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct BlobRefJobExecuteInput<'a> {
    pub chunk_root: &'a FilePath,
    pub submission_value: &'a IoValue,
    pub ledger_root: Option<&'a FilePath>,
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
    pub value: IoValue,
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
    pub value: IoValue,
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
    pub value: IoValue,
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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobWorkerExecution {
    pub request: Option<JobWorkerRequest>,
    pub assignment_value: IoValue,
    pub status_values: Vec<IoValue>,
    pub result: JobWorkerResult,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
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
    pub request_value: &'a IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct JobWorkerExecuteInput<'a> {
    pub target_registry: &'a FilePath,
    pub storage_root: &'a FilePath,
    pub cache_root: &'a FilePath,
    pub chunk_root: &'a FilePath,
    pub delivery: &'a crate::remote_dataspace::Delivery,
    pub delivery_log: Option<&'a crate::remote_dataspace::DeliveryLog>,
    pub admission_receipt_value: &'a IoValue,
    pub execution_request_value: &'a IoValue,
    pub ledger_root: Option<&'a FilePath>,
}

#[derive(Debug, Clone)]
pub struct NodeValueInput<'a> {
    pub id: &'a str,
    pub kind: &'a str,
    pub stage_artifact_ref: Option<&'a str>,
    pub input_ports: &'a [String],
    pub output_ports: &'a [String],
    pub config: IoValue,
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
    pub nodes: Vec<IoValue>,
    pub edges: Vec<IoValue>,
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
