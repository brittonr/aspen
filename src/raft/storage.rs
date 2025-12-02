use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures::{Stream, TryStreamExt};
use openraft::alias::{LogIdOf, SnapshotDataOf, VoteOf};
use openraft::entry::RaftEntry;
use openraft::storage::{
    EntryResponder, IOFlushed, RaftLogStorage, RaftSnapshotBuilder, RaftStateMachine, Snapshot,
};
use openraft::{EntryPayload, LogState, OptionalSend, RaftLogReader, StoredMembership};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::raft::types::{AppRequest, AppResponse, AppTypeConfig};

/// In-memory Raft log backed by a simple `BTreeMap`.
#[derive(Clone, Debug, Default)]
pub struct InMemoryLogStore {
    inner: Arc<Mutex<LogStoreInner>>,
}

#[derive(Debug, Default)]
struct LogStoreInner {
    last_purged_log_id: Option<LogIdOf<AppTypeConfig>>,
    log: BTreeMap<u64, <AppTypeConfig as openraft::RaftTypeConfig>::Entry>,
    committed: Option<LogIdOf<AppTypeConfig>>,
    vote: Option<VoteOf<AppTypeConfig>>,
}

impl LogStoreInner {
    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB,
    ) -> Result<Vec<<AppTypeConfig as openraft::RaftTypeConfig>::Entry>, io::Error>
    where
        RB: RangeBounds<u64> + Clone + Debug,
        <AppTypeConfig as openraft::RaftTypeConfig>::Entry: Clone,
    {
        Ok(self
            .log
            .range(range)
            .map(|(_, entry)| entry.clone())
            .collect())
    }

    async fn get_log_state(&mut self) -> Result<LogState<AppTypeConfig>, io::Error> {
        let last_log_id = self
            .log
            .iter()
            .next_back()
            .map(|(_, entry)| entry.log_id().clone());
        let last_purged = self.last_purged_log_id.clone();
        let last = last_log_id.or(last_purged.clone());
        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    async fn save_committed(
        &mut self,
        committed: Option<LogIdOf<AppTypeConfig>>,
    ) -> Result<(), io::Error> {
        self.committed = committed;
        Ok(())
    }

    async fn read_committed(&mut self) -> Result<Option<LogIdOf<AppTypeConfig>>, io::Error> {
        Ok(self.committed.clone())
    }

    async fn save_vote(&mut self, vote: &VoteOf<AppTypeConfig>) -> Result<(), io::Error> {
        self.vote = Some(vote.clone());
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<VoteOf<AppTypeConfig>>, io::Error> {
        Ok(self.vote.clone())
    }

    async fn append<I>(
        &mut self,
        entries: I,
        callback: IOFlushed<AppTypeConfig>,
    ) -> Result<(), io::Error>
    where
        I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry>,
    {
        for entry in entries {
            self.log.insert(entry.log_id().index(), entry);
        }
        callback.io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let keys = self
            .log
            .range(log_id.index()..)
            .map(|(k, _)| *k)
            .collect::<Vec<_>>();
        for key in keys {
            self.log.remove(&key);
        }
        Ok(())
    }

    async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        if let Some(prev) = &self.last_purged_log_id {
            assert!(prev <= &log_id);
        }
        self.last_purged_log_id = Some(log_id.clone());
        let keys = self
            .log
            .range(..=log_id.index())
            .map(|(k, _)| *k)
            .collect::<Vec<_>>();
        for key in keys {
            self.log.remove(&key);
        }
        Ok(())
    }
}

impl RaftLogReader<AppTypeConfig> for InMemoryLogStore
where
    <AppTypeConfig as openraft::RaftTypeConfig>::Entry: Clone,
{
    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB,
    ) -> Result<Vec<<AppTypeConfig as openraft::RaftTypeConfig>::Entry>, io::Error>
    where
        RB: RangeBounds<u64> + Clone + Debug + OptionalSend,
    {
        let mut inner = self.inner.lock().await;
        inner.try_get_log_entries(range).await
    }

    async fn read_vote(&mut self) -> Result<Option<VoteOf<AppTypeConfig>>, io::Error> {
        let mut inner = self.inner.lock().await;
        inner.read_vote().await
    }
}

impl RaftLogStorage<AppTypeConfig> for InMemoryLogStore
where
    <AppTypeConfig as openraft::RaftTypeConfig>::Entry: Clone,
{
    type LogReader = Self;

    async fn get_log_state(&mut self) -> Result<LogState<AppTypeConfig>, io::Error> {
        let mut inner = self.inner.lock().await;
        inner.get_log_state().await
    }

    async fn save_committed(
        &mut self,
        committed: Option<LogIdOf<AppTypeConfig>>,
    ) -> Result<(), io::Error> {
        let mut inner = self.inner.lock().await;
        inner.save_committed(committed).await
    }

    async fn read_committed(&mut self) -> Result<Option<LogIdOf<AppTypeConfig>>, io::Error> {
        let mut inner = self.inner.lock().await;
        inner.read_committed().await
    }

    async fn save_vote(&mut self, vote: &VoteOf<AppTypeConfig>) -> Result<(), io::Error> {
        let mut inner = self.inner.lock().await;
        inner.save_vote(vote).await
    }

    async fn append<I>(
        &mut self,
        entries: I,
        callback: IOFlushed<AppTypeConfig>,
    ) -> Result<(), io::Error>
    where
        I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        let mut inner = self.inner.lock().await;
        inner.append(entries, callback).await
    }

    async fn truncate(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let mut inner = self.inner.lock().await;
        inner.truncate(log_id).await
    }

    async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let mut inner = self.inner.lock().await;
        inner.purge(log_id).await
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }
}

/// Snapshot blob stored in memory for testing.
#[derive(Debug)]
pub struct StoredSnapshot {
    pub meta: openraft::SnapshotMeta<AppTypeConfig>,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct StateMachineData {
    pub last_applied_log: Option<openraft::LogId<AppTypeConfig>>,
    pub last_membership: StoredMembership<AppTypeConfig>,
    pub data: BTreeMap<String, String>,
}

/// Simple state machine that mirrors the openraft memstore example.
#[derive(Debug, Default)]
pub struct StateMachineStore {
    state_machine: RwLock<StateMachineData>,
    snapshot_idx: AtomicU64,
    current_snapshot: RwLock<Option<StoredSnapshot>>,
}

impl StateMachineStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let sm = self.state_machine.read().await;
        sm.data.get(key).cloned()
    }
}

impl RaftSnapshotBuilder<AppTypeConfig> for Arc<StateMachineStore> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn build_snapshot(&mut self) -> Result<Snapshot<AppTypeConfig>, io::Error> {
        let state_machine = self.state_machine.read().await;
        let data = serde_json::to_vec(&state_machine.data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let last_applied_log = state_machine.last_applied_log;
        let last_membership = state_machine.last_membership.clone();
        let mut current_snapshot = self.current_snapshot.write().await;
        drop(state_machine);

        let snapshot_idx = self.snapshot_idx.fetch_add(1, Ordering::Relaxed) + 1;
        let snapshot_id = if let Some(last) = last_applied_log {
            format!(
                "{}-{}-{snapshot_idx}",
                last.committed_leader_id(),
                last.index()
            )
        } else {
            format!("--{snapshot_idx}")
        };

        let meta = openraft::SnapshotMeta {
            last_log_id: last_applied_log,
            last_membership,
            snapshot_id,
        };

        let snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: data.clone(),
        };
        *current_snapshot = Some(snapshot);

        Ok(Snapshot {
            meta,
            snapshot: Cursor::new(data),
        })
    }
}

impl RaftStateMachine<AppTypeConfig> for Arc<StateMachineStore> {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> Result<
        (
            Option<openraft::LogId<AppTypeConfig>>,
            StoredMembership<AppTypeConfig>,
        ),
        io::Error,
    > {
        let state_machine = self.state_machine.read().await;
        Ok((
            state_machine.last_applied_log,
            state_machine.last_membership.clone(),
        ))
    }

    #[tracing::instrument(level = "trace", skip(self, entries))]
    async fn apply<Strm>(&mut self, mut entries: Strm) -> Result<(), io::Error>
    where
        Strm:
            Stream<Item = Result<EntryResponder<AppTypeConfig>, io::Error>> + Unpin + OptionalSend,
    {
        let mut sm = self.state_machine.write().await;
        while let Some((entry, responder)) = entries.try_next().await? {
            sm.last_applied_log = Some(entry.log_id);
            let response = match entry.payload {
                EntryPayload::Blank => AppResponse { value: None },
                EntryPayload::Normal(ref req) => match req {
                    AppRequest::Set { key, value } => {
                        sm.data.insert(key.clone(), value.clone());
                        AppResponse {
                            value: Some(value.clone()),
                        }
                    }
                    AppRequest::SetMulti { pairs } => {
                        for (key, value) in pairs {
                            sm.data.insert(key.clone(), value.clone());
                        }
                        AppResponse { value: None }
                    }
                },
                EntryPayload::Membership(ref membership) => {
                    sm.last_membership =
                        StoredMembership::new(Some(entry.log_id), membership.clone());
                    AppResponse { value: None }
                }
            };
            if let Some(responder) = responder {
                responder.send(response);
            }
        }
        Ok(())
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<SnapshotDataOf<AppTypeConfig>, io::Error> {
        let mut current_snapshot = self.current_snapshot.write().await;
        Ok(match current_snapshot.take() {
            Some(snapshot) => Cursor::new(snapshot.data),
            None => Cursor::new(Vec::new()),
        })
    }

    async fn install_snapshot(
        &mut self,
        meta: &openraft::SnapshotMeta<AppTypeConfig>,
        snapshot: SnapshotDataOf<AppTypeConfig>,
    ) -> Result<(), io::Error> {
        let new_data: BTreeMap<String, String> = serde_json::from_reader(snapshot)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let mut sm = self.state_machine.write().await;
        sm.data = new_data;
        sm.last_applied_log = meta.last_log_id;
        sm.last_membership = meta.last_membership.clone();
        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot<AppTypeConfig>>, io::Error> {
        let snapshot = self.current_snapshot.read().await;
        Ok(snapshot.as_ref().map(|snap| Snapshot {
            meta: snap.meta.clone(),
            snapshot: Cursor::new(snap.data.clone()),
        }))
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }
}
