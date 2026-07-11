use super::FabricNonClaim;
use super::FabricPortClass;
use super::MAX_FABRIC_COLLECTION_ITEMS;
use super::has_duplicates;
use super::validate_required_non_claims;

pub const FABRIC_REFERENCE_MATRIX_SCHEMA: &str = "molten.fabric.reference-matrix.v1";

const REFERENCE_SYSTEM_COUNT: usize = 3;
const BASE_CAPABILITY_COUNT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceSystemKind {
    TransactionalKeyValue,
    ReplicatedLog,
    DistributedScheduler,
}

impl ReferenceSystemKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TransactionalKeyValue => "transactional-key-value",
            Self::ReplicatedLog => "replicated-log",
            Self::DistributedScheduler => "distributed-scheduler",
        }
    }
}

pub const REQUIRED_REFERENCE_SYSTEMS: [ReferenceSystemKind; REFERENCE_SYSTEM_COUNT] = [
    ReferenceSystemKind::TransactionalKeyValue,
    ReferenceSystemKind::ReplicatedLog,
    ReferenceSystemKind::DistributedScheduler,
];

pub const BASE_REFERENCE_CAPABILITIES: [FabricPortClass; BASE_CAPABILITY_COUNT] = [
    FabricPortClass::Authority,
    FabricPortClass::Resources,
    FabricPortClass::DurableState,
    FabricPortClass::Transport,
    FabricPortClass::Scheduling,
    FabricPortClass::Simulation,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceSemantic {
    TransactionIsolation,
    ConflictResolution,
    ConsumerOffsets,
    LogRetention,
    SchedulingPolicy,
    TaskOwnership,
}

impl ReferenceSemantic {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TransactionIsolation => "transaction-isolation",
            Self::ConflictResolution => "conflict-resolution",
            Self::ConsumerOffsets => "consumer-offsets",
            Self::LogRetention => "log-retention",
            Self::SchedulingPolicy => "scheduling-policy",
            Self::TaskOwnership => "task-ownership",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SemanticOwner {
    FabricCore,
    SystemExtension,
}

impl SemanticOwner {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FabricCore => "fabric-core",
            Self::SystemExtension => "system-extension",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticOwnership {
    pub semantic: ReferenceSemantic,
    pub owner: SemanticOwner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceSystemMatrix {
    pub schema: String,
    pub system: ReferenceSystemKind,
    pub capabilities: Vec<FabricPortClass>,
    pub semantics: Vec<SemanticOwnership>,
    pub ambient_accesses: Vec<String>,
    pub non_claims: Vec<FabricNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceMatrixSummary {
    pub matrices: Vec<ReferenceSystemMatrix>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceMatrixIssue {
    TooManyMatrices {
        actual: usize,
        maximum: usize,
    },
    DuplicateSystem(ReferenceSystemKind),
    MissingSystem(ReferenceSystemKind),
    SchemaMismatch {
        system: ReferenceSystemKind,
        actual: String,
        expected: String,
    },
    TooManyCapabilities {
        system: ReferenceSystemKind,
        actual: usize,
        maximum: usize,
    },
    DuplicateCapability(ReferenceSystemKind),
    MissingCapability {
        system: ReferenceSystemKind,
        capability: FabricPortClass,
    },
    TooManySemantics {
        system: ReferenceSystemKind,
        actual: usize,
        maximum: usize,
    },
    DuplicateSemantic {
        system: ReferenceSystemKind,
        semantic: ReferenceSemantic,
    },
    MissingExtensionSemantic {
        system: ReferenceSystemKind,
        semantic: ReferenceSemantic,
    },
    SemanticLeakedIntoCore {
        system: ReferenceSystemKind,
        semantic: ReferenceSemantic,
    },
    TooManyAmbientAccesses {
        system: ReferenceSystemKind,
        actual: usize,
        maximum: usize,
    },
    AmbientAccessBypass {
        system: ReferenceSystemKind,
        access: String,
    },
    DuplicateNonClaim(ReferenceSystemKind),
    MissingNonClaim {
        system: ReferenceSystemKind,
        non_claim: FabricNonClaim,
    },
}

pub fn default_reference_system_matrices() -> Vec<ReferenceSystemMatrix> {
    vec![
        reference_matrix(ReferenceSystemKind::TransactionalKeyValue, &[
            ReferenceSemantic::TransactionIsolation,
            ReferenceSemantic::ConflictResolution,
        ]),
        reference_matrix(ReferenceSystemKind::ReplicatedLog, &[
            ReferenceSemantic::ConsumerOffsets,
            ReferenceSemantic::LogRetention,
        ]),
        reference_matrix(ReferenceSystemKind::DistributedScheduler, &[
            ReferenceSemantic::SchedulingPolicy,
            ReferenceSemantic::TaskOwnership,
        ]),
    ]
}

// r[impl molten.fabric_boundary.reference_system_exit_criteria]
// r[impl molten.fabric_boundary.mechanism_semantics_separation]
// r[impl molten.fabric_boundary.non_claims]
pub fn validate_reference_system_matrices(
    matrices: &[ReferenceSystemMatrix],
) -> Result<ReferenceMatrixSummary, Vec<ReferenceMatrixIssue>> {
    let mut issues = Vec::new();
    validate_matrix_set(matrices, &mut issues);
    for matrix in matrices {
        validate_matrix(matrix, &mut issues);
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut normalized = matrices.iter().cloned().map(normalize_matrix).collect::<Vec<_>>();
    normalized.sort_by_key(|matrix| matrix.system);
    Ok(ReferenceMatrixSummary { matrices: normalized })
}

fn reference_matrix(system: ReferenceSystemKind, semantics: &[ReferenceSemantic]) -> ReferenceSystemMatrix {
    ReferenceSystemMatrix {
        schema: FABRIC_REFERENCE_MATRIX_SCHEMA.to_string(),
        system,
        capabilities: vec![
            FabricPortClass::Authority,
            FabricPortClass::Transport,
            FabricPortClass::DurableState,
            FabricPortClass::Time,
            FabricPortClass::Scheduling,
            FabricPortClass::Membership,
            FabricPortClass::Placement,
            FabricPortClass::Consistency,
            FabricPortClass::Supervision,
            FabricPortClass::Policy,
            FabricPortClass::Resources,
            FabricPortClass::Simulation,
            FabricPortClass::Evidence,
        ],
        semantics: semantics
            .iter()
            .map(|semantic| SemanticOwnership {
                semantic: *semantic,
                owner: SemanticOwner::SystemExtension,
            })
            .collect(),
        ambient_accesses: Vec::new(),
        non_claims: super::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
    }
}

fn validate_matrix_set(matrices: &[ReferenceSystemMatrix], issues: &mut Vec<ReferenceMatrixIssue>) {
    if matrices.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(ReferenceMatrixIssue::TooManyMatrices {
            actual: matrices.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    let systems = matrices.iter().map(|matrix| matrix.system).collect::<Vec<_>>();
    for system in REQUIRED_REFERENCE_SYSTEMS {
        let count = systems.iter().filter(|candidate| **candidate == system).count();
        if count == 0 {
            issues.push(ReferenceMatrixIssue::MissingSystem(system));
        }
        if count > 1 {
            issues.push(ReferenceMatrixIssue::DuplicateSystem(system));
        }
    }
}

fn validate_matrix(matrix: &ReferenceSystemMatrix, issues: &mut Vec<ReferenceMatrixIssue>) {
    if matrix.schema != FABRIC_REFERENCE_MATRIX_SCHEMA {
        issues.push(ReferenceMatrixIssue::SchemaMismatch {
            system: matrix.system,
            actual: matrix.schema.clone(),
            expected: FABRIC_REFERENCE_MATRIX_SCHEMA.to_string(),
        });
    }
    validate_capabilities(matrix, issues);
    validate_semantics(matrix, issues);
    validate_ambient_access(matrix, issues);
    validate_matrix_non_claims(matrix, issues);
}

fn validate_capabilities(matrix: &ReferenceSystemMatrix, issues: &mut Vec<ReferenceMatrixIssue>) {
    if matrix.capabilities.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(ReferenceMatrixIssue::TooManyCapabilities {
            system: matrix.system,
            actual: matrix.capabilities.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(&matrix.capabilities) {
        issues.push(ReferenceMatrixIssue::DuplicateCapability(matrix.system));
    }
    for capability in BASE_REFERENCE_CAPABILITIES {
        if !matrix.capabilities.contains(&capability) {
            issues.push(ReferenceMatrixIssue::MissingCapability {
                system: matrix.system,
                capability,
            });
        }
    }
}

fn validate_semantics(matrix: &ReferenceSystemMatrix, issues: &mut Vec<ReferenceMatrixIssue>) {
    if matrix.semantics.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(ReferenceMatrixIssue::TooManySemantics {
            system: matrix.system,
            actual: matrix.semantics.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    let semantics = matrix.semantics.iter().map(|ownership| ownership.semantic).collect::<Vec<_>>();
    for semantic in &semantics {
        if semantics.iter().filter(|candidate| *candidate == semantic).count() > 1 {
            let issue = ReferenceMatrixIssue::DuplicateSemantic {
                system: matrix.system,
                semantic: *semantic,
            };
            if !issues.contains(&issue) {
                issues.push(issue);
            }
        }
    }
    let expected = expected_semantic(matrix.system);
    if !matrix
        .semantics
        .iter()
        .any(|ownership| ownership.semantic == expected && ownership.owner == SemanticOwner::SystemExtension)
    {
        issues.push(ReferenceMatrixIssue::MissingExtensionSemantic {
            system: matrix.system,
            semantic: expected,
        });
    }
    for ownership in &matrix.semantics {
        if ownership.owner == SemanticOwner::FabricCore {
            issues.push(ReferenceMatrixIssue::SemanticLeakedIntoCore {
                system: matrix.system,
                semantic: ownership.semantic,
            });
        }
    }
}

fn validate_ambient_access(matrix: &ReferenceSystemMatrix, issues: &mut Vec<ReferenceMatrixIssue>) {
    if matrix.ambient_accesses.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(ReferenceMatrixIssue::TooManyAmbientAccesses {
            system: matrix.system,
            actual: matrix.ambient_accesses.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    for access in &matrix.ambient_accesses {
        issues.push(ReferenceMatrixIssue::AmbientAccessBypass {
            system: matrix.system,
            access: access.clone(),
        });
    }
}

fn validate_matrix_non_claims(matrix: &ReferenceSystemMatrix, issues: &mut Vec<ReferenceMatrixIssue>) {
    if has_duplicates(&matrix.non_claims) {
        issues.push(ReferenceMatrixIssue::DuplicateNonClaim(matrix.system));
    }
    validate_required_non_claims(&matrix.non_claims, |missing| {
        issues.push(ReferenceMatrixIssue::MissingNonClaim {
            system: matrix.system,
            non_claim: missing,
        });
    });
}

const fn expected_semantic(system: ReferenceSystemKind) -> ReferenceSemantic {
    match system {
        ReferenceSystemKind::TransactionalKeyValue => ReferenceSemantic::TransactionIsolation,
        ReferenceSystemKind::ReplicatedLog => ReferenceSemantic::ConsumerOffsets,
        ReferenceSystemKind::DistributedScheduler => ReferenceSemantic::SchedulingPolicy,
    }
}

fn normalize_matrix(mut matrix: ReferenceSystemMatrix) -> ReferenceSystemMatrix {
    matrix.capabilities.sort();
    matrix.semantics.sort();
    matrix.ambient_accesses.sort();
    matrix.non_claims.sort();
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify molten.fabric_boundary.reference_system_exit_criteria]
    #[test]
    fn three_reference_systems_use_ports_and_keep_semantics_extension_owned() {
        let summary = validate_reference_system_matrices(&default_reference_system_matrices())
            .expect("reference systems satisfy the fabric boundary");

        assert_eq!(summary.matrices.len(), REFERENCE_SYSTEM_COUNT);
        for system in REQUIRED_REFERENCE_SYSTEMS {
            let matrix =
                summary.matrices.iter().find(|matrix| matrix.system == system).expect("required reference matrix");
            assert!(matrix.ambient_accesses.is_empty());
            for capability in BASE_REFERENCE_CAPABILITIES {
                assert!(matrix.capabilities.contains(&capability));
            }
            assert!(matrix.semantics.iter().all(|semantic| semantic.owner == SemanticOwner::SystemExtension));
        }
    }

    // r[verify molten.fabric_boundary.reference_system_exit_criteria]
    // r[verify molten.fabric_boundary.mechanism_semantics_separation]
    #[test]
    fn reference_matrix_reports_missing_port_ambient_bypass_and_semantic_leakage() {
        let mut matrices = default_reference_system_matrices();
        let matrix = matrices.first_mut().expect("transactional matrix");
        matrix.capabilities.retain(|capability| *capability != FabricPortClass::Simulation);
        matrix.ambient_accesses.push("std.fs.direct".to_string());
        let semantic = matrix.semantics.first_mut().expect("transaction semantic");
        semantic.owner = SemanticOwner::FabricCore;

        let issues = validate_reference_system_matrices(&matrices).expect_err("boundary bypass must deny");

        assert!(issues.contains(&ReferenceMatrixIssue::MissingCapability {
            system: ReferenceSystemKind::TransactionalKeyValue,
            capability: FabricPortClass::Simulation,
        }));
        assert!(issues.contains(&ReferenceMatrixIssue::AmbientAccessBypass {
            system: ReferenceSystemKind::TransactionalKeyValue,
            access: "std.fs.direct".to_string(),
        }));
        assert!(issues.contains(&ReferenceMatrixIssue::SemanticLeakedIntoCore {
            system: ReferenceSystemKind::TransactionalKeyValue,
            semantic: ReferenceSemantic::TransactionIsolation,
        }));
    }

    // r[verify molten.fabric_boundary.reference_system_exit_criteria]
    #[test]
    fn reference_suite_rejects_missing_service_class() {
        let mut matrices = default_reference_system_matrices();
        matrices.retain(|matrix| matrix.system != ReferenceSystemKind::ReplicatedLog);

        let issues = validate_reference_system_matrices(&matrices).expect_err("missing reference class must deny");

        assert!(issues.contains(&ReferenceMatrixIssue::MissingSystem(ReferenceSystemKind::ReplicatedLog)));
    }
}
