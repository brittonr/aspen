type IoValue = preserves::IOValue;
type Value<T> = preserves::Value<T>;

type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type RuntimeEvent = crate::runtime::RuntimeEvent;
type RuntimeState = crate::runtime::RuntimeState;
type RuntimeStep = crate::runtime::RuntimeStep;
type RuntimeValue = crate::runtime::RuntimeValue;

const ADMISSION_RECEIPT_SCHEMA: &str = crate::preserves_rail::REMOTE_DATASPACE_ADMISSION_RECEIPT_SCHEMA;
const DELIVERY_LOG_SCHEMA: &str = crate::preserves_rail::REMOTE_DATASPACE_DELIVERY_LOG_SCHEMA;
const ENVELOPE_SCHEMA: &str = crate::preserves_rail::REMOTE_DATASPACE_ENVELOPE_SCHEMA;
const GATE_RECEIPT_SCHEMA: &str = crate::preserves_rail::REMOTE_DATASPACE_GATE_RECEIPT_SCHEMA;
const TRANSPORT_RECEIPT_SCHEMA: &str = crate::preserves_rail::REMOTE_DATASPACE_TRANSPORT_RECEIPT_SCHEMA;

mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    #[cfg(test)]
    pub(super) fn remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    pub(super) fn write(path: impl AsRef<std::path::Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
}

fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
}

fn content_ref_hex(value: &str) -> Result<&str> {
    crate::preserves_rail::content_ref_hex(value)
}

fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    crate::preserves_rail::parse_canonical_bytes(bytes)
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

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const LOCAL_GOSSIP_TRANSPORT: &str = "iroh-local-gossip";
pub const LIVE_GOSSIP_TRANSPORT: &str = "iroh-gossip";

pub type CapabilityDataspaceRoot = crate::local_store::DataspaceStoreRoot;

pub fn open_capability_dataspace_root(root: &Path) -> Result<CapabilityDataspaceRoot> {
    crate::local_store::DataspaceStoreRoot::open(root)
}

const MAX_REPLAY_EVENTS: usize = 4_096;
const _: () = assert!(MAX_REPLAY_EVENTS > 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Message,
    Assert,
    Retract,
    Observe,
}

impl Operation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Assert => "assert",
            Self::Retract => "retract",
            Self::Observe => "observe",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "message" => Ok(Self::Message),
            "assert" => Ok(Self::Assert),
            "retract" => Ok(Self::Retract),
            "observe" => Ok(Self::Observe),
            _ => Err(MoltenError::invalid_harness(format!("unsupported remote dataspace operation {value}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub envelope_ref: String,
    pub from_peer: String,
    pub from_actor: String,
    pub to_peer: String,
    pub topic: String,
    pub operation: Operation,
    pub payload: IoValue,
    pub content_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub sequence: u64,
    pub operation_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange {
    pub envelope_ref: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    pub envelope: Envelope,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeliveryEvidence {
    pub peer_bootstrap_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub authority_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub events: Vec<RuntimeEvent>,
    pub admission_receipt_value: IoValue,
    pub turn_journal_context_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotentApplied {
    pub events: Vec<RuntimeEvent>,
    pub admission_receipt_value: IoValue,
    pub turn_journal_context_ref: String,
    pub idempotency_receipt_value: IoValue,
    pub operation_ref: String,
    pub prior_semantic_result_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryLog {
    pub log_ref: String,
    pub replayable: bool,
    pub entries: Vec<Delivery>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwoPeerHarness {
    pub delivery_log: DeliveryLog,
    pub admission_receipt_value: IoValue,
    pub receipt_value: IoValue,
    pub observed_events: Vec<RuntimeEvent>,
    pub replayed_events: Vec<RuntimeEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeInput {
    pub from_peer: String,
    pub from_actor: String,
    pub to_peer: String,
    pub topic: String,
    pub operation: Operation,
    pub payload: IoValue,
    pub content_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub struct AssertEnvelopeInput<'a> {
    pub from_peer: &'a str,
    pub from_actor: &'a str,
    pub to_peer: &'a str,
    pub topic: &'a str,
    pub payload: IoValue,
    pub capability_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub struct LocalTransportReceiptInput<'a> {
    pub operation: &'a str,
    pub decision: &'a str,
    pub node: &'a str,
    pub envelope: &'a Envelope,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
}

pub struct TransportReceiptInput<'a> {
    pub transport: &'a str,
    pub operation: &'a str,
    pub decision: &'a str,
    pub node: &'a str,
    pub envelope: &'a Envelope,
    pub diagnostics: Vec<String>,
    pub checks: Vec<(String, String)>,
}

struct AdmissionReceiptInput<'a> {
    decision: &'a str,
    envelope: &'a Envelope,
    transport_receipt_ref: &'a str,
    evidence: &'a DeliveryEvidence,
    turn_context_refs: &'a [String],
    diagnostics: Vec<String>,
}

pub fn build_envelope(input: EnvelopeInput) -> Result<Envelope> {
    validate_name(&input.from_peer, "from peer")?;
    validate_name(&input.from_actor, "from actor")?;
    validate_name(&input.to_peer, "to peer")?;
    validate_name(&input.topic, "topic")?;
    validate_refs(&input.content_refs, "content ref")?;
    validate_refs(&input.capability_refs, "capability ref")?;
    validate_refs(&input.evidence_refs, "evidence ref")?;
    let value = envelope_value(&input)?;
    parse_envelope(&value)
}

pub fn assert_envelope(input: AssertEnvelopeInput<'_>) -> Result<Envelope> {
    build_envelope(EnvelopeInput {
        from_peer: input.from_peer.to_owned(),
        from_actor: input.from_actor.to_owned(),
        to_peer: input.to_peer.to_owned(),
        topic: input.topic.to_owned(),
        operation: Operation::Assert,
        payload: input.payload,
        content_refs: Vec::new(),
        capability_refs: input.capability_refs,
        evidence_refs: input.evidence_refs,
    })
}
