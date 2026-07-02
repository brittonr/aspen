use super::*;

type Capability = crate::runtime::Capability;
type ContentRef = crate::runtime::ContentRef;
type EnvelopeInput = crate::runtime::EnvelopeInput;
type EvidenceRef = crate::runtime::EvidenceRef;

#[test]
fn local_routes_matching_envelope_subject() {
    let subject = Value::string("service.ready").expect("subject");
    let envelope = Envelope::new(EnvelopeInput {
        sender: ActorId::parse("actor:producer").expect("sender"),
        subject: subject.clone(),
        body: Value::string("ready").expect("body"),
        blob_refs: vec![ContentRef::parse(crate::preserves_rail::content_ref_from_bytes(b"payload")).expect("blob")],
        capabilities: vec![Capability::parse("send:service.ready").expect("capability")],
        evidence_refs: vec![
            EvidenceRef::parse(crate::preserves_rail::content_ref_from_bytes(b"route-evidence")).expect("evidence"),
        ],
    })
    .expect("envelope");
    let mut adapter = LocalAdapter::new();
    adapter.register_actor(ActorId::parse("actor:ignored").expect("ignored actor"));
    adapter.observe_subject(ActorId::parse("actor:consumer").expect("consumer"), &subject);

    let deliveries = adapter.route_envelope(&envelope).expect("deliveries");
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].actor.as_str(), "actor:consumer");
    assert_eq!(deliveries[0].boundary.subject_ref, subject.value_ref());
    assert_eq!(deliveries[0].boundary.envelope_ref, envelope.canonical_hash().expect("envelope ref"));
}

#[test]
fn values_and_events_expose_stable_content_refs() {
    let value = Value::string("service.ready").expect("runtime value");
    crate::preserves_rail::validate_content_ref(value.value_ref()).expect("value ref shape");
    assert_eq!(
        value.value_ref(),
        crate::preserves_rail::canonical_hash(value.as_iovalue()).expect("canonical value ref")
    );

    let message = Message {
        from: "producer".to_string(),
        to: "consumer".to_string(),
        body: value.clone(),
    };
    crate::preserves_rail::validate_content_ref(&message.message_ref().expect("message ref"))
        .expect("message ref shape");
    let mut state = RuntimeState::new(7);
    state.apply_step(&Step::Send {
        from: "producer".to_string(),
        to: "consumer".to_string(),
        body: value.clone(),
    });
    let snapshot_ref = state.snapshot().snapshot_ref().expect("snapshot ref");
    crate::preserves_rail::validate_content_ref(&snapshot_ref).expect("snapshot ref shape");

    let event = Event::MessageDelivered {
        from: "producer".to_string(),
        to: "consumer".to_string(),
        body: value,
    };
    let event_ref = event.event_ref().expect("event ref");
    crate::preserves_rail::validate_content_ref(&event_ref).expect("event ref shape");
    assert_eq!(event_ref, event.event_ref().expect("event ref stable"));
}

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
