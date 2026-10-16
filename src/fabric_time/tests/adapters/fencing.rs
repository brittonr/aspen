// r[verify molten.audit_f09.validation]
#[test]
fn resume_refreshes_fifo_priority_and_wait_accounting() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let admitted = self::profile(kind, SchedulerOverloadPolicy::Reject, TWO_SLOTS, TWO_SLOTS);
        let profile = &admitted.profile;
        let context = self::context(profile, GENERATION);
        let older = self::key("older");
        let resumed_key = self::key("resumed");
        let first =
            self::command(&context, profile, &new_scheduler_state(profile, GENERATION), SchedulerCommand::Wake {
                key: older.clone(),
                priority: 0,
            });
        let second = self::command(&context, profile, &first.next, SchedulerCommand::Wake {
            key: resumed_key.clone(),
            priority: 0,
        });
        let selected = context
            .choose_runnable(profile, profile.scheduler_policy, &second.next, Some(&older))
            .expect("select older work");
        let blocked = self::command(&context, profile, &selected.next, SchedulerCommand::Block {
            key: resumed_key.clone(),
        });
        assert_eq!(blocked.next.runnables[1].wait_turns, 1);
        let yielded =
            self::command(&context, profile, &blocked.next, SchedulerCommand::Yield { key: older.clone() });
        let resumed = self::command(&context, profile, &yielded.next, SchedulerCommand::Wake {
            key: resumed_key.clone(),
            priority: RESUME_PRIORITY,
        });
        assert_eq!(resumed.next.runnables[1].wait_turns, 0);
        assert_eq!(resumed.next.runnables[1].priority, RESUME_PRIORITY);
        assert_eq!(resumed.next.runnables[1].enqueue_sequence, yielded.next.next_enqueue_sequence);
        assert_eq!(resumed.next.runnables.len(), blocked.next.runnables.len());
        // FIFO keeps older ready work ahead of the resume, despite its supplied higher priority.
        let choice = context
            .choose_runnable(profile, profile.scheduler_policy, &resumed.next, Some(&older))
            .expect("fresh FIFO position controls choice");
        assert_eq!(choice.selected, older);
        if kind == TimeProfileKind::Live {
            assert_live_wake(&resumed);
        }
    }
}

// r[verify molten.audit_f09.validation]
// r[verify molten.audit_f10.validation]
#[test]
fn fence_stale_callbacks_without_current_generation_wakes() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let admitted = self::profile(kind, SchedulerOverloadPolicy::Reject, TWO_SLOTS, SINGLE_SLOT);
        let profile = &admitted.profile;
        let current = self::context(profile, STALE_GENERATION);
        let retired = self::context(profile, GENERATION);
        let stale_key = self::key("same-id");
        let current_key = RunnableKey {
            generation: STALE_GENERATION,
            ..stale_key.clone()
        };
        let woken = self::command(
            &current,
            profile,
            &new_scheduler_state(profile, STALE_GENERATION),
            SchedulerCommand::Wake {
                key: current_key.clone(),
                priority: 0,
            },
        );
        let blocked = self::command(&current, profile, &woken.next, SchedulerCommand::Block {
            key: current_key.clone(),
        })
        .next;
        for command in [
            SchedulerCommand::Wake {
                key: stale_key.clone(),
                priority: RESUME_PRIORITY,
            },
            SchedulerCommand::Yield { key: stale_key.clone() },
        ] {
            assert!(retired.apply_scheduler_command(profile, profile.scheduler_policy, &blocked, &command).is_err());
            let discarded = self::command(&current, profile, &blocked, command);
            assert_eq!(discarded.action, SchedulerAction::DiscardedStaleGeneration);
            assert_eq!(discarded.next, blocked);
            assert_eq!(event_action(&self::event(profile, &discarded)), "discarded-stale-generation");
            // Even with a same-name current-generation target, stale transitions are not routed.
            let mut adapter = ThreadSchedulerWakeAdapter::default();
            adapter.register(current_key.clone(), std::thread::current()).expect("current target");
            assert!(adapter.route(&discarded).is_err());
            assert!(adapter.unregister(&current_key));
        }
        let resumed = self::command(&current, profile, &blocked, SchedulerCommand::Wake {
            key: current_key.clone(),
            priority: RESUME_PRIORITY,
        });
        assert_eq!(resumed.runnable, current_key);
        assert_eq!(resumed.action, SchedulerAction::Woken);
        if kind == TimeProfileKind::Live {
            assert_live_wake(&resumed);
        }
    }
}

// r[verify molten.audit_f09.validation]
// r[verify molten.audit_f10.validation]
#[test]
fn deny_invalid_and_overflowing_ready_transitions() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let admitted = self::profile(kind, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
        let profile = &admitted.profile;
        let context = self::context(profile, GENERATION);
        let key = self::key("owner");
        let woken =
            self::command(&context, profile, &new_scheduler_state(profile, GENERATION), SchedulerCommand::Wake {
                key: key.clone(),
                priority: 0,
            });
        let selected = context
            .choose_runnable(profile, profile.scheduler_policy, &woken.next, Some(&key))
            .expect("select owner");
        let blocked =
            self::command(&context, profile, &selected.next, SchedulerCommand::Block { key: key.clone() });
        for mut state in [selected.next, blocked.next] {
            state.next_enqueue_sequence = u64::MAX;
            let command = if state.runnables[0].phase == RunnablePhase::Running {
                SchedulerCommand::Yield { key: key.clone() }
            } else {
                SchedulerCommand::Wake {
                    key: key.clone(),
                    priority: RESUME_PRIORITY,
                }
            };
            context
                .apply_scheduler_command(profile, profile.scheduler_policy, &state, &command)
                .expect_err("checked sequence exhaustion");
        }
        for command in [
            SchedulerCommand::Wake {
                key: key.clone(),
                priority: RESUME_PRIORITY,
            },
            SchedulerCommand::Yield { key: key.clone() },
        ] {
            assert!(context.apply_scheduler_command(profile, profile.scheduler_policy, &woken.next, &command).is_err());
        }
        // The valid Ready occurrence remains selectable after both rejected commands.
        let selected = context
            .choose_runnable(profile, profile.scheduler_policy, &woken.next, Some(&key))
            .expect("invalid commands did not consume readiness");
        assert_eq!(selected.selected, key);
    }
}