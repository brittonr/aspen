type IoValue = preserves::IOValue;

type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Envelope = crate::remote_dataspace::Envelope;
type EnvelopeInput = crate::remote_dataspace::EnvelopeInput;
type Operation = crate::remote_dataspace::Operation;

const PROTOCOL_ENDPOINT_SCHEMA: &str = crate::preserves_rail::PROTOCOL_ENDPOINT_SCHEMA;
const PROTOCOL_INSTALL_RECEIPT_SCHEMA: &str = crate::preserves_rail::PROTOCOL_INSTALL_RECEIPT_SCHEMA;
const PROTOCOL_LOCAL_STATE_SCHEMA: &str = crate::preserves_rail::PROTOCOL_LOCAL_STATE_SCHEMA;
const PROTOCOL_MANIFEST_SCHEMA: &str = crate::preserves_rail::PROTOCOL_MANIFEST_SCHEMA;
const PROTOCOL_MESSAGE_SCHEMA: &str = crate::preserves_rail::PROTOCOL_MESSAGE_SCHEMA;
const PROTOCOL_OPERATION_RECEIPT_SCHEMA: &str = crate::preserves_rail::PROTOCOL_OPERATION_RECEIPT_SCHEMA;
const PROTOCOL_SESSION_GATE_RECEIPT_SCHEMA: &str = crate::preserves_rail::PROTOCOL_SESSION_GATE_RECEIPT_SCHEMA;
const PROTOCOL_SESSION_STATE_SCHEMA: &str = crate::preserves_rail::PROTOCOL_SESSION_STATE_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: &str) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

fn build_remote_envelope(input: EnvelopeInput) -> Result<Envelope> {
    crate::remote_dataspace::build_envelope(input)
}

const MAX_PROTOCOL_ITEMS: usize = 1024;
const MAX_PROTOCOL_STEPS: usize = 256;

const _: () = assert!(MAX_PROTOCOL_ITEMS > 0);
const _: () = assert!(MAX_PROTOCOL_STEPS > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolPayloadInput {
    pub tag: String,
    pub schema_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolManifestInput {
    pub protocol_id: String,
    pub roles: Vec<String>,
    pub labels: Vec<String>,
    pub payloads: Vec<ProtocolPayloadInput>,
    pub global: IoValue,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolCommInput {
    pub from_role: String,
    pub to_role: String,
    pub label: String,
    pub payload_tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolBranchInput {
    pub label: String,
    pub steps: Vec<ProtocolCommInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolChoiceInput {
    pub decider: String,
    pub branches: Vec<ProtocolBranchInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolPayload {
    pub tag: String,
    pub schema_ref: String,
    pub payload_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolGlobal {
    Script(Vec<ProtocolCommInput>),
    Choice(ProtocolChoiceInput),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolManifest {
    pub manifest_ref: String,
    pub protocol_id: String,
    pub roles: Vec<String>,
    pub labels: Vec<String>,
    pub payloads: Vec<ProtocolPayload>,
    pub global: ProtocolGlobal,
    pub global_value: IoValue,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub name: String,
    pub id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolRegistries {
    pub roles: Vec<RegistryEntry>,
    pub labels: Vec<RegistryEntry>,
    pub payloads: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolLocalAction {
    pub direction: String,
    pub peer: String,
    pub label: String,
    pub payload_tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolLocalBranch {
    pub label: String,
    pub actions: Vec<ProtocolLocalAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolLocalTerminal {
    End,
    InternalChoice(Vec<ProtocolLocalBranch>),
    Offer {
        from_role: String,
        branches: Vec<ProtocolLocalBranch>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolLocalState {
    pub actions: Vec<ProtocolLocalAction>,
    pub terminal: ProtocolLocalTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolEndpoint {
    pub endpoint_ref: String,
    pub protocol_ref: String,
    pub role: String,
    pub role_id: u32,
    pub local_state: ProtocolLocalState,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolInstallReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub manifest: ProtocolManifest,
    pub registries: ProtocolRegistries,
    pub endpoints: Vec<ProtocolEndpoint>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionStateInput {
    pub protocol_ref: String,
    pub session_id: String,
    pub role: String,
    pub sequence: u64,
    pub endpoint: IoValue,
    pub local_state: IoValue,
    pub seen_message_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionState {
    pub state_ref: String,
    pub protocol_ref: String,
    pub session_id: String,
    pub role: String,
    pub sequence: u64,
    pub endpoint: ProtocolEndpoint,
    pub local_state: ProtocolLocalState,
    pub seen_message_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolMessageInput {
    pub protocol_ref: String,
    pub session_id: String,
    pub from_role: String,
    pub to_role: String,
    pub label: String,
    pub payload_tag: String,
    pub body_or_ref: IoValue,
    pub sequence: u64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolMessage {
    pub message_ref: String,
    pub protocol_ref: String,
    pub session_id: String,
    pub from_role: String,
    pub to_role: String,
    pub label: String,
    pub payload_tag: String,
    pub body_or_ref: IoValue,
    pub sequence: u64,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolOperationReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub protocol_ref: String,
    pub session_id: String,
    pub role: String,
    pub prior_state_ref: String,
    pub message_ref: Option<String>,
    pub next_state_ref: Option<String>,
    pub sequence: u64,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub carrier_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionGateInput {
    pub install_receipt: IoValue,
    pub initial_states: Vec<IoValue>,
    pub operation_receipts: Vec<IoValue>,
    pub messages: Vec<IoValue>,
    pub next_states: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionGate {
    pub receipt_ref: String,
    pub decision: String,
    pub install_ref: String,
    pub protocol_ref: String,
    pub session_ids: Vec<String>,
    pub initial_state_count: usize,
    pub operation_count: usize,
    pub message_count: usize,
    pub final_state_count: usize,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSessionGateReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub install_ref: String,
    pub protocol_ref: String,
    pub session_ids: Vec<String>,
    pub initial_state_refs: Vec<String>,
    pub operation_refs: Vec<String>,
    pub message_refs: Vec<String>,
    pub final_state_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}
