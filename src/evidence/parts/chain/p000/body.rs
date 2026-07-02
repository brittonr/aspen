type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;
type IoValue = preserves::IOValue;
type Path = std::path::Path;

type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type SignReceiptInput<'a> = crate::evidence::SignReceiptInput<'a>;
type SignedReceipt = crate::evidence::SignedReceipt;

const EVIDENCE_CHAIN_ANCHOR_SCHEMA: &str = crate::preserves_rail::EVIDENCE_CHAIN_ANCHOR_SCHEMA;
const EVIDENCE_CHAIN_APPEND_RECEIPT_SCHEMA: &str = crate::preserves_rail::EVIDENCE_CHAIN_APPEND_RECEIPT_SCHEMA;
const EVIDENCE_CHAIN_CHECKPOINT_SCHEMA: &str = crate::preserves_rail::EVIDENCE_CHAIN_CHECKPOINT_SCHEMA;
const EVIDENCE_CHAIN_FORK_EVIDENCE_SCHEMA: &str = crate::preserves_rail::EVIDENCE_CHAIN_FORK_EVIDENCE_SCHEMA;
const EVIDENCE_CHAIN_LINK_SCHEMA: &str = crate::preserves_rail::EVIDENCE_CHAIN_LINK_SCHEMA;
const EVIDENCE_CHAIN_PREDICATE_RECEIPT_SCHEMA: &str = crate::preserves_rail::EVIDENCE_CHAIN_PREDICATE_RECEIPT_SCHEMA;
const EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA: &str = crate::preserves_rail::EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA;
const EVIDENCE_SIGNED_RECEIPT_SCHEMA: &str = crate::preserves_rail::EVIDENCE_SIGNED_RECEIPT_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn sign_receipt(input: &SignReceiptInput<'_>) -> Result<IoValue> {
    crate::evidence::sign_receipt(input)
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

fn verify_signed_receipt(
    value: &IoValue,
    required_purpose: &str,
    trust_root: &str,
    key: &str,
) -> Result<SignedReceipt> {
    crate::evidence::verify_signed_receipt(value, required_purpose, trust_root, key)
}

pub const GENESIS_VALID_PREDICATE: &str = "molten.chain.genesis_valid.v1";
pub const APPEND_VALID_PREDICATE: &str = "molten.chain.append_valid.v1";
pub const SEGMENT_NO_GAP_PREDICATE: &str = "molten.chain.segment_no_gap.v1";
pub const SEGMENT_NO_FORK_PREDICATE: &str = "molten.chain.segment_no_fork.v1";
pub const DESCENDS_FROM_ANCHOR_PREDICATE: &str = "molten.chain.descends_from_anchor.v1";
pub const CHECKPOINT_COVERS_RANGE_PREDICATE: &str = "molten.chain.checkpoint_covers_range.v1";
pub const CHAIN_EVIDENCE_PURPOSE: &str = "chain-evidence";

const MAX_EVIDENCE_CHAIN_LINKS: usize = 100_000;

const _: () = assert!(MAX_EVIDENCE_CHAIN_LINKS <= 1_000_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainForkPolicy {
    RejectUnexpectedForks,
    RetainForkEvidence,
}

impl ChainForkPolicy {
    pub fn profile(self) -> &'static str {
        match self {
            Self::RejectUnexpectedForks => "reject-unexpected-forks",
            Self::RetainForkEvidence => "retain-fork-evidence",
        }
    }

    pub fn decision_for_fork(self) -> &'static str {
        match self {
            Self::RejectUnexpectedForks => "reject",
            Self::RetainForkEvidence => "retain",
        }
    }

    fn fork_diagnostics_are_fatal(self) -> bool {
        matches!(self, Self::RejectUnexpectedForks)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainScope {
    pub scope: String,
    pub id: String,
    pub epoch: String,
}

impl ChainScope {
    pub fn new(scope: impl Into<String>, id: impl Into<String>, epoch: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            id: id.into(),
            epoch: epoch.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainPayload {
    pub kind: String,
    pub artifact_ref: String,
    pub schema: String,
}

impl ChainPayload {
    pub fn new(kind: impl Into<String>, artifact_ref: impl Into<String>, schema: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            artifact_ref: artifact_ref.into(),
            schema: schema.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainContextRef {
    pub label: String,
    pub artifact_ref: String,
}

impl ChainContextRef {
    pub fn new(label: impl Into<String>, artifact_ref: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            artifact_ref: artifact_ref.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainProducer {
    pub id: String,
    pub key_ref: String,
}

impl ChainProducer {
    pub fn new(id: impl Into<String>, key_ref: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            key_ref: key_ref.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainTrellisEvidence {
    pub predicate: String,
    pub input_ref: String,
    pub decision: String,
}

impl ChainTrellisEvidence {
    pub fn pass(predicate: impl Into<String>, input_ref: impl Into<String>) -> Self {
        Self {
            predicate: predicate.into(),
            input_ref: input_ref.into(),
            decision: "pass".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainCheck {
    pub name: String,
    pub decision: String,
}

impl ChainCheck {
    pub fn pass(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            decision: "pass".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainLinkInput {
    pub chain: ChainScope,
    pub sequence: u64,
    pub previous_link_ref: Option<String>,
    pub payload: ChainPayload,
    pub context_refs: Vec<ChainContextRef>,
    pub producer: ChainProducer,
    pub trellis: ChainTrellisEvidence,
    pub checks: Vec<ChainCheck>,
}

impl ChainLinkInput {
    pub fn genesis(
        chain: ChainScope,
        payload: ChainPayload,
        context_refs: Vec<ChainContextRef>,
        producer: ChainProducer,
        trellis_input_ref: impl Into<String>,
    ) -> Self {
        Self {
            chain,
            sequence: 0,
            previous_link_ref: None,
            payload,
            context_refs,
            producer,
            trellis: ChainTrellisEvidence::pass(GENESIS_VALID_PREDICATE, trellis_input_ref),
            checks: genesis_checks(),
        }
    }

    pub fn append(
        previous: &ChainLink,
        payload: ChainPayload,
        context_refs: Vec<ChainContextRef>,
        producer: ChainProducer,
        trellis_input_ref: impl Into<String>,
    ) -> Self {
        Self {
            chain: previous.chain.clone(),
            sequence: previous.sequence.saturating_add(1),
            previous_link_ref: Some(previous.link_ref.clone()),
            payload,
            context_refs,
            producer,
            trellis: ChainTrellisEvidence::pass(APPEND_VALID_PREDICATE, trellis_input_ref),
            checks: append_checks(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainLink {
    pub link_ref: String,
    pub chain: ChainScope,
    pub sequence: u64,
    pub previous_link_ref: Option<String>,
    pub payload: ChainPayload,
    pub context_refs: Vec<ChainContextRef>,
    pub producer: ChainProducer,
    pub trellis: ChainTrellisEvidence,
    pub checks: Vec<ChainCheck>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChainIndex {
    pub links_by_ref: OrderedMap<String, ChainLink>,
    pub links_by_chain: OrderedMap<ChainScope, OrderedSet<String>>,
    pub children_by_parent: OrderedMap<String, OrderedSet<String>>,
    pub links_by_sequence: OrderedMap<(ChainScope, u64), OrderedSet<String>>,
    pub links_by_payload: OrderedMap<String, OrderedSet<String>>,
    pub heads_by_chain: OrderedMap<ChainScope, OrderedSet<String>>,
    pub predicate_receipts_by_ref: OrderedMap<String, ChainPredicateReceipt>,
    pub predicate_receipts_by_predicate: OrderedMap<String, OrderedSet<String>>,
    pub fork_evidence_by_ref: OrderedMap<String, ChainForkEvidence>,
    pub fork_evidence_by_chain: OrderedMap<ChainScope, OrderedSet<String>>,
    pub fork_evidence_by_parent: OrderedMap<String, OrderedSet<String>>,
    pub anchors_by_ref: OrderedMap<String, ChainAnchor>,
    pub anchors_by_chain: OrderedMap<ChainScope, OrderedSet<String>>,
    pub anchor_links_by_chain: OrderedMap<ChainScope, OrderedSet<String>>,
    pub checkpoints_by_ref: OrderedMap<String, ChainCheckpoint>,
    pub checkpoints_by_chain: OrderedMap<ChainScope, OrderedSet<String>>,
    pub checkpoint_heads_by_chain: OrderedMap<ChainScope, OrderedSet<String>>,
}
