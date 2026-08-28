#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldMutationEffect {
    PublishImmutableCommit,
    CompareAndSwapHead,
    CommitHeadAndReservations,
    FinalizeIndependentWitness,
    RecordAndDispatchEffectAttempt,
    PublishReplicaAvailability,
    PublishCapsuleAvailability,
    PublishRetentionRoots,
    PublishGarbageCollectionPlan,
}

impl WorldMutationEffect {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PublishImmutableCommit => "publish-immutable-commit",
            Self::CompareAndSwapHead => "compare-and-swap-head",
            Self::CommitHeadAndReservations => "commit-head-and-reservations",
            Self::FinalizeIndependentWitness => "finalize-independent-witness",
            Self::RecordAndDispatchEffectAttempt => "record-and-dispatch-effect-attempt",
            Self::PublishReplicaAvailability => "publish-replica-availability",
            Self::PublishCapsuleAvailability => "publish-capsule-availability",
            Self::PublishRetentionRoots => "publish-retention-roots",
            Self::PublishGarbageCollectionPlan => "publish-garbage-collection-plan",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinearizationPoint {
    ImmutableObjectPublication,
    HeadTransactionCommit,
    PromotionTransactionCommit,
    WitnessRecordCommit,
    AttemptRecordCommit,
    ReplicaAvailabilityCommit,
    CapsuleAvailabilityCommit,
    RetentionRootCommit,
    GarbageCollectionPlanCommit,
}

impl LinearizationPoint {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ImmutableObjectPublication => "immutable-object-publication",
            Self::HeadTransactionCommit => "head-transaction-commit",
            Self::PromotionTransactionCommit => "promotion-transaction-commit",
            Self::WitnessRecordCommit => "witness-record-commit",
            Self::AttemptRecordCommit => "attempt-record-commit",
            Self::ReplicaAvailabilityCommit => "replica-availability-commit",
            Self::CapsuleAvailabilityCommit => "capsule-availability-commit",
            Self::RetentionRootCommit => "retention-root-commit",
            Self::GarbageCollectionPlanCommit => "garbage-collection-plan-commit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DurableRecordKind {
    CaptureReceipt,
    HeadTransitionReceipt,
    PromotionAndReservationSet,
    IndependentWitnessRecord,
    EffectAttemptRecord,
    ReachabilityReceipt,
    ReplayImportReceipt,
    RetentionRootInventory,
    GarbageCollectionPlan,
}

impl DurableRecordKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CaptureReceipt => "capture-receipt",
            Self::HeadTransitionReceipt => "head-transition-receipt",
            Self::PromotionAndReservationSet => "promotion-and-reservation-set",
            Self::IndependentWitnessRecord => "independent-witness-record",
            Self::EffectAttemptRecord => "effect-attempt-record",
            Self::ReachabilityReceipt => "reachability-receipt",
            Self::ReplayImportReceipt => "replay-import-receipt",
            Self::RetentionRootInventory => "retention-root-inventory",
            Self::GarbageCollectionPlan => "garbage-collection-plan",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UncertainWindow {
    PublicationBeforeReceipt,
    CommitBeforeResponse,
    ExternalSubmitBeforeObservation,
    StagingBeforeAvailability,
    ReachabilityBeforeReceipt,
    PlanBeforeReceipt,
}

impl UncertainWindow {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PublicationBeforeReceipt => "publication-before-receipt",
            Self::CommitBeforeResponse => "commit-before-response",
            Self::ExternalSubmitBeforeObservation => "external-submit-before-observation",
            Self::StagingBeforeAvailability => "staging-before-availability",
            Self::ReachabilityBeforeReceipt => "reachability-before-receipt",
            Self::PlanBeforeReceipt => "plan-before-receipt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReconciliationEntry {
    CaptureReadBack,
    HeadReadBack,
    PromotionReadBack,
    WitnessReadBack,
    AttemptRecovery,
    ReplicationReadBack,
    ImportReadBack,
    RetentionReadBack,
    GarbageCollectionReplan,
}

impl ReconciliationEntry {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CaptureReadBack => "capture-read-back",
            Self::HeadReadBack => "head-read-back",
            Self::PromotionReadBack => "promotion-read-back",
            Self::WitnessReadBack => "witness-read-back",
            Self::AttemptRecovery => "attempt-recovery",
            Self::ReplicationReadBack => "replication-read-back",
            Self::ImportReadBack => "import-read-back",
            Self::RetentionReadBack => "retention-read-back",
            Self::GarbageCollectionReplan => "garbage-collection-replan",
        }
    }
}
