//! Handler sub-modules for Forge operations.
//!
//! Each module contains handlers for a specific category of operations.

pub mod federation;
#[cfg(feature = "git-bridge")]
pub mod git_bridge;
pub mod log;
pub mod objects;
pub mod refs;
pub mod repo;

// Type alias for the concrete ForgeNode type used in the context
pub(crate) type ForgeNodeRef =
    std::sync::Arc<aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, dyn aspen_core::KeyValueStore>>;
