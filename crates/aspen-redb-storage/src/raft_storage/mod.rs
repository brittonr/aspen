//! Single-fsync Redb storage implementation for reusable Raft KV.
//!
//! Provides a unified storage backend implementing both `RaftLogStorage`
//! and `RaftStateMachine` traits on a single struct, enabling single-fsync
//! writes for ~2-3ms latency.

mod chain;
mod error;
mod index;
mod initialization;
mod kv;
mod lease;
mod log_storage;
mod meta;
mod sm_trait;
mod snapshot;
mod state_machine;
mod types;

pub use snapshot::RedbKvSnapshotBuilder;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;

use aspen_layer::IndexRegistry;
use openraft::alias::LogIdOf;
use redb::Database;
use tokio::sync::broadcast;

pub use error::*;
pub use types::*;

use crate::ChainTipState;

use aspen_raft_kv_types::RaftKvResponse;
use aspen_raft_kv_types::RaftKvTypeConfig;

/// Single-fsync Redb storage for reusable Raft KV.
#[derive(Clone)]
pub struct RedbKvStorage {
    pub(crate) db: Arc<Database>,
    pub(crate) path: PathBuf,
    pub(crate) local_node_id: Option<u64>,
    pub(crate) index_registry: Arc<IndexRegistry>,
    pub(crate) chain_tip: Arc<StdRwLock<ChainTipState>>,
    pub(crate) pending_responses: Arc<StdRwLock<BTreeMap<u64, RaftKvResponse>>>,
    pub(crate) confirmed_last_applied: Arc<StdRwLock<Option<LogIdOf<RaftKvTypeConfig>>>>,
    pub(crate) snapshot_event_tx: Option<broadcast::Sender<SnapshotEvent>>,
}

impl std::fmt::Debug for RedbKvStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedbKvStorage")
            .field("path", &self.path)
            .field("local_node_id", &self.local_node_id)
            .finish_non_exhaustive()
    }
}



impl RedbKvStorage {
    pub fn db(&self) -> &Arc<Database> {
        &self.db
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn index_registry(&self) -> &Arc<IndexRegistry> {
        &self.index_registry
    }

    pub fn subscribe_snapshots(&self) -> Option<broadcast::Receiver<SnapshotEvent>> {
        self.snapshot_event_tx.as_ref().map(|tx| tx.subscribe())
    }
}
