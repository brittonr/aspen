use super::MAX_FABRIC_COLLECTION_ITEMS;
use super::has_duplicates;

const REQUIRED_MECHANISM_COUNT: usize = 11;
const REQUIRED_NON_CLAIM_COUNT: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FabricIdentity {
    WorkloadNeutralDistributedSystemsFabric,
    DatabaseFramework,
    GlobalActorRuntime,
}

impl FabricIdentity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WorkloadNeutralDistributedSystemsFabric => "workload-neutral-distributed-systems-fabric",
            Self::DatabaseFramework => "database-framework",
            Self::GlobalActorRuntime => "global-actor-runtime",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FabricMechanism {
    CanonicalCommunication,
    Lifecycle,
    Authority,
    Resources,
    Execution,
    Durability,
    Transport,
    Scheduling,
    Supervision,
    Policy,
    Evidence,
}

impl FabricMechanism {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalCommunication => "canonical-communication",
            Self::Lifecycle => "lifecycle",
            Self::Authority => "authority",
            Self::Resources => "resources",
            Self::Execution => "execution",
            Self::Durability => "durability",
            Self::Transport => "transport",
            Self::Scheduling => "scheduling",
            Self::Supervision => "supervision",
            Self::Policy => "policy",
            Self::Evidence => "evidence",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkloadSemantic {
    TransactionIsolation,
    ConflictResolution,
    LogOffsets,
    QueueDelivery,
    ShardPolicy,
    WorkflowState,
    ApplicationProtocol,
    SchedulingPolicy,
}

impl WorkloadSemantic {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TransactionIsolation => "transaction-isolation",
            Self::ConflictResolution => "conflict-resolution",
            Self::LogOffsets => "log-offsets",
            Self::QueueDelivery => "queue-delivery",
            Self::ShardPolicy => "shard-policy",
            Self::WorkflowState => "workflow-state",
            Self::ApplicationProtocol => "application-protocol",
            Self::SchedulingPolicy => "scheduling-policy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FabricNonClaim {
    DatabaseCorrectness,
    GlobalOrdering,
    GlobalConsensus,
    TransportDelivery,
    DurablePersistence,
    ByzantineTolerance,
    ProtocolCompatibility,
    ProductionReadiness,
    ExtensionSemanticCorrectness,
}

impl FabricNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DatabaseCorrectness => "does-not-prove-database-correctness",
            Self::GlobalOrdering => "does-not-prove-global-ordering",
            Self::GlobalConsensus => "does-not-prove-global-consensus",
            Self::TransportDelivery => "does-not-prove-transport-delivery",
            Self::DurablePersistence => "does-not-prove-durable-persistence",
            Self::ByzantineTolerance => "does-not-prove-byzantine-tolerance",
            Self::ProtocolCompatibility => "does-not-prove-protocol-compatibility",
            Self::ProductionReadiness => "does-not-prove-production-readiness",
            Self::ExtensionSemanticCorrectness => "does-not-prove-extension-semantic-correctness",
        }
    }
}

pub const REQUIRED_FABRIC_MECHANISMS: [FabricMechanism; REQUIRED_MECHANISM_COUNT] = [
    FabricMechanism::CanonicalCommunication,
    FabricMechanism::Lifecycle,
    FabricMechanism::Authority,
    FabricMechanism::Resources,
    FabricMechanism::Execution,
    FabricMechanism::Durability,
    FabricMechanism::Transport,
    FabricMechanism::Scheduling,
    FabricMechanism::Supervision,
    FabricMechanism::Policy,
    FabricMechanism::Evidence,
];

pub const REQUIRED_FABRIC_NON_CLAIMS: [FabricNonClaim; REQUIRED_NON_CLAIM_COUNT] = [
    FabricNonClaim::DatabaseCorrectness,
    FabricNonClaim::GlobalOrdering,
    FabricNonClaim::GlobalConsensus,
    FabricNonClaim::TransportDelivery,
    FabricNonClaim::DurablePersistence,
    FabricNonClaim::ByzantineTolerance,
    FabricNonClaim::ProtocolCompatibility,
    FabricNonClaim::ProductionReadiness,
    FabricNonClaim::ExtensionSemanticCorrectness,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricBoundaryDescriptor {
    pub identity: FabricIdentity,
    pub mechanisms: Vec<FabricMechanism>,
    pub core_owned_semantics: Vec<WorkloadSemantic>,
    pub non_claims: Vec<FabricNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricBoundaryReport {
    pub identity: FabricIdentity,
    pub mechanisms: Vec<FabricMechanism>,
    pub non_claims: Vec<FabricNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FabricBoundaryIssue {
    WrongIdentity(FabricIdentity),
    TooManyMechanisms { actual: usize, maximum: usize },
    DuplicateMechanism,
    MissingMechanism(FabricMechanism),
    TooManyCoreSemantics { actual: usize, maximum: usize },
    ExtensionSemanticOwnedByCore(WorkloadSemantic),
    TooManyNonClaims { actual: usize, maximum: usize },
    DuplicateNonClaim,
    MissingNonClaim(FabricNonClaim),
}

pub fn default_fabric_boundary_descriptor() -> FabricBoundaryDescriptor {
    FabricBoundaryDescriptor {
        identity: FabricIdentity::WorkloadNeutralDistributedSystemsFabric,
        mechanisms: REQUIRED_FABRIC_MECHANISMS.to_vec(),
        core_owned_semantics: Vec::new(),
        non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
    }
}

// r[impl molten.fabric_boundary.fabric_identity]
// r[impl molten.fabric_boundary.mechanism_semantics_separation]
// r[impl molten.fabric_boundary.non_claims]
pub fn validate_fabric_boundary(
    descriptor: &FabricBoundaryDescriptor,
) -> Result<FabricBoundaryReport, Vec<FabricBoundaryIssue>> {
    let mut issues = Vec::new();
    validate_identity(descriptor, &mut issues);
    validate_mechanisms(descriptor, &mut issues);
    validate_semantic_ownership(descriptor, &mut issues);
    validate_non_claims(descriptor, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut mechanisms = descriptor.mechanisms.clone();
    mechanisms.sort();
    let mut non_claims = descriptor.non_claims.clone();
    non_claims.sort();
    Ok(FabricBoundaryReport {
        identity: descriptor.identity,
        mechanisms,
        non_claims,
    })
}

fn validate_identity(descriptor: &FabricBoundaryDescriptor, issues: &mut Vec<FabricBoundaryIssue>) {
    if descriptor.identity != FabricIdentity::WorkloadNeutralDistributedSystemsFabric {
        issues.push(FabricBoundaryIssue::WrongIdentity(descriptor.identity));
    }
}

fn validate_mechanisms(descriptor: &FabricBoundaryDescriptor, issues: &mut Vec<FabricBoundaryIssue>) {
    if descriptor.mechanisms.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(FabricBoundaryIssue::TooManyMechanisms {
            actual: descriptor.mechanisms.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(&descriptor.mechanisms) {
        issues.push(FabricBoundaryIssue::DuplicateMechanism);
    }
    for required in REQUIRED_FABRIC_MECHANISMS {
        if !descriptor.mechanisms.contains(&required) {
            issues.push(FabricBoundaryIssue::MissingMechanism(required));
        }
    }
}

fn validate_semantic_ownership(descriptor: &FabricBoundaryDescriptor, issues: &mut Vec<FabricBoundaryIssue>) {
    if descriptor.core_owned_semantics.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(FabricBoundaryIssue::TooManyCoreSemantics {
            actual: descriptor.core_owned_semantics.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    for semantic in &descriptor.core_owned_semantics {
        issues.push(FabricBoundaryIssue::ExtensionSemanticOwnedByCore(*semantic));
    }
}

pub(crate) fn validate_required_non_claims(
    non_claims: &[FabricNonClaim],
    mut missing: impl FnMut(FabricNonClaim),
) -> bool {
    let mut complete = true;
    for required in REQUIRED_FABRIC_NON_CLAIMS {
        if !non_claims.contains(&required) {
            complete = false;
            missing(required);
        }
    }
    complete
}

fn validate_non_claims(descriptor: &FabricBoundaryDescriptor, issues: &mut Vec<FabricBoundaryIssue>) {
    if descriptor.non_claims.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(FabricBoundaryIssue::TooManyNonClaims {
            actual: descriptor.non_claims.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(&descriptor.non_claims) {
        issues.push(FabricBoundaryIssue::DuplicateNonClaim);
    }
    validate_required_non_claims(&descriptor.non_claims, |missing| {
        issues.push(FabricBoundaryIssue::MissingNonClaim(missing));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify molten.fabric_boundary.fabric_identity]
    // r[verify molten.fabric_boundary.mechanism_semantics_separation]
    // r[verify molten.fabric_boundary.non_claims]
    #[test]
    fn workload_neutral_boundary_accepts_mechanisms_without_workload_semantics() {
        let report = validate_fabric_boundary(&default_fabric_boundary_descriptor()).expect("valid fabric boundary");

        assert_eq!(report.identity, FabricIdentity::WorkloadNeutralDistributedSystemsFabric);
        assert_eq!(report.mechanisms, REQUIRED_FABRIC_MECHANISMS);
        assert_eq!(report.non_claims, REQUIRED_FABRIC_NON_CLAIMS);
    }

    // r[verify molten.fabric_boundary.mechanism_semantics_separation]
    // r[verify molten.fabric_boundary.non_claims]
    #[test]
    fn boundary_rejects_database_identity_semantic_leakage_and_missing_non_claim() {
        let mut descriptor = default_fabric_boundary_descriptor();
        descriptor.identity = FabricIdentity::DatabaseFramework;
        descriptor.core_owned_semantics = vec![WorkloadSemantic::TransactionIsolation];
        descriptor.non_claims.retain(|claim| *claim != FabricNonClaim::DatabaseCorrectness);

        let issues = validate_fabric_boundary(&descriptor).expect_err("semantic leakage must deny");

        assert!(issues.contains(&FabricBoundaryIssue::WrongIdentity(FabricIdentity::DatabaseFramework)));
        assert!(
            issues.contains(&FabricBoundaryIssue::ExtensionSemanticOwnedByCore(WorkloadSemantic::TransactionIsolation))
        );
        assert!(issues.contains(&FabricBoundaryIssue::MissingNonClaim(FabricNonClaim::DatabaseCorrectness)));
    }
}
