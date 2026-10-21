/// Durable publication progress, distinct from dedup acceptance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationPhase {
    Reserved,
    Publishing,
    Published,
    Dispatching,
    Dispatched,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DedupObservation {
    Unobserved,
    First,
    Duplicate,
    Rejected,
    OutcomeUnknown,
}

/// An authoritative observation, not an inference from a missing receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationObservation<'a> {
    Absent,
    Bound { operation: &'a str, request_ref: &'a str },
    Conflict,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicationIntentObservation<'a> {
    pub operation_ref: &'a str,
    pub request_ref: &'a str,
    pub phase: PublicationPhase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReconcileInput<'a> {
    pub operation_ref: &'a str,
    pub request_ref: &'a str,
    pub operation: &'a str,
    pub dedup: DedupObservation,
    pub intent: Option<PublicationIntentObservation<'a>>,
    pub inbox: PublicationObservation<'a>,
    pub queue: PublicationObservation<'a>,
    pub dispatch: PublicationObservation<'a>,
    pub quarantine: transactional_reconciliation_core::QuarantineStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicationDecision {
    CheckDedup,
    PublishFresh,
    RecoverPublication,
    RepairQueueReceipt,
    ReusePublication,
    CompletedDispatch,
    Conflict,
    OutcomeUnknown,
    Rejected,
}
