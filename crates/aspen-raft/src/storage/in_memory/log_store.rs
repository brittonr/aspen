//! In-memory Raft log store implementation.
//!
//! Provides a fast, non-persistent log store backed by a BTreeMap,
//! suitable for testing and simulations.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io;
use std::ops::RangeBounds;
use std::sync::Arc;

use openraft::LogState;
use openraft::OptionalSend;
use openraft::RaftLogReader;
use openraft::alias::LogIdOf;
use openraft::alias::VoteOf;
use openraft::entry::RaftEntry;
use openraft::storage::IOFlushed;
use openraft::storage::RaftLogStorage;
use tokio::sync::Mutex;

use crate::types::AppTypeConfig;

/// In-memory Raft log backed by a simple `BTreeMap`.
///
/// Provides fast, non-persistent log storage for testing and simulations.
/// All data is lost when the store is dropped.
#[derive(Clone, Debug, Default)]
pub struct InMemoryLogStore {
    /// Shared mutable state protected by an async mutex.
    inner: Arc<Mutex<LogStoreInner>>,
}

/// Internal state for InMemoryLogStore.
#[derive(Debug, Default)]
struct LogStoreInner {
    /// Last log ID that was purged (for snapshot compaction).
    last_purged_log_id: Option<LogIdOf<AppTypeConfig>>,
    /// Map of log index to log entry.
    log: BTreeMap<u64, <AppTypeConfig as openraft::RaftTypeConfig>::Entry>,
    /// Last committed log ID.
    committed: Option<LogIdOf<AppTypeConfig>>,
    /// Current vote state.
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
        Ok(self.log.range(range).map(|(_, entry)| entry.clone()).collect())
    }

    async fn get_log_state(&mut self) -> Result<LogState<AppTypeConfig>, io::Error> {
        let last_log_id = self.log.iter().next_back().map(|(_, entry)| entry.log_id());
        let last_purged = self.last_purged_log_id;
        let last = last_log_id.or(last_purged);
        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    async fn save_committed(&mut self, committed: Option<LogIdOf<AppTypeConfig>>) -> Result<(), io::Error> {
        self.committed = committed;
        Ok(())
    }

    async fn read_committed(&mut self) -> Result<Option<LogIdOf<AppTypeConfig>>, io::Error> {
        Ok(self.committed)
    }

    async fn save_vote(&mut self, vote: &VoteOf<AppTypeConfig>) -> Result<(), io::Error> {
        self.vote = Some(*vote);
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<VoteOf<AppTypeConfig>>, io::Error> {
        Ok(self.vote)
    }

    async fn append<I>(&mut self, entries: I, callback: IOFlushed<AppTypeConfig>) -> Result<(), io::Error>
    where I: IntoIterator<Item = <AppTypeConfig as openraft::RaftTypeConfig>::Entry> {
        for entry in entries {
            self.log.insert(entry.log_id().index(), entry);
        }
        callback.io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        let keys = self.log.range(log_id.index()..).map(|(k, _)| *k).collect::<Vec<_>>();
        for key in keys {
            self.log.remove(&key);
        }
        Ok(())
    }

    async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
        if let Some(prev) = &self.last_purged_log_id {
            assert!(prev <= &log_id);
        }
        self.last_purged_log_id = Some(log_id);
        let keys = self.log.range(..=log_id.index()).map(|(k, _)| *k).collect::<Vec<_>>();
        for key in keys {
            self.log.remove(&key);
        }
        Ok(())
    }
}

impl RaftLogReader<AppTypeConfig> for InMemoryLogStore
where <AppTypeConfig as openraft::RaftTypeConfig>::Entry: Clone
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
where <AppTypeConfig as openraft::RaftTypeConfig>::Entry: Clone
{
    type LogReader = Self;

    async fn get_log_state(&mut self) -> Result<LogState<AppTypeConfig>, io::Error> {
        let mut inner = self.inner.lock().await;
        inner.get_log_state().await
    }

    async fn save_committed(&mut self, committed: Option<LogIdOf<AppTypeConfig>>) -> Result<(), io::Error> {
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

    async fn append<I>(&mut self, entries: I, callback: IOFlushed<AppTypeConfig>) -> Result<(), io::Error>
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

#[cfg(test)]
mod tests {
    use openraft::Vote;

    use super::*;
    use crate::types::NodeId;

    #[test]
    fn test_inmemory_log_store_default() {
        let store = InMemoryLogStore::default();
        // Should be able to clone
        let _cloned = store.clone();
    }

    #[tokio::test]
    async fn test_inmemory_log_store_vote_roundtrip() {
        let mut store = InMemoryLogStore::default();
        let vote = Vote::new(1, NodeId::new(1));

        // Initially no vote
        assert_eq!(store.read_vote().await.unwrap(), None);

        // Save vote
        store.save_vote(&vote).await.unwrap();

        // Read back
        assert_eq!(store.read_vote().await.unwrap(), Some(vote));
    }

    #[tokio::test]
    async fn test_inmemory_log_store_committed_roundtrip() {
        use openraft::testing::log_id;
        let mut store = InMemoryLogStore::default();

        // Initially no committed
        assert_eq!(store.read_committed().await.unwrap(), None);

        // Save committed
        let committed = log_id::<AppTypeConfig>(1, NodeId::new(1), 5);
        store.save_committed(Some(committed)).await.unwrap();

        // Read back
        assert_eq!(store.read_committed().await.unwrap(), Some(committed));

        // Clear committed
        store.save_committed(None).await.unwrap();
        assert_eq!(store.read_committed().await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_inmemory_log_store_get_log_state_empty() {
        let mut store = InMemoryLogStore::default();
        let state = store.get_log_state().await.unwrap();

        assert_eq!(state.last_purged_log_id, None);
        assert_eq!(state.last_log_id, None);
    }

    #[tokio::test]
    async fn test_inmemory_log_store_clone_shares_state() {
        let store = InMemoryLogStore::default();
        let mut store1 = store.clone();
        let mut store2 = store.clone();

        let vote = Vote::new(2, NodeId::new(2));
        store1.save_vote(&vote).await.unwrap();

        // Both clones should see the vote (shared state)
        assert_eq!(store2.read_vote().await.unwrap(), Some(vote));
    }
}
