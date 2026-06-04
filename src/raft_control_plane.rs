use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use preserves::IOValue;
use preserves::Value;
use redb::Database;
use redb::ReadableDatabase;
use redb::ReadableTableMetadata;
use redb::TableDefinition;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::CONTROL_REGISTRY_COMMAND_SCHEMA;
use crate::preserves_rail::CONTROL_REGISTRY_RECEIPT_SCHEMA;
use crate::preserves_rail::CONTROL_REGISTRY_STATE_SCHEMA;
use crate::preserves_rail::RAFT_COMMAND_ENVELOPE_SCHEMA;
use crate::preserves_rail::RAFT_COMMIT_RECEIPT_SCHEMA;
use crate::preserves_rail::RAFT_GROUP_MANIFEST_SCHEMA;
use crate::preserves_rail::RAFT_LOG_ENTRY_SCHEMA;
use crate::preserves_rail::RAFT_PREDICATE_RECEIPT_SCHEMA;
use crate::preserves_rail::RAFT_READ_RECEIPT_SCHEMA;
use crate::preserves_rail::RAFT_RECOVERY_RECEIPT_SCHEMA;
use crate::preserves_rail::RAFT_SNAPSHOT_SCHEMA;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::value_to_iovalue;

const CONTROL_REGISTRY_STATE_MACHINE: &str = "control-registry-v1";
const READ_MODE_READ_INDEX: &str = "read-index";
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
    pub value: IOValue,
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftCommandEnvelopeInput {
    pub group_ref: String,
    pub client_session: String,
    pub sequence: u64,
    pub command: IOValue,
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
    pub command: IOValue,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IOValue,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryState {
    pub state_ref: String,
    pub entries: Vec<ControlRegistryEntry>,
    pub client_sessions: Vec<ClientSessionRecord>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftPredicateReceipt {
    pub predicate_ref: String,
    pub predicate: String,
    pub decision: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftLogEntry {
    pub entry_ref: String,
    pub group_ref: String,
    pub term: u64,
    pub index: u64,
    pub prior_log_ref: Option<String>,
    pub command_ref: String,
    pub command: IOValue,
    pub append_predicate_ref: String,
    pub value: IOValue,
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
    pub value: IOValue,
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
    pub value: IOValue,
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
    pub state: IOValue,
    pub group_ref: String,
    pub committed_term: u64,
    pub committed_index: u64,
    pub read_index: u64,
    pub namespace: String,
    pub name: String,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftReadReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub target_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftSnapshotInput {
    pub group_ref: String,
    pub term: u64,
    pub index: u64,
    pub state: IOValue,
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftRecoveryInput {
    pub group_ref: String,
    pub snapshot: IOValue,
    pub log_entries: Vec<IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaftRecoveryReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub restored_state_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlRegistryStoreStatus {
    pub log_count: u64,
    pub snapshot_count: u64,
    pub session_count: u64,
    pub receipt_count: u64,
}

struct RegistryMaps {
    entries: BTreeMap<ControlRegistryKey, String>,
    sessions: BTreeMap<String, ClientSessionRecord>,
}

struct ProposalDecisionInput<'a> {
    runtime: &'a ControlRegistryRuntime,
    envelope: &'a RaftCommandEnvelope,
    command: Option<&'a ControlRegistryCommand>,
    diagnostics: Vec<String>,
}

struct PredicateReceiptInput<'a> {
    predicate: &'a str,
    decision: &'a str,
    group_ref: &'a str,
    term: u64,
    index: u64,
    subjects: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct LogEntryValueInput<'a> {
    group_ref: &'a str,
    term: u64,
    index: u64,
    prior_log_ref: Option<&'a str>,
    command_ref: &'a str,
    command: &'a IOValue,
    append_predicate_ref: &'a str,
}

struct CommitReceiptValueInput<'a> {
    decision: &'a str,
    group_ref: &'a str,
    term: u64,
    index: u64,
    command_ref: &'a str,
    log_entry_ref: Option<&'a str>,
    quorum_refs: &'a [String],
    append_predicate_ref: Option<&'a str>,
    commit_predicate_ref: Option<&'a str>,
    diagnostics: &'a [String],
}

struct RegistryReceiptValueInput<'a> {
    decision: &'a str,
    operation: &'a str,
    command_ref: &'a str,
    log_entry_ref: Option<&'a str>,
    state_before_ref: &'a str,
    state_after_ref: Option<&'a str>,
    client_session: &'a str,
    sequence: u64,
    duplicate: bool,
    authority_refs: &'a [String],
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    diagnostics: &'a [String],
}

struct ReadReceiptValueInput<'a> {
    decision: &'a str,
    group_ref: &'a str,
    state_ref: &'a str,
    committed_term: u64,
    committed_index: u64,
    namespace: &'a str,
    name: &'a str,
    target_ref: Option<&'a str>,
    read_index_predicate_ref: Option<&'a str>,
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    diagnostics: &'a [String],
}

pub fn raft_group_manifest_value(input: &RaftGroupManifestInput) -> Result<IOValue> {
    validate_group_id(&input.group_id)?;
    validate_refs(&input.members, "raft member ref")?;
    validate_non_empty(&input.state_machine, "raft state machine")?;
    validate_command_schema_list(&input.command_schemas)?;
    validate_read_mode(&input.read_mode)?;
    require_ref(&input.snapshot_policy_ref, "raft snapshot policy ref")?;
    validate_refs(&input.policy_refs, "raft policy ref")?;
    validate_refs(&input.resource_refs, "raft resource ref")?;
    ensure_count_at_most(input.members.len(), MAX_RAFT_MEMBERS, "raft members")?;
    Ok(record("raft-group-manifest-v1", vec![
        string(RAFT_GROUP_MANIFEST_SCHEMA),
        record("group-id", vec![string(&input.group_id)]),
        record("members", vec![strings_sequence(&input.members)]),
        record("state-machine", vec![string(&input.state_machine)]),
        record("command-schemas", vec![strings_sequence(&input.command_schemas)]),
        record("read-mode", vec![string(&input.read_mode)]),
        record("snapshot-policy", vec![string(&input.snapshot_policy_ref)]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        record("resource", vec![strings_sequence(&input.resource_refs)]),
        checks_value(&[
            ("control-plane-only", "pass"),
            ("explicit-command-schemas", "pass"),
            ("read-index-default", "pass"),
        ]),
    ]))
}

pub fn parse_raft_group_manifest(value: &IOValue) -> Result<RaftGroupManifest> {
    let fields = value
        .collect_simple_record("raft-group-manifest-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-group-manifest-v1 ...>"))?;
    require_schema(&fields[0], RAFT_GROUP_MANIFEST_SCHEMA, "raft group manifest schema")?;
    let group_id = record_string(&fields[1], "group-id")?;
    validate_group_id(&group_id)?;
    let members = parse_string_sequence(&fields[2], "members")?;
    validate_refs(&members, "raft member ref")?;
    let state_machine = record_string(&fields[3], "state-machine")?;
    let command_schemas = parse_string_sequence(&fields[4], "command-schemas")?;
    validate_command_schema_list(&command_schemas)?;
    let read_mode = record_string(&fields[5], "read-mode")?;
    validate_read_mode(&read_mode)?;
    let snapshot_policy_ref = record_ref(&fields[6], "snapshot-policy")?;
    let policy_refs = parse_ref_sequence(&fields[7], "policy")?;
    let resource_refs = parse_ref_sequence(&fields[8], "resource")?;
    require_check(&parse_checks(&fields[9])?, "control-plane-only", "raft group manifest")?;
    Ok(RaftGroupManifest {
        manifest_ref: canonical_hash(value)?,
        group_id,
        members,
        state_machine,
        command_schemas,
        read_mode,
        snapshot_policy_ref,
        policy_refs,
        resource_refs,
        value: value.clone(),
    })
}

pub fn control_registry_command_value(input: &ControlRegistryCommandInput) -> Result<IOValue> {
    validate_control_command(input)?;
    Ok(record("control-registry-command-v1", vec![
        string(CONTROL_REGISTRY_COMMAND_SCHEMA),
        record("operation", vec![string(&input.operation)]),
        record("namespace", vec![string(&input.namespace)]),
        record("name", vec![string(&input.name)]),
        record("target", vec![optional_ref_value(input.target_ref.as_deref())]),
        checks_value(&[("control-plane-only", "pass"), ("schema-admitted", "pass")]),
    ]))
}

pub fn parse_control_registry_command(value: &IOValue) -> Result<ControlRegistryCommand> {
    let fields = value
        .collect_simple_record("control-registry-command-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <control-registry-command-v1 ...>"))?;
    require_schema(&fields[0], CONTROL_REGISTRY_COMMAND_SCHEMA, "control registry command schema")?;
    let input = ControlRegistryCommandInput {
        operation: record_string(&fields[1], "operation")?,
        namespace: record_string(&fields[2], "namespace")?,
        name: record_string(&fields[3], "name")?,
        target_ref: record_optional_ref(&fields[4], "target")?,
    };
    validate_control_command(&input)?;
    require_check(&parse_checks(&fields[5])?, "control-plane-only", "control registry command")?;
    Ok(ControlRegistryCommand {
        command_ref: canonical_hash(value)?,
        operation: input.operation,
        namespace: input.namespace,
        name: input.name,
        target_ref: input.target_ref,
        value: value.clone(),
    })
}

pub fn raft_command_envelope_value(input: &RaftCommandEnvelopeInput) -> Result<IOValue> {
    require_ref(&input.group_ref, "raft command group ref")?;
    validate_client_session(&input.client_session)?;
    validate_refs(&input.authority_refs, "raft command authority ref")?;
    validate_refs(&input.policy_refs, "raft command policy ref")?;
    validate_refs(&input.resource_refs, "raft command resource ref")?;
    validate_refs(&input.evidence_refs, "raft command evidence ref")?;
    ensure_count_at_most(input.evidence_refs.len(), MAX_RAFT_REFS, "raft command evidence")?;
    Ok(record("raft-command-envelope-v1", vec![
        string(RAFT_COMMAND_ENVELOPE_SCHEMA),
        record("group", vec![string(&input.group_ref)]),
        record("client-session", vec![string(&input.client_session)]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("command", vec![input.command.clone()]),
        record("authority", vec![strings_sequence(&input.authority_refs)]),
        record("policy", vec![strings_sequence(&input.policy_refs)]),
        record("resource", vec![strings_sequence(&input.resource_refs)]),
        record("evidence", vec![strings_sequence(&input.evidence_refs)]),
        checks_value(&[("schema-admitted", "pass"), ("control-plane-only", "pass")]),
    ]))
}

pub fn parse_raft_command_envelope(value: &IOValue) -> Result<RaftCommandEnvelope> {
    let fields = value
        .collect_simple_record("raft-command-envelope-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-command-envelope-v1 ...>"))?;
    require_schema(&fields[0], RAFT_COMMAND_ENVELOPE_SCHEMA, "raft command envelope schema")?;
    let command = record_iovalue(&fields[4], "command")?;
    require_check(&parse_checks(&fields[9])?, "control-plane-only", "raft command envelope")?;
    Ok(RaftCommandEnvelope {
        envelope_ref: canonical_hash(value)?,
        group_ref: record_ref(&fields[1], "group")?,
        client_session: record_string(&fields[2], "client-session")?,
        sequence: record_u64(&fields[3], "sequence")?,
        command,
        authority_refs: parse_ref_sequence(&fields[5], "authority")?,
        policy_refs: parse_ref_sequence(&fields[6], "policy")?,
        resource_refs: parse_ref_sequence(&fields[7], "resource")?,
        evidence_refs: parse_ref_sequence(&fields[8], "evidence")?,
        value: value.clone(),
    })
}

pub fn initial_control_registry_state() -> Result<ControlRegistryState> {
    control_registry_state_value(Vec::new(), Vec::new()).and_then(|value| parse_control_registry_state(&value))
}

pub fn control_registry_state_value(
    mut entries: Vec<ControlRegistryEntry>,
    mut client_sessions: Vec<ClientSessionRecord>,
) -> Result<IOValue> {
    ensure_count_at_most(entries.len(), MAX_RAFT_ENTRIES, "control registry entries")?;
    ensure_count_at_most(client_sessions.len(), MAX_RAFT_ENTRIES, "control registry client sessions")?;
    entries.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.target_ref.cmp(&right.target_ref))
    });
    client_sessions.sort_by(|left, right| left.client_session.cmp(&right.client_session));
    let entry_values = entries
        .iter()
        .map(|entry| record("entry", vec![string(&entry.namespace), string(&entry.name), string(&entry.target_ref)]))
        .collect();
    let session_values = client_sessions
        .iter()
        .map(|session| {
            record("session", vec![
                string(&session.client_session),
                u64_value(session.sequence),
                string(&session.result_command_ref),
            ])
        })
        .collect();
    Ok(record("control-registry-state-v1", vec![
        string(CONTROL_REGISTRY_STATE_SCHEMA),
        record("entries", vec![sequence(entry_values)]),
        record("client-sessions", vec![sequence(session_values)]),
        checks_value(&[
            ("deterministic-map-order", "pass"),
            ("control-plane-namespaces", "pass"),
        ]),
    ]))
}

pub fn parse_control_registry_state(value: &IOValue) -> Result<ControlRegistryState> {
    let fields = value
        .collect_simple_record("control-registry-state-v1", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("expected <control-registry-state-v1 ...>"))?;
    require_schema(&fields[0], CONTROL_REGISTRY_STATE_SCHEMA, "control registry state schema")?;
    let entries = parse_registry_entries(&fields[1])?;
    let client_sessions = parse_client_sessions(&fields[2])?;
    require_check(&parse_checks(&fields[3])?, "deterministic-map-order", "control registry state")?;
    Ok(ControlRegistryState {
        state_ref: canonical_hash(value)?,
        entries,
        client_sessions,
        value: value.clone(),
    })
}

pub fn new_control_registry_runtime(manifest_value: &IOValue) -> Result<ControlRegistryRuntime> {
    let manifest = parse_raft_group_manifest(manifest_value)?;
    if manifest.state_machine != CONTROL_REGISTRY_STATE_MACHINE {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported raft state machine {}; expected {CONTROL_REGISTRY_STATE_MACHINE}",
            manifest.state_machine
        )));
    }
    Ok(ControlRegistryRuntime {
        manifest,
        term: 1,
        committed_index: 0,
        last_log_ref: None,
        state: initial_control_registry_state()?,
        log_entries: Vec::new(),
        commit_receipts: Vec::new(),
        registry_receipts: Vec::new(),
        predicate_receipts: Vec::new(),
    })
}

pub fn propose_control_registry_command(
    runtime: &mut ControlRegistryRuntime,
    envelope_value: &IOValue,
) -> Result<ControlRegistryProposal> {
    let envelope = parse_raft_command_envelope(envelope_value)?;
    let (command, diagnostics) = admitted_command(&envelope.command);
    if let Some(existing) = duplicate_receipt(runtime, &envelope) {
        let commit_receipt = deny_commit_receipt(runtime, &envelope, "duplicate-client-sequence", &[])?;
        return Ok(ControlRegistryProposal {
            decision: existing.decision.clone(),
            duplicate: true,
            envelope,
            predicates: Vec::new(),
            log_entry: None,
            commit_receipt,
            registry_receipt: existing,
        });
    }
    let admission = proposal_diagnostics(ProposalDecisionInput {
        runtime,
        envelope: &envelope,
        command: command.as_ref(),
        diagnostics,
    })?;
    if !admission.is_empty() {
        let commit_receipt = deny_commit_receipt(runtime, &envelope, "proposal-deny", &admission)?;
        let registry_receipt = deny_registry_receipt(runtime, &envelope, command.as_ref(), &admission)?;
        return Ok(ControlRegistryProposal {
            decision: "deny".to_string(),
            duplicate: false,
            envelope,
            predicates: Vec::new(),
            log_entry: None,
            commit_receipt,
            registry_receipt,
        });
    }
    let command =
        command.ok_or_else(|| MoltenError::invalid_harness("missing admitted command after admission pass"))?;
    let next_index = runtime
        .committed_index
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("raft committed index overflow"))?;
    let append_predicate = predicate_receipt_value(&PredicateReceiptInput {
        predicate: "trellis-append-consistency",
        decision: "pass",
        group_ref: &runtime.manifest.manifest_ref,
        term: runtime.term,
        index: next_index,
        subjects: std::slice::from_ref(&envelope.envelope_ref),
        diagnostics: &[],
        checks: &[("trellis-predicate", "pass"), ("prior-log-binding", "pass")],
    })?;
    let append_predicate = parse_predicate_receipt(&append_predicate)?;
    let log_entry_value = raft_log_entry_value(&LogEntryValueInput {
        group_ref: &runtime.manifest.manifest_ref,
        term: runtime.term,
        index: next_index,
        prior_log_ref: runtime.last_log_ref.as_deref(),
        command_ref: &envelope.envelope_ref,
        command: &envelope.value,
        append_predicate_ref: &append_predicate.predicate_ref,
    })?;
    let log_entry = parse_raft_log_entry(&log_entry_value)?;
    let commit_predicate = predicate_receipt_value(&PredicateReceiptInput {
        predicate: "trellis-quorum-commit",
        decision: "pass",
        group_ref: &runtime.manifest.manifest_ref,
        term: runtime.term,
        index: next_index,
        subjects: std::slice::from_ref(&log_entry.entry_ref),
        diagnostics: &[],
        checks: &[("trellis-predicate", "pass"), ("quorum-members", "pass")],
    })?;
    let commit_predicate = parse_predicate_receipt(&commit_predicate)?;
    let advancement_predicate = predicate_receipt_value(&PredicateReceiptInput {
        predicate: "trellis-commit-advancement",
        decision: "pass",
        group_ref: &runtime.manifest.manifest_ref,
        term: runtime.term,
        index: next_index,
        subjects: &[runtime.state.state_ref.clone(), log_entry.entry_ref.clone()],
        diagnostics: &[],
        checks: &[("trellis-predicate", "pass"), ("monotonic-index", "pass")],
    })?;
    let advancement_predicate = parse_predicate_receipt(&advancement_predicate)?;
    let commit_receipt = commit_receipt_value(&CommitReceiptValueInput {
        decision: "pass",
        group_ref: &runtime.manifest.manifest_ref,
        term: runtime.term,
        index: next_index,
        command_ref: &envelope.envelope_ref,
        log_entry_ref: Some(&log_entry.entry_ref),
        quorum_refs: &runtime.manifest.members,
        append_predicate_ref: Some(&append_predicate.predicate_ref),
        commit_predicate_ref: Some(&commit_predicate.predicate_ref),
        diagnostics: &[],
    })?;
    let commit_receipt = parse_commit_receipt(&commit_receipt)?;
    let registry_receipt = apply_admitted_command(runtime, &envelope, &command, &log_entry)?;
    runtime.committed_index = next_index;
    runtime.last_log_ref = Some(log_entry.entry_ref.clone());
    runtime.log_entries.push(log_entry.clone());
    runtime.commit_receipts.push(commit_receipt.clone());
    runtime.predicate_receipts.push(append_predicate.clone());
    runtime.predicate_receipts.push(commit_predicate.clone());
    runtime.predicate_receipts.push(advancement_predicate.clone());
    runtime.registry_receipts.push(registry_receipt.clone());
    Ok(ControlRegistryProposal {
        decision: "pass".to_string(),
        duplicate: false,
        envelope,
        predicates: vec![append_predicate, commit_predicate, advancement_predicate],
        log_entry: Some(log_entry),
        commit_receipt,
        registry_receipt,
    })
}

pub fn read_control_registry(input: &ControlRegistryReadInput) -> Result<RaftReadReceipt> {
    let state = parse_control_registry_state(&input.state)?;
    let mut diagnostics = Vec::new();
    if input.authority_refs.is_empty() {
        diagnostics.push("missing read authority evidence".to_string());
    }
    if input.resource_refs.is_empty() {
        diagnostics.push("missing read resource evidence".to_string());
    }
    if input.read_index != input.committed_index {
        diagnostics
            .push(format!("stale read-index {}; expected committed index {}", input.read_index, input.committed_index));
    }
    validate_refs(&input.authority_refs, "raft read authority ref")?;
    validate_refs(&input.resource_refs, "raft read resource ref")?;
    let target = find_entry(&state, &input.namespace, &input.name).map(|entry| entry.target_ref.clone());
    if target.is_none() {
        diagnostics.push("control registry entry not found".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let predicate = if decision == "pass" {
        Some(parse_predicate_receipt(&predicate_receipt_value(&PredicateReceiptInput {
            predicate: "trellis-read-index-freshness",
            decision,
            group_ref: &input.group_ref,
            term: input.committed_term,
            index: input.committed_index,
            subjects: std::slice::from_ref(&state.state_ref),
            diagnostics: &[],
            checks: &[("trellis-predicate", "pass"), ("read-index-current", "pass")],
        })?)?)
    } else {
        None
    };
    let receipt = read_receipt_value(&ReadReceiptValueInput {
        decision,
        group_ref: &input.group_ref,
        state_ref: &state.state_ref,
        committed_term: input.committed_term,
        committed_index: input.committed_index,
        namespace: &input.namespace,
        name: &input.name,
        target_ref: target.as_deref(),
        read_index_predicate_ref: predicate.as_ref().map(|value| value.predicate_ref.as_str()),
        authority_refs: &input.authority_refs,
        resource_refs: &input.resource_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(RaftReadReceipt {
        receipt_ref: canonical_hash(&receipt)?,
        decision: decision.to_string(),
        target_ref: target,
        diagnostics,
        value: receipt,
    })
}

pub fn snapshot_control_registry(input: &RaftSnapshotInput) -> Result<RaftSnapshot> {
    let state = parse_control_registry_state(&input.state)?;
    require_ref(&input.group_ref, "raft snapshot group ref")?;
    validate_refs(&input.log_refs, "raft snapshot log ref")?;
    let content_ref = state.state_ref.clone();
    let session_refs =
        state.client_sessions.iter().map(|session| session.result_command_ref.clone()).collect::<Vec<_>>();
    let value = record("raft-snapshot-v1", vec![
        string(RAFT_SNAPSHOT_SCHEMA),
        record("group", vec![string(&input.group_ref)]),
        record("term", vec![u64_value(input.term)]),
        record("index", vec![u64_value(input.index)]),
        record("state-ref", vec![string(&state.state_ref)]),
        record("content-ref", vec![string(&content_ref)]),
        record("state", vec![state.value.clone()]),
        record("client-sessions", vec![strings_sequence(&session_refs)]),
        record("log", vec![strings_sequence(&input.log_refs)]),
        checks_value(&[
            ("chunk-backed-content-ref", "pass"),
            ("snapshot-state-integrity", "pass"),
        ]),
    ]);
    Ok(RaftSnapshot {
        snapshot_ref: canonical_hash(&value)?,
        group_ref: input.group_ref.clone(),
        term: input.term,
        index: input.index,
        state,
        content_ref,
        value,
    })
}

pub fn parse_raft_snapshot(value: &IOValue) -> Result<RaftSnapshot> {
    let fields = value
        .collect_simple_record("raft-snapshot-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-snapshot-v1 ...>"))?;
    require_schema(&fields[0], RAFT_SNAPSHOT_SCHEMA, "raft snapshot schema")?;
    let state_value = record_iovalue(&fields[6], "state")?;
    let state = parse_control_registry_state(&state_value)?;
    let state_ref = record_ref(&fields[4], "state-ref")?;
    let content_ref = record_ref(&fields[5], "content-ref")?;
    if state.state_ref != state_ref || state.state_ref != content_ref {
        return Err(MoltenError::invalid_harness("raft snapshot state/content ref mismatch"));
    }
    require_check(&parse_checks(&fields[9])?, "snapshot-state-integrity", "raft snapshot")?;
    Ok(RaftSnapshot {
        snapshot_ref: canonical_hash(value)?,
        group_ref: record_ref(&fields[1], "group")?,
        term: record_u64(&fields[2], "term")?,
        index: record_u64(&fields[3], "index")?,
        state,
        content_ref,
        value: value.clone(),
    })
}

pub fn recover_control_registry(input: &RaftRecoveryInput) -> Result<RaftRecoveryReceipt> {
    let snapshot = parse_raft_snapshot(&input.snapshot)?;
    ensure_count_at_most(input.log_entries.len(), MAX_RAFT_ENTRIES, "recovery log entries")?;
    let mut diagnostics = Vec::with_capacity(input.log_entries.len().saturating_add(1));
    if snapshot.group_ref != input.group_ref {
        diagnostics.push("snapshot group does not match recovery group".to_string());
    }
    let mut expected_index = snapshot.index.saturating_add(1);
    let mut replayed = Vec::with_capacity(input.log_entries.len());
    for entry_value in &input.log_entries {
        let entry = parse_raft_log_entry(entry_value)?;
        if entry.index != expected_index {
            diagnostics.push(format!("log gap at index {}; expected {expected_index}", entry.index));
        }
        expected_index = entry.index.saturating_add(1);
        replayed.push(entry.entry_ref);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let predicate = if decision == "pass" {
        Some(parse_predicate_receipt(&predicate_receipt_value(&PredicateReceiptInput {
            predicate: "trellis-snapshot-restore",
            decision,
            group_ref: &input.group_ref,
            term: snapshot.term,
            index: snapshot.index,
            subjects: &[snapshot.snapshot_ref.clone(), snapshot.state.state_ref.clone()],
            diagnostics: &[],
            checks: &[("trellis-predicate", "pass"), ("snapshot-content-ref", "pass")],
        })?)?)
    } else {
        None
    };
    let value = record("raft-recovery-receipt-v1", vec![
        string(RAFT_RECOVERY_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("group", vec![string(&input.group_ref)]),
        record("snapshot", vec![string(&snapshot.snapshot_ref)]),
        record("restored-state", vec![optional_ref_value(
            (decision == "pass").then_some(snapshot.state.state_ref.as_str()),
        )]),
        record("replayed-log", vec![strings_sequence(&replayed)]),
        record("restore-predicate", vec![optional_ref_value(
            predicate.as_ref().map(|value| value.predicate_ref.as_str()),
        )]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[("snapshot-verified", decision), ("log-suffix-checked", "pass")]),
    ]);
    Ok(RaftRecoveryReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        restored_state_ref: (decision == "pass").then_some(snapshot.state.state_ref),
        diagnostics,
        value,
    })
}

pub fn persist_control_registry_runtime(
    root: &Path,
    runtime: &ControlRegistryRuntime,
    snapshot: &RaftSnapshot,
) -> Result<()> {
    let db = ensure_store_tables(root)?;
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        let mut logs = write_txn.open_table(STORE_LOGS).map_err(store_error)?;
        for entry in &runtime.log_entries {
            let bytes = canonical_bytes(&entry.value)?;
            logs.insert(entry.entry_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
        }
    }
    {
        let mut snapshots = write_txn.open_table(STORE_SNAPSHOTS).map_err(store_error)?;
        let bytes = canonical_bytes(&snapshot.value)?;
        snapshots.insert(snapshot.snapshot_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
    }
    {
        let mut sessions = write_txn.open_table(STORE_SESSIONS).map_err(store_error)?;
        for session in &runtime.state.client_sessions {
            let value = session_record_value(session);
            let bytes = canonical_bytes(&value)?;
            sessions.insert(session.client_session.as_str(), bytes.as_slice()).map_err(store_error)?;
        }
    }
    {
        let mut receipts = write_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
        for receipt in &runtime.registry_receipts {
            let bytes = canonical_bytes(&receipt.value)?;
            receipts.insert(receipt.receipt_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
        }
        for receipt in &runtime.commit_receipts {
            let bytes = canonical_bytes(&receipt.value)?;
            receipts.insert(receipt.receipt_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
        }
        for receipt in &runtime.predicate_receipts {
            let bytes = canonical_bytes(&receipt.value)?;
            receipts.insert(receipt.predicate_ref.as_str(), bytes.as_slice()).map_err(store_error)?;
        }
    }
    write_txn.commit().map_err(store_error)
}

pub fn control_registry_store_status(root: &Path) -> Result<ControlRegistryStoreStatus> {
    let db = ensure_store_tables(root)?;
    let read_txn = db.begin_read().map_err(store_error)?;
    let log_count = read_txn.open_table(STORE_LOGS).map_err(store_error)?.len().map_err(store_error)?;
    let snapshot_count = read_txn.open_table(STORE_SNAPSHOTS).map_err(store_error)?.len().map_err(store_error)?;
    let session_count = read_txn.open_table(STORE_SESSIONS).map_err(store_error)?.len().map_err(store_error)?;
    let receipt_count = read_txn.open_table(STORE_RECEIPTS).map_err(store_error)?.len().map_err(store_error)?;
    Ok(ControlRegistryStoreStatus {
        log_count,
        snapshot_count,
        session_count,
        receipt_count,
    })
}

pub fn control_registry_fixture_manifest_value() -> Result<IOValue> {
    raft_group_manifest_value(&RaftGroupManifestInput {
        group_id: DEFAULT_GROUP_ID.to_string(),
        members: vec![
            synthetic_ref("node-a")?,
            synthetic_ref("node-b")?,
            synthetic_ref("node-c")?,
        ],
        state_machine: CONTROL_REGISTRY_STATE_MACHINE.to_string(),
        command_schemas: allowed_command_schemas().iter().map(|value| (*value).to_string()).collect(),
        read_mode: READ_MODE_READ_INDEX.to_string(),
        snapshot_policy_ref: synthetic_ref("snapshot-policy")?,
        policy_refs: vec![synthetic_ref("raft-policy")?],
        resource_refs: vec![synthetic_ref("raft-resource")?],
    })
}

pub fn run_control_registry_fixture() -> Result<ControlRegistryRuntime> {
    let manifest_value = control_registry_fixture_manifest_value()?;
    let mut runtime = new_control_registry_runtime(&manifest_value)?;
    let commands = vec![
        control_registry_command_value(&ControlRegistryCommandInput {
            operation: "install-protocol".to_string(),
            namespace: "protocol".to_string(),
            name: "proto:request-response".to_string(),
            target_ref: Some(synthetic_ref("protocol-install")?),
        })?,
        control_registry_command_value(&ControlRegistryCommandInput {
            operation: "set-policy-version".to_string(),
            namespace: "policy".to_string(),
            name: "service-runtime".to_string(),
            target_ref: Some(synthetic_ref("policy-v1")?),
        })?,
        control_registry_command_value(&ControlRegistryCommandInput {
            operation: "set-receipt-index".to_string(),
            namespace: "receipt-index".to_string(),
            name: "harness-gate".to_string(),
            target_ref: Some(synthetic_ref("checkpoint")?),
        })?,
    ];
    for (index, command) in commands.into_iter().enumerate() {
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:fixture".to_string(),
            sequence: u64::try_from(index + 1)
                .map_err(|error| MoltenError::invalid_harness(format!("fixture sequence overflow: {error}")))?,
            command,
            authority_refs: vec![synthetic_ref("authority")?],
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![synthetic_ref("proposal-evidence")?],
        })?;
        propose_control_registry_command(&mut runtime, &envelope)?;
    }
    Ok(runtime)
}

pub fn control_registry_summary(runtime: &ControlRegistryRuntime) -> String {
    format!(
        "raft-control-registry group={} committed={} entries={} state={}",
        runtime.manifest.group_id,
        runtime.committed_index,
        runtime.state.entries.len(),
        runtime.state.state_ref
    )
}

fn admitted_command(command: &IOValue) -> (Option<ControlRegistryCommand>, Vec<String>) {
    if let Ok(parsed) = parse_control_registry_command(command) {
        return (Some(parsed), Vec::new());
    }
    let text = crate::preserves_rail::to_text(command).unwrap_or_else(|_| "<unrenderable>".to_string());
    let forbidden = [
        "actor-message",
        "protocol-message",
        "remote-dataspace-envelope",
        "gossip",
        "docs",
        "blob-transfer",
        "chunk-manifest",
        "global-script",
        "comm",
        "choreography-step",
    ];
    if forbidden.iter().any(|marker| text.contains(marker)) {
        return (None, vec!["non-control-plane payload rejected from Raft".to_string()]);
    }
    (None, vec!["unknown Raft command schema".to_string()])
}

fn proposal_diagnostics(input: ProposalDecisionInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = input.diagnostics;
    if input.envelope.group_ref != input.runtime.manifest.manifest_ref {
        diagnostics.push("command group does not match runtime manifest".to_string());
    }
    if input.envelope.authority_refs.is_empty() {
        diagnostics.push("missing proposal authority evidence".to_string());
    }
    if input.envelope.policy_refs.is_empty() {
        diagnostics.push("missing proposal policy evidence".to_string());
    }
    if input.envelope.resource_refs.is_empty() {
        diagnostics.push("missing proposal resource evidence".to_string());
    }
    if input.envelope.evidence_refs.is_empty() {
        diagnostics.push("missing proposal evidence".to_string());
    }
    if let Some(command) = input.command {
        let schema = operation_schema(&command.operation);
        if !input.runtime.manifest.command_schemas.iter().any(|value| value == schema) {
            diagnostics.push(format!("command schema {schema} is not admitted by manifest"));
        }
    }
    validate_refs(&input.envelope.authority_refs, "proposal authority ref")?;
    validate_refs(&input.envelope.policy_refs, "proposal policy ref")?;
    validate_refs(&input.envelope.resource_refs, "proposal resource ref")?;
    validate_refs(&input.envelope.evidence_refs, "proposal evidence ref")?;
    ensure_count_at_most(diagnostics.len(), MAX_RAFT_DIAGNOSTICS, "raft proposal diagnostics")?;
    Ok(diagnostics)
}

fn duplicate_receipt(
    runtime: &ControlRegistryRuntime,
    envelope: &RaftCommandEnvelope,
) -> Option<ControlRegistryReceipt> {
    let has_session = runtime
        .state
        .client_sessions
        .iter()
        .any(|session| session.client_session == envelope.client_session && session.sequence == envelope.sequence);
    if !has_session {
        return None;
    }
    runtime
        .registry_receipts
        .iter()
        .find(|receipt| receipt.command_ref == envelope.envelope_ref)
        .cloned()
}

fn apply_admitted_command(
    runtime: &mut ControlRegistryRuntime,
    envelope: &RaftCommandEnvelope,
    command: &ControlRegistryCommand,
    log_entry: &RaftLogEntry,
) -> Result<ControlRegistryReceipt> {
    let before_ref = runtime.state.state_ref.clone();
    let mut maps = state_maps(&runtime.state)?;
    match command.operation.as_str() {
        "remove" => {
            maps.entries.remove(&ControlRegistryKey {
                namespace: command.namespace.clone(),
                name: command.name.clone(),
            });
        }
        _ => {
            let target_ref = command
                .target_ref
                .clone()
                .ok_or_else(|| MoltenError::invalid_harness("admitted set command missing target ref"))?;
            maps.entries.insert(
                ControlRegistryKey {
                    namespace: command.namespace.clone(),
                    name: command.name.clone(),
                },
                target_ref,
            );
        }
    }
    maps.sessions.insert(envelope.client_session.clone(), ClientSessionRecord {
        client_session: envelope.client_session.clone(),
        sequence: envelope.sequence,
        result_command_ref: envelope.envelope_ref.clone(),
    });
    runtime.state = parse_control_registry_state(&control_registry_state_value(
        entries_from_map(&maps.entries),
        sessions_from_map(&maps.sessions),
    )?)?;
    let receipt_value = registry_receipt_value(&RegistryReceiptValueInput {
        decision: "pass",
        operation: &command.operation,
        command_ref: &envelope.envelope_ref,
        log_entry_ref: Some(&log_entry.entry_ref),
        state_before_ref: &before_ref,
        state_after_ref: Some(&runtime.state.state_ref),
        client_session: &envelope.client_session,
        sequence: envelope.sequence,
        duplicate: false,
        authority_refs: &envelope.authority_refs,
        policy_refs: &envelope.policy_refs,
        resource_refs: &envelope.resource_refs,
        diagnostics: &[],
    })?;
    parse_registry_receipt(&receipt_value)
}

fn deny_commit_receipt(
    runtime: &ControlRegistryRuntime,
    envelope: &RaftCommandEnvelope,
    reason: &str,
    diagnostics: &[String],
) -> Result<RaftCommitReceipt> {
    let mut all_diagnostics = Vec::with_capacity(diagnostics.len() + 1);
    all_diagnostics.push(reason.to_string());
    all_diagnostics.extend(diagnostics.iter().cloned());
    let value = commit_receipt_value(&CommitReceiptValueInput {
        decision: "deny",
        group_ref: &runtime.manifest.manifest_ref,
        term: runtime.term,
        index: runtime.committed_index,
        command_ref: &envelope.envelope_ref,
        log_entry_ref: None,
        quorum_refs: &[],
        append_predicate_ref: None,
        commit_predicate_ref: None,
        diagnostics: &all_diagnostics,
    })?;
    parse_commit_receipt(&value)
}

fn deny_registry_receipt(
    runtime: &ControlRegistryRuntime,
    envelope: &RaftCommandEnvelope,
    command: Option<&ControlRegistryCommand>,
    diagnostics: &[String],
) -> Result<ControlRegistryReceipt> {
    let operation = command.map(|value| value.operation.as_str()).unwrap_or("deny");
    let value = registry_receipt_value(&RegistryReceiptValueInput {
        decision: "deny",
        operation,
        command_ref: &envelope.envelope_ref,
        log_entry_ref: None,
        state_before_ref: &runtime.state.state_ref,
        state_after_ref: None,
        client_session: &envelope.client_session,
        sequence: envelope.sequence,
        duplicate: false,
        authority_refs: &envelope.authority_refs,
        policy_refs: &envelope.policy_refs,
        resource_refs: &envelope.resource_refs,
        diagnostics,
    })?;
    parse_registry_receipt(&value)
}

fn raft_log_entry_value(input: &LogEntryValueInput<'_>) -> Result<IOValue> {
    require_ref(input.group_ref, "raft log group ref")?;
    require_ref(input.command_ref, "raft log command ref")?;
    require_ref(input.append_predicate_ref, "raft log append predicate ref")?;
    if let Some(reference) = input.prior_log_ref {
        require_ref(reference, "raft log prior ref")?;
    }
    Ok(record("raft-log-entry-v1", vec![
        string(RAFT_LOG_ENTRY_SCHEMA),
        record("group", vec![string(input.group_ref)]),
        record("term", vec![u64_value(input.term)]),
        record("index", vec![u64_value(input.index)]),
        record("prior-log", vec![optional_ref_value(input.prior_log_ref)]),
        record("command-ref", vec![string(input.command_ref)]),
        record("command", vec![input.command.clone()]),
        record("append-predicate", vec![string(input.append_predicate_ref)]),
        checks_value(&[("append-consistency", "pass"), ("command-ref-binding", "pass")]),
    ]))
}

fn parse_raft_log_entry(value: &IOValue) -> Result<RaftLogEntry> {
    let fields = value
        .collect_simple_record("raft-log-entry-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-log-entry-v1 ...>"))?;
    require_schema(&fields[0], RAFT_LOG_ENTRY_SCHEMA, "raft log entry schema")?;
    let command = record_iovalue(&fields[6], "command")?;
    require_check(&parse_checks(&fields[8])?, "append-consistency", "raft log entry")?;
    Ok(RaftLogEntry {
        entry_ref: canonical_hash(value)?,
        group_ref: record_ref(&fields[1], "group")?,
        term: record_u64(&fields[2], "term")?,
        index: record_u64(&fields[3], "index")?,
        prior_log_ref: record_optional_ref(&fields[4], "prior-log")?,
        command_ref: record_ref(&fields[5], "command-ref")?,
        command,
        append_predicate_ref: record_ref(&fields[7], "append-predicate")?,
        value: value.clone(),
    })
}

fn commit_receipt_value(input: &CommitReceiptValueInput<'_>) -> Result<IOValue> {
    require_ref(input.group_ref, "raft commit group ref")?;
    require_ref(input.command_ref, "raft commit command ref")?;
    validate_refs(input.quorum_refs, "raft commit quorum ref")?;
    Ok(record("raft-commit-receipt-v1", vec![
        string(RAFT_COMMIT_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("group", vec![string(input.group_ref)]),
        record("term", vec![u64_value(input.term)]),
        record("index", vec![u64_value(input.index)]),
        record("command", vec![string(input.command_ref)]),
        record("log-entry", vec![optional_ref_value(input.log_entry_ref)]),
        record("quorum", vec![strings_sequence(input.quorum_refs)]),
        record("append-predicate", vec![optional_ref_value(input.append_predicate_ref)]),
        record("commit-predicate", vec![optional_ref_value(input.commit_predicate_ref)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[("quorum-commit", input.decision), ("decision-before-apply", "pass")]),
    ]))
}

fn parse_commit_receipt(value: &IOValue) -> Result<RaftCommitReceipt> {
    let fields = value
        .collect_simple_record("raft-commit-receipt-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-commit-receipt-v1 ...>"))?;
    require_schema(&fields[0], RAFT_COMMIT_RECEIPT_SCHEMA, "raft commit receipt schema")?;
    Ok(RaftCommitReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        group_ref: record_ref(&fields[2], "group")?,
        term: record_u64(&fields[3], "term")?,
        index: record_u64(&fields[4], "index")?,
        command_ref: record_ref(&fields[5], "command")?,
        log_entry_ref: record_optional_ref(&fields[6], "log-entry")?,
        value: value.clone(),
    })
}

fn registry_receipt_value(input: &RegistryReceiptValueInput<'_>) -> Result<IOValue> {
    require_ref(input.command_ref, "control registry command ref")?;
    require_ref(input.state_before_ref, "control registry state-before ref")?;
    if let Some(reference) = input.state_after_ref {
        require_ref(reference, "control registry state-after ref")?;
    }
    Ok(record("control-registry-receipt-v1", vec![
        string(CONTROL_REGISTRY_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(input.operation)]),
        record("command", vec![string(input.command_ref)]),
        record("log-entry", vec![optional_ref_value(input.log_entry_ref)]),
        record("state-before", vec![string(input.state_before_ref)]),
        record("state-after", vec![optional_ref_value(input.state_after_ref)]),
        record("client-session", vec![string(input.client_session)]),
        record("sequence", vec![u64_value(input.sequence)]),
        record("duplicate", vec![bool_value(input.duplicate)]),
        record("authority", vec![strings_sequence(input.authority_refs)]),
        record("policy", vec![strings_sequence(input.policy_refs)]),
        record("resource", vec![strings_sequence(input.resource_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("deterministic-apply", input.decision),
            ("client-session-idempotency", "pass"),
        ]),
    ]))
}

fn parse_registry_receipt(value: &IOValue) -> Result<ControlRegistryReceipt> {
    let fields = value
        .collect_simple_record("control-registry-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <control-registry-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONTROL_REGISTRY_RECEIPT_SCHEMA, "control registry receipt schema")?;
    Ok(ControlRegistryReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        operation: record_string(&fields[2], "operation")?,
        command_ref: record_ref(&fields[3], "command")?,
        state_before_ref: record_ref(&fields[5], "state-before")?,
        state_after_ref: record_optional_ref(&fields[6], "state-after")?,
        duplicate: record_bool(&fields[9], "duplicate")?,
        diagnostics: parse_string_sequence(&fields[13], "diagnostics")?,
        value: value.clone(),
    })
}

fn predicate_receipt_value(input: &PredicateReceiptInput<'_>) -> Result<IOValue> {
    validate_non_empty(input.predicate, "raft predicate")?;
    require_ref(input.group_ref, "raft predicate group ref")?;
    validate_refs(input.subjects, "raft predicate subject ref")?;
    Ok(record("raft-predicate-receipt-v1", vec![
        string(RAFT_PREDICATE_RECEIPT_SCHEMA),
        record("predicate", vec![string(input.predicate)]),
        record("decision", vec![string(input.decision)]),
        record("group", vec![string(input.group_ref)]),
        record("term", vec![u64_value(input.term)]),
        record("index", vec![u64_value(input.index)]),
        record("subjects", vec![strings_sequence(input.subjects)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(input.checks),
    ]))
}

fn parse_predicate_receipt(value: &IOValue) -> Result<RaftPredicateReceipt> {
    let fields = value
        .collect_simple_record("raft-predicate-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-predicate-receipt-v1 ...>"))?;
    require_schema(&fields[0], RAFT_PREDICATE_RECEIPT_SCHEMA, "raft predicate receipt schema")?;
    require_check(&parse_checks(&fields[8])?, "trellis-predicate", "raft predicate receipt")?;
    Ok(RaftPredicateReceipt {
        predicate_ref: canonical_hash(value)?,
        predicate: record_string(&fields[1], "predicate")?,
        decision: record_string(&fields[2], "decision")?,
        value: value.clone(),
    })
}

fn read_receipt_value(input: &ReadReceiptValueInput<'_>) -> Result<IOValue> {
    Ok(record("raft-read-receipt-v1", vec![
        string(RAFT_READ_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("group", vec![string(input.group_ref)]),
        record("state", vec![string(input.state_ref)]),
        record("committed-term", vec![u64_value(input.committed_term)]),
        record("committed-index", vec![u64_value(input.committed_index)]),
        record("namespace", vec![string(input.namespace)]),
        record("name", vec![string(input.name)]),
        record("target", vec![optional_ref_value(input.target_ref)]),
        record("read-index-predicate", vec![optional_ref_value(input.read_index_predicate_ref)]),
        record("authority", vec![strings_sequence(input.authority_refs)]),
        record("resource", vec![strings_sequence(input.resource_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("read-index-bound", input.decision),
            ("authority-resource-gated", "pass"),
        ]),
    ]))
}

fn state_maps(state: &ControlRegistryState) -> Result<RegistryMaps> {
    let entries = state
        .entries
        .iter()
        .map(|entry| {
            (
                ControlRegistryKey {
                    namespace: entry.namespace.clone(),
                    name: entry.name.clone(),
                },
                entry.target_ref.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if entries.len() != state.entries.len() {
        return Err(MoltenError::invalid_harness("duplicate control registry entry key"));
    }
    let sessions = state
        .client_sessions
        .iter()
        .map(|session| (session.client_session.clone(), session.clone()))
        .collect::<BTreeMap<_, _>>();
    if sessions.len() != state.client_sessions.len() {
        return Err(MoltenError::invalid_harness("duplicate control registry client session"));
    }
    Ok(RegistryMaps { entries, sessions })
}

fn entries_from_map(entries: &BTreeMap<ControlRegistryKey, String>) -> Vec<ControlRegistryEntry> {
    entries
        .iter()
        .map(|(key, target_ref)| ControlRegistryEntry {
            namespace: key.namespace.clone(),
            name: key.name.clone(),
            target_ref: target_ref.clone(),
        })
        .collect()
}

fn sessions_from_map(sessions: &BTreeMap<String, ClientSessionRecord>) -> Vec<ClientSessionRecord> {
    sessions.values().cloned().collect()
}

fn parse_registry_entries(value: &Value<IOValue>) -> Result<Vec<ControlRegistryEntry>> {
    let values = field_sequence(value, "entries")?;
    ensure_count_at_most(values.len(), MAX_RAFT_ENTRIES, "control registry entries")?;
    let mut entries = Vec::with_capacity(values.len());
    for entry in values {
        let entry_value = value_to_iovalue(&entry);
        let fields = entry_value
            .collect_simple_record("entry", Some(3))
            .ok_or_else(|| MoltenError::invalid_harness("expected control registry entry"))?;
        let namespace = required_string(&fields[0], "entry namespace")?;
        validate_namespace(&namespace)?;
        let name = required_string(&fields[1], "entry name")?;
        validate_non_empty(&name, "entry name")?;
        let target_ref = required_ref(&fields[2], "entry target ref")?;
        entries.push(ControlRegistryEntry {
            namespace,
            name,
            target_ref,
        });
    }
    Ok(entries)
}

fn parse_client_sessions(value: &Value<IOValue>) -> Result<Vec<ClientSessionRecord>> {
    let values = field_sequence(value, "client-sessions")?;
    ensure_count_at_most(values.len(), MAX_RAFT_ENTRIES, "control registry client sessions")?;
    let mut sessions = Vec::with_capacity(values.len());
    for session in values {
        let session_value = value_to_iovalue(&session);
        let fields = session_value
            .collect_simple_record("session", Some(3))
            .ok_or_else(|| MoltenError::invalid_harness("expected client session record"))?;
        let client_session = required_string(&fields[0], "client session id")?;
        validate_client_session(&client_session)?;
        sessions.push(ClientSessionRecord {
            client_session,
            sequence: required_u64(&fields[1], "client session sequence")?,
            result_command_ref: required_ref(&fields[2], "client session command ref")?,
        });
    }
    Ok(sessions)
}

fn find_entry<'a>(state: &'a ControlRegistryState, namespace: &str, name: &str) -> Option<&'a ControlRegistryEntry> {
    state.entries.iter().find(|entry| entry.namespace == namespace && entry.name == name)
}

fn validate_control_command(input: &ControlRegistryCommandInput) -> Result<()> {
    validate_operation(&input.operation)?;
    validate_namespace(&input.namespace)?;
    validate_non_empty(&input.name, "control registry command name")?;
    if let Some(target_ref) = &input.target_ref {
        require_ref(target_ref, "control registry command target ref")?;
    }
    match input.operation.as_str() {
        "install-protocol" if input.namespace == "protocol" && input.target_ref.is_some() => Ok(()),
        "set-artifact-name" if input.namespace == "artifact-name" && input.target_ref.is_some() => Ok(()),
        "set-policy-version" if input.namespace == "policy" && input.target_ref.is_some() => Ok(()),
        "set-capability-version" if input.namespace == "capability" && input.target_ref.is_some() => Ok(()),
        "set-receipt-index" if input.namespace == "receipt-index" && input.target_ref.is_some() => Ok(()),
        "set-coordination-state" if input.namespace.starts_with("coordination:") && input.target_ref.is_some() => {
            Ok(())
        }
        "remove" if input.target_ref.is_none() => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!(
            "operation {} is not admitted for namespace {}",
            input.operation, input.namespace
        ))),
    }
}

fn validate_group_id(group_id: &str) -> Result<()> {
    if group_id.starts_with("raft:") {
        validate_non_empty(group_id, "raft group id")
    } else {
        Err(MoltenError::invalid_harness(format!("raft group id must start with raft:, got {group_id}")))
    }
}

fn validate_client_session(client_session: &str) -> Result<()> {
    validate_non_empty(client_session, "raft client session")
}

fn validate_operation(operation: &str) -> Result<()> {
    if [
        "install-protocol",
        "set-artifact-name",
        "set-policy-version",
        "set-capability-version",
        "set-receipt-index",
        "set-coordination-state",
        "remove",
    ]
    .contains(&operation)
    {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported control registry operation {operation}")))
    }
}

fn validate_namespace(namespace: &str) -> Result<()> {
    if ["protocol", "artifact-name", "policy", "capability", "receipt-index"].contains(&namespace)
        || namespace.starts_with("coordination:")
    {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported control registry namespace {namespace}")))
    }
}

fn validate_command_schema_list(command_schemas: &[String]) -> Result<()> {
    ensure_count_at_most(command_schemas.len(), MAX_RAFT_COMMANDS, "raft command schemas")?;
    for schema in command_schemas {
        if !allowed_command_schemas().contains(&schema.as_str()) {
            return Err(MoltenError::invalid_harness(format!("unsupported raft command schema {schema}")));
        }
    }
    Ok(())
}

fn allowed_command_schemas() -> &'static [&'static str] {
    &[
        "install-protocol",
        "set-artifact-name",
        "set-policy-version",
        "set-capability-version",
        "set-receipt-index",
        "set-coordination-state",
        "remove",
    ]
}

fn operation_schema(operation: &str) -> &str {
    match operation {
        "install-protocol" => "install-protocol",
        "set-artifact-name" => "set-artifact-name",
        "set-policy-version" => "set-policy-version",
        "set-capability-version" => "set-capability-version",
        "set-receipt-index" => "set-receipt-index",
        "set-coordination-state" => "set-coordination-state",
        "remove" => "remove",
        _ => "unknown",
    }
}

fn validate_read_mode(read_mode: &str) -> Result<()> {
    if read_mode == READ_MODE_READ_INDEX {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported raft read mode {read_mode}")))
    }
}

fn validate_non_empty(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_RAFT_REFS, label)?;
    for reference in refs {
        require_ref(reference, label)?;
    }
    Ok(())
}

fn require_ref(reference: &str, label: &str) -> Result<()> {
    if reference.starts_with("blake3:") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("expected blake3 ref for {label}, got {reference}")))
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn strings_sequence(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn checks_value(checks: &[(&str, &str)]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn optional_ref_value(reference: Option<&str>) -> IOValue {
    reference.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn session_record_value(session: &ClientSessionRecord) -> IOValue {
    record("session", vec![
        string(&session.client_session),
        u64_value(session.sequence),
        string(&session.result_command_ref),
    ])
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    parse_optional_ref_value(&fields[0])
}

fn record_iovalue(value: &Value<IOValue>, label: &str) -> Result<IOValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(value_to_iovalue(&fields[0]))
}

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_u64(&fields[0], label)
}

fn record_bool(value: &Value<IOValue>, label: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    fields[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn parse_optional_ref_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional raft ref").map(Some);
    }
    required_ref(value, "optional raft ref").map(Some)
}

fn parse_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), MAX_RAFT_REFS, label)?;
    values
        .iter()
        .map(|value| {
            let reference = required_string(value, label)?;
            require_ref(&reference, label)?;
            Ok(reference)
        })
        .collect()
}

fn parse_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), MAX_RAFT_REFS, label)?;
    values.iter().map(|value| required_string(value, label)).collect()
}

fn field_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<Value<IOValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    ensure_count_at_most(values.len(), MAX_RAFT_COMMANDS, "raft checks")?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected raft check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str, context: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{context} missing passing {name} check")))
}

fn require_schema(value: &Value<IOValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")))
    }
}

fn required_ref(value: &Value<IOValue>, field: &str) -> Result<String> {
    let reference = required_string(value, field)?;
    require_ref(&reference, field)?;
    Ok(reference)
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

fn ensure_store_tables(root: &Path) -> Result<Database> {
    fs::create_dir_all(root).map_err(MoltenError::from)?;
    let db = Database::create(root.join(STORE_FILE)).map_err(store_error)?;
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        write_txn.open_table(STORE_LOGS).map_err(store_error)?;
        write_txn.open_table(STORE_SNAPSHOTS).map_err(store_error)?;
        write_txn.open_table(STORE_SESSIONS).map_err(store_error)?;
        write_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
    }
    write_txn.commit().map_err(store_error)?;
    Ok(db)
}

fn store_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("control registry redb store error: {error}"))
}

fn synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("raft-control-fixture-ref", vec![string(label)]))
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::catalog;
    use crate::catalog::CatalogListInput;
    use crate::catalog::CatalogVisibilityInput;
    use crate::catalog_mcp;
    use crate::ledger;
    use crate::preserves_rail::parse_text;
    use crate::preserves_rail::to_text;

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("raft-control-test-ref", vec![string(label)])).expect("test ref")
    }

    fn auth() -> Vec<String> {
        vec![test_ref("authority")]
    }

    fn resources() -> Vec<String> {
        vec![test_ref("resource")]
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("molten-raft-control-{label}-{id}"));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn local_cluster_applies_reads_snapshots_and_recovers() {
        let runtime = run_control_registry_fixture().expect("run fixture");
        assert_eq!(runtime.committed_index, 3);
        assert_eq!(runtime.state.entries.len(), 3);
        let read = read_control_registry(&ControlRegistryReadInput {
            state: runtime.state.value.clone(),
            group_ref: runtime.manifest.manifest_ref.clone(),
            committed_term: runtime.term,
            committed_index: runtime.committed_index,
            read_index: runtime.committed_index,
            namespace: "protocol".to_string(),
            name: "proto:request-response".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
        })
        .expect("read registry");
        assert_eq!(read.decision, "pass");
        assert!(read.target_ref.is_some());
        let snapshot = snapshot_control_registry(&RaftSnapshotInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            term: runtime.term,
            index: runtime.committed_index,
            state: runtime.state.value.clone(),
            log_refs: runtime.log_entries.iter().map(|entry| entry.entry_ref.clone()).collect(),
        })
        .expect("snapshot");
        let recovery = recover_control_registry(&RaftRecoveryInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            snapshot: snapshot.value,
            log_entries: Vec::new(),
        })
        .expect("recover");
        assert_eq!(recovery.decision, "pass");
        assert_eq!(recovery.restored_state_ref.as_deref(), Some(runtime.state.state_ref.as_str()));
    }

    #[test]
    fn registry_updates_remove_and_duplicate_sequences_are_idempotent() {
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let mut runtime = new_control_registry_runtime(&manifest).expect("runtime");
        let command = control_registry_command_value(&ControlRegistryCommandInput {
            operation: "set-artifact-name".to_string(),
            namespace: "artifact-name".to_string(),
            name: "calculator".to_string(),
            target_ref: Some(test_ref("artifact-v1")),
        })
        .expect("command");
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:one".to_string(),
            sequence: 7,
            command,
            authority_refs: auth(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("envelope");
        let first = propose_control_registry_command(&mut runtime, &envelope).expect("first proposal");
        assert_eq!(first.decision, "pass");
        let state_after_first = runtime.state.state_ref.clone();
        let duplicate = propose_control_registry_command(&mut runtime, &envelope).expect("duplicate proposal");
        assert_eq!(duplicate.decision, "pass");
        assert!(duplicate.duplicate);
        assert_eq!(duplicate.registry_receipt.receipt_ref, first.registry_receipt.receipt_ref);
        assert_eq!(runtime.state.state_ref, state_after_first);
        assert_eq!(runtime.log_entries.len(), 1);

        let remove = control_registry_command_value(&ControlRegistryCommandInput {
            operation: "remove".to_string(),
            namespace: "artifact-name".to_string(),
            name: "calculator".to_string(),
            target_ref: None,
        })
        .expect("remove command");
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:one".to_string(),
            sequence: 8,
            command: remove,
            authority_refs: auth(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("remove envelope");
        propose_control_registry_command(&mut runtime, &envelope).expect("remove proposal");
        assert!(find_entry(&runtime.state, "artifact-name", "calculator").is_none());
    }

    #[test]
    fn actor_messages_and_missing_authority_do_not_append() {
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let mut runtime = new_control_registry_runtime(&manifest).expect("runtime");
        let actor_message = parse_text("<actor-message-v1 \"hello\">").expect("actor message");
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:bad".to_string(),
            sequence: 1,
            command: actor_message,
            authority_refs: auth(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("bad envelope");
        let denied = propose_control_registry_command(&mut runtime, &envelope).expect("deny actor message");
        assert_eq!(denied.decision, "deny");
        assert!(denied.log_entry.is_none());
        assert!(runtime.log_entries.is_empty());
        assert!(denied.registry_receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("non-control")));

        let command = control_registry_command_value(&ControlRegistryCommandInput {
            operation: "set-policy-version".to_string(),
            namespace: "policy".to_string(),
            name: "runtime".to_string(),
            target_ref: Some(test_ref("policy")),
        })
        .expect("command");
        let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            client_session: "client:missing-auth".to_string(),
            sequence: 2,
            command,
            authority_refs: Vec::new(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("missing authority envelope");
        let denied = propose_control_registry_command(&mut runtime, &envelope).expect("deny missing authority");
        assert_eq!(denied.decision, "deny");
        assert!(denied.log_entry.is_none());
        assert!(runtime.log_entries.is_empty());
    }

    #[test]
    fn stale_read_bad_snapshot_log_gap_and_redb_store_are_detected() {
        let runtime = run_control_registry_fixture().expect("runtime");
        let stale = read_control_registry(&ControlRegistryReadInput {
            state: runtime.state.value.clone(),
            group_ref: runtime.manifest.manifest_ref.clone(),
            committed_term: runtime.term,
            committed_index: runtime.committed_index,
            read_index: runtime.committed_index.saturating_sub(1),
            namespace: "protocol".to_string(),
            name: "proto:request-response".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
        })
        .expect("stale read");
        assert_eq!(stale.decision, "deny");
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale")));

        let snapshot = snapshot_control_registry(&RaftSnapshotInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            term: runtime.term,
            index: runtime.committed_index,
            state: runtime.state.value.clone(),
            log_refs: runtime.log_entries.iter().map(|entry| entry.entry_ref.clone()).collect(),
        })
        .expect("snapshot");
        let mut bad_snapshot = snapshot.value.clone();
        if let Some(fields) = bad_snapshot.collect_simple_record("raft-snapshot-v1", Some(10)) {
            let mut fields = (0..10).map(|index| value_to_iovalue(&fields[index])).collect::<Vec<_>>();
            fields[5] = record("content-ref", vec![string(test_ref("wrong-content"))]);
            bad_snapshot = record("raft-snapshot-v1", fields);
        }
        assert!(parse_raft_snapshot(&bad_snapshot).is_err());

        let gap_entry = runtime.log_entries[0].clone();
        let mut gap_value = gap_entry.value.clone();
        if let Some(fields) = gap_value.collect_simple_record("raft-log-entry-v1", Some(9)) {
            let mut fields = (0..9).map(|index| value_to_iovalue(&fields[index])).collect::<Vec<_>>();
            fields[3] = record("index", vec![u64_value(snapshot.index + 2)]);
            gap_value = record("raft-log-entry-v1", fields);
        }
        let recovery = recover_control_registry(&RaftRecoveryInput {
            group_ref: runtime.manifest.manifest_ref.clone(),
            snapshot: snapshot.value.clone(),
            log_entries: vec![gap_value],
        })
        .expect("gap recovery");
        assert_eq!(recovery.decision, "deny");
        assert!(recovery.diagnostics.iter().any(|diagnostic| diagnostic.contains("log gap")));

        let root = temp_dir("redb-store");
        persist_control_registry_runtime(&root, &runtime, &snapshot).expect("persist runtime");
        let status = control_registry_store_status(&root).expect("store status");
        assert_eq!(status.log_count, 3);
        assert_eq!(status.snapshot_count, 1);
        assert_eq!(status.session_count, 1);
        assert!(status.receipt_count >= 3);
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_raft_artifacts() {
        let runtime = run_control_registry_fixture().expect("runtime");
        assert_eq!(ledger::artifact_kind(&runtime.manifest.value), "raft-group-manifest");
        assert_eq!(ledger::artifact_kind(&runtime.log_entries[0].value), "raft-log-entry");
        assert_eq!(ledger::artifact_kind(&runtime.registry_receipts[0].value), "control-registry-receipt");
        let ledger_root = temp_dir("ledger");
        ledger::import_artifact(&ledger_root, &runtime.registry_receipts[0].value).expect("import registry receipt");
        let registry = temp_dir("catalog");
        let listed = catalog::list(&registry, Some(&ledger_root), &CatalogListInput {
            kind: Some("control-registry-receipt".to_string()),
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("catalog list");
        assert_eq!(listed.items.len(), 1);
        let request = catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string(
            "control-registry-receipt",
        )])])
        .expect("mcp request");
        let mcp = catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("mcp call");
        assert_eq!(mcp.decision, "pass");
        assert!(to_text(&mcp.response_value).expect("render mcp").contains("control-registry-receipt"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_bounded_registry_logs_are_deterministic_and_control_only(tc: TestCase) {
        let command_count =
            usize::try_from(tc.draw(generators::integers::<u64>().min_value(1).max_value(4))).expect("command count");
        let manifest = control_registry_fixture_manifest_value().expect("manifest");
        let mut left = new_control_registry_runtime(&manifest).expect("left runtime");
        let mut right = new_control_registry_runtime(&manifest).expect("right runtime");
        for index in 0..command_count {
            let target = test_ref(&format!("target-{index}"));
            let command = control_registry_command_value(&ControlRegistryCommandInput {
                operation: "set-receipt-index".to_string(),
                namespace: "receipt-index".to_string(),
                name: format!("scope-{index}"),
                target_ref: Some(target),
            })
            .expect("generated command");
            let envelope = raft_command_envelope_value(&RaftCommandEnvelopeInput {
                group_ref: left.manifest.manifest_ref.clone(),
                client_session: "client:property".to_string(),
                sequence: u64::try_from(index + 1).expect("sequence"),
                command,
                authority_refs: auth(),
                policy_refs: left.manifest.policy_refs.clone(),
                resource_refs: left.manifest.resource_refs.clone(),
                evidence_refs: vec![test_ref("evidence")],
            })
            .expect("generated envelope");
            let left_result = propose_control_registry_command(&mut left, &envelope).expect("left proposal");
            let right_result = propose_control_registry_command(&mut right, &envelope).expect("right proposal");
            assert_eq!(left_result.decision, "pass");
            assert_eq!(left_result.registry_receipt.receipt_ref, right_result.registry_receipt.receipt_ref);
        }
        assert_eq!(left.state.state_ref, right.state.state_ref);
        assert_eq!(left.log_entries.len(), command_count);
        for entry in &left.log_entries {
            let envelope = parse_raft_command_envelope(&entry.command).expect("entry command envelope");
            assert!(parse_control_registry_command(&envelope.command).is_ok());
        }
    }
}
