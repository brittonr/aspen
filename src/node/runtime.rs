use preserves::IOValue;
use preserves::Record;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;
use crate::octet_gate;
use crate::preserves_rail::NODE_ADAPTER_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONFIG_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_CONTROL_REQUEST_SCHEMA;
use crate::preserves_rail::NODE_HEALTH_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_SHUTDOWN_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_STARTUP_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::value_to_iovalue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeAdapterBinding {
    pub name: String,
    pub profile_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeAdapterReceiptRef {
    pub name: String,
    pub receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRuntimeStartInput {
    pub config_value: IOValue,
    pub identity_receipt_ref: String,
    pub index_receipt_refs: Vec<String>,
    pub source_gate_receipt_refs: Vec<String>,
    pub source_gate_receipt_values: Vec<IOValue>,
    pub capability_receipt_refs: Vec<String>,
    pub resource_receipt_refs: Vec<String>,
    pub version_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigValueInput<'a> {
    pub node_identity_ref: &'a str,
    pub state_root_ref: &'a str,
    pub adapters: &'a [NodeAdapterBinding],
    pub policy_refs: &'a [String],
    pub capability_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub effect_profile_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct AdapterLifecycleReceiptInput<'a> {
    pub operation: &'a str,
    pub decision: &'a str,
    pub adapter: &'a NodeAdapterBinding,
    pub index_receipt_refs: &'a [String],
    pub resource_receipt_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct StartupReceiptValueInput<'a> {
    pub decision: &'a str,
    pub config: &'a NodeConfig,
    pub identity_receipt_ref: &'a str,
    pub adapter_receipts: &'a [NodeAdapterReceiptRef],
    pub source_gate_receipt_refs: &'a [String],
    pub source_gate_validation_refs: &'a [String],
    pub capability_receipt_refs: &'a [String],
    pub resource_receipt_refs: &'a [String],
    pub version_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlRequestValueInput<'a> {
    pub operation: &'a str,
    pub target_ref: Option<&'a str>,
    pub payload_ref: Option<&'a str>,
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlReceiptValueInput<'a> {
    pub decision: &'a str,
    pub request: &'a NodeControlRequest,
    pub startup_receipt_ref: &'a str,
    pub authority_receipt_refs: &'a [String],
    pub resource_receipt_refs: &'a [String],
    pub subreceipt_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ShutdownReceiptValueInput<'a> {
    pub decision: &'a str,
    pub startup_receipt_ref: &'a str,
    pub adapter_receipts: &'a [NodeAdapterReceiptRef],
    pub drained_job_refs: &'a [String],
    pub index_receipt_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct HealthReceiptValueInput<'a> {
    pub decision: &'a str,
    pub startup_receipt_ref: &'a str,
    pub shutdown_receipt_ref: Option<&'a str>,
    pub adapter_receipts: &'a [NodeAdapterReceiptRef],
    pub index_receipt_refs: &'a [String],
    pub head_refs: &'a [String],
    pub open_job_refs: &'a [String],
    pub replay_is_eligible: bool,
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct RestartHealthReceiptValueInput<'a> {
    pub startup_receipt: &'a NodeStartupReceipt,
    pub shutdown_receipt_ref: Option<&'a str>,
    pub index_receipt_refs: &'a [String],
    pub head_refs: &'a [String],
    pub open_job_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRuntimeStart {
    pub decision: String,
    pub config: NodeConfig,
    pub adapter_receipt_values: Vec<IOValue>,
    pub adapter_receipts: Vec<NodeAdapterReceiptRef>,
    pub startup_receipt: NodeStartupReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlRequest {
    pub request_ref: String,
    pub operation: String,
    pub target_ref: Option<String>,
    pub payload_ref: Option<String>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub request_ref: String,
    pub startup_receipt_ref: String,
    pub authority_receipt_refs: Vec<String>,
    pub resource_receipt_refs: Vec<String>,
    pub subreceipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeShutdownReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub startup_receipt_ref: String,
    pub adapters: Vec<NodeAdapterReceiptRef>,
    pub drained_job_refs: Vec<String>,
    pub index_receipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeHealthReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub startup_receipt_ref: String,
    pub shutdown_receipt_ref: Option<String>,
    pub adapters: Vec<NodeAdapterReceiptRef>,
    pub index_receipt_refs: Vec<String>,
    pub head_refs: Vec<String>,
    pub open_job_refs: Vec<String>,
    pub replay_status: String,
    pub diagnostics: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeConfig {
    pub config_ref: String,
    pub node_identity_ref: String,
    pub state_root_ref: String,
    pub adapters: Vec<NodeAdapterBinding>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_profile_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeStartupReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub config_ref: String,
    pub identity_receipt_ref: String,
    pub adapters: Vec<NodeAdapterReceiptRef>,
    pub policy_refs: Vec<String>,
    pub source_gate_receipt_refs: Vec<String>,
    pub source_gate_validation_refs: Vec<String>,
    pub capability_receipt_refs: Vec<String>,
    pub resource_receipt_refs: Vec<String>,
    pub version_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub checks: Vec<String>,
    pub value: IOValue,
}

pub fn node_adapter_binding(name: &str, profile_ref: &str) -> Result<NodeAdapterBinding> {
    validate_adapter_name(name)?;
    validate_ref(profile_ref, "node adapter profile ref")?;
    Ok(NodeAdapterBinding {
        name: name.to_string(),
        profile_ref: profile_ref.to_string(),
    })
}

pub fn node_config_value(input: &ConfigValueInput<'_>) -> Result<IOValue> {
    validate_ref(input.node_identity_ref, "node config identity ref")?;
    validate_ref(input.state_root_ref, "node config state root ref")?;
    if input.adapters.is_empty() {
        return Err(MoltenError::invalid_harness("node config requires explicit adapter profiles"));
    }
    validate_refs(input.policy_refs, "node config policy ref")?;
    validate_refs(input.capability_refs, "node config capability ref")?;
    validate_refs(input.resource_refs, "node config resource ref")?;
    validate_refs(input.effect_profile_refs, "node config effect profile ref")?;
    Ok(record("node-config-v1", vec![
        string(NODE_CONFIG_SCHEMA),
        record("node-id", vec![string(input.node_identity_ref)]),
        record("state-root", vec![string(input.state_root_ref)]),
        record("adapters", vec![sequence(input.adapters.iter().map(adapter_binding_value).collect())]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("capability", vec![refs_sequence(input.capability_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("effects", vec![refs_sequence(input.effect_profile_refs)]),
        checks_value(&[
            ("explicit-state-root", "pass"),
            ("explicit-adapter-profiles", "pass"),
            ("no-ambient-authority", "pass"),
        ]),
    ]))
}

pub const REQUIRED_RUNTIME_ADAPTERS: &[&str] = &[
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
];

const MAX_NODE_ADAPTERS: usize = 16;
const MAX_NODE_SOURCE_GATE_RECEIPTS: usize = 16;
const MAX_NODE_DIAGNOSTICS: usize = 64;

const _: () = assert!(MAX_NODE_ADAPTERS >= REQUIRED_RUNTIME_ADAPTERS.len());
const _: () = assert!(MAX_NODE_SOURCE_GATE_RECEIPTS > 0);
const _: () = assert!(MAX_NODE_DIAGNOSTICS > 0);

struct GateScan {
    validation_refs: Vec<String>,
    diagnostics: Vec<String>,
}

struct AdapterStart {
    values: Vec<IOValue>,
    receipts: Vec<NodeAdapterReceiptRef>,
}

fn validate_start_input(config: &NodeConfig, input: &NodeRuntimeStartInput) -> Result<()> {
    validate_refs(&input.index_receipt_refs, "node runtime index receipt ref")?;
    validate_refs(&input.source_gate_receipt_refs, "node runtime source gate receipt ref")?;
    validate_refs(&input.capability_receipt_refs, "node runtime capability receipt ref")?;
    validate_refs(&input.resource_receipt_refs, "node runtime resource receipt ref")?;
    validate_refs(&input.version_refs, "node runtime version ref")?;
    ensure_count_at_most(config.adapters.len(), MAX_NODE_ADAPTERS, "node runtime adapters")?;
    ensure_count_at_most(
        input.source_gate_receipt_refs.len(),
        MAX_NODE_SOURCE_GATE_RECEIPTS,
        "node runtime source gate receipt refs",
    )?;
    ensure_count_at_most(
        input.source_gate_receipt_values.len(),
        MAX_NODE_SOURCE_GATE_RECEIPTS,
        "node runtime source gate receipt values",
    )?;
    validate_ref(&input.identity_receipt_ref, "node runtime identity receipt ref")
}

fn startup_diagnostics(config: &NodeConfig, input: &NodeRuntimeStartInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let missing = missing_required_adapters(&config.adapters);
    if !missing.is_empty() {
        push_bounded(
            &mut diagnostics,
            format!("missing required node runtime adapters: {}", missing.join(",")),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    if input.index_receipt_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "node runtime startup requires adapter index verification receipts".to_string(),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    if input.source_gate_receipt_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "node runtime startup requires strict Octet source gate receipt refs".to_string(),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    if input.resource_receipt_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "node runtime startup requires resource profile receipts".to_string(),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    if input.source_gate_receipt_refs.len() != input.source_gate_receipt_values.len() {
        push_bounded(
            &mut diagnostics,
            "node runtime source gate refs must have matching receipt values".to_string(),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn scan_gates(config_ref: &str, input: &NodeRuntimeStartInput) -> Result<GateScan> {
    let mut validation_refs = Vec::with_capacity(input.source_gate_receipt_values.len());
    let mut diagnostics = Vec::new();
    for (index, value) in input.source_gate_receipt_values.iter().enumerate() {
        let validation = octet_gate::validate_octet_source_gate(&octet_gate::OctetSourceGateValidationInput {
            consumer: "node-startup".to_string(),
            subject_ref: config_ref.to_string(),
            gate_receipt_value: Some(value.clone()),
            source_scope: Vec::new(),
        })?;
        if let Some(expected_ref) = input.source_gate_receipt_refs.get(index)
            && validation.gate_receipt_ref.as_ref() != Some(expected_ref)
        {
            push_bounded(
                &mut diagnostics,
                format!(
                    "node runtime source gate ref {expected_ref} does not match validated receipt {:?}",
                    validation.gate_receipt_ref
                ),
                MAX_NODE_DIAGNOSTICS,
                "node runtime startup diagnostics",
            )?;
        }
        push_bounded(
            &mut validation_refs,
            validation.validation_ref.clone(),
            MAX_NODE_SOURCE_GATE_RECEIPTS,
            "node runtime source gate validation refs",
        )?;
        if validation.decision != "pass" {
            push_bounded(
                &mut diagnostics,
                format!("node runtime strict Octet source gate validation {} denied", validation.validation_ref),
                MAX_NODE_DIAGNOSTICS,
                "node runtime startup diagnostics",
            )?;
        }
    }
    Ok(GateScan {
        validation_refs,
        diagnostics,
    })
}

fn adapter_start(
    ordered: &[NodeAdapterBinding],
    input: &NodeRuntimeStartInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<AdapterStart> {
    let mut values = Vec::with_capacity(ordered.len());
    for adapter in ordered {
        values.push(node_adapter_lifecycle_receipt_value(&AdapterLifecycleReceiptInput {
            operation: "start",
            decision,
            adapter,
            index_receipt_refs: &input.index_receipt_refs,
            resource_receipt_refs: &input.resource_receipt_refs,
            diagnostics,
        })?);
    }
    let mut receipts = Vec::with_capacity(ordered.len());
    for (adapter, receipt) in ordered.iter().zip(values.iter()) {
        receipts.push(NodeAdapterReceiptRef {
            name: adapter.name.clone(),
            receipt_ref: canonical_hash(receipt)?,
        });
    }
    Ok(AdapterStart { values, receipts })
}

pub fn start_node_runtime(input: &NodeRuntimeStartInput) -> Result<NodeRuntimeStart> {
    let config = parse_node_config(&input.config_value)?;
    validate_start_input(&config, input)?;
    let ordered = deterministic_adapter_bindings(&config.adapters);
    let mut diagnostics = startup_diagnostics(&config, input)?;
    let gate_scan = scan_gates(&config.config_ref, input)?;
    for diagnostic in gate_scan.diagnostics {
        push_bounded(&mut diagnostics, diagnostic, MAX_NODE_DIAGNOSTICS, "node runtime startup diagnostics")?;
    }
    let source_gate_validation_refs = gate_scan.validation_refs;
    let has_valid_source_gate_values = !source_gate_validation_refs.is_empty()
        && diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.contains("source gate") && !diagnostic.contains("Source gate"));
    if !has_valid_source_gate_values
        && !input.source_gate_receipt_refs.is_empty()
        && input.source_gate_receipt_values.is_empty()
    {
        push_bounded(
            &mut diagnostics,
            "node runtime source gate refs lack validated receipt content".to_string(),
            MAX_NODE_DIAGNOSTICS,
            "node runtime startup diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let adapter_start = adapter_start(&ordered, input, decision, &diagnostics)?;
    let startup_value = node_startup_receipt_value(&StartupReceiptValueInput {
        decision,
        config: &config,
        identity_receipt_ref: &input.identity_receipt_ref,
        adapter_receipts: &adapter_start.receipts,
        source_gate_receipt_refs: &input.source_gate_receipt_refs,
        source_gate_validation_refs: &source_gate_validation_refs,
        capability_receipt_refs: &input.capability_receipt_refs,
        resource_receipt_refs: &input.resource_receipt_refs,
        version_refs: &input.version_refs,
        diagnostics: &diagnostics,
    })?;
    let startup_receipt = parse_node_startup_receipt(&startup_value)?;
    Ok(NodeRuntimeStart {
        decision: decision.to_string(),
        config,
        adapter_receipt_values: adapter_start.values,
        adapter_receipts: adapter_start.receipts,
        startup_receipt,
    })
}

pub fn parse_node_config(value: &IOValue) -> Result<NodeConfig> {
    let fields = value
        .collect_simple_record("node-config-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-config-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONFIG_SCHEMA, "node config")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "explicit-state-root", "node config")?;
    require_check(&checks, "no-ambient-authority", "node config")?;
    let adapters = parse_adapter_bindings(&fields[3])?;
    if adapters.is_empty() {
        return Err(MoltenError::invalid_harness("node config requires explicit adapter profiles"));
    }
    Ok(NodeConfig {
        config_ref: canonical_hash(value)?,
        node_identity_ref: record_ref(&fields[1], "node-id")?,
        state_root_ref: record_ref(&fields[2], "state-root")?,
        adapters,
        policy_refs: record_ref_sequence(&fields[4], "policy")?,
        capability_refs: record_ref_sequence(&fields[5], "capability")?,
        resource_refs: record_ref_sequence(&fields[6], "resource")?,
        effect_profile_refs: record_ref_sequence(&fields[7], "effects")?,
        checks,
        value: value.clone(),
    })
}

pub fn node_adapter_receipt_value(
    operation: &str,
    decision: &str,
    adapter: &NodeAdapterBinding,
    diagnostics: &[String],
) -> Result<IOValue> {
    node_adapter_lifecycle_receipt_value(&AdapterLifecycleReceiptInput {
        operation,
        decision,
        adapter,
        index_receipt_refs: &[],
        resource_receipt_refs: &[],
        diagnostics,
    })
}

pub fn node_adapter_lifecycle_receipt_value(input: &AdapterLifecycleReceiptInput<'_>) -> Result<IOValue> {
    validate_adapter_operation(input.operation)?;
    validate_decision(input.decision)?;
    validate_refs(input.index_receipt_refs, "node adapter index receipt ref")?;
    validate_refs(input.resource_receipt_refs, "node adapter resource receipt ref")?;
    Ok(record("node-adapter-receipt-v1", vec![
        string(NODE_ADAPTER_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("adapter", vec![string(&input.adapter.name)]),
        record("profile", vec![string(&input.adapter.profile_ref)]),
        record("index", vec![refs_sequence(input.index_receipt_refs)]),
        record("resource", vec![refs_sequence(input.resource_receipt_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("adapter-profile-bound", "pass"),
            ("adapter-index-verified", status(!input.index_receipt_refs.is_empty())),
            ("adapter-resource-profile-bound", status(!input.resource_receipt_refs.is_empty())),
            ("no-invisible-startup", if input.decision == "pass" { "pass" } else { "fail" }),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn node_startup_receipt_value(input: &StartupReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_ref(input.identity_receipt_ref, "node startup identity receipt ref")?;
    validate_refs(input.source_gate_receipt_refs, "node startup source gate receipt ref")?;
    validate_refs(input.source_gate_validation_refs, "node startup source gate validation ref")?;
    validate_refs(input.capability_receipt_refs, "node startup capability receipt ref")?;
    validate_refs(input.resource_receipt_refs, "node startup resource receipt ref")?;
    validate_refs(input.version_refs, "node startup version ref")?;
    for receipt in input.adapter_receipts {
        validate_adapter_name(&receipt.name)?;
        validate_ref(&receipt.receipt_ref, "node startup adapter receipt ref")?;
    }
    let adapter_names = input.adapter_receipts.iter().map(|receipt| receipt.name.clone()).collect::<Vec<_>>();
    let has_deterministic_adapter_order = adapter_names == deterministic_adapter_order(&input.config.adapters);
    Ok(record("node-startup-receipt-v1", vec![
        string(NODE_STARTUP_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("node-config", vec![string(&input.config.config_ref)]),
        record("identity", vec![string(input.identity_receipt_ref)]),
        record("adapters", vec![sequence(
            input.adapter_receipts.iter().map(adapter_receipt_ref_value).collect(),
        )]),
        record("policy", vec![refs_sequence(&input.config.policy_refs)]),
        record("source-gates", vec![refs_sequence(input.source_gate_receipt_refs)]),
        record("source-gate-validations", vec![refs_sequence(input.source_gate_validation_refs)]),
        record("capability", vec![refs_sequence(input.capability_receipt_refs)]),
        record("resource", vec![refs_sequence(input.resource_receipt_refs)]),
        record("version", vec![refs_sequence(input.version_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("explicit-state-root", "pass"),
            ("adapter-order-deterministic", status(has_deterministic_adapter_order)),
            (
                "strict-octet-source-gate-bound",
                status(!input.source_gate_receipt_refs.is_empty() && !input.source_gate_validation_refs.is_empty()),
            ),
            ("no-ambient-authority", "pass"),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn node_control_request_value(input: &ControlRequestValueInput<'_>) -> Result<IOValue> {
    validate_control_operation(input.operation)?;
    if let Some(target_ref) = input.target_ref {
        validate_ref(target_ref, "node control target ref")?;
    }
    if let Some(payload_ref) = input.payload_ref {
        validate_ref(payload_ref, "node control payload ref")?;
    }
    validate_refs(input.authority_refs, "node control authority ref")?;
    validate_refs(input.policy_refs, "node control policy ref")?;
    validate_refs(input.resource_refs, "node control resource ref")?;
    validate_refs(input.evidence_refs, "node control evidence ref")?;
    Ok(record("node-control-request-v1", vec![
        string(NODE_CONTROL_REQUEST_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("target", vec![optional_ref_value(input.target_ref)]),
        record("payload", vec![optional_ref_value(input.payload_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
        record("control-profile", vec![string("local-preserves-control-v1")]),
        checks_value(&[
            ("local-only-control", "pass"),
            ("preserves-control-surface", "pass"),
            ("authority-refs-explicit", status(!input.authority_refs.is_empty())),
            ("resource-refs-explicit", status(!input.resource_refs.is_empty())),
            ("evidence-refs-canonical", "pass"),
        ]),
    ]))
}

pub fn legacy_node_control_request_value(input: &ControlRequestValueInput<'_>) -> Result<IOValue> {
    validate_control_operation(input.operation)?;
    if let Some(target_ref) = input.target_ref {
        validate_ref(target_ref, "node control target ref")?;
    }
    if let Some(payload_ref) = input.payload_ref {
        validate_ref(payload_ref, "node control payload ref")?;
    }
    validate_refs(input.authority_refs, "node control authority ref")?;
    validate_refs(input.policy_refs, "node control policy ref")?;
    validate_refs(input.resource_refs, "node control resource ref")?;
    Ok(record("node-control-request-v1", vec![
        string(NODE_CONTROL_REQUEST_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("target", vec![optional_ref_value(input.target_ref)]),
        record("payload", vec![optional_ref_value(input.payload_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("control-profile", vec![string("local-preserves-control-v1")]),
        checks_value(&[
            ("local-only-control", "pass"),
            ("preserves-control-surface", "pass"),
            ("authority-refs-explicit", status(!input.authority_refs.is_empty())),
            ("resource-refs-explicit", status(!input.resource_refs.is_empty())),
        ]),
    ]))
}

pub fn parse_node_control_request(value: &IOValue) -> Result<NodeControlRequest> {
    if let Some(fields) = value.collect_simple_record("node-control-request-v1", Some(10)) {
        require_schema(&fields[0], NODE_CONTROL_REQUEST_SCHEMA, "node control request")?;
        let operation = record_string(&fields[1], "operation")?;
        validate_control_operation(&operation)?;
        return Ok(NodeControlRequest {
            request_ref: canonical_hash(value)?,
            operation,
            target_ref: record_optional_ref(&fields[2], "target")?,
            payload_ref: record_optional_ref(&fields[3], "payload")?,
            authority_refs: record_ref_sequence(&fields[4], "authority")?,
            policy_refs: record_ref_sequence(&fields[5], "policy")?,
            resource_refs: record_ref_sequence(&fields[6], "resource")?,
            evidence_refs: record_ref_sequence(&fields[7], "evidence")?,
            value: value.clone(),
        });
    }
    let fields = value
        .collect_simple_record("node-control-request-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-request-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_REQUEST_SCHEMA, "node control request")?;
    let operation = record_string(&fields[1], "operation")?;
    validate_control_operation(&operation)?;
    Ok(NodeControlRequest {
        request_ref: canonical_hash(value)?,
        operation,
        target_ref: record_optional_ref(&fields[2], "target")?,
        payload_ref: record_optional_ref(&fields[3], "payload")?,
        authority_refs: record_ref_sequence(&fields[4], "authority")?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        resource_refs: record_ref_sequence(&fields[6], "resource")?,
        evidence_refs: Vec::new(),
        value: value.clone(),
    })
}

pub fn node_control_receipt_value(input: &ControlReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_ref(input.startup_receipt_ref, "node control startup receipt ref")?;
    validate_refs(input.authority_receipt_refs, "node control authority receipt ref")?;
    validate_refs(input.resource_receipt_refs, "node control resource receipt ref")?;
    validate_refs(input.subreceipt_refs, "node control subreceipt ref")?;
    let has_authority_receipts = !input.request.authority_refs.is_empty() && !input.authority_receipt_refs.is_empty();
    let has_resource_receipts = !input.request.resource_refs.is_empty() && !input.resource_receipt_refs.is_empty();
    let has_required_subreceipts =
        input.request.operation == "status" || !input.subreceipt_refs.is_empty() || input.decision == "deny";
    Ok(record("node-control-receipt-v1", vec![
        string(NODE_CONTROL_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("request", vec![string(&input.request.request_ref)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("authority", vec![refs_sequence(input.authority_receipt_refs)]),
        record("resource", vec![refs_sequence(input.resource_receipt_refs)]),
        record("subreceipts", vec![refs_sequence(input.subreceipt_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("local-preserves-control", "pass"),
            ("authority-gated", status(has_authority_receipts)),
            ("resource-gated", status(has_resource_receipts)),
            ("subreceipts-bound", status(has_required_subreceipts)),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn parse_node_control_receipt(value: &IOValue) -> Result<NodeControlReceipt> {
    let fields = value
        .collect_simple_record("node-control-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_CONTROL_RECEIPT_SCHEMA, "node control receipt")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "canonical-receipt", "node control receipt")?;
    Ok(NodeControlReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        request_ref: record_ref(&fields[2], "request")?,
        startup_receipt_ref: record_ref(&fields[3], "startup")?,
        authority_receipt_refs: record_ref_sequence(&fields[4], "authority")?,
        resource_receipt_refs: record_ref_sequence(&fields[5], "resource")?,
        subreceipt_refs: record_ref_sequence(&fields[6], "subreceipts")?,
        diagnostics: record_string_sequence(&fields[7], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn node_control_deny_receipt_value(
    request: &NodeControlRequest,
    startup_receipt_ref: &str,
    diagnostic: &str,
) -> Result<IOValue> {
    let diagnostics = [diagnostic.to_string()];
    node_control_receipt_value(&ControlReceiptValueInput {
        decision: "deny",
        request,
        startup_receipt_ref,
        authority_receipt_refs: &[],
        resource_receipt_refs: &[],
        subreceipt_refs: &[],
        diagnostics: &diagnostics,
    })
}

pub fn node_shutdown_receipt_value(input: &ShutdownReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_ref(input.startup_receipt_ref, "node shutdown startup receipt ref")?;
    validate_refs(input.drained_job_refs, "node shutdown drained job ref")?;
    validate_refs(input.index_receipt_refs, "node shutdown index receipt ref")?;
    for adapter in input.adapter_receipts {
        validate_adapter_name(&adapter.name)?;
        validate_ref(&adapter.receipt_ref, "node shutdown adapter receipt ref")?;
    }
    let is_graceful_shutdown =
        input.diagnostics.is_empty() && !input.adapter_receipts.is_empty() && !input.index_receipt_refs.is_empty();
    Ok(record("node-shutdown-receipt-v1", vec![
        string(NODE_SHUTDOWN_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("adapters", vec![sequence(
            input.adapter_receipts.iter().map(adapter_receipt_ref_value).collect(),
        )]),
        record("drained-jobs", vec![refs_sequence(input.drained_job_refs)]),
        record("indexes", vec![refs_sequence(input.index_receipt_refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("stop-intake", "pass"),
            ("drain-complete", status(input.diagnostics.is_empty())),
            ("indexes-persisted", status(!input.index_receipt_refs.is_empty())),
            ("adapters-closed", status(!input.adapter_receipts.is_empty())),
            ("graceful-shutdown", status(is_graceful_shutdown && input.decision == "pass")),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn parse_node_shutdown_receipt(value: &IOValue) -> Result<NodeShutdownReceipt> {
    let fields = value
        .collect_simple_record("node-shutdown-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-shutdown-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_SHUTDOWN_RECEIPT_SCHEMA, "node shutdown receipt")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-receipt", "node shutdown receipt")?;
    Ok(NodeShutdownReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        startup_receipt_ref: record_ref(&fields[2], "startup")?,
        adapters: parse_adapter_receipt_refs(&fields[3])?,
        drained_job_refs: record_ref_sequence(&fields[4], "drained-jobs")?,
        index_receipt_refs: record_ref_sequence(&fields[5], "indexes")?,
        diagnostics: record_string_sequence(&fields[6], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn node_health_receipt_value(input: &HealthReceiptValueInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_ref(input.startup_receipt_ref, "node health startup receipt ref")?;
    if let Some(shutdown_receipt_ref) = input.shutdown_receipt_ref {
        validate_ref(shutdown_receipt_ref, "node health shutdown receipt ref")?;
    }
    validate_refs(input.index_receipt_refs, "node health index receipt ref")?;
    validate_refs(input.head_refs, "node health head ref")?;
    validate_refs(input.open_job_refs, "node health open job ref")?;
    for adapter in input.adapter_receipts {
        validate_adapter_name(&adapter.name)?;
        validate_ref(&adapter.receipt_ref, "node health adapter receipt ref")?;
    }
    Ok(record("node-health-receipt-v1", vec![
        string(NODE_HEALTH_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("shutdown", vec![optional_ref_value(input.shutdown_receipt_ref)]),
        record("adapters", vec![sequence(
            input.adapter_receipts.iter().map(adapter_receipt_ref_value).collect(),
        )]),
        record("indexes", vec![refs_sequence(input.index_receipt_refs)]),
        record("heads", vec![refs_sequence(input.head_refs)]),
        record("open-jobs", vec![refs_sequence(input.open_job_refs)]),
        record("replay", vec![string(if input.replay_is_eligible {
            "eligible"
        } else {
            "ineligible"
        })]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&[
            ("startup-verified", "pass"),
            ("shutdown-verified", status(input.shutdown_receipt_ref.is_some())),
            ("adapter-indexes-current", status(!input.index_receipt_refs.is_empty())),
            ("health-heads-bound", status(!input.head_refs.is_empty())),
            ("no-open-jobs-for-replay", status(input.open_job_refs.is_empty())),
            ("replay-eligibility", status(input.replay_is_eligible)),
            ("canonical-receipt", "pass"),
        ]),
    ]))
}

pub fn parse_node_health_receipt(value: &IOValue) -> Result<NodeHealthReceipt> {
    let fields = value
        .collect_simple_record("node-health-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-health-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_HEALTH_RECEIPT_SCHEMA, "node health receipt")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "canonical-receipt", "node health receipt")?;
    Ok(NodeHealthReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        startup_receipt_ref: record_ref(&fields[2], "startup")?,
        shutdown_receipt_ref: record_optional_ref(&fields[3], "shutdown")?,
        adapters: parse_adapter_receipt_refs(&fields[4])?,
        index_receipt_refs: record_ref_sequence(&fields[5], "indexes")?,
        head_refs: record_ref_sequence(&fields[6], "heads")?,
        open_job_refs: record_ref_sequence(&fields[7], "open-jobs")?,
        replay_status: record_string(&fields[8], "replay")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

pub fn node_restart_health_receipt_value(input: &RestartHealthReceiptValueInput<'_>) -> Result<IOValue> {
    let mut health_diagnostics = input.diagnostics.to_vec();
    if input.startup_receipt.decision != "pass" {
        health_diagnostics.push("previous startup receipt did not pass".to_string());
    }
    if input.shutdown_receipt_ref.is_none() {
        health_diagnostics.push("previous shutdown receipt missing".to_string());
    }
    if input.index_receipt_refs.is_empty() {
        health_diagnostics.push("adapter indexes not verified on restart".to_string());
    }
    if !input.open_job_refs.is_empty() {
        health_diagnostics.push("restart has open jobs; replay not eligible".to_string());
    }
    let is_replay_eligible = health_diagnostics.is_empty();
    let decision = if is_replay_eligible { "pass" } else { "deny" };
    node_health_receipt_value(&HealthReceiptValueInput {
        decision,
        startup_receipt_ref: &input.startup_receipt.receipt_ref,
        shutdown_receipt_ref: input.shutdown_receipt_ref,
        adapter_receipts: &input.startup_receipt.adapters,
        index_receipt_refs: input.index_receipt_refs,
        head_refs: input.head_refs,
        open_job_refs: input.open_job_refs,
        replay_is_eligible: is_replay_eligible,
        diagnostics: &health_diagnostics,
    })
}

pub fn parse_node_startup_receipt(value: &IOValue) -> Result<NodeStartupReceipt> {
    let fields = value
        .collect_simple_record("node-startup-receipt-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-startup-receipt-v1 ...>"))?;
    require_schema(&fields[0], NODE_STARTUP_RECEIPT_SCHEMA, "node startup receipt")?;
    let checks = parse_checks(&fields[12])?;
    require_check(&checks, "canonical-receipt", "node startup receipt")?;
    Ok(NodeStartupReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        config_ref: record_ref(&fields[2], "node-config")?,
        identity_receipt_ref: record_ref(&fields[3], "identity")?,
        adapters: parse_adapter_receipt_refs(&fields[4])?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        source_gate_receipt_refs: record_ref_sequence(&fields[6], "source-gates")?,
        source_gate_validation_refs: record_ref_sequence(&fields[7], "source-gate-validations")?,
        capability_receipt_refs: record_ref_sequence(&fields[8], "capability")?,
        resource_receipt_refs: record_ref_sequence(&fields[9], "resource")?,
        version_refs: record_ref_sequence(&fields[10], "version")?,
        diagnostics: record_string_sequence(&fields[11], "diagnostics")?,
        checks,
        value: value.clone(),
    })
}

fn deterministic_adapter_order(adapters: &[NodeAdapterBinding]) -> Vec<String> {
    deterministic_adapter_bindings(adapters).into_iter().map(|adapter| adapter.name).collect()
}

fn deterministic_adapter_bindings(adapters: &[NodeAdapterBinding]) -> Vec<NodeAdapterBinding> {
    let mut adapters = adapters.to_vec();
    adapters.sort_by(|left, right| adapter_sort_key(&left.name).cmp(&adapter_sort_key(&right.name)));
    adapters
}

fn adapter_sort_key(name: &str) -> (bool, usize, &str) {
    match REQUIRED_RUNTIME_ADAPTERS.iter().position(|required| required == &name) {
        Some(rank) => (false, rank, name),
        None => (true, 0, name),
    }
}

fn missing_required_adapters(adapters: &[NodeAdapterBinding]) -> Vec<String> {
    REQUIRED_RUNTIME_ADAPTERS
        .iter()
        .filter(|required| !adapters.iter().any(|adapter| adapter.name == **required))
        .map(|required| (*required).to_string())
        .collect()
}

fn ensure_unique_adapter_names(adapters: &[NodeAdapterBinding]) -> Result<()> {
    for (index, adapter) in adapters.iter().enumerate() {
        if adapters.iter().skip(index + 1).any(|other| other.name == adapter.name) {
            return Err(MoltenError::invalid_harness(format!("duplicate node adapter name {}", adapter.name)));
        }
    }
    Ok(())
}

fn adapter_binding_value(binding: &NodeAdapterBinding) -> IOValue {
    record("adapter", vec![string(&binding.name), string(&binding.profile_ref)])
}

fn adapter_receipt_ref_value(receipt: &NodeAdapterReceiptRef) -> IOValue {
    record("adapter", vec![string(&receipt.name), string(&receipt.receipt_ref)])
}

fn parse_adapter_bindings(value: &Value<IOValue>) -> Result<Vec<NodeAdapterBinding>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, "adapters", 1)?;
    let items = required_sequence(&record[0], "node adapters")?;
    ensure_count_at_most(items.len(), MAX_NODE_ADAPTERS, "node adapters")?;
    let mut adapters = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let fields = simple_record(&item, "adapter", 2)?;
        adapters.push(node_adapter_binding(
            &required_string(&fields[0], "adapter name")?,
            &required_ref(&fields[1], "adapter profile")?,
        )?);
    }
    ensure_unique_adapter_names(&adapters)?;
    Ok(adapters)
}

fn parse_adapter_receipt_refs(value: &Value<IOValue>) -> Result<Vec<NodeAdapterReceiptRef>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, "adapters", 1)?;
    let items = required_sequence(&record[0], "node adapter receipt refs")?;
    ensure_count_at_most(items.len(), MAX_NODE_ADAPTERS, "node adapter receipt refs")?;
    let mut adapters = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let fields = simple_record(&item, "adapter", 2)?;
        let name = required_string(&fields[0], "adapter name")?;
        validate_adapter_name(&name)?;
        adapters.push(NodeAdapterReceiptRef {
            name,
            receipt_ref: required_ref(&fields[1], "adapter receipt ref")?,
        });
    }
    Ok(adapters)
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

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        refs.push(required_ref(item, label)?);
    }
    Ok(refs)
}

fn record_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    let mut strings = Vec::with_capacity(items.len());
    for item in items.iter() {
        strings.push(required_string(item, label)?);
    }
    Ok(strings)
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

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn checks_value(checks: &[(&str, &str)]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "checks")?;
    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("node runtime check {name} has status {status}")));
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

fn validate_adapter_name(name: &str) -> Result<()> {
    validate_non_empty(name, "node adapter name")?;
    if name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "node adapter name {name} must use ascii alphanumeric, '-', or '_'"
        )))
    }
}

fn validate_adapter_operation(operation: &str) -> Result<()> {
    if matches!(operation, "start" | "verify" | "deny" | "shutdown") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported node adapter operation {operation}")))
    }
}

fn validate_control_operation(operation: &str) -> Result<()> {
    if matches!(operation, "status" | "install" | "run" | "gate" | "shutdown") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported node control operation {operation}")))
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported node runtime decision {decision}")))
    }
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    crate::preserves_rail::validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical blake3 content ref: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn ensure_count_at_most(actual: usize, maximum: usize, field: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{field} count {actual} exceeds bound {maximum}")))
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, field: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{field} count overflow")))?;
    ensure_count_at_most(total, maximum, field)?;
    values.push_item(value);
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

fn status(ok: bool) -> &'static str {
    if ok { "pass" } else { "fail" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("node-runtime-test-ref", vec![string(label)])).expect("test ref")
    }

    fn clean_source_gate() -> (String, IOValue) {
        let value = octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean octet gate fixture");
        let reference = canonical_hash(&value).expect("octet gate ref");
        (reference, value)
    }

    fn required_adapter_bindings_scrambled() -> Vec<NodeAdapterBinding> {
        [
            "jobs",
            "ledger",
            "catalog-mcp",
            "control",
            "cache",
            "coordination",
            "registry",
            "remote-dataspace",
            "plugin-host",
            "chunks",
            "services",
            "storage",
        ]
        .iter()
        .map(|name| node_adapter_binding(name, &test_ref(&format!("{name}-profile"))).expect("adapter"))
        .collect()
    }

    fn test_node_config_value(adapters: &[NodeAdapterBinding]) -> IOValue {
        let node_identity_ref = test_ref("node-id");
        let state_root_ref = test_ref("state-root");
        let policy_refs = vec![test_ref("policy")];
        let capability_refs = vec![test_ref("capability")];
        let resource_refs = vec![test_ref("resource")];
        let effect_profile_refs = vec![test_ref("effects")];
        node_config_value(&ConfigValueInput {
            node_identity_ref: &node_identity_ref,
            state_root_ref: &state_root_ref,
            adapters,
            policy_refs: &policy_refs,
            capability_refs: &capability_refs,
            resource_refs: &resource_refs,
            effect_profile_refs: &effect_profile_refs,
        })
        .expect("node config")
    }

    #[test]
    fn node_config_requires_explicit_state_and_adapters() {
        let adapters = vec![node_adapter_binding("ledger", &test_ref("ledger-profile")).expect("adapter")];
        let value = test_node_config_value(&adapters);
        let config = parse_node_config(&value).expect("parse config");
        assert_eq!(config.adapters[0].name, "ledger");
        assert_eq!(crate::ledger::artifact_kind(&value), "node-config");
        let node_identity_ref = test_ref("node-id");
        let state_root_ref = test_ref("state-root");
        assert!(
            node_config_value(&ConfigValueInput {
                node_identity_ref: &node_identity_ref,
                state_root_ref: "./state",
                adapters: &adapters,
                policy_refs: &[],
                capability_refs: &[],
                resource_refs: &[],
                effect_profile_refs: &[],
            })
            .is_err()
        );
        assert!(
            node_config_value(&ConfigValueInput {
                node_identity_ref: &node_identity_ref,
                state_root_ref: &state_root_ref,
                adapters: &[],
                policy_refs: &[],
                capability_refs: &[],
                resource_refs: &[],
                effect_profile_refs: &[],
            })
            .is_err()
        );
    }

    #[test]
    fn startup_receipt_binds_config_identity_and_adapter_receipts() {
        let adapters = vec![
            node_adapter_binding("ledger", &test_ref("ledger-profile")).expect("ledger"),
            node_adapter_binding("registry", &test_ref("registry-profile")).expect("registry"),
        ];
        let config_value = test_node_config_value(&adapters);
        let config = parse_node_config(&config_value).expect("parse config");
        let adapter_receipts = adapters
            .iter()
            .map(|adapter| {
                let receipt = node_adapter_receipt_value("start", "pass", adapter, &[]).expect("adapter receipt");
                NodeAdapterReceiptRef {
                    name: adapter.name.clone(),
                    receipt_ref: canonical_hash(&receipt).expect("adapter receipt ref"),
                }
            })
            .collect::<Vec<_>>();
        let identity_receipt_ref = test_ref("identity-receipt");
        let source_gate_receipt_refs = vec![test_ref("octet-gate-receipt")];
        let source_gate_validation_refs = vec![test_ref("octet-source-gate-validation")];
        let capability_receipt_refs = vec![test_ref("capability-receipt")];
        let resource_receipt_refs = vec![test_ref("resource-receipt")];
        let version_refs = vec![test_ref("version")];
        let receipt_value = node_startup_receipt_value(&StartupReceiptValueInput {
            decision: "pass",
            config: &config,
            identity_receipt_ref: &identity_receipt_ref,
            adapter_receipts: &adapter_receipts,
            source_gate_receipt_refs: &source_gate_receipt_refs,
            source_gate_validation_refs: &source_gate_validation_refs,
            capability_receipt_refs: &capability_receipt_refs,
            resource_receipt_refs: &resource_receipt_refs,
            version_refs: &version_refs,
            diagnostics: &[],
        })
        .expect("startup receipt");
        let receipt = parse_node_startup_receipt(&receipt_value).expect("parse startup");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.config_ref, config.config_ref);
        assert_eq!(receipt.source_gate_receipt_refs, vec![test_ref("octet-gate-receipt")]);
        assert_eq!(receipt.source_gate_validation_refs, vec![test_ref("octet-source-gate-validation")]);
        assert_eq!(crate::ledger::artifact_kind(&receipt_value), "node-startup-receipt");
    }

    #[test]
    fn adapter_lifecycle_receipts_cover_verify_deny_and_shutdown_decisions() {
        let adapter = node_adapter_binding("ledger", &test_ref("ledger-profile")).expect("adapter");
        for (operation, decision) in [("verify", "pass"), ("deny", "deny"), ("shutdown", "pass")] {
            let index_refs = vec![test_ref("index")];
            let resource_refs = vec![test_ref("resource")];
            let receipt = node_adapter_lifecycle_receipt_value(&AdapterLifecycleReceiptInput {
                operation,
                decision,
                adapter: &adapter,
                index_receipt_refs: &index_refs,
                resource_receipt_refs: &resource_refs,
                diagnostics: &[],
            })
            .expect("adapter lifecycle receipt");
            let text = crate::preserves_rail::to_text(&receipt).expect("receipt text");
            assert!(text.contains(operation));
            assert!(text.contains(decision));
            assert_eq!(crate::ledger::artifact_kind(&receipt), "node-adapter-receipt");
        }
    }

    #[test]
    fn control_request_and_receipt_bind_authority_resource_and_subreceipts() {
        let target_ref = test_ref("target");
        let payload_ref = test_ref("payload");
        let authority_refs = vec![test_ref("authority")];
        let policy_refs = vec![test_ref("policy")];
        let resource_refs = vec![test_ref("resource")];
        let request_value = node_control_request_value(&ControlRequestValueInput {
            operation: "install",
            target_ref: Some(&target_ref),
            payload_ref: Some(&payload_ref),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("control request");
        let request = parse_node_control_request(&request_value).expect("parse control request");
        let startup_ref = test_ref("startup");
        let authority_receipt_refs = vec![test_ref("authority-receipt")];
        let resource_receipt_refs = vec![test_ref("resource-receipt")];
        let subreceipt_refs = vec![test_ref("artifact-install-receipt")];
        let receipt_value = node_control_receipt_value(&ControlReceiptValueInput {
            decision: "pass",
            request: &request,
            startup_receipt_ref: &startup_ref,
            authority_receipt_refs: &authority_receipt_refs,
            resource_receipt_refs: &resource_receipt_refs,
            subreceipt_refs: &subreceipt_refs,
            diagnostics: &[],
        })
        .expect("control receipt");
        let receipt = parse_node_control_receipt(&receipt_value).expect("parse control receipt");

        assert_eq!(request.operation, "install");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.request_ref, request.request_ref);
        assert_eq!(crate::ledger::artifact_kind(&request_value), "node-control-request");
        assert_eq!(crate::ledger::artifact_kind(&receipt_value), "node-control-receipt");
    }

    #[test]
    fn control_request_rejects_short_fixture_ref_shape() {
        let authority_refs = vec![test_ref("authority")];
        let policy_refs = vec![test_ref("policy")];
        let resource_refs = vec![test_ref("resource")];
        let error = node_control_request_value(&ControlRequestValueInput {
            operation: "install",
            target_ref: Some("blake3:target-fixture"),
            payload_ref: Some("blake3:payload-fixture"),
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect_err("short fixture refs are rejected");

        assert!(error.to_string().contains("canonical blake3 content ref"));
    }

    #[test]
    fn control_denial_is_canonical_when_authority_or_resource_evidence_is_missing() {
        let payload_ref = test_ref("payload");
        let request_value = node_control_request_value(&ControlRequestValueInput {
            operation: "gate",
            target_ref: None,
            payload_ref: Some(&payload_ref),
            authority_refs: &[],
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("request");
        let request = parse_node_control_request(&request_value).expect("parse request");
        let receipt_value =
            node_control_deny_receipt_value(&request, &test_ref("startup"), "missing authority/resource evidence")
                .expect("deny receipt");
        let receipt = parse_node_control_receipt(&receipt_value).expect("parse receipt");
        let text = crate::preserves_rail::to_text(&receipt_value).expect("receipt text");

        assert_eq!(receipt.decision, "deny");
        assert!(text.contains("authority-gated"));
        assert!(text.contains("resource-gated"));
        assert!(text.contains("missing authority/resource evidence"));
    }

    #[test]
    fn shutdown_receipt_binds_drain_index_and_adapter_close_evidence() {
        let adapter = NodeAdapterReceiptRef {
            name: "ledger".to_string(),
            receipt_ref: test_ref("ledger-shutdown"),
        };
        let drained = vec![test_ref("job-drained")];
        let index = vec![test_ref("index-persisted")];
        let startup_ref = test_ref("startup");
        let receipt_value = node_shutdown_receipt_value(&ShutdownReceiptValueInput {
            decision: "pass",
            startup_receipt_ref: &startup_ref,
            adapter_receipts: std::slice::from_ref(&adapter),
            drained_job_refs: &drained,
            index_receipt_refs: &index,
            diagnostics: &[],
        })
        .expect("shutdown receipt");
        let receipt = parse_node_shutdown_receipt(&receipt_value).expect("parse shutdown receipt");
        let text = crate::preserves_rail::to_text(&receipt_value).expect("shutdown text");

        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.adapters, vec![adapter]);
        assert_eq!(receipt.drained_job_refs, drained);
        assert_eq!(receipt.index_receipt_refs, index);
        assert!(text.contains("graceful-shutdown"));
        assert_eq!(crate::ledger::artifact_kind(&receipt_value), "node-shutdown-receipt");
    }

    #[test]
    fn restart_health_receipt_requires_shutdown_indexes_and_no_open_jobs() {
        let adapters = required_adapter_bindings_scrambled();
        let config_value = test_node_config_value(&adapters);
        let (source_gate_ref, source_gate_value) = clean_source_gate();
        let started = start_node_runtime(&NodeRuntimeStartInput {
            config_value,
            identity_receipt_ref: test_ref("identity-receipt"),
            index_receipt_refs: vec![test_ref("startup-index")],
            source_gate_receipt_refs: vec![source_gate_ref],
            source_gate_receipt_values: vec![source_gate_value],
            capability_receipt_refs: vec![test_ref("capability-receipt")],
            resource_receipt_refs: vec![test_ref("resource-receipt")],
            version_refs: vec![test_ref("version")],
        })
        .expect("start runtime");
        let shutdown_index_refs = vec![test_ref("shutdown-index")];
        let shutdown = node_shutdown_receipt_value(&ShutdownReceiptValueInput {
            decision: "pass",
            startup_receipt_ref: &started.startup_receipt.receipt_ref,
            adapter_receipts: &started.adapter_receipts,
            drained_job_refs: &[],
            index_receipt_refs: &shutdown_index_refs,
            diagnostics: &[],
        })
        .expect("shutdown");
        let shutdown = parse_node_shutdown_receipt(&shutdown).expect("parse shutdown");
        let restart_index_refs = vec![test_ref("restart-index")];
        let head_refs = vec![test_ref("chain-head")];
        let healthy = node_restart_health_receipt_value(&RestartHealthReceiptValueInput {
            startup_receipt: &started.startup_receipt,
            shutdown_receipt_ref: Some(&shutdown.receipt_ref),
            index_receipt_refs: &restart_index_refs,
            head_refs: &head_refs,
            open_job_refs: &[],
            diagnostics: &[],
        })
        .expect("healthy restart");
        let health = parse_node_health_receipt(&healthy).expect("parse health");
        assert_eq!(health.decision, "pass");
        assert_eq!(health.replay_status, "eligible");
        assert_eq!(crate::ledger::artifact_kind(&healthy), "node-health-receipt");

        let unhealthy_head_refs = vec![test_ref("chain-head")];
        let open_job_refs = vec![test_ref("open-job")];
        let unhealthy = node_restart_health_receipt_value(&RestartHealthReceiptValueInput {
            startup_receipt: &started.startup_receipt,
            shutdown_receipt_ref: None,
            index_receipt_refs: &[],
            head_refs: &unhealthy_head_refs,
            open_job_refs: &open_job_refs,
            diagnostics: &[],
        })
        .expect("unhealthy restart");
        let unhealthy = parse_node_health_receipt(&unhealthy).expect("parse unhealthy");
        assert_eq!(unhealthy.decision, "deny");
        assert_eq!(unhealthy.replay_status, "ineligible");
        assert!(
            unhealthy
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("previous shutdown receipt missing"))
        );
        assert!(unhealthy.diagnostics.iter().any(|diagnostic| diagnostic.contains("adapter indexes not verified")));
        assert!(unhealthy.diagnostics.iter().any(|diagnostic| diagnostic.contains("open jobs")));
    }

    #[test]
    fn runtime_start_orders_adapters_and_binds_index_and_resource_receipts() {
        let adapters = required_adapter_bindings_scrambled();
        let config_value = test_node_config_value(&adapters);
        let index_ref = test_ref("index-verify");
        let resource_ref = test_ref("resource-profile");
        let (source_gate_ref, source_gate_value) = clean_source_gate();
        let started = start_node_runtime(&NodeRuntimeStartInput {
            config_value,
            identity_receipt_ref: test_ref("identity-receipt"),
            index_receipt_refs: vec![index_ref.clone()],
            source_gate_receipt_refs: vec![source_gate_ref],
            source_gate_receipt_values: vec![source_gate_value],
            capability_receipt_refs: vec![test_ref("capability-receipt")],
            resource_receipt_refs: vec![resource_ref.clone()],
            version_refs: vec![test_ref("version")],
        })
        .expect("start runtime");

        assert_eq!(started.decision, "pass");
        assert_eq!(
            started.adapter_receipts.iter().map(|receipt| receipt.name.as_str()).collect::<Vec<_>>(),
            REQUIRED_RUNTIME_ADAPTERS
        );
        let adapter_text = crate::preserves_rail::to_text(&started.adapter_receipt_values[0]).expect("adapter text");
        assert!(adapter_text.contains(&index_ref));
        assert!(adapter_text.contains(&resource_ref));
        assert_eq!(started.startup_receipt.decision, "pass");
    }

    #[test]
    fn runtime_start_denies_missing_index_resource_or_required_adapters() {
        let adapters = vec![node_adapter_binding("ledger", &test_ref("ledger-profile")).expect("ledger")];
        let config_value = test_node_config_value(&adapters);
        let started = start_node_runtime(&NodeRuntimeStartInput {
            config_value,
            identity_receipt_ref: test_ref("identity-receipt"),
            index_receipt_refs: Vec::new(),
            source_gate_receipt_refs: Vec::new(),
            source_gate_receipt_values: Vec::new(),
            capability_receipt_refs: vec![test_ref("capability-receipt")],
            resource_receipt_refs: Vec::new(),
            version_refs: vec![test_ref("version")],
        })
        .expect("deny runtime start");

        assert_eq!(started.decision, "deny");
        assert!(
            started
                .startup_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("missing required node runtime adapters"))
        );
        assert!(
            started
                .startup_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("index verification"))
        );
        assert!(
            started
                .startup_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("strict Octet source gate"))
        );
        assert!(started.startup_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource profile")));
    }
}
