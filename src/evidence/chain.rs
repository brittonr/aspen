//! Scoped hash-chain evidence continuity.
//!
//! Chain links in this module are local evidence-continuity records. They are
//! deliberately scoped by `(scope, id, epoch)` and do not provide a global
//! actor-message order, fork-choice protocol, cryptocurrency ledger, or ambient
//! authority. A link's identity is only the Blake3 hash of its canonical
//! Preserves bytes; linking names payload refs without rewriting the payloads.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use preserves::IOValue;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;
use crate::evidence::SignReceiptInput;
use crate::evidence::SignedReceipt;
use crate::evidence::sign_receipt;
use crate::evidence::verify_signed_receipt;
use crate::ledger;
use crate::preserves_rail::EVIDENCE_CHAIN_LINK_SCHEMA;
use crate::preserves_rail::EVIDENCE_CHAIN_PREDICATE_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;

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
    pub links_by_ref: BTreeMap<String, ChainLink>,
    pub links_by_chain: BTreeMap<ChainScope, BTreeSet<String>>,
    pub children_by_parent: BTreeMap<String, BTreeSet<String>>,
    pub links_by_sequence: BTreeMap<(ChainScope, u64), BTreeSet<String>>,
    pub links_by_payload: BTreeMap<String, BTreeSet<String>>,
    pub heads_by_chain: BTreeMap<ChainScope, BTreeSet<String>>,
    pub predicate_receipts_by_ref: BTreeMap<String, ChainPredicateReceipt>,
    pub predicate_receipts_by_predicate: BTreeMap<String, BTreeSet<String>>,
    pub fork_evidence_by_ref: BTreeMap<String, ChainForkEvidence>,
    pub fork_evidence_by_chain: BTreeMap<ChainScope, BTreeSet<String>>,
    pub fork_evidence_by_parent: BTreeMap<String, BTreeSet<String>>,
    pub anchors_by_ref: BTreeMap<String, ChainAnchor>,
    pub anchors_by_chain: BTreeMap<ChainScope, BTreeSet<String>>,
    pub anchor_links_by_chain: BTreeMap<ChainScope, BTreeSet<String>>,
    pub checkpoints_by_ref: BTreeMap<String, ChainCheckpoint>,
    pub checkpoints_by_chain: BTreeMap<ChainScope, BTreeSet<String>>,
    pub checkpoint_heads_by_chain: BTreeMap<ChainScope, BTreeSet<String>>,
}

impl ChainIndex {
    pub fn links_for_chain(&self, chain: &ChainScope) -> Vec<String> {
        sorted_refs(self.links_by_chain.get(chain))
    }

    pub fn heads_for_chain(&self, chain: &ChainScope) -> Vec<String> {
        sorted_refs(self.heads_by_chain.get(chain))
    }

    pub fn children_for_parent(&self, parent_ref: &str) -> Vec<String> {
        sorted_refs(self.children_by_parent.get(parent_ref))
    }

    pub fn links_for_payload(&self, payload_ref: &str) -> Vec<String> {
        sorted_refs(self.links_by_payload.get(payload_ref))
    }

    pub fn links_for_sequence(&self, chain: &ChainScope, sequence: u64) -> Vec<String> {
        sorted_refs(self.links_by_sequence.get(&(chain.clone(), sequence)))
    }

    pub fn predicate_receipts_for_predicate(&self, predicate: &str) -> Vec<String> {
        sorted_refs(self.predicate_receipts_by_predicate.get(predicate))
    }

    pub fn fork_evidence_for_chain(&self, chain: &ChainScope) -> Vec<String> {
        sorted_refs(self.fork_evidence_by_chain.get(chain))
    }

    pub fn fork_evidence_for_parent(&self, parent_ref: &str) -> Vec<String> {
        sorted_refs(self.fork_evidence_by_parent.get(parent_ref))
    }

    pub fn anchors_for_chain(&self, chain: &ChainScope) -> Vec<String> {
        sorted_refs(self.anchors_by_chain.get(chain))
    }

    pub fn anchor_links_for_chain(&self, chain: &ChainScope) -> Vec<String> {
        sorted_refs(self.anchor_links_by_chain.get(chain))
    }

    pub fn checkpoints_for_chain(&self, chain: &ChainScope) -> Vec<String> {
        sorted_refs(self.checkpoints_by_chain.get(chain))
    }

    pub fn checkpoint_heads_for_chain(&self, chain: &ChainScope) -> Vec<String> {
        sorted_refs(self.checkpoint_heads_by_chain.get(chain))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainPredicateReceipt {
    pub receipt_ref: String,
    pub predicate: String,
    pub decision: String,
    pub subject_refs: Vec<String>,
    pub input_refs: Vec<String>,
    pub context_refs: Vec<String>,
    pub checks: Vec<ChainCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainForkEvidence {
    pub evidence_ref: String,
    pub chain: ChainScope,
    pub parent_ref: Option<String>,
    pub child_refs: Vec<String>,
    pub selected_head: Option<String>,
    pub profile: String,
    pub decision: String,
    pub checks: Vec<ChainCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainAnchor {
    pub anchor_ref: String,
    pub chain: ChainScope,
    pub link_ref: String,
    pub policy_refs: Vec<String>,
    pub producer: ChainProducer,
    pub checks: Vec<ChainCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainCheckpointInput {
    pub chain: ChainScope,
    pub prior_checkpoint_ref: Option<String>,
    pub anchor_link_ref: String,
    pub head_ref: String,
    pub verify_receipt_ref: String,
    pub range_predicate_ref: String,
    pub policy_refs: Vec<String>,
    pub membership_refs: Vec<String>,
    pub producer: ChainProducer,
    pub checks: Vec<ChainCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainCheckpoint {
    pub checkpoint_ref: String,
    pub chain: ChainScope,
    pub prior_checkpoint_ref: Option<String>,
    pub anchor_link_ref: String,
    pub head_ref: String,
    pub verify_receipt_ref: String,
    pub range_predicate_ref: String,
    pub policy_refs: Vec<String>,
    pub membership_refs: Vec<String>,
    pub producer: ChainProducer,
    pub checks: Vec<ChainCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainAppend {
    pub link_ref: String,
    pub payload_ref: String,
    pub head_before: Option<String>,
    pub head_after: String,
    pub predicate_receipt_ref: String,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainDiagnostic {
    pub kind: String,
    pub detail: String,
    pub refs: Vec<String>,
}

impl ChainDiagnostic {
    pub fn new(kind: impl Into<String>, detail: impl Into<String>, refs: Vec<String>) -> Self {
        Self {
            kind: kind.into(),
            detail: detail.into(),
            refs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainVerify {
    pub decision: String,
    pub chain: ChainScope,
    pub anchor_ref: Option<String>,
    pub expected_head: Option<String>,
    pub discovered_heads: Vec<String>,
    pub verified_links: Vec<String>,
    pub payload_refs: Vec<String>,
    pub diagnostics: Vec<ChainDiagnostic>,
    pub predicate_receipt_refs: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ChainVerifyReceiptValueInput<'a> {
    pub decision: &'a str,
    pub chain: &'a ChainScope,
    pub anchor_ref: Option<&'a str>,
    pub expected_head: Option<&'a str>,
    pub discovered_heads: &'a [String],
    pub verified_links: &'a [String],
    pub payload_refs: &'a [String],
    pub diagnostics: &'a [ChainDiagnostic],
}

#[derive(Debug, Clone, Copy)]
pub struct ChainVerifyReceiptPolicyValueInput<'a> {
    pub receipt: ChainVerifyReceiptValueInput<'a>,
    pub predicate_receipt_refs: &'a [String],
    pub fork_policy: ChainForkPolicy,
}

#[derive(Debug, Clone, Copy)]
pub struct ChainPredicateReceiptValueInput<'a> {
    pub predicate: &'a str,
    pub decision: &'a str,
    pub subject_refs: &'a [String],
    pub input_refs: &'a [String],
    pub context_refs: &'a [String],
    pub checks: &'a [ChainCheck],
}

#[derive(Debug, Clone, Copy)]
pub struct ChainForkEvidenceValueInput<'a> {
    pub chain: &'a ChainScope,
    pub parent_ref: Option<&'a str>,
    pub child_refs: &'a [String],
    pub selected_head: Option<&'a str>,
    pub fork_policy: ChainForkPolicy,
}

struct CheckpointVerifyReceiptValidationInput<'a> {
    root: &'a Path,
    value: &'a IOValue,
    chain: &'a ChainScope,
    anchor_link_ref: &'a str,
    head_ref: &'a str,
    range_predicate_ref: &'a str,
}

struct RangeBindingInput<'a> {
    root: &'a Path,
    value: &'a IOValue,
    chain: &'a ChainScope,
    anchor_link_ref: &'a str,
    head_ref: &'a str,
    range_predicate_ref: &'a str,
}

struct ForkDetectionInput<'a> {
    root: &'a Path,
    index: &'a ChainIndex,
    chain: &'a ChainScope,
    chain_refs: &'a [String],
    discovered_heads: &'a [String],
    selected_head: Option<&'a str>,
    fork_policy: ChainForkPolicy,
    diagnostics: &'a mut Vec<ChainDiagnostic>,
}

struct StoreChainForkEvidenceInput<'a> {
    root: &'a Path,
    chain: &'a ChainScope,
    parent_ref: Option<&'a str>,
    child_refs: &'a [String],
    selected_head: Option<&'a str>,
    fork_policy: ChainForkPolicy,
}

struct StoreChainPredicateReceiptInput<'a> {
    root: &'a Path,
    receipt: ChainPredicateReceiptValueInput<'a>,
}

struct SegmentPredicateReceiptsInput<'a> {
    root: &'a Path,
    decision: &'a str,
    chain: &'a ChainScope,
    anchor_ref: Option<&'a str>,
    expected_head: Option<&'a str>,
    discovered_heads: &'a [String],
    verified_links: &'a [String],
    payload_refs: &'a [String],
    diagnostics: &'a [ChainDiagnostic],
    fork_policy: ChainForkPolicy,
}

#[derive(Debug, Clone, Copy)]
struct LinkWalkInput<'a> {
    root: &'a Path,
    index: &'a ChainIndex,
    chain: &'a ChainScope,
    anchor_ref: Option<&'a str>,
    head_ref: &'a str,
}

#[derive(Debug, Default)]
struct WalkState {
    reverse_segment: Vec<String>,
    payload_refs: BTreeSet<String>,
    seen_refs: BTreeSet<String>,
}

struct WalkOutput {
    verified_links: Vec<String>,
    payload_refs: Vec<String>,
}

struct FinishInput<'a> {
    root: &'a Path,
    decision: String,
    chain: &'a ChainScope,
    anchor_ref: Option<&'a str>,
    expected_head: Option<&'a str>,
    discovered_heads: Vec<String>,
    verified_links: Vec<String>,
    payload_refs: Vec<String>,
    diagnostics: Vec<ChainDiagnostic>,
    predicate_receipt_refs: Vec<String>,
    fork_policy: ChainForkPolicy,
}

pub fn genesis_checks() -> Vec<ChainCheck> {
    vec![
        ChainCheck::pass("genesis-sequence"),
        ChainCheck::pass("no-previous-link"),
        ChainCheck::pass("payload-ref-binding"),
        ChainCheck::pass("scoped-chain-not-global-order"),
    ]
}

pub fn append_checks() -> Vec<ChainCheck> {
    vec![
        ChainCheck::pass("same-chain-scope"),
        ChainCheck::pass("previous-link-binding"),
        ChainCheck::pass("sequence-monotonicity"),
        ChainCheck::pass("payload-ref-binding"),
    ]
}

pub fn chain_link_value(input: &ChainLinkInput) -> IOValue {
    let context_refs = input
        .context_refs
        .iter()
        .map(|context| record("ref", vec![string(&context.label), string(&context.artifact_ref)]))
        .collect();
    let checks = input
        .checks
        .iter()
        .map(|check| record("check", vec![string(&check.name), string(&check.decision)]))
        .collect();

    record("chain-link-v1", vec![
        string(EVIDENCE_CHAIN_LINK_SCHEMA),
        record("chain", vec![
            record("scope", vec![string(&input.chain.scope)]),
            record("id", vec![string(&input.chain.id)]),
            record("epoch", vec![string(&input.chain.epoch)]),
        ]),
        record("seq", vec![u64_value(input.sequence)]),
        record("prev", vec![optional_ref_value(input.previous_link_ref.as_deref())]),
        record("payload", vec![
            record("kind", vec![string(&input.payload.kind)]),
            record("ref", vec![string(&input.payload.artifact_ref)]),
            record("schema", vec![string(&input.payload.schema)]),
        ]),
        record("context", vec![sequence(context_refs)]),
        record("producer", vec![
            record("id", vec![string(&input.producer.id)]),
            record("key", vec![string(&input.producer.key_ref)]),
        ]),
        record("trellis", vec![
            record("predicate", vec![string(&input.trellis.predicate)]),
            record("input", vec![string(&input.trellis.input_ref)]),
            record("decision", vec![string(&input.trellis.decision)]),
        ]),
        record("checks", vec![sequence(checks)]),
    ])
}

pub fn chain_link_ref(value: &IOValue) -> Result<String> {
    canonical_hash(value)
}

fn ensure_entry_ref(kind: &str, entry_ref: &str, parsed_ref: &str) -> Result<()> {
    if parsed_ref == entry_ref {
        return Ok(());
    }

    Err(MoltenError::invalid_harness(format!(
        "ledger {kind} ref mismatch: index entry {entry_ref} parsed as {parsed_ref}"
    )))
}

fn index_link_entry(index: &mut ChainIndex, entry_ref: &str, value: &IOValue) -> Result<()> {
    let link = parse_chain_link(value)?;
    ensure_entry_ref("chain-link", entry_ref, &link.link_ref)?;
    index.links_by_chain.entry(link.chain.clone()).or_default().insert(link.link_ref.clone());
    index
        .links_by_sequence
        .entry((link.chain.clone(), link.sequence))
        .or_default()
        .insert(link.link_ref.clone());
    index
        .links_by_payload
        .entry(link.payload.artifact_ref.clone())
        .or_default()
        .insert(link.link_ref.clone());
    if let Some(parent_ref) = &link.previous_link_ref {
        index.children_by_parent.entry(parent_ref.clone()).or_default().insert(link.link_ref.clone());
    }
    index.links_by_ref.insert(link.link_ref.clone(), link);
    Ok(())
}

fn index_predicate_entry(index: &mut ChainIndex, entry_ref: &str, value: &IOValue) -> Result<()> {
    let receipt = parse_chain_predicate_receipt(value)?;
    ensure_entry_ref("chain-predicate-receipt", entry_ref, &receipt.receipt_ref)?;
    index
        .predicate_receipts_by_predicate
        .entry(receipt.predicate.clone())
        .or_default()
        .insert(receipt.receipt_ref.clone());
    index.predicate_receipts_by_ref.insert(receipt.receipt_ref.clone(), receipt);
    Ok(())
}

fn index_fork_entry(index: &mut ChainIndex, entry_ref: &str, value: &IOValue) -> Result<()> {
    let fork = parse_chain_fork_evidence(value)?;
    ensure_entry_ref("chain-fork-evidence", entry_ref, &fork.evidence_ref)?;
    index
        .fork_evidence_by_chain
        .entry(fork.chain.clone())
        .or_default()
        .insert(fork.evidence_ref.clone());
    if let Some(parent_ref) = &fork.parent_ref {
        index
            .fork_evidence_by_parent
            .entry(parent_ref.clone())
            .or_default()
            .insert(fork.evidence_ref.clone());
    }
    index.fork_evidence_by_ref.insert(fork.evidence_ref.clone(), fork);
    Ok(())
}

fn index_anchor_entry(index: &mut ChainIndex, entry_ref: &str, value: &IOValue) -> Result<()> {
    let anchor = parse_chain_anchor(value)?;
    ensure_entry_ref("chain-anchor", entry_ref, &anchor.anchor_ref)?;
    index.anchors_by_chain.entry(anchor.chain.clone()).or_default().insert(anchor.anchor_ref.clone());
    index.anchor_links_by_chain.entry(anchor.chain.clone()).or_default().insert(anchor.link_ref.clone());
    index.anchors_by_ref.insert(anchor.anchor_ref.clone(), anchor);
    Ok(())
}

fn index_checkpoint_entry(index: &mut ChainIndex, entry_ref: &str, value: &IOValue) -> Result<()> {
    let checkpoint = parse_chain_checkpoint(value)?;
    ensure_entry_ref("chain-checkpoint", entry_ref, &checkpoint.checkpoint_ref)?;
    index
        .checkpoints_by_chain
        .entry(checkpoint.chain.clone())
        .or_default()
        .insert(checkpoint.checkpoint_ref.clone());
    index
        .checkpoint_heads_by_chain
        .entry(checkpoint.chain.clone())
        .or_default()
        .insert(checkpoint.head_ref.clone());
    index.checkpoints_by_ref.insert(checkpoint.checkpoint_ref.clone(), checkpoint);
    Ok(())
}

fn finish_heads(index: &mut ChainIndex) -> Result<()> {
    for (chain, links) in &index.links_by_chain {
        let mut heads = links.clone();
        for link_ref in links {
            let Some(link) = index.links_by_ref.get(link_ref) else {
                return Err(MoltenError::invalid_harness(format!("chain index missing link {link_ref}")));
            };
            if let Some(parent_ref) = &link.previous_link_ref {
                heads.remove(parent_ref);
            }
        }
        index.heads_by_chain.insert(chain.clone(), heads);
    }
    Ok(())
}

pub fn build_chain_index(root: &Path) -> Result<ChainIndex> {
    let mut index = ChainIndex::default();
    for entry in ledger::list_artifacts(root)? {
        let value = ledger::read_artifact(root, &entry.artifact_ref)?;
        match entry.artifact_kind.as_str() {
            "chain-link" => index_link_entry(&mut index, &entry.artifact_ref, &value)?,
            "chain-predicate-receipt" => index_predicate_entry(&mut index, &entry.artifact_ref, &value)?,
            "chain-fork-evidence" => index_fork_entry(&mut index, &entry.artifact_ref, &value)?,
            "chain-anchor" => index_anchor_entry(&mut index, &entry.artifact_ref, &value)?,
            "chain-checkpoint" => index_checkpoint_entry(&mut index, &entry.artifact_ref, &value)?,
            _ => {}
        }
    }

    finish_heads(&mut index)?;
    Ok(index)
}

fn prior_head(index: &ChainIndex, link: &ChainLink) -> Result<Option<String>> {
    if let Some(previous_ref) = &link.previous_link_ref {
        let previous = index.links_by_ref.get(previous_ref).ok_or_else(|| {
            MoltenError::invalid_harness(format!("append previous link {previous_ref} is unavailable in ledger"))
        })?;
        validate_append(previous, link)?;
        let children = index.children_for_parent(previous_ref);
        if !children.is_empty() {
            return Err(MoltenError::invalid_harness(format!(
                "unexpected fork for parent {previous_ref}: existing children {:?}",
                children
            )));
        }
        let current_heads = index.heads_for_chain(&link.chain);
        if current_heads != vec![previous_ref.clone()] {
            return Err(MoltenError::invalid_harness(format!(
                "stale chain head for {:?}: expected current head {}, found {:?}",
                link.chain, previous_ref, current_heads
            )));
        }
        ensure_sequence_unoccupied(index, link)?;
        return Ok(Some(previous_ref.clone()));
    }

    validate_genesis(link)?;
    let current_heads = index.heads_for_chain(&link.chain);
    if !current_heads.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "genesis append has stale chain head for {:?}: existing heads {:?}",
            link.chain, current_heads
        )));
    }
    ensure_sequence_unoccupied(index, link)?;
    Ok(None)
}

fn append_predicate_ref(root: &Path, link: &ChainLink, head_before: Option<&str>) -> Result<String> {
    let predicate = if head_before.is_some() {
        APPEND_VALID_PREDICATE
    } else {
        GENESIS_VALID_PREDICATE
    };
    let predicate_subject_refs = vec![link.link_ref.clone()];
    let predicate_input_refs = predicate_input_refs(link);
    let predicate_context_refs = predicate_context_refs(link);
    let predicate_checks = vec![
        ChainCheck::pass("trellis-bounded-predicate"),
        ChainCheck::pass("predicate-decision-binding"),
    ];
    store_chain_predicate_receipt(StoreChainPredicateReceiptInput {
        root,
        receipt: ChainPredicateReceiptValueInput {
            predicate,
            decision: "pass",
            subject_refs: &predicate_subject_refs,
            input_refs: &predicate_input_refs,
            context_refs: &predicate_context_refs,
            checks: &predicate_checks,
        },
    })
}

pub fn append_chain_link(root: &Path, value: &IOValue) -> Result<ChainAppend> {
    let link = parse_chain_link(value)?;
    let index = build_chain_index(root)?;
    if index.links_by_ref.contains_key(&link.link_ref) {
        return Err(MoltenError::invalid_harness(format!(
            "chain link {} is already present in the ledger",
            link.link_ref
        )));
    }
    ledger::read_artifact(root, &link.payload.artifact_ref).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "chain link payload {} is unavailable in ledger: {error}",
            link.payload.artifact_ref
        ))
    })?;

    let head_before = prior_head(&index, &link)?;
    let imported = ledger::import_artifact(root, value)?;
    if imported.artifact_ref != link.link_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported chain link ref mismatch: got {}, expected {}",
            imported.artifact_ref, link.link_ref
        )));
    }

    let predicate_receipt_ref = append_predicate_ref(root, &link, head_before.as_deref())?;
    let receipt_value = chain_append_receipt_value(&link, head_before.as_deref(), &predicate_receipt_ref);
    let receipt_ref = canonical_hash(&receipt_value)?;
    let imported_receipt = ledger::import_artifact(root, &receipt_value)?;
    if imported_receipt.artifact_ref != receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported chain append receipt ref mismatch: got {}, expected {receipt_ref}",
            imported_receipt.artifact_ref
        )));
    }

    Ok(ChainAppend {
        link_ref: link.link_ref.clone(),
        payload_ref: link.payload.artifact_ref.clone(),
        head_before,
        head_after: link.link_ref,
        predicate_receipt_ref,
        receipt_ref,
        receipt_value,
    })
}

pub fn chain_append_receipt_value(link: &ChainLink, head_before: Option<&str>, predicate_receipt_ref: &str) -> IOValue {
    let mut checks = vec![
        record("check", vec![string("canonical-link-ref"), string("pass")]),
        record("check", vec![string("payload-ref-available"), string("pass")]),
        record("check", vec![string("immutable-ledger-artifact"), string("pass")]),
        record("check", vec![string("head-advance"), string("pass")]),
        record("check", vec![string("no-unexpected-fork"), string("pass")]),
    ];
    if head_before.is_some() {
        checks.push(record("check", vec![string("append-valid"), string("pass")]));
    } else {
        checks.push(record("check", vec![string("genesis-valid"), string("pass")]));
    }

    record("chain-append-receipt-v1", vec![
        string(crate::preserves_rail::EVIDENCE_CHAIN_APPEND_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        chain_record(&link.chain),
        record("link", vec![string(&link.link_ref)]),
        record("payload", vec![string(&link.payload.artifact_ref)]),
        record("head-before", vec![optional_ref_value(head_before)]),
        record("head-after", vec![string(&link.link_ref)]),
        record("predicates", vec![ref_sequence_value(&[predicate_receipt_ref.to_string()])]),
        record("checks", vec![sequence(checks)]),
    ])
}

pub fn chain_anchor_value(
    chain: &ChainScope,
    link_ref: &str,
    policy_refs: &[String],
    producer: &ChainProducer,
) -> IOValue {
    record("chain-anchor-v1", vec![
        string(crate::preserves_rail::EVIDENCE_CHAIN_ANCHOR_SCHEMA),
        chain_record(chain),
        record("anchor", vec![string(link_ref)]),
        record("policy", vec![ref_sequence_value(policy_refs)]),
        producer_record(producer),
        record("checks", vec![sequence(vec![
            record("check", vec![string("trusted-anchor"), string("pass")]),
            record("check", vec![string("anchor-link-available"), string("pass")]),
        ])]),
    ])
}

pub fn publish_chain_anchor(
    root: &Path,
    chain: &ChainScope,
    link_ref: &str,
    policy_refs: &[String],
    producer: &ChainProducer,
) -> Result<ChainAnchor> {
    let link_value = ledger::read_artifact(root, link_ref)?;
    let link = parse_chain_link(&link_value)?;
    if &link.chain != chain {
        return Err(MoltenError::invalid_harness(format!(
            "anchor link {link_ref} belongs to {:?}, expected {:?}",
            link.chain, chain
        )));
    }
    for policy_ref in policy_refs {
        require_ref(policy_ref, "anchor policy ref")?;
    }
    validate_producer(producer)?;
    let value = chain_anchor_value(chain, link_ref, policy_refs, producer);
    let imported = ledger::import_artifact(root, &value)?;
    parse_chain_anchor(&ledger::read_artifact(root, &imported.artifact_ref)?)
}

pub fn chain_checkpoint_value(input: &ChainCheckpointInput) -> IOValue {
    record("chain-checkpoint-v1", vec![
        string(crate::preserves_rail::EVIDENCE_CHAIN_CHECKPOINT_SCHEMA),
        chain_record(&input.chain),
        record("prior-checkpoint", vec![optional_ref_value(input.prior_checkpoint_ref.as_deref())]),
        record("range", vec![
            record("anchor", vec![string(&input.anchor_link_ref)]),
            record("head", vec![string(&input.head_ref)]),
            record("verify-receipt", vec![string(&input.verify_receipt_ref)]),
            record("predicate", vec![string(&input.range_predicate_ref)]),
        ]),
        record("policy", vec![ref_sequence_value(&input.policy_refs)]),
        record("membership", vec![ref_sequence_value(&input.membership_refs)]),
        record("control-plane", vec![
            record("mode", vec![string("trellis-raft")]),
            record("command", vec![string("accept-chain-head")]),
        ]),
        producer_record(&input.producer),
        record("checks", vec![sequence(input.checks.iter().map(check_value).collect())]),
    ])
}

pub fn accept_chain_checkpoint(root: &Path, input: &ChainCheckpointInput) -> Result<ChainCheckpoint> {
    validate_checkpoint_input(root, input)?;
    let value = chain_checkpoint_value(input);
    let imported = ledger::import_artifact(root, &value)?;
    parse_chain_checkpoint(&ledger::read_artifact(root, &imported.artifact_ref)?)
}

pub fn validate_chain_checkpoint_freshness(
    root: &Path,
    chain: &ChainScope,
    checkpoint_ref: &str,
    expected_head: Option<&str>,
) -> Result<ChainCheckpoint> {
    let value = ledger::read_artifact(root, checkpoint_ref)?;
    let checkpoint = parse_chain_checkpoint(&value)?;
    if &checkpoint.chain != chain {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint {checkpoint_ref} belongs to {:?}, expected {:?}",
            checkpoint.chain, chain
        )));
    }
    if let Some(expected_head) = expected_head
        && checkpoint.head_ref != expected_head
    {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint head {} does not match expected head {expected_head}",
            checkpoint.head_ref
        )));
    }
    let index = build_chain_index(root)?;
    let heads = index.heads_for_chain(chain);
    if heads != vec![checkpoint.head_ref.clone()] {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint {checkpoint_ref} is stale: checkpoint head {}, current heads {:?}",
            checkpoint.head_ref, heads
        )));
    }
    Ok(checkpoint)
}

pub fn verify_chain_segment(
    root: &Path,
    chain: &ChainScope,
    anchor_ref: Option<&str>,
    expected_head: Option<&str>,
) -> Result<ChainVerify> {
    verify_chain_segment_with_policy(root, chain, anchor_ref, expected_head, ChainForkPolicy::RejectUnexpectedForks)
}

pub fn verify_chain_segment_with_policy(
    root: &Path,
    chain: &ChainScope,
    anchor_ref: Option<&str>,
    expected_head: Option<&str>,
    fork_policy: ChainForkPolicy,
) -> Result<ChainVerify> {
    let index = build_chain_index(root)?;
    let chain_refs = index.links_for_chain(chain);
    let discovered_heads = index.heads_for_chain(chain);
    let mut diagnostics = Vec::new();

    if chain_refs.is_empty() {
        diagnostics.push(ChainDiagnostic::new("missing-chain", "no links found for requested chain", Vec::new()));
    }

    note_anchor(&index, chain, anchor_ref, &mut diagnostics);

    let selected_head = select_head_for_verification(expected_head, &discovered_heads, &index, chain, &mut diagnostics);
    detect_forks_sequence_conflicts_and_store_evidence(ForkDetectionInput {
        root,
        index: &index,
        chain,
        chain_refs: &chain_refs,
        discovered_heads: &discovered_heads,
        selected_head: selected_head.as_deref(),
        fork_policy,
        diagnostics: &mut diagnostics,
    })?;

    let (verified_links, payload_refs) = if let Some(head_ref) = selected_head {
        let walked = walk_links(
            LinkWalkInput {
                root,
                index: &index,
                chain,
                anchor_ref,
                head_ref: &head_ref,
            },
            &mut diagnostics,
        )?;
        (walked.verified_links, walked.payload_refs)
    } else {
        (Vec::new(), Vec::new())
    };
    let decision = if diagnostics.iter().any(|diagnostic| diagnostic_is_fatal(diagnostic, fork_policy)) {
        "fail"
    } else {
        "pass"
    }
    .to_string();
    let predicate_receipt_refs = store_segment_predicate_receipts(SegmentPredicateReceiptsInput {
        root,
        decision: &decision,
        chain,
        anchor_ref,
        expected_head,
        discovered_heads: &discovered_heads,
        verified_links: &verified_links,
        payload_refs: &payload_refs,
        diagnostics: &diagnostics,
        fork_policy,
    })?;
    finish_verify(FinishInput {
        root,
        decision,
        chain,
        anchor_ref,
        expected_head,
        discovered_heads,
        verified_links,
        payload_refs,
        diagnostics,
        predicate_receipt_refs,
        fork_policy,
    })
}

fn note_anchor(
    index: &ChainIndex,
    chain: &ChainScope,
    anchor_ref: Option<&str>,
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) {
    if let Some(anchor_ref) = anchor_ref {
        match index.links_by_ref.get(anchor_ref) {
            Some(anchor) if &anchor.chain == chain => {}
            Some(_) => diagnostics.push_item(ChainDiagnostic::new(
                "anchor-chain-mismatch",
                "anchor link belongs to a different chain scope/id/epoch",
                vec![anchor_ref.to_string()],
            )),
            None => diagnostics.push_item(ChainDiagnostic::new(
                "missing-anchor",
                "anchor link is unavailable in the ledger",
                vec![anchor_ref.to_string()],
            )),
        }
    }
}

fn walk_links(
    input: LinkWalkInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) -> Result<WalkOutput> {
    ensure_count_at_most(input.index.links_by_ref.len(), MAX_EVIDENCE_CHAIN_LINKS, "evidence chain links")?;
    let mut state = WalkState::default();
    let mut current_ref = input.head_ref.to_string();
    for _ in 0..=input.index.links_by_ref.len() {
        let Some(previous_ref) = walk_next(input, &current_ref, &mut state, diagnostics)? else {
            break;
        };
        current_ref = previous_ref;
    }
    if state.reverse_segment.len() > input.index.links_by_ref.len() {
        diagnostics.push_item(ChainDiagnostic::new(
            "chain-bound",
            "chain segment traversal exceeded available link count",
            vec![input.head_ref.to_string()],
        ));
    }
    let verified_links = state.reverse_segment.into_iter().rev().collect::<Vec<_>>();
    validate_verified_segment(input.index, input.anchor_ref, &verified_links, diagnostics);
    Ok(WalkOutput {
        verified_links,
        payload_refs: state.payload_refs.into_iter().collect(),
    })
}

fn walk_next(
    input: LinkWalkInput<'_>,
    current_ref: &str,
    state: &mut WalkState,
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) -> Result<Option<String>> {
    if !state.seen_refs.insert(current_ref.to_string()) {
        diagnostics.push_item(ChainDiagnostic::new("cycle", "chain segment contains a repeated link ref", vec![
            current_ref.to_string(),
        ]));
        return Ok(None);
    }
    let Some(link) = input.index.links_by_ref.get(current_ref) else {
        diagnostics.push_item(ChainDiagnostic::new(
            "missing-link",
            "chain segment names an unavailable previous link",
            vec![current_ref.to_string()],
        ));
        return Ok(None);
    };
    if &link.chain != input.chain {
        diagnostics.push_item(ChainDiagnostic::new(
            "link-chain-mismatch",
            "chain segment crossed into a different scope/id/epoch",
            vec![link.link_ref.clone()],
        ));
        return Ok(None);
    }
    note_payload(input.root, link, state, diagnostics);
    push_bounded(
        &mut state.reverse_segment,
        link.link_ref.clone(),
        MAX_EVIDENCE_CHAIN_LINKS,
        "evidence chain verified links",
    )?;
    if input.anchor_ref == Some(link.link_ref.as_str()) {
        return Ok(None);
    }
    if let Some(previous_ref) = &link.previous_link_ref {
        return Ok(Some(previous_ref.clone()));
    }
    if let Some(anchor_ref) = input.anchor_ref {
        diagnostics.push_item(ChainDiagnostic::new(
            "anchor-descent",
            "head does not descend from requested anchor",
            vec![anchor_ref.to_string(), link.link_ref.clone()],
        ));
    }
    Ok(None)
}

fn note_payload(
    root: &Path,
    link: &ChainLink,
    state: &mut WalkState,
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) {
    if let Err(error) = ledger::read_artifact(root, &link.payload.artifact_ref) {
        diagnostics.push_item(ChainDiagnostic::new(
            "missing-payload",
            format!("payload ref is unavailable in the ledger: {error}"),
            vec![link.payload.artifact_ref.clone(), link.link_ref.clone()],
        ));
    } else {
        state.payload_refs.insert(link.payload.artifact_ref.clone());
    }
}

fn finish_verify(input: FinishInput<'_>) -> Result<ChainVerify> {
    let receipt_value = finish_value(&input);
    let receipt_ref = canonical_hash(&receipt_value)?;
    let imported_receipt = ledger::import_artifact(input.root, &receipt_value)?;
    if imported_receipt.artifact_ref != receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported chain verify receipt ref mismatch: got {}, expected {receipt_ref}",
            imported_receipt.artifact_ref
        )));
    }

    Ok(ChainVerify {
        decision: input.decision,
        chain: input.chain.clone(),
        anchor_ref: input.anchor_ref.map(ToOwned::to_owned),
        expected_head: input.expected_head.map(ToOwned::to_owned),
        discovered_heads: input.discovered_heads,
        verified_links: input.verified_links,
        payload_refs: input.payload_refs,
        diagnostics: input.diagnostics,
        predicate_receipt_refs: input.predicate_receipt_refs,
        receipt_ref,
        receipt_value,
    })
}

fn finish_value(input: &FinishInput<'_>) -> IOValue {
    let receipt = ChainVerifyReceiptValueInput {
        decision: &input.decision,
        chain: input.chain,
        anchor_ref: input.anchor_ref,
        expected_head: input.expected_head,
        discovered_heads: &input.discovered_heads,
        verified_links: &input.verified_links,
        payload_refs: &input.payload_refs,
        diagnostics: &input.diagnostics,
    };
    chain_verify_receipt_value_with_policy(&ChainVerifyReceiptPolicyValueInput {
        receipt,
        predicate_receipt_refs: &input.predicate_receipt_refs,
        fork_policy: input.fork_policy,
    })
}

pub fn chain_verify_receipt_value(input: &ChainVerifyReceiptValueInput<'_>) -> IOValue {
    chain_verify_receipt_value_with_policy(&ChainVerifyReceiptPolicyValueInput {
        receipt: *input,
        predicate_receipt_refs: &[],
        fork_policy: ChainForkPolicy::RejectUnexpectedForks,
    })
}

pub fn chain_verify_receipt_value_with_policy(input: &ChainVerifyReceiptPolicyValueInput<'_>) -> IOValue {
    let receipt = input.receipt;
    let checks = vec![
        diagnostic_check("no-gap-segment", receipt.diagnostics, &["gap", "genesis-invalid", "append-invalid", "cycle"]),
        fork_policy_check(receipt.diagnostics, input.fork_policy),
        diagnostic_check("payload-availability", receipt.diagnostics, &["missing-payload"]),
        diagnostic_check("anchor-descent", receipt.diagnostics, &[
            "missing-anchor",
            "anchor-chain-mismatch",
            "anchor-descent",
        ]),
        diagnostic_check("expected-head", receipt.diagnostics, &["missing-head", "head-chain-mismatch", "stale-head"]),
    ];
    record("chain-verify-receipt-v1", vec![
        string(crate::preserves_rail::EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA),
        record("decision", vec![string(receipt.decision)]),
        chain_record(receipt.chain),
        record("anchor", vec![optional_ref_value(receipt.anchor_ref)]),
        record("expected-head", vec![optional_ref_value(receipt.expected_head)]),
        record("discovered-heads", vec![ref_sequence_value(receipt.discovered_heads)]),
        record("verified-links", vec![ref_sequence_value(receipt.verified_links)]),
        record("payloads", vec![ref_sequence_value(receipt.payload_refs)]),
        record("predicates", vec![ref_sequence_value(input.predicate_receipt_refs)]),
        record("diagnostics", vec![sequence(receipt.diagnostics.iter().map(diagnostic_value).collect())]),
        record("checks", vec![sequence(checks)]),
    ])
}

pub fn chain_predicate_receipt_value(input: &ChainPredicateReceiptValueInput<'_>) -> IOValue {
    record("chain-predicate-receipt-v1", vec![
        string(EVIDENCE_CHAIN_PREDICATE_RECEIPT_SCHEMA),
        record("predicate", vec![string(input.predicate)]),
        record("decision", vec![string(input.decision)]),
        record("subjects", vec![ref_sequence_value(input.subject_refs)]),
        record("inputs", vec![ref_sequence_value(input.input_refs)]),
        record("context", vec![ref_sequence_value(input.context_refs)]),
        record("checks", vec![sequence(input.checks.iter().map(check_value).collect())]),
    ])
}

pub fn parse_chain_predicate_receipt(value: &IOValue) -> Result<ChainPredicateReceipt> {
    let receipt = value
        .collect_simple_record("chain-predicate-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-predicate-receipt-v1 ...>"))?;
    require_schema(&receipt[0], EVIDENCE_CHAIN_PREDICATE_RECEIPT_SCHEMA, "chain predicate receipt schema")?;
    let parsed = ChainPredicateReceipt {
        receipt_ref: canonical_hash(value)?,
        predicate: record_string(&receipt[1], "predicate", "chain predicate name")?,
        decision: record_string(&receipt[2], "decision", "chain predicate decision")?,
        subject_refs: record_ref_sequence(&receipt[3], "subjects")?,
        input_refs: record_ref_sequence(&receipt[4], "inputs")?,
        context_refs: record_ref_sequence(&receipt[5], "context")?,
        checks: parse_checks(&receipt[6])?,
    };
    validate_chain_predicate_receipt_shape(&parsed)?;
    Ok(parsed)
}

pub fn chain_fork_evidence_value(input: &ChainForkEvidenceValueInput<'_>) -> IOValue {
    record("chain-fork-evidence-v1", vec![
        string(crate::preserves_rail::EVIDENCE_CHAIN_FORK_EVIDENCE_SCHEMA),
        chain_record(input.chain),
        record("parent", vec![optional_ref_value(input.parent_ref)]),
        record("children", vec![ref_sequence_value(input.child_refs)]),
        record("selected-head", vec![optional_ref_value(input.selected_head)]),
        record("profile", vec![string(input.fork_policy.profile())]),
        record("decision", vec![string(input.fork_policy.decision_for_fork())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("fork-detected"), string("pass")]),
            record("check", vec![string("fork-policy-profile"), string("pass")]),
            record("check", vec![string("diagnostic-retention"), string("pass")]),
        ])]),
    ])
}

pub fn sign_chain_receipt(
    receipt: &IOValue,
    signer: &str,
    trust_root: &str,
    key: &str,
    parents: &[String],
) -> Result<IOValue> {
    sign_receipt(&SignReceiptInput {
        receipt,
        signer,
        purpose: CHAIN_EVIDENCE_PURPOSE,
        trust_root,
        key,
        parents,
    })
}

pub fn verify_signed_chain_receipt(value: &IOValue, trust_root: &str, key: &str) -> Result<SignedReceipt> {
    verify_signed_receipt(value, CHAIN_EVIDENCE_PURPOSE, trust_root, key)
}

pub fn signed_receipt_payload(signed_receipt_ref: impl Into<String>) -> ChainPayload {
    ChainPayload::new(
        "signed-receipt",
        signed_receipt_ref.into(),
        crate::preserves_rail::EVIDENCE_SIGNED_RECEIPT_SCHEMA,
    )
}

pub fn parse_chain_link(value: &IOValue) -> Result<ChainLink> {
    let link = value
        .collect_simple_record("chain-link-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-link-v1 ...>"))?;
    let schema = required_string(&link[0], "chain link schema")?;
    if schema != EVIDENCE_CHAIN_LINK_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chain link schema {schema}; expected {EVIDENCE_CHAIN_LINK_SCHEMA}"
        )));
    }

    let parsed = ChainLink {
        link_ref: chain_link_ref(value)?,
        chain: parse_chain(&link[1])?,
        sequence: record_u64(&link[2], "seq", "chain link sequence")?,
        previous_link_ref: parse_previous_link_ref(&link[3])?,
        payload: parse_payload(&link[4])?,
        context_refs: parse_context_refs(&link[5])?,
        producer: parse_producer(&link[6])?,
        trellis: parse_trellis(&link[7])?,
        checks: parse_checks(&link[8])?,
    };
    validate_chain_link_shape(&parsed)?;
    Ok(parsed)
}

pub fn parse_chain_fork_evidence(value: &IOValue) -> Result<ChainForkEvidence> {
    let fork = value
        .collect_simple_record("chain-fork-evidence-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-fork-evidence-v1 ...>"))?;
    require_schema(&fork[0], crate::preserves_rail::EVIDENCE_CHAIN_FORK_EVIDENCE_SCHEMA, "chain fork evidence schema")?;
    let parsed = ChainForkEvidence {
        evidence_ref: canonical_hash(value)?,
        chain: parse_chain(&fork[1])?,
        parent_ref: record_optional_ref(&fork[2], "parent")?,
        child_refs: record_ref_sequence(&fork[3], "children")?,
        selected_head: record_optional_ref(&fork[4], "selected-head")?,
        profile: record_string(&fork[5], "profile", "fork policy profile")?,
        decision: record_string(&fork[6], "decision", "fork policy decision")?,
        checks: parse_checks(&fork[7])?,
    };
    validate_chain_fork_evidence_shape(&parsed)?;
    Ok(parsed)
}

pub fn parse_chain_anchor(value: &IOValue) -> Result<ChainAnchor> {
    let anchor = value
        .collect_simple_record("chain-anchor-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-anchor-v1 ...>"))?;
    require_schema(&anchor[0], crate::preserves_rail::EVIDENCE_CHAIN_ANCHOR_SCHEMA, "chain anchor schema")?;
    let parsed = ChainAnchor {
        anchor_ref: canonical_hash(value)?,
        chain: parse_chain(&anchor[1])?,
        link_ref: record_string(&anchor[2], "anchor", "anchor link ref")?,
        policy_refs: record_ref_sequence(&anchor[3], "policy")?,
        producer: parse_producer(&anchor[4])?,
        checks: parse_checks(&anchor[5])?,
    };
    validate_chain_anchor_shape(&parsed)?;
    Ok(parsed)
}

pub fn parse_chain_checkpoint(value: &IOValue) -> Result<ChainCheckpoint> {
    let checkpoint = value
        .collect_simple_record("chain-checkpoint-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-checkpoint-v1 ...>"))?;
    require_schema(&checkpoint[0], crate::preserves_rail::EVIDENCE_CHAIN_CHECKPOINT_SCHEMA, "chain checkpoint schema")?;
    let range = parse_checkpoint_range(&checkpoint[3])?;
    let parsed = ChainCheckpoint {
        checkpoint_ref: canonical_hash(value)?,
        chain: parse_chain(&checkpoint[1])?,
        prior_checkpoint_ref: record_optional_ref(&checkpoint[2], "prior-checkpoint")?,
        anchor_link_ref: range.0,
        head_ref: range.1,
        verify_receipt_ref: range.2,
        range_predicate_ref: range.3,
        policy_refs: record_ref_sequence(&checkpoint[4], "policy")?,
        membership_refs: record_ref_sequence(&checkpoint[5], "membership")?,
        producer: parse_producer(&checkpoint[7])?,
        checks: parse_checks(&checkpoint[8])?,
    };
    parse_control_plane(&checkpoint[6])?;
    validate_chain_checkpoint_shape(&parsed)?;
    Ok(parsed)
}

pub fn validate_genesis(link: &ChainLink) -> Result<()> {
    validate_chain_link_shape(link)?;
    if link.sequence != 0 {
        return Err(MoltenError::invalid_harness(format!(
            "genesis chain link sequence must be 0, got {}",
            link.sequence
        )));
    }
    if link.previous_link_ref.is_some() {
        return Err(MoltenError::invalid_harness("genesis chain link must not name a previous link"));
    }
    require_trellis_pass(link, GENESIS_VALID_PREDICATE)?;
    require_pass_check(link, "genesis-sequence")?;
    require_pass_check(link, "no-previous-link")?;
    require_pass_check(link, "payload-ref-binding")?;
    require_pass_check(link, "scoped-chain-not-global-order")
}

pub fn validate_append(previous: &ChainLink, link: &ChainLink) -> Result<()> {
    validate_chain_link_shape(previous)?;
    validate_chain_link_shape(link)?;
    if previous.chain != link.chain {
        return Err(MoltenError::invalid_harness(format!(
            "append link must stay in the same chain scope/id/epoch: previous={:?} next={:?}",
            previous.chain, link.chain
        )));
    }
    let Some(previous_link_ref) = &link.previous_link_ref else {
        return Err(MoltenError::invalid_harness("append chain link must name a previous link"));
    };
    if previous_link_ref != &previous.link_ref {
        return Err(MoltenError::invalid_harness(format!(
            "append previous link ref mismatch: got {previous_link_ref}, expected {}",
            previous.link_ref
        )));
    }
    let expected_sequence = previous.sequence.checked_add(1).ok_or_else(|| {
        MoltenError::invalid_harness(format!("cannot append after max sequence {}", previous.sequence))
    })?;
    if link.sequence != expected_sequence {
        return Err(MoltenError::invalid_harness(format!(
            "append sequence must be previous + 1: got {}, expected {expected_sequence}",
            link.sequence
        )));
    }
    require_trellis_pass(link, APPEND_VALID_PREDICATE)?;
    require_pass_check(link, "same-chain-scope")?;
    require_pass_check(link, "previous-link-binding")?;
    require_pass_check(link, "sequence-monotonicity")?;
    require_pass_check(link, "payload-ref-binding")
}

fn validate_checkpoint_input(root: &Path, input: &ChainCheckpointInput) -> Result<()> {
    validate_chain_scope(&input.chain)?;
    require_ref(&input.anchor_link_ref, "checkpoint anchor link ref")?;
    require_ref(&input.head_ref, "checkpoint head ref")?;
    require_ref(&input.verify_receipt_ref, "checkpoint verify receipt ref")?;
    require_ref(&input.range_predicate_ref, "checkpoint range predicate ref")?;
    if let Some(prior_checkpoint_ref) = &input.prior_checkpoint_ref {
        let prior_value = ledger::read_artifact(root, prior_checkpoint_ref)?;
        let prior = parse_chain_checkpoint(&prior_value)?;
        if prior.chain != input.chain {
            return Err(MoltenError::invalid_harness(format!(
                "prior checkpoint {prior_checkpoint_ref} belongs to {:?}, expected {:?}",
                prior.chain, input.chain
            )));
        }
    }
    let index = build_chain_index(root)?;
    let Some(anchor) = index.links_by_ref.get(&input.anchor_link_ref) else {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint anchor link {} is unavailable in ledger",
            input.anchor_link_ref
        )));
    };
    if anchor.chain != input.chain {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint anchor link {} belongs to {:?}, expected {:?}",
            input.anchor_link_ref, anchor.chain, input.chain
        )));
    }
    let Some(head) = index.links_by_ref.get(&input.head_ref) else {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint head {} is unavailable in ledger",
            input.head_ref
        )));
    };
    if head.chain != input.chain {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint head {} belongs to {:?}, expected {:?}",
            input.head_ref, head.chain, input.chain
        )));
    }
    let verify_value = ledger::read_artifact(root, &input.verify_receipt_ref)?;
    validate_checkpoint_verify_receipt(CheckpointVerifyReceiptValidationInput {
        root,
        value: &verify_value,
        chain: &input.chain,
        anchor_link_ref: &input.anchor_link_ref,
        head_ref: &input.head_ref,
        range_predicate_ref: &input.range_predicate_ref,
    })?;
    for policy_ref in &input.policy_refs {
        require_ref(policy_ref, "checkpoint policy ref")?;
    }
    for membership_ref in &input.membership_refs {
        require_ref(membership_ref, "checkpoint membership ref")?;
    }
    validate_producer(&input.producer)?;
    for check in &input.checks {
        require_non_empty(&check.name, "checkpoint check name")?;
        require_non_empty(&check.decision, "checkpoint check decision")?;
    }
    require_input_pass_check(input, "raft-control-plane-command")?;
    require_input_pass_check(input, "verified-range")?;
    require_input_pass_check(input, "checkpoint-freshness")
}

fn validate_checkpoint_verify_receipt(input: CheckpointVerifyReceiptValidationInput<'_>) -> Result<()> {
    let receipt = input
        .value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain verify receipt for checkpoint"))?;
    let root = input.root;
    let chain = input.chain;
    let anchor_link_ref = input.anchor_link_ref;
    let head_ref = input.head_ref;
    let range_predicate_ref = input.range_predicate_ref;
    require_schema(
        &receipt[0],
        crate::preserves_rail::EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA,
        "chain verify receipt schema",
    )?;
    let decision = record_string(&receipt[1], "decision", "chain verify decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint verify receipt decision must be pass, got {decision}"
        )));
    }
    let receipt_chain = parse_chain(&receipt[2])?;
    if &receipt_chain != chain {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint verify receipt chain {:?} does not match {:?}",
            receipt_chain, chain
        )));
    }
    let anchor = record_optional_ref(&receipt[3], "anchor")?
        .ok_or_else(|| MoltenError::invalid_harness("checkpoint verify receipt must name an anchor"))?;
    if anchor != anchor_link_ref {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint verify receipt anchor {anchor} does not match {anchor_link_ref}"
        )));
    }
    let expected_head = record_optional_ref(&receipt[4], "expected-head")?
        .ok_or_else(|| MoltenError::invalid_harness("checkpoint verify receipt must name expected head"))?;
    if expected_head != head_ref {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint verify receipt head {expected_head} does not match {head_ref}"
        )));
    }
    validate_range_binding(RangeBindingInput {
        root,
        value: input.value,
        chain,
        anchor_link_ref,
        head_ref,
        range_predicate_ref,
    })
}

fn validate_range_binding(input: RangeBindingInput<'_>) -> Result<()> {
    let receipt = input
        .value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain verify receipt for checkpoint"))?;
    let predicate_refs = record_ref_sequence(&receipt[8], "predicates")?;
    if !predicate_refs.iter().any(|predicate_ref| predicate_ref == input.range_predicate_ref) {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint verify receipt does not bind range predicate {}",
            input.range_predicate_ref
        )));
    }
    let predicate_value = ledger::read_artifact(input.root, input.range_predicate_ref).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "checkpoint range predicate {} is unavailable in ledger: {error}",
            input.range_predicate_ref
        ))
    })?;
    let predicate = parse_chain_predicate_receipt(&predicate_value)?;
    if predicate.predicate != CHECKPOINT_COVERS_RANGE_PREDICATE || predicate.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint range predicate {} must be a passing {CHECKPOINT_COVERS_RANGE_PREDICATE} receipt",
            input.range_predicate_ref
        )));
    }
    let verified_links = record_ref_sequence(&receipt[6], "verified-links")?;
    if verified_links.first().map(String::as_str) != Some(input.anchor_link_ref) {
        return Err(MoltenError::invalid_harness("checkpoint verify receipt segment does not begin at anchor"));
    }
    if verified_links.last().map(String::as_str) != Some(input.head_ref) {
        return Err(MoltenError::invalid_harness("checkpoint verify receipt segment does not end at head"));
    }
    let payload_refs = record_ref_sequence(&receipt[7], "payloads")?;
    if predicate.subject_refs != verified_links {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint range predicate {} subjects do not match verified range",
            input.range_predicate_ref
        )));
    }
    if predicate.input_refs != payload_refs {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint range predicate {} inputs do not match verified payload refs",
            input.range_predicate_ref
        )));
    }
    let expected_context_refs = scope_context_refs(input.chain)?;
    if predicate.context_refs != expected_context_refs {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint range predicate {} context does not match checkpoint chain scope",
            input.range_predicate_ref
        )));
    }
    Ok(())
}

fn select_head_for_verification(
    expected_head: Option<&str>,
    discovered_heads: &[String],
    index: &ChainIndex,
    chain: &ChainScope,
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) -> Option<String> {
    if let Some(expected_head) = expected_head {
        match index.links_by_ref.get(expected_head) {
            Some(head) if &head.chain == chain => {
                if !discovered_heads.iter().any(|head| head == expected_head) {
                    diagnostics.push_item(ChainDiagnostic::new(
                        "stale-head",
                        "expected head is not a current chain head",
                        vec![expected_head.to_string()],
                    ));
                }
                Some(expected_head.to_string())
            }
            Some(_) => {
                diagnostics.push_item(ChainDiagnostic::new(
                    "head-chain-mismatch",
                    "expected head belongs to a different chain scope/id/epoch",
                    vec![expected_head.to_string()],
                ));
                None
            }
            None => {
                diagnostics.push_item(ChainDiagnostic::new(
                    "missing-head",
                    "expected head is unavailable in the ledger",
                    vec![expected_head.to_string()],
                ));
                None
            }
        }
    } else {
        if discovered_heads.len() > 1 {
            diagnostics.push_item(ChainDiagnostic::new(
                "fork",
                "chain has multiple current heads under no-fork verification policy",
                discovered_heads.to_vec(),
            ));
        }
        discovered_heads.first().cloned()
    }
}

fn detect_forks_sequence_conflicts_and_store_evidence(input: ForkDetectionInput<'_>) -> Result<()> {
    let root = input.root;
    let index = input.index;
    let chain = input.chain;
    let chain_refs = input.chain_refs;
    let discovered_heads = input.discovered_heads;
    let selected_head = input.selected_head;
    let fork_policy = input.fork_policy;
    let diagnostics = input.diagnostics;
    if discovered_heads.len() > 1 {
        let evidence_ref = store_chain_fork_evidence(StoreChainForkEvidenceInput {
            root,
            chain,
            parent_ref: None,
            child_refs: discovered_heads,
            selected_head,
            fork_policy,
        })?;
        let mut refs = discovered_heads.to_vec();
        refs.push(evidence_ref);
        diagnostics.push(ChainDiagnostic::new("fork", "chain has more than one derived head", refs));
    }
    for link_ref in chain_refs {
        let children = index
            .children_for_parent(link_ref)
            .into_iter()
            .filter(|child_ref| index.links_by_ref.get(child_ref).is_some_and(|child| &child.chain == chain))
            .collect::<Vec<_>>();
        if children.len() > 1 {
            let evidence_ref = store_chain_fork_evidence(StoreChainForkEvidenceInput {
                root,
                chain,
                parent_ref: Some(link_ref),
                child_refs: &children,
                selected_head,
                fork_policy,
            })?;
            let mut refs = children;
            refs.push(evidence_ref);
            diagnostics.push(ChainDiagnostic::new(
                "fork",
                "chain parent has multiple children under fork verification policy",
                refs,
            ));
        }
    }
    for ((entry_chain, sequence), occupants) in &index.links_by_sequence {
        if entry_chain == chain && occupants.len() > 1 {
            let occupants = occupants.iter().cloned().collect::<Vec<_>>();
            let evidence_ref = store_chain_fork_evidence(StoreChainForkEvidenceInput {
                root,
                chain,
                parent_ref: None,
                child_refs: &occupants,
                selected_head,
                fork_policy,
            })?;
            let mut refs = occupants;
            refs.push(evidence_ref);
            diagnostics.push(ChainDiagnostic::new(
                "sequence-conflict",
                format!("chain sequence {sequence} has multiple links"),
                refs,
            ));
        }
    }
    Ok(())
}

fn store_chain_fork_evidence(input: StoreChainForkEvidenceInput<'_>) -> Result<String> {
    let value = chain_fork_evidence_value(&ChainForkEvidenceValueInput {
        chain: input.chain,
        parent_ref: input.parent_ref,
        child_refs: input.child_refs,
        selected_head: input.selected_head,
        fork_policy: input.fork_policy,
    });
    let parsed = parse_chain_fork_evidence(&value)?;
    let imported = ledger::import_artifact(input.root, &value)?;
    if imported.artifact_ref != parsed.evidence_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported fork evidence ref mismatch: got {}, expected {}",
            imported.artifact_ref, parsed.evidence_ref
        )));
    }
    Ok(parsed.evidence_ref)
}

fn store_chain_predicate_receipt(input: StoreChainPredicateReceiptInput<'_>) -> Result<String> {
    let value = chain_predicate_receipt_value(&input.receipt);
    let parsed = parse_chain_predicate_receipt(&value)?;
    let imported = ledger::import_artifact(input.root, &value)?;
    if imported.artifact_ref != parsed.receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported chain predicate receipt ref mismatch: got {}, expected {}",
            imported.artifact_ref, parsed.receipt_ref
        )));
    }
    Ok(parsed.receipt_ref)
}

fn predicate_input_refs(link: &ChainLink) -> Vec<String> {
    let mut refs = Vec::new();
    if let Some(previous_link_ref) = &link.previous_link_ref {
        refs.push(previous_link_ref.clone());
    }
    refs.push(link.payload.artifact_ref.clone());
    refs.push(link.trellis.input_ref.clone());
    refs
}

fn predicate_context_refs(link: &ChainLink) -> Vec<String> {
    link.context_refs.iter().map(|context| context.artifact_ref.clone()).collect()
}

fn store_segment_predicate_receipts(input: SegmentPredicateReceiptsInput<'_>) -> Result<Vec<String>> {
    let scope_refs = scope_context_refs(input.chain)?;
    let mut refs = Vec::with_capacity(4);
    refs.push(gap_receipt_ref(&input, &scope_refs)?);
    refs.push(fork_receipt_ref(&input, &scope_refs)?);
    if let Some(receipt_ref) = anchor_receipt_ref(&input, &scope_refs)? {
        refs.push(receipt_ref);
    }
    if let Some(receipt_ref) = checkpoint_receipt_ref(&input, &scope_refs)? {
        refs.push(receipt_ref);
    }
    Ok(refs)
}

fn gap_receipt_ref(input: &SegmentPredicateReceiptsInput<'_>, scope_refs: &[String]) -> Result<String> {
    let decision = predicate_decision(input.diagnostics, input.fork_policy, &[
        "gap",
        "genesis-invalid",
        "append-invalid",
        "cycle",
    ]);
    let checks = vec![
        ChainCheck::pass("segment-contiguity"),
        ChainCheck::pass("canonical-link-order"),
    ];
    store_chain_predicate_receipt(StoreChainPredicateReceiptInput {
        root: input.root,
        receipt: ChainPredicateReceiptValueInput {
            predicate: SEGMENT_NO_GAP_PREDICATE,
            decision,
            subject_refs: input.verified_links,
            input_refs: input.payload_refs,
            context_refs: scope_refs,
            checks: &checks,
        },
    })
}

fn fork_receipt_ref(input: &SegmentPredicateReceiptsInput<'_>, scope_refs: &[String]) -> Result<String> {
    let decision = match (
        input
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind.as_str(), "fork" | "sequence-conflict")),
        input.fork_policy,
    ) {
        (false, _) => "pass",
        (true, ChainForkPolicy::RejectUnexpectedForks) => "fail",
        (true, ChainForkPolicy::RetainForkEvidence) => "retained",
    };
    let checks = vec![
        ChainCheck::pass("fork-policy-profile"),
        ChainCheck::pass("fork-evidence-binding"),
    ];
    store_chain_predicate_receipt(StoreChainPredicateReceiptInput {
        root: input.root,
        receipt: ChainPredicateReceiptValueInput {
            predicate: SEGMENT_NO_FORK_PREDICATE,
            decision,
            subject_refs: input.discovered_heads,
            input_refs: input.verified_links,
            context_refs: scope_refs,
            checks: &checks,
        },
    })
}

fn anchor_receipt_ref(input: &SegmentPredicateReceiptsInput<'_>, scope_refs: &[String]) -> Result<Option<String>> {
    if input.anchor_ref.is_none() && input.expected_head.is_none() {
        return Ok(None);
    }
    let mut subjects = Vec::with_capacity(2);
    if let Some(anchor_ref) = input.anchor_ref {
        subjects.push(anchor_ref.to_string());
    }
    if let Some(expected_head) = input.expected_head {
        subjects.push(expected_head.to_string());
    }
    let checks = vec![ChainCheck::pass("anchor-descent"), ChainCheck::pass("head-binding")];
    let receipt_ref = store_chain_predicate_receipt(StoreChainPredicateReceiptInput {
        root: input.root,
        receipt: ChainPredicateReceiptValueInput {
            predicate: DESCENDS_FROM_ANCHOR_PREDICATE,
            decision: predicate_decision(input.diagnostics, input.fork_policy, &[
                "missing-anchor",
                "anchor-chain-mismatch",
                "anchor-descent",
                "missing-head",
                "head-chain-mismatch",
                "stale-head",
            ]),
            subject_refs: &subjects,
            input_refs: input.verified_links,
            context_refs: scope_refs,
            checks: &checks,
        },
    })?;
    Ok(Some(receipt_ref))
}

fn checkpoint_receipt_ref(input: &SegmentPredicateReceiptsInput<'_>, scope_refs: &[String]) -> Result<Option<String>> {
    if input.anchor_ref.is_none() || input.expected_head.is_none() || input.decision != "pass" {
        return Ok(None);
    }
    let checks = vec![
        ChainCheck::pass("checkpoint-range-coverage"),
        ChainCheck::pass("verified-range"),
    ];
    let receipt_ref = store_chain_predicate_receipt(StoreChainPredicateReceiptInput {
        root: input.root,
        receipt: ChainPredicateReceiptValueInput {
            predicate: CHECKPOINT_COVERS_RANGE_PREDICATE,
            decision: "pass",
            subject_refs: input.verified_links,
            input_refs: input.payload_refs,
            context_refs: scope_refs,
            checks: &checks,
        },
    })?;
    Ok(Some(receipt_ref))
}

fn predicate_decision(
    diagnostics: &[ChainDiagnostic],
    fork_policy: ChainForkPolicy,
    failing_kinds: &[&str],
) -> &'static str {
    if diagnostics.iter().any(|diagnostic| {
        failing_kinds.contains(&diagnostic.kind.as_str()) && diagnostic_is_fatal(diagnostic, fork_policy)
    }) {
        "fail"
    } else {
        "pass"
    }
}

fn scope_context_refs(chain: &ChainScope) -> Result<Vec<String>> {
    Ok(vec![canonical_hash(&chain_record(chain))?])
}

fn validate_verified_segment(
    index: &ChainIndex,
    anchor_ref: Option<&str>,
    verified_links: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) {
    if verified_links.is_empty() {
        return;
    }
    if let Some(anchor_ref) = anchor_ref {
        if verified_links.first().map(String::as_str) != Some(anchor_ref) {
            diagnostics.push_item(ChainDiagnostic::new(
                "anchor-descent",
                "verified segment does not begin at requested anchor",
                vec![anchor_ref.to_string()],
            ));
        }
    } else if let Some(first_ref) = verified_links.first() {
        let Some(first) = index.links_by_ref.get(first_ref) else {
            return;
        };
        if let Err(error) = validate_genesis(first) {
            diagnostics.push_item(ChainDiagnostic::new(
                "genesis-invalid",
                format!("segment does not begin with a valid genesis link: {error}"),
                vec![first_ref.clone()],
            ));
        }
    }

    for window in verified_links.windows(2) {
        let previous_ref = &window[0];
        let next_ref = &window[1];
        let (Some(previous), Some(next)) = (index.links_by_ref.get(previous_ref), index.links_by_ref.get(next_ref))
        else {
            diagnostics.push_item(ChainDiagnostic::new(
                "gap",
                "verified segment references an unavailable adjacent link",
                vec![previous_ref.clone(), next_ref.clone()],
            ));
            continue;
        };
        if let Err(error) = validate_append(previous, next) {
            diagnostics.push_item(ChainDiagnostic::new(
                "gap",
                format!("adjacent links are not a valid append: {error}"),
                vec![previous_ref.clone(), next_ref.clone()],
            ));
        }
    }
}

fn diagnostic_is_fatal(diagnostic: &ChainDiagnostic, fork_policy: ChainForkPolicy) -> bool {
    match diagnostic.kind.as_str() {
        "fork" | "sequence-conflict" => fork_policy.fork_diagnostics_are_fatal(),
        _ => true,
    }
}

fn diagnostic_check(label: &str, diagnostics: &[ChainDiagnostic], failing_kinds: &[&str]) -> IOValue {
    let decision = if diagnostics.iter().any(|diagnostic| failing_kinds.contains(&diagnostic.kind.as_str())) {
        "fail"
    } else {
        "pass"
    };
    record("check", vec![string(label), string(decision)])
}

fn fork_policy_check(diagnostics: &[ChainDiagnostic], fork_policy: ChainForkPolicy) -> IOValue {
    let has_fork = diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.kind.as_str(), "fork" | "sequence-conflict"));
    let decision = match (has_fork, fork_policy) {
        (false, _) => "pass",
        (true, ChainForkPolicy::RejectUnexpectedForks) => "fail",
        (true, ChainForkPolicy::RetainForkEvidence) => "retained",
    };
    record("check", vec![string("no-fork-policy"), string(decision)])
}

fn diagnostic_value(diagnostic: &ChainDiagnostic) -> IOValue {
    record("diagnostic", vec![
        string(&diagnostic.kind),
        string(&diagnostic.detail),
        ref_sequence_value(&diagnostic.refs),
    ])
}

fn ref_sequence_value(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn check_value(check: &ChainCheck) -> IOValue {
    record("check", vec![string(&check.name), string(&check.decision)])
}

fn producer_record(producer: &ChainProducer) -> IOValue {
    record("producer", vec![
        record("id", vec![string(&producer.id)]),
        record("key", vec![string(&producer.key_ref)]),
    ])
}

fn parse_checkpoint_range(value: &Value<IOValue>) -> Result<(String, String, String, String)> {
    let value = value_to_iovalue(value);
    let range = value
        .collect_simple_record("range", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("chain checkpoint missing range record"))?;
    Ok((
        record_string(&range[0], "anchor", "checkpoint range anchor")?,
        record_string(&range[1], "head", "checkpoint range head")?,
        record_string(&range[2], "verify-receipt", "checkpoint range verify receipt")?,
        record_string(&range[3], "predicate", "checkpoint range predicate receipt")?,
    ))
}

fn parse_control_plane(value: &Value<IOValue>) -> Result<()> {
    let value = value_to_iovalue(value);
    let control = value
        .collect_simple_record("control-plane", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("chain checkpoint missing control-plane record"))?;
    let mode = record_string(&control[0], "mode", "checkpoint control-plane mode")?;
    if mode != "trellis-raft" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported checkpoint control-plane mode {mode}; expected trellis-raft"
        )));
    }
    let command = record_string(&control[1], "command", "checkpoint control-plane command")?;
    if command != "accept-chain-head" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported checkpoint control-plane command {command}; expected accept-chain-head"
        )));
    }
    Ok(())
}

fn chain_record(chain: &ChainScope) -> IOValue {
    record("chain", vec![
        record("scope", vec![string(&chain.scope)]),
        record("id", vec![string(&chain.id)]),
        record("epoch", vec![string(&chain.epoch)]),
    ])
}

fn ensure_sequence_unoccupied(index: &ChainIndex, link: &ChainLink) -> Result<()> {
    let occupants = index.links_for_sequence(&link.chain, link.sequence);
    if occupants.is_empty() {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "chain sequence {} for {:?} is already occupied by {:?}",
            link.sequence, link.chain, occupants
        )))
    }
}

fn sorted_refs(refs: Option<&BTreeSet<String>>) -> Vec<String> {
    refs.map_or_else(Vec::new, |refs| refs.iter().cloned().collect())
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_chain(value: &Value<IOValue>) -> Result<ChainScope> {
    let value = value_to_iovalue(value);
    let chain = value
        .collect_simple_record("chain", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing chain record"))?;
    Ok(ChainScope {
        scope: record_string(&chain[0], "scope", "chain scope")?,
        id: record_string(&chain[1], "id", "chain id")?,
        epoch: record_string(&chain[2], "epoch", "chain epoch")?,
    })
}

fn parse_previous_link_ref(value: &Value<IOValue>) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let prev = value
        .collect_simple_record("prev", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing prev record"))?;
    let prev_value = value_to_iovalue(&prev[0]);
    if prev_value.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = prev_value.collect_simple_record("some", Some(1)) {
        required_string(&some[0], "previous link ref").map(Some)
    } else {
        Err(MoltenError::invalid_harness("chain link prev must be <none> or <some ref>"))
    }
}

fn parse_payload(value: &Value<IOValue>) -> Result<ChainPayload> {
    let value = value_to_iovalue(value);
    let payload = value
        .collect_simple_record("payload", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing payload record"))?;
    Ok(ChainPayload {
        kind: record_string(&payload[0], "kind", "payload kind")?,
        artifact_ref: record_string(&payload[1], "ref", "payload ref")?,
        schema: record_string(&payload[2], "schema", "payload schema")?,
    })
}

fn parse_context_refs(value: &Value<IOValue>) -> Result<Vec<ChainContextRef>> {
    let value = value_to_iovalue(value);
    let context = value
        .collect_simple_record("context", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing context record"))?;
    let refs = context[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("chain link context must be a sequence"))?;
    refs.iter()
        .map(|value| {
            let value = value_to_iovalue(value);
            let context_ref = value
                .collect_simple_record("ref", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("chain context item must be <ref label artifact-ref>"))?;
            Ok(ChainContextRef {
                label: required_string(&context_ref[0], "context ref label")?,
                artifact_ref: required_string(&context_ref[1], "context artifact ref")?,
            })
        })
        .collect()
}

fn parse_producer(value: &Value<IOValue>) -> Result<ChainProducer> {
    let value = value_to_iovalue(value);
    let producer = value
        .collect_simple_record("producer", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing producer record"))?;
    Ok(ChainProducer {
        id: record_string(&producer[0], "id", "producer id")?,
        key_ref: record_string(&producer[1], "key", "producer key ref")?,
    })
}

fn parse_trellis(value: &Value<IOValue>) -> Result<ChainTrellisEvidence> {
    let value = value_to_iovalue(value);
    let trellis = value
        .collect_simple_record("trellis", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing trellis record"))?;
    Ok(ChainTrellisEvidence {
        predicate: record_string(&trellis[0], "predicate", "trellis predicate")?,
        input_ref: record_string(&trellis[1], "input", "trellis predicate input ref")?,
        decision: record_string(&trellis[2], "decision", "trellis decision")?,
    })
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<ChainCheck>> {
    let value = value_to_iovalue(value);
    let checks = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("chain link missing checks record"))?;
    let sequence = checks[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("chain link checks must be a sequence"))?;
    sequence
        .iter()
        .map(|value| {
            let value = value_to_iovalue(value);
            let check = value
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("chain check item must be <check name decision>"))?;
            Ok(ChainCheck {
                name: required_string(&check[0], "check name")?,
                decision: required_string(&check[1], "check decision")?,
            })
        })
        .collect()
}

fn validate_chain_link_shape(link: &ChainLink) -> Result<()> {
    validate_chain_scope(&link.chain)?;
    require_ref(&link.link_ref, "chain link ref")?;
    if let Some(previous_link_ref) = &link.previous_link_ref {
        require_ref(previous_link_ref, "previous link ref")?;
    }
    require_non_empty(&link.payload.kind, "payload kind")?;
    require_ref(&link.payload.artifact_ref, "payload ref")?;
    require_non_empty(&link.payload.schema, "payload schema")?;
    for context_ref in &link.context_refs {
        require_non_empty(&context_ref.label, "context ref label")?;
        require_ref(&context_ref.artifact_ref, "context artifact ref")?;
    }
    require_non_empty(&link.producer.id, "producer id")?;
    require_ref(&link.producer.key_ref, "producer key ref")?;
    require_non_empty(&link.trellis.predicate, "trellis predicate")?;
    require_ref(&link.trellis.input_ref, "trellis input ref")?;
    require_non_empty(&link.trellis.decision, "trellis decision")?;
    for check in &link.checks {
        require_non_empty(&check.name, "chain check name")?;
        require_non_empty(&check.decision, "chain check decision")?;
    }
    Ok(())
}

fn validate_chain_scope(chain: &ChainScope) -> Result<()> {
    require_non_empty(&chain.scope, "chain scope")?;
    require_non_empty(&chain.id, "chain id")?;
    require_non_empty(&chain.epoch, "chain epoch")
}

fn validate_producer(producer: &ChainProducer) -> Result<()> {
    require_non_empty(&producer.id, "producer id")?;
    require_ref(&producer.key_ref, "producer key ref")
}

fn validate_chain_predicate_receipt_shape(receipt: &ChainPredicateReceipt) -> Result<()> {
    require_ref(&receipt.receipt_ref, "chain predicate receipt ref")?;
    require_non_empty(&receipt.predicate, "chain predicate name")?;
    match receipt.decision.as_str() {
        "pass" | "fail" | "retained" => {}
        other => return Err(MoltenError::invalid_harness(format!("unsupported chain predicate decision {other}"))),
    }
    for subject_ref in &receipt.subject_refs {
        require_ref(subject_ref, "chain predicate subject ref")?;
    }
    for input_ref in &receipt.input_refs {
        require_ref(input_ref, "chain predicate input ref")?;
    }
    for context_ref in &receipt.context_refs {
        require_ref(context_ref, "chain predicate context ref")?;
    }
    require_pass_check_in(&receipt.checks, "trellis-bounded-predicate")
        .or_else(|_| require_pass_check_in(&receipt.checks, "segment-contiguity"))
        .or_else(|_| require_pass_check_in(&receipt.checks, "fork-policy-profile"))
        .or_else(|_| require_pass_check_in(&receipt.checks, "anchor-descent"))
        .or_else(|_| require_pass_check_in(&receipt.checks, "checkpoint-range-coverage"))
}

fn validate_chain_fork_evidence_shape(fork: &ChainForkEvidence) -> Result<()> {
    require_ref(&fork.evidence_ref, "chain fork evidence ref")?;
    validate_chain_scope(&fork.chain)?;
    if let Some(parent_ref) = &fork.parent_ref {
        require_ref(parent_ref, "fork parent ref")?;
    }
    if fork.child_refs.len() < 2 {
        return Err(MoltenError::invalid_harness("fork evidence must name at least two child/head refs"));
    }
    for child_ref in &fork.child_refs {
        require_ref(child_ref, "fork child ref")?;
    }
    if let Some(selected_head) = &fork.selected_head {
        require_ref(selected_head, "fork selected head ref")?;
    }
    match fork.profile.as_str() {
        "reject-unexpected-forks" | "retain-fork-evidence" => {}
        other => {
            return Err(MoltenError::invalid_harness(format!("unsupported fork policy profile {other}")));
        }
    }
    match fork.decision.as_str() {
        "reject" | "retain" => {}
        other => return Err(MoltenError::invalid_harness(format!("unsupported fork decision {other}"))),
    }
    require_pass_check_in(&fork.checks, "fork-detected")?;
    require_pass_check_in(&fork.checks, "fork-policy-profile")?;
    require_pass_check_in(&fork.checks, "diagnostic-retention")
}

fn validate_chain_anchor_shape(anchor: &ChainAnchor) -> Result<()> {
    require_ref(&anchor.anchor_ref, "chain anchor ref")?;
    validate_chain_scope(&anchor.chain)?;
    require_ref(&anchor.link_ref, "anchor link ref")?;
    for policy_ref in &anchor.policy_refs {
        require_ref(policy_ref, "anchor policy ref")?;
    }
    validate_producer(&anchor.producer)?;
    require_pass_check_in(&anchor.checks, "trusted-anchor")?;
    require_pass_check_in(&anchor.checks, "anchor-link-available")
}

fn validate_chain_checkpoint_shape(checkpoint: &ChainCheckpoint) -> Result<()> {
    require_ref(&checkpoint.checkpoint_ref, "chain checkpoint ref")?;
    validate_chain_scope(&checkpoint.chain)?;
    if let Some(prior_checkpoint_ref) = &checkpoint.prior_checkpoint_ref {
        require_ref(prior_checkpoint_ref, "prior checkpoint ref")?;
    }
    require_ref(&checkpoint.anchor_link_ref, "checkpoint anchor link ref")?;
    require_ref(&checkpoint.head_ref, "checkpoint head ref")?;
    require_ref(&checkpoint.verify_receipt_ref, "checkpoint verify receipt ref")?;
    require_ref(&checkpoint.range_predicate_ref, "checkpoint range predicate ref")?;
    for policy_ref in &checkpoint.policy_refs {
        require_ref(policy_ref, "checkpoint policy ref")?;
    }
    for membership_ref in &checkpoint.membership_refs {
        require_ref(membership_ref, "checkpoint membership ref")?;
    }
    validate_producer(&checkpoint.producer)?;
    require_pass_check_in(&checkpoint.checks, "raft-control-plane-command")?;
    require_pass_check_in(&checkpoint.checks, "verified-range")?;
    require_pass_check_in(&checkpoint.checks, "checkpoint-freshness")
}

fn require_trellis_pass(link: &ChainLink, expected_predicate: &str) -> Result<()> {
    if link.trellis.predicate != expected_predicate {
        return Err(MoltenError::invalid_harness(format!(
            "chain link trellis predicate {} does not match expected {expected_predicate}",
            link.trellis.predicate
        )));
    }
    if link.trellis.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "chain link trellis decision must be pass, got {}",
            link.trellis.decision
        )));
    }
    Ok(())
}

fn require_pass_check(link: &ChainLink, name: &str) -> Result<()> {
    require_pass_check_in(&link.checks, name)
        .map_err(|_| MoltenError::invalid_harness(format!("chain link missing pass check {name}")))
}

fn require_input_pass_check(input: &ChainCheckpointInput, name: &str) -> Result<()> {
    require_pass_check_in(&input.checks, name)
        .map_err(|_| MoltenError::invalid_harness(format!("chain checkpoint missing pass check {name}")))
}

fn require_pass_check_in(checks: &[ChainCheck], name: &str) -> Result<()> {
    if checks.iter().any(|check| check.name == name && check.decision == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("missing pass check {name}")))
    }
}

fn record_string(value: &Value<IOValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> for {field}")))?;
    required_string(&record[0], field)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> optional ref")))?;
    let value = value_to_iovalue(&record[0]);
    if value.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = value.collect_simple_record("some", Some(1)) {
        required_string(&some[0], label).map(Some)
    } else {
        Err(MoltenError::invalid_harness(format!("expected <some ref> or <none> for {label}")))
    }
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> ref sequence")))?;
    let sequence = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    sequence.iter().map(|value| required_string(value, label)).collect()
}

fn record_u64(value: &Value<IOValue>, label: &str, field: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> for {field}")))?;
    required_u64(&record[0], field)
}

fn require_schema(value: &Value<IOValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {field} {actual}; expected {expected}")))
    }
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn require_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn require_ref(value: &str, field: &str) -> Result<()> {
    require_non_empty(value, field)?;
    validate_content_ref(value).map_err(|error| {
        MoltenError::invalid_harness(format!("unsupported {field} {value}; expected canonical content ref: {error}"))
    })
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    } else {
        Ok(())
    }
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let count = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(count, maximum, label)?;
    values.push_item(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::preserves_rail::canonical_bytes;
    use crate::preserves_rail::parse_text;

    #[test]
    fn chain_link_identity_is_canonical_and_stable() {
        let input = sample_genesis_input("evidence-ledger", "node-a", "epoch-1", "payload-a");
        let first = chain_link_value(&input);
        let second = chain_link_value(&input);
        assert_eq!(canonical_bytes(&first).expect("first bytes"), canonical_bytes(&second).expect("second bytes"));
        assert_eq!(chain_link_ref(&first).expect("first ref"), chain_link_ref(&second).expect("second ref"));

        let parsed = parse_chain_link(&first).expect("parse chain link");
        assert_eq!(parsed.link_ref, chain_link_ref(&first).expect("chain link ref"));
        validate_genesis(&parsed).expect("valid genesis");
    }

    #[test]
    fn chain_link_preserves_payload_ref_without_rewriting_payload() {
        let payload_value = parse_text("<gate-receipt-placeholder \"ok\">").expect("parse payload");
        let payload_ref = canonical_hash(&payload_value).expect("payload ref");
        let input = ChainLinkInput::genesis(
            ChainScope::new("evidence-ledger", "node-a", "epoch-1"),
            ChainPayload::new("gate-receipt", payload_ref.clone(), "molten.harness.gate-receipt.v1"),
            vec![ChainContextRef::new("policy", ref_for("policy"))],
            sample_producer(),
            ref_for("genesis-input"),
        );

        let link_value = chain_link_value(&input);
        let parsed = parse_chain_link(&link_value).expect("parse chain link");
        validate_genesis(&parsed).expect("valid genesis");
        assert_eq!(parsed.payload.artifact_ref, payload_ref);
        assert_ne!(parsed.link_ref, payload_ref);
        assert_eq!(canonical_hash(&payload_value).expect("payload ref after link"), payload_ref);
    }

    #[test]
    fn append_validation_binds_previous_ref_sequence_and_scope() {
        let previous = sample_genesis("evidence-ledger", "node-a", "epoch-1", "payload-a");
        let good_input = ChainLinkInput::append(
            &previous,
            sample_payload("payload-b"),
            vec![ChainContextRef::new("policy", ref_for("policy"))],
            sample_producer(),
            ref_for("append-input"),
        );
        let good = parse_chain_link(&chain_link_value(&good_input)).expect("parse append");
        validate_append(&previous, &good).expect("valid append");

        let mut wrong_previous_input = good_input.clone();
        wrong_previous_input.previous_link_ref = Some(ref_for("wrong-previous"));
        let wrong_previous = parse_chain_link(&chain_link_value(&wrong_previous_input)).expect("parse wrong previous");
        let error = validate_append(&previous, &wrong_previous).expect_err("wrong previous ref rejected");
        assert!(error.to_string().contains("previous link ref mismatch"));

        let mut gap_input = good_input.clone();
        gap_input.sequence += 1;
        let gap = parse_chain_link(&chain_link_value(&gap_input)).expect("parse sequence gap");
        let error = validate_append(&previous, &gap).expect_err("sequence gap rejected");
        assert!(error.to_string().contains("previous + 1"));

        let mut wrong_scope_input = good_input;
        wrong_scope_input.chain.id = "node-b".to_string();
        let wrong_scope = parse_chain_link(&chain_link_value(&wrong_scope_input)).expect("parse wrong scope");
        let error = validate_append(&previous, &wrong_scope).expect_err("wrong scope rejected");
        assert!(error.to_string().contains("same chain scope"));
    }

    #[test]
    fn genesis_validation_rejects_previous_links_and_nonzero_sequences() {
        let mut input = sample_genesis_input("evidence-ledger", "node-a", "epoch-1", "payload-a");
        input.previous_link_ref = Some(ref_for("unexpected-previous"));
        let with_previous = parse_chain_link(&chain_link_value(&input)).expect("parse genesis with previous");
        let error = validate_genesis(&with_previous).expect_err("previous rejected");
        assert!(error.to_string().contains("must not name a previous"));

        let mut input = sample_genesis_input("evidence-ledger", "node-a", "epoch-1", "payload-a");
        input.sequence = 1;
        let nonzero = parse_chain_link(&chain_link_value(&input)).expect("parse nonzero genesis");
        let error = validate_genesis(&nonzero).expect_err("nonzero rejected");
        assert!(error.to_string().contains("sequence must be 0"));
    }

    #[test]
    fn chain_hashing_is_scoped_not_global_ordering() {
        let left = sample_genesis("evidence-ledger", "node-a", "epoch-1", "payload-left");
        let right = sample_genesis("artifact-lineage", "catalog-a", "epoch-1", "payload-right");
        validate_genesis(&left).expect("left genesis valid");
        validate_genesis(&right).expect("right genesis valid");
        assert_eq!(left.sequence, 0);
        assert_eq!(right.sequence, 0);
        assert_ne!(left.chain, right.chain);

        let mut cross_scope_input = ChainLinkInput::append(
            &left,
            sample_payload("payload-cross"),
            Vec::new(),
            sample_producer(),
            ref_for("cross-scope-input"),
        );
        cross_scope_input.chain = right.chain.clone();
        let cross_scope = parse_chain_link(&chain_link_value(&cross_scope_input)).expect("parse cross-scope append");
        let error = validate_append(&left, &cross_scope).expect_err("cross-scope append rejected");
        assert!(error.to_string().contains("same chain scope"));
    }

    #[test]
    fn chain_ledger_append_stores_links_indexes_heads_and_receipts() {
        let root = temp_dir("chain-append");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_input = ChainLinkInput::genesis(
            chain.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        );
        let genesis_value = chain_link_value(&genesis_input);
        let genesis_link = parse_chain_link(&genesis_value).expect("parse genesis");
        let genesis_append = append_chain_link(&root, &genesis_value).expect("append genesis");
        assert_eq!(genesis_append.head_before, None);
        assert_eq!(genesis_append.head_after, genesis_link.link_ref);
        assert_eq!(genesis_append.payload_ref, genesis_link.payload.artifact_ref);
        let append_receipt = genesis_append
            .receipt_value
            .collect_simple_record("chain-append-receipt-v1", Some(9))
            .expect("append receipt shape");
        let append_predicates = record_ref_sequence(&append_receipt[7], "predicates").expect("append predicates");
        assert_eq!(append_predicates, vec![genesis_append.predicate_receipt_ref.clone()]);
        let append_predicate = parse_chain_predicate_receipt(
            &ledger::read_artifact(&root, &genesis_append.predicate_receipt_ref).expect("append predicate receipt"),
        )
        .expect("parse append predicate");
        assert_eq!(append_predicate.predicate, GENESIS_VALID_PREDICATE);
        assert_eq!(ledger::read_artifact(&root, &genesis_append.link_ref).expect("stored genesis"), genesis_value);
        assert_eq!(
            ledger::read_artifact(&root, &genesis_append.receipt_ref).expect("stored append receipt"),
            genesis_append.receipt_value
        );

        let index = build_chain_index(&root).expect("build index after genesis");
        assert_eq!(index.heads_for_chain(&chain), vec![genesis_append.link_ref.clone()]);
        assert_eq!(index.links_for_payload(&genesis_link.payload.artifact_ref), vec![genesis_append.link_ref.clone()]);

        let second_input = ChainLinkInput::append(
            &genesis_link,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        );
        let second_value = chain_link_value(&second_input);
        let second_link = parse_chain_link(&second_value).expect("parse second link");
        let second_append = append_chain_link(&root, &second_value).expect("append second link");
        assert_eq!(second_append.head_before, Some(genesis_append.link_ref.clone()));
        assert_eq!(second_append.head_after, second_link.link_ref);

        let index = build_chain_index(&root).expect("build index after append");
        assert_eq!(index.heads_for_chain(&chain), vec![second_append.link_ref.clone()]);
        assert_eq!(index.children_for_parent(&genesis_append.link_ref), vec![second_append.link_ref.clone()]);
        assert_eq!(index.links_for_sequence(&chain, 0), vec![genesis_append.link_ref]);
        assert_eq!(index.links_for_sequence(&chain, 1), vec![second_append.link_ref]);
    }

    #[test]
    fn chain_ledger_append_rejects_missing_payload_sequence_gaps_and_forks() {
        let missing_root = temp_dir("chain-missing-payload");
        let missing_payload_link =
            chain_link_value(&sample_genesis_input("evidence-ledger", "node-a", "epoch-1", "missing-payload"));
        let error = append_chain_link(&missing_root, &missing_payload_link).expect_err("missing payload rejected");
        assert!(error.to_string().contains("payload"));

        let root = temp_dir("chain-rejections");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            chain,
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let genesis_link = parse_chain_link(&genesis_value).expect("parse genesis");
        append_chain_link(&root, &genesis_value).expect("append genesis");

        let mut gap_input = ChainLinkInput::append(
            &genesis_link,
            stored_payload(&root, "payload-gap"),
            Vec::new(),
            sample_producer(),
            ref_for("gap-input"),
        );
        gap_input.sequence += 1;
        let gap_error = append_chain_link(&root, &chain_link_value(&gap_input)).expect_err("sequence gap rejected");
        assert!(gap_error.to_string().contains("previous + 1"));

        let first_child_value = chain_link_value(&ChainLinkInput::append(
            &genesis_link,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-b"),
        ));
        append_chain_link(&root, &first_child_value).expect("append first child");
        let fork_value = chain_link_value(&ChainLinkInput::append(
            &genesis_link,
            stored_payload(&root, "payload-c"),
            Vec::new(),
            sample_producer(),
            ref_for("append-c"),
        ));
        let fork_error = append_chain_link(&root, &fork_value).expect_err("fork rejected");
        assert!(fork_error.to_string().contains("unexpected fork"));
    }

    #[test]
    fn chain_verify_receipt_passes_for_anchor_to_head_segment() {
        let root = temp_dir("chain-verify-pass");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            chain.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let genesis = parse_chain_link(&genesis_value).expect("parse genesis");
        append_chain_link(&root, &genesis_value).expect("append genesis");
        let second_value = chain_link_value(&ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        ));
        let second = parse_chain_link(&second_value).expect("parse second");
        append_chain_link(&root, &second_value).expect("append second");

        let verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&second.link_ref))
            .expect("verify segment");
        assert_eq!(verified.decision, "pass");
        assert!(verified.diagnostics.is_empty());
        assert_eq!(verified.verified_links, vec![genesis.link_ref, second.link_ref]);
        let verify_receipt = verified
            .receipt_value
            .collect_simple_record("chain-verify-receipt-v1", Some(11))
            .expect("verify receipt shape");
        assert_eq!(
            record_ref_sequence(&verify_receipt[8], "predicates").expect("verify predicates"),
            verified.predicate_receipt_refs
        );
        let predicate_names = predicate_names(&root, &verified.predicate_receipt_refs);
        assert!(predicate_names.contains(&SEGMENT_NO_GAP_PREDICATE.to_string()));
        assert!(predicate_names.contains(&SEGMENT_NO_FORK_PREDICATE.to_string()));
        assert!(predicate_names.contains(&DESCENDS_FROM_ANCHOR_PREDICATE.to_string()));
        assert!(predicate_names.contains(&CHECKPOINT_COVERS_RANGE_PREDICATE.to_string()));
        assert_eq!(
            ledger::read_artifact(&root, &verified.receipt_ref).expect("stored verify receipt"),
            verified.receipt_value
        );
    }

    #[test]
    fn chain_verify_receipt_reports_fork_gap_stale_head_and_missing_payload() {
        assert_fork_diagnostic();
        assert_gap_diagnostic();
        assert_stale_head_diagnostic();
        assert_missing_payload_diagnostic();
    }

    fn assert_fork_diagnostic() {
        let root = temp_dir("chain-verify-fork");
        let scope = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis = import_genesis_link(&root, scope.clone(), "payload-a");
        import_raw_link(
            &root,
            &ChainLinkInput::append(
                &genesis,
                stored_payload(&root, "payload-b"),
                Vec::new(),
                sample_producer(),
                ref_for("append-b"),
            ),
        );
        import_raw_link(
            &root,
            &ChainLinkInput::append(
                &genesis,
                stored_payload(&root, "payload-c"),
                Vec::new(),
                sample_producer(),
                ref_for("append-c"),
            ),
        );
        let verified = verify_chain_segment(&root, &scope, None, None).expect("verify forked chain");
        assert_eq!(verified.decision, "fail");
        assert!(has_diagnostic(&verified, "fork"));
    }

    fn assert_gap_diagnostic() {
        let root = temp_dir("chain-verify-gap");
        let scope = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis = import_genesis_link(&root, scope.clone(), "payload-a");
        let mut input = ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-gap"),
            Vec::new(),
            sample_producer(),
            ref_for("gap-input"),
        );
        input.sequence += 1;
        let link = import_raw_link(&root, &input);
        let verified = verify_chain_segment(&root, &scope, None, Some(&link.link_ref)).expect("verify gap chain");
        assert_eq!(verified.decision, "fail");
        assert!(has_diagnostic(&verified, "gap"));
    }

    fn assert_stale_head_diagnostic() {
        let root = temp_dir("chain-verify-stale");
        let scope = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            scope.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let genesis = parse_chain_link(&genesis_value).expect("parse stale genesis");
        append_chain_link(&root, &genesis_value).expect("append stale genesis");
        let child_value = chain_link_value(&ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        ));
        append_chain_link(&root, &child_value).expect("append stale child");
        let verified = verify_chain_segment(&root, &scope, None, Some(&genesis.link_ref)).expect("verify stale head");
        assert_eq!(verified.decision, "fail");
        assert!(has_diagnostic(&verified, "stale-head"));
    }

    fn assert_missing_payload_diagnostic() {
        let root = temp_dir("chain-verify-missing-payload");
        let scope = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let value = chain_link_value(&sample_genesis_input(&scope.scope, &scope.id, &scope.epoch, "missing-payload"));
        let link = parse_chain_link(&value).expect("parse missing payload link");
        ledger::import_artifact(&root, &value).expect("raw import missing payload link");
        let verified = verify_chain_segment(&root, &scope, None, Some(&link.link_ref)).expect("verify missing payload");
        assert_eq!(verified.decision, "fail");
        assert!(has_diagnostic(&verified, "missing-payload"));
    }

    #[test]
    fn signed_chain_receipts_can_be_link_payloads_without_changing_subject_hashes() {
        let root = temp_dir("chain-signed-receipts");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            chain.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let append = append_chain_link(&root, &genesis_value).expect("append genesis");
        let append_subject_ref = canonical_hash(&append.receipt_value).expect("append receipt subject ref");
        assert_eq!(append_subject_ref, append.receipt_ref);

        let signed_append =
            sign_chain_receipt(&append.receipt_value, "node:local", "root", "key", &[]).expect("sign append receipt");
        let verified_signed_append =
            verify_signed_chain_receipt(&signed_append, "root", "key").expect("verify signed append receipt");
        assert_eq!(verified_signed_append.subject_ref, append.receipt_ref);
        assert_eq!(
            canonical_hash(&append.receipt_value).expect("append receipt ref after signing"),
            append.receipt_ref
        );

        let signed_append_ref = canonical_hash(&signed_append).expect("signed append receipt ref");
        ledger::import_artifact(&root, &signed_append).expect("import signed append receipt");
        let signed_chain = ChainScope::new("evidence-ledger", "signed-receipts", "epoch-1");
        let signed_payload_link_value = chain_link_value(&ChainLinkInput::genesis(
            signed_chain.clone(),
            signed_receipt_payload(signed_append_ref.clone()),
            Vec::new(),
            sample_producer(),
            ref_for("signed-receipt-link"),
        ));
        let linked = append_chain_link(&root, &signed_payload_link_value).expect("append signed receipt payload link");
        let linked_link = parse_chain_link(&signed_payload_link_value).expect("parse signed payload link");
        assert_eq!(linked_link.payload.artifact_ref, signed_append_ref);

        let verified_segment = verify_chain_segment(&root, &signed_chain, None, Some(&linked.link_ref))
            .expect("verify signed receipt chain segment");
        assert_eq!(verified_segment.decision, "pass");
        let signed_verify = sign_chain_receipt(
            &verified_segment.receipt_value,
            "node:local",
            "root",
            "key",
            std::slice::from_ref(&signed_append_ref),
        )
        .expect("sign verify receipt");
        let verified_signed_verify =
            verify_signed_chain_receipt(&signed_verify, "root", "key").expect("verify signed verify receipt");
        assert_eq!(verified_signed_verify.subject_ref, verified_segment.receipt_ref);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_chain_segment_append_no_gap_fork_and_anchor_properties(tc: TestCase) {
        let length = tc.draw(generators::integers::<usize>().min_value(1).max_value(5));
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let labels = (0..length).map(|index| format!("hegel-{salt}-payload-{index}")).collect::<Vec<_>>();
        let chain = ChainScope::new("evidence-ledger", format!("hegel-node-{salt}"), "epoch-1");

        let deterministic_first = deterministic_link_refs(&chain, &labels, salt);
        let deterministic_second = deterministic_link_refs(&chain, &labels, salt);
        assert_eq!(deterministic_first, deterministic_second);

        let root = temp_dir("chain-hegel-linear");
        let linear = append_linear_chain(&root, &chain, &labels);
        assert_eq!(linear.len(), length);
        let linear_head = linear.last().expect("non-empty linear chain");
        let no_gap = verify_chain_segment(&root, &chain, Some(&linear[0].link_ref), Some(&linear_head.link_ref))
            .expect("verify generated no-gap chain");
        assert_eq!(no_gap.decision, "pass");
        assert!(no_gap.diagnostics.is_empty());

        let fork_root = temp_dir("chain-hegel-fork");
        let fork_chain = ChainScope::new("evidence-ledger", format!("hegel-fork-node-{salt}"), "epoch-1");
        let fork_genesis = import_genesis_link(&fork_root, fork_chain.clone(), &format!("hegel-{salt}-fork-root"));
        let left = import_raw_link(
            &fork_root,
            &ChainLinkInput::append(
                &fork_genesis,
                stored_payload(&fork_root, &format!("hegel-{salt}-fork-left")),
                Vec::new(),
                sample_producer(),
                ref_for(&format!("hegel-{salt}-fork-left-input")),
            ),
        );
        let right = import_raw_link(
            &fork_root,
            &ChainLinkInput::append(
                &fork_genesis,
                stored_payload(&fork_root, &format!("hegel-{salt}-fork-right")),
                Vec::new(),
                sample_producer(),
                ref_for(&format!("hegel-{salt}-fork-right-input")),
            ),
        );
        let production_fork =
            verify_chain_segment(&fork_root, &fork_chain, Some(&fork_genesis.link_ref), Some(&left.link_ref))
                .expect("production fork verification");
        assert_eq!(production_fork.decision, "fail");
        assert!(has_diagnostic(&production_fork, "fork"));

        let diagnostic_fork = verify_chain_segment_with_policy(
            &fork_root,
            &fork_chain,
            Some(&fork_genesis.link_ref),
            Some(&left.link_ref),
            ChainForkPolicy::RetainForkEvidence,
        )
        .expect("diagnostic fork verification");
        assert_eq!(diagnostic_fork.decision, "pass");
        assert!(has_diagnostic(&diagnostic_fork, "fork"));
        assert_eq!(diagnostic_fork.verified_links, vec![fork_genesis.link_ref.clone(), left.link_ref.clone()]);

        let non_descending = verify_chain_segment(&fork_root, &fork_chain, Some(&left.link_ref), Some(&right.link_ref))
            .expect("non-descending anchor verification");
        assert_eq!(non_descending.decision, "fail");
        assert!(has_diagnostic(&non_descending, "anchor-descent"));
    }

    #[test]
    fn diagnostic_fork_policy_retains_evidence_while_verifying_selected_head() {
        let root = temp_dir("chain-diagnostic-fork");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis = import_genesis_link(&root, chain.clone(), "payload-a");
        let left = import_raw_link(
            &root,
            &ChainLinkInput::append(
                &genesis,
                stored_payload(&root, "payload-left"),
                Vec::new(),
                sample_producer(),
                ref_for("append-left"),
            ),
        );
        let right = import_raw_link(
            &root,
            &ChainLinkInput::append(
                &genesis,
                stored_payload(&root, "payload-right"),
                Vec::new(),
                sample_producer(),
                ref_for("append-right"),
            ),
        );

        let production = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&left.link_ref))
            .expect("production verify fork");
        assert_eq!(production.decision, "fail");
        assert!(has_diagnostic(&production, "fork"));

        let diagnostic = verify_chain_segment_with_policy(
            &root,
            &chain,
            Some(&genesis.link_ref),
            Some(&left.link_ref),
            ChainForkPolicy::RetainForkEvidence,
        )
        .expect("diagnostic verify fork");
        assert_eq!(diagnostic.decision, "pass");
        assert!(has_diagnostic(&diagnostic, "fork"));
        assert_eq!(diagnostic.verified_links, vec![genesis.link_ref.clone(), left.link_ref.clone()]);

        let index = build_chain_index(&root).expect("index fork evidence");
        let evidence_refs = index.fork_evidence_for_parent(&genesis.link_ref);
        assert!(!evidence_refs.is_empty());
        let evidence = index.fork_evidence_by_ref.get(&evidence_refs[0]).expect("indexed fork evidence");
        assert_eq!(evidence.parent_ref.as_deref(), Some(genesis.link_ref.as_str()));
        assert_eq!(evidence.selected_head.as_deref(), Some(left.link_ref.as_str()));
        assert_eq!(evidence.profile, ChainForkPolicy::RetainForkEvidence.profile());
        assert_eq!(evidence.decision, ChainForkPolicy::RetainForkEvidence.decision_for_fork());
        assert!(evidence.child_refs.contains(&left.link_ref));
        assert!(evidence.child_refs.contains(&right.link_ref));
    }

    #[test]
    fn chain_anchor_and_checkpoint_indexes_track_accepted_heads_and_freshness() {
        let root = temp_dir("chain-checkpoint");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            chain.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let genesis = parse_chain_link(&genesis_value).expect("parse genesis");
        append_chain_link(&root, &genesis_value).expect("append genesis");
        let second_value = chain_link_value(&ChainLinkInput::append(
            &genesis,
            stored_payload(&root, "payload-b"),
            Vec::new(),
            sample_producer(),
            ref_for("append-input"),
        ));
        let second = parse_chain_link(&second_value).expect("parse second");
        append_chain_link(&root, &second_value).expect("append second");

        let policy_ref = ref_for("checkpoint-policy");
        let membership_ref = ref_for("membership");
        let anchor = publish_chain_anchor(
            &root,
            &chain,
            &genesis.link_ref,
            std::slice::from_ref(&policy_ref),
            &sample_producer(),
        )
        .expect("publish anchor");
        let verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&second.link_ref))
            .expect("verify checkpoint range");
        assert_eq!(verified.decision, "pass");
        let checkpoint = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: second.link_ref.clone(),
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&root, &verified),
            policy_refs: vec![policy_ref],
            membership_refs: vec![membership_ref],
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect("accept checkpoint");

        let index = build_chain_index(&root).expect("index checkpoints");
        assert_eq!(index.anchors_for_chain(&chain), vec![anchor.anchor_ref.clone()]);
        assert_eq!(index.anchor_links_for_chain(&chain), vec![genesis.link_ref.clone()]);
        assert_eq!(index.checkpoints_for_chain(&chain), vec![checkpoint.checkpoint_ref.clone()]);
        assert_eq!(index.checkpoint_heads_for_chain(&chain), vec![second.link_ref.clone()]);
        assert_eq!(checkpoint.range_predicate_ref, checkpoint_range_predicate(&root, &verified));
        validate_chain_checkpoint_freshness(&root, &chain, &checkpoint.checkpoint_ref, Some(&second.link_ref))
            .expect("fresh checkpoint");

        let third_value = chain_link_value(&ChainLinkInput::append(
            &second,
            stored_payload(&root, "payload-c"),
            Vec::new(),
            sample_producer(),
            ref_for("append-third"),
        ));
        append_chain_link(&root, &third_value).expect("append third");
        let stale = validate_chain_checkpoint_freshness(&root, &chain, &checkpoint.checkpoint_ref, None)
            .expect_err("checkpoint becomes stale after head advances");
        assert!(stale.to_string().contains("stale"));
    }

    #[test]
    fn chain_checkpoint_rejects_mismatched_verify_receipt() {
        let root = temp_dir("chain-checkpoint-mismatch");
        let chain = ChainScope::new("evidence-ledger", "node-a", "epoch-1");
        let genesis_value = chain_link_value(&ChainLinkInput::genesis(
            chain.clone(),
            stored_payload(&root, "payload-a"),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        ));
        let genesis = parse_chain_link(&genesis_value).expect("parse genesis");
        append_chain_link(&root, &genesis_value).expect("append genesis");
        let verified = verify_chain_segment(&root, &chain, Some(&genesis.link_ref), Some(&genesis.link_ref))
            .expect("verify genesis range");
        let wrong_head = ref_for("wrong-head");
        let error = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: wrong_head,
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: checkpoint_range_predicate(&root, &verified),
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect_err("mismatched checkpoint rejected");
        assert!(error.to_string().contains("head"));

        let missing_predicate = accept_chain_checkpoint(&root, &ChainCheckpointInput {
            chain: chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: genesis.link_ref.clone(),
            head_ref: genesis.link_ref.clone(),
            verify_receipt_ref: verified.receipt_ref.clone(),
            range_predicate_ref: ref_for("missing-range-predicate"),
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect_err("missing range predicate rejected");
        assert!(missing_predicate.to_string().contains("predicate"));

        let case = MismatchCase {
            root: &root,
            chain: &chain,
            genesis: &genesis,
            verified: &verified,
        };
        assert_bad_subjects(&case);
        assert_wrong_kind(&case);
    }

    struct MismatchCase<'a> {
        root: &'a Path,
        chain: &'a ChainScope,
        genesis: &'a ChainLink,
        verified: &'a ChainVerify,
    }

    fn assert_bad_subjects(case: &MismatchCase<'_>) {
        let wrong_range_subjects = vec![ref_for("wrong-range-subject")];
        let checkpoint_context_refs = scope_context_refs(case.chain).expect("scope context");
        let predicate_checkpoint_checks = vec![ChainCheck::pass("checkpoint-range-coverage")];
        let tampered_predicate_value = chain_predicate_receipt_value(&ChainPredicateReceiptValueInput {
            predicate: CHECKPOINT_COVERS_RANGE_PREDICATE,
            decision: "pass",
            subject_refs: &wrong_range_subjects,
            input_refs: &case.verified.payload_refs,
            context_refs: &checkpoint_context_refs,
            checks: &predicate_checkpoint_checks,
        });
        let tampered_predicate_ref = ledger::import_artifact(case.root, &tampered_predicate_value)
            .expect("import tampered predicate")
            .artifact_ref;
        let fake_verify_link_refs = vec![case.genesis.link_ref.clone()];
        let fake_verify_diagnostics = Vec::new();
        let fake_verify_predicate_refs = vec![tampered_predicate_ref.clone()];
        let fake_verify_receipt = ChainVerifyReceiptValueInput {
            decision: "pass",
            chain: case.chain,
            anchor_ref: Some(&case.genesis.link_ref),
            expected_head: Some(&case.genesis.link_ref),
            discovered_heads: &fake_verify_link_refs,
            verified_links: &fake_verify_link_refs,
            payload_refs: &case.verified.payload_refs,
            diagnostics: &fake_verify_diagnostics,
        };
        let fake_verify_value = chain_verify_receipt_value_with_policy(&ChainVerifyReceiptPolicyValueInput {
            receipt: fake_verify_receipt,
            predicate_receipt_refs: &fake_verify_predicate_refs,
            fork_policy: ChainForkPolicy::RejectUnexpectedForks,
        });
        let fake_verify_ref = ledger::import_artifact(case.root, &fake_verify_value)
            .expect("import fake verify receipt")
            .artifact_ref;
        let tampered_predicate = accept_chain_checkpoint(case.root, &ChainCheckpointInput {
            chain: case.chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: case.genesis.link_ref.clone(),
            head_ref: case.genesis.link_ref.clone(),
            verify_receipt_ref: fake_verify_ref,
            range_predicate_ref: tampered_predicate_ref,
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect_err("tampered range predicate rejected");
        assert!(tampered_predicate.to_string().contains("subjects"));
    }

    fn assert_wrong_kind(case: &MismatchCase<'_>) {
        let wrong_predicate_ref = case
            .verified
            .predicate_receipt_refs
            .iter()
            .find(|predicate_ref| {
                let value = ledger::read_artifact(case.root, predicate_ref).expect("read predicate");
                parse_chain_predicate_receipt(&value).expect("parse predicate").predicate == SEGMENT_NO_GAP_PREDICATE
            })
            .expect("no-gap predicate")
            .clone();
        let wrong_predicate = accept_chain_checkpoint(case.root, &ChainCheckpointInput {
            chain: case.chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: case.genesis.link_ref.clone(),
            head_ref: case.genesis.link_ref.clone(),
            verify_receipt_ref: case.verified.receipt_ref.clone(),
            range_predicate_ref: wrong_predicate_ref,
            policy_refs: Vec::new(),
            membership_refs: Vec::new(),
            producer: sample_producer(),
            checks: checkpoint_checks(),
        })
        .expect_err("wrong range predicate rejected");
        assert!(wrong_predicate.to_string().contains("range predicate"));
    }

    fn sample_genesis(scope: &str, id: &str, epoch: &str, payload_label: &str) -> ChainLink {
        let input = sample_genesis_input(scope, id, epoch, payload_label);
        let link = parse_chain_link(&chain_link_value(&input)).expect("parse genesis");
        validate_genesis(&link).expect("validate genesis");
        link
    }

    fn sample_genesis_input(scope: &str, id: &str, epoch: &str, payload_label: &str) -> ChainLinkInput {
        ChainLinkInput::genesis(
            ChainScope::new(scope, id, epoch),
            sample_payload(payload_label),
            vec![ChainContextRef::new("policy", ref_for("policy"))],
            sample_producer(),
            ref_for("genesis-input"),
        )
    }

    fn sample_payload(label: &str) -> ChainPayload {
        ChainPayload::new("gate-receipt", ref_for(label), "molten.harness.gate-receipt.v1")
    }

    fn deterministic_link_refs(chain: &ChainScope, labels: &[String], salt: u64) -> Vec<String> {
        let mut previous = None;
        let mut refs = Vec::with_capacity(labels.len());
        for (index, label) in labels.iter().enumerate() {
            let input_ref = ref_for(&format!("hegel-{salt}-input-{index}"));
            let input = if let Some(previous) = &previous {
                ChainLinkInput::append(previous, sample_payload(label), Vec::new(), sample_producer(), input_ref)
            } else {
                ChainLinkInput::genesis(chain.clone(), sample_payload(label), Vec::new(), sample_producer(), input_ref)
            };
            let link = parse_chain_link(&chain_link_value(&input)).expect("parse deterministic link");
            refs.push(link.link_ref.clone());
            previous = Some(link);
        }
        refs
    }

    fn append_linear_chain(root: &Path, chain: &ChainScope, labels: &[String]) -> Vec<ChainLink> {
        let mut previous = None;
        let mut links = Vec::with_capacity(labels.len());
        for (index, label) in labels.iter().enumerate() {
            let input = if let Some(previous) = &previous {
                ChainLinkInput::append(
                    previous,
                    stored_payload(root, label),
                    Vec::new(),
                    sample_producer(),
                    ref_for(&format!("linear-input-{index}")),
                )
            } else {
                ChainLinkInput::genesis(
                    chain.clone(),
                    stored_payload(root, label),
                    Vec::new(),
                    sample_producer(),
                    ref_for(&format!("linear-input-{index}")),
                )
            };
            let value = chain_link_value(&input);
            append_chain_link(root, &value).expect("append linear chain link");
            let link = parse_chain_link(&value).expect("parse linear chain link");
            previous = Some(link.clone());
            links.push(link);
        }
        links
    }

    fn stored_payload(root: &Path, label: &str) -> ChainPayload {
        let artifact = record("test-payload", vec![string(label)]);
        let imported = ledger::import_artifact(root, &artifact).expect("import payload");
        ChainPayload::new("test-payload", imported.artifact_ref, "molten.test.payload.v1")
    }

    fn import_genesis_link(root: &Path, chain: ChainScope, payload_label: &str) -> ChainLink {
        let input = ChainLinkInput::genesis(
            chain,
            stored_payload(root, payload_label),
            Vec::new(),
            sample_producer(),
            ref_for("genesis-input"),
        );
        import_raw_link(root, &input)
    }

    fn import_raw_link(root: &Path, input: &ChainLinkInput) -> ChainLink {
        let value = chain_link_value(input);
        let link = parse_chain_link(&value).expect("parse raw link");
        ledger::import_artifact(root, &value).expect("raw import link");
        link
    }

    fn has_diagnostic(verify: &ChainVerify, kind: &str) -> bool {
        verify.diagnostics.iter().any(|diagnostic| diagnostic.kind == kind)
    }

    fn checkpoint_range_predicate(root: &Path, verify: &ChainVerify) -> String {
        verify
            .predicate_receipt_refs
            .iter()
            .find(|predicate_ref| {
                let value = ledger::read_artifact(root, predicate_ref).expect("read predicate receipt");
                parse_chain_predicate_receipt(&value).expect("parse predicate receipt").predicate
                    == CHECKPOINT_COVERS_RANGE_PREDICATE
            })
            .cloned()
            .expect("checkpoint range predicate ref")
    }

    fn predicate_names(root: &Path, predicate_refs: &[String]) -> Vec<String> {
        predicate_refs
            .iter()
            .map(|predicate_ref| {
                let value = ledger::read_artifact(root, predicate_ref).expect("read predicate receipt");
                parse_chain_predicate_receipt(&value).expect("parse predicate receipt").predicate
            })
            .collect()
    }

    fn checkpoint_checks() -> Vec<ChainCheck> {
        vec![
            ChainCheck::pass("raft-control-plane-command"),
            ChainCheck::pass("verified-range"),
            ChainCheck::pass("checkpoint-freshness"),
        ]
    }

    fn sample_producer() -> ChainProducer {
        ChainProducer::new("node:local", ref_for("producer-key"))
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
