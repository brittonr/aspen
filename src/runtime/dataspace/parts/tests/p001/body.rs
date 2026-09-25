
#[test]
fn snapshot_model_covers_handler_state_indexes() {
    let mut state = RuntimeState::new(7);
    let message = Value::string("hello").expect("runtime test value");
    state.apply_step(&Step::Send {
        from: "producer".into(),
        to: "consumer".into(),
        body: message,
    });
    state.apply_step(&Step::Observe {
        actor: "consumer".into(),
        pattern: Value::string("service.ready").expect("runtime test value"),
    });
    state.apply_step(&Step::Assert {
        actor: "producer".into(),
        value: Value::string("service.ready").expect("runtime test value"),
    });
    state.apply_step(&Step::Clock {
        actor: "producer".into(),
    });
    state.apply_step(&Step::Random {
        actor: "producer".into(),
        upper: 100,
    });
    let snapshot = state.snapshot();
    assert_eq!(snapshot.logical_time, 1);
    assert_ne!(snapshot.rng_state, 7);
    assert_eq!(snapshot.effect_sequence, 2);
    assert_eq!(snapshot.messages.len(), 1);
    assert_eq!(snapshot.assertions.len(), 1);
    assert_eq!(snapshot.observers.len(), 1);
    crate::preserves_rail::validate_content_ref(&snapshot.snapshot_ref().expect("snapshot ref"))
        .expect("snapshot ref shape");
}

#[test]
fn transition_is_deterministic_from_explicit_seed() {
    let steps = [
        Step::Observe {
            actor: "consumer".into(),
            pattern: Value::string("service.ready").expect("runtime test value"),
        },
        Step::Assert {
            actor: "producer".into(),
            value: Value::string("service.ready").expect("runtime test value"),
        },
        Step::Clock {
            actor: "producer".into(),
        },
        Step::Random {
            actor: "producer".into(),
            upper: 100,
        },
    ];
    let mut left = RuntimeState::new(7);
    let mut right = RuntimeState::new(7);
    for step in &steps {
        assert_eq!(left.apply_step(step), right.apply_step(step));
        assert_eq!(left.snapshot(), right.snapshot());
    }
}

#[test]
fn clock_and_random_emit_request_response_pairs() {
    let mut state = RuntimeState::new(7);
    let clock = state.apply_step(&Step::Clock { actor: "a".into() });
    assert!(matches!(clock.as_slice(), [Event::EffectRequest { sequence: 0, .. }, Event::EffectResponse {
        sequence: 0,
        value: 0,
        ..
    }]));
    let random = state.apply_step(&Step::Random {
        actor: "a".into(),
        upper: 10,
    });
    assert!(matches!(random.as_slice(), [
        Event::EffectRequest {
            sequence: 1,
            upper: Some(10),
            ..
        },
        Event::EffectResponse {
            sequence: 1,
            upper: Some(10),
            ..
        }
    ]));

    let mut replay = RuntimeState::new(7);
    assert_eq!(clock, replay.apply_step(&Step::Clock { actor: "a".into() }));
    assert_eq!(
        random,
        replay.apply_step(&Step::Random {
            actor: "a".into(),
            upper: 10
        })
    );
}

#[test]
fn recorded_effect_response_transition_is_pure_and_validated() {
    // r[verify molten.runtime_state_machine_proof.turn_commit_delta]
    let mut state = RuntimeState::new(RECORDED_EFFECT_TEST_SEED);
    let clock_request = state
        .begin_effect_for_step(&Step::Clock {
            actor: "clock-actor".into(),
        })
        .expect("clock effect request");
    let before_clock = state.snapshot();
    let clock_law =
        recorded_effect_response_transition(&before_clock, &clock_request, RECORDED_CLOCK_VALUE).expect("clock law");
    let clock_response = state
        .apply_recorded_effect_response(&clock_request, RECORDED_CLOCK_VALUE)
        .expect("apply clock response");
    assert_eq!(clock_response, clock_law.response);
    assert_eq!(state.snapshot(), clock_law.after);
    assert_eq!(state.snapshot().logical_time, RECORDED_CLOCK_VALUE + 1);

    let random_request = state
        .begin_effect_for_step(&Step::Random {
            actor: "random-actor".into(),
            upper: RECORDED_RANDOM_UPPER,
        })
        .expect("random effect request");
    let before_random = state.snapshot();
    let random_law = recorded_effect_response_transition(&before_random, &random_request, RECORDED_RANDOM_VALUE)
        .expect("random law");
    let random_response = state
        .apply_recorded_effect_response(&random_request, RECORDED_RANDOM_VALUE)
        .expect("apply random response");
    assert_eq!(random_response, random_law.response);
    assert_eq!(state.snapshot(), random_law.after);
    assert_ne!(state.snapshot().rng_state, before_random.rng_state);

    let malformed_random = Event::EffectRequest {
        effect: Effect::Random,
        actor: "random-actor".into(),
        sequence: before_random.effect_sequence,
        upper: None,
    };
    let missing_upper = recorded_effect_response_transition(&before_random, &malformed_random, RECORDED_RANDOM_VALUE)
        .expect_err("missing upper is denied");
    assert!(missing_upper.to_string().contains("recorded random effect request missing upper bound"));

    let non_effect_request = Event::TurnRolledBack {
        actor: "svc".into(),
        reason: "not an effect request".into(),
    };
    let non_effect = recorded_effect_response_transition(&before_random, &non_effect_request, RECORDED_RANDOM_VALUE)
        .expect_err("non-effect request is denied");
    assert!(non_effect.to_string().contains("recorded effect response requires an effect request"));
}

#[test]
fn rollback_leaves_staged_actions_uncommitted() {
    let mut state = RuntimeState::new(1);
    let before = state.snapshot();
    let step = Step::Assert {
        actor: "producer".into(),
        value: Value::string("service.ready").expect("runtime test value"),
    };
    let turn = state.begin_turn(&step);
    assert_eq!(state.snapshot(), before);
    let events = state.rollback_turn(turn, step.primary_actor(), "policy denied");
    assert_eq!(state.snapshot(), before);
    assert!(matches!(events.as_slice(), [Event::TurnRolledBack { .. }]));

    let committed = state.apply_step(&step);
    assert!(matches!(committed.as_slice(), [Event::AssertionCommitted { .. }]));
    assert_ne!(state.snapshot(), before);
}

#[test]
fn apply_step_with_predicate_receipt_gates_dataspace_commit() {
    let mut state = RuntimeState::new(1);
    let value = Value::string("service.ready").expect("runtime test value");
    let step = Step::Assert {
        actor: "producer".into(),
        value,
    };

    let (events, receipt) = state.apply_step_with_predicate_receipt(&step).expect("predicate-gated apply succeeds");

    assert!(matches!(events.as_slice(), [Event::AssertionCommitted { .. }]));
    let receipt = receipt.expect("dataspace turn returns predicate receipt");
    assert_eq!(receipt.decision, PredicateDecision::Pass);
    assert!(state.assertions.iter().any(|assertion| assertion.actor == "producer"));
}
