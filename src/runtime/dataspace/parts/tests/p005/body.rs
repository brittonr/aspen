// Regression tests for monotonic counters, recorded RNG ranges, honest rollback
// receipts, actor-attributed facet turns, facet-stop retraction notices, and
// force-run demand precedence.

const DETERMINISM_SEED: u64 = 13;
const DETERMINISM_RANDOM_UPPER: u64 = 10;
const DETERMINISM_SERVICE_REF: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DETERMINISM_DEMANDER_REF: &str = "scope:determinism-demander";

fn determinism_state_with_counters(logical_time: u64, effect_sequence: u64) -> RuntimeState {
    let mut snapshot = RuntimeState::new(DETERMINISM_SEED).snapshot();
    snapshot.logical_time = logical_time;
    snapshot.effect_sequence = effect_sequence;
    RuntimeState::from_snapshot(snapshot).expect("restorable counters")
}

fn determinism_ready_value() -> Value {
    Value::string("service.ready").expect("ready value")
}

fn determinism_exact_pattern(value: &Value) -> Value {
    Value::new(RuntimePattern::exact(value.clone()).to_value()).expect("pattern value")
}

fn determinism_retraction_notices(events: &[Event]) -> Vec<&Event> {
    events
        .iter()
        .filter(|event| matches!(event, Event::AssertionRetractionObserved { .. }))
        .collect()
}

fn determinism_committed_actor(events: &[Event]) -> String {
    events
        .iter()
        .find_map(|event| match event {
            Event::TurnCommitted { actor, .. } => Some(actor.clone()),
            _ => None,
        })
        .expect("the committed turn record")
}

#[test]
fn recorded_clock_regression_and_exhaustion_are_denied() {
    // r[verify molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    let mut state = determinism_state_with_counters(40, 0);
    let request = state
        .begin_effect_for_step(&Step::Clock {
            actor: "clock-actor".into(),
        })
        .expect("checked sequence")
        .expect("clock request");
    let before = state.snapshot();

    let regression = recorded_effect_response_transition(&before, &request, 39).expect_err("regression denied");
    assert!(regression.to_string().contains("regresses logical time 40"), "{regression}");
    for exhausted in [u64::MAX - 1, u64::MAX] {
        let error = recorded_effect_response_transition(&before, &request, exhausted).expect_err("exhaustion denied");
        assert!(error.to_string().contains("logical time is exhausted"), "{error}");
    }

    let same = recorded_effect_response_transition(&before, &request, 40).expect("equal time is monotonic");
    assert_eq!(same.after.logical_time, 41);
    let highest = recorded_effect_response_transition(&before, &request, u64::MAX - 2).expect("highest restorable");
    assert_eq!(highest.after.logical_time, u64::MAX - 1);
    RuntimeState::from_snapshot(highest.after).expect("the replayed snapshot restores");
}

#[test]
fn recorded_random_value_must_lie_below_its_upper_bound() {
    // r[verify molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    let mut state = RuntimeState::new(DETERMINISM_SEED);
    let request = state
        .begin_effect_for_step(&Step::Random {
            actor: "random-actor".into(),
            upper: DETERMINISM_RANDOM_UPPER,
        })
        .expect("checked sequence")
        .expect("random request");
    let before = state.snapshot();
    for out_of_range in [DETERMINISM_RANDOM_UPPER, DETERMINISM_RANDOM_UPPER + 1, u64::MAX] {
        let error =
            recorded_effect_response_transition(&before, &request, out_of_range).expect_err("out-of-range denied");
        assert!(error.to_string().contains("outside the admitted range 0..10"), "{error}");
    }
    let accepted = recorded_effect_response_transition(&before, &request, DETERMINISM_RANDOM_UPPER - 1)
        .expect("in-range value replays");
    assert_ne!(accepted.after.rng_state, before.rng_state);

    let empty_range = Event::EffectRequest {
        effect: Effect::Random,
        actor: "random-actor".into(),
        sequence: before.effect_sequence,
        upper: Some(0),
    };
    recorded_effect_response_transition(&before, &empty_range, 0).expect_err("an empty range admits no value");
    let mut live = RuntimeState::new(DETERMINISM_SEED);
    let live_before = live.snapshot();
    live.apply_step_with_predicate_receipt(&Step::Random {
        actor: "random-actor".into(),
        upper: 0,
    })
    .expect_err("a live empty range is denied too");
    assert_eq!(live.snapshot(), live_before);
}

#[test]
fn live_counters_never_reach_an_unrestorable_maximum() {
    // r[verify molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    let clock = Step::Clock {
        actor: "clock-actor".into(),
    };
    let random = Step::Random {
        actor: "random-actor".into(),
        upper: DETERMINISM_RANDOM_UPPER,
    };

    let mut time_exhausted = determinism_state_with_counters(u64::MAX - 1, 0);
    let before = time_exhausted.snapshot();
    let error = time_exhausted.apply_step_with_predicate_receipt(&clock).expect_err("logical time exhausted");
    assert!(error.to_string().contains("logical time is exhausted"), "{error}");
    assert_eq!(time_exhausted.snapshot(), before, "no partial counter advance");
    assert!(matches!(time_exhausted.apply_step(&clock).as_slice(), [Event::TurnRolledBack { .. }]));
    assert_eq!(time_exhausted.snapshot(), before);

    let mut sequence_exhausted = determinism_state_with_counters(0, u64::MAX - 1);
    let before = sequence_exhausted.snapshot();
    for step in [&clock, &random] {
        let error = sequence_exhausted.apply_step_with_predicate_receipt(step).expect_err("sequence exhausted");
        assert!(error.to_string().contains("effect sequence is exhausted"), "{error}");
        sequence_exhausted.begin_effect_for_step(step).expect_err("a recorded request cannot exhaust it either");
        assert_eq!(sequence_exhausted.snapshot(), before);
    }

    let mut last_tick = determinism_state_with_counters(u64::MAX - 2, u64::MAX - 2);
    let (events, receipt) = last_tick.apply_step_with_predicate_receipt(&clock).expect("last restorable tick");
    assert!(receipt.is_none());
    assert!(matches!(events.as_slice(), [
        Event::EffectRequest { sequence, .. },
        Event::EffectResponse { value, .. }
    ] if *sequence == u64::MAX - 2 && *value == u64::MAX - 2));
    let snapshot = last_tick.snapshot();
    assert_eq!((snapshot.logical_time, snapshot.effect_sequence), (u64::MAX - 1, u64::MAX - 1));
    RuntimeState::from_snapshot(snapshot).expect("the advanced snapshot restores");
}

#[test]
fn rolled_back_retraction_returns_only_a_rollback_receipt() {
    // r[verify molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    let ready = determinism_ready_value();
    let step = Step::Retract {
        actor: "owner-a".into(),
        value: ready.clone(),
    };
    let mut state = RuntimeState::new(DETERMINISM_SEED);
    let before = state.snapshot();
    let turn = state.begin_turn(&step);

    let (events, receipt) = state.apply_step_with_predicate_receipt(&step).expect("rollback is an outcome");
    let receipt = receipt.expect("rollback receipt");
    assert_eq!(events, vec![Event::TurnRolledBack {
        actor: "owner-a".to_string(),
        reason: RETRACTION_WITHOUT_ASSERTION.to_string(),
    }]);
    let rolled_back = crate::runtime::turn_transition_input_ref(&before, &turn, &before, TurnOutcome::RolledBack)
        .expect("rolled-back input ref");
    let committed = crate::runtime::turn_transition_input_ref(&before, &turn, &before, TurnOutcome::Committed)
        .expect("committed input ref");
    assert_eq!(receipt.input_ref, rolled_back, "the receipt binds the rolled-back outcome");
    assert_ne!(receipt.input_ref, committed, "the receipt never claims a commit");
    assert_eq!(state.snapshot(), before);

    let (direct_events, _) = state.commit_turn_with_predicate_receipt(turn).expect("direct staged rollback");
    assert!(matches!(direct_events.as_slice(), [Event::TurnRolledBack { .. }]));

    state.apply_facet_create("owner-a", "facet:owner", None, None).expect("facet");
    let (facet_events, _) = state.apply_facet_retract("owner-a", "facet:owner", &ready).expect("facet rollback");
    assert!(matches!(facet_events.as_slice(), [Event::TurnRolledBack { .. }]));

    state.apply_step(&Step::Assert {
        actor: "owner-a".into(),
        value: ready.clone(),
    });
    let (held_events, held_receipt) = state.apply_step_with_predicate_receipt(&step).expect("held retraction");
    assert!(matches!(held_events.as_slice(), [Event::AssertionRetracted { .. }, Event::TurnCommitted { .. }]));
    assert_eq!(held_receipt.expect("commit receipt").decision, PredicateDecision::Pass);
}

#[test]
fn facet_turns_are_attributed_to_the_owning_actor() {
    // r[verify molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    let ready = determinism_ready_value();
    let pattern = determinism_exact_pattern(&ready);
    let mut state = RuntimeState::new(DETERMINISM_SEED);
    let turns = [
        state.apply_facet_create("actor-1", "facet:root", None, None).expect("create").0,
        state.apply_facet_assert("actor-1", "facet:root", &ready).expect("assert").0,
        state.apply_facet_observe("actor-1", "facet:root", &pattern).expect("observe").0,
        state.apply_facet_retract("actor-1", "facet:root", &ready).expect("retract").0,
        state.apply_facet_stop("actor-1", "facet:root").expect("stop").0,
    ];
    for events in &turns {
        assert_eq!(determinism_committed_actor(events), "actor-1");
    }
}

#[test]
fn facet_stop_notifies_observers_when_the_last_owner_retracts() {
    // r[verify molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    let ready = determinism_ready_value();
    let pattern = determinism_exact_pattern(&ready);
    let mut state = RuntimeState::new(DETERMINISM_SEED);
    state.apply_step(&Step::Observe {
        actor: "watcher".into(),
        pattern: pattern.clone(),
    });
    state.apply_facet_create("actor-1", "facet:parent", None, None).expect("parent");
    state.apply_facet_create("actor-1", "facet:child", Some("facet:parent"), None).expect("child");
    state.apply_facet_observe("actor-1", "facet:child", &pattern).expect("stopping observer");
    state.apply_facet_assert("actor-1", "facet:parent", &ready).expect("parent assertion");
    state.apply_facet_assert("actor-1", "facet:child", &ready).expect("child assertion");

    let (events, _) = state.apply_facet_stop("actor-1", "facet:parent").expect("stop");
    assert_eq!(determinism_retraction_notices(&events), vec![&Event::AssertionRetractionObserved {
        observer: "watcher".to_string(),
        owner: "actor-1".to_string(),
        value: ready.clone(),
    }]);
    assert!(state.snapshot().assertions.is_empty());

    let mut shared = RuntimeState::new(DETERMINISM_SEED);
    shared.apply_step(&Step::Observe {
        actor: "watcher".into(),
        pattern,
    });
    shared.apply_step(&Step::Assert {
        actor: "other-owner".into(),
        value: ready.clone(),
    });
    shared.apply_facet_create("actor-1", "facet:root", None, None).expect("facet");
    shared.apply_facet_assert("actor-1", "facet:root", &ready).expect("facet assertion");
    let (shared_events, _) = shared.apply_facet_stop("actor-1", "facet:root").expect("stop");
    assert!(determinism_retraction_notices(&shared_events).is_empty(), "another owner keeps the value visible");
}

#[test]
fn force_run_demand_outranks_dependency_gated_in_any_order() {
    // r[verify molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
    let demand_assertion = |kind: ServiceDemandKind, operation: &str| Assertion {
        actor: DETERMINISM_DEMANDER_REF.to_string(),
        value: ServiceDemandAssertion {
            service_ref: DETERMINISM_SERVICE_REF.to_string(),
            kind,
            demander_ref: DETERMINISM_DEMANDER_REF.to_string(),
            operation_identity: operation.to_string(),
        }
        .to_assertion_value()
        .expect("demand value"),
        facet: None,
    };
    let gated = demand_assertion(ServiceDemandKind::DependencyGated, "operation:gated");
    let forced = demand_assertion(ServiceDemandKind::ForceRun, "operation:forced");
    for assertions in [[gated.clone(), forced.clone()], [forced.clone(), gated.clone()]] {
        let demands = ServiceDemandSet::derive(&assertions).expect("demand set");
        assert_eq!(demands.force_run_service_refs(), vec![DETERMINISM_SERVICE_REF.to_string()]);
        assert_eq!(demands.live_demand_count(DETERMINISM_SERVICE_REF), 1);
    }

    let mut state = RuntimeState::new(DETERMINISM_SEED);
    for assertion in [&gated, &forced] {
        state.apply_step(&Step::Assert {
            actor: assertion.actor.clone(),
            value: assertion.value.clone(),
        });
    }
    let live = state.live_assertions().iter().cloned().collect::<Vec<_>>();
    let demands = ServiceDemandSet::derive(&live).expect("live demand set");
    assert_eq!(demands.force_run_service_refs(), vec![DETERMINISM_SERVICE_REF.to_string()]);

    let gated_only = ServiceDemandSet::derive(std::slice::from_ref(&gated)).expect("gated demand set");
    assert!(gated_only.force_run_service_refs().is_empty());
    assert_eq!(gated_only.demanded_service_refs(), vec![DETERMINISM_SERVICE_REF.to_string()]);
}
