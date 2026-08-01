use std::collections::BTreeMap;

use super::valid_ref;
use crate::fabric::FabricPortClass;
use crate::fabric::ReferenceSystemKind;

const INITIAL_STATE_VERSION: u64 = 0;
const VERSION_INCREMENT: u64 = 1;
const INITIAL_LOG_OFFSET: u64 = 0;
const LOG_OFFSET_INCREMENT: u64 = 1;
const INITIAL_LEASE_EPOCH: u64 = 0;
const LEASE_EPOCH_INCREMENT: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceServiceState {
    TransactionalKeyValue(TransactionalKeyValueState),
    ReplicatedLog(ReplicatedLogState),
    DistributedScheduler(DistributedSchedulerState),
}

impl ReferenceServiceState {
    pub const fn kind(&self) -> ReferenceSystemKind {
        match self {
            Self::TransactionalKeyValue(_) => ReferenceSystemKind::TransactionalKeyValue,
            Self::ReplicatedLog(_) => ReferenceSystemKind::ReplicatedLog,
            Self::DistributedScheduler(_) => ReferenceSystemKind::DistributedScheduler,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransactionalKeyValueState {
    pub version: u64,
    pub values: BTreeMap<String, String>,
    pub recovered_version: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionalKeyValueOperation {
    Commit {
        expected_version: u64,
        writes: Vec<(String, String)>,
    },
    Recover {
        checkpoint_version: u64,
        checkpoint_values: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicatedLogEntry {
    pub offset: u64,
    pub payload_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicatedLogState {
    pub next_offset: u64,
    pub retained_from: u64,
    pub replicated_through: Option<u64>,
    pub entries: Vec<ReplicatedLogEntry>,
}

impl Default for ReplicatedLogState {
    fn default() -> Self {
        Self {
            next_offset: INITIAL_LOG_OFFSET,
            retained_from: INITIAL_LOG_OFFSET,
            replicated_through: None,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicatedLogOperation {
    Append {
        payload_ref: String,
    },
    ReplicateThrough {
        offset: u64,
    },
    RetainFrom {
        offset: u64,
    },
    Recover {
        entries: Vec<ReplicatedLogEntry>,
        replicated_through: Option<u64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceJobPhase {
    Pending,
    Leased,
    Completed,
}

impl ReferenceJobPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Leased => "leased",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceJobState {
    pub phase: ReferenceJobPhase,
    pub lease_owner: Option<String>,
    pub lease_epoch: u64,
    pub authoritative_completion_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DistributedSchedulerState {
    pub jobs: BTreeMap<String, ReferenceJobState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistributedSchedulerOperation {
    Submit {
        job_id: String,
    },
    Lease {
        job_id: String,
        owner: String,
    },
    Complete {
        job_id: String,
        owner: String,
        completion_ref: String,
    },
    Failover {
        job_id: String,
        next_owner: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceServiceOperation {
    TransactionalKeyValue(TransactionalKeyValueOperation),
    ReplicatedLog(ReplicatedLogOperation),
    DistributedScheduler(DistributedSchedulerOperation),
}

impl ReferenceServiceOperation {
    pub const fn kind(&self) -> ReferenceSystemKind {
        match self {
            Self::TransactionalKeyValue(_) => ReferenceSystemKind::TransactionalKeyValue,
            Self::ReplicatedLog(_) => ReferenceSystemKind::ReplicatedLog,
            Self::DistributedScheduler(_) => ReferenceSystemKind::DistributedScheduler,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceTransitionDecision {
    Applied,
    Conflict,
    Denied,
}

impl ReferenceTransitionDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Conflict => "conflict",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceServiceTransition {
    pub next: ReferenceServiceState,
    pub decision: ReferenceTransitionDecision,
    pub required_ports: Vec<FabricPortClass>,
    pub semantic_invariants: Vec<&'static str>,
    pub state_material: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceServiceIssue {
    ServiceKindMismatch {
        state: ReferenceSystemKind,
        operation: ReferenceSystemKind,
    },
    MalformedIdentifier {
        field: &'static str,
        value: String,
    },
    MalformedRef {
        field: &'static str,
        value: String,
    },
    DuplicateWriteKey(String),
    VersionOverflow,
    LogOffsetOverflow,
    LogOffsetGap {
        expected: u64,
        actual: u64,
    },
    UnknownLogOffset(u64),
    RetentionBeyondReplication {
        retained_from: u64,
        replicated_through: Option<u64>,
    },
    DuplicateJob(String),
    UnknownJob(String),
    InvalidJobPhase {
        job_id: String,
        expected: ReferenceJobPhase,
        actual: ReferenceJobPhase,
    },
    LeaseOwnerMismatch {
        job_id: String,
        expected: String,
        actual: String,
    },
    MissingLeaseOwner(String),
    LeaseEpochOverflow,
    DuplicateAuthoritativeCompletion(String),
}

pub fn initial_reference_state(kind: ReferenceSystemKind) -> ReferenceServiceState {
    match kind {
        ReferenceSystemKind::TransactionalKeyValue => {
            ReferenceServiceState::TransactionalKeyValue(TransactionalKeyValueState::default())
        }
        ReferenceSystemKind::ReplicatedLog => ReferenceServiceState::ReplicatedLog(ReplicatedLogState::default()),
        ReferenceSystemKind::DistributedScheduler => {
            ReferenceServiceState::DistributedScheduler(DistributedSchedulerState::default())
        }
    }
}

pub fn apply_reference_operation(
    state: &ReferenceServiceState,
    operation: &ReferenceServiceOperation,
) -> Result<ReferenceServiceTransition, ReferenceServiceIssue> {
    if state.kind() != operation.kind() {
        return Err(ReferenceServiceIssue::ServiceKindMismatch {
            state: state.kind(),
            operation: operation.kind(),
        });
    }
    match (state, operation) {
        (
            ReferenceServiceState::TransactionalKeyValue(state),
            ReferenceServiceOperation::TransactionalKeyValue(operation),
        ) => apply_transaction(state, operation),
        (ReferenceServiceState::ReplicatedLog(state), ReferenceServiceOperation::ReplicatedLog(operation)) => {
            apply_log(state, operation)
        }
        (
            ReferenceServiceState::DistributedScheduler(state),
            ReferenceServiceOperation::DistributedScheduler(operation),
        ) => apply_scheduler(state, operation),
        _ => Err(ReferenceServiceIssue::ServiceKindMismatch {
            state: state.kind(),
            operation: operation.kind(),
        }),
    }
}

fn apply_transaction(
    state: &TransactionalKeyValueState,
    operation: &TransactionalKeyValueOperation,
) -> Result<ReferenceServiceTransition, ReferenceServiceIssue> {
    let mut next = state.clone();
    let decision = match operation {
        TransactionalKeyValueOperation::Commit {
            expected_version,
            writes,
        } => {
            validate_writes(writes)?;
            if *expected_version != state.version {
                ReferenceTransitionDecision::Conflict
            } else {
                next.version =
                    next.version.checked_add(VERSION_INCREMENT).ok_or(ReferenceServiceIssue::VersionOverflow)?;
                for (key, value_ref) in writes {
                    next.values.insert(key.clone(), value_ref.clone());
                }
                ReferenceTransitionDecision::Applied
            }
        }
        TransactionalKeyValueOperation::Recover {
            checkpoint_version,
            checkpoint_values,
        } => {
            validate_writes(
                &checkpoint_values.iter().map(|(key, value)| (key.clone(), value.clone())).collect::<Vec<_>>(),
            )?;
            next.version = *checkpoint_version;
            next.values = checkpoint_values.clone();
            next.recovered_version = Some(*checkpoint_version);
            ReferenceTransitionDecision::Applied
        }
    };
    let next_state = ReferenceServiceState::TransactionalKeyValue(next);
    Ok(reference_transition(next_state, decision, super::REQUIRED_SIMULATION_PORT_CLASSES.to_vec(), vec![
        "transaction-version-monotonic",
        "conflict-does-not-mutate",
    ]))
}

fn apply_log(
    state: &ReplicatedLogState,
    operation: &ReplicatedLogOperation,
) -> Result<ReferenceServiceTransition, ReferenceServiceIssue> {
    let mut next = state.clone();
    let decision = match operation {
        ReplicatedLogOperation::Append { payload_ref } => {
            validate_reference("log-payload-ref", payload_ref)?;
            let offset = next.next_offset;
            next.next_offset =
                next.next_offset.checked_add(LOG_OFFSET_INCREMENT).ok_or(ReferenceServiceIssue::LogOffsetOverflow)?;
            next.entries.push(ReplicatedLogEntry {
                offset,
                payload_ref: payload_ref.clone(),
            });
            ReferenceTransitionDecision::Applied
        }
        ReplicatedLogOperation::ReplicateThrough { offset } => {
            if !next.entries.iter().any(|entry| entry.offset == *offset) {
                return Err(ReferenceServiceIssue::UnknownLogOffset(*offset));
            }
            next.replicated_through = Some(*offset);
            ReferenceTransitionDecision::Applied
        }
        ReplicatedLogOperation::RetainFrom { offset } => {
            if next.replicated_through.is_none_or(|replicated| *offset > replicated) {
                return Err(ReferenceServiceIssue::RetentionBeyondReplication {
                    retained_from: *offset,
                    replicated_through: next.replicated_through,
                });
            }
            next.entries.retain(|entry| entry.offset >= *offset);
            next.retained_from = *offset;
            ReferenceTransitionDecision::Applied
        }
        ReplicatedLogOperation::Recover {
            entries,
            replicated_through,
        } => {
            validate_log_entries(entries)?;
            next.entries = entries.clone();
            next.next_offset = match entries.last() {
                None => INITIAL_LOG_OFFSET,
                Some(entry) => {
                    entry.offset.checked_add(LOG_OFFSET_INCREMENT).ok_or(ReferenceServiceIssue::LogOffsetOverflow)?
                }
            };
            next.retained_from = entries.first().map_or(INITIAL_LOG_OFFSET, |entry| entry.offset);
            next.replicated_through = *replicated_through;
            ReferenceTransitionDecision::Applied
        }
    };
    let next_state = ReferenceServiceState::ReplicatedLog(next);
    Ok(reference_transition(next_state, decision, super::REQUIRED_SIMULATION_PORT_CLASSES.to_vec(), vec![
        "log-offsets-contiguous",
        "retention-follows-replication",
    ]))
}

fn apply_scheduler(
    state: &DistributedSchedulerState,
    operation: &DistributedSchedulerOperation,
) -> Result<ReferenceServiceTransition, ReferenceServiceIssue> {
    let mut next = state.clone();
    let decision = match operation {
        DistributedSchedulerOperation::Submit { job_id } => {
            validate_identifier("job-id", job_id)?;
            if next.jobs.contains_key(job_id) {
                return Err(ReferenceServiceIssue::DuplicateJob(job_id.clone()));
            }
            next.jobs.insert(job_id.clone(), ReferenceJobState {
                phase: ReferenceJobPhase::Pending,
                lease_owner: None,
                lease_epoch: INITIAL_LEASE_EPOCH,
                authoritative_completion_ref: None,
            });
            ReferenceTransitionDecision::Applied
        }
        DistributedSchedulerOperation::Lease { job_id, owner } => {
            validate_identifier("job-id", job_id)?;
            validate_identifier("lease-owner", owner)?;
            let job = next.jobs.get_mut(job_id).ok_or_else(|| ReferenceServiceIssue::UnknownJob(job_id.clone()))?;
            require_phase(job_id, job.phase, ReferenceJobPhase::Pending)?;
            job.lease_epoch = job
                .lease_epoch
                .checked_add(LEASE_EPOCH_INCREMENT)
                .ok_or(ReferenceServiceIssue::LeaseEpochOverflow)?;
            job.phase = ReferenceJobPhase::Leased;
            job.lease_owner = Some(owner.clone());
            ReferenceTransitionDecision::Applied
        }
        DistributedSchedulerOperation::Complete {
            job_id,
            owner,
            completion_ref,
        } => {
            validate_identifier("job-id", job_id)?;
            validate_identifier("completion-owner", owner)?;
            validate_reference("completion-ref", completion_ref)?;
            let job = next.jobs.get_mut(job_id).ok_or_else(|| ReferenceServiceIssue::UnknownJob(job_id.clone()))?;
            if job.authoritative_completion_ref.is_some() {
                return Err(ReferenceServiceIssue::DuplicateAuthoritativeCompletion(job_id.clone()));
            }
            require_phase(job_id, job.phase, ReferenceJobPhase::Leased)?;
            let expected =
                job.lease_owner.as_deref().ok_or_else(|| ReferenceServiceIssue::MissingLeaseOwner(job_id.clone()))?;
            if expected != owner {
                return Err(ReferenceServiceIssue::LeaseOwnerMismatch {
                    job_id: job_id.clone(),
                    expected: expected.to_string(),
                    actual: owner.clone(),
                });
            }
            job.phase = ReferenceJobPhase::Completed;
            job.authoritative_completion_ref = Some(completion_ref.clone());
            ReferenceTransitionDecision::Applied
        }
        DistributedSchedulerOperation::Failover { job_id, next_owner } => {
            validate_identifier("job-id", job_id)?;
            validate_identifier("failover-owner", next_owner)?;
            let job = next.jobs.get_mut(job_id).ok_or_else(|| ReferenceServiceIssue::UnknownJob(job_id.clone()))?;
            require_phase(job_id, job.phase, ReferenceJobPhase::Leased)?;
            job.lease_epoch = job
                .lease_epoch
                .checked_add(LEASE_EPOCH_INCREMENT)
                .ok_or(ReferenceServiceIssue::LeaseEpochOverflow)?;
            job.lease_owner = Some(next_owner.clone());
            ReferenceTransitionDecision::Applied
        }
    };
    let next_state = ReferenceServiceState::DistributedScheduler(next);
    Ok(reference_transition(next_state, decision, super::REQUIRED_SIMULATION_PORT_CLASSES.to_vec(), vec![
        "single-authoritative-completion",
        "completion-requires-current-lease",
    ]))
}

fn reference_transition(
    next: ReferenceServiceState,
    decision: ReferenceTransitionDecision,
    required_ports: Vec<FabricPortClass>,
    semantic_invariants: Vec<&'static str>,
) -> ReferenceServiceTransition {
    let state_material = format!("{next:?}");
    ReferenceServiceTransition {
        next,
        decision,
        required_ports,
        semantic_invariants,
        state_material,
    }
}

fn validate_writes(writes: &[(String, String)]) -> Result<(), ReferenceServiceIssue> {
    let mut keys = std::collections::BTreeSet::new();
    for (key, value_ref) in writes {
        validate_identifier("transaction-key", key)?;
        validate_reference("transaction-value-ref", value_ref)?;
        if !keys.insert(key.clone()) {
            return Err(ReferenceServiceIssue::DuplicateWriteKey(key.clone()));
        }
    }
    Ok(())
}

fn validate_log_entries(entries: &[ReplicatedLogEntry]) -> Result<(), ReferenceServiceIssue> {
    let mut expected = entries.first().map_or(INITIAL_LOG_OFFSET, |entry| entry.offset);
    for entry in entries {
        validate_reference("log-entry-ref", &entry.payload_ref)?;
        if entry.offset != expected {
            return Err(ReferenceServiceIssue::LogOffsetGap {
                expected,
                actual: entry.offset,
            });
        }
        expected = expected.checked_add(LOG_OFFSET_INCREMENT).ok_or(ReferenceServiceIssue::LogOffsetOverflow)?;
    }
    Ok(())
}

fn require_phase(
    job_id: &str,
    actual: ReferenceJobPhase,
    expected: ReferenceJobPhase,
) -> Result<(), ReferenceServiceIssue> {
    if actual == expected {
        Ok(())
    } else {
        Err(ReferenceServiceIssue::InvalidJobPhase {
            job_id: job_id.to_string(),
            expected,
            actual,
        })
    }
}

fn validate_reference(field: &'static str, value: &str) -> Result<(), ReferenceServiceIssue> {
    if valid_ref(value) {
        Ok(())
    } else {
        Err(ReferenceServiceIssue::MalformedRef {
            field,
            value: value.to_string(),
        })
    }
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ReferenceServiceIssue> {
    let valid = !value.is_empty()
        && value.len() <= super::MAX_WORLD_IDENTIFIER_BYTES
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | ':' | '-' | '_'));
    if valid {
        Ok(())
    } else {
        Err(ReferenceServiceIssue::MalformedIdentifier {
            field,
            value: value.to_string(),
        })
    }
}

pub const fn initial_transaction_version() -> u64 {
    INITIAL_STATE_VERSION
}
