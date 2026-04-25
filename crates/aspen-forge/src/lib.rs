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
//! 1. **Immutable Objects**: Git objects and COB changes are stored as content-addressed blobs in
//!    iroh-blobs. This enables efficient P2P distribution and deduplication.
//!
//! 2. **Strong Consistency for Refs**: Branch and tag updates go through Raft consensus, ensuring
//!    all nodes agree on the canonical state.
//!
//! 3. **DAG-based COBs**: Issues, patches, and other collaborative objects are modeled as immutable
//!    change DAGs (like Git commits), enabling offline-first collaboration.
//!
//! 4. **Native BLAKE3**: All content addressing uses BLAKE3 (no SHA-1 compatibility layer).

pub mod constants;
pub mod dag_sync;
pub mod error;
pub mod federation;
pub mod mirror;
pub mod mirror_worker;
pub mod resolver;
pub mod types;
/// Verified pure functions for Forge logic.
pub mod verified;

pub mod cob;
pub mod git;
pub mod identity;
pub mod jj;
pub mod nostr_bridge;
pub mod refs;

pub mod gossip;
mod node;
pub mod protection;
pub mod status;
pub mod sync;

// Re-export primary types for convenient access
pub use aspen_dag::DAG_SYNC_ALPN;
pub use aspen_dag::DagSyncProtocolHandler;
pub use cob::Approval;
pub use cob::ChangeRequest;
pub use cob::CobChange;
pub use cob::CobOperation;
pub use cob::CobStore;
pub use cob::CobType;
pub use cob::CobUpdateEvent;
pub use cob::ConflictReport;
pub use cob::ConflictingValue;
pub use cob::Discussion;
pub use cob::DiscussionReply;
pub use cob::DiscussionScalarFieldValue;
pub use cob::DiscussionState;
pub use cob::FieldConflict;
pub use cob::FieldResolution;
pub use cob::MergeStrategy;
pub use dag_sync::CobLinkExtractor;
pub use dag_sync::ForgeNodeType;
pub use dag_sync::GitLinkExtractor;
pub use error::ForgeError;
// Federation
pub use federation::FORGE_RESOURCE_TYPE;
pub use federation::forge_repo_policy;
pub use git::BlobObject;
pub use git::CommitObject;
pub use git::ConflictKind;
pub use git::DiffEntry;
pub use git::DiffKind;
pub use git::DiffOptions;
pub use git::DiffResult;
#[cfg(feature = "git-bridge")]
pub use git::GIT_BRIDGE_ALPN;
pub use git::GitBlobStore;
pub use git::GitMergeStrategy;
pub use git::GitObject;
pub use git::MergeConflict;
pub use git::TagObject;
pub use git::TreeEntry;
pub use git::TreeMergeResult;
pub use git::TreeObject;
pub use gossip::Announcement;
pub use gossip::AnnouncementCallback;
pub use gossip::ForgeAnnouncementHandler;
pub use gossip::ForgeGossipRateLimiter;
pub use gossip::ForgeGossipService;
pub use gossip::ForgeTopic;
pub use gossip::SignedAnnouncement;
pub use identity::Author;
pub use identity::ForkInfo;
pub use identity::RepoId;
pub use identity::RepoIdentity;
pub use jj::EncodedJjObject;
pub use jj::JjBookmarkRecord;
pub use jj::JjBookmarkUpdate;
pub use jj::JjChangeHeadRecord;
pub use jj::JjChangeHeadUpdate;
pub use jj::JjHeadKind;
pub use jj::JjObjectEncodingError;
pub use jj::JjObjectEnvelope;
pub use jj::JjObjectGraphError;
pub use jj::JjObjectKind;
pub use jj::JjObjectStore;
pub use jj::JjObjectStoreError;
pub use jj::JjPublishConflict;
pub use jj::JjPublishReceipt;
pub use jj::JjStagedPublish;
pub use jj::JjStoredObjectRef;
pub use jj::conflict_if_head_mismatch;
pub use jj::decode_jj_object;
pub use jj::encode_jj_object;
pub use jj::jj_bookmark_key;
pub use jj::jj_change_head_key;
pub use jj::jj_reachability_key;
pub use jj::validate_jj_object;
pub use jj::validate_jj_object_graph;
pub use mirror::MirrorConfig;
pub use mirror::MirrorStatus;
pub use mirror_worker::MIRROR_SYNC_JOB_TYPE;
pub use mirror_worker::MirrorSyncPayload;
pub use mirror_worker::MirrorSyncResult;
pub use mirror_worker::mirror_sync_job_spec;
pub use mirror_worker::sync_mirror_refs;
pub use node::ForgeNode;
pub use node::MergeCheckResult;
pub use protection::BranchProtection;
pub use protection::MergeChecker;
pub use protection::ProtectionStore;
pub use refs::RefStore;
pub use refs::RefUpdateEvent;
pub use status::CommitCheckState;
pub use status::CommitStatus;
pub use status::StatusStore;
pub use sync::DagSyncPlan;
pub use sync::DagSyncResult;
pub use sync::DagSyncType;
pub use sync::DagSyncWorker;
pub use sync::SyncService;
pub use types::SignedObject;
