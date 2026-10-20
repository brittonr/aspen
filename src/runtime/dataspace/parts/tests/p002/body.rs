#[test]
fn observer_notifications_coalesce_equal_assertions() {
    // r[verify molten.runtime_spine.assertion_notification_coalescing]
    let ready = Value::string("service.ready").expect("ready value");
    let pattern = Value::new(RuntimePattern::exact(ready.clone()).to_value()).expect("pattern value");
    let mut state = RuntimeState::new(1);
    let observed = state.apply_step(&Step::Observe {
        actor: "observer-a".into(),
        pattern,
    });
    assert!(matches!(observed.as_slice(), [Event::ObserveRegistered { .. }]));

    let first = state.apply_step(&Step::Assert {
        actor: "owner-a".into(),
        value: ready.clone(),
    });
    assert!(matches!(first.as_slice(), [Event::AssertionCommitted { .. }, Event::AssertionObserved { .. }]));

    let duplicate_owner = state.apply_step(&Step::Assert {
        actor: "owner-b".into(),
        value: ready.clone(),
    });
    assert!(matches!(duplicate_owner.as_slice(), [Event::AssertionCommitted { .. }]));

    let duplicate_owner_again = state.apply_step(&Step::Assert {
        actor: "owner-b".into(),
        value: ready,
    });
    assert!(matches!(duplicate_owner_again.as_slice(), [Event::AssertionCommitted { .. }]));
}

#[test]
fn retraction_notifies_only_after_the_last_owner_withdraws() {
    // r[verify molten.runtime_spine.assertion_notification_coalescing]
    let ready = Value::string("service.ready").expect("ready value");
    let pattern = Value::new(RuntimePattern::exact(ready.clone()).to_value()).expect("pattern value");
    let mut state = RuntimeState::new(1);
    state.apply_step(&Step::Observe {
        actor: "observer-a".into(),
        pattern,
    });
    state.apply_step(&Step::Assert {
        actor: "owner-a".into(),
        value: ready.clone(),
    });
    state.apply_step(&Step::Assert {
        actor: "owner-b".into(),
        value: ready.clone(),
    });

    let first = state.apply_step(&Step::Retract {
        actor: "owner-a".into(),
        value: ready.clone(),
    });
    assert!(matches!(first.as_slice(), [Event::AssertionRetracted { .. }]));
    assert_eq!(state.snapshot().assertions.len(), 1);
    assert!(state.snapshot().assertions.iter().any(|assertion| assertion.actor == "owner-b"));

    let last = state.apply_step(&Step::Retract {
        actor: "owner-b".into(),
        value: ready,
    });
    assert!(matches!(last.as_slice(), [
        Event::AssertionRetracted { .. },
        Event::AssertionRetractionObserved { .. }
    ]));
    assert!(state.snapshot().assertions.is_empty());
}

#[test]
fn distinct_assertion_values_notify_separately() {
    // r[verify molten.runtime_spine.assertion_notification_coalescing]
    let pattern = Value::new(RuntimePattern::wildcard("subject").to_value()).expect("wildcard value");
    let mut state = RuntimeState::new(1);
    state.apply_step(&Step::Observe {
        actor: "observer-a".into(),
        pattern,
    });

    for value in ["service.ready", "service.ready.extra"] {
        let events = state.apply_step(&Step::Assert {
            actor: "owner-a".into(),
            value: Value::string(value).expect("runtime value"),
        });
        assert!(matches!(events.as_slice(), [Event::AssertionCommitted { .. }, Event::AssertionObserved { .. }]));
    }
}

#[test]
fn owner_scope_cleanup_keeps_a_value_visible_while_another_owner_remains() {
    // r[verify molten.runtime_spine.assertion_notification_coalescing]
    let ready = Value::string("service.ready").expect("ready value");
    let pattern = Value::new(RuntimePattern::exact(ready.clone()).to_value()).expect("pattern value");
    let mut state = RuntimeState::new(1);
    state.apply_step(&Step::Observe {
        actor: "observer-a".into(),
        pattern,
    });
    state.apply_step(&Step::Assert {
        actor: "owner-a".into(),
        value: ready.clone(),
    });
    state.apply_step(&Step::Assert {
        actor: "owner-b".into(),
        value: ready,
    });

    state.cleanup_actor_scope("owner-a").expect("cleanup owner-a");
    assert_eq!(state.snapshot().assertions.len(), 1);
    assert!(state.snapshot().assertions.iter().any(|assertion| assertion.actor == "owner-b"));

    state.cleanup_actor_scope("owner-b").expect("cleanup owner-b");
    assert!(state.snapshot().assertions.is_empty());
}

#[test]
fn retracting_an_unheld_assertion_rolls_back_before_commit() {
    // r[verify molten.runtime_spine.assertion_notification_coalescing]
    let ready = Value::string("service.ready").expect("ready value");
    let mut state = RuntimeState::new(1);
    let before = state.snapshot();

    let events = state.apply_step(&Step::Retract {
        actor: "owner-a".into(),
        value: ready,
    });

    assert!(matches!(events.as_slice(), [Event::TurnRolledBack { reason, .. }]
        if reason == "runtime retraction requires a committed assertion"));
    assert_eq!(state.snapshot(), before);
}
