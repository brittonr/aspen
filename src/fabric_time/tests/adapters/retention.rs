// r[verify molten.audit_f11.validation]
#[test]
fn concrete_delivery_rejects_tokens_before_publication() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        for is_resume in [false, true] {
            let admitted = profile(kind, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
            let key = key("publication-order", 1);
            let harness = Harness::new(&admitted.profile, GENERATION, std::slice::from_ref(&key));
            harness.wake(&key, 0);
            harness.choose(&key).expect("select owner before control");
            if is_resume {
                harness.command(SchedulerCommand::Block { key: key.clone() }).expect("block owner before resume control");
            }
            let command = if is_resume { SchedulerCommand::Wake { key: key.clone(), priority: RESUME_PRIORITY } } else { SchedulerCommand::Yield { key: key.clone() } };
            let before = harness.snapshot();
            harness.arm_fence_probe();
            {
                let mut owner = harness.owner.lock().expect("negative-control owner lock");
                // Call the concrete adapter directly so its guard, not the
                // harness's accounting, must reject the unpublished state.
                let (unpublished, effects) = owner.delivery_parts_for_test(0);
                match &mut effects.concrete {
                    ConcreteEffects::Live(port) => {
                        let prepared = port.prepare(&key, 0).expect("registered target");
                        assert!(port.deliver(prepared, unpublished).is_err());
                    }
                    ConcreteEffects::Simulation(port) => {
                        let prepared = port.prepare(&key, 0).expect("simulation target");
                        assert!(port.deliver(prepared, unpublished).is_err());
                        assert_eq!(port.drain_next(), None);
                    }
                }
            }
            harness.assert_no_fence_wake();
            assert_eq!(harness.snapshot(), before);
            harness.no_delivery();
            assert_eq!(harness.command(command).expect("normal owning publication"), if is_resume { SchedulerAction::Woken } else { SchedulerAction::Yielded });
            harness.consume(&key);
            harness.usage(1, 1);
        }
    }
}

// r[verify molten.audit_f11.validation]
#[test]
fn stale_prepared_delivery_cannot_unpark_or_queue_a_reused_slot() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        for overload in [SchedulerOverloadPolicy::Reject, SchedulerOverloadPolicy::Backpressure] {
            let admitted = profile(kind, overload, SINGLE_SLOT, SINGLE_SLOT);
            let old = key("same-name", 1);
            let intermediate = key("same-name", 2);
            let fresh = key("same-name", 3);
            let harness = Harness::new(&admitted.profile, GENERATION, &[old.clone(), intermediate.clone(), fresh.clone()]);
            harness.wake(&old, 0);
            let stale = {
                let mut owner = harness.owner.lock().expect("owner");
                match &mut owner.effects_mut().concrete {
                    ConcreteEffects::Live(port) => Prepared::Live(port.prepare(&old, 0).expect("prepare old target")),
                    ConcreteEffects::Simulation(port) => Prepared::Simulation(port.prepare(&old, 0).expect("prepare old target")),
                }
            };
            harness.command(SchedulerCommand::Cancel { key: old.clone() }).expect("retire old");
            harness.wake(&intermediate, 0);
            harness.command(SchedulerCommand::Cancel { key: intermediate }).expect("evict oldest terminal");
            harness.wake(&fresh, 0);
            let before = harness.snapshot();
            assert_eq!(before.0.runnables[0].as_ref().expect("reused slot").key, fresh);
            assert!(!before.0.runnables.iter().flatten().any(|row| row.key == old));
            harness.arm_fence_probe();
            {
                let mut owner = harness.owner.lock().expect("owner");
                let (committed, effects) = owner.delivery_parts_for_test(0);
                match (&mut effects.concrete, stale) {
                    (ConcreteEffects::Live(port), Prepared::Live(prepared)) => assert!(port.deliver(prepared, committed).is_err()),
                    (ConcreteEffects::Simulation(port), Prepared::Simulation(token)) => {
                        assert!(!committed.accepts(token));
                        assert!(port.deliver(token, committed).is_err());
                        assert_eq!(port.drain_next(), None);
                    }
                    _ => unreachable!(),
                }
            }
            harness.assert_no_fence_wake();
            assert_eq!(harness.snapshot(), before);
            harness.no_delivery();
            // A matching new occurrence still reaches the actual receiver.
            harness.choose(&fresh).expect("choose fresh");
            harness.command(SchedulerCommand::Yield { key: fresh.clone() }).expect("yield fresh");
            harness.consume(&fresh);
        }
    }
}

// r[verify molten.audit_f11.validation]
#[test]
fn deferred_simulation_token_does_not_resolve_after_slot_reuse() {
    let admitted = profile(TimeProfileKind::DeterministicSimulation, SchedulerOverloadPolicy::Reject, SINGLE_SLOT, SINGLE_SLOT);
    let mut owner = SchedulerRuntime::activate(context(&admitted.profile, GENERATION), admitted.profile.clone(), SimulationSchedulerWakeAdapter::activate(&admitted.profile).expect("simulation allocation")).expect("activate");
    let mut deferred = None;
    for sequence in 1..=3 {
        let key = key("same-name", sequence);
        owner.command(SchedulerCommand::Wake { key: key.clone(), priority: 0 }).expect("wake fresh");
        let token = owner.effects_mut().drain_next().expect("real simulation queue delivery");
        assert_eq!(owner.resolve_wake(token).expect("matching consumer token").key, key);
        if sequence == 1 { deferred = Some(token); }
        if sequence == 3 {
            let stale = deferred.expect("held old callback");
            assert_eq!(stale.occurrence, token.occurrence, "exercise physical slot reuse");
            assert!(owner.resolve_wake(stale).is_none());
            for invalid in [
                SchedulerWakeToken { generation: GENERATION + 1, ..token },
                SchedulerWakeToken { occurrence_sequence: 0, ..token },
                SchedulerWakeToken { occurrence: usize::MAX, ..token },
            ] { assert!(owner.resolve_wake(invalid).is_none()); }
        }
        owner.command(SchedulerCommand::Cancel { key }).expect("cancel fresh");
        assert!(owner.resolve_wake(token).is_none());
    }
}

// r[verify molten.audit_f11.validation]
#[test]
fn occurrence_issuance_exhaustion_preserves_state_and_never_delivers() {
    for kind in [TimeProfileKind::Live, TimeProfileKind::DeterministicSimulation] {
        let admitted = profile(kind, SchedulerOverloadPolicy::Reject, TWO_SLOTS, TWO_SLOTS);
        let current = key("current", 1);
        let harness = Harness::new(&admitted.profile, GENERATION, std::slice::from_ref(&current));
        harness.wake(&current, 0);
        let before = harness.snapshot();
        assert!(harness.command(SchedulerCommand::Wake { key: key("zero-sequence", 0), priority: 0 }).is_err());
        assert_eq!(harness.snapshot(), before);
        harness.owner.lock().expect("owner").state_mut_for_test().highest_occurrence_sequence = u64::MAX;
        let exhausted = harness.snapshot();
        assert!(harness.owner.lock().expect("owner").next_occurrence_sequence().is_err());
        assert!(harness.command(SchedulerCommand::Wake { key: key("cannot-be-fresh", u64::MAX), priority: 0 }).is_err());
        assert_eq!(harness.snapshot(), exhausted);
        harness.no_delivery();
        // Exhausted fresh issuance must not prevent exact-identity progress.
        harness.choose(&current).expect("select existing occurrence");
        harness.command(SchedulerCommand::Yield { key: current.clone() }).expect("yield exact occurrence despite exhausted issuance");
        harness.consume(&current);
    }
}