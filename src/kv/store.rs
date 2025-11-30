use std::collections::BTreeMap;
use std::io::Cursor;
use std::ops::RangeBounds;
use std::sync::Arc;

use openraft::AnyError;
use openraft::storage::{
    LogFlushed, RaftLogReader, RaftLogStorage, RaftSnapshotBuilder, RaftStateMachine,
};
use openraft::{
    Entry, EntryPayload, LogId, LogState, OptionalSend, RaftTypeConfig, Snapshot, SnapshotMeta,
    StorageError, StorageIOError, StoredMembership, Vote,
};
use redb::{Database, ReadableTable, TableDefinition};
use tokio::sync::RwLock;

use crate::kv::error::Error;
use crate::kv::types::{self, TypeConfig};

type NodeIdOf = <TypeConfig as RaftTypeConfig>::NodeId;
type NodeOf = <TypeConfig as RaftTypeConfig>::Node;

fn sto(err: StorageIOError<NodeIdOf>) -> StorageError<NodeIdOf> {
    StorageError::from(err)
}

const LOG_TABLE: TableDefinition<u64, Vec<u8>> = TableDefinition::new("raft_log");
const META_TABLE: TableDefinition<&str, Vec<u8>> = TableDefinition::new("raft_meta");
const STATE_TABLE: TableDefinition<&str, Vec<u8>> = TableDefinition::new("state_machine");

type StoResult<T> = Result<T, StorageError<NodeIdOf>>;

/// Log store backed by redb.
#[derive(Clone)]
pub struct KvLogStore {
    db: Arc<Database>,
}

impl KvLogStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    fn read_meta<T: serde::de::DeserializeOwned>(&self, key: &str) -> StoResult<Option<T>> {
        let tx = self.db.begin_read().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read(err))
        })?;
        let table = tx.open_table(META_TABLE).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read(err))
        })?;
        let val = table.get(key).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read(err))
        })?;
        match val {
            Some(bytes) => {
                let data = bytes.value();
                let des = serde_json::from_slice(data.as_slice()).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::read(err))
                })?;
                Ok(Some(des))
            }
            None => Ok(None),
        }
    }

    fn write_meta<T: serde::Serialize>(&self, key: &str, value: &T) -> StoResult<()> {
        let tx = self.db.begin_write().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write(err))
        })?;
        {
            let mut table = tx.open_table(META_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write(err))
            })?;
            let bytes = serde_json::to_vec(value).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write(err))
            })?;
            table.insert(key, bytes).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write(err))
            })?;
        }
        tx.commit().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write(err))
        })
    }
}

/// State machine that persists key/value pairs in redb.
pub struct KvStateMachine {
    db: Arc<Database>,
    last_applied: RwLock<Option<LogId<NodeIdOf>>>,
    last_membership: RwLock<StoredMembership<NodeIdOf, NodeOf>>,
}

impl KvStateMachine {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            last_applied: RwLock::new(None),
            last_membership: RwLock::new(StoredMembership::default()),
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, Error> {
        let tx = self.db.begin_read().map_err(|e| Error::Raft {
            message: e.to_string(),
        })?;
        let table = tx.open_table(STATE_TABLE).map_err(|e| Error::Raft {
            message: e.to_string(),
        })?;
        let value = table
            .get(key)
            .map_err(|e| Error::Raft {
                message: e.to_string(),
            })?
            .map(|v| {
                let data = v.value();
                String::from_utf8_lossy(data.as_slice()).to_string()
            });
        Ok(value)
    }

    async fn read_applied_state(
        &self,
    ) -> StoResult<(Option<LogId<NodeIdOf>>, StoredMembership<NodeIdOf, NodeOf>)> {
        let tx = self.db.begin_read().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_state_machine(err))
        })?;
        let table = tx.open_table(META_TABLE).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_state_machine(err))
        })?;

        let last_applied = table
            .get("last_applied")
            .map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::read_state_machine(err))
            })?
            .map(|bytes| {
                let data = bytes.value();
                serde_json::from_slice(data.as_slice()).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::read_state_machine(err))
                })
            })
            .transpose()?;

        let membership = table
            .get("last_membership")
            .map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::read_state_machine(err))
            })?
            .map(|bytes| {
                let data = bytes.value();
                serde_json::from_slice(data.as_slice()).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::read_state_machine(err))
                })
            })
            .transpose()?
            .unwrap_or_else(StoredMembership::default);

        Ok((last_applied, membership))
    }
}

pub async fn new_storage(db_path: &str) -> Result<(KvLogStore, KvStateMachine), Error> {
    let db = Database::create(db_path).map_err(|e| Error::Raft {
        message: e.to_string(),
    })?;
    {
        let tx = db.begin_write().map_err(|e| Error::Raft {
            message: e.to_string(),
        })?;
        tx.open_table(LOG_TABLE).map_err(|e| Error::Raft {
            message: e.to_string(),
        })?;
        tx.open_table(META_TABLE).map_err(|e| Error::Raft {
            message: e.to_string(),
        })?;
        tx.open_table(STATE_TABLE).map_err(|e| Error::Raft {
            message: e.to_string(),
        })?;
        tx.commit().map_err(|e| Error::Raft {
            message: e.to_string(),
        })?;
    }
    let db = Arc::new(db);
    Ok((KvLogStore::new(db.clone()), KvStateMachine::new(db)))
}

impl RaftLogReader<TypeConfig> for KvLogStore {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + std::fmt::Debug + OptionalSend>(
        &mut self,
        range: RB,
    ) -> StoResult<Vec<Entry<TypeConfig>>> {
        let tx = self.db.begin_read().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_logs(err))
        })?;
        let table = tx.open_table(LOG_TABLE).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_logs(err))
        })?;
        let mut entries = Vec::new();
        for item in table.range(range).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_logs(err))
        })? {
            let (_, value) = item.map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::read_logs(err))
            })?;
            let bytes = value.value();
            let entry: Entry<TypeConfig> =
                serde_json::from_slice(bytes.as_slice()).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::read_logs(err))
                })?;
            entries.push(entry);
        }
        Ok(entries)
    }
}

impl RaftLogStorage<TypeConfig> for KvLogStore {
    type LogReader = Self;

    async fn get_log_state(&mut self) -> StoResult<LogState<TypeConfig>> {
        let tx = self.db.begin_read().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_logs(err))
        })?;
        let table = tx.open_table(LOG_TABLE).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_logs(err))
        })?;

        let last_log_id = table
            .last()
            .map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::read_logs(err))
            })?
            .map(|(_, value)| {
                serde_json::from_slice::<Entry<TypeConfig>>(value.value().as_slice())
                    .map(|entry| entry.log_id)
                    .map_err(|e| {
                        let err = AnyError::new(&e);
                        sto(StorageIOError::read_logs(err))
                    })
            })
            .transpose()?;

        let last_purged_log_id = self.read_meta("last_purged_log_id")?;

        Ok(LogState {
            last_purged_log_id,
            last_log_id,
        })
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn save_vote(&mut self, vote: &Vote<NodeIdOf>) -> StoResult<()> {
        self.write_meta("vote", vote)
    }

    async fn read_vote(&mut self) -> StoResult<Option<Vote<NodeIdOf>>> {
        self.read_meta("vote")
    }

    async fn append<I>(&mut self, entries: I, callback: LogFlushed<TypeConfig>) -> StoResult<()>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        let tx = self.db.begin_write().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_logs(err))
        })?;
        {
            let mut table = tx.open_table(LOG_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_logs(err))
            })?;
            for entry in entries {
                let bytes = serde_json::to_vec(&entry).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_logs(err))
                })?;
                table.insert(entry.log_id.index, bytes).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_logs(err))
                })?;
            }
        }
        tx.commit().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_logs(err))
        })?;
        callback.log_io_completed(Ok(()));
        Ok(())
    }

    async fn truncate(&mut self, log_id: LogId<NodeIdOf>) -> StoResult<()> {
        let tx = self.db.begin_write().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_logs(err))
        })?;
        {
            let mut table = tx.open_table(LOG_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_logs(err))
            })?;
            let to_remove: Vec<_> = table
                .range(log_id.index..)
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_logs(err))
                })?
                .map(|item| item.map(|(key, _)| key.value()))
                .collect::<Result<_, _>>()
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_logs(err))
                })?;
            for key in to_remove {
                table.remove(key).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_logs(err))
                })?;
            }
        }
        tx.commit().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_logs(err))
        })
    }

    async fn purge(&mut self, log_id: LogId<NodeIdOf>) -> StoResult<()> {
        self.write_meta("last_purged_log_id", &log_id)?;
        let tx = self.db.begin_write().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_logs(err))
        })?;
        {
            let mut table = tx.open_table(LOG_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_logs(err))
            })?;
            let to_remove: Vec<_> = table
                .range(..=log_id.index)
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_logs(err))
                })?
                .map(|item| item.map(|(key, _)| key.value()))
                .collect::<Result<_, _>>()
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_logs(err))
                })?;
            for key in to_remove {
                table.remove(key).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_logs(err))
                })?;
            }
        }
        tx.commit().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_logs(err))
        })
    }
}

impl RaftSnapshotBuilder<TypeConfig> for KvStateMachine {
    async fn build_snapshot(&mut self) -> StoResult<Snapshot<TypeConfig>> {
        let (last_applied, membership) = self.read_applied_state().await?;
        let tx = self.db.begin_read().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })?;
        let table = tx.open_table(STATE_TABLE).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })?;
        let mut sm_data = BTreeMap::new();
        for item in table.iter().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })? {
            let (key, value) = item.map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::read_snapshot(None, err))
            })?;
            sm_data.insert(key.value().to_string(), value.value().to_vec());
        }
        let bytes = serde_json::to_vec(&sm_data).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })?;
        let meta = SnapshotMeta {
            last_log_id: last_applied,
            last_membership: membership,
            snapshot_id: format!("kv-{}", uuid::Uuid::new_v4()),
        };
        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(bytes)),
        })
    }
}

impl RaftStateMachine<TypeConfig> for KvStateMachine {
    type SnapshotBuilder = Self;

    async fn applied_state(
        &mut self,
    ) -> StoResult<(Option<LogId<NodeIdOf>>, StoredMembership<NodeIdOf, NodeOf>)> {
        self.read_applied_state().await
    }

    async fn apply<I>(&mut self, entries: I) -> StoResult<Vec<types::Response>>
    where
        I: IntoIterator<Item = Entry<TypeConfig>> + OptionalSend,
        I::IntoIter: OptionalSend,
    {
        let mut responses = Vec::new();
        let tx = self.db.begin_write().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        {
            let mut state = tx.open_table(STATE_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;
            let mut meta = tx.open_table(META_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;

            for entry in entries {
                *self.last_applied.write().await = Some(entry.log_id);
                meta.insert(
                    "last_applied",
                    serde_json::to_vec(&entry.log_id).map_err(|e| {
                        let err = AnyError::new(&e);
                        sto(StorageIOError::write_state_machine(err))
                    })?,
                )
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_state_machine(err))
                })?;

                let response = match entry.payload {
                    EntryPayload::Blank => types::Response::Get { value: None },
                    EntryPayload::Normal(cmd) => match cmd {
                        types::Request::Set { key, value } => {
                            let bytes = value.into_bytes();
                            let previous = state
                                .insert(key.as_str(), bytes)
                                .map_err(|e| {
                                    let err = AnyError::new(&e);
                                    sto(StorageIOError::write_state_machine(err))
                                })?
                                .map(|v| {
                                    let data = v.value();
                                    String::from_utf8_lossy(data.as_slice()).to_string()
                                });
                            types::Response::Set { previous }
                        }
                        types::Request::Delete { key } => {
                            let previous = state
                                .remove(key.as_str())
                                .map_err(|e| {
                                    let err = AnyError::new(&e);
                                    sto(StorageIOError::write_state_machine(err))
                                })?
                                .map(|v| {
                                    let data = v.value();
                                    String::from_utf8_lossy(data.as_slice()).to_string()
                                });
                            types::Response::Delete { previous }
                        }
                    },
                    EntryPayload::Membership(m) => {
                        let stored = StoredMembership::new(Some(entry.log_id), m);
                        *self.last_membership.write().await = stored.clone();
                        meta.insert(
                            "last_membership",
                            serde_json::to_vec(&stored).map_err(|e| {
                                let err = AnyError::new(&e);
                                sto(StorageIOError::write_state_machine(err))
                            })?,
                        )
                        .map_err(|e| {
                            let err = AnyError::new(&e);
                            sto(StorageIOError::write_state_machine(err))
                        })?;
                        types::Response::Get { value: None }
                    }
                };
                responses.push(response);
            }
        }
        tx.commit().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        Ok(responses)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }

    async fn begin_receiving_snapshot(&mut self) -> StoResult<Box<Cursor<Vec<u8>>>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<NodeIdOf, NodeOf>,
        snapshot: Box<Cursor<Vec<u8>>>,
    ) -> StoResult<()> {
        let raw_snapshot = snapshot.into_inner();
        let tx = self.db.begin_write().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_snapshot(Some(meta.signature()), err))
        })?;
        {
            let mut table = tx.open_table(STATE_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_snapshot(Some(meta.signature()), err))
            })?;
            let keys: Vec<String> = table
                .iter()
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(meta.signature()), err))
                })?
                .map(|item| {
                    item.map(|(key, _)| key.value().to_string()).map_err(|e| {
                        let err = AnyError::new(&e);
                        sto(StorageIOError::write_snapshot(Some(meta.signature()), err))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            for key in keys {
                table.remove(key.as_str()).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(meta.signature()), err))
                })?;
            }

            let new_data: BTreeMap<String, Vec<u8>> =
                serde_json::from_slice(raw_snapshot.as_slice()).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(meta.signature()), err))
                })?;
            for (key, value) in new_data {
                table.insert(key.as_str(), value).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(meta.signature()), err))
                })?;
            }

            *self.last_applied.write().await = meta.last_log_id;
            *self.last_membership.write().await = meta.last_membership.clone();
        }
        tx.commit().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_snapshot(Some(meta.signature()), err))
        })?;
        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> StoResult<Option<Snapshot<TypeConfig>>> {
        Ok(Some(self.build_snapshot().await?))
    }
}

impl Clone for KvStateMachine {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            last_applied: RwLock::new(self.last_applied.blocking_read().clone()),
            last_membership: RwLock::new(self.last_membership.blocking_read().clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::strategy::Strategy;
    use std::collections::{BTreeMap, BTreeSet};
    use tempfile::TempDir;
    use tokio_test::block_on;

    #[tokio::test]
    async fn state_machine_set_and_get() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("db.redb");
        let (store, machine) = new_storage(path.to_str().unwrap()).await.unwrap();
        let mut machine = machine;
        let entry = Entry {
            log_id: LogId::new(openraft::CommittedLeaderId::new(1, 1), 1),
            payload: EntryPayload::Normal(types::Request::Set {
                key: "a".into(),
                value: "value".into(),
            }),
        };

        let _ = store;
        machine.apply(vec![entry]).await.unwrap();
        let val = machine.get("a").await.unwrap();
        assert_eq!(val, Some("value".into()));
    }

    #[derive(Clone, Debug)]
    enum Operation {
        Set { key: String, value: String },
        Delete { key: String },
    }

    impl Operation {
        fn to_request(&self) -> types::Request {
            match self {
                Operation::Set { key, value } => types::Request::Set {
                    key: key.clone(),
                    value: value.clone(),
                },
                Operation::Delete { key } => types::Request::Delete { key: key.clone() },
            }
        }

        fn key(&self) -> &String {
            match self {
                Operation::Set { key, .. } | Operation::Delete { key } => key,
            }
        }
    }

    fn key_strategy() -> impl Strategy<Value = String> {
        proptest::collection::vec(proptest::char::range('a', 'z'), 1..=4)
            .prop_map(|chars| chars.into_iter().collect())
    }

    fn value_strategy() -> impl Strategy<Value = String> {
        proptest::collection::vec(proptest::char::range('a', 'z'), 1..=7)
            .prop_map(|chars| chars.into_iter().collect())
    }

    fn op_strategy() -> impl Strategy<Value = Operation> {
        prop_oneof![
            (key_strategy(), value_strategy())
                .prop_map(|(key, value)| Operation::Set { key, value }),
            key_strategy().prop_map(|key| Operation::Delete { key }),
        ]
    }

    fn entry_for(op: &Operation, index: u64) -> Entry<TypeConfig> {
        Entry {
            log_id: LogId::new(openraft::CommittedLeaderId::new(1, 1), index),
            payload: EntryPayload::Normal(op.to_request()),
        }
    }

    proptest! {
        #[test]
        fn proptest_state_machine_responses_track_model(ops in proptest::collection::vec(op_strategy(), 1..32)) {
            block_on(async move {
                let dir = TempDir::new().unwrap();
                let path = dir.path().join("db.redb");
                let (_store, mut sm) = new_storage(path.to_str().unwrap()).await.unwrap();

                let entries: Vec<_> = ops.iter().enumerate().map(|(idx, op)| entry_for(op, idx as u64 + 1)).collect();
                let responses = sm.apply(entries).await.unwrap();
                let mut model = BTreeMap::new();

                for (op, response) in ops.iter().zip(responses.iter()) {
                    match op {
                        Operation::Set { key, value } => {
                            let expected_prev = model.insert(key.clone(), value.clone());
                            match response {
                                types::Response::Set { previous } => {
                                    assert_eq!(previous.as_ref(), expected_prev.as_ref());
                                }
                                other => panic!("expected set response, got {:?}", other),
                            }
                        }
                        Operation::Delete { key } => {
                            let expected_prev = model.remove(key);
                            match response {
                                types::Response::Delete { previous } => {
                                    assert_eq!(previous.as_ref(), expected_prev.as_ref());
                                }
                                other => panic!("expected delete response, got {:?}", other),
                            }
                        }
                    }
                }

                let mut keys = BTreeSet::new();
                for op in &ops {
                    keys.insert(op.key().clone());
                }

                for key in keys {
                    let stored = sm.get(&key).await.unwrap();
                    let expected = model.get(&key).cloned();
                    assert_eq!(stored, expected, "state mismatch for key {}", key);
                }
            });
        }
    }

    proptest! {
        #[test]
        fn proptest_idempotent_sets(key in key_strategy(), value in value_strategy()) {
            block_on(async move {
                let dir = TempDir::new().unwrap();
                let path = dir.path().join("db.redb");
                let (_store, mut sm) = new_storage(path.to_str().unwrap()).await.unwrap();

                let set_op = Operation::Set { key: key.clone(), value: value.clone() };
                let entries = vec![
                    entry_for(&set_op, 1),
                    entry_for(&set_op, 2),
                ];

                let responses = sm.apply(entries).await.unwrap();
                assert_eq!(responses.len(), 2);

                match &responses[0] {
                    types::Response::Set { previous } => assert!(previous.is_none()),
                    other => panic!("expected set response, got {:?}", other),
                }

                match &responses[1] {
                    types::Response::Set { previous } => assert_eq!(previous.as_deref(), Some(value.as_str())),
                    other => panic!("expected set response, got {:?}", other),
                }

                let stored = sm.get(&key).await.unwrap();
                assert_eq!(stored.as_deref(), Some(value.as_str()));
            });
        }
    }

    proptest! {
        #[test]
        fn proptest_delete_semantics(key in key_strategy(), value in value_strategy()) {
            block_on(async move {
                let dir = TempDir::new().unwrap();
                let path = dir.path().join("db.redb");
                let (_store, mut sm) = new_storage(path.to_str().unwrap()).await.unwrap();

                let set_op = Operation::Set { key: key.clone(), value: value.clone() };
                let delete_op = Operation::Delete { key: key.clone() };
                let entries = vec![
                    entry_for(&set_op, 1),
                    entry_for(&delete_op, 2),
                    entry_for(&delete_op, 3),
                ];

                let responses = sm.apply(entries).await.unwrap();
                assert_eq!(responses.len(), 3);

                match &responses[1] {
                    types::Response::Delete { previous } => assert_eq!(previous.as_deref(), Some(value.as_str())),
                    other => panic!("expected delete response, got {:?}", other),
                }

                match &responses[2] {
                    types::Response::Delete { previous } => assert!(previous.is_none()),
                    other => panic!("expected delete response, got {:?}", other),
                }

                let stored = sm.get(&key).await.unwrap();
                assert_eq!(stored, None);
            });
        }
    }
}
