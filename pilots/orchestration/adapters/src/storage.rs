// SPDX-License-Identifier: AGPL-3.0-or-later
use std::sync::Arc;

use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_codec as codec;
use redb::Database;
use redb::Durability;
use redb::ReadableDatabase;
use redb::ReadableTable;
use redb::TableDefinition;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::content;
use crate::content_id;
use crate::verify_state;
use crate::verify_worker;
use crate::CapabilityRoot;

const SNAPSHOTS: TableDefinition<&str, &[u8]> = TableDefinition::new("orchestrator.snapshots.v1");
const BLOBS: TableDefinition<&str, &[u8]> = TableDefinition::new("orchestrator.content.v1");
const COUNTERS: TableDefinition<&str, u64> = TableDefinition::new("orchestrator.content-counts.v1");

/// The Database owns the capability-opened file and its exclusive process lock.
/// Every adapter for one process shares this Arc, never reopens by ambient path.
pub struct Disk {
    db: Arc<Database>,
}
impl Disk {
    pub fn open(root: &CapabilityRoot) -> Result<Arc<Self>, Error> {
        let file = root.file("orchestration.redb", true)?;
        let db = Database::builder().set_cache_size(16 * 1024 * 1024).create_file(file).map_err(crate::invalid)?;
        let this = Arc::new(Self { db: Arc::new(db) });
        let mut tx = this.db.begin_write().map_err(crate::invalid)?;
        tx.set_durability(Durability::Immediate).map_err(crate::invalid)?;
        {
            tx.open_table(SNAPSHOTS).map_err(crate::invalid)?;
            tx.open_table(BLOBS).map_err(crate::invalid)?;
            tx.open_table(COUNTERS).map_err(crate::invalid)?;
        }
        tx.commit().map_err(|_| Error::Unknown)?;
        Ok(this)
    }
    fn initialize<T: Serialize + DeserializeOwned>(&self, schema: &str, value: &T) -> Result<(), Error> {
        let encoded = codec::encode(schema, value).map_err(crate::invalid)?;
        let projection_key = format!("{schema}.projection");
        let mut tx = self.db.begin_write().map_err(crate::invalid)?;
        tx.set_durability(Durability::Immediate).map_err(crate::invalid)?;
        {
            let mut table = tx.open_table(SNAPSHOTS).map_err(crate::invalid)?;
            let opposite = match schema {
                "controller-state.v1" => "worker-state.v1",
                "worker-state.v1" => "controller-state.v1",
                _ => return Err(Error::Unsupported),
            };
            if table.get(opposite).map_err(crate::invalid)?.is_some() {
                return Err(Error::Invalid("database role already bound".into()));
            }
            let a = table.get(schema).map_err(crate::invalid)?.map(|g| g.value().to_vec());
            let b = table.get(projection_key.as_str()).map_err(crate::invalid)?.map(|g| g.value().to_vec());
            match (a, b) {
                (None, None) => {
                    table.insert(schema, encoded.canonical.as_slice()).map_err(crate::invalid)?;
                    table.insert(projection_key.as_str(), encoded.projection.as_slice()).map_err(crate::invalid)?;
                }
                (Some(a), Some(b)) => {
                    let _: T = codec::decode(schema, &a, &b).map_err(crate::invalid)?;
                }
                _ => return Err(Error::Invalid("incomplete durable snapshot/cache pair".into())),
            }
        }
        tx.commit().map_err(|_| Error::Unknown)
    }
    fn load<T: Serialize + DeserializeOwned>(&self, schema: &str) -> Result<T, Error> {
        let tx = self.db.begin_read().map_err(crate::invalid)?;
        let table = tx.open_table(SNAPSHOTS).map_err(crate::invalid)?;
        let projection_key = format!("{schema}.projection");
        let a = table
            .get(schema)
            .map_err(crate::invalid)?
            .ok_or(Error::Invalid("missing canonical snapshot".into()))?;
        let b = table
            .get(projection_key.as_str())
            .map_err(crate::invalid)?
            .ok_or(Error::Invalid("missing reconstruction projection".into()))?;
        codec::decode(schema, a.value(), b.value()).map_err(crate::invalid)
    }
    fn cas<T: Serialize>(&self, schema: &str, expected: &T, next: &T) -> Result<Commit, Error> {
        let old = codec::encode(schema, expected).map_err(crate::invalid)?;
        let new = codec::encode(schema, next).map_err(crate::invalid)?;
        let projection_key = format!("{schema}.projection");
        let mut tx = self.db.begin_write().map_err(crate::invalid)?;
        tx.set_durability(Durability::Immediate).map_err(crate::invalid)?;
        {
            let mut table = tx.open_table(SNAPSHOTS).map_err(crate::invalid)?;
            let matches = {
                let a = table.get(schema).map_err(crate::invalid)?;
                let b = table.get(projection_key.as_str()).map_err(crate::invalid)?;
                a.as_ref().is_some_and(|g| g.value() == old.canonical.as_slice())
                    && b.as_ref().is_some_and(|g| g.value() == old.projection.as_slice())
            };
            if !matches {
                return Ok(Commit::Conflict);
            }
            table.insert(schema, new.canonical.as_slice()).map_err(crate::invalid)?;
            table.insert(projection_key.as_str(), new.projection.as_slice()).map_err(crate::invalid)?;
        }
        // Once commit is attempted, an I/O error cannot prove non-application.
        Ok(if tx.commit().is_ok() {
            Commit::Applied
        } else {
            Commit::Unknown
        })
    }
}

pub struct RedbControllerStore {
    disk: Arc<Disk>,
}
impl RedbControllerStore {
    pub fn open(disk: Arc<Disk>, initial: State) -> Result<Arc<Self>, Error> {
        verify_state(&initial)?;
        disk.initialize("controller-state.v1", &initial)?;
        let this = Arc::new(Self { disk });
        let old = this.load()?;
        if old.cluster != initial.cluster
            || old.generation != initial.generation
            || old.policy_ref != initial.policy_ref
            || old.freshness_ticks != initial.freshness_ticks
        {
            return Err(Error::Invalid("controller namespace/profile changed".into()));
        }
        Ok(this)
    }
}
impl StateStore for RedbControllerStore {
    fn load(&self) -> Result<State, Error> {
        let state = self.disk.load("controller-state.v1")?;
        verify_state(&state)?;
        Ok(state)
    }
    fn compare_commit(&self, old: &State, next: &State) -> Result<Commit, Error> {
        verify_state(old)?;
        verify_state(next)?;
        if old.cluster != next.cluster
            || old.generation != next.generation
            || old.policy_ref != next.policy_ref
            || old.revision.checked_add(1) != Some(next.revision)
        {
            return Err(Error::Invalid("invalid controller commit cohort/revision".into()));
        }
        self.disk.cas("controller-state.v1", old, next)
    }
}
pub struct RedbWorkerJournal {
    disk: Arc<Disk>,
}
impl RedbWorkerJournal {
    pub fn open(disk: Arc<Disk>, initial: WorkerState) -> Result<Arc<Self>, Error> {
        verify_worker(&initial)?;
        disk.initialize("worker-state.v1", &initial)?;
        let this = Arc::new(Self { disk });
        let old = this.load()?;
        if old.cluster != initial.cluster
            || old.node != initial.node
            || old.generation != initial.generation
            || old.policy_ref != initial.policy_ref
            || old.capacity != initial.capacity
        {
            return Err(Error::Invalid("worker namespace/profile changed".into()));
        }
        Ok(this)
    }
}
impl WorkerJournal for RedbWorkerJournal {
    fn load(&self) -> Result<WorkerState, Error> {
        let state = self.disk.load("worker-state.v1")?;
        verify_worker(&state)?;
        Ok(state)
    }
    fn compare_commit(&self, old: &WorkerState, next: &WorkerState) -> Result<Commit, Error> {
        verify_worker(old)?;
        verify_worker(next)?;
        if old.cluster != next.cluster
            || old.node != next.node
            || old.generation != next.generation
            || old.policy_ref != next.policy_ref
            || old.capacity != next.capacity
            || old.revision.checked_add(1) != Some(next.revision)
        {
            return Err(Error::Invalid("invalid worker commit cohort/revision".into()));
        }
        self.disk.cas("worker-state.v1", old, next)
    }
}

pub struct RedbContent {
    disk: Arc<Disk>,
}
impl RedbContent {
    pub fn new(disk: Arc<Disk>) -> Arc<Self> {
        Arc::new(Self { disk })
    }
}
impl ContentPort for RedbContent {
    fn get(&self, id: Id) -> Result<Content, Error> {
        let key = id.hex();
        let canonical_key = format!("{key}.manifest");
        let projection_key = format!("{key}.projection");
        let tx = self.disk.db.begin_read().map_err(crate::invalid)?;
        let table = tx.open_table(BLOBS).map_err(crate::invalid)?;
        let raw = table.get(key.as_str()).map_err(crate::invalid)?.ok_or(Error::Unavailable)?;
        if raw.value().len() > MAX_CONTENT_BYTES {
            return Err(Error::Bound);
        }
        let a = table
            .get(canonical_key.as_str())
            .map_err(crate::invalid)?
            .ok_or(Error::Invalid("missing content manifest".into()))?;
        let b = table
            .get(projection_key.as_str())
            .map_err(crate::invalid)?
            .ok_or(Error::Invalid("missing content projection".into()))?;
        let manifest = codec::decode("content-manifest.v1", a.value(), b.value()).map_err(crate::invalid)?;
        let content = Content {
            manifest,
            bytes: raw.value().to_vec(),
        };
        if content_id(&content)? != id {
            return Err(Error::Invalid("content substitution".into()));
        }
        Ok(content)
    }
    fn put(&self, c: &Content) -> Result<Id, Error> {
        let id = content_id(c)?;
        let key = id.hex();
        let akey = format!("{key}.manifest");
        let bkey = format!("{key}.projection");
        let encoded = codec::encode("content-manifest.v1", &c.manifest).map_err(crate::invalid)?;
        let mut tx = self.disk.db.begin_write().map_err(crate::invalid)?;
        tx.set_durability(Durability::Immediate).map_err(crate::invalid)?;
        {
            let mut table = tx.open_table(BLOBS).map_err(crate::invalid)?;
            let old = table.get(key.as_str()).map_err(crate::invalid)?.map(|g| g.value().to_vec());
            if let Some(old) = old {
                let a = table.get(akey.as_str()).map_err(crate::invalid)?;
                let b = table.get(bkey.as_str()).map_err(crate::invalid)?;
                if old != c.bytes
                    || !a.is_some_and(|v| v.value() == encoded.canonical.as_slice())
                    || !b.is_some_and(|v| v.value() == encoded.projection.as_slice())
                {
                    return Err(Error::Invalid("content collision/corruption".into()));
                }
                return Ok(id);
            }
            let mut counters = tx.open_table(COUNTERS).map_err(crate::invalid)?;
            let count = counters.get("objects").map_err(crate::invalid)?.map_or(0, |v| v.value());
            let bytes = counters.get("bytes").map_err(crate::invalid)?.map_or(0, |v| v.value());
            let next_bytes = bytes.checked_add(c.bytes.len() as u64).ok_or(Error::Bound)?;
            if count >= MAX_CONTENT_OBJECTS || next_bytes > MAX_CONTENT_TOTAL_BYTES {
                return Err(Error::Bound);
            }
            table.insert(key.as_str(), c.bytes.as_slice()).map_err(crate::invalid)?;
            table.insert(akey.as_str(), encoded.canonical.as_slice()).map_err(crate::invalid)?;
            table.insert(bkey.as_str(), encoded.projection.as_slice()).map_err(crate::invalid)?;
            counters.insert("objects", count + 1).map_err(crate::invalid)?;
            counters.insert("bytes", next_bytes).map_err(crate::invalid)?;
        }
        tx.commit().map_err(|_| Error::Unknown)?;
        Ok(id)
    }
    fn make_data(&self, bytes: Vec<u8>) -> Result<Content, Error> {
        content(ContentKind::Data, bytes)
    }
}
