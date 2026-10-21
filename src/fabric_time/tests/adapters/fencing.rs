// r[verify molten.audit_f09.validation]
#[test]
fn resume_refreshes_fifo_priority_and_wait_accounting() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let admitted = profile(kind, SchedulerOverloadPolicy::Reject, TWO_SLOTS, TWO_SLOTS);
        let older = key("older", 1);
        let resumed = key("resumed", 2);
        let harness = Harness::new(&admitted.profile, GENERATION, &[older.clone(), resumed.clone()]);
        harness.wake(&older, 0);
        harness.wake(&resumed, 0);
        harness.choose(&older).expect("select older");
        harness.command(SchedulerCommand::Block { key: resumed.clone() }).expect("block waiting work");
        assert_eq!(harness.snapshot().0.runnables[1].as_ref().expect("resumed").wait_turns, 1);
        harness.command(SchedulerCommand::Yield { key: older.clone() }).expect("yield older");
        harness.consume(&older);
        let before = harness.snapshot();
        harness.wake(&resumed, RESUME_PRIORITY);
        let after = harness.snapshot();
        let resumed_row = after.0.runnables.iter().flatten().find(|row| row.key == resumed).expect("resumed");
        assert_eq!(resumed_row.wait_turns, 0);
        assert_eq!(resumed_row.priority, RESUME_PRIORITY);
        assert_eq!(resumed_row.enqueue_sequence, before.0.next_enqueue_sequence);
        let choice = harness.choose(&older).expect("fresh FIFO position wins");
        assert_eq!(harness.snapshot().0.runnables[choice.index].as_ref().expect("chosen").key, older);
        harness.usage(2, 1);
    }
}

// r[verify molten.audit_f09.validation]
// r[verify molten.audit_f10.validation]
#[test]
fn fence_stale_callbacks_without_current_generation_wakes() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let admitted = profile(kind, SchedulerOverloadPolicy::Reject, TWO_SLOTS, SINGLE_SLOT);
        let retired = key("same-id", 1);
        let current = RunnableKey { generation: STALE_GENERATION, ..retired.clone() };
        let harness = Harness::new(&admitted.profile, STALE_GENERATION, std::slice::from_ref(&current));
        harness.wake(&current, 0);
        harness.command(SchedulerCommand::Block { key: current.clone() }).expect("block current");
        let before = harness.snapshot();
        for command in [SchedulerCommand::Wake { key: retired.clone(), priority: RESUME_PRIORITY }, SchedulerCommand::Yield { key: retired }] {
            assert_eq!(harness.command(command).expect("discard stale"), SchedulerAction::DiscardedStaleGeneration);
            assert_eq!(harness.snapshot(), before);
            harness.no_delivery();
        }
        let mut wrong_service = current.clone();
        wrong_service.service_id = "other-service".to_string();
        assert!(matches!(harness.command(SchedulerCommand::Wake { key: wrong_service, priority: 0 }), Err(SchedulerRuntimeError::Admission(_))));
        assert_eq!(harness.snapshot(), before);
        harness.no_delivery();
        harness.wake(&current, RESUME_PRIORITY);
        let mut wrong_profile = admitted.profile.clone();
        wrong_profile.profile_id = "other-profile".to_string();
        let effects = SimulationSchedulerWakeAdapter::activate(&admitted.profile).expect("effects");
        assert!(matches!(SchedulerRuntime::activate(context(&admitted.profile, GENERATION), wrong_profile, effects), Err(SchedulerRuntimeError::Admission(_))));
        let capacity = harness.snapshot().1;
        let mut stale = capacity.clone();
        stale.generation = GENERATION;
        let mut wrong_profile = capacity.clone();
        wrong_profile.profile_ref = "wrong-profile".to_string();
        let mut wrong_plan = capacity.clone();
        wrong_plan.plan_ref = "wrong-plan".to_string();
        for (changed, expected) in [
            (stale, molten_core::fabric_time::capacity::UseDecisionKind::StaleGeneration),
            (wrong_profile, molten_core::fabric_time::capacity::UseDecisionKind::ProfileMismatch),
            (wrong_plan, molten_core::fabric_time::capacity::UseDecisionKind::PlanMismatch),
        ] {
            *harness.owner.lock().expect("owner").capacity_mut_for_test() = changed;
            let before = harness.snapshot();
            assert!(matches!(harness.command(SchedulerCommand::Cancel { key: current.clone() }), Err(SchedulerRuntimeError::Capacity(kind)) if kind == expected));
            assert_eq!(harness.snapshot(), before);
            harness.no_delivery();
        }
        *harness.owner.lock().expect("owner").capacity_mut_for_test() = capacity;
        harness.command(SchedulerCommand::Cancel { key: current }).expect("matching capacity remains usable");
        harness.usage(0, 0);
    }
}

// r[verify molten.audit_f09.validation]
// r[verify molten.audit_f10.validation]
#[test]
fn deny_invalid_and_overflowing_ready_transitions() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let admitted = profile(kind, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
        let key = key("owner", 1);
        let harness = Harness::new(&admitted.profile, GENERATION, std::slice::from_ref(&key));
        harness.wake(&key, 0);
        let before = harness.snapshot();
        for command in [SchedulerCommand::Wake { key: key.clone(), priority: RESUME_PRIORITY }, SchedulerCommand::Yield { key: key.clone() }, SchedulerCommand::Complete { key: key.clone() }] {
            assert!(harness.command(command).is_err());
            assert_eq!(harness.snapshot(), before);
            harness.no_delivery();
        }
        harness.owner.lock().expect("owner").state_mut_for_test().choice_sequence = u64::MAX;
        let before = harness.snapshot();
        assert!(matches!(harness.choose(&key), Err(SchedulerRuntimeError::Core(SchedulerError::Overflow))));
        assert_eq!(harness.snapshot(), before);
        harness.no_delivery();
        harness.owner.lock().expect("owner").state_mut_for_test().choice_sequence = 0;
        harness.choose(&key).expect("valid selection after errors");
        for is_resume in [false, true] {
            if is_resume { harness.command(SchedulerCommand::Block { key: key.clone() }).expect("block"); }
            harness.owner.lock().expect("owner").state_mut_for_test().next_enqueue_sequence = u64::MAX;
            let before = harness.snapshot();
            let command = if is_resume { SchedulerCommand::Wake { key: key.clone(), priority: RESUME_PRIORITY } } else { SchedulerCommand::Yield { key: key.clone() } };
            assert!(matches!(harness.command(command), Err(SchedulerRuntimeError::Core(SchedulerError::Overflow))));
            assert_eq!(harness.snapshot(), before);
            harness.no_delivery();
        }
        harness.owner.lock().expect("owner").state_mut_for_test().next_enqueue_sequence = 1;
        harness.wake(&key, RESUME_PRIORITY);
    }
}

#[test]
fn replay_errors_preserve_owned_readiness_before_valid_selection() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let mut admitted = profile(kind, SchedulerOverloadPolicy::Reject, TWO_SLOTS, TWO_SLOTS);
        admitted.profile.scheduler_policy.replay = SchedulerReplayPolicy::Deterministic;
        let first = key("first", 1);
        let second = key("second", 2);
        let harness = Harness::new(&admitted.profile, GENERATION, &[first.clone(), second.clone()]);
        harness.wake(&first, 0);
        harness.wake(&second, RESUME_PRIORITY);
        let before = harness.snapshot();
        assert!(matches!(harness.choose(&second), Err(SchedulerRuntimeError::Core(SchedulerError::UnexpectedReplayChoice(_)))));
        assert_eq!(harness.snapshot(), before);
        harness.no_delivery();
        harness.choose(&first).expect("original FIFO remains authoritative");
        harness.usage(2, 1);
    }
    let mut admitted = profile(TimeProfileKind::DeterministicSimulation, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
    admitted.profile.scheduler_policy.replay = SchedulerReplayPolicy::RecordedChoiceRequired;
    let key = key("recorded", 1);
    let harness = Harness::new(&admitted.profile, GENERATION, std::slice::from_ref(&key));
    harness.wake(&key, 0);
    let before = harness.snapshot();
    assert!(matches!(harness.owner.lock().expect("owner").choose(None), Err(SchedulerRuntimeError::Core(SchedulerError::ReplayChoiceRequired))));
    assert_eq!(harness.snapshot(), before);
    harness.no_delivery();
    harness.choose(&key).expect("explicit recorded choice");
}

#[test]
fn fallible_delivery_preparation_never_half_publishes() {
    let admitted = profile(TimeProfileKind::Live, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
    let key = key("target", 1);
    let mut owner = SchedulerRuntime::activate(context(&admitted.profile, GENERATION), admitted.profile.clone(), ThreadSchedulerWakeAdapter::default()).expect("activate");
    let before = (owner.state().clone(), owner.capacity_state().clone());
    assert!(matches!(owner.command(SchedulerCommand::Wake { key: key.clone(), priority: 0 }), Err(SchedulerRuntimeError::Prepare(_))));
    assert_eq!((owner.state().clone(), owner.capacity_state().clone()), before);
    let worker = std::thread::spawn(std::thread::park);
    owner.effects_mut().register(key.clone(), worker.thread().clone()).expect("register");
    let result = owner.command(SchedulerCommand::Wake { key, priority: 0 });
    if result.is_err() { worker.thread().unpark(); }
    worker.join().expect("join target");
    assert_eq!(result.expect("admission after explicit registration"), SchedulerAction::Woken);

    let admitted = profile(TimeProfileKind::DeterministicSimulation, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
    let mut owner = SchedulerRuntime::activate(context(&admitted.profile, GENERATION), admitted.profile.clone(), SimulationSchedulerWakeAdapter::activate(&admitted.profile).expect("simulation allocation")).expect("activate");
    let key = self::key("pending", 1);
    owner.command(SchedulerCommand::Wake { key: key.clone(), priority: 0 }).expect("first delivery pending");
    owner.command(SchedulerCommand::Block { key: key.clone() }).expect("block before drain");
    let before = (owner.state().clone(), owner.capacity_state().clone());
    assert!(matches!(owner.command(SchedulerCommand::Wake { key: key.clone(), priority: 0 }), Err(SchedulerRuntimeError::Prepare(_))));
    assert_eq!((owner.state().clone(), owner.capacity_state().clone()), before);
    let token = owner.effects_mut().drain_next().expect("pending delivery");
    assert_eq!(token, SchedulerWakeToken { occurrence: 0, generation: GENERATION, occurrence_sequence: key.occurrence_sequence });
    assert!(owner.resolve_wake(token).is_none(), "blocked occurrences cannot consume deferred wake tokens");
    assert_eq!(owner.effects_mut().drain_next(), None);
    owner.command(SchedulerCommand::Wake { key, priority: 0 }).expect("explicit resume after consumer drain");
    assert_eq!(owner.effects_mut().drain_next(), Some(token));
    assert_eq!(owner.resolve_wake(token).expect("committed resume token").phase, RunnablePhase::Ready);
}

#[test]
fn post_commit_uncertainty_is_not_denial_or_automatic_retry() {
    struct UncertainDelivery(std::sync::mpsc::SyncSender<SchedulerWakeToken>);
    impl SchedulerWakePort for UncertainDelivery {
        type Prepared = SchedulerWakeToken;
        fn prepare(&mut self, key: &RunnableKey, index: usize) -> crate::fabric::FabricPortResult<SchedulerWakeToken> {
            Ok(SchedulerWakeToken { occurrence: index, generation: key.generation, occurrence_sequence: key.occurrence_sequence })
        }
        fn deliver(&mut self, token: SchedulerWakeToken, committed: SchedulerWakeCommit<'_>) -> crate::fabric::FabricPortResult<()> {
            assert!(committed.accepts(token));
            self.0.send(token).expect("receiving transport");
            Err(crate::fabric::FabricPortError::uncertain("acknowledgement lost after delivery"))
        }
    }
    let admitted = profile(TimeProfileKind::DeterministicSimulation, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
    let key = key("uncertain", 1);
    // One committed effect is consumed before the duplicate command is attempted.
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let mut owner = SchedulerRuntime::activate(context(&admitted.profile, GENERATION), admitted.profile, UncertainDelivery(sender)).expect("activate");
    assert!(matches!(owner.command(SchedulerCommand::Wake { key: key.clone(), priority: 0 }), Err(SchedulerRuntimeError::CommittedEffectUncertain(_))));
    let delivered = receiver.recv_timeout(WAKE_TIMEOUT).expect("independent receiver got effect despite uncertainty");
    assert_eq!(owner.resolve_wake(delivered).expect("committed token").key, key);
    assert_eq!(owner.resolve_wake(delivered).expect("committed token").phase, RunnablePhase::Ready);
    assert_eq!((owner.capacity_state().runnable_usage, owner.capacity_state().queue_usage), (1, 1));
    let committed = (owner.state().clone(), owner.capacity_state().clone());
    assert!(matches!(owner.command(SchedulerCommand::Wake { key, priority: 0 }), Err(SchedulerRuntimeError::Core(SchedulerError::DuplicateRunnable(_)))));
    assert_eq!((owner.state().clone(), owner.capacity_state().clone()), committed);
    assert!(matches!(receiver.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
}

