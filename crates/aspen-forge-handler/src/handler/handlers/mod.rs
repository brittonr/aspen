//! Handler sub-modules for Forge operations.
//!
//! Repos, objects, refs, issues, and patches have been migrated to
//! `aspen-forge-plugin` (WASM). Only federation and git-bridge remain
//! as native handlers (they require `ForgeNode` context access).

pub mod federation;
#[cfg(feature = "git-bridge")]
pub mod git_bridge;

// Type alias for the concrete ForgeNode type used in the context
pub(crate) type ForgeNodeRef =
    std::sync::Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>;
