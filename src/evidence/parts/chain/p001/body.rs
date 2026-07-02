
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
    pub receipt_value: IoValue,
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
    pub receipt_value: IoValue,
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
    value: &'a IoValue,
    chain: &'a ChainScope,
    anchor_link_ref: &'a str,
    head_ref: &'a str,
    range_predicate_ref: &'a str,
}

struct RangeBindingInput<'a> {
    root: &'a Path,
    value: &'a IoValue,
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
    payload_refs: OrderedSet<String>,
    seen_refs: OrderedSet<String>,
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
