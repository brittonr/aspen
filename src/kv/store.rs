use std::collections::{BTreeMap, VecDeque};
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
use redb::{Database, ReadableTable, Table, TableDefinition};
use serde::{Deserialize, Serialize};
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
const LEASE_TABLE: TableDefinition<u64, Vec<u8>> = TableDefinition::new("leases");
const LEASE_INDEX_TABLE: TableDefinition<&str, u64> = TableDefinition::new("lease_index");
const WATCH_TABLE: TableDefinition<u64, Vec<u8>> = TableDefinition::new("watches");

type StoResult<T> = Result<T, StorageError<NodeIdOf>>;
const TXN_MAX_OPERATIONS: usize = 64;
const WATCH_QUEUE_LIMIT: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredWatch {
    watch_id: u64,
    key: String,
    prefix: bool,
    events: VecDeque<types::WatchEvent>,
}

impl StoredWatch {
    fn new(watch_id: u64, key: String, prefix: bool) -> Self {
        Self {
            watch_id,
            key,
            prefix,
            events: VecDeque::new(),
        }
    }

    fn matches(&self, candidate: &str) -> bool {
        if self.prefix {
            candidate.starts_with(&self.key)
        } else {
            candidate == self.key
        }
    }

    fn push_event(&mut self, event: types::WatchEvent) {
        if self.events.len() >= WATCH_QUEUE_LIMIT {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    fn take_events(&mut self) -> Vec<types::WatchEvent> {
        self.events.drain(..).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotState {
    kv: BTreeMap<String, Vec<u8>>,
    leases: BTreeMap<u64, types::LeaseRecord>,
    lease_index: BTreeMap<String, u64>,
    watches: BTreeMap<u64, StoredWatch>,
}

impl SnapshotState {
    fn empty() -> Self {
        Self {
            kv: BTreeMap::new(),
            leases: BTreeMap::new(),
            lease_index: BTreeMap::new(),
            watches: BTreeMap::new(),
        }
    }
}

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
    last_applied: Arc<RwLock<Option<LogId<NodeIdOf>>>>,
    last_membership: Arc<RwLock<StoredMembership<NodeIdOf, NodeOf>>>,
}

impl KvStateMachine {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            last_applied: Arc::new(RwLock::new(None)),
            last_membership: Arc::new(RwLock::new(StoredMembership::default())),
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

    fn apply_command(
        &self,
        state: &mut Table<&str, Vec<u8>>,
        leases: &mut Table<u64, Vec<u8>>,
        lease_index: &mut Table<&str, u64>,
        watches: &mut Table<u64, Vec<u8>>,
        revision: u64,
        request: types::Request,
    ) -> StoResult<types::Response> {
        match request {
            types::Request::Set { key, value } => {
                self.apply_set(state, watches, revision, key, value)
            }
            types::Request::Delete { key } => {
                self.apply_delete(state, leases, lease_index, watches, revision, key)
            }
            types::Request::Txn { operations } => {
                self.apply_txn(state, leases, lease_index, watches, revision, operations)
            }
            types::Request::LeaseGrant {
                lease_id,
                ttl_ms,
                timestamp_ms,
            } => self.apply_lease_grant(leases, lease_id, ttl_ms, timestamp_ms),
            types::Request::LeaseAttach { lease_id, key } => {
                self.apply_lease_attach(leases, lease_index, lease_id, key)
            }
            types::Request::LeaseKeepAlive {
                lease_id,
                timestamp_ms,
            } => self.apply_lease_keepalive(leases, lease_id, timestamp_ms),
            types::Request::LeaseRevoke { lease_id } => {
                self.apply_lease_revoke(leases, lease_index, lease_id)
            }
            types::Request::WatchRegister {
                watch_id,
                key,
                prefix,
            } => self.apply_watch_register(watches, watch_id, key, prefix),
            types::Request::WatchCancel { watch_id } => self.apply_watch_cancel(watches, watch_id),
            types::Request::WatchFetch { watch_id } => self.apply_watch_fetch(watches, watch_id),
        }
    }

    fn apply_set(
        &self,
        state: &mut Table<&str, Vec<u8>>,
        watches: &mut Table<u64, Vec<u8>>,
        revision: u64,
        key: String,
        value: String,
    ) -> StoResult<types::Response> {
        let bytes = value.clone().into_bytes();
        let previous = Self::write_value(state, key.as_str(), bytes)?;
        self.enqueue_watch_event(
            watches,
            revision,
            &key,
            Some(value),
            types::WatchEventKind::Put,
        )?;
        Ok(types::Response::Set { previous })
    }

    fn apply_delete(
        &self,
        state: &mut Table<&str, Vec<u8>>,
        leases: &mut Table<u64, Vec<u8>>,
        lease_index: &mut Table<&str, u64>,
        watches: &mut Table<u64, Vec<u8>>,
        revision: u64,
        key: String,
    ) -> StoResult<types::Response> {
        let previous = Self::remove_value(state, key.as_str())?;
        self.detach_key_from_leases(leases, lease_index, key.as_str())?;
        if previous.is_some() {
            self.enqueue_watch_event(watches, revision, &key, None, types::WatchEventKind::Delete)?;
        }
        Ok(types::Response::Delete { previous })
    }

    fn apply_txn(
        &self,
        state: &mut Table<&str, Vec<u8>>,
        leases: &mut Table<u64, Vec<u8>>,
        lease_index: &mut Table<&str, u64>,
        watches: &mut Table<u64, Vec<u8>>,
        revision: u64,
        operations: Vec<types::TxnOp>,
    ) -> StoResult<types::Response> {
        let limited_ops: Vec<_> = operations.into_iter().take(TXN_MAX_OPERATIONS).collect();
        let (results, committed) = self.preview_txn(state, &limited_ops)?;
        if committed {
            self.commit_txn(state, leases, lease_index, watches, revision, &limited_ops)?;
        }
        Ok(types::Response::Txn { committed, results })
    }

    fn preview_txn(
        &self,
        state: &mut Table<&str, Vec<u8>>,
        ops: &[types::TxnOp],
    ) -> StoResult<(Vec<types::TxnOpResult>, bool)> {
        let mut results = Vec::new();
        let mut staged = BTreeMap::new();
        let mut committed = true;

        for op in ops {
            match op {
                types::TxnOp::Assert { key, equals } => {
                    let current = Self::project_value(state, &staged, key)?;
                    let success = &current == equals;
                    results.push(types::TxnOpResult::Assert {
                        key: key.clone(),
                        current: current.clone(),
                        success,
                    });
                    if !success {
                        committed = false;
                        break;
                    }
                }
                types::TxnOp::Set { key, value } => {
                    let previous = Self::project_value(state, &staged, key)?;
                    staged.insert(key.clone(), Some(value.clone()));
                    results.push(types::TxnOpResult::Set {
                        key: key.clone(),
                        previous,
                    });
                }
                types::TxnOp::Delete { key } => {
                    let previous = Self::project_value(state, &staged, key)?;
                    staged.insert(key.clone(), None);
                    results.push(types::TxnOpResult::Delete {
                        key: key.clone(),
                        previous,
                    });
                }
            }
        }

        Ok((results, committed))
    }

    fn commit_txn(
        &self,
        state: &mut Table<&str, Vec<u8>>,
        leases: &mut Table<u64, Vec<u8>>,
        lease_index: &mut Table<&str, u64>,
        watches: &mut Table<u64, Vec<u8>>,
        revision: u64,
        ops: &[types::TxnOp],
    ) -> StoResult<()> {
        for op in ops {
            match op {
                types::TxnOp::Set { key, value } => {
                    let bytes = value.clone().into_bytes();
                    Self::write_value(state, key.as_str(), bytes)?;
                    self.enqueue_watch_event(
                        watches,
                        revision,
                        key,
                        Some(value.clone()),
                        types::WatchEventKind::Put,
                    )?;
                }
                types::TxnOp::Delete { key } => {
                    let previous = Self::remove_value(state, key.as_str())?;
                    self.detach_key_from_leases(leases, lease_index, key.as_str())?;
                    if previous.is_some() {
                        self.enqueue_watch_event(
                            watches,
                            revision,
                            key,
                            None,
                            types::WatchEventKind::Delete,
                        )?;
                    }
                }
                types::TxnOp::Assert { .. } => {}
            }
        }
        Ok(())
    }

    fn apply_lease_grant(
        &self,
        leases: &mut Table<u64, Vec<u8>>,
        lease_id: u64,
        ttl_ms: u64,
        timestamp_ms: u64,
    ) -> StoResult<types::Response> {
        let mut record = types::LeaseRecord {
            lease_id,
            ttl_ms,
            expires_at_ms: timestamp_ms.saturating_add(ttl_ms),
            keys: Vec::new(),
        };
        record.keys.sort();
        let bytes = serde_json::to_vec(&record).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        leases.insert(lease_id, bytes).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        Ok(types::Response::LeaseGranted { lease: record })
    }

    fn apply_lease_attach(
        &self,
        leases: &mut Table<u64, Vec<u8>>,
        lease_index: &mut Table<&str, u64>,
        lease_id: u64,
        key: String,
    ) -> StoResult<types::Response> {
        if let Some(existing) = Self::take_lease_mapping(lease_index, key.as_str())? {
            self.remove_key_from_lease(leases, existing, key.as_str())?;
        }

        let mut record = match Self::read_lease(leases, lease_id)? {
            Some(record) => record,
            None => types::LeaseRecord {
                lease_id,
                ttl_ms: 0,
                expires_at_ms: 0,
                keys: Vec::new(),
            },
        };
        record.keys.push(key.clone());
        record.keys.sort();
        record.keys.dedup();
        Self::write_lease(leases, &record)?;
        lease_index.insert(key.as_str(), lease_id).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        Ok(types::Response::LeaseAttached { lease: record })
    }

    fn apply_lease_keepalive(
        &self,
        leases: &mut Table<u64, Vec<u8>>,
        lease_id: u64,
        timestamp_ms: u64,
    ) -> StoResult<types::Response> {
        let mut record = match Self::read_lease(leases, lease_id)? {
            Some(record) => record,
            None => return Ok(types::Response::LeaseKeepAlive { lease: None }),
        };
        record.expires_at_ms = timestamp_ms.saturating_add(record.ttl_ms);
        Self::write_lease(leases, &record)?;
        Ok(types::Response::LeaseKeepAlive {
            lease: Some(record),
        })
    }

    fn apply_lease_revoke(
        &self,
        leases: &mut Table<u64, Vec<u8>>,
        lease_index: &mut Table<&str, u64>,
        lease_id: u64,
    ) -> StoResult<types::Response> {
        let record = Self::read_lease(leases, lease_id)?;
        if let Some(record) = record {
            let released = record.keys.clone();
            leases.remove(lease_id).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;
            for key in &released {
                lease_index.remove(key.as_str()).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_state_machine(err))
                })?;
            }
            Ok(types::Response::LeaseRevoked {
                lease_id,
                released_keys: released,
            })
        } else {
            Ok(types::Response::LeaseRevoked {
                lease_id,
                released_keys: Vec::new(),
            })
        }
    }

    fn apply_watch_register(
        &self,
        watches: &mut Table<u64, Vec<u8>>,
        watch_id: u64,
        key: String,
        prefix: bool,
    ) -> StoResult<types::Response> {
        let watch = StoredWatch::new(watch_id, key, prefix);
        let bytes = serde_json::to_vec(&watch).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        watches.insert(watch_id, bytes).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        Ok(types::Response::WatchRegistered { watch_id })
    }

    fn apply_watch_cancel(
        &self,
        watches: &mut Table<u64, Vec<u8>>,
        watch_id: u64,
    ) -> StoResult<types::Response> {
        let existed = watches
            .remove(watch_id)
            .map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?
            .is_some();
        Ok(types::Response::WatchCanceled { watch_id, existed })
    }

    fn apply_watch_fetch(
        &self,
        watches: &mut Table<u64, Vec<u8>>,
        watch_id: u64,
    ) -> StoResult<types::Response> {
        let mut watch = match Self::read_watch(watches, watch_id)? {
            Some(watch) => watch,
            None => {
                return Ok(types::Response::WatchEvents {
                    watch_id,
                    events: Vec::new(),
                });
            }
        };
        let events = watch.take_events();
        let bytes = serde_json::to_vec(&watch).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        watches.insert(watch_id, bytes).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        Ok(types::Response::WatchEvents { watch_id, events })
    }

    fn project_value(
        state: &mut Table<&str, Vec<u8>>,
        staged: &BTreeMap<String, Option<String>>,
        key: &str,
    ) -> StoResult<Option<String>> {
        if let Some(value) = staged.get(key) {
            return Ok(value.clone());
        }
        let value = state.get(key).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        Ok(value.map(|v| {
            let data = v.value();
            String::from_utf8_lossy(data.as_slice()).to_string()
        }))
    }

    fn write_value(
        state: &mut Table<&str, Vec<u8>>,
        key: &str,
        value: Vec<u8>,
    ) -> StoResult<Option<String>> {
        let previous = state.insert(key, value).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        Ok(previous.map(|v| {
            let data = v.value();
            String::from_utf8_lossy(data.as_slice()).to_string()
        }))
    }

    fn remove_value(state: &mut Table<&str, Vec<u8>>, key: &str) -> StoResult<Option<String>> {
        let previous = state.remove(key).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        Ok(previous.map(|v| {
            let data = v.value();
            String::from_utf8_lossy(data.as_slice()).to_string()
        }))
    }

    fn enqueue_watch_event(
        &self,
        watches: &mut Table<u64, Vec<u8>>,
        revision: u64,
        key: &str,
        value: Option<String>,
        kind: types::WatchEventKind,
    ) -> StoResult<()> {
        let mut updates = Vec::new();
        let iter = watches.iter().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        for item in iter {
            let (watch_id, raw) = item.map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;
            let bytes = raw.value();
            let mut watch: StoredWatch = serde_json::from_slice(bytes.as_slice()).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;
            if watch.matches(key) {
                let event = types::WatchEvent {
                    revision,
                    key: key.to_string(),
                    kind: kind.clone(),
                    value: value.clone(),
                };
                watch.push_event(event);
                updates.push((watch_id.value(), watch));
            }
        }
        for (watch_id, watch) in updates {
            let bytes = serde_json::to_vec(&watch).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;
            watches.insert(watch_id, bytes).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;
        }
        Ok(())
    }

    fn read_watch(
        watches: &mut Table<u64, Vec<u8>>,
        watch_id: u64,
    ) -> StoResult<Option<StoredWatch>> {
        let value = watches.get(watch_id).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        if let Some(bytes) = value {
            let watch = serde_json::from_slice(bytes.value().as_slice()).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;
            Ok(Some(watch))
        } else {
            Ok(None)
        }
    }

    fn read_lease(
        leases: &mut Table<u64, Vec<u8>>,
        lease_id: u64,
    ) -> StoResult<Option<types::LeaseRecord>> {
        let value = leases.get(lease_id).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        if let Some(bytes) = value {
            let record = serde_json::from_slice(bytes.value().as_slice()).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    fn write_lease(leases: &mut Table<u64, Vec<u8>>, record: &types::LeaseRecord) -> StoResult<()> {
        let bytes = serde_json::to_vec(record).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        leases.insert(record.lease_id, bytes).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        Ok(())
    }

    fn detach_key_from_leases(
        &self,
        leases: &mut Table<u64, Vec<u8>>,
        lease_index: &mut Table<&str, u64>,
        key: &str,
    ) -> StoResult<()> {
        if let Some(lease_id) = Self::take_lease_mapping(lease_index, key)? {
            self.remove_key_from_lease(leases, lease_id, key)?;
        }
        Ok(())
    }

    fn take_lease_mapping(lease_index: &mut Table<&str, u64>, key: &str) -> StoResult<Option<u64>> {
        let removed = lease_index.remove(key).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_state_machine(err))
        })?;
        Ok(removed.map(|v| v.value()))
    }

    fn remove_key_from_lease(
        &self,
        leases: &mut Table<u64, Vec<u8>>,
        lease_id: u64,
        key: &str,
    ) -> StoResult<()> {
        if let Some(mut record) = Self::read_lease(leases, lease_id)? {
            record.keys.retain(|k| k != key);
            record.keys.sort();
            record.keys.dedup();
            Self::write_lease(leases, &record)?;
        }
        Ok(())
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
        tx.open_table(LEASE_TABLE).map_err(|e| Error::Raft {
            message: e.to_string(),
        })?;
        tx.open_table(LEASE_INDEX_TABLE).map_err(|e| Error::Raft {
            message: e.to_string(),
        })?;
        tx.open_table(WATCH_TABLE).map_err(|e| Error::Raft {
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
        let state_table = tx.open_table(STATE_TABLE).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })?;
        let lease_table = tx.open_table(LEASE_TABLE).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })?;
        let lease_index = tx.open_table(LEASE_INDEX_TABLE).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })?;
        let watch_table = tx.open_table(WATCH_TABLE).map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })?;

        let mut snapshot = SnapshotState::empty();
        for item in state_table.iter().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })? {
            let (key, value) = item.map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::read_snapshot(None, err))
            })?;
            snapshot
                .kv
                .insert(key.value().to_string(), value.value().to_vec());
        }
        for item in lease_table.iter().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })? {
            let (key, value) = item.map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::read_snapshot(None, err))
            })?;
            let record: types::LeaseRecord = serde_json::from_slice(value.value().as_slice())
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::read_snapshot(None, err))
                })?;
            snapshot.leases.insert(key.value(), record);
        }
        for item in lease_index.iter().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })? {
            let (key, value) = item.map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::read_snapshot(None, err))
            })?;
            snapshot
                .lease_index
                .insert(key.value().to_string(), value.value());
        }
        for item in watch_table.iter().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::read_snapshot(None, err))
        })? {
            let (key, value) = item.map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::read_snapshot(None, err))
            })?;
            let watch: StoredWatch =
                serde_json::from_slice(value.value().as_slice()).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::read_snapshot(None, err))
                })?;
            snapshot.watches.insert(key.value(), watch);
        }

        let bytes = serde_json::to_vec(&snapshot).map_err(|e| {
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
            let mut leases = tx.open_table(LEASE_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;
            let mut lease_index = tx.open_table(LEASE_INDEX_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;
            let mut watches = tx.open_table(WATCH_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_state_machine(err))
            })?;

            for entry in entries {
                {
                    let mut guard = self.last_applied.write().await;
                    *guard = Some(entry.log_id);
                }
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
                    EntryPayload::Normal(cmd) => self.apply_command(
                        &mut state,
                        &mut leases,
                        &mut lease_index,
                        &mut watches,
                        entry.log_id.index,
                        cmd,
                    )?,
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
        let signature = meta.signature();
        let tx = self.db.begin_write().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
        })?;
        {
            let mut state_table = tx.open_table(STATE_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
            })?;
            let mut leases = tx.open_table(LEASE_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
            })?;
            let mut lease_index = tx.open_table(LEASE_INDEX_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
            })?;
            let mut watches = tx.open_table(WATCH_TABLE).map_err(|e| {
                let err = AnyError::new(&e);
                sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
            })?;

            let snapshot_state: SnapshotState = serde_json::from_slice(raw_snapshot.as_slice())
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;

            let state_keys: Vec<String> = state_table
                .iter()
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?
                .map(|item| {
                    item.map(|(key, _)| key.value().to_string()).map_err(|e| {
                        let err = AnyError::new(&e);
                        sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            for key in state_keys {
                state_table.remove(key.as_str()).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;
            }
            for (key, value) in snapshot_state.kv {
                state_table.insert(key.as_str(), value).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;
            }

            let lease_keys: Vec<u64> = leases
                .iter()
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?
                .map(|item| {
                    item.map(|(key, _)| key.value()).map_err(|e| {
                        let err = AnyError::new(&e);
                        sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            for key in lease_keys {
                leases.remove(key).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;
            }
            for (key, record) in snapshot_state.leases {
                let bytes = serde_json::to_vec(&record).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;
                leases.insert(key, bytes).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;
            }

            let lease_index_keys: Vec<String> = lease_index
                .iter()
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?
                .map(|item| {
                    item.map(|(key, _)| key.value().to_string()).map_err(|e| {
                        let err = AnyError::new(&e);
                        sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            for key in lease_index_keys {
                lease_index.remove(key.as_str()).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;
            }
            for (key, lease_id) in snapshot_state.lease_index {
                lease_index.insert(key.as_str(), lease_id).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;
            }

            let watch_keys: Vec<u64> = watches
                .iter()
                .map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?
                .map(|item| {
                    item.map(|(key, _)| key.value()).map_err(|e| {
                        let err = AnyError::new(&e);
                        sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            for key in watch_keys {
                watches.remove(key).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;
            }
            for (key, watch) in snapshot_state.watches {
                let bytes = serde_json::to_vec(&watch).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;
                watches.insert(key, bytes).map_err(|e| {
                    let err = AnyError::new(&e);
                    sto(StorageIOError::write_snapshot(Some(signature.clone()), err))
                })?;
            }

            *self.last_applied.write().await = meta.last_log_id;
            *self.last_membership.write().await = meta.last_membership.clone();
        }
        tx.commit().map_err(|e| {
            let err = AnyError::new(&e);
            sto(StorageIOError::write_snapshot(Some(signature), err))
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
            last_applied: self.last_applied.clone(),
            last_membership: self.last_membership.clone(),
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

    fn request_entry(request: types::Request, index: u64) -> Entry<TypeConfig> {
        Entry {
            log_id: LogId::new(openraft::CommittedLeaderId::new(1, 1), index),
            payload: EntryPayload::Normal(request),
        }
    }

    #[tokio::test]
    async fn transaction_commits_when_assertions_hold() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("txn.redb");
        let (_store, mut sm) = new_storage(path.to_str().unwrap()).await.unwrap();

        let set_entry = request_entry(
            types::Request::Set {
                key: "alpha".into(),
                value: "v1".into(),
            },
            1,
        );
        sm.apply(vec![set_entry]).await.unwrap();

        let txn_entry = request_entry(
            types::Request::Txn {
                operations: vec![
                    types::TxnOp::Assert {
                        key: "alpha".into(),
                        equals: Some("v1".into()),
                    },
                    types::TxnOp::Set {
                        key: "alpha".into(),
                        value: "v2".into(),
                    },
                ],
            },
            2,
        );
        let responses = sm.apply(vec![txn_entry]).await.unwrap();
        match &responses[0] {
            types::Response::Txn { committed, results } => {
                assert!(*committed);
                assert_eq!(results.len(), 2);
            }
            other => panic!("unexpected response: {:?}", other),
        }
        let persisted = sm.get("alpha").await.unwrap();
        assert_eq!(persisted.as_deref(), Some("v2"));
    }

    #[tokio::test]
    async fn transaction_aborts_on_failed_assert() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("txn_fail.redb");
        let (_store, mut sm) = new_storage(path.to_str().unwrap()).await.unwrap();

        let txn_entry = request_entry(
            types::Request::Txn {
                operations: vec![
                    types::TxnOp::Assert {
                        key: "missing".into(),
                        equals: Some("value".into()),
                    },
                    types::TxnOp::Set {
                        key: "missing".into(),
                        value: "should-not-write".into(),
                    },
                ],
            },
            1,
        );
        let responses = sm.apply(vec![txn_entry]).await.unwrap();
        match &responses[0] {
            types::Response::Txn { committed, .. } => assert!(!committed),
            other => panic!("unexpected response: {:?}", other),
        }
        let persisted = sm.get("missing").await.unwrap();
        assert!(persisted.is_none());
    }

    #[tokio::test]
    async fn lease_lifecycle_tracks_keys() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("lease.redb");
        let (_store, mut sm) = new_storage(path.to_str().unwrap()).await.unwrap();

        let grant = request_entry(
            types::Request::LeaseGrant {
                lease_id: 7,
                ttl_ms: 100,
                timestamp_ms: 10,
            },
            1,
        );
        sm.apply(vec![grant]).await.unwrap();

        let set_entry = request_entry(
            types::Request::Set {
                key: "wat".into(),
                value: "value".into(),
            },
            2,
        );
        sm.apply(vec![set_entry]).await.unwrap();

        let attach = request_entry(
            types::Request::LeaseAttach {
                lease_id: 7,
                key: "wat".into(),
            },
            3,
        );
        let attach_resp = sm.apply(vec![attach]).await.unwrap();
        match &attach_resp[0] {
            types::Response::LeaseAttached { lease } => {
                assert_eq!(lease.lease_id, 7);
                assert_eq!(lease.keys, vec!["wat".to_string()]);
            }
            other => panic!("unexpected response: {:?}", other),
        }

        let revoke = request_entry(types::Request::LeaseRevoke { lease_id: 7 }, 4);
        let revoke_resp = sm.apply(vec![revoke]).await.unwrap();
        match &revoke_resp[0] {
            types::Response::LeaseRevoked {
                lease_id,
                released_keys,
            } => {
                assert_eq!(*lease_id, 7);
                assert_eq!(released_keys, &vec!["wat".to_string()]);
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }

    #[tokio::test]
    async fn watch_registration_collects_events() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("watch.redb");
        let (_store, mut sm) = new_storage(path.to_str().unwrap()).await.unwrap();

        let register = request_entry(
            types::Request::WatchRegister {
                watch_id: 5,
                key: "cfg/".into(),
                prefix: true,
            },
            1,
        );
        sm.apply(vec![register]).await.unwrap();

        let set_entry = request_entry(
            types::Request::Set {
                key: "cfg/node".into(),
                value: "payload".into(),
            },
            2,
        );
        sm.apply(vec![set_entry]).await.unwrap();

        let fetch = request_entry(types::Request::WatchFetch { watch_id: 5 }, 3);
        let responses = sm.apply(vec![fetch]).await.unwrap();
        match &responses[0] {
            types::Response::WatchEvents { watch_id, events } => {
                assert_eq!(*watch_id, 5);
                assert_eq!(events.len(), 1);
                assert_eq!(events[0].key, "cfg/node");
                assert!(matches!(events[0].kind, types::WatchEventKind::Put));
                assert_eq!(events[0].value.as_deref(), Some("payload"));
            }
            other => panic!("unexpected response: {:?}", other),
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
