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
//! - **Patches vs Snapshots**: Pijul stores changes (patches) that can be applied
//!   in any order, while Git stores snapshots of file states.
//! - **Commutative Merges**: Patches can be merged in any order with the same result,
//!   eliminating the need for merge commits and history rewriting.
//! - **Cherry-picking**: Pulling individual patches doesn't create duplicate commits.
//! - **Channels**: Similar to branches, but a channel is exactly a set of patches.
//!
//! ## Storage Strategy
//!
//! - **Pristine** (sanakirja): libpijul's internal file graph representation, stored
//!   node-locally as it can be reconstructed from changes.
//! - **Changes** (iroh-blobs): Immutable patch objects, content-addressed with BLAKE3,
//!   automatically P2P distributable.
//! - **Channel Heads** (Raft KV): Strongly consistent pointers to latest changes,
//!   ensuring cluster-wide agreement on branch state.
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

pub mod apply;
pub mod change_store;
pub mod constants;
pub mod error;
pub mod output;
pub mod pristine;
pub mod record;
pub mod refs;
pub mod store;
pub mod types;

pub mod gossip;
pub mod sync;

// Re-export primary types
pub use apply::{ApplyResult, ChangeApplicator, ChangeDirectory};
pub use change_store::AspenChangeStore;
pub use constants::*;
pub use error::{PijulError, PijulResult};
pub use gossip::{
    PijulAnnouncement, PijulAnnouncementHandler, PijulTopic, SignedPijulAnnouncement,
};
pub use output::{OutputResult, WorkingDirOutput};
pub use sync::{PijulSyncCallback, PijulSyncService};
pub use pristine::{PristineHandle, PristineManager, ReadTxn, WriteTxn};
pub use record::{ChangeRecorder, RecordResult};
pub use refs::{ChannelUpdateEvent, PijulRefStore};
pub use store::{PijulStore, PijulStoreEvent};
pub use types::*;
