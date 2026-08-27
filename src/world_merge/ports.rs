use molten_core::world_commit::RootKind;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_merge::WorldApplicationHandlerProfile;
use molten_core::world_merge::WorldMergePolicyRef;
use molten_core::world_merge::WorldMergeSchemaRef;
use molten_core::world_merge::WorldMigrationBinding;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldMergePortError {
    pub class: &'static str,
    pub message: String,
}

impl WorldMergePortError {
    pub fn new(class: &'static str, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for WorldMergePortError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.class, self.message)
    }
}

impl std::error::Error for WorldMergePortError {}

pub trait WorldMergeObjectPort {
    fn load_root(&mut self, root: &WorldRootRef, maximum_bytes: u64) -> Result<Vec<u8>, WorldMergePortError>;

    fn persist_generated_root(
        &mut self,
        kind: RootKind,
        schema_ref: &WorldMergeSchemaRef,
        canonical_bytes: &[u8],
    ) -> Result<WorldRootRef, WorldMergePortError>;
}

pub trait WorldMergeMigrationPort {
    fn materialize_migration(
        &mut self,
        binding: &WorldMigrationBinding,
        source_bytes: &[u8],
    ) -> Result<Vec<u8>, WorldMergePortError>;
}

pub trait WorldMergeHandlerPort {
    fn load_handler(
        &mut self,
        profile: &WorldApplicationHandlerProfile,
    ) -> Result<Box<dyn molten_core::world_merge::WorldMergeHandler>, WorldMergePortError>;
}

pub trait WorldMergeAuthorityPort {
    fn recheck_merge_authority(
        &mut self,
        source_heads: &[WorldCommitRef],
        policy_ref: &WorldMergePolicyRef,
    ) -> Result<String, WorldMergePortError>;
}

pub trait WorldMergeConflictPort {
    fn persist_conflict(&mut self, conflict_ref: &str, canonical_bytes: &[u8]) -> Result<(), WorldMergePortError>;
}

pub trait WorldMergeCommitPort {
    fn publish_merge_commit(
        &mut self,
        base_head: &WorldCommitRef,
        source_heads: &[WorldCommitRef],
        roots: &[WorldRootRef],
    ) -> Result<WorldCommitRef, WorldMergePortError>;
}
