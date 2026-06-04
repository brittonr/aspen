use std::fs;
use std::path::Path;
use std::path::PathBuf;

use preserves::IOValue;

use crate::artifacts;
use crate::error::MoltenError;
use crate::error::Result;
use crate::job_dag;
use crate::ledger;
use crate::node_identity;
use crate::node_runtime;
use crate::octet_gate;
use crate::preserves_rail::NODE_CONTROL_HEARTBEAT_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_LOOP_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_OPERATION_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_QUEUE_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_text;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::to_text;
use crate::provenance;

const CONFIG_FILE: &str = "config.preserves";
const STARTUP_FILE: &str = "startup-receipt.preserves";
const HEALTH_FILE: &str = "health-receipt.preserves";
const SHUTDOWN_FILE: &str = "shutdown-receipt.preserves";
const CONTROL_STATUS_FILE: &str = "status-control-receipt.preserves";
const CONTROL_STOP_FILE: &str = "stop-control-receipt.preserves";
const CONTROL_INBOX_DIR: &str = "control/inbox";
const CONTROL_OUTBOX_DIR: &str = "control/outbox";
const CONTROL_LOCK_FILE: &str = "control/node.lock.preserves";
const IDENTITY_RECEIPT_FILE: &str = "identity-receipt.preserves";
const IDENTITY_FILE: &str = "identity.preserves";
const MAX_PENDING_CONTROL_REQUESTS: usize = 1024;
const MAX_CONTROL_LOOP_REQUESTS: u64 = 1024;
pub const DEFAULT_CONTROL_LOOP_REQUESTS: u64 = 64;

const _: () = assert!(MAX_PENDING_CONTROL_REQUESTS > 0);
const _: () = assert!(MAX_CONTROL_LOOP_REQUESTS > 0);
const _: () = assert!(DEFAULT_CONTROL_LOOP_REQUESTS > 0);
const _: () = assert!(DEFAULT_CONTROL_LOOP_REQUESTS <= MAX_CONTROL_LOOP_REQUESTS);

#[derive(Debug, Clone, Copy)]
pub struct NodeDaemonInitInput<'a> {
    pub state_root: &'a Path,
    pub node_id: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeDaemonRunInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeDaemonStatusInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeDaemonStopInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlSubmitInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlDispatchInput<'a> {
    pub state_root: &'a Path,
    pub request_path: Option<&'a Path>,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeControlLoopInput<'a> {
    pub state_root: &'a Path,
    pub max_requests: u64,
}

#[derive(Debug, Clone, Copy)]
struct QueueReceiptValueInput<'a> {
    decision: &'a str,
    phase: &'a str,
    operation: &'a str,
    request_ref: &'a str,
    location_ref: &'a str,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct OperationReceiptValueInput<'a> {
    decision: &'a str,
    request: &'a node_runtime::NodeControlRequest,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct HeartbeatReceiptValueInput<'a> {
    startup_receipt_ref: &'a str,
    lock_ref: &'a str,
    loop_sequence: u64,
    processed_count: u64,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct LoopReceiptValueInput<'a> {
    decision: &'a str,
    startup_receipt_ref: &'a str,
    heartbeat_receipt_ref: &'a str,
    max_requests: u64,
    processed_request_refs: &'a [String],
    dispatch_receipt_refs: &'a [String],
    has_stopped: bool,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct OperationFinalizeInput<'a> {
    state_root: &'a Path,
    request: &'a node_runtime::NodeControlRequest,
    startup_receipt_ref: &'a str,
    subreceipt_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDaemonInit {
    pub config_ref: String,
    pub identity_ref: String,
    pub identity_receipt_ref: String,
    pub config_value: IOValue,
    pub identity_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDaemonRun {
    pub startup_ref: String,
    pub startup_value: IOValue,
    pub adapter_receipt_refs: Vec<node_runtime::NodeAdapterReceiptRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDaemonStatus {
    pub health_ref: String,
    pub control_receipt_ref: String,
    pub health_value: IOValue,
    pub control_receipt_value: IOValue,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDaemonStop {
    pub shutdown_ref: String,
    pub control_receipt_ref: String,
    pub shutdown_value: IOValue,
    pub control_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlSubmit {
    pub request_ref: String,
    pub inbox_path: PathBuf,
    pub queue_receipt_ref: String,
    pub queue_receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlDispatch {
    pub operation: String,
    pub request_ref: String,
    pub control_receipt_ref: String,
    pub control_receipt_value: IOValue,
    pub subreceipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlLoop {
    pub loop_receipt_ref: String,
    pub loop_receipt_value: IOValue,
    pub heartbeat_receipt_ref: String,
    pub heartbeat_receipt_value: IOValue,
    pub processed_request_refs: Vec<String>,
    pub dispatch_receipt_refs: Vec<String>,
    pub has_stopped: bool,
}

pub fn init_local_node(input: &NodeDaemonInitInput<'_>) -> Result<NodeDaemonInit> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.node_id)?;
    ensure_state_layout(input.state_root)?;
    let policy_refs = vec![local_ref("node-policy", input.node_id)?];
    let identity_config = node_identity::NodeIdentityConfig {
        node_id: input.node_id.to_string(),
        display_name: input.node_id.to_string(),
        data_dir: input.state_root.join("identity"),
        explicit_key: None,
        allow_generate: true,
        allow_rotation: false,
        policy_refs: policy_refs.clone(),
    };
    let identity_resolution = node_identity::resolve_node_identity(&identity_config)?;
    let identity = identity_resolution
        .identity
        .ok_or_else(|| MoltenError::invalid_harness("node daemon identity resolution denied"))?;
    let adapters = default_adapter_bindings(input.state_root)?;
    let capability_refs = vec![local_ref("node-capability", input.node_id)?];
    let resource_refs = vec![local_ref("node-resource", input.node_id)?];
    let effect_profile_refs = vec![local_ref("node-effect-profile", input.node_id)?];
    let state_root_ref = state_root_profile_ref(input.state_root)?;
    let config_value = node_runtime::node_config_value(&node_runtime::ConfigValueInput {
        node_identity_ref: &identity.identity_ref,
        state_root_ref: &state_root_ref,
        adapters: &adapters,
        policy_refs: &policy_refs,
        capability_refs: &capability_refs,
        resource_refs: &resource_refs,
        effect_profile_refs: &effect_profile_refs,
    })?;
    write_preserves(&input.state_root.join(CONFIG_FILE), &config_value)?;
    write_preserves(&input.state_root.join(IDENTITY_RECEIPT_FILE), &identity_resolution.receipt_value)?;
    write_preserves(&input.state_root.join(IDENTITY_FILE), &identity.value)?;
    Ok(NodeDaemonInit {
        config_ref: canonical_hash(&config_value)?,
        identity_ref: identity.identity_ref,
        identity_receipt_ref: identity_resolution.receipt_ref,
        config_value,
        identity_receipt_value: identity_resolution.receipt_value,
    })
}

pub fn run_local_node(input: &NodeDaemonRunInput<'_>) -> Result<NodeDaemonRun> {
    ensure_state_layout(input.state_root)?;
    verify_restart_state(input.state_root)?;
    let config_value = read_preserves(&input.state_root.join(CONFIG_FILE))?;
    let identity_receipt = read_preserves(&input.state_root.join(IDENTITY_RECEIPT_FILE))?;
    let identity_receipt_ref = canonical_hash(&identity_receipt)?;
    let index_receipt_refs = index_receipt_refs(input.state_root)?;
    let resource_receipt_refs = resource_receipt_refs(input.state_root)?;
    let capability_receipt_refs = capability_receipt_refs(input.state_root)?;
    let version_refs = vec![local_ref("molten-binary-version", env!("CARGO_PKG_VERSION"))?];
    let source_gate_value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let source_gate_ref = canonical_hash(&source_gate_value)?;
    let run = node_runtime::start_node_runtime(&node_runtime::NodeRuntimeStartInput {
        config_value,
        identity_receipt_ref,
        index_receipt_refs,
        source_gate_receipt_refs: vec![source_gate_ref],
        source_gate_receipt_values: vec![source_gate_value],
        capability_receipt_refs,
        resource_receipt_refs,
        version_refs,
    })?;
    for (adapter, value) in run.adapter_receipts.iter().zip(run.adapter_receipt_values.iter()) {
        write_preserves(
            &input.state_root.join("receipts").join(format!("adapter-start-{}.preserves", adapter.name)),
            value,
        )?;
    }
    write_preserves(&input.state_root.join(STARTUP_FILE), &run.startup_receipt.value)?;
    if run.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "node daemon startup denied receipt={}",
            run.startup_receipt.receipt_ref
        )));
    }
    let startup_ref = run.startup_receipt.receipt_ref.clone();
    write_active_lock(input.state_root, &startup_ref)?;
    import_node_artifact(input.state_root, &run.startup_receipt.value)?;
    Ok(NodeDaemonRun {
        startup_ref,
        startup_value: run.startup_receipt.value,
        adapter_receipt_refs: run.adapter_receipts,
    })
}

pub fn status_local_node(input: &NodeDaemonStatusInput<'_>) -> Result<NodeDaemonStatus> {
    let request = status_request()?;
    status_local_node_with_request(input, &request)
}

fn status_local_node_with_request(
    input: &NodeDaemonStatusInput<'_>,
    request: &node_runtime::NodeControlRequest,
) -> Result<NodeDaemonStatus> {
    let startup_value = read_preserves(&input.state_root.join(STARTUP_FILE))?;
    let startup = node_runtime::parse_node_startup_receipt(&startup_value)?;
    let shutdown_ref = if input.state_root.join(SHUTDOWN_FILE).exists() {
        Some(canonical_hash(&read_preserves(&input.state_root.join(SHUTDOWN_FILE))?)?)
    } else {
        None
    };
    let status = if shutdown_ref.is_some() { "stopped" } else { "running" }.to_string();
    let health_value = node_runtime::node_health_receipt_value(&node_runtime::HealthReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: &startup.receipt_ref,
        shutdown_receipt_ref: shutdown_ref.as_deref(),
        adapter_receipts: &startup.adapters,
        index_receipt_refs: &index_receipt_refs(input.state_root)?,
        head_refs: std::slice::from_ref(&startup.receipt_ref),
        open_job_refs: &[],
        replay_is_eligible: shutdown_ref.is_some(),
        diagnostics: &[],
    })?;
    let health_ref = canonical_hash(&health_value)?;
    write_preserves(&input.state_root.join(HEALTH_FILE), &health_value)?;
    import_node_artifact(input.state_root, &health_value)?;
    let control_receipt_value = control_receipt_for_request(
        input.state_root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&health_ref),
        &[],
    )?;
    let control_receipt_ref = canonical_hash(&control_receipt_value)?;
    write_preserves(&input.state_root.join(CONTROL_STATUS_FILE), &control_receipt_value)?;
    import_node_artifact(input.state_root, &control_receipt_value)?;
    Ok(NodeDaemonStatus {
        health_ref,
        control_receipt_ref,
        health_value,
        control_receipt_value,
        status,
    })
}

pub fn stop_local_node(input: &NodeDaemonStopInput<'_>) -> Result<NodeDaemonStop> {
    let request = shutdown_request()?;
    stop_local_node_with_request(input, &request)
}

fn stop_local_node_with_request(
    input: &NodeDaemonStopInput<'_>,
    request: &node_runtime::NodeControlRequest,
) -> Result<NodeDaemonStop> {
    let startup_value = read_preserves(&input.state_root.join(STARTUP_FILE))?;
    let startup = node_runtime::parse_node_startup_receipt(&startup_value)?;
    let mut shutdown_adapters = Vec::with_capacity(startup.adapters.len());
    for adapter in startup.adapters.iter().rev() {
        let binding = node_runtime::node_adapter_binding(&adapter.name, &adapter.receipt_ref)?;
        let value = node_runtime::node_adapter_lifecycle_receipt_value(&node_runtime::AdapterLifecycleReceiptInput {
            operation: "shutdown",
            decision: "pass",
            adapter: &binding,
            index_receipt_refs: &index_receipt_refs(input.state_root)?,
            resource_receipt_refs: &resource_receipt_refs(input.state_root)?,
            diagnostics: &[],
        })?;
        let receipt_ref = canonical_hash(&value)?;
        write_preserves(
            &input.state_root.join("receipts").join(format!("adapter-shutdown-{}.preserves", adapter.name)),
            &value,
        )?;
        import_node_artifact(input.state_root, &value)?;
        shutdown_adapters.push(node_runtime::NodeAdapterReceiptRef {
            name: adapter.name.clone(),
            receipt_ref,
        });
    }
    let index_refs = index_receipt_refs(input.state_root)?;
    let shutdown_value = node_runtime::node_shutdown_receipt_value(&node_runtime::ShutdownReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: &startup.receipt_ref,
        adapter_receipts: &shutdown_adapters,
        drained_job_refs: &[],
        index_receipt_refs: &index_refs,
        diagnostics: &[],
    })?;
    let shutdown_ref = canonical_hash(&shutdown_value)?;
    write_preserves(&input.state_root.join(SHUTDOWN_FILE), &shutdown_value)?;
    import_node_artifact(input.state_root, &shutdown_value)?;
    let control_receipt_value = control_receipt_for_request(
        input.state_root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&shutdown_ref),
        &[],
    )?;
    let control_receipt_ref = canonical_hash(&control_receipt_value)?;
    write_preserves(&input.state_root.join(CONTROL_STOP_FILE), &control_receipt_value)?;
    import_node_artifact(input.state_root, &control_receipt_value)?;
    remove_active_lock(input.state_root)?;
    Ok(NodeDaemonStop {
        shutdown_ref,
        control_receipt_ref,
        shutdown_value,
        control_receipt_value,
    })
}

pub fn submit_control_request(input: &NodeControlSubmitInput<'_>) -> Result<NodeControlSubmit> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let request = node_runtime::parse_node_control_request(input.request_value)?;
    import_node_artifact(input.state_root, input.request_value)?;
    let inbox_path = control_inbox_path(input.state_root, &request.request_ref);
    write_preserves(&inbox_path, input.request_value)?;
    let location_ref = local_ref("node-control-inbox-path", &inbox_path.display().to_string())?;
    let receipt_value = queue_receipt_value(&QueueReceiptValueInput {
        decision: "pass",
        phase: "enqueue",
        operation: &request.operation,
        request_ref: &request.request_ref,
        location_ref: &location_ref,
        diagnostics: &[],
    })?;
    let queue_receipt_ref = canonical_hash(&receipt_value)?;
    write_preserves(&queue_receipt_path(input.state_root, &request.request_ref), &receipt_value)?;
    import_node_artifact(input.state_root, &receipt_value)?;
    Ok(NodeControlSubmit {
        request_ref: request.request_ref,
        inbox_path,
        queue_receipt_ref,
        queue_receipt_value: receipt_value,
    })
}

pub fn dispatch_control_request(input: &NodeControlDispatchInput<'_>) -> Result<NodeControlDispatch> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    require_active_lock(input.state_root)?;
    let request_path = match input.request_path {
        Some(path) => path.to_path_buf(),
        None => first_pending_control_request(input.state_root)?,
    };
    let request_value = read_preserves(&request_path)?;
    let request = node_runtime::parse_node_control_request(&request_value)?;
    import_node_artifact(input.state_root, &request_value)?;
    if let Some(prior) = prior_dispatch_for_request(input.state_root, &request)? {
        archive_dispatched_request(input.state_root, &request_path, &request.value)?;
        write_dispatch_queue_receipt(input.state_root, &request, "duplicate-dispatch")?;
        return Ok(prior);
    }
    let dispatch = match request.operation.as_str() {
        "status" => dispatch_status_request(input.state_root, &request)?,
        "shutdown" => dispatch_shutdown_request(input.state_root, &request)?,
        "install" => dispatch_install_request(input.state_root, &request)?,
        "run" => dispatch_run_request(input.state_root, &request)?,
        "gate" => dispatch_gate_request(input.state_root, &request)?,
        other => {
            return Err(MoltenError::invalid_harness(format!("node control request operation unsupported: {other}")));
        }
    };
    archive_dispatched_request(input.state_root, &request_path, &request.value)?;
    write_dispatch_queue_receipt(input.state_root, &request, "dispatch")?;
    Ok(dispatch)
}

pub fn run_control_loop(input: &NodeControlLoopInput<'_>) -> Result<NodeControlLoop> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let max_requests = validate_loop_request_limit(input.max_requests)?;
    require_active_lock(input.state_root)?;
    let startup = current_startup_receipt(input.state_root)?;
    let lock_value = read_preserves(&input.state_root.join(CONTROL_LOCK_FILE))?;
    let lock_ref = canonical_hash(&lock_value)?;
    let initial_diagnostics = Vec::new();
    let heartbeat_value = heartbeat_receipt_value(&HeartbeatReceiptValueInput {
        startup_receipt_ref: &startup.receipt_ref,
        lock_ref: &lock_ref,
        loop_sequence: 0,
        processed_count: 0,
        diagnostics: &initial_diagnostics,
    })?;
    let heartbeat_receipt_ref = canonical_hash(&heartbeat_value)?;
    write_preserves(&control_heartbeat_receipt_path(input.state_root, &heartbeat_receipt_ref), &heartbeat_value)?;
    import_node_artifact(input.state_root, &heartbeat_value)?;

    let mut processed_request_refs = Vec::with_capacity(max_requests);
    let mut dispatch_receipt_refs = Vec::with_capacity(max_requests);
    let mut diagnostics = Vec::new();
    let mut has_stopped = false;
    for _ in 0..max_requests {
        let Some(request_path) = next_pending_control_request(input.state_root)? else {
            break;
        };
        let dispatched = dispatch_control_request(&NodeControlDispatchInput {
            state_root: input.state_root,
            request_path: Some(&request_path),
        })?;
        let control = node_runtime::parse_node_control_receipt(&dispatched.control_receipt_value)?;
        processed_request_refs.push(dispatched.request_ref.clone());
        dispatch_receipt_refs.push(dispatched.control_receipt_ref.clone());
        if dispatched.operation == "shutdown" && control.decision == "pass" {
            has_stopped = true;
            break;
        }
    }
    if processed_request_refs.len() == max_requests && next_pending_control_request(input.state_root)?.is_some() {
        diagnostics.push("node control loop reached max requests with pending inbox entries".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let loop_value = loop_receipt_value(&LoopReceiptValueInput {
        decision,
        startup_receipt_ref: &startup.receipt_ref,
        heartbeat_receipt_ref: &heartbeat_receipt_ref,
        max_requests: input.max_requests,
        processed_request_refs: &processed_request_refs,
        dispatch_receipt_refs: &dispatch_receipt_refs,
        has_stopped,
        diagnostics: &diagnostics,
    })?;
    let loop_receipt_ref = canonical_hash(&loop_value)?;
    write_preserves(&control_loop_receipt_path(input.state_root, &loop_receipt_ref), &loop_value)?;
    import_node_artifact(input.state_root, &loop_value)?;
    Ok(NodeControlLoop {
        loop_receipt_ref,
        loop_receipt_value: loop_value,
        heartbeat_receipt_ref,
        heartbeat_receipt_value: heartbeat_value,
        processed_request_refs,
        dispatch_receipt_refs,
        has_stopped,
    })
}

fn prior_dispatch_for_request(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
) -> Result<Option<NodeControlDispatch>> {
    let receipt_path = control_outbox_receipt_path(state_root, &request.request_ref);
    if !receipt_path.exists() {
        return Ok(None);
    }
    let archived_path = control_outbox_request_path(state_root, &request.request_ref);
    if archived_path.exists() {
        let archived_value = read_preserves(&archived_path)?;
        let archived_ref = canonical_hash(&archived_value)?;
        if archived_ref != request.request_ref {
            return Err(MoltenError::invalid_harness(
                "node control duplicate request conflicts with archived request evidence",
            ));
        }
    }
    let control_receipt_value = read_preserves(&receipt_path)?;
    let control = node_runtime::parse_node_control_receipt(&control_receipt_value)?;
    if control.request_ref != request.request_ref {
        return Err(MoltenError::invalid_harness("node control duplicate receipt conflicts with request ref"));
    }
    Ok(Some(NodeControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: control.receipt_ref,
        control_receipt_value: control.value,
        subreceipt_refs: control.subreceipt_refs,
    }))
}

fn write_dispatch_queue_receipt(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
    phase: &str,
) -> Result<String> {
    let location_ref = local_ref(
        "node-control-outbox-path",
        &control_outbox_receipt_path(state_root, &request.request_ref).display().to_string(),
    )?;
    let diagnostics = Vec::new();
    let queue_receipt = queue_receipt_value(&QueueReceiptValueInput {
        decision: "pass",
        phase,
        operation: &request.operation,
        request_ref: &request.request_ref,
        location_ref: &location_ref,
        diagnostics: &diagnostics,
    })?;
    let queue_receipt_ref = canonical_hash(&queue_receipt)?;
    write_preserves(&dispatch_receipt_path(state_root, &request.request_ref), &queue_receipt)?;
    import_node_artifact(state_root, &queue_receipt)?;
    Ok(queue_receipt_ref)
}

fn dispatch_status_request(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
) -> Result<NodeControlDispatch> {
    let status = status_local_node_with_request(&NodeDaemonStatusInput { state_root }, request)?;
    write_preserves(&control_outbox_receipt_path(state_root, &request.request_ref), &status.control_receipt_value)?;
    Ok(NodeControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: status.control_receipt_ref,
        control_receipt_value: status.control_receipt_value,
        subreceipt_refs: vec![status.health_ref],
    })
}

fn dispatch_shutdown_request(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
) -> Result<NodeControlDispatch> {
    let stop = stop_local_node_with_request(&NodeDaemonStopInput { state_root }, request)?;
    write_preserves(&control_outbox_receipt_path(state_root, &request.request_ref), &stop.control_receipt_value)?;
    Ok(NodeControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: stop.control_receipt_ref,
        control_receipt_value: stop.control_receipt_value,
        subreceipt_refs: vec![stop.shutdown_ref],
    })
}

#[derive(Debug, Clone, Copy)]
struct NodeControlProvenanceInput<'a> {
    state_root: &'a Path,
    request: &'a node_runtime::NodeControlRequest,
    artifact_ref: &'a str,
    operation: &'a str,
    subreceipt_kind: &'a str,
}

fn evaluate_node_control_provenance(
    input: &NodeControlProvenanceInput<'_>,
) -> Result<provenance::ProvenanceEvaluation> {
    let mut provenance_diagnostics = Vec::with_capacity(input.request.evidence_refs.len().saturating_add(1));
    if input.request.evidence_refs.is_empty() {
        provenance_diagnostics.push("node control provenance evidence refs missing".to_string());
    }
    let mut provenance_values = Vec::with_capacity(input.request.evidence_refs.len());
    for evidence_ref in &input.request.evidence_refs {
        match read_node_ledger_artifact(input.state_root, evidence_ref) {
            Ok(value) => provenance_values.push(value),
            Err(error) => provenance_diagnostics
                .push(format!("node control provenance evidence {evidence_ref} not found in node ledger: {error}")),
        }
    }
    let evaluation = provenance::evaluate_provenance(&provenance::ProvenanceEvaluationInput {
        operation: input.operation,
        profile: "node-control",
        artifact_ref: input.artifact_ref,
        provenance_values: &provenance_values,
        prior_diagnostics: &provenance_diagnostics,
    })?;
    write_preserves(
        &control_operation_subreceipt_path(input.state_root, &input.request.request_ref, input.subreceipt_kind),
        &evaluation.receipt_value,
    )?;
    import_node_artifact(input.state_root, &evaluation.receipt_value)?;
    Ok(evaluation)
}

fn dispatch_install_request(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
) -> Result<NodeControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
    let mut diagnostics = side_effect_preflight_diagnostics(request);
    let Some(payload_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control install requires payload ref".to_string());
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    };
    if !diagnostics.is_empty() {
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    }
    let payload_value = match read_node_ledger_artifact(state_root, payload_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control install payload not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &[],
                diagnostics: &diagnostics,
            });
        }
    };
    let provenance = evaluate_node_control_provenance(&NodeControlProvenanceInput {
        state_root,
        request,
        artifact_ref: payload_ref,
        operation: "install",
        subreceipt_kind: "artifact-provenance",
    })?;
    let provenance_receipt_refs = [provenance.receipt_ref.clone()];
    diagnostics.extend(provenance.diagnostics.iter().cloned());
    if provenance.decision != "pass" {
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &provenance_receipt_refs,
            diagnostics: &diagnostics,
        });
    }
    let schema_refs = match request.target_ref.as_ref() {
        Some(target_ref) => vec![target_ref.clone()],
        None => vec![local_ref("node-control-install-schema", &request.request_ref)?],
    };
    let extra_evidence_refs = if request.target_ref.is_some() { 3 } else { 2 };
    let mut evidence_refs =
        Vec::with_capacity(request.resource_refs.len() + request.evidence_refs.len() + extra_evidence_refs);
    evidence_refs.extend(request.resource_refs.iter().cloned());
    evidence_refs.extend(request.evidence_refs.iter().cloned());
    evidence_refs.push(provenance_receipt_refs[0].clone());
    evidence_refs.push(payload_ref.to_string());
    if let Some(target_ref) = request.target_ref.as_ref() {
        evidence_refs.push(target_ref.clone());
    }
    let install = match artifacts::install_artifact(&state_root.join("registry"), &artifacts::ArtifactInstallInput {
        kind: "node-control-artifact".to_string(),
        payload: payload_value,
        schema_refs,
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: request.policy_refs.clone(),
        evidence_refs,
        installer_ref: request.request_ref.clone(),
        capability_refs: request.authority_refs.clone(),
    }) {
        Ok(install) => install,
        Err(error) => {
            diagnostics.push(format!("node control artifact install failed: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &provenance_receipt_refs,
                diagnostics: &diagnostics,
            });
        }
    };
    let install_receipt_ref = canonical_hash(&install.receipt_value)?;
    write_preserves(
        &control_operation_subreceipt_path(state_root, &request.request_ref, "artifact-install"),
        &install.receipt_value,
    )?;
    import_node_artifact(state_root, &install.receipt_value)?;
    if install.decision == "pass" {
        import_node_artifact(state_root, &install.artifact.value)?;
    } else if install.missing_dependencies.is_empty() {
        diagnostics.push("node control artifact install denied".to_string());
    } else {
        diagnostics
            .extend(install.missing_dependencies.iter().map(|reference| format!("missing dependency {reference}")));
    }
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        subreceipt_refs: &[provenance.receipt_ref, install_receipt_ref],
        diagnostics: &diagnostics,
    })
}

fn dispatch_run_request(state_root: &Path, request: &node_runtime::NodeControlRequest) -> Result<NodeControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
    let mut diagnostics = side_effect_preflight_diagnostics(request);
    let Some(execution_request_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control run requires execution request payload ref".to_string());
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    };
    let Some(admission_ref) = request.target_ref.as_deref() else {
        diagnostics.push("node control run requires admission receipt target ref".to_string());
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    };
    if !diagnostics.is_empty() {
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    }
    let execution_request_value = match read_node_ledger_artifact(state_root, execution_request_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control run execution request not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &[],
                diagnostics: &diagnostics,
            });
        }
    };
    let execution_request = match job_dag::parse_job_execution_request_value(&execution_request_value) {
        Ok(execution_request) => execution_request,
        Err(error) => {
            diagnostics.push(format!("node control run execution request malformed: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &[],
                diagnostics: &diagnostics,
            });
        }
    };
    let provenance = evaluate_node_control_provenance(&NodeControlProvenanceInput {
        state_root,
        request,
        artifact_ref: &execution_request.job_ref,
        operation: "run",
        subreceipt_kind: "job-provenance",
    })?;
    let provenance_receipt_refs = [provenance.receipt_ref.clone()];
    diagnostics.extend(provenance.diagnostics.iter().cloned());
    if provenance.decision != "pass" {
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &provenance_receipt_refs,
            diagnostics: &diagnostics,
        });
    }
    let admission_receipt_value = match read_node_ledger_artifact(state_root, admission_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control run admission receipt not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &provenance_receipt_refs,
                diagnostics: &diagnostics,
            });
        }
    };
    let execution = job_dag::execution_loopback(job_dag::ExecutionLoopbackInput {
        target_registry: &state_root.join("registry"),
        storage_root: &state_root.join("storage"),
        cache_root: &state_root.join("cache"),
        chunk_root: &state_root.join("chunks"),
        admission_receipt_value: &admission_receipt_value,
        request_value: &execution_request_value,
    })?;
    write_preserves(
        &control_operation_subreceipt_path(state_root, &request.request_ref, "job-execution"),
        &execution.receipt_value,
    )?;
    import_node_artifact(state_root, &execution.receipt_value)?;
    let mut subreceipt_refs = Vec::with_capacity(3);
    subreceipt_refs.push(provenance.receipt_ref);
    subreceipt_refs.push(execution.receipt_ref.clone());
    if let Some(run) = execution.run.as_ref() {
        let run_ref = canonical_hash(&run.receipt_value)?;
        write_preserves(
            &control_operation_subreceipt_path(state_root, &request.request_ref, "job-run"),
            &run.receipt_value,
        )?;
        import_node_artifact(state_root, &run.receipt_value)?;
        subreceipt_refs.push(run_ref);
    }
    diagnostics.extend(execution.diagnostics.iter().cloned());
    if execution.decision != "pass" && diagnostics.is_empty() {
        diagnostics.push("node control run execution denied".to_string());
    }
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        subreceipt_refs: &subreceipt_refs,
        diagnostics: &diagnostics,
    })
}

fn dispatch_gate_request(state_root: &Path, request: &node_runtime::NodeControlRequest) -> Result<NodeControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
    let mut diagnostics = side_effect_preflight_diagnostics(request);
    let Some(subject_ref) = request.target_ref.as_deref() else {
        diagnostics.push("node control gate requires target subject ref".to_string());
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    };
    let Some(gate_receipt_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control gate requires gate receipt payload ref".to_string());
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    };
    if !diagnostics.is_empty() {
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    }
    let gate_value = match read_node_ledger_artifact(state_root, gate_receipt_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control gate receipt not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &[],
                diagnostics: &diagnostics,
            });
        }
    };
    let validation = octet_gate::validate_octet_source_gate(&octet_gate::OctetSourceGateValidationInput {
        consumer: "node-control-gate".to_string(),
        subject_ref: subject_ref.to_string(),
        gate_receipt_value: Some(gate_value),
        source_scope: octet_gate::default_source_scope("node-control-gate")?,
    })?;
    write_preserves(
        &control_operation_subreceipt_path(state_root, &request.request_ref, "octet-source-gate"),
        &validation.value,
    )?;
    import_node_artifact(state_root, &validation.value)?;
    diagnostics.extend(validation.diagnostics.iter().cloned());
    if validation.decision != "pass" && diagnostics.is_empty() {
        diagnostics.push("node control gate validation denied".to_string());
    }
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        subreceipt_refs: std::slice::from_ref(&validation.validation_ref),
        diagnostics: &diagnostics,
    })
}

fn finalize_operation_dispatch(input: &OperationFinalizeInput<'_>) -> Result<NodeControlDispatch> {
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let operation_receipt = operation_receipt_value(&OperationReceiptValueInput {
        decision,
        request: input.request,
        diagnostics: input.diagnostics,
    })?;
    let operation_receipt_ref = canonical_hash(&operation_receipt)?;
    write_preserves(&control_operation_receipt_path(input.state_root, &input.request.request_ref), &operation_receipt)?;
    import_node_artifact(input.state_root, &operation_receipt)?;
    let mut all_subreceipt_refs = Vec::with_capacity(input.subreceipt_refs.len() + 1);
    all_subreceipt_refs.extend(input.subreceipt_refs.iter().cloned());
    all_subreceipt_refs.push(operation_receipt_ref);
    let control_receipt = control_receipt_for_request(
        input.state_root,
        input.request,
        input.startup_receipt_ref,
        &all_subreceipt_refs,
        input.diagnostics,
    )?;
    let control_receipt_ref = canonical_hash(&control_receipt)?;
    write_preserves(&control_outbox_receipt_path(input.state_root, &input.request.request_ref), &control_receipt)?;
    import_node_artifact(input.state_root, &control_receipt)?;
    Ok(NodeControlDispatch {
        operation: input.request.operation.clone(),
        request_ref: input.request.request_ref.clone(),
        control_receipt_ref,
        control_receipt_value: control_receipt,
        subreceipt_refs: all_subreceipt_refs,
    })
}

fn side_effect_preflight_diagnostics(request: &node_runtime::NodeControlRequest) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    if request.authority_refs.is_empty() {
        diagnostics.push("node control authority refs missing".to_string());
    }
    if request.policy_refs.is_empty() {
        diagnostics.push("node control policy refs missing".to_string());
    }
    if request.resource_refs.is_empty() {
        diagnostics.push("node control resource refs missing".to_string());
    }
    diagnostics
}

fn read_node_ledger_artifact(state_root: &Path, artifact_ref: &str) -> Result<IOValue> {
    ledger::read_artifact(&state_root.join("ledger"), artifact_ref)
}

fn control_receipt_for_request(
    state_root: &Path,
    request: &node_runtime::NodeControlRequest,
    startup_receipt_ref: &str,
    subreceipt_refs: &[String],
    diagnostics: &[String],
) -> Result<IOValue> {
    let decision = if diagnostics.is_empty()
        && !request.authority_refs.is_empty()
        && !request.policy_refs.is_empty()
        && !request.resource_refs.is_empty()
    {
        "pass"
    } else {
        "deny"
    };
    let mut receipt_diagnostics = Vec::with_capacity(diagnostics.len() + 3);
    receipt_diagnostics.extend(diagnostics.iter().cloned());
    if request.authority_refs.is_empty() {
        receipt_diagnostics.push("node control authority refs missing".to_string());
    }
    if request.policy_refs.is_empty() {
        receipt_diagnostics.push("node control policy refs missing".to_string());
    }
    if request.resource_refs.is_empty() {
        receipt_diagnostics.push("node control resource refs missing".to_string());
    }
    let final_decision = if receipt_diagnostics.is_empty() {
        decision
    } else {
        "deny"
    };
    let authority_receipt_refs = if final_decision == "pass" {
        capability_receipt_refs(state_root)?
    } else {
        Vec::new()
    };
    let resource_receipt_refs = if final_decision == "pass" {
        resource_receipt_refs(state_root)?
    } else {
        Vec::new()
    };
    node_runtime::node_control_receipt_value(&node_runtime::ControlReceiptValueInput {
        decision: final_decision,
        request,
        startup_receipt_ref,
        authority_receipt_refs: &authority_receipt_refs,
        resource_receipt_refs: &resource_receipt_refs,
        subreceipt_refs,
        diagnostics: &receipt_diagnostics,
    })
}

fn queue_receipt_value(input: &QueueReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-queue-receipt-v1", vec![
        string(NODE_CONTROL_QUEUE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("phase", vec![string(input.phase)]),
        record("operation", vec![string(input.operation)]),
        record("request", vec![string(input.request_ref)]),
        record("profile", vec![string("local-preserves-control-file-v1")]),
        record("location", vec![string(input.location_ref)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("canonical-request-ref"), string("pass")]),
            record("check", vec![string("durable-control-profile"), string("pass")]),
            record("check", vec![string("explicit-state-root"), string("pass")]),
        ])]),
    ]))
}

fn operation_receipt_value(input: &OperationReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-operation-receipt-v1", vec![
        string(NODE_CONTROL_OPERATION_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(&input.request.operation)]),
        record("request", vec![string(&input.request.request_ref)]),
        record("target", vec![optional_string(input.request.target_ref.as_deref())]),
        record("payload", vec![optional_string(input.request.payload_ref.as_deref())]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("operation-dispatch-explicit"), string("pass")]),
            record("check", vec![string("side-effects-receipted"), string("pass")]),
            record("check", vec![string("canonical-receipt"), string("pass")]),
        ])]),
    ]))
}

fn heartbeat_receipt_value(input: &HeartbeatReceiptValueInput<'_>) -> Result<IOValue> {
    Ok(record("node-control-heartbeat-receipt-v1", vec![
        string(NODE_CONTROL_HEARTBEAT_RECEIPT_SCHEMA),
        record("decision", vec![string(if input.diagnostics.is_empty() { "pass" } else { "deny" })]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("lock", vec![string(input.lock_ref)]),
        record("loop-sequence", vec![string(input.loop_sequence.to_string())]),
        record("processed-count", vec![string(input.processed_count.to_string())]),
        record("profile", vec![string("local-preserves-control-loop-v1")]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("active-lock-bound"), string("pass")]),
            record("check", vec![string("heartbeat-is-receipted"), string("pass")]),
            record("check", vec![string("no-ambient-socket-authority"), string("pass")]),
        ])]),
    ]))
}

fn loop_receipt_value(input: &LoopReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    Ok(record("node-control-loop-receipt-v1", vec![
        string(NODE_CONTROL_LOOP_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("heartbeat", vec![string(input.heartbeat_receipt_ref)]),
        record("max-requests", vec![string(input.max_requests.to_string())]),
        record("processed-requests", vec![sequence(input.processed_request_refs.iter().map(string).collect())]),
        record("dispatch-receipts", vec![sequence(input.dispatch_receipt_refs.iter().map(string).collect())]),
        record("stopped", vec![string(if input.has_stopped { "yes" } else { "no" })]),
        record("profile", vec![string("local-preserves-control-loop-v1")]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("bounded-request-loop"), string("pass")]),
            record("check", vec![string("deterministic-inbox-order"), string("pass")]),
            record("check", vec![string("idempotent-request-dispatch"), string("pass")]),
            record("check", vec![string("shutdown-stops-loop"), string("pass")]),
        ])]),
    ]))
}

pub fn node_daemon_summary(value: &IOValue) -> Result<String> {
    if let Ok(config) = node_runtime::parse_node_config(value) {
        return Ok(format!(
            "node config ref={} identity={} adapters={}",
            config.config_ref,
            config.node_identity_ref,
            config.adapters.len()
        ));
    }
    if let Ok(startup) = node_runtime::parse_node_startup_receipt(value) {
        return Ok(format!(
            "node startup decision={} receipt={} adapters={}",
            startup.decision,
            startup.receipt_ref,
            startup.adapters.len()
        ));
    }
    if let Ok(control) = node_runtime::parse_node_control_receipt(value) {
        return Ok(format!(
            "node control decision={} receipt={} request={}",
            control.decision, control.receipt_ref, control.request_ref
        ));
    }
    if let Ok(health) = node_runtime::parse_node_health_receipt(value) {
        return Ok(format!(
            "node health decision={} receipt={} replay={}",
            health.decision, health.receipt_ref, health.replay_status
        ));
    }
    if let Ok(shutdown) = node_runtime::parse_node_shutdown_receipt(value) {
        return Ok(format!(
            "node shutdown decision={} receipt={} adapters={}",
            shutdown.decision,
            shutdown.receipt_ref,
            shutdown.adapters.len()
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-lock-v1", Some(6)) {
        return Ok(format!(
            "node control lock startup={} owner={}",
            record_string(&fields[2], "startup")?,
            record_string(&fields[3], "owner")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-queue-receipt-v1", Some(9)) {
        return Ok(format!(
            "node control queue decision={} phase={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "phase")?,
            record_string(&fields[4], "request")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-operation-receipt-v1", Some(8)) {
        return Ok(format!(
            "node control operation decision={} operation={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "operation")?,
            record_string(&fields[3], "request")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-heartbeat-receipt-v1", Some(9)) {
        return Ok(format!(
            "node control heartbeat decision={} startup={} processed={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_string(&fields[5], "processed-count")?
        ));
    }
    if let Some(fields) = value.collect_simple_record("node-control-loop-receipt-v1", Some(11)) {
        return Ok(format!(
            "node control loop decision={} startup={} processed={} stopped={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_sequence_len(&fields[5], "processed-requests")?,
            record_string(&fields[7], "stopped")?
        ));
    }
    if let Ok(summary) = provenance::provenance_summary(value) {
        return Ok(summary);
    }
    Err(MoltenError::invalid_harness("unsupported node daemon artifact for show"))
}

fn current_startup_receipt(state_root: &Path) -> Result<node_runtime::NodeStartupReceipt> {
    let startup_value = read_preserves(&state_root.join(STARTUP_FILE))?;
    node_runtime::parse_node_startup_receipt(&startup_value)
}

fn write_active_lock(state_root: &Path, startup_receipt_ref: &str) -> Result<()> {
    let lock_value = active_lock_value(state_root, startup_receipt_ref)?;
    write_preserves(&state_root.join(CONTROL_LOCK_FILE), &lock_value)?;
    import_node_artifact(state_root, &lock_value)?;
    Ok(())
}

fn require_active_lock(state_root: &Path) -> Result<()> {
    let lock_path = state_root.join(CONTROL_LOCK_FILE);
    if !lock_path.exists() {
        return Err(MoltenError::invalid_harness("node control dispatch requires active node lock"));
    }
    let lock_value = read_preserves(&lock_path)?;
    let fields = lock_value
        .collect_simple_record("node-control-lock-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-lock-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_LOCK_SCHEMA, "node control lock")?;
    let locked_startup = record_string(&fields[2], "startup")?;
    let startup = current_startup_receipt(state_root)?;
    if locked_startup != startup.receipt_ref {
        return Err(MoltenError::invalid_harness("node control lock is stale for current startup receipt"));
    }
    Ok(())
}

fn remove_active_lock(state_root: &Path) -> Result<()> {
    let path = state_root.join(CONTROL_LOCK_FILE);
    if path.exists() {
        fs::remove_file(path).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn active_lock_value(state_root: &Path, startup_receipt_ref: &str) -> Result<IOValue> {
    Ok(record("node-control-lock-v1", vec![
        string(NODE_CONTROL_LOCK_SCHEMA),
        record("state-root", vec![string(&state_root_profile_ref(state_root)?)]),
        record("startup", vec![string(startup_receipt_ref)]),
        record("owner", vec![string(&local_ref("node-control-owner", startup_receipt_ref)?)]),
        record("profile", vec![string("local-preserves-control-file-v1")]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("startup-bound"), string("pass")]),
            record("check", vec![string("not-authority-token"), string("pass")]),
            record("check", vec![string("explicit-state-root"), string("pass")]),
        ])]),
    ]))
}

fn import_node_artifact(state_root: &Path, value: &IOValue) -> Result<String> {
    let imported = ledger::import_artifact(&state_root.join("ledger"), value)?;
    let receipt_path = state_root
        .join("receipts")
        .join(format!("ledger-import-{}.preserves", ref_file_stem(&imported.artifact_ref)));
    write_preserves(&receipt_path, &imported.receipt_value)?;
    Ok(imported.artifact_ref)
}

fn first_pending_control_request(state_root: &Path) -> Result<PathBuf> {
    next_pending_control_request(state_root)?
        .ok_or_else(|| MoltenError::invalid_harness("node control inbox has no pending requests"))
}

fn next_pending_control_request(state_root: &Path) -> Result<Option<PathBuf>> {
    let mut paths = pending_control_request_paths(state_root)?;
    Ok(paths.pop())
}

fn pending_control_request_paths(state_root: &Path) -> Result<Vec<PathBuf>> {
    let inbox = state_root.join(CONTROL_INBOX_DIR);
    let mut paths = Vec::with_capacity(MAX_PENDING_CONTROL_REQUESTS);
    for entry_result in fs::read_dir(&inbox).map_err(MoltenError::from)? {
        if paths.len() >= MAX_PENDING_CONTROL_REQUESTS {
            return Err(MoltenError::invalid_harness("too many pending node control requests"));
        }
        let entry = entry_result.map_err(MoltenError::from)?;
        let path = entry.path();
        let name = path.file_name().and_then(|value| value.to_str()).unwrap_or_default();
        if path.is_file() && name.ends_with(".preserves") && !name.contains("receipt") {
            paths.push(path);
        }
    }
    paths.sort_by(|left, right| right.cmp(left));
    Ok(paths)
}

fn archive_dispatched_request(state_root: &Path, request_path: &Path, request_value: &IOValue) -> Result<()> {
    let request_ref = canonical_hash(request_value)?;
    let archived = control_outbox_request_path(state_root, &request_ref);
    write_preserves(&archived, request_value)?;
    if request_path.starts_with(state_root.join(CONTROL_INBOX_DIR)) && request_path.exists() {
        fs::remove_file(request_path).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn control_inbox_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root.join(CONTROL_INBOX_DIR).join(format!("{}.preserves", ref_file_stem(request_ref)))
}

fn queue_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INBOX_DIR)
        .join(format!("{}.queue-receipt.preserves", ref_file_stem(request_ref)))
}

fn dispatch_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.dispatch-receipt.preserves", ref_file_stem(request_ref)))
}

fn control_outbox_request_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.request.preserves", ref_file_stem(request_ref)))
}

fn control_outbox_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.control-receipt.preserves", ref_file_stem(request_ref)))
}

fn control_operation_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.operation-receipt.preserves", ref_file_stem(request_ref)))
}

fn control_operation_subreceipt_path(state_root: &Path, request_ref: &str, label: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.{}.preserves", ref_file_stem(request_ref), label))
}

fn control_heartbeat_receipt_path(state_root: &Path, heartbeat_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.heartbeat-receipt.preserves", ref_file_stem(heartbeat_ref)))
}

fn control_loop_receipt_path(state_root: &Path, loop_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.loop-receipt.preserves", ref_file_stem(loop_ref)))
}

fn ref_file_stem(value_ref: &str) -> String {
    value_ref.replace(':', "-")
}

fn optional_string(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny" | "fail") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("invalid node control decision `{decision}`")))
    }
}

fn validate_loop_request_limit(max_requests: u64) -> Result<usize> {
    if max_requests == 0 {
        return Err(MoltenError::invalid_harness("node control loop max requests must be positive"));
    }
    if max_requests > MAX_CONTROL_LOOP_REQUESTS {
        return Err(MoltenError::invalid_harness(format!(
            "node control loop max requests exceeds bounded limit {MAX_CONTROL_LOOP_REQUESTS}"
        )));
    }
    usize::try_from(max_requests)
        .map_err(|_| MoltenError::invalid_harness("node control loop max requests does not fit this platform"))
}

fn record_string(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<String> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} string>")))?;
    fields[0]
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a string")))
}

fn record_sequence_len(value: &preserves::Value<preserves::IOValue>, tag: &str) -> Result<usize> {
    let record_value = crate::preserves_rail::value_to_iovalue(value);
    let fields = record_value
        .collect_simple_record(tag, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{tag} sequence>")))?;
    fields[0]
        .collect_sequence()
        .map(|items| items.len())
        .ok_or_else(|| MoltenError::invalid_harness(format!("{tag} must contain a sequence")))
}

fn require_schema(value: &preserves::Value<preserves::IOValue>, expected: &str, context: &str) -> Result<()> {
    let actual = value
        .as_string()
        .ok_or_else(|| MoltenError::invalid_harness(format!("{context} schema must be a string")))?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{context} schema mismatch: expected {expected}, got {actual}"
        )))
    }
}

fn verify_restart_state(state_root: &Path) -> Result<()> {
    let startup_path = state_root.join(STARTUP_FILE);
    if startup_path.exists() {
        let shutdown_path = state_root.join(SHUTDOWN_FILE);
        if !shutdown_path.exists() {
            return Err(MoltenError::invalid_harness(
                "node daemon restart denied: previous startup has no clean shutdown receipt",
            ));
        }
        let startup_value = read_preserves(&startup_path)?;
        let startup = node_runtime::parse_node_startup_receipt(&startup_value)?;
        let shutdown_ref = canonical_hash(&read_preserves(&shutdown_path)?)?;
        let head_refs = vec![startup.receipt_ref.clone()];
        let health_value =
            node_runtime::node_restart_health_receipt_value(&node_runtime::RestartHealthReceiptValueInput {
                startup_receipt: &startup,
                shutdown_receipt_ref: Some(&shutdown_ref),
                index_receipt_refs: &index_receipt_refs(state_root)?,
                head_refs: &head_refs,
                open_job_refs: &[],
                diagnostics: &[],
            })?;
        let health = node_runtime::parse_node_health_receipt(&health_value)?;
        write_preserves(&state_root.join(HEALTH_FILE), &health_value)?;
        if health.decision != "pass" {
            return Err(MoltenError::invalid_harness(format!(
                "node daemon restart recovery denied receipt={}",
                health.receipt_ref
            )));
        }
        fs::remove_file(shutdown_path).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn default_adapter_bindings(state_root: &Path) -> Result<Vec<node_runtime::NodeAdapterBinding>> {
    let mut adapters = Vec::with_capacity(node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        let profile_ref =
            local_ref("node-adapter-profile", &format!("{}:{name}", state_root_profile_ref(state_root)?))?;
        adapters.push(node_runtime::node_adapter_binding(name, &profile_ref)?);
    }
    Ok(adapters)
}

fn status_request() -> Result<node_runtime::NodeControlRequest> {
    control_request("status")
}

fn shutdown_request() -> Result<node_runtime::NodeControlRequest> {
    control_request("shutdown")
}

fn control_request(operation: &str) -> Result<node_runtime::NodeControlRequest> {
    let authority_refs = vec![local_ref("node-control-authority", operation)?];
    let policy_refs = vec![local_ref("node-control-policy", operation)?];
    let resource_refs = vec![local_ref("node-control-resource", operation)?];
    let value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
        operation,
        target_ref: None,
        payload_ref: None,
        authority_refs: &authority_refs,
        policy_refs: &policy_refs,
        resource_refs: &resource_refs,
        evidence_refs: &[],
    })?;
    node_runtime::parse_node_control_request(&value)
}

fn index_receipt_refs(state_root: &Path) -> Result<Vec<String>> {
    let root_ref = state_root_profile_ref(state_root)?;
    let mut refs = Vec::with_capacity(node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
    for name in node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        refs.push(local_ref("node-index-verify", &format!("{root_ref}:{name}"))?);
    }
    Ok(refs)
}

fn resource_receipt_refs(state_root: &Path) -> Result<Vec<String>> {
    Ok(vec![local_ref(
        "node-resource-profile",
        &state_root_profile_ref(state_root)?,
    )?])
}

fn capability_receipt_refs(state_root: &Path) -> Result<Vec<String>> {
    Ok(vec![local_ref(
        "node-authority-profile",
        &state_root_profile_ref(state_root)?,
    )?])
}

fn state_root_profile_ref(state_root: &Path) -> Result<String> {
    local_ref("node-state-root-profile", &state_root.display().to_string())
}

fn local_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("node-daemon-local-ref-v1", vec![string(kind), string(label)]))
}

fn ensure_state_layout(state_root: &Path) -> Result<()> {
    fs::create_dir_all(state_root).map_err(MoltenError::from)?;
    for child in [
        "identity",
        "ledger",
        "registry",
        "chunks",
        "storage",
        "cache",
        "remote-dataspace",
        "services",
        "jobs",
        "coordination",
        "plugin-host",
        "catalog-mcp",
        "control",
        CONTROL_INBOX_DIR,
        CONTROL_OUTBOX_DIR,
        "receipts",
    ] {
        fs::create_dir_all(state_root.join(child)).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn validate_state_root(state_root: &Path) -> Result<()> {
    if state_root.as_os_str().is_empty() {
        return Err(MoltenError::invalid_harness("node daemon requires explicit state root"));
    }
    if state_root == Path::new(".") {
        return Err(MoltenError::invalid_harness("node daemon state root cannot be ambient current directory"));
    }
    Ok(())
}

fn validate_node_id(node_id: &str) -> Result<()> {
    if node_id.trim().is_empty() {
        Err(MoltenError::invalid_harness("node daemon id must not be empty"))
    } else {
        Ok(())
    }
}

fn write_preserves(path: &Path, value: &IOValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, to_text(value)?).map_err(MoltenError::from)
}

fn read_preserves(path: &Path) -> Result<IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

pub fn config_path(state_root: &Path) -> PathBuf {
    state_root.join(CONFIG_FILE)
}

pub fn startup_path(state_root: &Path) -> PathBuf {
    state_root.join(STARTUP_FILE)
}

pub fn shutdown_path(state_root: &Path) -> PathBuf {
    state_root.join(SHUTDOWN_FILE)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use super::*;

    #[test]
    fn local_node_init_run_status_stop_and_restart_recovery_are_receipted() {
        let root = temp_dir("node-daemon-lifecycle");
        let init = init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:test",
        })
        .expect("init node");
        assert!(init.config_ref.starts_with("blake3:"));
        let run = run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        assert!(run.startup_ref.starts_with("blake3:"));
        assert_eq!(run.adapter_receipt_refs.len(), node_runtime::REQUIRED_RUNTIME_ADAPTERS.len());
        let status = status_local_node(&NodeDaemonStatusInput { state_root: &root }).expect("status node");
        assert_eq!(status.status, "running");
        let stop = stop_local_node(&NodeDaemonStopInput { state_root: &root }).expect("stop node");
        assert!(stop.shutdown_ref.starts_with("blake3:"));
        let stopped = status_local_node(&NodeDaemonStatusInput { state_root: &root }).expect("stopped status");
        assert_eq!(stopped.status, "stopped");
        let restarted = run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("restart node");
        assert!(restarted.startup_ref.starts_with("blake3:"));
        let restarted_status =
            status_local_node(&NodeDaemonStatusInput { state_root: &root }).expect("restarted status");
        assert_eq!(restarted_status.status, "running");
        let stale = run_local_node(&NodeDaemonRunInput { state_root: &root }).expect_err("stale running state denied");
        assert!(stale.to_string().contains("previous startup has no clean shutdown receipt"));
        let startup = node_runtime::parse_node_startup_receipt(&run.startup_value).expect("startup parse");
        let restart = node_runtime::node_restart_health_receipt_value(&node_runtime::RestartHealthReceiptValueInput {
            startup_receipt: &startup,
            shutdown_receipt_ref: Some(&stop.shutdown_ref),
            index_receipt_refs: &index_receipt_refs(&root).expect("index refs"),
            head_refs: std::slice::from_ref(&run.startup_ref),
            open_job_refs: &[],
            diagnostics: &[],
        })
        .expect("restart health");
        let restart_health = node_runtime::parse_node_health_receipt(&restart).expect("parse health");
        assert_eq!(restart_health.decision, "pass");
    }

    #[test]
    fn ambient_current_directory_state_root_is_denied() {
        let denied = init_local_node(&NodeDaemonInitInput {
            state_root: Path::new("."),
            node_id: "node:test",
        })
        .expect_err("ambient state denied");
        assert!(denied.to_string().contains("ambient current directory"));
        let request = status_request().expect("status request");
        let control_denied = submit_control_request(&NodeControlSubmitInput {
            state_root: Path::new("."),
            request_value: &request.value,
        })
        .expect_err("ambient control denied");
        assert!(control_denied.to_string().contains("ambient current directory"));
    }

    #[test]
    fn control_inbox_dispatch_imports_receipts_and_denies_missing_operation_payloads() {
        let root = temp_dir("node-control-socket");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:control",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let status_request = status_request().expect("status request");
        let submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("submit status");
        assert!(submitted.inbox_path.exists());
        let dispatched = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch status");
        assert_eq!(dispatched.operation, "status");
        let receipt =
            node_runtime::parse_node_control_receipt(&dispatched.control_receipt_value).expect("control receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.request_ref, status_request.request_ref);
        let kinds = ledger::list_artifacts(&root.join("ledger"))
            .expect("list ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<Vec<_>>();
        assert!(kinds.iter().any(|kind| kind == "node-control-request"));
        assert!(kinds.iter().any(|kind| kind == "node-control-queue-receipt"));
        assert!(kinds.iter().any(|kind| kind == "node-health-receipt"));
        assert!(kinds.iter().any(|kind| kind == "node-control-receipt"));

        let target_ref = local_ref("install-target", "fixture").expect("target ref");
        let install_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: Some(&target_ref),
            payload_ref: None,
            authority_refs: &status_request.authority_refs,
            policy_refs: &status_request.policy_refs,
            resource_refs: &status_request.resource_refs,
            evidence_refs: &[],
        })
        .expect("install request");
        let install_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &install_value,
        })
        .expect("submit install");
        let install_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&install_submitted.inbox_path),
        })
        .expect("dispatch install");
        let install_receipt =
            node_runtime::parse_node_control_receipt(&install_dispatch.control_receipt_value).expect("install receipt");
        assert_eq!(install_receipt.decision, "deny");
        assert!(install_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("requires payload ref")));

        let missing_authority = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &[],
            policy_refs: &status_request.policy_refs,
            resource_refs: &status_request.resource_refs,
            evidence_refs: &[],
        })
        .expect("missing authority request");
        let missing_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &missing_authority,
        })
        .expect("submit missing authority");
        let missing_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&missing_submitted.inbox_path),
        })
        .expect("dispatch missing authority");
        let missing_receipt =
            node_runtime::parse_node_control_receipt(&missing_dispatch.control_receipt_value).expect("missing receipt");
        assert_eq!(missing_receipt.decision, "deny");
        assert!(missing_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority refs missing")));

        let shutdown_request = shutdown_request().expect("shutdown request");
        let shutdown_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &shutdown_request.value,
        })
        .expect("submit shutdown");
        let shutdown_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&shutdown_submitted.inbox_path),
        })
        .expect("dispatch shutdown");
        let shutdown_receipt = node_runtime::parse_node_control_receipt(&shutdown_dispatch.control_receipt_value)
            .expect("shutdown receipt");
        assert_eq!(shutdown_receipt.decision, "pass");
        assert!(!root.join(CONTROL_LOCK_FILE).exists());

        let after_stop = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: None,
        })
        .expect_err("dispatch requires lock");
        assert!(after_stop.to_string().contains("active node lock"));
    }

    #[test]
    fn control_loop_processes_queue_idempotently_and_stops_on_shutdown() {
        let root = temp_dir("node-control-loop");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:loop",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let status_request = status_request().expect("status request");
        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("submit status");
        let first_loop = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("run one status request");
        assert_eq!(first_loop.processed_request_refs, vec![status_request.request_ref.clone()]);
        assert!(!first_loop.has_stopped);
        assert_eq!(ledger::artifact_kind(&first_loop.loop_receipt_value), "node-control-loop-receipt");
        assert_eq!(ledger::artifact_kind(&first_loop.heartbeat_receipt_value), "node-control-heartbeat-receipt");

        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("resubmit duplicate status");
        let duplicate_loop = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("run duplicate status request");
        assert_eq!(duplicate_loop.processed_request_refs, vec![status_request.request_ref.clone()]);
        assert_eq!(duplicate_loop.dispatch_receipt_refs, first_loop.dispatch_receipt_refs);

        let shutdown_request = shutdown_request().expect("shutdown request");
        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &shutdown_request.value,
        })
        .expect("submit shutdown");
        let shutdown_loop = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: DEFAULT_CONTROL_LOOP_REQUESTS,
        })
        .expect("run shutdown request");
        assert!(shutdown_loop.has_stopped);
        assert!(!root.join(CONTROL_LOCK_FILE).exists());
        let after_stop = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect_err("stopped node loop denied");
        assert!(after_stop.to_string().contains("active node lock"));

        let kinds = ledger::list_artifacts(&root.join("ledger"))
            .expect("list loop ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<Vec<_>>();
        assert!(kinds.iter().any(|kind| kind == "node-control-loop-receipt"));
        assert!(kinds.iter().any(|kind| kind == "node-control-heartbeat-receipt"));
    }

    #[test]
    fn duplicate_request_with_conflicting_archive_fails_closed() {
        let root = temp_dir("node-control-duplicate-conflict");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:duplicate",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let status_request = status_request().expect("status request");
        let submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("submit status");
        dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch status");
        write_preserves(
            &control_outbox_request_path(&root, &status_request.request_ref),
            &record("tampered-node-control-request", vec![string("conflict")]),
        )
        .expect("tamper archived request");
        let duplicate = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &status_request.value,
        })
        .expect("resubmit duplicate");
        let denied = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&duplicate.inbox_path),
        })
        .expect_err("conflicting duplicate denied");
        assert!(denied.to_string().contains("conflicts with archived request evidence"));
    }

    #[test]
    fn node_control_provenance_gate_denies_missing_and_tampered_evidence_before_side_effects() {
        let root = temp_dir("node-control-provenance");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:provenance",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "provenance").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "provenance").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "provenance").expect("resource ref")];

        let payload_value = record("node-control-install-payload", vec![string("missing-provenance")]);
        let payload_ref = import_node_artifact(&root, &payload_value).expect("import payload");
        let missing_provenance_request =
            node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
                operation: "install",
                target_ref: None,
                payload_ref: Some(&payload_ref),
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("missing provenance request");
        let submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &missing_provenance_request,
        })
        .expect("submit missing provenance");
        let dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&submitted.inbox_path),
        })
        .expect("dispatch missing provenance");
        let receipt =
            node_runtime::parse_node_control_receipt(&dispatch.control_receipt_value).expect("control receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.subreceipt_refs.iter().any(|reference| reference.starts_with("blake3:")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("provenance evidence refs missing")));
        assert!(
            artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
                .expect("list registry")
                .is_empty()
        );

        let queued_payload = record("node-control-install-payload", vec![string("queued-missing-provenance")]);
        let queued_payload_ref = import_node_artifact(&root, &queued_payload).expect("import queued payload");
        let queued_request = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&queued_payload_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("queued missing provenance request");
        let queued = node_runtime::parse_node_control_request(&queued_request).expect("queued request parse");
        submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &queued_request,
        })
        .expect("submit queued missing provenance");
        let loop_result = run_control_loop(&NodeControlLoopInput {
            state_root: &root,
            max_requests: 1,
        })
        .expect("process queued missing provenance");
        assert_eq!(loop_result.processed_request_refs, vec![queued.request_ref.clone()]);
        let queued_receipt_value =
            read_preserves(&control_outbox_receipt_path(&root, &queued.request_ref)).expect("queued receipt value");
        let queued_receipt = node_runtime::parse_node_control_receipt(&queued_receipt_value).expect("queued receipt");
        assert_eq!(queued_receipt.decision, "deny");
        assert!(
            queued_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("missing provenance evidence"))
        );

        let tampered_payload = record("node-control-install-payload", vec![string("tampered-provenance")]);
        let tampered_payload_ref = import_node_artifact(&root, &tampered_payload).expect("import tampered payload");
        let wrong_artifact_ref = local_ref("node-control-wrong-provenance-artifact", "tampered").expect("wrong ref");
        let wrong_provenance =
            provenance::synthetic_reviewed_provenance_record(&wrong_artifact_ref).expect("wrong provenance");
        let wrong_provenance_ref = import_node_artifact(&root, &wrong_provenance).expect("import wrong provenance");
        let tampered_evidence_refs = vec![wrong_provenance_ref];
        let tampered_request = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&tampered_payload_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &tampered_evidence_refs,
        })
        .expect("tampered request");
        let tampered_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &tampered_request,
        })
        .expect("submit tampered provenance");
        let tampered_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&tampered_submitted.inbox_path),
        })
        .expect("dispatch tampered provenance");
        let tampered_receipt = node_runtime::parse_node_control_receipt(&tampered_dispatch.control_receipt_value)
            .expect("tampered receipt");
        assert_eq!(tampered_receipt.decision, "deny");
        assert!(
            tampered_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("no provenance record matches"))
        );
        assert!(
            artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
                .expect("list registry after tampered")
                .is_empty()
        );
    }

    #[test]
    fn control_operation_dispatch_installs_runs_and_gates_with_receipts() {
        let root = temp_dir("node-control-operations");
        init_local_node(&NodeDaemonInitInput {
            state_root: &root,
            node_id: "node:ops",
        })
        .expect("init node");
        run_local_node(&NodeDaemonRunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "ops").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "ops").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "ops").expect("resource ref")];

        let payload_value = record("node-control-install-payload", vec![string("payload")]);
        let payload_ref = import_node_artifact(&root, &payload_value).expect("import payload");
        let payload_provenance =
            provenance::synthetic_reviewed_provenance_record(&payload_ref).expect("payload provenance");
        let payload_provenance_ref =
            import_node_artifact(&root, &payload_provenance).expect("import payload provenance");
        let install_evidence_refs = vec![payload_provenance_ref];
        let install_value = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&payload_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &install_evidence_refs,
        })
        .expect("install request");
        let install_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &install_value,
        })
        .expect("submit install");
        let install_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&install_submitted.inbox_path),
        })
        .expect("dispatch install");
        let install_receipt =
            node_runtime::parse_node_control_receipt(&install_dispatch.control_receipt_value).expect("install receipt");
        assert_eq!(install_receipt.decision, "pass");
        let installed = artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
            .expect("list installed artifacts");
        assert_eq!(installed.len(), 1);

        let gate_value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("gate receipt");
        let gate_ref = import_node_artifact(&root, &gate_value).expect("import gate");
        let gate_target = local_ref("node-control-gate-target", "ops").expect("gate target");
        let gate_request = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "gate",
            target_ref: Some(&gate_target),
            payload_ref: Some(&gate_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("gate request");
        let gate_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &gate_request,
        })
        .expect("submit gate");
        let gate_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&gate_submitted.inbox_path),
        })
        .expect("dispatch gate");
        let gate_receipt =
            node_runtime::parse_node_control_receipt(&gate_dispatch.control_receipt_value).expect("gate receipt");
        assert_eq!(gate_receipt.decision, "pass");
        assert!(gate_receipt.subreceipt_refs.iter().any(|reference| reference.starts_with("blake3:")));

        let job_fixture = install_node_job_fixture(&root);
        let execution_request_ref =
            import_node_artifact(&root, &job_fixture.execution_request).expect("import execution request");
        let admission_ref =
            import_node_artifact(&root, &job_fixture.admission_receipt).expect("import admission receipt");
        let job_provenance =
            provenance::synthetic_reviewed_provenance_record(&job_fixture.job_ref).expect("job provenance");
        let job_provenance_ref = import_node_artifact(&root, &job_provenance).expect("import job provenance");
        let run_evidence_refs = vec![job_provenance_ref];
        let run_request = node_runtime::node_control_request_value(&node_runtime::ControlRequestValueInput {
            operation: "run",
            target_ref: Some(&admission_ref),
            payload_ref: Some(&execution_request_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &run_evidence_refs,
        })
        .expect("run request");
        let run_submitted = submit_control_request(&NodeControlSubmitInput {
            state_root: &root,
            request_value: &run_request,
        })
        .expect("submit run");
        let run_dispatch = dispatch_control_request(&NodeControlDispatchInput {
            state_root: &root,
            request_path: Some(&run_submitted.inbox_path),
        })
        .expect("dispatch run");
        let run_receipt =
            node_runtime::parse_node_control_receipt(&run_dispatch.control_receipt_value).expect("run receipt");
        assert_eq!(run_receipt.decision, "pass");

        let kinds = ledger::list_artifacts(&root.join("ledger"))
            .expect("list operation ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<Vec<_>>();
        assert!(kinds.iter().any(|kind| kind == "artifact-registry-receipt"));
        assert!(kinds.iter().any(|kind| kind == "provenance-record"));
        assert!(kinds.iter().any(|kind| kind == "provenance-receipt"));
        assert!(kinds.iter().any(|kind| kind == "job-execution-receipt"));
        assert!(kinds.iter().any(|kind| kind == "octet-source-gate-validation"));
        assert!(kinds.iter().any(|kind| kind == "node-control-operation-receipt"));
    }

    struct NodeJobFixture {
        execution_request: IOValue,
        admission_receipt: IOValue,
        job_ref: String,
    }

    fn install_node_job_fixture(root: &Path) -> NodeJobFixture {
        let registry = root.join("registry");
        let stage_schema = local_ref("node-job-stage-schema", "ops").expect("stage schema");
        let stage_policy = local_ref("node-job-stage-policy", "ops").expect("stage policy");
        let stage_evidence = local_ref("node-job-stage-evidence", "ops").expect("stage evidence");
        let stage_installer = local_ref("node-job-stage-installer", "ops").expect("stage installer");
        let stage_capability = local_ref("node-job-stage-capability", "ops").expect("stage capability");
        let source_stage = artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: job_dag::builtin_stage_operation_value("source").expect("source operation"),
            schema_refs: vec![stage_schema.clone()],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![stage_policy.clone()],
            evidence_refs: vec![stage_evidence.clone()],
            installer_ref: stage_installer.clone(),
            capability_refs: vec![stage_capability.clone()],
        })
        .expect("install source stage");
        let map_stage = artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: job_dag::builtin_stage_operation_value("identity").expect("identity operation"),
            schema_refs: vec![stage_schema],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![stage_policy],
            evidence_refs: vec![stage_evidence],
            installer_ref: stage_installer,
            capability_refs: vec![stage_capability],
        })
        .expect("install map stage");
        let source_node = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: Some(&source_stage.artifact_ref),
            input_ports: &[],
            output_ports: &["out".to_string()],
            config: record("source", vec![record("values", vec![sequence(vec![string("node-job")])])]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("source node");
        let map_node = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "map",
            kind: "map",
            stage_artifact_ref: Some(&map_stage.artifact_ref),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: record("op", vec![string("identity")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("map node");
        let edge = job_dag::job_edge_value(job_dag::EdgeValueInput {
            from_node: "source",
            from_port: "out",
            to_node: "map",
            to_port: "in",
            schema_ref: None,
            partitioning: "single",
            materialization: "stream",
        })
        .expect("edge");
        let dag_value = job_dag::job_dag_value(job_dag::DagValueInput {
            nodes: vec![source_node, map_node],
            edges: vec![edge],
            output_roots: &["map".to_string()],
            schema_refs: &[],
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("dag value");
        let installed = job_dag::install_job_dag(&registry, &dag_value).expect("install job dag");
        let authority_ref = install_node_job_authority(&registry, &installed.job_ref);
        let gate_ref = install_node_clean_gate(&registry);
        let sync_ref = local_ref("node-job-sync", &installed.job_ref).expect("sync ref");
        let resource_refs = vec![local_ref("node-job-resource", &installed.job_ref).expect("resource ref")];
        let policy_refs = vec![local_ref("node-job-policy", &installed.job_ref).expect("policy ref")];
        let capability_refs = vec![authority_ref.clone()];
        let evidence_refs = vec![sync_ref.clone(), gate_ref];
        let admission_request = job_dag::job_admission_request_value(job_dag::AdmissionRequestValueInput {
            job_ref: &installed.job_ref,
            sync_ref: &sync_ref,
            stage_ids: &[],
            target_peer: "node:ops",
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            evidence_refs: &evidence_refs,
            resource_refs: &resource_refs,
        })
        .expect("admission request");
        let admission = job_dag::admission_loopback(&registry, &admission_request).expect("admission loopback");
        assert_eq!(admission.plan.decision, "pass");
        let admission_ref = canonical_hash(&admission.receipt_value).expect("admission ref");
        let execution_request = job_dag::job_execution_request_value(job_dag::ExecutionRequestValueInput {
            job_ref: &installed.job_ref,
            admission_ref: &admission_ref,
            stage_ids: &admission.plan.stage_order,
            target_peer: "node:ops",
            storage_profile_ref: &local_ref("node-job-storage", &installed.job_ref).expect("storage ref"),
            cache_profile_ref: &local_ref("node-job-cache", &installed.job_ref).expect("cache ref"),
            chunk_profile_ref: &local_ref("node-job-chunks", &installed.job_ref).expect("chunks ref"),
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            resource_refs: &resource_refs,
        })
        .expect("execution request");
        NodeJobFixture {
            execution_request,
            admission_receipt: admission.receipt_value,
            job_ref: installed.job_ref,
        }
    }

    fn install_node_job_authority(registry: &Path, job_ref: &str) -> String {
        let subject_ref = local_ref("node-job-authority-subject", job_ref).expect("authority subject");
        let policy_ref = local_ref("node-job-authority-policy", job_ref).expect("authority policy");
        let evidence_ref = local_ref("node-job-authority-evidence", job_ref).expect("authority evidence");
        let context_value = crate::authority::authority_context_value(crate::authority::ContextValueInput {
            subject_ref: &subject_ref,
            capabilities: &[crate::authority::AuthorityCapability {
                capability: "job:execute".to_string(),
                scope: job_ref.to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: std::slice::from_ref(&policy_ref),
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("authority context");
        let context_ref = canonical_hash(&context_value).expect("authority context ref");
        let install = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
            kind: "authority-context".to_string(),
            payload: context_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![policy_ref],
            evidence_refs: vec![evidence_ref],
            installer_ref: local_ref("node-job-authority-installer", job_ref).expect("authority installer"),
            capability_refs: vec![local_ref("node-job-authority-capability", job_ref).expect("authority capability")],
        })
        .expect("install authority context");
        assert_eq!(install.decision, "pass");
        context_ref
    }

    fn install_node_clean_gate(registry: &Path) -> String {
        let gate_value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean gate");
        let gate_ref = canonical_hash(&gate_value).expect("gate ref");
        let install = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
            kind: "octet-gate-receipt".to_string(),
            payload: gate_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![local_ref("node-job-gate-policy", &gate_ref).expect("gate policy")],
            evidence_refs: vec![local_ref("node-job-gate-evidence", &gate_ref).expect("gate evidence")],
            installer_ref: local_ref("node-job-gate-installer", &gate_ref).expect("gate installer"),
            capability_refs: vec![local_ref("node-job-gate-capability", &gate_ref).expect("gate capability")],
        })
        .expect("install gate");
        assert_eq!(install.decision, "pass");
        gate_ref
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
