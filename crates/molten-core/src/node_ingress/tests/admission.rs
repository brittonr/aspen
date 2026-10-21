use super::*;

#[test]
fn unobserved_dedup_never_permits_effects() {
    let mut observation = input(DedupObservation::Unobserved);
    assert_eq!(reconcile(&observation), PublicationDecision::CheckDedup);
    observation.inbox = bound();
    observation.queue = bound();
    observation.dispatch = bound();
    assert_eq!(reconcile(&observation), PublicationDecision::CheckDedup);
}

// r[verify molten.audit_f03.preserve_state]
#[test]
fn rejected_or_unknown_dedup_cannot_reuse_even_completed_evidence() {
    for (dedup, expected) in [
        (DedupObservation::Rejected, PublicationDecision::Rejected),
        (DedupObservation::OutcomeUnknown, PublicationDecision::OutcomeUnknown),
    ] {
        let mut observation = input(dedup);
        assert_eq!(reconcile(&observation), expected);
        observation.inbox = bound();
        observation.queue = bound();
        observation.dispatch = bound();
        assert_eq!(reconcile(&observation), expected);
    }
}

// r[verify molten.audit_f03.uncertainty]
#[test]
fn unavailable_observations_block_new_and_completed_results() {
    for dedup in [
        DedupObservation::Unobserved,
        DedupObservation::First,
        DedupObservation::Duplicate,
    ] {
        let mut observation = input(dedup);
        observation.dispatch = bound();
        observation.inbox = PublicationObservation::Unavailable;
        assert_eq!(reconcile(&observation), PublicationDecision::OutcomeUnknown);
        observation.inbox = bound();
        observation.queue = PublicationObservation::Unavailable;
        assert_eq!(reconcile(&observation), PublicationDecision::OutcomeUnknown);
        observation.queue = bound();
        observation.dispatch = PublicationObservation::Unavailable;
        assert_eq!(reconcile(&observation), PublicationDecision::OutcomeUnknown);
    }
}

// r[verify molten.audit_f03.uncertainty]
#[test]
fn every_quarantine_reason_blocks_otherwise_supported_results() {
    for reason in [
        transactional_reconciliation_core::QuarantineReason::Corrupt,
        transactional_reconciliation_core::QuarantineReason::Inconsistent,
        transactional_reconciliation_core::QuarantineReason::CommitOutcomeUnknown,
        transactional_reconciliation_core::QuarantineReason::RepairReported,
        transactional_reconciliation_core::QuarantineReason::Missing,
    ] {
        for dedup in [
            DedupObservation::Unobserved,
            DedupObservation::First,
            DedupObservation::Duplicate,
        ] {
            let mut observation = input(dedup);
            observation.quarantine = transactional_reconciliation_core::QuarantineStatus::Quarantined(reason);
            assert_eq!(reconcile(&observation), PublicationDecision::OutcomeUnknown, "{observation:?}");
            observation.inbox = bound();
            observation.queue = bound();
            observation.dispatch = bound();
            assert_eq!(reconcile(&observation), PublicationDecision::OutcomeUnknown, "{observation:?}");
        }
    }
}
