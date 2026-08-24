//! Pure durable-state port contracts and deterministic transition laws.
//!
//! This module owns no filesystem, Redb, clock, process, or network effects.
//! Adapter shells persist only transitions admitted by these functions.

pub mod cache;
pub mod ownership;

mod transition;

use std::collections::BTreeMap;

pub use transition::*;

use crate::fabric::valid_blake3_ref;
use crate::fabric::valid_fabric_token;

pub const DURABLE_STATE_PROFILE_SCHEMA: &str = "molten.fabric.durability.profile.v1";
pub const DURABLE_STATE_NAMESPACE_SCHEMA: &str = "molten.fabric.durability.namespace.v1";
pub const DURABLE_STATE_OPERATION_SCHEMA: &str = "molten.fabric.durability.operation.v1";
pub const DURABLE_STATE_OUTCOME_SCHEMA: &str = "molten.fabric.durability.outcome.v1";
pub const DURABLE_STATE_SNAPSHOT_SCHEMA: &str = "molten.fabric.durability.snapshot.v1";
pub const DURABLE_STATE_EFFECT_SCHEMA: &str = "molten.fabric.durability.effect-transaction.v1";
pub const DURABLE_STATE_RECOVERY_SCHEMA: &str = "molten.fabric.durability.recovery.v1";

pub const MAX_DURABILITY_COLLECTION_ITEMS: usize = 4_096;
pub const MAX_DURABILITY_TEXT_BYTES: usize = 256;
const REQUIRED_DURABILITY_NON_CLAIM_COUNT: usize = 8;
const ADJACENT_PAIR_WIDTH: usize = 2;
pub const REQUIRED_DURABILITY_NON_CLAIMS: [DurabilityNonClaim; REQUIRED_DURABILITY_NON_CLAIM_COUNT] = [
    DurabilityNonClaim::NoReplication,
    DurabilityNonClaim::NoConsensus,
    DurabilityNonClaim::NoDistributedTransaction,
    DurabilityNonClaim::NoLinearizability,
    DurabilityNonClaim::NoRemotePersistence,
    DurabilityNonClaim::NoExtensionCorrectness,
    DurabilityNonClaim::NoRecoveryCorrectness,
    DurabilityNonClaim::NoDeletionAuthority,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DurabilityNonClaim {
    NoReplication,
    NoConsensus,
    NoDistributedTransaction,
    NoLinearizability,
    NoRemotePersistence,
    NoExtensionCorrectness,
    NoRecoveryCorrectness,
    NoDeletionAuthority,
}

impl DurabilityNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoReplication => "does-not-prove-replication",
            Self::NoConsensus => "does-not-prove-consensus",
            Self::NoDistributedTransaction => "does-not-prove-distributed-transactions",
            Self::NoLinearizability => "does-not-prove-linearizability",
            Self::NoRemotePersistence => "does-not-prove-remote-persistence",
            Self::NoExtensionCorrectness => "does-not-prove-extension-correctness",
            Self::NoRecoveryCorrectness => "does-not-prove-recovery-correctness",
            Self::NoDeletionAuthority => "does-not-grant-deletion-authority",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DurableAdapterKind {
    LiveRedb,
    DeterministicSimulation,
}

impl DurableAdapterKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LiveRedb => "live-redb",
            Self::DeterministicSimulation => "deterministic-simulation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DurabilityLevel {
    Buffered,
    ProcessLoss,
    MachineLoss,
}

impl DurabilityLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Buffered => "buffered",
            Self::ProcessLoss => "process-loss",
            Self::MachineLoss => "machine-loss",
        }
    }

    pub const fn is_durable(self) -> bool {
        !matches!(self, Self::Buffered)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationOutcome {
    Validated,
    Buffered,
    Durable,
    FailedBeforeMutation,
    FailedAfterPossibleMutation,
    CancelledBeforeMutation,
    CancelledAfterPossibleMutation,
    Uncertain,
    PreconditionFailed,
    DuplicateTerminal,
}

impl MutationOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validated => "validated",
            Self::Buffered => "buffered",
            Self::Durable => "durable",
            Self::FailedBeforeMutation => "failed-before-mutation",
            Self::FailedAfterPossibleMutation => "failed-after-possible-mutation",
            Self::CancelledBeforeMutation => "cancelled-before-mutation",
            Self::CancelledAfterPossibleMutation => "cancelled-after-possible-mutation",
            Self::Uncertain => "uncertain",
            Self::PreconditionFailed => "precondition-failed",
            Self::DuplicateTerminal => "duplicate-terminal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableStateProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub adapter_kind: DurableAdapterKind,
    pub supported_levels: Vec<DurabilityLevel>,
    pub max_namespaces: u64,
    pub max_log_records: u64,
    pub max_ordered_entries: u64,
    pub max_operation_bytes: u64,
    pub max_namespace_bytes: u64,
    pub max_batch_operations: u64,
    pub max_snapshots: u64,
    pub max_effect_transactions: u64,
    pub non_claims: Vec<DurabilityNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicityDomain {
    pub domain_id: String,
    pub adapter_id: String,
    pub namespace_id: String,
    pub generation: u64,
    pub object_classes: Vec<DurableObjectClass>,
    pub max_operations: u64,
    pub max_bytes: u64,
    pub supported_levels: Vec<DurabilityLevel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DurableObjectClass {
    LogRecord,
    OrderedValue,
    Snapshot,
    Checkpoint,
    EffectTransaction,
}

impl DurableObjectClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LogRecord => "log-record",
            Self::OrderedValue => "ordered-value",
            Self::Snapshot => "snapshot",
            Self::Checkpoint => "checkpoint",
            Self::EffectTransaction => "effect-transaction",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableNamespaceDescriptor {
    pub schema: String,
    pub profile_ref: String,
    pub adapter_id: String,
    pub namespace_id: String,
    pub generation: u64,
    pub value_schema_ref: String,
    pub atomicity_domain: AtomicityDomain,
    pub retention_authority_ref: Option<String>,
    pub quota_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    pub sequence: u64,
    pub value: Vec<u8>,
    pub value_ref: String,
    pub durability: DurabilityLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionedValue {
    pub value: Vec<u8>,
    pub value_ref: String,
    pub version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotKind {
    Snapshot,
    Checkpoint,
}

impl SnapshotKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
            Self::Checkpoint => "checkpoint",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRecord {
    pub kind: SnapshotKind,
    pub snapshot_ref: String,
    pub content_ref: String,
    pub source_namespace: String,
    pub source_generation: u64,
    pub value_schema_ref: String,
    pub covered_log_sequence: Option<u64>,
    pub ordered_state_ref: String,
    pub durability: DurabilityLevel,
    pub corrupted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectTransactionPhase {
    Reserved,
    Committed,
    Aborted,
    Expired,
    Uncertain,
    ReconciledCommitted,
    ReconciledAborted,
}

impl EffectTransactionPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Committed => "committed",
            Self::Aborted => "aborted",
            Self::Expired => "expired",
            Self::Uncertain => "uncertain",
            Self::ReconciledCommitted => "reconciled-committed",
            Self::ReconciledAborted => "reconciled-aborted",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Committed | Self::Aborted | Self::Expired | Self::ReconciledCommitted | Self::ReconciledAborted
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectTransactionProfile {
    pub durable_reservation: bool,
    pub exclusive: bool,
    pub expiring: bool,
    pub idempotent_commit: bool,
    pub compensating_abort: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectTransactionState {
    pub transaction_id: String,
    pub generation: u64,
    pub operation_ref: String,
    pub phase: EffectTransactionPhase,
    pub expires_at_tick: Option<u64>,
    pub profile: EffectTransactionProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableState {
    pub descriptor: DurableNamespaceDescriptor,
    pub buffered_log: Vec<LogRecord>,
    pub durable_log: Vec<LogRecord>,
    pub ordered: BTreeMap<Vec<u8>, VersionedValue>,
    pub snapshots: BTreeMap<String, SnapshotRecord>,
    pub effects: BTreeMap<String, EffectTransactionState>,
    pub buffered_bytes: u64,
    pub durable_bytes: u64,
}

impl DurableState {
    pub fn empty(descriptor: DurableNamespaceDescriptor) -> Self {
        Self {
            descriptor,
            buffered_log: Vec::new(),
            durable_log: Vec::new(),
            ordered: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            effects: BTreeMap::new(),
            buffered_bytes: 0,
            durable_bytes: 0,
        }
    }

    pub fn next_log_sequence(&self) -> Result<u64, DurabilityIssue> {
        let last = self.buffered_log.last().or_else(|| self.durable_log.last()).map_or(0, |record| record.sequence);
        if self.buffered_log.is_empty() && self.durable_log.is_empty() {
            return Ok(0);
        }
        last.checked_add(1).ok_or(DurabilityIssue::SequenceOverflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppendRequest {
    pub adapter_id: String,
    pub namespace_id: String,
    pub generation: u64,
    pub expected_sequence: u64,
    pub value: Vec<u8>,
    pub value_ref: String,
    pub durability: DurabilityLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValuePrecondition {
    Any,
    Missing,
    Version(u64),
    ValueRef(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderedMutation {
    Put {
        key: Vec<u8>,
        value: Vec<u8>,
        value_ref: String,
        precondition: ValuePrecondition,
    },
    Delete {
        key: Vec<u8>,
        precondition: ValuePrecondition,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicBatchRequest {
    pub domain: AtomicityDomain,
    pub generation: u64,
    pub mutations: Vec<OrderedMutation>,
    pub durability: DurabilityLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRequest {
    pub kind: SnapshotKind,
    pub generation: u64,
    pub snapshot_ref: String,
    pub content_ref: String,
    pub ordered_state_ref: String,
    pub covered_log_sequence: Option<u64>,
    pub durability: DurabilityLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectTransactionCommand {
    Reserve {
        transaction_id: String,
        generation: u64,
        operation_ref: String,
        expires_at_tick: Option<u64>,
        profile: EffectTransactionProfile,
    },
    Commit {
        transaction_id: String,
        generation: u64,
    },
    Abort {
        transaction_id: String,
        generation: u64,
    },
    Expire {
        transaction_id: String,
        generation: u64,
        observed_tick: u64,
    },
    MarkUncertain {
        transaction_id: String,
        generation: u64,
    },
    Reconcile {
        transaction_id: String,
        generation: u64,
        committed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableTransition {
    pub next: DurableState,
    pub outcome: MutationOutcome,
    pub operation: String,
    pub affected_items: u64,
    pub affected_bytes: u64,
    pub retry_safe: bool,
    pub reconciliation_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedScanRequest {
    pub start_inclusive: Option<Vec<u8>>,
    pub end_exclusive: Option<Vec<u8>>,
    pub limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedScanPage {
    pub entries: Vec<(Vec<u8>, VersionedValue)>,
    pub continuation: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogScanPage {
    pub records: Vec<LogRecord>,
    pub continuation: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRestorePlan {
    pub snapshot: SnapshotRecord,
    pub target_generation: u64,
    pub restored_state_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryDisposition {
    Admit,
    RepairRequired,
    QuarantineRequired,
    Deny,
}

impl RecoveryDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admit => "admit",
            Self::RepairRequired => "repair-required",
            Self::QuarantineRequired => "quarantine-required",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryInventory {
    pub active_generation: u64,
    pub expected_schema_ref: String,
    pub permit_repair: bool,
    pub permit_quarantine: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryDecision {
    pub disposition: RecoveryDisposition,
    pub diagnostics: Vec<DurabilityIssue>,
    pub durable_log_tail: Option<u64>,
    pub snapshot_count: u64,
    pub unresolved_effect_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurabilityIssue {
    ProfileSchemaMismatch,
    NamespaceSchemaMismatch,
    EmptyField(&'static str),
    MalformedField(&'static str),
    MalformedContentRef(&'static str),
    ZeroLimit(&'static str),
    DuplicateValue(&'static str),
    MissingNonClaim(DurabilityNonClaim),
    UnsupportedDurability(DurabilityLevel),
    AdapterMismatch,
    NamespaceMismatch,
    AtomicityDomainMismatch,
    CrossAdapterBatch,
    CrossNamespaceBatch,
    GenerationMismatch { expected: u64, actual: u64 },
    StaleGeneration { active: u64, requested: u64 },
    EmptyValue,
    EmptyKey,
    KeyRangeInvalid,
    OperationLimitExceeded { actual: u64, maximum: u64 },
    ByteLimitExceeded { actual: u64, maximum: u64 },
    NamespaceQuotaExceeded { actual: u64, maximum: u64 },
    SequenceMismatch { expected: u64, actual: u64 },
    SequenceOverflow,
    VersionOverflow,
    PreconditionFailed,
    RetentionAuthorityRequired,
    SnapshotLimitExceeded,
    SnapshotNotFound,
    SnapshotCorrupt,
    SnapshotSchemaMismatch,
    EffectLimitExceeded,
    EffectNotFound,
    EffectAlreadyExists,
    EffectTerminal(EffectTransactionPhase),
    EffectNotExpired,
    EffectReconciliationRequired,
    LogGap { expected: u64, actual: u64 },
    UnresolvedEffect(String),
    CollectionLimitExceeded,
}

pub fn validate_durable_profile(profile: &DurableStateProfile) -> Result<(), Vec<DurabilityIssue>> {
    let mut issues = Vec::new();
    if profile.schema != DURABLE_STATE_PROFILE_SCHEMA {
        issues.push(DurabilityIssue::ProfileSchemaMismatch);
    }
    validate_token("profile-id", &profile.profile_id, &mut issues);
    if !valid_blake3_ref(&profile.profile_ref) {
        issues.push(DurabilityIssue::MalformedContentRef("profile-ref"));
    }
    validate_positive_limit("max-namespaces", profile.max_namespaces, &mut issues);
    validate_positive_limit("max-log-records", profile.max_log_records, &mut issues);
    validate_positive_limit("max-ordered-entries", profile.max_ordered_entries, &mut issues);
    validate_positive_limit("max-operation-bytes", profile.max_operation_bytes, &mut issues);
    validate_positive_limit("max-namespace-bytes", profile.max_namespace_bytes, &mut issues);
    validate_positive_limit("max-batch-operations", profile.max_batch_operations, &mut issues);
    validate_positive_limit("max-snapshots", profile.max_snapshots, &mut issues);
    validate_positive_limit("max-effect-transactions", profile.max_effect_transactions, &mut issues);
    validate_unique("supported-levels", &profile.supported_levels, &mut issues);
    validate_unique("non-claims", &profile.non_claims, &mut issues);
    for required in REQUIRED_DURABILITY_NON_CLAIMS {
        if !profile.non_claims.contains(&required) {
            issues.push(DurabilityIssue::MissingNonClaim(required));
        }
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

pub fn validate_namespace_descriptor(
    profile: &DurableStateProfile,
    descriptor: &DurableNamespaceDescriptor,
) -> Result<(), Vec<DurabilityIssue>> {
    let mut issues = validate_durable_profile(profile).err().unwrap_or_default();
    if descriptor.schema != DURABLE_STATE_NAMESPACE_SCHEMA {
        issues.push(DurabilityIssue::NamespaceSchemaMismatch);
    }
    if descriptor.profile_ref != profile.profile_ref {
        issues.push(DurabilityIssue::MalformedContentRef("namespace-profile-ref"));
    }
    validate_token("adapter-id", &descriptor.adapter_id, &mut issues);
    validate_token("namespace-id", &descriptor.namespace_id, &mut issues);
    if descriptor.generation == 0 {
        issues.push(DurabilityIssue::ZeroLimit("generation"));
    }
    if !valid_blake3_ref(&descriptor.value_schema_ref) {
        issues.push(DurabilityIssue::MalformedContentRef("value-schema-ref"));
    }
    if let Some(authority_ref) = &descriptor.retention_authority_ref
        && !valid_blake3_ref(authority_ref)
    {
        issues.push(DurabilityIssue::MalformedContentRef("retention-authority-ref"));
    }
    validate_positive_limit("quota-bytes", descriptor.quota_bytes, &mut issues);
    validate_atomicity_domain(profile, descriptor, &mut issues);
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

fn validate_atomicity_domain(
    profile: &DurableStateProfile,
    descriptor: &DurableNamespaceDescriptor,
    issues: &mut Vec<DurabilityIssue>,
) {
    let domain = &descriptor.atomicity_domain;
    validate_token("atomicity-domain-id", &domain.domain_id, issues);
    if domain.adapter_id != descriptor.adapter_id {
        issues.push(DurabilityIssue::AdapterMismatch);
    }
    if domain.namespace_id != descriptor.namespace_id {
        issues.push(DurabilityIssue::NamespaceMismatch);
    }
    if domain.generation != descriptor.generation {
        issues.push(DurabilityIssue::GenerationMismatch {
            expected: descriptor.generation,
            actual: domain.generation,
        });
    }
    if domain.object_classes.is_empty() {
        issues.push(DurabilityIssue::EmptyField("atomicity-object-classes"));
    }
    validate_unique("atomicity-object-classes", &domain.object_classes, issues);
    validate_unique("atomicity-supported-levels", &domain.supported_levels, issues);
    validate_positive_limit("atomicity-max-operations", domain.max_operations, issues);
    validate_positive_limit("atomicity-max-bytes", domain.max_bytes, issues);
    if domain.max_operations > profile.max_batch_operations {
        issues.push(DurabilityIssue::OperationLimitExceeded {
            actual: domain.max_operations,
            maximum: profile.max_batch_operations,
        });
    }
    if domain.max_bytes > profile.max_namespace_bytes {
        issues.push(DurabilityIssue::ByteLimitExceeded {
            actual: domain.max_bytes,
            maximum: profile.max_namespace_bytes,
        });
    }
    for level in &domain.supported_levels {
        if !profile.supported_levels.contains(level) {
            issues.push(DurabilityIssue::UnsupportedDurability(*level));
        }
    }
}

fn validate_token(field: &'static str, value: &str, issues: &mut Vec<DurabilityIssue>) {
    if value.is_empty() {
        issues.push(DurabilityIssue::EmptyField(field));
    } else if value.len() > MAX_DURABILITY_TEXT_BYTES || !valid_fabric_token(value) {
        issues.push(DurabilityIssue::MalformedField(field));
    }
}

fn validate_positive_limit(field: &'static str, value: u64, issues: &mut Vec<DurabilityIssue>) {
    if value == 0 {
        issues.push(DurabilityIssue::ZeroLimit(field));
    }
}

fn validate_unique<T: Ord>(field: &'static str, values: &[T], issues: &mut Vec<DurabilityIssue>) {
    let mut sorted = values.iter().collect::<Vec<_>>();
    sorted.sort();
    if sorted.windows(ADJACENT_PAIR_WIDTH).any(|pair| pair[0] == pair[1]) {
        issues.push(DurabilityIssue::DuplicateValue(field));
    }
    if values.len() > MAX_DURABILITY_COLLECTION_ITEMS {
        issues.push(DurabilityIssue::CollectionLimitExceeded);
    }
}

#[cfg(test)]
mod tests;
