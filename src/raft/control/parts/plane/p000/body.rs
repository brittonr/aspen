type IoValue = preserves::IOValue;
use redb::ReadableDatabase;
use redb::ReadableTableMetadata;

type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type Path = std::path::Path;
type Value<T> = preserves::Value<T>;
type Database = redb::Database;
type TableDefinition<K, V> = redb::TableDefinition<'static, K, V>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

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

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

const CONTROL_REGISTRY_STATE_MACHINE: &str = "control-registry-v1";
const READ_MODE_READ_INDEX: &str = "read-index";
pub const READ_CONSISTENCY_LINEARIZABLE: &str = "linearizable";
pub const READ_CONSISTENCY_LOCAL_STALE: &str = "local-stale";
pub const CONSENSUS_PROFILE_RAFT: &str = "raft";
pub const CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL: &str = "leaderless-quorum-experimental";
const CONSENSUS_PROFILE_VERSION_RAFT: &str = "raft-production-v1";
const CONSENSUS_PROFILE_VERSION_LEADERLESS_EXPERIMENTAL: &str = "leaderless-quorum-experimental-v1";
const QUORUM_RULE_MAJORITY_READ_INDEX: &str = "majority-read-index";
const QUORUM_RULE_LEADERLESS_MAJORITY: &str = "leaderless-majority";
const PRODUCTION_STATUS_ADMITTED: &str = "admitted-production";
const PRODUCTION_STATUS_EXPERIMENTAL: &str = "experimental-denied-production";
const DEFAULT_GROUP_ID: &str = "raft:control";
const STORE_FILE: &str = "control-registry.redb";

const STORE_LOGS: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_control_logs_v1");
const STORE_SNAPSHOTS: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_control_snapshots_v1");
const STORE_SESSIONS: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_control_sessions_v1");
const STORE_RECEIPTS: TableDefinition<&str, &[u8]> = TableDefinition::new("raft_control_receipts_v1");

const MAX_RAFT_MEMBERS: usize = 32;
const MAX_RAFT_REFS: usize = 4096;
const MAX_RAFT_COMMANDS: usize = 128;
const MAX_RAFT_ENTRIES: usize = 4096;
const MAX_RAFT_DIAGNOSTICS: usize = 256;
const MAX_RAFT_STORE_SCAN: usize = 100_000;
const RAFT_GROUP_MANIFEST_FIELD_COUNT: usize = 18;

const _: () = assert!(MAX_RAFT_MEMBERS <= 1024);
const _: () = assert!(MAX_RAFT_REFS <= 100_000);
const _: () = assert!(MAX_RAFT_COMMANDS <= 10_000);
const _: () = assert!(MAX_RAFT_ENTRIES <= 100_000);
const _: () = assert!(MAX_RAFT_DIAGNOSTICS <= 10_000);
const _: () = assert!(MAX_RAFT_STORE_SCAN <= 1_000_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftGroupManifestInput {
    pub group_id: String,
    pub members: Vec<String>,
    pub state_machine: String,
    pub command_schemas: Vec<String>,
    pub read_mode: String,
    pub snapshot_policy_ref: String,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusAlgorithmProfileInput {
    pub algorithm_profile: String,
    pub admitted_profile_version: String,
    pub read_consistency_support: Vec<String>,
    pub quorum_rule: String,
    pub membership_policy_refs: Vec<String>,
    pub placement_ref: Option<String>,
    pub fault_model_caveats: Vec<String>,
    pub required_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftGroupManifest {
    pub manifest_ref: String,
    pub group_id: String,
    pub members: Vec<String>,
    pub state_machine: String,
    pub command_schemas: Vec<String>,
    pub read_mode: String,
    pub snapshot_policy_ref: String,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub algorithm_profile: String,
    pub admitted_profile_version: String,
    pub read_consistency_support: Vec<String>,
    pub quorum_rule: String,
    pub membership_policy_refs: Vec<String>,
    pub placement_ref: Option<String>,
    pub fault_model_caveats: Vec<String>,
    pub required_evidence_refs: Vec<String>,
    pub production_status: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryCommandInput {
    pub operation: String,
    pub namespace: String,
    pub name: String,
    pub target_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryCommand {
    pub command_ref: String,
    pub operation: String,
    pub namespace: String,
    pub name: String,
    pub target_ref: Option<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftCommandEnvelopeInput {
    pub group_ref: String,
    pub client_session: String,
    pub sequence: u64,
    pub command: IoValue,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftCommandEnvelope {
    pub envelope_ref: String,
    pub group_ref: String,
    pub client_session: String,
    pub sequence: u64,
    pub command: IoValue,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ControlRegistryKey {
    pub namespace: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryEntry {
    pub namespace: String,
    pub name: String,
    pub target_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSessionRecord {
    pub client_session: String,
    pub sequence: u64,
    pub result_command_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ClientSequenceKey {
    client_session: String,
    sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryState {
    pub state_ref: String,
    pub entries: Vec<ControlRegistryEntry>,
    pub client_sessions: Vec<ClientSessionRecord>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftPredicateReceipt {
    pub predicate_ref: String,
    pub predicate: String,
    pub decision: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftLogEntry {
    pub entry_ref: String,
    pub group_ref: String,
    pub term: u64,
    pub index: u64,
    pub prior_log_ref: Option<String>,
    pub command_ref: String,
    pub command: IoValue,
    pub append_predicate_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftCommitReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub group_ref: String,
    pub term: u64,
    pub index: u64,
    pub command_ref: String,
    pub log_entry_ref: Option<String>,
    pub value: IoValue,
}

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
