//! Storage-fault outcomes, adapter quarantine, and read-back reconciliation.
//!
//! A shell reports one typed observation of what storage did with a mutation.
//! The core separates the requested durability from that observation, never
//! turns an uncertain commit or synchronization into durable success, and
//! keeps an adapter quarantined after an uncertain, corrupt, repaired, or
//! inconsistent observation until an explicit reopen and read-back
//! reconciliation succeeds. Retrying the call that failed never clears
//! quarantine. No storage error value reaches this core.

use super::*;

#[cfg(test)]
mod tests;

/// What the shell observed after asking storage to apply a mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PersistenceObservation {
    /// Storage refused the mutation before any byte could change.
    NotApplied,
    /// The mutation is visible to the process; no synchronization was observed.
    AppliedProcessVisible,
    /// The adapter observed the synchronization the requested level needs.
    SyncObserved,
    /// A commit or synchronization error that cannot prove non-application.
    OutcomeUnknown,
    /// A decode, checksum, repair, or invariant failure.
    CorruptOrInconsistent,
}

/// The core's reading of one observation against the requested durability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PersistenceClassification {
    /// Acknowledge the mutation at the requested level.
    Acknowledged(DurabilityLevel),
    /// Nothing changed; the caller may decide to retry.
    NotApplied,
    /// Visible, but the requested durability was not observed: no durable
    /// acknowledgement.
    VisibleNotDurable,
    /// Whether the mutation reached storage is unknown.
    OutcomeUnknown,
    /// Storage returned corrupt or inconsistent state.
    CorruptOrInconsistent,
}

/// Classify one observation against the requested durability.
///
/// Only an observation that satisfies the requested level acknowledges it:
/// process visibility satisfies buffered and process-loss durability, and
/// machine-loss durability needs an observed synchronization.
// r[impl molten.fabric_durability.storage_fault_outcomes]
pub const fn classify_persistence(
    requested: DurabilityLevel,
    observation: PersistenceObservation,
) -> PersistenceClassification {
    match observation {
        PersistenceObservation::NotApplied => PersistenceClassification::NotApplied,
        PersistenceObservation::SyncObserved => PersistenceClassification::Acknowledged(requested),
        PersistenceObservation::AppliedProcessVisible => match requested {
            DurabilityLevel::Buffered | DurabilityLevel::ProcessLoss => {
                PersistenceClassification::Acknowledged(requested)
            }
            DurabilityLevel::MachineLoss => PersistenceClassification::VisibleNotDurable,
        },
        PersistenceObservation::OutcomeUnknown => PersistenceClassification::OutcomeUnknown,
        PersistenceObservation::CorruptOrInconsistent => PersistenceClassification::CorruptOrInconsistent,
    }
}

/// Why an adapter is quarantined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuarantineReason {
    OutcomeUnknown,
    CorruptOrInconsistent,
    /// Storage repaired its own structures when it opened.
    RepairedOnOpen,
    /// Read-back after reopen matched neither the acknowledged state nor the
    /// state of the uncertain mutation.
    ReconciliationMismatch,
    /// No acknowledged reference exists to reconcile a read-back against.
    ReferenceUnavailable,
}

/// Whether an adapter instance may serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdapterHealth {
    Serving,
    Quarantined(QuarantineReason),
}

/// The health after a classified observation. Quarantine is sticky: a later
/// success, including a retry of the failed call, leaves it in place.
// r[impl molten.fabric_durability.adapter_quarantine]
pub const fn health_after(health: AdapterHealth, classification: PersistenceClassification) -> AdapterHealth {
    match (health, classification) {
        (AdapterHealth::Quarantined(reason), _) => AdapterHealth::Quarantined(reason),
        (AdapterHealth::Serving, PersistenceClassification::OutcomeUnknown) => {
            AdapterHealth::Quarantined(QuarantineReason::OutcomeUnknown)
        }
        (AdapterHealth::Serving, PersistenceClassification::CorruptOrInconsistent) => {
            AdapterHealth::Quarantined(QuarantineReason::CorruptOrInconsistent)
        }
        (AdapterHealth::Serving, _) => AdapterHealth::Serving,
    }
}

/// The kind of operation a caller asks an adapter for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdapterOperation {
    Read,
    Mutation,
    /// The adapter's health and last acknowledged status.
    Status,
    /// An explicit reopen and reconciliation.
    Reopen,
}

/// Admit an operation. A quarantined adapter admits only status and reopen:
/// it never serves reads or mutations from cached state as durable truth.
// r[impl molten.fabric_durability.adapter_quarantine]
pub const fn admit_adapter_operation(
    health: AdapterHealth,
    operation: AdapterOperation,
) -> Result<(), QuarantineReason> {
    match (health, operation) {
        (AdapterHealth::Serving, _)
        | (AdapterHealth::Quarantined(_), AdapterOperation::Status | AdapterOperation::Reopen) => Ok(()),
        (AdapterHealth::Quarantined(reason), AdapterOperation::Read | AdapterOperation::Mutation) => Err(reason),
    }
}

/// What reconciliation found the read-back to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReconciledState {
    /// The read-back equals the last acknowledged state: the uncertain
    /// mutation, if any, did not apply.
    Acknowledged,
    /// The read-back equals the state of the uncertain mutation: it applied.
    Pending,
    /// No trusted reference exists; the read-back passed its own consistency
    /// checks. The repair that required this stays recorded.
    SelfConsistent,
}

/// The facts of one explicit reopen.
#[derive(Debug, Clone, Copy)]
pub struct ReconciliationInput<'a> {
    /// The last state the adapter acknowledged, if this adapter instance
    /// knows it. A fresh open after a crash does not.
    pub acknowledged: Option<&'a DurableState>,
    /// The state the uncertain mutation would have produced, if any.
    pub pending: Option<&'a DurableState>,
    /// The state read back from storage after reopen.
    pub read_back: &'a DurableState,
    /// Storage repaired its own structures during the reopen.
    pub is_repair_observed: bool,
}

/// Reconcile a read-back after an explicit reopen.
///
/// The adapter serves again only when the read-back equals the trusted
/// reference or the state of the uncertain mutation. A store repair rebuilds
/// structure, not content: it is acceptable only when the read-back matches a
/// trusted reference exactly. Without a reference, a repaired store stays
/// quarantined, and any other read-back keeps it quarantined.
// r[impl molten.fabric_durability.adapter_quarantine]
pub fn reconcile_after_reopen(input: &ReconciliationInput<'_>) -> Result<ReconciledState, QuarantineReason> {
    let Some(acknowledged) = input.acknowledged else {
        return Err(if input.is_repair_observed {
            QuarantineReason::RepairedOnOpen
        } else {
            QuarantineReason::ReferenceUnavailable
        });
    };
    if durable_content_matches(acknowledged, input.read_back) {
        return Ok(ReconciledState::Acknowledged);
    }
    if input.pending.is_some_and(|pending| durable_content_matches(pending, input.read_back)) {
        return Ok(ReconciledState::Pending);
    }
    Err(QuarantineReason::ReconciliationMismatch)
}

/// Reconcile a store that has no trusted reference by its own consistency.
///
/// The read-back must decode, its recovery evaluation must admit it, and every
/// snapshot payload must match its identity. This is storage reconciliation
/// only: a repaired store stays suspect for any protocol that needs proof of
/// retained history, such as Raft voting.
// r[impl molten.fabric_durability.adapter_quarantine]
pub const fn reconcile_self_consistent(
    disposition: RecoveryDisposition,
    is_payload_consistent: bool,
) -> Result<ReconciledState, QuarantineReason> {
    match (disposition, is_payload_consistent) {
        (RecoveryDisposition::Admit, true) => Ok(ReconciledState::SelfConsistent),
        _ => Err(QuarantineReason::ReconciliationMismatch),
    }
}

/// Whether two states hold the same durable content. Buffered records are
/// process memory and never survive a reopen.
pub fn durable_content_matches(left: &DurableState, right: &DurableState) -> bool {
    left.descriptor == right.descriptor
        && left.durable_log == right.durable_log
        && left.ordered == right.ordered
        && left.snapshots == right.snapshots
        && left.effects == right.effects
}

/// How a snapshot payload publication ended, as the publication shell reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapshotPublicationObservation {
    /// The payload never became visible.
    NotCommitted,
    /// A payload with the same content-addressed name already exists.
    DestinationExists,
    /// The payload and its parent directory were synchronized.
    CommittedAndParentSynchronized,
    /// The payload became visible, but parent synchronization failed or was not observed.
    CommittedDurabilityUnknown,
}

/// What the snapshot shell does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapshotMetadataDecision {
    /// Commit the snapshot metadata.
    CommitMetadata,
    /// Verify the existing destination's content identity, then commit metadata.
    VerifyExistingThenCommit,
    /// Commit nothing; the payload is not visible.
    Abandon,
    /// Commit nothing and quarantine: the payload is visible with unknown
    /// durability, and startup reconciliation must check its identity before
    /// any metadata repair.
    QuarantineUntilReconciled,
}

/// Decide the metadata step after one snapshot payload publication. Metadata
/// becomes visible only after the payload and its parent are synchronized.
// r[impl molten.fabric_durability.snapshot_publication]
pub const fn snapshot_metadata_decision(observation: SnapshotPublicationObservation) -> SnapshotMetadataDecision {
    match observation {
        SnapshotPublicationObservation::NotCommitted => SnapshotMetadataDecision::Abandon,
        SnapshotPublicationObservation::DestinationExists => SnapshotMetadataDecision::VerifyExistingThenCommit,
        SnapshotPublicationObservation::CommittedAndParentSynchronized => SnapshotMetadataDecision::CommitMetadata,
        SnapshotPublicationObservation::CommittedDurabilityUnknown => {
            SnapshotMetadataDecision::QuarantineUntilReconciled
        }
    }
}
