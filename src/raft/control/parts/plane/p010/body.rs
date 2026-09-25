
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub operation: String,
    pub command_ref: String,
    pub state_before_ref: String,
    pub state_after_ref: Option<String>,
    pub duplicate: bool,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryProposal {
    pub decision: String,
    pub duplicate: bool,
    pub envelope: RaftCommandEnvelope,
    pub predicates: Vec<RaftPredicateReceipt>,
    pub log_entry: Option<RaftLogEntry>,
    pub commit_receipt: RaftCommitReceipt,
    pub registry_receipt: ControlRegistryReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryTransition {
    pub proposal: ControlRegistryProposal,
    pub state_after: Option<ControlRegistryState>,
    pub next_committed_index: u64,
    pub next_last_log_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryRuntime {
    pub manifest: RaftGroupManifest,
    pub term: u64,
    pub committed_index: u64,
    pub last_log_ref: Option<String>,
    pub state: ControlRegistryState,
    pub log_entries: Vec<RaftLogEntry>,
    pub commit_receipts: Vec<RaftCommitReceipt>,
    pub registry_receipts: Vec<ControlRegistryReceipt>,
    pub predicate_receipts: Vec<RaftPredicateReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryReadInput {
    pub state: IoValue,
    pub group_ref: String,
    pub committed_term: u64,
    pub committed_index: u64,
    pub read_index: u64,
    pub read_consistency_mode: String,
    pub namespace: String,
    pub name: String,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftReadReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub read_consistency_mode: String,
    pub target_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftSnapshotInput {
    pub group_ref: String,
    pub term: u64,
    pub index: u64,
    pub state: IoValue,
    pub log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftSnapshot {
    pub snapshot_ref: String,
    pub group_ref: String,
    pub term: u64,
    pub index: u64,
    pub state: ControlRegistryState,
    pub content_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftRecoveryInput {
    pub group_ref: String,
    pub snapshot: IoValue,
    pub log_entries: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftRecoveryReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub restored_state_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}
