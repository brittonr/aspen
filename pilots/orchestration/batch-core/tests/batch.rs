// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeMap;

use molten_batch_core::*;
fn id(b: u8) -> Id {
    Id([b; 32])
}
fn resources() -> Resources {
    Resources {
        cpu_millis: 1000,
        memory_bytes: 65536,
        slots: 1,
    }
}
fn task(name: &str, deps: &[&str]) -> TaskSpec {
    TaskSpec {
        id: name.into(),
        workload: Workload::Component { artifact: id(3) },
        inputs: vec![],
        dependencies: deps.iter().map(|v| (*v).into()).collect(),
        resources: resources(),
        required_labels: BTreeMap::new(),
        fuel: 10000,
        timeout_ms: 1000,
        max_output_bytes: 1024,
        max_attempts: 2,
        retry_delay_ticks: 1,
    }
}
fn initial() -> State {
    let s = State::empty(id(1), id(2), 1, 3).unwrap();
    let s = apply(
        &s,
        &Command::Register(Node {
            id: id(4),
            capacity: resources(),
            labels: BTreeMap::new(),
            draining: false,
            enabled: true,
            last_seen: None,
            boot: 0,
        }),
    )
    .unwrap();
    apply(&s, &Command::ObserveNode { node: id(4), boot: 1 }).unwrap()
}
fn submit(s: &State, key: &str, run: Id, tasks: Vec<TaskSpec>) -> State {
    apply(s, &Command::Submit {
        key: key.into(),
        spec_ref: id(6),
        run,
        spec: JobSpec {
            name: key.into(),
            tasks,
        },
    })
    .unwrap()
}
fn request(s: &State, p: &Placement, assignment: Id) -> ExecutionRequest {
    let t = s.runs[&p.run].spec.tasks.iter().find(|t| t.id == p.task).unwrap();
    ExecutionRequest {
        fence: Fence {
            cluster: s.cluster,
            generation: s.generation,
            run: p.run,
            task: p.task.clone(),
            assignment,
            spec_ref: s.runs[&p.run].spec_ref,
            node: p.node,
            epoch: p.number,
        },
        policy_ref: s.policy_ref,
        workload: t.workload,
        inputs: p.inputs.clone(),
        resources: t.resources,
        fuel: t.fuel,
        timeout_ms: t.timeout_ms,
        max_output_bytes: t.max_output_bytes,
        max_attempts: t.max_attempts,
    }
}
fn schedule(s: &State, assignment: Id) -> State {
    let p = next_placement(s, None).unwrap().unwrap();
    let r = request(s, &p, assignment);
    apply(s, &Command::Schedule {
        placement: p,
        request: &r,
        request_ref: assignment,
    })
    .unwrap()
}
fn terminal(s: &State, assignment: Id, outcome: ExecutionOutcome) -> State {
    apply(s, &Command::Terminal {
        attempt: assignment,
        result_ref: id(20),
        result: ExecutionResult {
            request_ref: assignment,
            outcome,
            stopped: true,
            report: None,
        },
    })
    .unwrap()
}
#[test]
fn dag_executes_only_after_dependencies_and_reserves_capacity() {
    let s = submit(&initial(), "dag", id(5), vec![task("a", &[]), task("b", &["a"])]);
    let s = schedule(&s, id(10));
    assert!(next_placement(&s, None).unwrap().is_none());
    assert_eq!(s.usage(id(4)).unwrap(), resources());
    let s = terminal(&s, id(10), ExecutionOutcome::Succeeded { output: id(9) });
    let p = next_placement(&s, None).unwrap().unwrap();
    assert_eq!(p.task, "b");
    assert_eq!(p.inputs, vec![id(9)]);
    assert_eq!(s.usage(id(4)).unwrap(), Resources::default());
    let s = schedule(&s, id(11));
    let s = terminal(&s, id(11), ExecutionOutcome::Succeeded { output: id(9) });
    assert_eq!(s.runs[&id(5)].phase(), RunPhase::Succeeded);
}
#[test]
fn placement_cursor_advances_and_wraps_within_one_run() {
    let s = submit(&initial(), "siblings", id(5), vec![task("a", &[]), task("z", &[])]);
    let first = next_placement(&s, None).unwrap().unwrap();
    assert_eq!(first.task, "a");
    let second = next_placement(&s, Some(PlacementCursor::at(&s, &first).unwrap())).unwrap().unwrap();
    assert_eq!(second.task, "z");
    assert_eq!(next_placement(&s, Some(PlacementCursor::at(&s, &second).unwrap())).unwrap(), Some(first));
}

#[test]
fn uncertain_attempt_holds_capacity_and_cannot_be_replaced() {
    let s = submit(&initial(), "uncertain", id(5), vec![task("a", &[])]);
    let s = schedule(&s, id(10));
    let s = apply(&s, &Command::Uncertain {
        attempt: id(10),
        request_ref: id(10),
    })
    .unwrap();
    assert_eq!(s.usage(id(4)).unwrap(), resources());
    assert!(next_placement(&s, None).unwrap().is_none());
}
#[test]
fn retry_requires_stopped_terminal_and_advanced_epoch() {
    let s = submit(&initial(), "retry", id(5), vec![task("a", &[])]);
    let s = schedule(&s, id(10));
    let no_stop = Command::Terminal {
        attempt: id(10),
        result_ref: id(20),
        result: ExecutionResult {
            request_ref: id(10),
            outcome: ExecutionOutcome::Failed {
                code: FailureCode::Trap,
            },
            stopped: false,
            report: None,
        },
    };
    assert!(apply(&s, &no_stop).is_err());
    let s = terminal(&s, id(10), ExecutionOutcome::Failed {
        code: FailureCode::Trap,
    });
    assert!(next_placement(&s, None).unwrap().is_none());
    let s = apply(&s, &Command::Tick).unwrap();
    let p = next_placement(&s, None).unwrap().unwrap();
    assert_eq!(p.number, 2);
    let s = schedule(&s, id(11));
    assert!(apply(&s, &Command::Terminal {
        attempt: id(11),
        result_ref: id(20),
        result: ExecutionResult {
            request_ref: id(10),
            outcome: ExecutionOutcome::Succeeded { output: id(9) },
            stopped: true,
            report: None,
        }
    })
    .is_err());
}
#[test]
fn cancel_retains_active_reservation_until_teardown() {
    let s = submit(&initial(), "cancel", id(5), vec![task("a", &[]), task("b", &["a"])]);
    let s = schedule(&s, id(10));
    let s = apply(&s, &Command::CancelRun { run: id(5) }).unwrap();
    assert_eq!(s.runs[&id(5)].phase(), RunPhase::Cancelling);
    assert_eq!(s.usage(id(4)).unwrap(), resources());
    let s = terminal(&s, id(10), ExecutionOutcome::Cancelled);
    assert_eq!(s.runs[&id(5)].phase(), RunPhase::Cancelled);
}
#[test]
fn failed_dependency_blocks_children() {
    let mut first = task("a", &[]);
    first.max_attempts = 1;
    let s = submit(&initial(), "failed", id(5), vec![first, task("b", &["a"])]);
    let s = schedule(&s, id(10));
    let s = terminal(&s, id(10), ExecutionOutcome::Failed {
        code: FailureCode::Trap,
    });
    assert_eq!(s.runs[&id(5)].tasks["b"].phase, TaskPhase::Blocked);
    assert_eq!(s.runs[&id(5)].phase(), RunPhase::Failed);
}
#[test]
fn duplicate_submit_is_idempotent_but_key_rebinding_denies() {
    let tasks = vec![task("a", &[])];
    let s = submit(&initial(), "key", id(5), tasks.clone());
    assert_eq!(submit(&s, "key", id(5), tasks), s);
    assert!(apply(&s, &Command::Submit {
        key: "key".into(),
        spec_ref: id(99),
        run: id(5),
        spec: s.runs[&id(5)].spec.clone()
    })
    .is_err());
}
#[test]
fn cycle_missing_dependency_and_duplicate_task_are_denied() {
    assert!(validate_job(&JobSpec {
        name: "cycle".into(),
        tasks: vec![task("a", &["b"]), task("b", &["a"])]
    })
    .is_err());
    assert!(validate_job(&JobSpec {
        name: "missing".into(),
        tasks: vec![task("a", &["z"])]
    })
    .is_err());
    assert!(validate_job(&JobSpec {
        name: "duplicate".into(),
        tasks: vec![task("a", &[]), task("a", &[])]
    })
    .is_err());
    assert!(validate_job(&JobSpec {
        name: "unsorted".into(),
        tasks: vec![task("z", &[]), task("a", &[])],
    })
    .is_err());
}
#[test]
fn stale_and_drained_nodes_are_not_eligible() {
    let s = submit(&initial(), "stale", id(5), vec![task("a", &[])]);
    let drained = apply(&s, &Command::Drain {
        node: id(4),
        draining: true,
    })
    .unwrap();
    assert!(next_placement(&drained, None).unwrap().is_none());
    let mut stale = s;
    for _ in 0..4 {
        stale = apply(&stale, &Command::Tick).unwrap();
    }
    assert!(next_placement(&stale, None).unwrap().is_none());
}
#[test]
fn restore_rejects_missing_history_and_false_resource_release() {
    let s = schedule(&submit(&initial(), "restore", id(5), vec![task("a", &[])]), id(10));
    let mut bad = s.clone();
    bad.attempts.clear();
    assert!(validate_state(&bad).is_err());
    let mut bad = s;
    bad.attempts.get_mut(&id(10)).unwrap().reservation_held = false;
    assert!(validate_state(&bad).is_err());
}
fn worker_and_request() -> (WorkerState, ExecutionRequest) {
    let state = WorkerState::empty(id(1), id(4), 1, id(2), resources()).unwrap();
    let state = worker_apply(&state, &WorkerCommand::RecoverAfterProcessExit).unwrap();
    let s = submit(&initial(), "worker", id(5), vec![task("a", &[])]);
    let p = next_placement(&s, None).unwrap().unwrap();
    (state, request(&s, &p, id(10)))
}
#[test]
fn cancellation_tombstone_blocks_delayed_start() {
    let (s, r) = worker_and_request();
    let s = worker_apply(&s, &WorkerCommand::Cancel {
        request: r.clone(),
        request_ref: id(10),
    })
    .unwrap();
    let delayed = worker_apply(&s, &WorkerCommand::Accept {
        request: r,
        request_ref: id(10),
    })
    .unwrap();
    assert_eq!(delayed, s);
    assert!(matches!(s.records[&id(10)].phase, WorkerPhase::Terminal(_)));
}
#[test]
fn worker_duplicate_and_capacity_rejection_do_not_start_twice() {
    let (s, r) = worker_and_request();
    let command = WorkerCommand::Accept {
        request: r.clone(),
        request_ref: id(10),
    };
    let s = worker_apply(&s, &command).unwrap();
    assert_eq!(worker_apply(&s, &command).unwrap(), s);
    let mut other = r;
    other.fence.assignment = id(11);
    let s = worker_apply(&s, &WorkerCommand::Accept {
        request: other,
        request_ref: id(11),
    })
    .unwrap();
    assert!(matches!(s.records[&id(11)].phase, WorkerPhase::Terminal(_)));
}
#[test]
fn worker_restart_preserves_tombstones_and_stops_in_process_work() {
    let (s, r) = worker_and_request();
    let s = worker_apply(&s, &WorkerCommand::Accept {
        request: r,
        request_ref: id(10),
    })
    .unwrap();
    let s = worker_apply(&s, &WorkerCommand::Begin {
        attempt: id(10),
        request_ref: id(10),
        boot: 1,
    })
    .unwrap();
    let restarted = worker_apply(&s, &WorkerCommand::RecoverAfterProcessExit).unwrap();
    assert_eq!(restarted.boot, 2);
    assert_eq!(restarted.usage().unwrap(), Resources::default());
    assert!(matches!(
        restarted.records[&id(10)].phase,
        WorkerPhase::Terminal(ExecutionResult {
            outcome: ExecutionOutcome::Failed {
                code: FailureCode::Interrupted
            },
            stopped: true,
            ..
        })
    ));
}
#[test]
fn late_success_cannot_override_committed_cancel() {
    let (s, r) = worker_and_request();
    let s = worker_apply(&s, &WorkerCommand::Accept {
        request: r.clone(),
        request_ref: id(10),
    })
    .unwrap();
    let s = worker_apply(&s, &WorkerCommand::Cancel {
        request: r,
        request_ref: id(10),
    })
    .unwrap();
    assert!(worker_apply(&s, &WorkerCommand::Finish {
        attempt: id(10),
        result: ExecutionResult {
            request_ref: id(10),
            outcome: ExecutionOutcome::Succeeded { output: id(9) },
            stopped: true,
            report: None,
        }
    })
    .is_err());
}
#[test]
fn independent_pending_tasks_prevent_premature_terminal_run_status() {
    let mut first = task("a", &[]);
    first.max_attempts = 1;
    let s = submit(&initial(), "independent", id(5), vec![first, task("b", &[])]);
    let s = schedule(&s, id(10));
    let s = terminal(&s, id(10), ExecutionOutcome::Failed {
        code: FailureCode::Trap,
    });
    assert_eq!(s.runs[&id(5)].phase(), RunPhase::Pending);
    assert_eq!(next_placement(&s, None).unwrap().unwrap().task, "b");
    let s = schedule(&s, id(11));
    let s = terminal(&s, id(11), ExecutionOutcome::Succeeded { output: id(9) });
    assert_eq!(s.runs[&id(5)].phase(), RunPhase::Failed);
}

fn policy(profile: &str) -> Policy {
    Policy {
        profile: profile.into(),
        cluster: id(1),
        generation: 1,
        allow_any_pure_component: false,
        allowed_components: Default::default(),
        allowed_pure_net_manifests: Default::default(),
        allowed_pure_net_programs: Default::default(),
        max_memory_bytes: 65536,
        max_fuel: 10000,
        max_timeout_ms: 1000,
        max_output_bytes: 1024,
    }
}

#[test]
fn native_authority_requires_profile_and_both_exact_unrevoked_grants() {
    let mut native = task("native", &[]);
    native.workload = Workload::PureNet {
        manifest: id(7),
        artifact: id(8),
    };
    let mut component = policy(PROFILE);
    component.allow_any_pure_component = true;
    assert!(policy_allows_task(&component, &Default::default(), &task("wasm", &[])).unwrap());
    assert!(!policy_allows_task(&component, &Default::default(), &native).unwrap());
    component.allowed_pure_net_manifests.insert(id(7));
    assert!(validate_policy(&component).is_err());

    let mut p = policy(PURE_NET_PROFILE);
    assert!(!policy_allows_task(&p, &Default::default(), &native).unwrap());
    p.allowed_pure_net_manifests.insert(id(7));
    assert!(!policy_allows_task(&p, &Default::default(), &native).unwrap());
    p.allowed_pure_net_programs.insert(id(9));
    assert!(!policy_allows_task(&p, &Default::default(), &native).unwrap());
    p.allowed_pure_net_programs.insert(id(8));
    assert!(policy_allows_task(&p, &Default::default(), &native).unwrap());
    assert!(!policy_allows_task(&p, &Default::default(), &task("wasm", &[])).unwrap());
    for revoked in [id(7), id(8)] {
        assert!(!policy_allows_task(&p, &[revoked].into(), &native).unwrap());
    }
    p.allow_any_pure_component = true;
    assert!(validate_policy(&p).is_err());
}

#[test]
fn native_revocation_denies_execution_but_not_stopped_reconciliation() {
    let (_, mut r) = worker_and_request();
    r.workload = Workload::PureNet {
        manifest: id(7),
        artifact: id(8),
    };
    let mut p = policy(PURE_NET_PROFILE);
    p.allowed_pure_net_manifests.insert(id(7));
    p.allowed_pure_net_programs.insert(id(8));
    assert!(policy_allows_request(&p, &Default::default(), &r, true).unwrap());
    for revoked in [id(7), id(8)] {
        let revoked = [revoked].into();
        assert!(!policy_allows_request(&p, &revoked, &r, true).unwrap());
        assert!(policy_allows_request(&p, &revoked, &r, false).unwrap());
    }
    r.fence.generation += 1;
    assert!(!policy_allows_request(&p, &Default::default(), &r, true).unwrap());
    assert!(!policy_allows_request(&p, &Default::default(), &r, false).unwrap());
}

#[test]
fn request_attempt_bound_is_finite_and_bound_to_originating_task() {
    let s = submit(&initial(), "bounded", id(5), vec![task("a", &[])]);
    let p = next_placement(&s, None).unwrap().unwrap();
    let r = request(&s, &p, id(10));
    for max_attempts in [0, MAX_ATTEMPTS + 1] {
        let bad = ExecutionRequest {
            max_attempts,
            ..r.clone()
        };
        assert!(validate_request(&bad).is_err());
    }
    let mut bad = r.clone();
    bad.fence.epoch = bad.max_attempts + 1;
    assert!(validate_request(&bad).is_err());
    let bad = ExecutionRequest {
        max_attempts: r.max_attempts + 1,
        ..r.clone()
    };
    assert!(apply(&s, &Command::Schedule {
        placement: p,
        request: &bad,
        request_ref: id(10)
    })
    .is_err());
    let scheduled = schedule(&s, id(10));
    let mut restored = scheduled;
    restored.attempts.get_mut(&id(10)).unwrap().request.max_attempts += 1;
    assert!(validate_state(&restored).is_err());
}

#[test]
fn native_running_cancel_wins_late_success_and_duplicates_reconcile() {
    let (s, mut r) = worker_and_request();
    r.workload = Workload::PureNet {
        manifest: id(7),
        artifact: id(8),
    };
    let accept = WorkerCommand::Accept {
        request: r.clone(),
        request_ref: id(10),
    };
    let s = worker_apply(&s, &accept).unwrap();
    assert_eq!(worker_apply(&s, &accept).unwrap(), s);
    let s = worker_apply(&s, &WorkerCommand::Begin {
        attempt: id(10),
        request_ref: id(10),
        boot: s.boot,
    })
    .unwrap();
    let s = worker_apply(&s, &WorkerCommand::Cancel {
        request: r,
        request_ref: id(10),
    })
    .unwrap();
    assert_eq!(s.usage().unwrap(), resources());
    let s = worker_apply(&s, &WorkerCommand::Finish {
        attempt: id(10),
        result: ExecutionResult {
            request_ref: id(10),
            outcome: ExecutionOutcome::Succeeded { output: id(9) },
            stopped: true,
            report: None,
        },
    })
    .unwrap();
    assert!(matches!(
        s.records[&id(10)].phase,
        WorkerPhase::Terminal(ExecutionResult {
            outcome: ExecutionOutcome::Cancelled,
            ..
        })
    ));
    assert_eq!(s.usage().unwrap(), Resources::default());
    assert_eq!(worker_apply(&s, &accept).unwrap(), s);
}

#[test]
fn native_request_accepts_exact_carrier_limits_and_rejects_one_extra_input() {
    let (_, mut request) = worker_and_request();
    request.workload = Workload::PureNet {
        manifest: id(7),
        artifact: id(8),
    };
    request.max_attempts = MAX_ATTEMPTS;
    request.fence.epoch = MAX_ATTEMPTS;
    request.max_output_bytes = MAX_CONTENT_BYTES as u64;
    request.inputs = vec![id(9); MAX_INPUTS];
    validate_request(&request).unwrap();
    request.inputs.push(id(9));
    assert!(validate_request(&request).is_err());
    request.inputs.pop();
    request.max_output_bytes += 1;
    assert!(validate_request(&request).is_err());
}
