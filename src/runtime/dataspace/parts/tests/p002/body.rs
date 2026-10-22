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
    assert!(matches!(
        observed.as_slice(),
        [Event::ObserveRegistered { .. }, Event::TurnCommitted { .. }]
    ));

    let first = state.apply_step(&Step::Assert {
        actor: "owner-a".into(),
        value: ready.clone(),
    });
    assert!(matches!(
        first.as_slice(),
        [
            Event::AssertionCommitted { .. },
            Event::AssertionObserved { .. },
            Event::TurnCommitted { .. }
        ]
    ));

    let duplicate_owner = state.apply_step(&Step::Assert {
        actor: "owner-b".into(),
        value: ready.clone(),
    });
    assert!(matches!(
        duplicate_owner.as_slice(),
        [Event::AssertionCommitted { .. }, Event::TurnCommitted { .. }]
    ));

    let duplicate_owner_again = state.apply_step(&Step::Assert {
        actor: "owner-b".into(),
        value: ready,
    });
    assert!(matches!(
        duplicate_owner_again.as_slice(),
        [Event::AssertionCommitted { .. }, Event::TurnCommitted { .. }]
    ));
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
    assert!(matches!(
        first.as_slice(),
        [Event::AssertionRetracted { .. }, Event::TurnCommitted { .. }]
    ));
    assert_eq!(state.snapshot().assertions.len(), 1);
    assert!(state.snapshot().assertions.iter().any(|assertion| assertion.actor == "owner-b"));

    let last = state.apply_step(&Step::Retract {
        actor: "owner-b".into(),
        value: ready,
    });
    assert!(matches!(last.as_slice(), [
        Event::AssertionRetracted { .. },
        Event::AssertionRetractionObserved { .. },
        Event::TurnCommitted { .. }
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
        assert!(matches!(
            events.as_slice(),
            [
                Event::AssertionCommitted { .. },
                Event::AssertionObserved { .. },
                Event::TurnCommitted { .. }
            ]
        ));
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

    assert!(matches!(events.as_slice(), [Event::TurnRolledBack { reason, .. }, Event::TurnCommitted { .. }]
        if reason == "runtime retraction requires a committed assertion"));
    assert_eq!(state.snapshot(), before);
}

#[test]
fn facet_stop_orders_children_before_parent_and_runs_handlers_last() {
    // r[verify molten.runtime_spine.facet_owner_scopes]
    let parent_value = Value::string("parent.ready").expect("parent value");
    let child_value = Value::string("child.ready").expect("child value");
    let mut state = RuntimeState::new(1);
    state
        .apply_facet_create("actor-1", "facet:parent", None, Some("handler:parent"))
        .expect("create parent facet");
    state
        .apply_facet_create("actor-1", "facet:child", Some("facet:parent"), Some("handler:child"))
        .expect("create child facet");
    state.apply_facet_assert("actor-1", "facet:parent", &parent_value).expect("assert parent state");
    state.apply_facet_assert("actor-1", "facet:child", &child_value).expect("assert child state");

    let (events, receipt) = state.apply_facet_stop("actor-1", "facet:parent").expect("stop parent facet");

    assert_eq!(receipt.decision, PredicateDecision::Pass);
    let (facet_events, turn_record) = events.split_at(6);
    assert_eq!(facet_events, vec![
        Event::AssertionRetracted {
            actor: "actor-1".to_string(),
            value: child_value,
        },
        Event::FacetStopped {
            facet_id: "facet:child".to_string(),
        },
        Event::FacetStopHandlerRan {
            facet_id: "facet:child".to_string(),
            handler_ref: "handler:child".to_string(),
        },
        Event::AssertionRetracted {
            actor: "actor-1".to_string(),
            value: parent_value,
        },
        Event::FacetStopped {
            facet_id: "facet:parent".to_string(),
        },
        Event::FacetStopHandlerRan {
            facet_id: "facet:parent".to_string(),
            handler_ref: "handler:parent".to_string(),
        },
    ]);
    assert!(matches!(turn_record, [Event::TurnCommitted { .. }]));
    let snapshot = state.snapshot();
    assert!(snapshot.assertions.is_empty());
    assert_eq!(
        snapshot.facets.get("facet:parent").map(|facet| facet.state),
        Some(crate::runtime::RuntimeFacetState::Stopped)
    );
    assert_eq!(
        snapshot.facets.get("facet:child").map(|facet| facet.state),
        Some(crate::runtime::RuntimeFacetState::Stopped)
    );
}

#[test]
fn stopped_facet_denies_late_assertions_observers_and_children() {
    // r[verify molten.runtime_spine.facet_owner_scopes]
    let value = Value::string("service.ready").expect("runtime value");
    let pattern = Value::new(RuntimePattern::exact(value.clone()).to_value()).expect("pattern value");
    let mut state = RuntimeState::new(1);
    state.apply_facet_create("actor-1", "facet:parent", None, None).expect("create parent facet");
    state.apply_facet_stop("actor-1", "facet:parent").expect("stop parent facet");

    assert!(state.apply_facet_assert("actor-1", "facet:parent", &value).is_err());
    assert!(state.apply_facet_observe("actor-1", "facet:parent", &pattern).is_err());
    assert!(state
        .apply_facet_create("actor-1", "facet:child", Some("facet:parent"), None)
        .is_err());
    assert!(state.apply_facet_stop("actor-1", "facet:parent").is_err());
    assert_eq!(
        state.snapshot().facets.get("facet:parent").map(|facet| facet.state),
        Some(crate::runtime::RuntimeFacetState::Stopped)
    );
}

#[test]
fn crash_cleanup_removes_facet_state_without_running_handlers() {
    // r[verify molten.runtime_spine.facet_owner_scopes]
    let parent_value = Value::string("parent.ready").expect("parent value");
    let child_value = Value::string("child.ready").expect("child value");
    let pattern = Value::new(RuntimePattern::wildcard("subject").to_value()).expect("pattern value");
    let mut state = RuntimeState::new(1);
    state
        .apply_facet_create("actor-1", "facet:parent", None, Some("handler:parent"))
        .expect("create parent facet");
    state
        .apply_facet_create("actor-1", "facet:child", Some("facet:parent"), Some("handler:child"))
        .expect("create child facet");
    state.apply_facet_assert("actor-1", "facet:parent", &parent_value).expect("assert parent state");
    state.apply_facet_assert("actor-1", "facet:child", &child_value).expect("assert child state");
    state.apply_facet_observe("actor-1", "facet:child", &pattern).expect("observe in child facet");

    let cleanup = state.cleanup_actor_scope("actor-1").expect("crash cleanup");

    assert_eq!(cleanup.actor, "actor-1");
    assert_eq!(cleanup.assertion_refs.len(), 2);
    assert_eq!(cleanup.observer_refs.len(), 1);
    let snapshot = state.snapshot();
    assert!(snapshot.assertions.is_empty());
    assert!(snapshot.observers.is_empty());
    assert!(snapshot.facets.is_empty());
}
