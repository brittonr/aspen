// SPDX-License-Identifier: AGPL-3.0-or-later
mod common;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Barrier;

use common::*;
use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_adapters::*;
#[test]
fn exact_whole_state_cas_has_one_winner() {
    let env = setup();
    let old = env.store.load().unwrap();
    let node = |n| Node {
        id: id(n),
        capacity: resources(),
        labels: BTreeMap::new(),
        draining: false,
        enabled: true,
        last_seen: None,
        boot: 0,
    };
    let a = apply(&old, &Command::Register(node(4))).unwrap();
    let b = apply(&old, &Command::Register(node(5))).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut threads = vec![];
    for next in [a, b] {
        let store = env.store.clone();
        let old = old.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            store.compare_commit(&old, &next).unwrap()
        }));
    }
    barrier.wait();
    let results = threads.into_iter().map(|t| t.join().unwrap()).collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|&&r| r == Commit::Applied).count(), 1);
    assert_eq!(results.iter().filter(|&&r| r == Commit::Conflict).count(), 1);
    assert_eq!(env.store.load().unwrap().nodes.len(), 1);
}
#[test]
fn content_roundtrip_is_idempotent_and_checks_bytes() {
    let env = setup();
    let c = content(ContentKind::Data, b"data".to_vec()).unwrap();
    let id = env.content.put(&c).unwrap();
    assert_eq!(env.content.put(&c).unwrap(), id);
    assert_eq!(env.content.get(id).unwrap(), c);
    let mut bad = c;
    bad.bytes.push(0);
    assert!(env.content.put(&bad).is_err());
}
#[test]
fn actual_redb_reopen_preserves_state_and_content() {
    let env = setup();
    let data = content(ContentKind::Data, b"survives-reopen".to_vec()).unwrap();
    let hash = env.content.put(&data).unwrap();
    let old = env.store.load().unwrap();
    let next = apply(&old, &Command::Tick).unwrap();
    assert_eq!(env.store.compare_commit(&old, &next).unwrap(), Commit::Applied);
    let path = env.temporary.path().to_path_buf();
    drop(env.store);
    drop(env.content);
    drop(env.disk);
    let root = CapabilityRoot::open(&path).unwrap();
    let disk = Disk::open(&root).unwrap();
    let store = RedbControllerStore::open(disk.clone(), State::empty(id(1), id(2), 1, 3).unwrap()).unwrap();
    assert_eq!(store.load().unwrap(), next);
    assert_eq!(RedbContent::new(disk).get(hash).unwrap(), data);
}
#[test]
fn worker_reopen_recovers_interrupted_attempt_without_reexecution() {
    let temp = tempfile::tempdir().unwrap();
    let root = CapabilityRoot::open(temp.path()).unwrap();
    let disk = Disk::open(&root).unwrap();
    let initial = WorkerState::empty(id(1), id(4), 1, id(2), resources()).unwrap();
    let journal = RedbWorkerJournal::open(disk.clone(), initial.clone()).unwrap();
    let old = journal.load().unwrap();
    let boot = worker_apply(&old, &WorkerCommand::RecoverAfterProcessExit).unwrap();
    journal.compare_commit(&old, &boot).unwrap();
    let c = fixture();
    let r = request(content_id(&c).unwrap());
    let request_ref = CanonicalIdentity.request(&r).unwrap();
    let accepted = worker_apply(&boot, &WorkerCommand::Accept {
        request: r.clone(),
        request_ref,
    })
    .unwrap();
    journal.compare_commit(&boot, &accepted).unwrap();
    let running = worker_apply(&accepted, &WorkerCommand::Begin {
        attempt: r.fence.assignment,
        request_ref,
        boot: 1,
    })
    .unwrap();
    journal.compare_commit(&accepted, &running).unwrap();
    drop(journal);
    drop(disk);
    let disk = Disk::open(&root).unwrap();
    let journal = RedbWorkerJournal::open(disk, initial).unwrap();
    let old = journal.load().unwrap();
    assert_eq!(old, running);
    let recovered = worker_apply(&old, &WorkerCommand::RecoverAfterProcessExit).unwrap();
    journal.compare_commit(&old, &recovered).unwrap();
    assert!(matches!(journal.load().unwrap().records[&r.fence.assignment].phase, WorkerPhase::Terminal(_)));
}
#[cfg(unix)]
#[test]
fn root_refuses_parent_paths_and_symlinked_database() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    let root = CapabilityRoot::open(temp.path()).unwrap();
    assert!(root.read("../outside", 10).is_err());
    symlink(outside.path(), temp.path().join("orchestration.redb")).unwrap();
    assert!(Disk::open(&root).is_err());
}
#[test]
fn database_exclusive_lock_prevents_two_live_process_owners() {
    let env = setup();
    let root = CapabilityRoot::open(env.temporary.path()).unwrap();
    assert!(Disk::open(&root).is_err());
}
