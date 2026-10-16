// r[verify molten.audit_f09.validation]
// r[verify molten.audit_f10.validation]
#[test]
fn resume_and_yield_at_the_active_limit_before_host_wakes() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let admitted = self::profile(kind, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
        let profile = &admitted.profile;
        let context = self::context(profile, GENERATION);
        let key = self::key("resumed");
        let initial = new_scheduler_state(profile, GENERATION);
        let woken = self::command(&context, profile, &initial, SchedulerCommand::Wake {
            key: key.clone(),
            priority: 0,
        });
        assert_eq!(event_action(&self::event(profile, &woken)), "woken");
        if kind == TimeProfileKind::Live {
            assert_live_wake(&woken);
        }
        let blocked = self::command(&context, profile, &woken.next, SchedulerCommand::Block { key: key.clone() });
        assert_eq!(blocked.action, SchedulerAction::Blocked);
        assert_denied_wake(&blocked);
        assert_eq!(event_action(&self::event(profile, &blocked)), "blocked");
        // The active envelope is full even though Block freed the only ready slot.
        assert!(
            context
                .apply_scheduler_command(profile, profile.scheduler_policy, &blocked.next, &SchedulerCommand::Wake {
                    key: self::key("extra"),
                    priority: 0,
                })
                .is_err()
        );
        let resumed = self::command(&context, profile, &blocked.next, SchedulerCommand::Wake {
            key: key.clone(),
            priority: RESUME_PRIORITY,
        });
        assert_eq!(resumed.action, SchedulerAction::Woken);
        let mut expected = blocked.next.clone();
        expected.runnables[0].phase = RunnablePhase::Ready;
        expected.runnables[0].priority = RESUME_PRIORITY;
        expected.runnables[0].enqueue_sequence = blocked.next.next_enqueue_sequence;
        expected.runnables[0].wait_turns = 0;
        expected.next_enqueue_sequence += 1;
        assert_eq!(resumed.next, expected);

        assert_eq!(event_action(&self::event(profile, &resumed)), "woken");
        if kind == TimeProfileKind::Live {
            assert_live_wake(&resumed);
        }
        let selected = context
            .choose_runnable(profile, profile.scheduler_policy, &resumed.next, Some(&key))
            .expect("select resumed occurrence");
        let yielded = self::command(&context, profile, &selected.next, SchedulerCommand::Yield { key });
        assert_eq!(yielded.action, SchedulerAction::Yielded);
        assert_eq!(yielded.next.runnables.len(), selected.next.runnables.len());
        assert_eq!(yielded.next.runnables[0].phase, RunnablePhase::Ready);
        assert_eq!(yielded.next.runnables[0].enqueue_sequence, selected.next.next_enqueue_sequence);

        assert_eq!(event_action(&self::event(profile, &yielded)), "yielded");
        if kind == TimeProfileKind::Live {
            assert_live_wake(&yielded);
        }
    }
}

// r[verify molten.audit_f09.validation]
// r[verify molten.audit_f10.validation]
#[test]
fn preserve_saturated_resume_and_yield_for_both_overload_policies() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        for (overload, action, canonical_action) in [
            (SchedulerOverloadPolicy::Reject, SchedulerAction::RejectedOverload, "rejected-overload"),
            (SchedulerOverloadPolicy::Backpressure, SchedulerAction::Backpressure, "backpressure"),
        ] {
            let admitted = self::profile(kind, overload, TWO_SLOTS, SINGLE_SLOT);
            let profile = &admitted.profile;
            let context = self::context(profile, GENERATION);
            let key = self::key("owner");
            let queued = self::key("queued");
            let initial = new_scheduler_state(profile, GENERATION);
            let woken = self::command(&context, profile, &initial, SchedulerCommand::Wake {
                key: key.clone(),
                priority: 0,
            });
            let selected = context
                .choose_runnable(profile, profile.scheduler_policy, &woken.next, Some(&key))
                .expect("select owner");
            let saturated = self::command(&context, profile, &selected.next, SchedulerCommand::Wake {
                key: queued.clone(),
                priority: RESUME_PRIORITY,
            })
            .next;
            let blocked =
                self::command(&context, profile, &saturated, SchedulerCommand::Block { key: key.clone() }).next;
            for (before, command, accepted_action, accepted_canonical) in [
                (saturated, SchedulerCommand::Yield { key: key.clone() }, SchedulerAction::Yielded, "yielded"),
                (
                    blocked,
                    SchedulerCommand::Wake {
                        key: key.clone(),
                        priority: RESUME_PRIORITY,
                    },
                    SchedulerAction::Woken,
                    "woken",
                ),
            ] {
                let denied = self::command(&context, profile, &before, command.clone());
                assert_eq!(denied.action, action);
                assert_eq!(denied.next, before);
                assert_eq!(denied.runnable, key);
                assert_denied_wake(&denied);
                let denial_event = self::event(profile, &denied);
                assert_eq!(event_action(&denial_event), canonical_action);

                // A denied Yield retains Running ownership. A denied resume remains Blocked.
                // Free only the competing ready slot, then retry the original command explicitly.
                let released = self::command(&context, profile, &denied.next, SchedulerCommand::Cancel {
                    key: queued.clone(),
                });
                let retried = self::command(&context, profile, &released.next, command);
                assert_eq!(retried.action, accepted_action);
                assert_eq!(event_action(&self::event(profile, &retried)), accepted_canonical);
                let owner = retried.next.runnables.iter().find(|runnable| runnable.key == key).expect("same owner");
                assert_eq!(owner.phase, RunnablePhase::Ready);
                assert_eq!(owner.enqueue_sequence, before.next_enqueue_sequence);
                assert_eq!(retried.next.runnables.len(), before.runnables.len());
                assert_eq!(retried.next.choice_sequence, before.choice_sequence);
                if kind == TimeProfileKind::Live {
                    assert_live_wake(&retried);
                }
            }
        }
    }
}