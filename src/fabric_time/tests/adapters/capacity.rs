// r[verify molten.audit_f09.validation]
// r[verify molten.audit_f10.validation]
#[test]
fn resume_and_yield_at_the_active_limit_before_host_wakes() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let admitted = profile(kind, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
        let key = key("resumed");
        let harness = Harness::new(&admitted.profile, GENERATION, std::slice::from_ref(&key));
        let backing = harness.owner.lock().expect("owner").backing_capacity();
        harness.wake(&key, 0);
        harness.usage(1, 1);
        harness.choose(&key).expect("choose initial work");
        harness.usage(1, 0);
        harness.command(SchedulerCommand::Block { key: key.clone() }).expect("block");
        harness.usage(1, 0);
        let running = harness.snapshot();
        assert!(matches!(harness.owner.lock().expect("owner").release(), Err(SchedulerRuntimeError::Admission(_))));
        assert_eq!(harness.snapshot(), running);
        harness.no_delivery();
        let blocked = harness.snapshot();
        assert_eq!(harness.command(SchedulerCommand::Wake { key: self::key("extra"), priority: 0 }).expect("active overload"), SchedulerAction::RejectedOverload);
        assert_eq!(harness.snapshot(), blocked);
        harness.no_delivery();
        harness.wake(&key, RESUME_PRIORITY);
        let resumed = harness.snapshot();
        assert_eq!(resumed.0.runnables.len(), 1);
        assert_eq!(resumed.0.runnables[0].enqueue_sequence, blocked.0.next_enqueue_sequence);
        assert_eq!(resumed.0.runnables[0].priority, RESUME_PRIORITY);
        assert_eq!(resumed.0.runnables[0].wait_turns, 0);
        harness.usage(1, 1);
        harness.choose(&key).expect("select resumed occurrence");
        harness.usage(1, 0);
        assert_eq!(harness.command(SchedulerCommand::Yield { key: key.clone() }).expect("yield at active limit"), SchedulerAction::Yielded);
        harness.consume(&key);
        harness.usage(1, 1);
        let transition = SchedulerTransition { next: harness.snapshot().0, action: SchedulerAction::Yielded, runnable: key.clone() };
        assert_eq!(event_action(&scheduler_transition(&admitted.profile.profile_ref, &transition).expect("canonical action")), "yielded");
        assert_eq!(harness.owner.lock().expect("owner").backing_capacity(), backing);
    }
}

// r[verify molten.audit_f09.validation]
// r[verify molten.audit_f10.validation]
#[test]
fn preserve_saturated_resume_and_yield_for_both_overload_policies() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        for (overload, action, canonical) in [
            (SchedulerOverloadPolicy::Reject, SchedulerAction::RejectedOverload, "rejected-overload"),
            (SchedulerOverloadPolicy::Backpressure, SchedulerAction::Backpressure, "backpressure"),
        ] {
            for is_resume in [false, true] {
                let admitted = profile(kind, overload, TWO_SLOTS, SINGLE_SLOT);
                let key = key("owner");
                let queued = self::key("queued");
                let harness = Harness::new(&admitted.profile, GENERATION, &[key.clone(), queued.clone()]);
                harness.wake(&key, 0);
                harness.choose(&key).expect("choose owner");
                if is_resume { harness.command(SchedulerCommand::Block { key: key.clone() }).expect("block owner"); }
                harness.wake(&queued, RESUME_PRIORITY);
                harness.usage(2, 1);
                let before = harness.snapshot();
                let backing = harness.owner.lock().expect("owner").backing_capacity();
                let command = if is_resume { SchedulerCommand::Wake { key: key.clone(), priority: RESUME_PRIORITY } } else { SchedulerCommand::Yield { key: key.clone() } };
                assert_eq!(harness.command(command.clone()).expect("typed overload"), action);
                assert_eq!(harness.snapshot(), before);
                assert_eq!(harness.owner.lock().expect("owner").backing_capacity(), backing);
                harness.no_delivery();
                let transition = SchedulerTransition { next: before.0.clone(), action, runnable: key.clone() };
                assert_eq!(event_action(&scheduler_transition(&admitted.profile.profile_ref, &transition).expect("canonical denial")), canonical);
                assert_eq!(before.0.runnables[0].phase, if is_resume { RunnablePhase::Blocked } else { RunnablePhase::Running });
                harness.command(SchedulerCommand::Cancel { key: queued }).expect("explicitly free Ready space");
                harness.usage(1, 0);
                assert_eq!(harness.command(command).expect("explicit later admission"), if is_resume { SchedulerAction::Woken } else { SchedulerAction::Yielded });
                harness.consume(&key);
                harness.usage(1, 1);
                assert_eq!(harness.snapshot().0.runnables[0].enqueue_sequence, before.0.next_enqueue_sequence);
            }
        }
    }
}

#[test]
fn terminal_history_cleanup_release_and_physical_exhaustion_stay_distinct() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        for overload in [SchedulerOverloadPolicy::Reject, SchedulerOverloadPolicy::Backpressure] {
            let admitted = profile(kind, overload, TWO_SLOTS, TWO_SLOTS);
            let first = key("first");
            let second = key("second");
            let harness = Harness::new(&admitted.profile, GENERATION, &[first.clone(), second.clone()]);
            harness.wake(&first, 0);
            harness.choose(&first).expect("select");
            harness.command(SchedulerCommand::Complete { key: first.clone() }).expect("complete");
            harness.usage(0, 0);
            harness.wake(&second, 0);
            harness.owner.lock().expect("owner").cleanup_generation(GENERATION).expect("cleanup active generation");
            harness.usage(0, 0);
            let before = harness.snapshot();
            assert_eq!(before.0.runnables[0].phase, RunnablePhase::Completed);
            assert_eq!(before.0.runnables[1].phase, RunnablePhase::Cancelled);
            for retired in [first, second] {
                assert!(matches!(harness.command(SchedulerCommand::Wake { key: retired, priority: 0 }), Err(SchedulerRuntimeError::Core(SchedulerError::DuplicateRunnable(_)))));
                assert_eq!(harness.snapshot(), before);
            }
            let expected = match overload { SchedulerOverloadPolicy::Reject => SchedulerAction::RejectedOverload, SchedulerOverloadPolicy::Backpressure => SchedulerAction::Backpressure };
            assert_eq!(harness.command(SchedulerCommand::Wake { key: key("third"), priority: 0 }).expect("physical backing overload"), expected);
            assert_eq!(harness.snapshot(), before);
            harness.no_delivery();
            harness.owner.lock().expect("owner").release().expect("quiescent release");
            assert_eq!(harness.snapshot().0, before.0);
            assert!(harness.snapshot().1.is_released);
            assert!(matches!(harness.command(SchedulerCommand::Wake { key: key("after-release"), priority: 0 }), Err(SchedulerRuntimeError::Capacity(molten_core::fabric_time::capacity::UseDecisionKind::Released))));
            harness.no_delivery();
        }
    }
}
