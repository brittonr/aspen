// SPDX-License-Identifier: AGPL-3.0-or-later
mod common;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use async_trait::async_trait;
use common::*;
use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_adapters::*;

struct Allow;
impl AdmissionPort for Allow {
    fn submit(&self, _: &JobSpec, _: Id) -> Result<(), Error> {
        Ok(())
    }
    fn assignment(&self, _: &ExecutionRequest, _: AdmissionAction) -> Result<(), Error> {
        Ok(())
    }
}
struct UnknownStore {
    inner: Arc<RedbControllerStore>,
    after: bool,
}
impl StateStore for UnknownStore {
    fn load(&self) -> Result<State, Error> {
        self.inner.load()
    }
    fn compare_commit(&self, old: &State, next: &State) -> Result<Commit, Error> {
        if next.attempts.len() > old.attempts.len() {
            if self.after {
                assert_eq!(self.inner.compare_commit(old, next)?, Commit::Applied);
            }
            return Ok(Commit::Unknown);
        }
        self.inner.compare_commit(old, next)
    }
}
struct RecordingWorker {
    starts: AtomicUsize,
}
#[async_trait]
impl WorkerClient for RecordingWorker {
    async fn info(&self, node: Id) -> Result<WorkerInfo, Error> {
        Ok(WorkerInfo {
            node,
            cluster: id(1),
            generation: 1,
            policy_ref: id(2),
            capacity: resources(),
            boot: 1,
            draining: false,
        })
    }
    async fn put(&self, _: Id, _: Id, _: Content) -> Result<(), Error> {
        Ok(())
    }
    async fn get(&self, _: Id, _: Id) -> Result<Content, Error> {
        Err(Error::Unavailable)
    }
    async fn start(&self, r: ExecutionRequest) -> Result<WorkerObservation, Error> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        Ok(WorkerObservation::Running {
            request_ref: CanonicalIdentity.request(&r)?,
        })
    }
    async fn observe(&self, _: &ExecutionRequest) -> Result<WorkerObservation, Error> {
        Err(Error::Unavailable)
    }
    async fn cancel(&self, _: &ExecutionRequest) -> Result<WorkerObservation, Error> {
        Err(Error::Unavailable)
    }
}
#[tokio::test]
async fn unknown_before_or_after_commit_never_dispatches() {
    for after in [false, true] {
        let env = setup();
        let hash = env.content.put(&fixture()).unwrap();
        let workers = Arc::new(RecordingWorker {
            starts: AtomicUsize::new(0),
        });
        let controller = Controller::new(ControllerPorts {
            store: Arc::new(UnknownStore {
                inner: env.store.clone(),
                after,
            }),
            content: env.content.clone(),
            identity: Arc::new(CanonicalIdentity),
            placement: Arc::new(FirstFitPlacement),
            admission: Arc::new(Allow),
            workers: workers.clone(),
            observations: Arc::new(Counters::default()),
        });
        controller
            .register(Node {
                id: id(4),
                capacity: resources(),
                labels: BTreeMap::new(),
                draining: false,
                enabled: true,
                last_seen: None,
                boot: 0,
            })
            .unwrap();
        controller
            .submit("unknown".into(), JobSpec {
                name: "unknown".into(),
                tasks: vec![task(hash)],
            })
            .unwrap();
        controller.tick().await.unwrap();
        assert_eq!(workers.starts.load(Ordering::SeqCst), 0);
        assert_eq!(controller.state().unwrap().attempts.len(), usize::from(after));
    }
}
struct UnknownWorkerJournal {
    inner: Arc<RedbWorkerJournal>,
}
impl WorkerJournal for UnknownWorkerJournal {
    fn load(&self) -> Result<WorkerState, Error> {
        self.inner.load()
    }
    fn compare_commit(&self, old: &WorkerState, next: &WorkerState) -> Result<Commit, Error> {
        let accepted = next.records.len() > old.records.len();
        let result = self.inner.compare_commit(old, next)?;
        Ok(if accepted && result == Commit::Applied {
            Commit::Unknown
        } else {
            result
        })
    }
}
struct NeverRun {
    calls: AtomicUsize,
}
#[async_trait]
impl ExecutionPort for NeverRun {
    async fn run(&self, _: ExecutionRequest, _: Content, _: Vec<Vec<u8>>) -> Result<ExecutionOutput, Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(Error::Unknown)
    }
    fn cancel(&self, _: Id) -> Result<(), Error> {
        Ok(())
    }
}
#[test]
fn unknown_worker_acceptance_does_not_mint_a_launch_ticket() {
    let temp = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(temp.path()).unwrap();
    let disk = Disk::open(&root).unwrap();
    let journal =
        RedbWorkerJournal::open(disk.clone(), WorkerState::empty(id(1), id(4), 1, id(2), resources()).unwrap())
            .unwrap();
    let runtime = Arc::new(NeverRun {
        calls: AtomicUsize::new(0),
    });
    let worker = WorkerService::new(WorkerPorts {
        journal: Arc::new(UnknownWorkerJournal { inner: journal.clone() }),
        content: RedbContent::new(disk),
        identity: Arc::new(CanonicalIdentity),
        admission: Arc::new(Allow),
        execution: runtime.clone(),
        observations: Arc::new(Counters::default()),
    });
    worker.recover_after_exclusive_open().unwrap();
    let r = request(content_id(&fixture()).unwrap());
    assert!(matches!(worker.start(r.clone()), Err(Error::Unknown)));
    assert_eq!(runtime.calls.load(Ordering::SeqCst), 0);
    let (_, ticket) = worker.start(r).unwrap();
    assert!(ticket.is_none());
    assert_eq!(runtime.calls.load(Ordering::SeqCst), 0);
}
