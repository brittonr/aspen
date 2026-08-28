use super::super::*;

#[allow(
    tigerstyle::function_length,
    reason = "one exhaustive match keeps every required negative case attached to its mutation owner"
)]
pub(super) fn expected_failure_cases(mutation: WorldMutationKind) -> Vec<RequiredFailureCase> {
    match mutation {
        WorldMutationKind::Capture => vec![
            RequiredFailureCase::TornRecord,
            RequiredFailureCase::DuplicateSubmission,
            RequiredFailureCase::MissingObject,
            RequiredFailureCase::CorruptRecord,
            RequiredFailureCase::ContradictoryObservation,
            RequiredFailureCase::FaultCoverageOverclaim,
        ],
        WorldMutationKind::Head => vec![
            RequiredFailureCase::TornRecord,
            RequiredFailureCase::StalePlan,
            RequiredFailureCase::CorruptRecord,
            RequiredFailureCase::GenerationRace,
            RequiredFailureCase::RollbackWithoutWitness,
            RequiredFailureCase::ContradictoryObservation,
            RequiredFailureCase::FaultCoverageOverclaim,
        ],
        WorldMutationKind::Promotion => vec![
            RequiredFailureCase::TornRecord,
            RequiredFailureCase::LostResponse,
            RequiredFailureCase::DuplicateSubmission,
            RequiredFailureCase::StalePlan,
            RequiredFailureCase::CorruptRecord,
            RequiredFailureCase::GenerationRace,
            RequiredFailureCase::EffectUncertainty,
            RequiredFailureCase::ContradictoryObservation,
            RequiredFailureCase::FaultCoverageOverclaim,
        ],
        WorldMutationKind::Witness => vec![
            RequiredFailureCase::CorruptRecord,
            RequiredFailureCase::RollbackWithoutWitness,
            RequiredFailureCase::ContradictoryObservation,
            RequiredFailureCase::FaultCoverageOverclaim,
        ],
        WorldMutationKind::Outbox => vec![
            RequiredFailureCase::LostResponse,
            RequiredFailureCase::DuplicateSubmission,
            RequiredFailureCase::CorruptRecord,
            RequiredFailureCase::EffectUncertainty,
            RequiredFailureCase::ContradictoryObservation,
            RequiredFailureCase::FaultCoverageOverclaim,
        ],
        WorldMutationKind::Replication | WorldMutationKind::Import => vec![
            RequiredFailureCase::TornRecord,
            RequiredFailureCase::DuplicateSubmission,
            RequiredFailureCase::MissingObject,
            RequiredFailureCase::CorruptRecord,
            RequiredFailureCase::ContradictoryObservation,
            RequiredFailureCase::FaultCoverageOverclaim,
        ],
        WorldMutationKind::Retention => vec![
            RequiredFailureCase::StalePlan,
            RequiredFailureCase::CorruptRecord,
            RequiredFailureCase::UnsafeCleanup,
            RequiredFailureCase::ContradictoryObservation,
            RequiredFailureCase::FaultCoverageOverclaim,
        ],
        WorldMutationKind::GarbageCollection => vec![
            RequiredFailureCase::StalePlan,
            RequiredFailureCase::MissingObject,
            RequiredFailureCase::CorruptRecord,
            RequiredFailureCase::UnsafeCleanup,
            RequiredFailureCase::ContradictoryObservation,
            RequiredFailureCase::FaultCoverageOverclaim,
        ],
    }
}
