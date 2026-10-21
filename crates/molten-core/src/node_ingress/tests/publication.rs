use super::*;

// r[verify molten.audit_f03.recover_publication]
#[test]
fn only_reserved_absence_permits_fresh_or_recovery_enqueue() {
    for (dedup, expected) in [
        (DedupObservation::First, PublicationDecision::PublishFresh),
        (DedupObservation::Duplicate, PublicationDecision::RecoverPublication),
    ] {
        assert_eq!(reconcile(&input(dedup)), expected);
        for phase in [
            PublicationPhase::Publishing,
            PublicationPhase::Published,
            PublicationPhase::Dispatching,
            PublicationPhase::Dispatched,
        ] {
            let mut observation = input(dedup);
            observation.intent = Some(intent(phase));
            assert_eq!(reconcile(&observation), PublicationDecision::OutcomeUnknown, "{observation:?}");
        }
        let mut legacy = input(dedup);
        legacy.intent = None;
        assert_eq!(reconcile(&legacy), PublicationDecision::OutcomeUnknown);
    }
}

// r[verify molten.audit_f03.no_blind_replay]
#[test]
fn receipt_loss_windows_require_observed_evidence() {
    let absent = PublicationObservation::Absent;
    let bound = bound();
    for dedup in [DedupObservation::First, DedupObservation::Duplicate] {
        for (inbox, queue, dispatch, expected) in [
            (bound, absent, absent, PublicationDecision::RepairQueueReceipt),
            (bound, bound, absent, PublicationDecision::ReusePublication),
            (absent, bound, absent, PublicationDecision::OutcomeUnknown),
            (absent, absent, bound, PublicationDecision::CompletedDispatch),
            (bound, absent, bound, PublicationDecision::CompletedDispatch),
            (absent, bound, bound, PublicationDecision::CompletedDispatch),
            (bound, bound, bound, PublicationDecision::CompletedDispatch),
        ] {
            let mut observation = input(dedup);
            observation.inbox = inbox;
            observation.queue = queue;
            observation.dispatch = dispatch;
            assert_eq!(reconcile(&observation), expected, "{observation:?}");
        }
    }
}

#[test]
fn legacy_intent_absence_does_not_erase_matching_inbox() {
    let mut observation = input(DedupObservation::Duplicate);
    observation.intent = None;
    observation.inbox = bound();
    assert_eq!(reconcile(&observation), PublicationDecision::RepairQueueReceipt);
    observation.queue = bound();
    assert_eq!(reconcile(&observation), PublicationDecision::ReusePublication);
    observation.inbox = PublicationObservation::Absent;
    assert_eq!(reconcile(&observation), PublicationDecision::OutcomeUnknown);
    observation.dispatch = bound();
    assert_eq!(reconcile(&observation), PublicationDecision::CompletedDispatch);
}

#[test]
fn interrupted_enqueue_uses_inbox_instead_of_repeating_effects() {
    for phase in [
        PublicationPhase::Reserved,
        PublicationPhase::Publishing,
        PublicationPhase::Published,
    ] {
        let mut observation = input(DedupObservation::Duplicate);
        observation.intent = Some(intent(phase));
        observation.inbox = bound();
        assert_eq!(reconcile(&observation), PublicationDecision::RepairQueueReceipt, "{phase:?}");
        observation.queue = bound();
        assert_eq!(reconcile(&observation), PublicationDecision::ReusePublication, "{phase:?}");
    }
}

// r[verify molten.audit_f03.no_blind_replay]
#[test]
fn interrupted_dispatch_never_becomes_pending_work() {
    for phase in [PublicationPhase::Dispatching, PublicationPhase::Dispatched] {
        for inbox in [PublicationObservation::Absent, bound()] {
            for queue in [PublicationObservation::Absent, bound()] {
                let mut observation = input(DedupObservation::Duplicate);
                observation.intent = Some(intent(phase));
                observation.inbox = inbox;
                observation.queue = queue;
                assert_eq!(reconcile(&observation), PublicationDecision::OutcomeUnknown, "{observation:?}");
            }
        }
    }
}

#[test]
fn completed_dispatch_evidence_wins_over_every_intent_phase() {
    for phase in [
        PublicationPhase::Reserved,
        PublicationPhase::Publishing,
        PublicationPhase::Published,
        PublicationPhase::Dispatching,
        PublicationPhase::Dispatched,
    ] {
        let mut observation = input(DedupObservation::Duplicate);
        observation.intent = Some(intent(phase));
        observation.dispatch = bound();
        assert_eq!(reconcile(&observation), PublicationDecision::CompletedDispatch, "{phase:?}");
    }
}
