/// Reconcile supplied facts without allocating or performing effects.
// r[impl molten.audit_f03.recover_publication]
// r[impl molten.audit_f03.uncertainty]
// r[impl molten.audit_f03.preserve_state]
pub fn reconcile(input: &super::ReconcileInput<'_>) -> super::PublicationDecision {
    if !super::validation::has_valid_identity(input) || super::validation::has_conflicting_observation(input) {
        return super::PublicationDecision::Conflict;
    }
    match input.quarantine {
        transactional_reconciliation_core::QuarantineStatus::Clear => {}
        transactional_reconciliation_core::QuarantineStatus::Quarantined(_) => {
            return super::PublicationDecision::OutcomeUnknown;
        }
    }
    if super::validation::has_unavailable_observation(input) {
        return super::PublicationDecision::OutcomeUnknown;
    }
    match input.dedup {
        super::DedupObservation::Unobserved => super::PublicationDecision::CheckDedup,
        super::DedupObservation::Rejected => super::PublicationDecision::Rejected,
        super::DedupObservation::OutcomeUnknown => super::PublicationDecision::OutcomeUnknown,
        super::DedupObservation::First => reconcile_accepted(input, super::PublicationDecision::PublishFresh),
        super::DedupObservation::Duplicate => reconcile_accepted(input, super::PublicationDecision::RecoverPublication),
    }
}

// r[impl molten.audit_f03.no_blind_replay]
fn reconcile_accepted(
    input: &super::ReconcileInput<'_>,
    reserved_decision: super::PublicationDecision,
) -> super::PublicationDecision {
    match input.dispatch {
        super::PublicationObservation::Bound { .. } => return super::PublicationDecision::CompletedDispatch,
        super::PublicationObservation::Conflict => return super::PublicationDecision::Conflict,
        super::PublicationObservation::Unavailable => return super::PublicationDecision::OutcomeUnknown,
        super::PublicationObservation::Absent => {}
    }
    if input.intent.is_some_and(|intent| match intent.phase {
        super::PublicationPhase::Dispatching | super::PublicationPhase::Dispatched => true,
        super::PublicationPhase::Reserved
        | super::PublicationPhase::Publishing
        | super::PublicationPhase::Published => false,
    }) {
        return super::PublicationDecision::OutcomeUnknown;
    }
    match input.inbox {
        super::PublicationObservation::Bound { .. } => match input.queue {
            super::PublicationObservation::Bound { .. } => super::PublicationDecision::ReusePublication,
            super::PublicationObservation::Absent => super::PublicationDecision::RepairQueueReceipt,
            super::PublicationObservation::Conflict => super::PublicationDecision::Conflict,
            super::PublicationObservation::Unavailable => super::PublicationDecision::OutcomeUnknown,
        },
        super::PublicationObservation::Absent => match input.queue {
            super::PublicationObservation::Absent => reconcile_absence(input, reserved_decision),
            super::PublicationObservation::Bound { .. } | super::PublicationObservation::Unavailable => {
                super::PublicationDecision::OutcomeUnknown
            }
            super::PublicationObservation::Conflict => super::PublicationDecision::Conflict,
        },
        super::PublicationObservation::Conflict => super::PublicationDecision::Conflict,
        super::PublicationObservation::Unavailable => super::PublicationDecision::OutcomeUnknown,
    }
}

fn reconcile_absence(
    input: &super::ReconcileInput<'_>,
    reserved_decision: super::PublicationDecision,
) -> super::PublicationDecision {
    match input.intent.map(|intent| intent.phase) {
        Some(super::PublicationPhase::Reserved) => reserved_decision,
        Some(
            super::PublicationPhase::Publishing
            | super::PublicationPhase::Published
            | super::PublicationPhase::Dispatching
            | super::PublicationPhase::Dispatched,
        )
        | None => super::PublicationDecision::OutcomeUnknown,
    }
}
