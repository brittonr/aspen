//! Storage adapters for Aspen.
//!
//! # Responsibilities
//! - Provide explicit interfaces for Raft log/state-machine implementations
//!   so we can swap between the production `redb` backend and deterministic
//!   in-memory engines under `madsim`.
//! - Surface property-test seam points in `storage::log` and
//!   `storage::state_machine` so future contributors can drop in deterministic
//!   harnesses quickly.
//! - Keep implementations statically allocated where possible per Tiger Style.
//!
//! # Current layout
//! - [`log`] exposes the log-IO contract plus an in-memory implementation that
//!   already enforces ordering/truncation invariants.
//! - [`state_machine`] handles state-application snapshots and similarly ships
//!   with an in-memory engine.
//! - [`StorageSurface`] pipes both handles together so the Raft actor can depend
//!   on a single struct while still swapping backends independently.

use std::sync::Arc;

pub mod log;
pub mod state_machine;

use log::{InMemoryLog, LogHandle, RedbLog};
use state_machine::{InMemoryStateMachine, RedbStateMachine, StateMachineHandle};
/// Storage surface that bundles both log + state machine handles.
pub struct StorageSurface {
    log: Arc<dyn LogHandle>,
    state_machine: Arc<dyn StateMachineHandle>,
}

impl std::fmt::Debug for StorageSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageSurface")
            .field("log", &"dyn LogHandle")
            .field("state_machine", &"dyn StateMachineHandle")
            .finish()
    }
}

impl Clone for StorageSurface {
    fn clone(&self) -> Self {
        Self {
            log: Arc::clone(&self.log),
            state_machine: Arc::clone(&self.state_machine),
        }
    }
}

impl StorageSurface {
    /// Builds an in-memory surface suitable for deterministic tests.
    pub fn in_memory() -> Self {
        Self {
            log: Arc::new(InMemoryLog::default()),
            state_machine: Arc::new(InMemoryStateMachine::default()),
        }
    }

    /// Construct a surface from explicit handles (e.g., redb, forthcoming).
    pub fn from_parts(log: Arc<dyn LogHandle>, state_machine: Arc<dyn StateMachineHandle>) -> Self {
        Self { log, state_machine }
    }

    /// Access the log handle.
    pub fn log(&self) -> Arc<dyn LogHandle> {
        Arc::clone(&self.log)
    }

    /// Access the state machine handle.
    pub fn state_machine(&self) -> Arc<dyn StateMachineHandle> {
        Arc::clone(&self.state_machine)
    }
}

/// Storage plan describing how to construct deterministic backends.
#[derive(Debug, Clone)]
pub struct StoragePlan {
    /// Node identifier for namespacing deterministic data directories.
    pub node_id: String,
    /// Optional deterministic seed.
    pub seed: Option<u64>,
    /// Path on disk where persistent storage will land during Phase 2 (redb).
    pub data_dir: Option<std::path::PathBuf>,
}

impl StoragePlan {
    /// Validate invariants before the plan is executed.
    pub fn validate(&self) {
        assert!(
            !self.node_id.is_empty(),
            "storage plans require stable node identifiers"
        );
    }

    /// For now the plan materializes an in-memory surface; Phase 2 will route
    /// `data_dir` to the redb backend.
    pub fn materialize(&self) -> anyhow::Result<StorageSurface> {
        self.validate();
        if let Some(dir) = &self.data_dir {
            let log_path = dir.join("log.redb");
            let sm_path = dir.join("sm.redb");
            let log: Arc<dyn LogHandle> = Arc::new(RedbLog::open(&log_path)?);
            let sm: Arc<dyn StateMachineHandle> = Arc::new(RedbStateMachine::open(&sm_path)?);
            Ok(StorageSurface::from_parts(log, sm))
        } else {
            let mut log_impl = InMemoryLog::default();
            if let Some(seed) = self.seed {
                log_impl.seed(seed);
            }
            let log: Arc<dyn LogHandle> = Arc::new(log_impl);
            let sm: Arc<dyn StateMachineHandle> = Arc::new(InMemoryStateMachine::default());
            Ok(StorageSurface::from_parts(log, sm))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn storage_surface_in_memory_isolated() {
        let plan = StoragePlan {
            node_id: "node-a".into(),
            seed: Some(42),
            data_dir: None,
        };
        let surface = plan.materialize().unwrap();
        surface.log().append(1, b"one").unwrap();
        assert_eq!(surface.log().next_index().unwrap(), 2);
        let bytes = surface.state_machine().snapshot().unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn storage_plan_with_data_dir_uses_redb() {
        let tmp = TempDir::new().unwrap();
        let plan = StoragePlan {
            node_id: "node-b".into(),
            seed: None,
            data_dir: Some(tmp.path().to_path_buf()),
        };
        let surface = plan.materialize().unwrap();
        surface.log().append(1, b"persisted").unwrap();
        assert_eq!(surface.log().next_index().unwrap(), 2);
        surface.state_machine().apply(b"alpha").unwrap();
        assert_eq!(surface.state_machine().last_applied().unwrap(), 1);
    }

    mod openraft_suite {
        use super::*;
        use anyerror::AnyError;
        use anyhow::Context;
        use openraft::impls::OneshotResponder;
        use openraft::storage::{
            LogFlushed, RaftLogReader, RaftLogStorage, RaftSnapshotBuilder, RaftStateMachine,
        };
        use openraft::testing::{StoreBuilder, Suite};
        use openraft::{
            BasicNode, Entry, EntryPayload, ErrorSubject, ErrorVerb, LogId, LogState, OptionalSend,
            RaftTypeConfig, Snapshot, SnapshotMeta, StorageError, StorageIOError, StoredMembership,
            TokioRuntime, Vote,
        };
        use std::fmt::Debug;
        use std::io::{Cursor, Read};
        use std::path::PathBuf;
        use std::sync::{Arc, Mutex};

        type Result<T> = std::result::Result<T, StorageError<u64>>;

        #[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
        struct SuiteConfig;

        impl RaftTypeConfig for SuiteConfig {
            type D = ();
            type R = ();
            type NodeId = u64;
            type Node = BasicNode;
            type Entry = Entry<Self>;
            type SnapshotData = Cursor<Vec<u8>>;
            type AsyncRuntime = TokioRuntime;
            type Responder = OneshotResponder<Self>;
        }

        #[derive(Clone)]
        struct SuiteLogStore {
            handle: Arc<dyn log::LogHandle>,
            vote: Arc<Mutex<Option<Vote<u64>>>>,
            committed: Arc<Mutex<Option<LogId<u64>>>>,
            last_purged: Arc<Mutex<Option<LogId<u64>>>>,
        }

        struct SuiteStateMachine {
            sm: Arc<dyn state_machine::StateMachineHandle>,
            last_applied: Arc<Mutex<Option<LogId<u64>>>>,
            last_membership: Arc<Mutex<StoredMembership<u64, BasicNode>>>,
            snapshot_meta: Arc<Mutex<Option<SnapshotMeta<u64, BasicNode>>>>,
        }

        struct SuiteSnapshotBuilder {
            sm: Arc<dyn state_machine::StateMachineHandle>,
            meta: SnapshotMeta<u64, BasicNode>,
        }

        #[derive(Clone)]
        enum BuilderVariant {
            InMemory,
            Persistent,
        }

        #[derive(Clone)]
        struct SurfaceStoreBuilder {
            variant: BuilderVariant,
        }

        impl SurfaceStoreBuilder {
            fn in_memory() -> Self {
                Self {
                    variant: BuilderVariant::InMemory,
                }
            }

            fn persistent() -> Self {
                Self {
                    variant: BuilderVariant::Persistent,
                }
            }
        }

        #[derive(Default)]
        struct StorageGuard {
            path: Option<PathBuf>,
        }

        impl Drop for StorageGuard {
            fn drop(&mut self) {
                if let Some(path) = &self.path {
                    let _ = std::fs::remove_dir_all(path);
                }
            }
        }

        impl StoreBuilder<SuiteConfig, SuiteLogStore, SuiteStateMachine, StorageGuard>
            for SurfaceStoreBuilder
        {
            fn build(
                &self,
            ) -> impl std::future::Future<
                Output = Result<(StorageGuard, SuiteLogStore, SuiteStateMachine)>,
            > + Send {
                async move {
                    let (plan, guard) = match self.variant {
                        BuilderVariant::InMemory => (
                            StoragePlan {
                                node_id: "suite-mem".into(),
                                seed: Some(42),
                                data_dir: None,
                            },
                            StorageGuard::default(),
                        ),
                        BuilderVariant::Persistent => {
                            let tmp = tempfile::tempdir().expect("create temp dir");
                            let path = tmp.into_path();
                            let plan = StoragePlan {
                                node_id: "suite-redb".into(),
                                seed: None,
                                data_dir: Some(path.clone()),
                            };
                            (plan, StorageGuard { path: Some(path) })
                        }
                    };

                    let surface = plan
                        .materialize()
                        .map_err(|e| storage_err(ErrorSubject::Store, ErrorVerb::Write, e))?;
                    let log = surface.log();
                    let sm = surface.state_machine();
                    Ok((
                        guard,
                        SuiteLogStore {
                            handle: log,
                            vote: Arc::new(Mutex::new(None)),
                            committed: Arc::new(Mutex::new(None)),
                            last_purged: Arc::new(Mutex::new(None)),
                        },
                        SuiteStateMachine {
                            sm,
                            last_applied: Arc::new(Mutex::new(None)),
                            last_membership: Arc::new(Mutex::new(StoredMembership::default())),
                            snapshot_meta: Arc::new(Mutex::new(None)),
                        },
                    ))
                }
            }
        }

        fn storage_err(
            subject: ErrorSubject<u64>,
            verb: ErrorVerb,
            err: anyhow::Error,
        ) -> StorageError<u64> {
            StorageIOError::new(subject, verb, AnyError::from(err)).into()
        }

        fn encode_entry(entry: &Entry<SuiteConfig>) -> anyhow::Result<Vec<u8>> {
            bincode::serialize(entry).context("serialize raft entry")
        }

        fn decode_entry(bytes: &[u8]) -> anyhow::Result<Entry<SuiteConfig>> {
            bincode::deserialize(bytes).context("deserialize raft entry")
        }

        fn load_entries(
            handle: &dyn log::LogHandle,
            range: impl std::ops::RangeBounds<u64> + Clone + Debug,
        ) -> Result<Vec<Entry<SuiteConfig>>> {
            use std::ops::Bound;
            let start = match range.start_bound() {
                Bound::Included(v) => *v,
                Bound::Excluded(v) => v.saturating_add(1),
                Bound::Unbounded => 0,
            };
            let end = match range.end_bound() {
                Bound::Included(v) => v + 1,
                Bound::Excluded(v) => *v,
                Bound::Unbounded => handle
                    .next_index()
                    .map_err(|e| storage_err(ErrorSubject::Logs, ErrorVerb::Read, e))?,
            };
            if end <= start {
                return Ok(Vec::new());
            }
            let raw = handle
                .read(start..=end - 1)
                .map_err(|e| storage_err(ErrorSubject::Logs, ErrorVerb::Read, e))?;
            let mut out = Vec::with_capacity(raw.len());
            for entry in raw {
                out.push(
                    decode_entry(&entry.bytes)
                        .map_err(|e| storage_err(ErrorSubject::Logs, ErrorVerb::Read, e))?,
                );
            }
            Ok(out)
        }

        impl SuiteLogStore {
            fn last_entry(&self) -> Result<Option<Entry<SuiteConfig>>> {
                let next = self
                    .handle
                    .next_index()
                    .map_err(|e| storage_err(ErrorSubject::Logs, ErrorVerb::Read, e))?;
                if next == 1 {
                    return Ok(None);
                }
                let entries = load_entries(&*self.handle, (next - 1)..=next - 1)?;
                Ok(entries.into_iter().next())
            }
        }

        impl RaftLogStorage<SuiteConfig> for SuiteLogStore {
            type LogReader = SuiteLogStore;

            fn get_log_state(
                &mut self,
            ) -> impl std::future::Future<Output = Result<LogState<SuiteConfig>>> + Send
            {
                async move {
                    let last = self.last_entry()?.map(|entry| entry.log_id);
                    let last_purged = self.last_purged.lock().unwrap().clone();
                    Ok(LogState {
                        last_purged_log_id: last_purged,
                        last_log_id: last,
                    })
                }
            }

            fn get_log_reader(
                &mut self,
            ) -> impl std::future::Future<Output = Self::LogReader> + Send {
                let reader = self.clone();
                async move { reader }
            }

            fn save_vote(
                &mut self,
                vote: &Vote<u64>,
            ) -> impl std::future::Future<Output = Result<()>> + Send {
                let vote = vote.clone();
                async move {
                    *self.vote.lock().unwrap() = Some(vote);
                    Ok(())
                }
            }

            fn read_vote(
                &mut self,
            ) -> impl std::future::Future<Output = Result<Option<Vote<u64>>>> + Send {
                async move { Ok(self.vote.lock().unwrap().clone()) }
            }

            fn save_committed(
                &mut self,
                committed: Option<LogId<u64>>,
            ) -> impl std::future::Future<Output = Result<()>> + Send {
                async move {
                    *self.committed.lock().unwrap() = committed;
                    Ok(())
                }
            }

            fn read_committed(
                &mut self,
            ) -> impl std::future::Future<Output = Result<Option<LogId<u64>>>> + Send {
                async move { Ok(self.committed.lock().unwrap().clone()) }
            }

            fn append<I>(
                &mut self,
                entries: I,
                callback: LogFlushed<SuiteConfig>,
            ) -> impl std::future::Future<Output = Result<()>> + Send
            where
                I: IntoIterator<Item = Entry<SuiteConfig>> + OptionalSend,
                I::IntoIter: OptionalSend,
            {
                async move {
                    for entry in entries {
                        let bytes = encode_entry(&entry)
                            .map_err(|e| storage_err(ErrorSubject::Logs, ErrorVerb::Write, e))?;
                        self.handle
                            .append(entry.log_id.index, &bytes)
                            .map_err(|e| storage_err(ErrorSubject::Logs, ErrorVerb::Write, e))?;
                    }
                    callback.log_io_completed(Ok(()));
                    Ok(())
                }
            }

            fn truncate(
                &mut self,
                log_id: LogId<u64>,
            ) -> impl std::future::Future<Output = Result<()>> + Send {
                async move {
                    self.handle
                        .truncate(log_id.index)
                        .map_err(|e| storage_err(ErrorSubject::Logs, ErrorVerb::Delete, e))
                }
            }

            fn purge(
                &mut self,
                log_id: LogId<u64>,
            ) -> impl std::future::Future<Output = Result<()>> + Send {
                async move {
                    self.handle
                        .purge(log_id.index)
                        .map_err(|e| storage_err(ErrorSubject::Logs, ErrorVerb::Delete, e))?;
                    *self.last_purged.lock().unwrap() = Some(log_id);
                    Ok(())
                }
            }
        }

        impl RaftLogReader<SuiteConfig> for SuiteLogStore {
            fn try_get_log_entries<RB>(
                &mut self,
                range: RB,
            ) -> impl std::future::Future<Output = Result<Vec<Entry<SuiteConfig>>>> + Send
            where
                RB: std::ops::RangeBounds<u64> + Clone + Debug + OptionalSend,
            {
                async move { load_entries(&*self.handle, range) }
            }
        }

        impl RaftStateMachine<SuiteConfig> for SuiteStateMachine {
            type SnapshotBuilder = SuiteSnapshotBuilder;

            fn applied_state(
                &mut self,
            ) -> impl std::future::Future<
                Output = Result<(Option<LogId<u64>>, StoredMembership<u64, BasicNode>)>,
            > + Send {
                async move {
                    Ok((
                        self.last_applied.lock().unwrap().clone(),
                        self.last_membership.lock().unwrap().clone(),
                    ))
                }
            }

            fn apply<I>(
                &mut self,
                entries: I,
            ) -> impl std::future::Future<Output = Result<Vec<()>>> + Send
            where
                I: IntoIterator<Item = Entry<SuiteConfig>> + OptionalSend,
                I::IntoIter: OptionalSend,
            {
                async move {
                    let mut responses = Vec::new();
                    for entry in entries {
                        match entry.payload {
                            EntryPayload::Normal(_) | EntryPayload::Blank => {
                                self.sm
                                    .apply(&encode_entry(&entry).map_err(|e| {
                                        storage_err(ErrorSubject::StateMachine, ErrorVerb::Write, e)
                                    })?)
                                    .map_err(|e| {
                                        storage_err(ErrorSubject::StateMachine, ErrorVerb::Write, e)
                                    })?;
                            }
                            EntryPayload::Membership(mem) => {
                                *self.last_membership.lock().unwrap() =
                                    StoredMembership::new(Some(entry.log_id.clone()), mem);
                            }
                        }
                        *self.last_applied.lock().unwrap() = Some(entry.log_id);
                        responses.push(());
                    }
                    Ok(responses)
                }
            }

            fn get_snapshot_builder(
                &mut self,
            ) -> impl std::future::Future<Output = Self::SnapshotBuilder> + Send {
                async move {
                    let meta = SnapshotMeta {
                        last_log_id: self.last_applied.lock().unwrap().clone(),
                        last_membership: self.last_membership.lock().unwrap().clone(),
                        snapshot_id: Default::default(),
                    };
                    SuiteSnapshotBuilder {
                        sm: self.sm.clone(),
                        meta,
                    }
                }
            }

            fn begin_receiving_snapshot(
                &mut self,
            ) -> impl std::future::Future<Output = Result<Box<Cursor<Vec<u8>>>>> + Send
            {
                async move { Ok(Box::new(Cursor::new(Vec::new()))) }
            }

            fn install_snapshot(
                &mut self,
                meta: &SnapshotMeta<u64, BasicNode>,
                mut snapshot: Box<Cursor<Vec<u8>>>,
            ) -> impl std::future::Future<Output = Result<()>> + Send {
                async move {
                    let mut bytes = Vec::new();
                    snapshot.read_to_end(&mut bytes).map_err(|e| {
                        storage_err(
                            ErrorSubject::Snapshot(Some(meta.signature())),
                            ErrorVerb::Read,
                            e.into(),
                        )
                    })?;
                    self.sm.hydrate(&bytes).map_err(|e| {
                        storage_err(
                            ErrorSubject::Snapshot(Some(meta.signature())),
                            ErrorVerb::Write,
                            e,
                        )
                    })?;
                    *self.last_applied.lock().unwrap() = meta.last_log_id.clone();
                    *self.last_membership.lock().unwrap() = meta.last_membership.clone();
                    *self.snapshot_meta.lock().unwrap() = Some(meta.clone());
                    Ok(())
                }
            }

            fn get_current_snapshot(
                &mut self,
            ) -> impl std::future::Future<Output = Result<Option<Snapshot<SuiteConfig>>>> + Send
            {
                async move {
                    if let Some(meta) = self.snapshot_meta.lock().unwrap().clone() {
                        let bytes = self.sm.snapshot().map_err(|e| {
                            storage_err(
                                ErrorSubject::Snapshot(Some(meta.signature())),
                                ErrorVerb::Read,
                                e,
                            )
                        })?;
                        Ok(Some(Snapshot {
                            meta,
                            snapshot: Box::new(Cursor::new(bytes)),
                        }))
                    } else {
                        Ok(None)
                    }
                }
            }
        }

        impl RaftSnapshotBuilder<SuiteConfig> for SuiteSnapshotBuilder {
            fn build_snapshot(
                &mut self,
            ) -> impl std::future::Future<Output = Result<Snapshot<SuiteConfig>>> + Send
            {
                async move {
                    let bytes = self.sm.snapshot().map_err(|e| {
                        storage_err(
                            ErrorSubject::Snapshot(Some(self.meta.signature())),
                            ErrorVerb::Read,
                            e,
                        )
                    })?;
                    Ok(Snapshot {
                        meta: self.meta.clone(),
                        snapshot: Box::new(Cursor::new(bytes)),
                    })
                }
            }
        }

        #[test]
        fn suite_in_memory() {
            let builder = SurfaceStoreBuilder::in_memory();
            Suite::<SuiteConfig, _, _, _, _>::test_all(builder).expect("in-memory suite");
        }

        #[test]
        fn suite_redb() {
            let builder = SurfaceStoreBuilder::persistent();
            Suite::<SuiteConfig, _, _, _, _>::test_all(builder).expect("redb suite");
        }
    }
}
