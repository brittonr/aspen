
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
    pub target_registry: &'a FilePath,
    pub storage_root: &'a FilePath,
    pub cache_root: &'a FilePath,
    pub chunk_root: &'a FilePath,
    pub admission_receipt_value: &'a IoValue,
    pub request_value: &'a IoValue,
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
    status_values: Vec<IoValue>,
}

struct FinishInput<'a> {
    ledger_root: Option<&'a FilePath>,
    submission: BlobRefJobSubmission,
    status_values: Vec<IoValue>,
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
    delivery: &'a crate::remote_dataspace::Delivery,
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

pub fn job_node_value(input: NodeValueInput<'_>) -> Result<IoValue> {
    validate_node_id(input.id)?;
    validate_stage_kind(input.kind)?;
    if let Some(stage_artifact_ref) = input.stage_artifact_ref {
        validate_ref(stage_artifact_ref, "job stage artifact ref")?;
    }
    validate_refs(input.effect_manifest_refs, "job node effect manifest ref")?;
    validate_refs(input.policy_refs, "job node policy ref")?;
    validate_refs(input.evidence_refs, "job node evidence ref")?;
    reject_mobile_closure_config(&input.config)?;
    Ok(crate::preserves_rail::record("job-node-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_DAG_NODE_SCHEMA),
        crate::preserves_rail::record("id", vec![crate::preserves_rail::string(input.id)]),
        crate::preserves_rail::record("kind", vec![crate::preserves_rail::string(input.kind)]),
        crate::preserves_rail::record("stage-artifact", vec![optional_ref_value(input.stage_artifact_ref)]),
        crate::preserves_rail::record("inputs", vec![ports_sequence(input.input_ports)]),
        crate::preserves_rail::record("outputs", vec![ports_sequence(input.output_ports)]),
        crate::preserves_rail::record("config", vec![input.config]),
        crate::preserves_rail::record("effects", vec![refs_sequence(&sorted_unique(input.effect_manifest_refs))]),
        crate::preserves_rail::record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        crate::preserves_rail::record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        checks_value(&[
            "stage-artifact-not-closure",
            "bounded-stage-kind",
            "explicit-effect-boundary",
        ]),
    ]))
}

pub fn job_edge_value(input: EdgeValueInput<'_>) -> Result<IoValue> {
    validate_node_id(input.from_node)?;
    validate_node_id(input.to_node)?;
    validate_non_empty(input.from_port, "job edge from port")?;
    validate_non_empty(input.to_port, "job edge to port")?;
    if let Some(schema_ref) = input.schema_ref {
        validate_ref(schema_ref, "job edge schema ref")?;
    }
    validate_partitioning(input.partitioning)?;
    validate_materialization(input.materialization)?;
    Ok(crate::preserves_rail::record("job-edge-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_DAG_EDGE_SCHEMA),
        crate::preserves_rail::record("from", vec![
            crate::preserves_rail::string(input.from_node),
            crate::preserves_rail::string(input.from_port),
        ]),
        crate::preserves_rail::record("to", vec![
            crate::preserves_rail::string(input.to_node),
            crate::preserves_rail::string(input.to_port),
        ]),
        crate::preserves_rail::record("schema", vec![optional_ref_value(input.schema_ref)]),
        crate::preserves_rail::record("partitioning", vec![crate::preserves_rail::string(input.partitioning)]),
        crate::preserves_rail::record("materialization", vec![crate::preserves_rail::string(input.materialization)]),
        checks_value(&["schema-bound", "canonical-edge", "explicit-materialization"]),
    ]))
}

pub fn job_dag_value(input: DagValueInput<'_>) -> Result<IoValue> {
    validate_refs(input.schema_refs, "job schema ref")?;
    validate_refs(input.effect_manifest_refs, "job effect manifest ref")?;
    validate_refs(input.policy_refs, "job policy ref")?;
    validate_refs(input.evidence_refs, "job evidence ref")?;
    Ok(crate::preserves_rail::record("job-dag-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_DAG_SCHEMA),
        crate::preserves_rail::record("version", vec![crate::preserves_rail::string("v1")]),
        crate::preserves_rail::record("nodes", vec![crate::preserves_rail::sequence(input.nodes)]),
        crate::preserves_rail::record("edges", vec![crate::preserves_rail::sequence(input.edges)]),
        crate::preserves_rail::record("outputs", vec![crate::preserves_rail::sequence(
            input.output_roots.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("schemas", vec![refs_sequence(&sorted_unique(input.schema_refs))]),
        crate::preserves_rail::record("effect-manifests", vec![refs_sequence(&sorted_unique(
            input.effect_manifest_refs,
        ))]),
        crate::preserves_rail::record("policies", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        crate::preserves_rail::record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        checks_value(&[
            "canonical-dag",
            "no-name-identity",
            "stage-artifacts-explicit",
            "deterministic-local-profile",
        ]),
    ]))
}

pub fn job_output_request_value(input: OutputRequestValueInput<'_>) -> Result<IoValue> {
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
    Ok(crate::preserves_rail::record("job-output-request-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_DAG_OUTPUT_REQUEST_SCHEMA),
        crate::preserves_rail::record("dag", vec![crate::preserves_rail::string(input.dag_ref)]),
        crate::preserves_rail::record("roots", vec![crate::preserves_rail::sequence(
            input.roots.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("materialization", vec![crate::preserves_rail::string(input.materialization)]),
        crate::preserves_rail::record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        crate::preserves_rail::record("handler-profile", vec![optional_ref_value(input.handler_profile_ref)]),
        crate::preserves_rail::record("seed-config", vec![optional_ref_value(input.seed_config_ref)]),
        checks_value(&["request-ref-bound", "full-ref-identity", "deterministic-inputs-bound"]),
    ]))
}

pub fn builtin_stage_operation_value(operation: &str) -> Result<IoValue> {
    validate_stage_operation(operation)?;
    Ok(crate::preserves_rail::record("job-stage-operation-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_STAGE_OPERATION_SCHEMA),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(operation)]),
        checks_value(&["bounded-built-in", "no-mobile-closure", "canonical-operation"]),
    ]))
}

pub fn builtin_stage_operation_ref(operation: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&builtin_stage_operation_value(operation)?)
}

pub fn job_sync_request_value(input: SyncRequestValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.job_ref, "job sync request job ref")?;
    for stage_id in input.stage_ids {
        validate_node_id(stage_id)?;
    }
    validate_non_empty(input.target_peer, "job sync target peer")?;
    validate_refs(input.policy_refs, "job sync policy ref")?;
    validate_refs(input.capability_refs, "job sync capability ref")?;
    validate_refs(input.evidence_refs, "job sync evidence ref")?;
    Ok(crate::preserves_rail::record("job-sync-request-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::JOB_SYNC_REQUEST_SCHEMA),
        crate::preserves_rail::record("job", vec![crate::preserves_rail::string(input.job_ref)]),
        crate::preserves_rail::record("stages", vec![crate::preserves_rail::sequence(
            input.stage_ids.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("target-peer", vec![crate::preserves_rail::string(input.target_peer)]),
        crate::preserves_rail::record("policy", vec![refs_sequence(&sorted_unique(input.policy_refs))]),
        crate::preserves_rail::record("capability", vec![refs_sequence(&sorted_unique(input.capability_refs))]),
        crate::preserves_rail::record("evidence", vec![refs_sequence(&sorted_unique(input.evidence_refs))]),
        checks_value(&["transport-neutral", "no-execution", "full-ref-identity"]),
    ]))
}
