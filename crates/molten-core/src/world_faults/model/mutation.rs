use super::MAX_WORLD_FAULT_SCHEDULES;
use super::REQUIRED_WORLD_MUTATION_COUNT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldMutationKind {
    Capture,
    Head,
    Promotion,
    Witness,
    Outbox,
    Replication,
    Import,
    Retention,
    GarbageCollection,
}

impl WorldMutationKind {
    pub const ALL: [Self; REQUIRED_WORLD_MUTATION_COUNT] = [
        Self::Capture,
        Self::Head,
        Self::Promotion,
        Self::Witness,
        Self::Outbox,
        Self::Replication,
        Self::Import,
        Self::Retention,
        Self::GarbageCollection,
    ];

    pub const CONCURRENT: [Self; MAX_WORLD_FAULT_SCHEDULES] = [
        Self::Head,
        Self::Promotion,
        Self::Witness,
        Self::Outbox,
        Self::Import,
        Self::Replication,
        Self::Retention,
        Self::GarbageCollection,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::Head => "head",
            Self::Promotion => "promotion",
            Self::Witness => "witness",
            Self::Outbox => "outbox",
            Self::Replication => "replication",
            Self::Import => "import",
            Self::Retention => "retention",
            Self::GarbageCollection => "garbage-collection",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldMutationOwner {
    WorldCommit,
    WorldHead,
    WorldPromotion,
    WorldHeadWitness,
    WorldPromotionOutbox,
    WorldDistribution,
    WorldReplay,
    WorldRetention,
    WorldGarbageCollection,
}

impl WorldMutationOwner {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WorldCommit => "world-commit",
            Self::WorldHead => "world-head",
            Self::WorldPromotion => "world-promotion",
            Self::WorldHeadWitness => "world-head-witness",
            Self::WorldPromotionOutbox => "world-promotion-outbox",
            Self::WorldDistribution => "world-distribution",
            Self::WorldReplay => "world-replay",
            Self::WorldRetention => "world-retention",
            Self::WorldGarbageCollection => "world-garbage-collection",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationIdentityDomain {
    Capture,
    HeadTransition,
    PromotionTransaction,
    WitnessFinalization,
    EffectAttempt,
    ReplicationUpdate,
    CapsuleImport,
    RetentionUpdate,
    GarbageCollectionPlan,
}

impl OperationIdentityDomain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Capture => "molten.world-capture-operation.v1",
            Self::HeadTransition => "molten.world-head-transition-operation.v1",
            Self::PromotionTransaction => "molten.world-promotion-operation.v1",
            Self::WitnessFinalization => "molten.world-witness-operation.v1",
            Self::EffectAttempt => "molten.world-effect-attempt.v1",
            Self::ReplicationUpdate => "molten.world-replication-update.v1",
            Self::CapsuleImport => "molten.world-capsule-import.v1",
            Self::RetentionUpdate => "molten.world-retention-update.v1",
            Self::GarbageCollectionPlan => "molten.world-gc-plan.v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MutationSupport {
    Supported,
    UnsupportedIndependentWitness,
}
