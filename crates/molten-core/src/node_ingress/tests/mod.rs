mod admission;
mod identity;
mod publication;

use super::*;

const OPERATION_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const REQUEST_REF: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
const OTHER_REF: &str = "blake3:3333333333333333333333333333333333333333333333333333333333333333";

fn input(dedup: DedupObservation) -> ReconcileInput<'static> {
    ReconcileInput {
        operation_ref: OPERATION_REF,
        request_ref: REQUEST_REF,
        operation: "status",
        dedup,
        intent: Some(intent(PublicationPhase::Reserved)),
        inbox: PublicationObservation::Absent,
        queue: PublicationObservation::Absent,
        dispatch: PublicationObservation::Absent,
        quarantine: transactional_reconciliation_core::QuarantineStatus::Clear,
    }
}

fn intent(phase: PublicationPhase) -> PublicationIntentObservation<'static> {
    PublicationIntentObservation {
        operation_ref: OPERATION_REF,
        request_ref: REQUEST_REF,
        phase,
    }
}

fn bound() -> PublicationObservation<'static> {
    PublicationObservation::Bound {
        operation: "status",
        request_ref: REQUEST_REF,
    }
}
