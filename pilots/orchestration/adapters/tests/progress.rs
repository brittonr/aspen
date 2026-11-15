// SPDX-License-Identifier: AGPL-3.0-or-later
mod common;
use common::*;
use molten_batch_application::*;
use molten_batch_core::*;
use molten_orchestrator_adapters::*;

fn commit(env: &Setup, command: &Command<'_>) -> State {
    let old = env.store.load().unwrap();
    let next = apply(&old, command).unwrap();
    assert_eq!(env.store.compare_commit(&old, &next).unwrap(), Commit::Applied);
    next
}
fn active(env: &Setup) -> ExecutionRequest {
    let request = request(id(7));
    let identity = CanonicalIdentity;
    let spec = JobSpec {
        name: "test".into(),
        tasks: vec![task(id(7))],
    };
    commit(
        env,
        &Command::Register(Node {
            id: id(4),
            capacity: resources(),
            labels: Default::default(),
            draining: false,
            enabled: true,
            last_seen: None,
            boot: 0,
        }),
    );
    commit(env, &Command::ObserveNode { node: id(4), boot: 1 });
    commit(env, &Command::Submit {
        key: "test".into(),
        spec_ref: request.fence.spec_ref,
        run: request.fence.run,
        spec,
    });
    commit(env, &Command::Schedule {
        placement: Placement {
            run: request.fence.run,
            task: "task".into(),
            node: id(4),
            number: 1,
            inputs: vec![],
        },
        request: &request,
        request_ref: identity.request(&request).unwrap(),
    });
    request
}
fn finish(env: &Setup, request: &ExecutionRequest, output: Id) -> Command<'static> {
    let result = ExecutionResult {
        request_ref: CanonicalIdentity.request(request).unwrap(),
        outcome: ExecutionOutcome::Succeeded { output },
        stopped: true,
    };
    let _ = env;
    Command::Terminal {
        attempt: request.fence.assignment,
        result_ref: CanonicalIdentity.result(&result).unwrap(),
        result,
    }
}
// r[verify molten.observations.stream]
// r[verify molten.observations.cursors]
#[test]
fn output_state_progress_and_watermark_reopen_together() {
    let env = setup();
    let request = active(&env);
    let before = env.store.progress_snapshot(request.fence.run).unwrap();
    assert!(matches!(before.tasks["task"], ProgressTask::Dispatching { .. }));
    let bytes = content(ContentKind::Data, b"accepted".to_vec()).unwrap();
    let output = env.content.put(&bytes).unwrap();
    let terminal = finish(&env, &request, output);
    let accepted = commit(&env, &terminal);
    let page = env.store.progress_read(before.watermark.clone(), 1).unwrap();
    let ProgressPage::Events { events, next } = &page else {
        panic!("expected event")
    };
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0].payload, ProgressPayload::AcceptedOutput { fence, output: got, .. }
        if *got == output && fence == &request.fence));
    assert_eq!(env.store.progress_read(before.watermark.clone(), 1).unwrap(), page);
    let ProgressPage::Events { events: tail, .. } = env.store.progress_read(next.clone(), 1).unwrap() else {
        panic!()
    };
    assert!(matches!(tail[0].payload, ProgressPayload::RunTerminal {
        phase: RunPhase::Succeeded,
        ..
    }));
    assert_eq!(apply(&accepted, &terminal).unwrap(), accepted);
    let projected = env.store.progress_snapshot(request.fence.run).unwrap();
    let current = commit(&env, &Command::Tick);
    let snapshot = env.store.progress_snapshot(request.fence.run).unwrap();
    assert_eq!(snapshot.state_ref, molten_orchestrator_codec::identity("controller-state.v1", &current).unwrap());
    assert_eq!(snapshot.watermark, projected.watermark);
    assert_eq!(snapshot.tasks, projected.tasks);
    assert_eq!(snapshot.phase, RunPhase::Succeeded);
    assert!(matches!(snapshot.tasks["task"], ProgressTask::Accepted { output: got, .. } if got == output));
    let path = env.temporary.path().to_owned();
    let Setup {
        temporary,
        disk,
        store,
        content,
    } = env;
    drop((disk, store, content));
    let reopened = Disk::open(&CapabilityRoot::open(&path).unwrap()).unwrap();
    let store = RedbControllerStore::open(reopened.clone(), State::empty(id(1), id(2), 1, 3).unwrap()).unwrap();
    assert_eq!(store.progress_snapshot(request.fence.run).unwrap(), snapshot);
    assert_eq!(store.progress_read(before.watermark, 1).unwrap(), page);
    assert_eq!(RedbContent::new(reopened).get(output).unwrap(), bytes);
    drop(temporary);
}
#[test]
fn unmaterialized_outputs_cannot_commit_and_history_gap_is_not_artifact_gap() {
    let env = setup();
    let request = active(&env);
    let old = env.store.load().unwrap();
    let terminal = finish(&env, &request, id(9));
    let next = apply(&old, &terminal).unwrap();
    assert_eq!(env.store.compare_commit(&old, &next), Err(Error::Unavailable));
    assert_eq!(env.store.load().unwrap(), old);
    assert_eq!(env.content.get(id(9)), Err(Error::Unavailable));
    let snapshot = env.store.progress_snapshot(request.fence.run).unwrap();
    assert_eq!(env.store.progress_read(snapshot.watermark.clone(), 0), Err(Error::Bound));
    assert_eq!(env.store.progress_read(snapshot.watermark.clone(), 65), Err(Error::Bound));
    let mut stale = snapshot.watermark.clone();
    stale.stream_epoch += 1;
    assert!(matches!(env.store.progress_read(stale, 1).unwrap(), ProgressPage::StaleEpoch { .. }));
    let mut foreign = snapshot.watermark;
    foreign.service_id = "another-service".into();
    assert_eq!(env.store.progress_read(foreign, 1).unwrap(), ProgressPage::ForeignService);
}
#[test]
fn uncertain_attempt_and_cancel_request_do_not_claim_executor_stop() {
    let env = setup();
    let request = active(&env);
    commit(&env, &Command::Uncertain {
        attempt: request.fence.assignment,
        request_ref: CanonicalIdentity.request(&request).unwrap(),
    });
    let watermark = env.store.progress_snapshot(request.fence.run).unwrap().watermark;
    commit(&env, &Command::CancelRun { run: request.fence.run });
    let snapshot = env.store.progress_snapshot(request.fence.run).unwrap();
    assert_eq!(snapshot.phase, RunPhase::Cancelling);
    assert!(matches!(snapshot.tasks["task"], ProgressTask::Uncertain { .. }));
    assert!(env.store.load().unwrap().attempts[&request.fence.assignment].reservation_held);
    assert!(
        matches!(env.store.progress_read(watermark, 64).unwrap(), ProgressPage::Events { events, .. } if events.is_empty())
    );
    assert!(!env.store.load().unwrap().runs[&request.fence.run].tasks.is_empty());
}

#[test]
fn late_success_from_a_failed_attempt_cannot_replace_the_accepted_retry() {
    let env = setup();
    let first = active(&env);
    let request_ref = CanonicalIdentity.request(&first).unwrap();
    let failure = ExecutionResult {
        request_ref,
        outcome: ExecutionOutcome::Failed {
            code: FailureCode::Interrupted,
        },
        stopped: true,
    };
    commit(&env, &Command::Terminal {
        attempt: first.fence.assignment,
        result_ref: CanonicalIdentity.result(&failure).unwrap(),
        result: failure,
    });
    commit(&env, &Command::Tick);
    let placement = Placement {
        run: first.fence.run,
        task: "task".into(),
        node: id(4),
        number: 2,
        inputs: vec![],
    };
    let mut retry = first.clone();
    retry.fence.assignment = CanonicalIdentity.assignment(id(1), 1, &placement, retry.fence.spec_ref).unwrap();
    retry.fence.epoch = 2;
    commit(&env, &Command::Schedule {
        placement,
        request: &retry,
        request_ref: CanonicalIdentity.request(&retry).unwrap(),
    });
    let output = env.content.put(&content(ContentKind::Data, b"current".to_vec()).unwrap()).unwrap();
    commit(&env, &finish(&env, &retry, output));
    let accepted = env.store.load().unwrap();
    assert!(apply(&accepted, &finish(&env, &first, id(9))).is_err());
    let snapshot = env.store.progress_snapshot(first.fence.run).unwrap();
    assert!(matches!(&snapshot.tasks["task"], ProgressTask::Accepted { fence, output: got, .. }
        if fence.assignment == retry.fence.assignment && *got == output));
}
