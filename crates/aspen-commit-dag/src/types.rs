//! Core types for the commit DAG.

use aspen_raft::verified::ChainHash;
use serde::Deserialize;
use serde::Serialize;

/// Unique identifier for a commit, computed as a BLAKE3 chain hash.
///
/// `CommitId = blake3(parent_hash || branch_id || mutations_hash || raft_revision || timestamp_ms)`
pub type CommitId = ChainHash;

/// The type of mutation applied to a key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationType {
    /// Key was set to this value.
    Set(String),
    /// Key was deleted.
    Delete,
}

/// An immutable commit snapshot stored in KV at `_sys:commit:{hex}`.
///
/// Captures the mutations, Raft position, and chain linkage for a single
/// `BranchOverlay.commit()` operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    /// The chain-hashed identifier for this commit.
    pub id: CommitId,
    /// Parent commit in the chain (None for the first commit on a branch).
    pub parent: Option<CommitId>,
    /// The branch that produced this commit.
    pub branch_id: String,
    /// Sorted mutations snapshot from the dirty map at commit time.
    pub mutations: Vec<(String, MutationType)>,
    /// BLAKE3 hash over the sorted mutations.
    pub mutations_hash: ChainHash,
    /// The Raft log revision at which this commit landed.
    pub raft_revision: u64,
    /// The Raft chain hash at the commit's log position.
    pub chain_hash_at_commit: ChainHash,
    /// Wall-clock timestamp when the commit was created.
    pub timestamp_ms: u64,
}

/// Result of comparing two commits' mutation snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffEntry {
    /// Key exists in `b` but not in `a`.
    Added { key: String, value: MutationType },
    /// Key exists in `a` but not in `b`.
    Removed { key: String },
    /// Key exists in both but with different mutations.
    Changed {
        key: String,
        old: MutationType,
        new: MutationType,
    },
}
