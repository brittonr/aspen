//! Git bridge for interoperability with standard Git.
//!
//! This module provides bidirectional synchronization between Aspen Forge's
//! BLAKE3-based storage and standard Git repositories (GitHub, GitLab, Gitea, etc.).
//!
//! ## Architecture
//!
//! ```text
//! Standard Git Repository          Aspen Forge
//! ┌──────────────────────┐         ┌──────────────────────┐
//! │ Objects (SHA-1)      │         │ Objects (BLAKE3)     │
//! │ - blob abc123...     │ <-----> │ - SignedObject<Blob> │
//! │ - tree def456...     │         │ - SignedObject<Tree> │
//! │ - commit 789abc...   │         │ - SignedObject<Commit>│
//! └──────────────────────┘         └──────────────────────┘
//!           │                                │
//!           └────────┬───────────────────────┘
//!                    │
//!           ┌────────▼────────┐
//!           │ Hash Mapping    │
//!           │ SHA-1 <-> BLAKE3│
//!           │ (Raft KV)       │
//!           └─────────────────┘
//! ```
//!
//! ## Key Components
//!
//! - **HashMappingStore**: Bidirectional SHA-1 ↔ BLAKE3 mapping storage
//! - **GitObjectConverter**: Translates between git format and `SignedObject<GitObject>`
//! - **TopologicalConverter**: Processes object DAGs in dependency order
//! - **GitImporter**: Imports from external git remotes
//! - **GitExporter**: Exports to external git remotes
//!
//! ## Usage
//!
//! The primary interface is the `git-remote-aspen` binary, which implements
//! the Git remote helper protocol. Users interact via standard git commands:
//!
//! ```bash
//! # Clone from Aspen Forge
//! git clone aspen://<ticket>/<repo_id> my-repo
//!
//! # Push to Aspen Forge
//! git push aspen main
//!
//! # Fetch from Aspen Forge
//! git fetch aspen
//! ```
//!
//! ## Hash Translation Challenge
//!
//! Standard Git and Aspen Forge use incompatible hash schemes:
//!
//! | System      | Hash     | Format                              |
//! |-------------|----------|-------------------------------------|
//! | Standard Git| SHA-1    | `<type> <size>\0<content>`          |
//! | Aspen Forge | BLAKE3   | `postcard(SignedObject<GitObject>)` |
//!
//! The same logical content produces different hashes because:
//! 1. Different hash algorithms (SHA-1 vs BLAKE3)
//! 2. Forge wraps objects in `SignedObject` (adds author, timestamp, signature)
//! 3. Different serialization formats
//!
//! This module maintains a persistent mapping table to translate between the two.

pub mod constants;
pub mod converter;
pub mod error;
pub mod exporter;
pub mod importer;
pub mod mapping;
pub mod sha1;
pub mod topological;

// Re-export primary types
pub use converter::GitObjectConverter;
pub use error::{BridgeError, BridgeResult};
pub use exporter::{ExportResult, ExportedObject, GitExporter};
pub use importer::{GitImporter, ImportResult};
pub use mapping::{GitObjectType, HashMapping, HashMappingStore};
pub use sha1::Sha1Hash;
pub use topological::{
    extract_commit_dependencies, extract_tag_dependencies, extract_tree_dependencies,
    topological_sort, ObjectCollector, PendingObject, TopologicalOrder,
};
