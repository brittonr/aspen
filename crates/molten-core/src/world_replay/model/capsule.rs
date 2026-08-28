use super::trace::WorldReplayProfile;
use crate::world_commit::RootKind;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldRootRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReplayCapsuleMemberRole {
    Trace,
    WorldCommit,
    TypedRoot(RootKind),
    Artifact,
    Schema,
    Policy,
    RuntimeProfile,
    RuntimeCohort,
    SnapshotDescriptor,
    TransitionInput,
    ContentManifest,
    SealedReproductionBundle,
}

impl WorldReplayCapsuleMemberRole {
    pub fn label(self) -> String {
        match self {
            Self::Trace => "trace".to_string(),
            Self::WorldCommit => "world-commit".to_string(),
            Self::TypedRoot(kind) => format!("typed-root:{}", kind.as_str()),
            Self::Artifact => "artifact".to_string(),
            Self::Schema => "schema".to_string(),
            Self::Policy => "policy".to_string(),
            Self::RuntimeProfile => "runtime-profile".to_string(),
            Self::RuntimeCohort => "runtime-cohort".to_string(),
            Self::SnapshotDescriptor => "snapshot-descriptor".to_string(),
            Self::TransitionInput => "transition-input".to_string(),
            Self::ContentManifest => "content-manifest".to_string(),
            Self::SealedReproductionBundle => "sealed-reproduction-bundle".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReplayMemberCodec {
    CanonicalPreservesV1,
    RawBytesV1,
    ContentManifestV1,
    SealedReproductionBundleV1,
}

impl WorldReplayMemberCodec {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalPreservesV1 => "canonical-preserves-v1",
            Self::RawBytesV1 => "raw-bytes-v1",
            Self::ContentManifestV1 => "content-manifest-v1",
            Self::SealedReproductionBundleV1 => "sealed-reproduction-bundle-v1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReplayMemberProtection {
    Public,
    Ciphertext { descriptor_ref: String },
}

impl WorldReplayMemberProtection {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Ciphertext { .. } => "ciphertext",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayCapsuleMember {
    pub object_ref: String,
    pub roles: Vec<WorldReplayCapsuleMemberRole>,
    pub codec: WorldReplayMemberCodec,
    pub byte_length: u64,
    pub protection: WorldReplayMemberProtection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayCapsule {
    pub schema: String,
    pub capsule_ref: String,
    pub trace_ref: String,
    pub profile: WorldReplayProfile,
    pub members: Vec<WorldReplayCapsuleMember>,
    pub non_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorldReplayClosureRequirement {
    pub object_ref: String,
    pub role: WorldReplayCapsuleMemberRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayCommitClosure {
    pub commit_ref: WorldCommitRef,
    pub parents: Vec<WorldCommitRef>,
    pub roots: Vec<WorldRootRef>,
    pub canonical_identity_verified: bool,
}
