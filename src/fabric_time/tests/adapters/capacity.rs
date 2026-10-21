// r[verify molten.audit_f09.validation]
// r[verify molten.audit_f10.validation]
#[test]
fn resume_and_yield_at_the_active_limit_before_host_wakes() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let admitted = profile(kind, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
        let key = key("resumed", 1);
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
        assert_eq!(harness.command(SchedulerCommand::Wake { key: self::key("extra", 2), priority: 0 }).expect("active overload"), SchedulerAction::RejectedOverload);
        assert_eq!(harness.snapshot(), blocked);
        harness.no_delivery();
        harness.wake(&key, RESUME_PRIORITY);
        let resumed = harness.snapshot();
        assert_eq!(resumed.0.runnables.len(), 1);
        let resumed_row = resumed.0.runnables.iter().flatten().find(|row| row.key == key).expect("resumed occurrence");
        assert_eq!(resumed_row.enqueue_sequence, blocked.0.next_enqueue_sequence);
        assert_eq!(resumed_row.priority, RESUME_PRIORITY);
        assert_eq!(resumed_row.wait_turns, 0);
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
                let key = key("owner", 1);
                let queued = self::key("queued", 2);
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
                assert_eq!(before.0.runnables.iter().flatten().find(|row| row.key == key).expect("owner").phase, if is_resume { RunnablePhase::Blocked } else { RunnablePhase::Running });
                harness.command(SchedulerCommand::Cancel { key: queued }).expect("explicitly free Ready space");
                harness.usage(1, 0);
                assert_eq!(harness.command(command).expect("explicit later admission"), if is_resume { SchedulerAction::Woken } else { SchedulerAction::Yielded });
                harness.consume(&key);
                harness.usage(1, 1);
                assert_eq!(harness.snapshot().0.runnables.iter().flatten().find(|row| row.key == key).expect("owner").enqueue_sequence, before.0.next_enqueue_sequence);
            }
        }
    }
}

// r[verify molten.audit_f11.validation]
#[test]
fn terminal_retention_reuses_backing_across_completion_cancel_and_cleanup() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        for overload in [SchedulerOverloadPolicy::Reject, SchedulerOverloadPolicy::Backpressure] {
            let admitted = profile(kind, overload, SINGLE_SLOT, SINGLE_SLOT);
            let harness = Harness::new(&admitted.profile, GENERATION, &[]);
            let backing = harness.owner.lock().expect("owner").backing_capacity();
            for sequence in 1..=18 {
                let fresh = key("reused-logical-name", sequence);
                assert_eq!(harness.owner.lock().expect("owner").next_occurrence_sequence().expect("next issuance"), sequence);
                harness.register(&fresh);
                harness.wake(&fresh, 0);
                harness.usage(1, 1);
                if sequence >= 3 {
                    let old = key("reused-logical-name", 1);
                    let before = harness.snapshot();
                    assert!(!before.0.runnables.iter().flatten().any(|row| row.key == old), "old identity was evicted before exercising callbacks");
                    for command in [
                        SchedulerCommand::Wake { key: old.clone(), priority: RESUME_PRIORITY },
                        SchedulerCommand::Yield { key: old.clone() },
                        SchedulerCommand::Block { key: old.clone() },
                        SchedulerCommand::Complete { key: old.clone() },
                        SchedulerCommand::Cancel { key: old },
                    ] {
                        assert!(harness.command(command).is_err(), "evicted occurrence must not target fresh work");
                        assert_eq!(harness.snapshot(), before);
                    }
                }
                match sequence % 3 {
                    0 => { harness.owner.lock().expect("owner").cleanup_generation(GENERATION).expect("cleanup current generation"); }
                    1 => {
                        harness.choose(&fresh).expect("choose before completion");
                        harness.command(SchedulerCommand::Complete { key: fresh.clone() }).expect("complete fresh");
                    }
                    _ => { harness.command(SchedulerCommand::Cancel { key: fresh.clone() }).expect("cancel fresh"); }
                }
                harness.no_delivery();
                harness.usage(0, 0);
                let snapshot = harness.snapshot();
                assert!(snapshot.0.runnables.iter().flatten().filter(|row| matches!(row.phase, RunnablePhase::Completed | RunnablePhase::Cancelled)).count() <= 1);
                assert!(snapshot.0.runnables.len() <= 2);
                assert_eq!(harness.owner.lock().expect("owner").backing_capacity(), backing);
                harness.unregister(&fresh);
            }
            harness.owner.lock().expect("owner").release().expect("release quiescent owner");
            let before = harness.snapshot();
            assert!(before.1.is_released);
            assert!(matches!(harness.command(SchedulerCommand::Wake { key: key("after-release", 19), priority: 0 }), Err(SchedulerRuntimeError::Capacity(molten_core::fabric_time::capacity::UseDecisionKind::Released))));
            assert_eq!(harness.snapshot(), before);
        }
    }
}
