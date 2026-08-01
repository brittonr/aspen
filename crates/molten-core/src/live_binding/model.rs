use artifact_binding_core::ArtifactAttribution;
use artifact_binding_core::ArtifactId;
use artifact_binding_core::BindingSnapshot;
use artifact_binding_core::GenerationId;
use artifact_binding_core::GraphEdge;
use artifact_binding_core::GraphRoot;
use artifact_binding_core::Resolution;
use artifact_binding_core::ResolutionRequest;
use artifact_binding_core::RetirementDecision;
use artifact_binding_core::RootClassCompleteness;
use artifact_binding_core::RootClassId;
use artifact_binding_core::SnapshotId;
use artifact_binding_core::TransitionPlan;
use artifact_binding_core::TransitionRequest;
use kamacite_core::CompatibilityContext;
use kamacite_core::Identity;
use kamacite_core::SemanticOperationCompatibility;

pub const ARTIFACT_BINDING_REVISION: &str = "c932138d880ddf4c2967f4c024b489b5c0022bf1";
pub const KAMACITE_SEMANTIC_REVISION: &str = "d76fe4abe543724d8fc0ac4b362187caf2e27622";
pub const ARTIFACT_BINDING_SOURCE: &str = "ssh://git@github.com/OnixResearch/onix-artifact.git";
pub const KAMACITE_SEMANTIC_SOURCE: &str = "ssh://git@github.com/OnixResearch/kamacite.git";
pub const IDENTIFIER_MAXIMUM_BYTES: usize = 256;
pub const SNAPSHOT_BINDING_LIMIT: usize = 1_024;
pub const ROOT_LIMIT: usize = 4_096;
pub const EDGE_LIMIT: usize = 16_384;
pub const REACHABLE_NODE_LIMIT: usize = 16_384;
pub const PIN_PATH_NODE_LIMIT: usize = 1_024;
pub const DIAGNOSTIC_LIMIT: usize = 1_024;
pub const ATTRIBUTION_LIMIT: usize = 16_384;
pub const ROOT_CLASS_LIMIT: usize = 32;
pub const RETIREMENT_ISSUE_LIMIT: usize = 1_024;
pub const SEMANTIC_SURFACE_COUNT: usize = 13;

pub const ROOT_CLASS_ACTIVE_EXECUTION: &str = "active-execution";
pub const ROOT_CLASS_SESSION: &str = "session";
pub const ROOT_CLASS_TASK: &str = "task";
pub const ROOT_CLASS_DURABLE_VALUE: &str = "durable-value";
pub const ROOT_CLASS_QUEUE: &str = "queue";
pub const ROOT_CLASS_TIMER: &str = "timer";
pub const ROOT_CLASS_REGISTRY: &str = "registry";
pub const ROOT_CLASS_EFFECT_HANDLE: &str = "effect-handle";
pub const ROOT_CLASS_SNAPSHOT: &str = "snapshot";
pub const ROOT_CLASS_ROLLBACK_RETENTION: &str = "rollback-retention";
pub const ROOT_CLASS_REMOTE_CACHE: &str = "remote-cache";
pub const SYSTEM_EXTENSION_LATE_BINDING_PILOT_PROFILE: &str = "sandboxed-component";

pub const REQUIRED_ROOT_CLASSES: [&str; 11] = [
    ROOT_CLASS_ACTIVE_EXECUTION,
    ROOT_CLASS_SESSION,
    ROOT_CLASS_TASK,
    ROOT_CLASS_DURABLE_VALUE,
    ROOT_CLASS_QUEUE,
    ROOT_CLASS_TIMER,
    ROOT_CLASS_REGISTRY,
    ROOT_CLASS_EFFECT_HANDLE,
    ROOT_CLASS_SNAPSHOT,
    ROOT_CLASS_ROLLBACK_RETENTION,
    ROOT_CLASS_REMOTE_CACHE,
];

pub const BINDING_NON_CLAIMS: [&str; 5] = [
    "resolution is not authority",
    "transition planning is not publication",
    "retirement is not retention policy",
    "retirement is not garbage-collection authority",
    "retirement is not deletion authority",
];

pub const SEMANTIC_NON_CLAIMS: [&str; 5] = [
    "identity equality is not behavior correctness",
    "compatibility evidence is directional and context-bounded",
    "semantic identity is not handler authority",
    "semantic identity is not host authorization",
    "semantic identity is not release eligibility",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePinObservation {
    pub artifact_binding_source: String,
    pub artifact_binding_revision: String,
    pub kamacite_source: String,
    pub kamacite_revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePinReport {
    pub artifact_binding_exact: bool,
    pub kamacite_exact: bool,
    pub release_eligible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductGateFacts {
    pub target_loaded: bool,
    pub target_verified: bool,
    pub product_compatible: bool,
    pub migration_required: bool,
    pub migration_satisfied: bool,
    pub authority_admitted: bool,
    pub policy_admitted: bool,
    pub provenance_admitted: bool,
    pub resource_admitted: bool,
    pub lifecycle_admitted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoltenCutoverRequest {
    pub transition: TransitionRequest,
    pub gates: ProductGateFacts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoltenCutoverPlan {
    pub shared_plan: TransitionPlan,
    pub publication_authorized: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitBoundary {
    Request,
    Turn,
    CallbackPass,
    Job,
    ProtocolSession,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactDependencyClosure {
    pub artifact: ArtifactId,
    pub dependencies: Vec<ArtifactId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitResolutionInput {
    pub boundary: UnitBoundary,
    pub request: ResolutionRequest,
    pub snapshot: Option<BindingSnapshot>,
    pub closures: Vec<ArtifactDependencyClosure>,
    pub nested_lookup: bool,
    pub nested_late_binding_declared: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitResolution {
    pub boundary: UnitBoundary,
    pub shared_resolution: Resolution,
    pub pinned_dependencies: Vec<ArtifactId>,
    pub nested_lookup_authorized: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemExtensionCallbackInput {
    pub profile: String,
    pub resolution: UnitResolutionInput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootInventoryInput {
    pub profile: String,
    pub snapshot: SnapshotId,
    pub generation: GenerationId,
    pub instrumented: bool,
    pub roots: Vec<GraphRoot>,
    pub edges: Vec<GraphEdge>,
    pub class_completeness: Vec<RootClassCompleteness>,
    pub edge_inventory_complete: bool,
    pub attribution_inventory_complete: bool,
    pub attributions: Vec<ArtifactAttribution>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoltenRetirementReport {
    pub profile: String,
    pub snapshot: SnapshotId,
    pub decision: RetirementDecision,
    pub observation_only: bool,
    pub retention_authorized: bool,
    pub deletion_authorized: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeployDiagnostic {
    StaleCompareAndSwap,
    IncompatibleTarget,
    UnreachableSuccessor,
    SemanticHandlerMismatch,
    IncompleteRootInventory {
        root_class: String,
    },
    AmbiguousAttribution,
    LivePinPath {
        root_class: String,
        root_id: String,
        target: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticSurfaceBindings {
    pub manifest: Identity,
    pub handler_binding: Identity,
    pub handle: Identity,
    pub request: Identity,
    pub response: Identity,
    pub effect_log: Identity,
    pub adapter_import: Identity,
    pub remote_execution: Identity,
    pub runtime_receipt: Identity,
    pub replay_identity: Identity,
    pub evaluation_cache_key: Identity,
    pub job: Identity,
    pub upgrade_check: Identity,
}

impl SemanticSurfaceBindings {
    pub fn identities(&self) -> [&Identity; SEMANTIC_SURFACE_COUNT] {
        [
            &self.manifest,
            &self.handler_binding,
            &self.handle,
            &self.request,
            &self.response,
            &self.effect_log,
            &self.adapter_import,
            &self.remote_execution,
            &self.runtime_receipt,
            &self.replay_identity,
            &self.evaluation_cache_key,
            &self.job,
            &self.upgrade_check,
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticCompatibilityAdmissionInput {
    pub compatibility: SemanticOperationCompatibility,
    pub source_operation: Identity,
    pub target_operation: Identity,
    pub context: CompatibilityContext,
    pub molten_policy_admitted: bool,
    pub capability_admitted: bool,
    pub provenance_admitted: bool,
    pub live_execution: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticCompatibilityAdmission {
    pub source_operation: Identity,
    pub target_operation: Identity,
    pub context: CompatibilityContext,
    pub compatibility_admitted: bool,
    pub runtime_authorized_by_identity: bool,
    pub non_claims: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticDerivedKind {
    Replay,
    Transcript,
    EvaluationCache,
    Job,
    RemoteExecution,
    UpgradeCheck,
}

impl SemanticDerivedKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Replay => "replay",
            Self::Transcript => "transcript",
            Self::EvaluationCache => "evaluation-cache",
            Self::Job => "job",
            Self::RemoteExecution => "remote-execution",
            Self::UpgradeCheck => "upgrade-check",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDerivedIdentityInput {
    pub kind: SemanticDerivedKind,
    pub operation: Identity,
    pub subject_ref: String,
    pub handler_profile_ref: String,
    pub dependency_closure_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LiveBindingError {
    SourcePinMismatch,
    ProductGateDenied { gate: &'static str },
    SharedTransition(String),
    SharedResolution(String),
    MissingDependencyClosure,
    DuplicateDependencyClosure,
    ImplicitNestedLateBinding,
    UnsupportedLateBindingProfile,
    InvalidRootClass(String),
    SharedRetirement(String),
    SemanticOperation(String),
    SemanticSurfaceMismatch,
    SemanticContextDenied,
}

pub fn root_class(value: &str) -> Result<RootClassId, LiveBindingError> {
    RootClassId::try_new(value, IDENTIFIER_MAXIMUM_BYTES)
        .map_err(|error| LiveBindingError::InvalidRootClass(format!("{error:?}")))
}
