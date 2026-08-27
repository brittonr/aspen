//! Nominal executable-extent identities and admission values.

// r[impl molten.world_extents.identity_domains]
// r[impl molten.world_extents.profile]

macro_rules! identity_type {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; blake3::OUT_LEN]);

        impl $name {
            /// Wraps an already validated BLAKE3 identity.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; blake3::OUT_LEN]) -> Self {
                Self(bytes)
            }

            /// Returns the identity bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; blake3::OUT_LEN] {
                &self.0
            }
        }
    };
}

identity_type!(SemanticCodeIdentity, "Semantic code identity owned by Molten.");
identity_type!(BuiltArtifactIdentity, "Exact built artifact byte identity.");
identity_type!(ExtentManifestIdentity, "Immutable producer extent-manifest identity.");
identity_type!(ExecutableExtentIdentity, "Exact immutable extent byte identity.");
identity_type!(LiveMappingIdentity, "Detached live mapping identity.");
identity_type!(RuntimeCohortIdentity, "Exact Molten runtime cohort identity.");
identity_type!(PolicyIdentity, "Exact current policy identity.");
identity_type!(ProducerReceiptIdentity, "Exact Mantle producer receipt identity.");

/// Optional world code-root profile bound by the artifact root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtentCodeRootProfile {
    /// Semantic code identity.
    pub semantic_code: SemanticCodeIdentity,
    /// Built artifact identity.
    pub built_artifact: BuiltArtifactIdentity,
    /// Executable extent manifest identity.
    pub extent_manifest: ExtentManifestIdentity,
    /// Detached producer receipt identity.
    pub producer_receipt: ProducerReceiptIdentity,
    /// Exact runtime cohort identity.
    pub runtime_cohort: RuntimeCohortIdentity,
    /// Exact policy identity.
    pub policy: PolicyIdentity,
}

/// One ordered producer extent descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtentDescriptor {
    /// Stable zero-based ordinal.
    pub ordinal: u32,
    /// Source artifact byte offset.
    pub source_offset_bytes: u64,
    /// Immutable virtual layout byte offset.
    pub virtual_offset_bytes: u64,
    /// Exact extent length.
    pub length_bytes: u64,
    /// Exact extent byte identity.
    pub identity: ExecutableExtentIdentity,
    /// Final immutable permission intent.
    pub permission: executable_extent_core::ExtentPermission,
}

/// Independently observed extent bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemeasuredExtent {
    /// Stable descriptor ordinal.
    pub ordinal: u32,
    /// Independently observed byte length.
    pub length_bytes: u64,
    /// Independently measured byte identity.
    pub identity: ExecutableExtentIdentity,
}

/// Decoded and independently identified producer bundle facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerBundleFacts {
    /// Optional world code-root profile.
    pub code_root: ExtentCodeRootProfile,
    /// Producer layout identity from the shared core.
    pub layout_identity: [u8; blake3::OUT_LEN],
    /// Exact executable format.
    pub format: String,
    /// Exact target architecture.
    pub architecture: String,
    /// Exact target ABI.
    pub abi: String,
    /// Exact target byte order.
    pub endianness: executable_extent_core::Endianness,
    /// Exact producer page size.
    pub page_size_bytes: u64,
    /// Exact virtual-layout bound.
    pub maximum_virtual_bytes: u64,
    /// Ordered producer extent descriptors.
    pub extents: Vec<ExtentDescriptor>,
    /// Whether every declared bundle member is available.
    pub closure_complete: bool,
}

/// Current consumer compatibility facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsumerProfile {
    /// Runtime-supported architecture.
    pub architecture: String,
    /// Runtime-supported ABI.
    pub abi: String,
    /// Runtime-supported byte order.
    pub endianness: executable_extent_core::Endianness,
    /// Runtime-supported page size.
    pub page_size_bytes: u64,
    /// Runtime cohort selected for this operation.
    pub runtime_cohort: RuntimeCohortIdentity,
    /// Current policy selected for this operation.
    pub policy: PolicyIdentity,
}

/// Current activation facts owned by Molten.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivationFacts {
    /// Artifact admission is current.
    pub artifact_current: bool,
    /// Runtime cohort admission is current.
    pub runtime_current: bool,
    /// Required resources are available.
    pub resources_available: bool,
    /// Policy admission is current.
    pub policy_current: bool,
    /// Current execution authority permits activation.
    pub execution_authorized: bool,
}

/// Explicit weaker or stronger code profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeProfile {
    /// Existing ordinary-artifact path. This never satisfies extent-required policy.
    OrdinaryArtifact(BuiltArtifactIdentity),
    /// Exact executable-extent profile.
    ExecutableExtent(Box<ProducerBundleFacts>),
}

/// Reason that valid extent facts remain inert.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationDenial {
    /// Current artifact admission is missing.
    ArtifactNotCurrent,
    /// Current runtime admission is missing.
    RuntimeNotCurrent,
    /// Required runtime resources are unavailable.
    ResourcesUnavailable,
    /// Current policy admission is missing.
    PolicyNotCurrent,
    /// Current execution authority denies activation.
    ExecutionUnauthorized,
}

/// Current activation decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationDecision {
    /// Mapping and activation may proceed under current supplied facts.
    Admit,
    /// Extents remain inert.
    Deny(ActivationDenial),
}

/// One shared W^X mapping transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MappingTransition {
    /// Shared core transition plan.
    pub plan: executable_extent_core::TransitionPlan,
}

/// Mapping intent for one admitted extent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtentMappingIntent {
    /// Stable extent ordinal.
    pub ordinal: u32,
    /// Initial read-only mapping transition.
    pub map_read_only: MappingTransition,
    /// Optional executable read-only protection transition.
    pub protect_executable: Option<MappingTransition>,
    /// Explicit unmap transition from the final mapped state.
    pub unmap: MappingTransition,
}

/// Pure executable-extent admission plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtentPlan {
    /// Optional world code-root profile admitted by this plan.
    pub code_root: ExtentCodeRootProfile,
    /// Recomputed shared layout identity.
    pub layout_identity: [u8; blake3::OUT_LEN],
    /// Ordered mapping intents.
    pub mappings: Vec<ExtentMappingIntent>,
    /// Current activation decision.
    pub activation: ActivationDecision,
}

/// Pure profile decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmissionDecision {
    /// Existing weaker path was selected explicitly.
    OrdinaryArtifact(BuiltArtifactIdentity),
    /// Exact executable extents were admitted.
    ExecutableExtents(Box<ExtentPlan>),
}

/// Deterministic admission denial.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionError {
    /// Extent-required policy received the weaker profile.
    ExtentProfileRequired,
    /// Producer format is unsupported.
    UnsupportedFormat,
    /// Producer closure is incomplete.
    IncompleteClosure,
    /// Producer and consumer runtime cohort identities differ.
    RuntimeCohortMismatch,
    /// Producer and consumer policy identities differ.
    PolicyIdentityMismatch,
    /// Producer extents or observations are empty.
    EmptyExtents,
    /// Extent descriptor count differs from remeasurement count.
    RemeasurementShapeMismatch,
    /// Extent ordinals are not exact and ordered.
    ExtentOrdinalMismatch,
    /// Independent byte length differs.
    ExtentLengthMismatch,
    /// Independent byte identity differs.
    ExtentIdentityMismatch,
    /// Shared layout admission failed.
    Layout(executable_extent_core::LayoutError),
    /// Recomputed layout identity differs from the producer fact.
    LayoutIdentityMismatch,
    /// Producer and consumer compatibility differs.
    Compatibility(executable_extent_core::CompatibilityError),
    /// Shared W^X transition admission failed.
    Transition(executable_extent_core::TransitionError),
}
