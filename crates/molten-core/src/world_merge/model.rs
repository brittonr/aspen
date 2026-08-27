use std::collections::BTreeMap;

use crate::world_commit::RootKind;
use crate::world_commit::WorldCommitRef;
use crate::world_commit::WorldRootRef;

pub const WORLD_DIFF_SCHEMA: &str = "molten.world-diff.v1";
pub const WORLD_MERGE_PLAN_SCHEMA: &str = "molten.world-merge-plan.v1";
pub const WORLD_MERGE_CONFLICT_SCHEMA: &str = "molten.world-merge-conflict.v1";
pub const WORLD_MERGE_RESULT_SCHEMA: &str = "molten.world-merge-result.v1";
pub const MAX_WORLD_MERGE_ROOTS: u32 = 32;
pub const MAX_WORLD_MERGE_KEYS: u32 = 4_096;
pub const MAX_WORLD_MERGE_VALUE_BYTES: u64 = 1_048_576;
pub const MAX_WORLD_MERGE_CONFLICTS: u32 = 256;

macro_rules! digest_reference {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, WorldMergeReferenceError> {
                let value = value.into();
                validate_digest_reference(&value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

digest_reference!(WorldMergeProfileRef);
digest_reference!(WorldMergeSchemaRef);
digest_reference!(WorldMigrationPlanRef);
digest_reference!(WorldMergeHandlerRef);
digest_reference!(WorldMergePolicyRef);
digest_reference!(WorldMergePlanRef);
digest_reference!(WorldMergeConflictRef);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldRootDiffClass {
    Equal,
    Changed,
    Absent,
    Unavailable,
    Incompatible,
    ProfileExcluded,
}

impl WorldRootDiffClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Changed => "changed",
            Self::Absent => "absent",
            Self::Unavailable => "unavailable",
            Self::Incompatible => "incompatible",
            Self::ProfileExcluded => "profile-excluded",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldMergeMode {
    IdenticalOnly,
    AncestorReplacement,
    KeyedDurableValues,
    ApplicationHandler,
}

impl WorldMergeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IdenticalOnly => "identical-only",
            Self::AncestorReplacement => "ancestor-replacement",
            Self::KeyedDurableValues => "keyed-durable-values",
            Self::ApplicationHandler => "application-handler",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMergeBounds {
    pub max_roots: u32,
    pub max_keys: u32,
    pub max_value_bytes: u64,
    pub max_conflicts: u32,
}

impl WorldMergeBounds {
    pub const fn standard() -> Self {
        Self {
            max_roots: MAX_WORLD_MERGE_ROOTS,
            max_keys: MAX_WORLD_MERGE_KEYS,
            max_value_bytes: MAX_WORLD_MERGE_VALUE_BYTES,
            max_conflicts: MAX_WORLD_MERGE_CONFLICTS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMergeValue {
    pub root: Option<WorldRootRef>,
    pub schema_ref: Option<WorldMergeSchemaRef>,
    pub available: bool,
    pub canonical_bytes: Option<Vec<u8>>,
    pub keyed_values: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMergeRootInput {
    pub kind: RootKind,
    pub base: WorldMergeValue,
    pub left: WorldMergeValue,
    pub right: WorldMergeValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMigrationBinding {
    pub plan_ref: WorldMigrationPlanRef,
    pub profile_id: String,
    pub source_schema: WorldMergeSchemaRef,
    pub target_schema: WorldMergeSchemaRef,
    pub admitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldApplicationHandlerProfile {
    pub handler_ref: WorldMergeHandlerRef,
    pub behavior_ref: WorldMergeHandlerRef,
    pub input_schema: WorldMergeSchemaRef,
    pub output_schema: WorldMergeSchemaRef,
    pub policy_ref: WorldMergePolicyRef,
    pub max_output_bytes: u64,
    pub pure: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMergeProfile {
    pub profile_ref: WorldMergeProfileRef,
    pub policy_ref: WorldMergePolicyRef,
    pub root_modes: BTreeMap<RootKind, WorldMergeMode>,
    pub migrations: BTreeMap<RootKind, WorldMigrationBinding>,
    pub handlers: BTreeMap<RootKind, WorldApplicationHandlerProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMergeRequest {
    pub base_head: WorldCommitRef,
    pub source_heads: Vec<WorldCommitRef>,
    pub common_ancestor_verified: bool,
    pub common_ancestor_ambiguous: bool,
    pub roots: Vec<WorldMergeRootInput>,
    pub profile: WorldMergeProfile,
    pub bounds: WorldMergeBounds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldRootDiff {
    pub kind: RootKind,
    pub class: WorldRootDiffClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldDiffReport {
    pub base_head: WorldCommitRef,
    pub source_heads: Vec<WorldCommitRef>,
    pub roots: Vec<WorldRootDiff>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMergeConflict {
    pub kind: RootKind,
    pub key: Option<String>,
    pub code: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMergedRoot {
    pub kind: RootKind,
    pub selected_root: Option<WorldRootRef>,
    pub generated_values: BTreeMap<String, Vec<u8>>,
    pub generated_bytes: Option<Vec<u8>>,
    pub output_schema: Option<WorldMergeSchemaRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMergePlan {
    pub plan_ref: WorldMergePlanRef,
    pub base_head: WorldCommitRef,
    pub source_heads: Vec<WorldCommitRef>,
    pub outputs: Vec<WorldMergedRoot>,
    pub conflicts: Vec<WorldMergeConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldMergeIssue {
    InvalidBounds(&'static str),
    MissingBase,
    AmbiguousBase,
    SourceCountInvalid,
    DuplicateSource,
    DuplicateRoot,
    RootLimitExceeded,
    KeyLimitExceeded,
    ValueLimitExceeded,
    ConflictLimitExceeded,
    UnavailableRoot(RootKind),
    IncompatibleSchema(RootKind),
    MigrationRequired(RootKind),
    MigrationMismatch(RootKind),
    MigrationProfileInvalid(RootKind),
    RuntimeSensitiveRoot(RootKind),
    ModeNotDeclared(RootKind),
    HandlerMissing(RootKind),
    HandlerMismatch(RootKind),
    HandlerEffectRequested(RootKind),
    HandlerFailed(RootKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldMergeReferenceError {
    InvalidDigest,
}

impl std::fmt::Display for WorldMergeReferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for WorldMergeReferenceError {}

fn validate_digest_reference(value: &str) -> Result<(), WorldMergeReferenceError> {
    const BLAKE3_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_LENGTH: usize = 64;
    let digest = value.strip_prefix(BLAKE3_PREFIX).ok_or(WorldMergeReferenceError::InvalidDigest)?;
    if digest.len() != BLAKE3_HEX_LENGTH
        || !digest.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(WorldMergeReferenceError::InvalidDigest);
    }
    Ok(())
}
