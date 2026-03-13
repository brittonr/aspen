//! Copy-on-write branch overlay for Aspen's `KeyValueStore` trait.
//!
//! `BranchOverlay<S>` wraps any `KeyValueStore` and buffers writes in-memory.
//! Reads fall through to the parent for keys not in the dirty map. Commit
//! flushes all buffered mutations as a single atomic Raft batch. Dropping
//! discards everything with no Raft interaction.
//!
//! # Usage
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use aspen_kv_branch::BranchOverlay;
//! use aspen_kv_types::{ReadRequest, WriteRequest};
//! use aspen_traits::KeyValueStore;
//!
//! // Wrap any KeyValueStore in a branch.
//! let store: Arc<dyn KeyValueStore> = /* ... */;
//! let branch = BranchOverlay::new("my-branch", store);
//!
//! // Writes buffer in-memory — parent is not touched.
//! branch.write(WriteRequest::set("key", "value")).await?;
//!
//! // Reads check the branch first, then fall through to parent.
//! let result = branch.read(ReadRequest::new("key")).await?;
//!
//! // Commit flushes everything as one atomic Raft batch.
//! branch.commit().await?;
//!
//! // Or just drop the branch to discard all writes (zero-cost abort).
//! // drop(branch);
//! ```
//!
//! # Nested Branches
//!
//! Branches compose: `BranchOverlay<BranchOverlay<S>>`. An inner commit
//! merges into the outer branch's dirty map. Only the root commit
//! touches Raft.
//!
//! ```rust,ignore
//! let outer = Arc::new(BranchOverlay::new("outer", store));
//! let inner = outer.child("inner")?;
//!
//! inner.write(WriteRequest::set("key", "val")).await?;
//! inner.commit().await?;  // Merges into outer, no Raft
//! outer.commit().await?;  // Now hits Raft
//! ```
//!
//! # Resource Bounds
//!
//! Tiger Style limits prevent memory exhaustion:
//! - `MAX_BRANCH_DIRTY_KEYS` (10,000): max dirty entries per branch
//! - `MAX_BRANCH_TOTAL_BYTES` (64 MB): max dirty value bytes
//! - `MAX_BRANCH_DEPTH` (8): max nesting depth
//! - `BRANCH_COMMIT_TIMEOUT_MS` (10s): Raft write timeout
//!
//! Override with [`BranchConfig`].

mod config;
mod constants;
mod entry;
mod error;
mod overlay;
pub mod verified;

pub use config::BranchConfig;
pub use config::BranchStats;
pub use constants::*;
pub use entry::BranchEntry;
pub use error::BranchError;
pub use overlay::BranchOverlay;
