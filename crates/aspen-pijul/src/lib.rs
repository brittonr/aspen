//! Pijul: Native patch-based version control for Aspen.
//!
//! This module provides native Pijul integration, embedding libpijul as a first-class
//! VCS format alongside Git. Unlike Git's snapshot-based model, Pijul uses a patch-based
//! approach with commutative merges based on category theory.
//!
//! ## Architecture
//!
//! ```text
//! +------------------------------------------------------------------+
//! |                        PIJUL MODULE                              |
//! |  +----------------+  +-----------------+  +------------------+   |
//! |  | PristineStore  |  | AspenChangeStore|  | PijulRefStore    |   |
//! |  | (sanakirja)    |  | (ChangeStore)   |  | (Raft KV)        |   |
//! |  | node-local     |  | wraps BlobStore |  | channel heads    |   |
//! |  +----------------+  +-----------------+  +------------------+   |
//! |          |                   |                    |              |
//! |          v                   v                    v              |
//! |    .pijul/pristine/    iroh-blobs          Raft consensus       |
//! |    (sanakirja DB)      (P2P ready)         (strongly consistent)|
//! +------------------------------------------------------------------+
//! ```
//!
//! ## Key Differences from Git
//!
//! - **Patches vs Snapshots**: Pijul stores changes (patches) that can be applied in any order,
//!   while Git stores snapshots of file states.
//! - **Commutative Merges**: Patches can be merged in any order with the same result, eliminating
//!   the need for merge commits and history rewriting.
//! - **Cherry-picking**: Pulling individual patches doesn't create duplicate commits.
//! - **Channels**: Similar to branches, but a channel is exactly a set of patches.
//!
//! ## Storage Strategy
//!
//! - **Pristine** (sanakirja): libpijul's internal file graph representation, stored node-locally
//!   as it can be reconstructed from changes.
//! - **Changes** (iroh-blobs): Immutable patch objects, content-addressed with BLAKE3,
//!   automatically P2P distributable.
//! - **Channel Heads** (Raft KV): Strongly consistent pointers to latest changes, ensuring
//!   cluster-wide agreement on branch state.
//!
//! ## Usage
//!
//! ```ignore
//! use aspen::pijul::{PijulStore, PijulRepoIdentity};
//!
//! // Create a Pijul store
//! let store = PijulStore::new(blob_store, kv_store, data_dir).await?;
//!
//! // Create a new repository
//! let identity = PijulRepoIdentity::new("my-project", delegates);
//! let repo_id = store.create_repo(identity).await?;
//!
//! // Record a change
//! let change_hash = store.record(&repo_id, "main", "Initial commit").await?;
//!
//! // Sync changes from a peer
//! store.sync_channel(&repo_id, "main", peer_id).await?;
//! ```

// Allow some clippy lints that are difficult to fix in libpijul integration code
#![allow(clippy::manual_flatten)]
#![allow(clippy::let_and_return)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::collapsible_if)]

pub mod apply;
pub mod blame;
pub mod change_store;
pub mod constants;
pub mod error;
pub mod handler;
pub mod output;
pub mod pristine;
pub mod record;
pub mod refs;
pub mod resolution;
pub mod store;
pub mod types;

pub mod gossip;
pub mod sync;
pub mod working_dir;

#[cfg(test)]
mod tests;

// Re-export primary types
pub use apply::ApplyResult;
pub use apply::ChangeApplicator;
pub use apply::ChangeDirectory;
pub use blame::BlameResult;
pub use blame::FileAttribution;
pub use change_store::AspenChangeStore;
pub use constants::*;
pub use error::PijulError;
pub use error::PijulResult;
pub use gossip::PijulAnnouncement;
pub use gossip::PijulAnnouncementHandler;
pub use gossip::PijulTopic;
pub use gossip::SignedPijulAnnouncement;
// Resolution exports
pub use handler::DownloadMetrics;
pub use handler::PijulSyncHandler;
pub use handler::PijulSyncHandlerHandle;
pub use output::OutputResult;
pub use output::WorkingDirOutput;
pub use pristine::PristineHandle;
pub use pristine::PristineManager;
pub use pristine::ReadTxn;
pub use pristine::WriteTxn;
pub use record::ChangeRecorder;
pub use record::DiffHunkInfo;
pub use record::DiffResult;
pub use record::RecordResult;
pub use refs::ChannelUpdateEvent;
pub use refs::PijulRefStore;
pub use resolution::ChannelResolutionState;
pub use resolution::ResolutionAction;
pub use resolution::ResolutionStrategy;
pub use store::PijulStore;
pub use store::PijulStoreEvent;
pub use store::SyncResult;
pub use sync::PijulSyncCallback;
pub use sync::PijulSyncService;
pub use types::*;
pub use working_dir::AddResult;
pub use working_dir::FileStatus;
pub use working_dir::FileStatusEntry;
pub use working_dir::ResetResult;
pub use working_dir::WorkingDirectory;
pub use working_dir::WorkingDirectoryConfig;
pub use working_dir::WorkingDirectoryStatus;
