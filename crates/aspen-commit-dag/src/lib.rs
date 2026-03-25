//! Chain-hashed commit DAG for Aspen KV branch history.
//!
//! Every `BranchOverlay.commit()` produces a `CommitId` (BLAKE3 chain hash)
//! linking it to its parent, forming an append-only DAG. Commits capture the
//! mutations, the Raft revision they landed at, and the chain hash at that point.
//!
//! Federation sync carries commit metadata alongside KV data, enabling the
//! importing cluster to verify that received state was produced by legitimate
//! Raft consensus.

pub mod constants;
pub mod error;
pub mod fork;
pub mod gc;
pub mod store;
pub mod types;
pub mod verified;

pub use constants::*;
pub use error::*;
pub use fork::ForkSource;
pub use fork::load_fork_source;
pub use gc::CommitGc;
pub use store::CommitStore;
pub use types::*;
pub use verified::commit_hash::compute_commit_id;
pub use verified::commit_hash::compute_mutations_hash;
pub use verified::commit_hash::verify_commit_integrity;
pub use verified::diff::diff;
