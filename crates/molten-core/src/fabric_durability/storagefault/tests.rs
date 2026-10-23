use super::*;

const OBSERVATIONS: [PersistenceObservation; 5] = [
    PersistenceObservation::NotApplied,
    PersistenceObservation::AppliedProcessVisible,
    PersistenceObservation::SyncObserved,
    PersistenceObservation::OutcomeUnknown,
    PersistenceObservation::CorruptOrInconsistent,
];
const LEVELS: [DurabilityLevel; 3] = [
    DurabilityLevel::Buffered,
    DurabilityLevel::ProcessLoss,
    DurabilityLevel::MachineLoss,
];

// r[verify molten.fabric_durability.storage_fault_outcomes]
#[test]
fn only_an_observation_that_satisfies_the_requested_level_acknowledges_it() {
    for requested in LEVELS {
        assert_eq!(
            classify_persistence(requested, PersistenceObservation::SyncObserved),
            PersistenceClassification::Acknowledged(requested)
        );
    }
    assert_eq!(
        classify_persistence(DurabilityLevel::ProcessLoss, PersistenceObservation::AppliedProcessVisible),
        PersistenceClassification::Acknowledged(DurabilityLevel::ProcessLoss)
    );
    assert_eq!(
        classify_persistence(DurabilityLevel::MachineLoss, PersistenceObservation::AppliedProcessVisible),
        PersistenceClassification::VisibleNotDurable,
        "process visibility never stands in for machine-loss durability"
    );
}

// r[verify molten.fabric_durability.storage_fault_outcomes]
// r[verify molten.fabric_durability.storage_fault_validation]
#[test]
fn an_uncertain_or_corrupt_observation_never_acknowledges_durability() {
    for requested in LEVELS {
        for observation in OBSERVATIONS {
            let classification = classify_persistence(requested, observation);
            let is_acknowledged = matches!(classification, PersistenceClassification::Acknowledged(_));
            let is_uncertain = matches!(
                observation,
                PersistenceObservation::OutcomeUnknown
                    | PersistenceObservation::CorruptOrInconsistent
                    | PersistenceObservation::NotApplied
            );
            assert!(!(is_uncertain && is_acknowledged), "{observation:?} acknowledged {requested:?}");
        }
        assert_eq!(
            classify_persistence(requested, PersistenceObservation::OutcomeUnknown),
            PersistenceClassification::OutcomeUnknown
        );
    }
}

// r[verify molten.fabric_durability.adapter_quarantine]
#[test]
fn quarantine_follows_uncertainty_and_survives_a_later_success() {
    let unknown = health_after(AdapterHealth::Serving, PersistenceClassification::OutcomeUnknown);
    assert_eq!(unknown, AdapterHealth::Quarantined(QuarantineReason::OutcomeUnknown));
    let corrupt = health_after(AdapterHealth::Serving, PersistenceClassification::CorruptOrInconsistent);
    assert_eq!(corrupt, AdapterHealth::Quarantined(QuarantineReason::CorruptOrInconsistent));
    let retried = health_after(unknown, PersistenceClassification::Acknowledged(DurabilityLevel::MachineLoss));
    assert_eq!(retried, unknown, "a successful retry of the failed call does not clear quarantine");
    for classification in [
        PersistenceClassification::NotApplied,
        PersistenceClassification::VisibleNotDurable,
        PersistenceClassification::Acknowledged(DurabilityLevel::MachineLoss),
    ] {
        assert_eq!(health_after(AdapterHealth::Serving, classification), AdapterHealth::Serving);
    }
}

// r[verify molten.fabric_durability.adapter_quarantine]
#[test]
fn a_quarantined_adapter_admits_only_status_and_reopen() {
    let quarantined = AdapterHealth::Quarantined(QuarantineReason::OutcomeUnknown);
    for operation in [AdapterOperation::Read, AdapterOperation::Mutation] {
        assert_eq!(admit_adapter_operation(quarantined, operation), Err(QuarantineReason::OutcomeUnknown));
        assert_eq!(admit_adapter_operation(AdapterHealth::Serving, operation), Ok(()));
    }
    for operation in [AdapterOperation::Status, AdapterOperation::Reopen] {
        assert_eq!(admit_adapter_operation(quarantined, operation), Ok(()));
    }
}

fn state_with(sequence: u64) -> DurableState {
    let mut state = DurableState::empty(crate::fabric_durability::tests::descriptor());
    state.durable_log.push(LogRecord {
        sequence,
        value: vec![1, 2, 3],
        value_ref: format!("blake3:{sequence:064}"),
        durability: DurabilityLevel::MachineLoss,
    });
    state
}

// r[verify molten.fabric_durability.adapter_quarantine]
#[test]
fn reconciliation_resolves_an_unknown_outcome_only_from_a_matching_read_back() {
    let acknowledged = state_with(1);
    let pending = state_with(2);
    let input = |read_back: &DurableState, is_repair_observed: bool| -> Result<ReconciledState, QuarantineReason> {
        reconcile_after_reopen(&ReconciliationInput {
            acknowledged: Some(&acknowledged),
            pending: Some(&pending),
            read_back,
            is_repair_observed,
        })
    };
    assert_eq!(input(&acknowledged, false), Ok(ReconciledState::Acknowledged), "the mutation did not apply");
    assert_eq!(input(&pending, false), Ok(ReconciledState::Pending), "the mutation applied");
    assert_eq!(input(&state_with(3), false), Err(QuarantineReason::ReconciliationMismatch));
    assert_eq!(
        input(&acknowledged, true),
        Ok(ReconciledState::Acknowledged),
        "a repair is acceptable when the read-back matches a trusted reference"
    );
    assert_eq!(input(&state_with(3), true), Err(QuarantineReason::ReconciliationMismatch));

    let mut buffered = acknowledged.clone();
    buffered.buffered_log.push(state_with(9).durable_log[0].clone());
    assert!(durable_content_matches(&acknowledged, &buffered), "buffered records are not durable content");

    let fresh = reconcile_after_reopen(&ReconciliationInput {
        acknowledged: None,
        pending: None,
        read_back: &acknowledged,
        is_repair_observed: false,
    });
    assert_eq!(fresh, Err(QuarantineReason::ReferenceUnavailable));
    let repaired = reconcile_after_reopen(&ReconciliationInput {
        acknowledged: None,
        pending: None,
        read_back: &acknowledged,
        is_repair_observed: true,
    });
    assert_eq!(
        repaired,
        Err(QuarantineReason::RepairedOnOpen),
        "a repaired store without a reference stays quarantined"
    );
}

// r[verify molten.fabric_durability.adapter_quarantine]
#[test]
fn self_consistency_reconciles_only_an_admitted_store_with_matching_payloads() {
    assert_eq!(reconcile_self_consistent(RecoveryDisposition::Admit, true), Ok(ReconciledState::SelfConsistent));
    for (disposition, is_payload_consistent) in [
        (RecoveryDisposition::Admit, false),
        (RecoveryDisposition::RepairRequired, true),
        (RecoveryDisposition::QuarantineRequired, true),
        (RecoveryDisposition::Deny, true),
    ] {
        assert_eq!(
            reconcile_self_consistent(disposition, is_payload_consistent),
            Err(QuarantineReason::ReconciliationMismatch)
        );
    }
}

// r[verify molten.fabric_durability.snapshot_publication]
#[test]
fn snapshot_metadata_commits_only_after_payload_and_parent_synchronization() {
    assert_eq!(
        snapshot_metadata_decision(SnapshotPublicationObservation::CommittedAndParentSynchronized),
        SnapshotMetadataDecision::CommitMetadata
    );
    assert_eq!(
        snapshot_metadata_decision(SnapshotPublicationObservation::CommittedDurabilityUnknown),
        SnapshotMetadataDecision::QuarantineUntilReconciled
    );
    assert_eq!(
        snapshot_metadata_decision(SnapshotPublicationObservation::NotCommitted),
        SnapshotMetadataDecision::Abandon
    );
    assert_eq!(
        snapshot_metadata_decision(SnapshotPublicationObservation::DestinationExists),
        SnapshotMetadataDecision::VerifyExistingThenCommit
    );
}
