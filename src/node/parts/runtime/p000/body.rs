type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Record<T> = preserves::Record<T>;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

const NODE_ADAPTER_RECEIPT_SCHEMA: &str = crate::preserves_rail::NODE_ADAPTER_RECEIPT_SCHEMA;
const NODE_CONFIG_SCHEMA: &str = crate::preserves_rail::NODE_CONFIG_SCHEMA;
const NODE_CONTROL_RECEIPT_SCHEMA: &str = crate::preserves_rail::NODE_CONTROL_RECEIPT_SCHEMA;
const NODE_CONTROL_REQUEST_SCHEMA: &str = crate::preserves_rail::NODE_CONTROL_REQUEST_SCHEMA;
const NODE_STARTUP_RECEIPT_SCHEMA: &str = crate::preserves_rail::NODE_STARTUP_RECEIPT_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

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
    pub config_value: IoValue,
    pub identity_receipt_ref: String,
    pub index_receipt_refs: Vec<String>,
    pub source_gate_receipt_refs: Vec<String>,
    pub source_gate_receipt_values: Vec<IoValue>,
    pub profile_metadata_refs: Vec<String>,
    pub capability_receipt_refs: Vec<String>,
    pub resource_receipt_refs: Vec<String>,
    pub version_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigValueInput<'a> {
    pub identity_ref: &'a str,
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
    pub profile_metadata_refs: &'a [String],
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
    pub request: &'a ControlRequest,
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
    pub adapter_receipt_values: Vec<IoValue>,
    pub adapter_receipts: Vec<NodeAdapterReceiptRef>,
    pub startup_receipt: NodeStartupReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRequest {
    pub request_ref: String,
    pub operation: String,
    pub target_ref: Option<String>,
    pub payload_ref: Option<String>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub request_ref: String,
    pub startup_receipt_ref: String,
    pub authority_receipt_refs: Vec<String>,
    pub resource_receipt_refs: Vec<String>,
    pub subreceipt_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
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
    pub value: IoValue,
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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeConfig {
    pub config_ref: String,
    pub identity_ref: String,
    pub state_root_ref: String,
    pub adapters: Vec<NodeAdapterBinding>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_profile_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
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
    pub profile_metadata_refs: Vec<String>,
    pub capability_receipt_refs: Vec<String>,
    pub resource_receipt_refs: Vec<String>,
    pub version_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

pub fn node_adapter_binding(name: &str, profile_ref: &str) -> Result<NodeAdapterBinding> {
    validate_adapter_name(name)?;
    validate_ref(profile_ref, "node adapter profile ref")?;
    Ok(NodeAdapterBinding {
        name: name.to_string(),
        profile_ref: profile_ref.to_string(),
    })
}

pub fn node_config_value(input: &ConfigValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.identity_ref, "node config identity ref")?;
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
        record("node-id", vec![string(input.identity_ref)]),
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
