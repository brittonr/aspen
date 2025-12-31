//! Forge: Git on Aspen - decentralized code collaboration.
//!
//! This crate provides a decentralized code collaboration system with Radicle-like
//! features, built on Aspen's distributed primitives:
//!
//! - **Git Objects**: Commits, trees, blobs stored in iroh-blobs (content-addressed)
//! - **Collaborative Objects (COBs)**: Issues, patches, reviews as immutable DAGs
//! - **Refs**: Strongly consistent branch/tag storage via Raft consensus
//! - **Gossip**: Real-time announcements of new commits and COB changes
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    IMMUTABLE LAYER                           │
//! │                    (iroh-blobs)                              │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
//! │  │ Git Objects │ │ COB Changes │ │ Signed Attestations │   │
//! │  └─────────────┘ └─────────────┘ └─────────────────────┘   │
//! │         ↓               ↓               ↓                   │
//! │    BLAKE3 hash     BLAKE3 hash     BLAKE3 hash             │
//! └─────────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    MUTABLE LAYER                             │
//! │                    (Raft KV)                                 │
//! │  refs/heads/main → Hash, cobs/issue/{id}:heads → [Hash]    │
//! └─────────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    DISCOVERY LAYER                           │
//! │  iroh-gossip (announcements) + DHT (find seeders)           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use aspen::forge::{ForgeNode, RepoId};
//!
//! // Create a forge node from an Aspen cluster
//! let forge = ForgeNode::new(blob_store, kv_store, gossip).await?;
//!
//! // Create a new repository
//! let repo_id = forge.create_repo("my-project", &delegates).await?;
//!
//! // Create a commit
//! let tree = forge.git.create_tree(&[("README.md", readme_hash)]).await?;
//! let commit = forge.git.commit(&repo_id, tree, vec![], "Initial commit").await?;
//!
//! // Push to main branch
//! forge.refs.update(&repo_id, "heads/main", commit).await?;
//!
//! // Create an issue
//! let issue = forge.cobs.create_issue(&repo_id, "Bug report", "Description").await?;
//! ```
//!
//! ## Design Principles
//!
//! 1. **Immutable Objects**: Git objects and COB changes are stored as content-addressed
//!    blobs in iroh-blobs. This enables efficient P2P distribution and deduplication.
//!
//! 2. **Strong Consistency for Refs**: Branch and tag updates go through Raft consensus,
//!    ensuring all nodes agree on the canonical state.
//!
//! 3. **DAG-based COBs**: Issues, patches, and other collaborative objects are modeled
//!    as immutable change DAGs (like Git commits), enabling offline-first collaboration.
//!
//! 4. **Native BLAKE3**: All content addressing uses BLAKE3 (no SHA-1 compatibility layer).

pub mod constants;
pub mod error;
pub mod types;

pub mod cob;
pub mod git;
pub mod identity;
pub mod refs;

pub mod gossip;
mod node;
pub mod sync;

// Re-export primary types for convenient access
pub use cob::{
    CobChange, CobOperation, CobStore, CobType, CobUpdateEvent, ConflictReport, ConflictingValue,
    FieldConflict, FieldResolution, MergeStrategy,
};
pub use error::ForgeError;
pub use git::{GitBlobStore, GitObject, TreeEntry};
#[cfg(feature = "git-bridge")]
pub use git::GIT_BRIDGE_ALPN;
pub use gossip::{
    Announcement, AnnouncementCallback, ForgeAnnouncementHandler, ForgeGossipRateLimiter,
    ForgeGossipService, ForgeTopic, SignedAnnouncement,
};
pub use identity::{Author, RepoId, RepoIdentity};
pub use node::ForgeNode;
pub use refs::{RefStore, RefUpdateEvent};
pub use sync::SyncService;
pub use types::SignedObject;
