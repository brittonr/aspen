use super::*;

type Capability = crate::runtime::Capability;
type ContentRef = crate::runtime::ContentRef;
type EnvelopeInput = crate::runtime::EnvelopeInput;
type EvidenceRef = crate::runtime::EvidenceRef;

const RECORDED_EFFECT_TEST_SEED: u64 = 7;
const RECORDED_CLOCK_VALUE: u64 = 42;
const RECORDED_RANDOM_VALUE: u64 = 3;
const RECORDED_RANDOM_UPPER: u64 = 10;
const THROTTLE_MAX_FANOUT: usize = 1;

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
fn local_adapter_routes_wildcard_patterns_and_rejects_unsupported_bindings() {
    // r[verify molten.preserves_boundary_codegen.pattern_ast]
    // r[verify molten.preserves_boundary_codegen.pattern_routing]
    // r[verify molten.preserves_boundary_codegen.fixture_corpus]
    let subject = Value::string("service.ready").expect("subject");
    let envelope = Envelope::new(EnvelopeInput {
        sender: ActorId::parse("actor:producer").expect("sender"),
        subject: subject.clone(),
        body: Value::string("ready").expect("body"),
        blob_refs: Vec::new(),
        capabilities: Vec::new(),
        evidence_refs: Vec::new(),
    })
    .expect("envelope");
    let mut adapter = LocalAdapter::new();
    adapter
        .observe_pattern(
            ActorId::parse("actor:wildcard-consumer").expect("consumer"),
            RuntimePattern::wildcard("subject"),
        )
        .expect("wildcard subscription");

    let deliveries = adapter.route_envelope(&envelope).expect("deliveries");
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].actor.as_str(), "actor:wildcard-consumer");

    let unsupported = adapter
        .observe_pattern(ActorId::parse("actor:bad-consumer").expect("bad consumer"), RuntimePattern::wildcard(""));
    assert!(unsupported.is_err());
    let denied = crate::runtime::evaluate_pattern_match(&RuntimePattern::wildcard(""), &subject)
        .expect("pattern denial receipt");
    assert!(!denied.is_match);
    assert_eq!(denied.receipt.decision, PredicateDecision::Deny);
    assert!(denied.receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("binding must not be empty")));
}

#[test]
fn local_state_routes_pattern_observe_for_current_future_and_retractions() {
    // r[verify molten.preserves_boundary_codegen.pattern_routing]
    let ready = Value::string("service.ready").expect("ready value");
    let pattern_value = Value::new(RuntimePattern::exact(ready.clone()).to_value()).expect("pattern value");
    let mut state = RuntimeState::new(1);
    state.apply_step(&Step::Assert {
        actor: "owner-a".into(),
        value: ready.clone(),
    });

    let initial = state.apply_step(&Step::Observe {
        actor: "observer-a".into(),
        pattern: pattern_value.clone(),
    });
    assert!(matches!(initial.as_slice(), [Event::ObserveRegistered { .. }, Event::AssertionObserved { .. }]));

    let future = state.apply_step(&Step::Assert {
        actor: "owner-b".into(),
        value: ready.clone(),
    });
    assert!(matches!(future.as_slice(), [Event::AssertionCommitted { .. }, Event::AssertionObserved { .. }]));

    let retraction = state.apply_step(&Step::Retract {
        actor: "owner-b".into(),
        value: ready,
    });
    assert!(matches!(retraction.as_slice(), [
        Event::AssertionRetracted { .. },
        Event::AssertionRetractionObserved { .. }
    ]));
}

#[test]
fn syndicate_reference_harness_matches_molten_observe_lifecycle() {
    // r[verify molten.syndicate_dataspace.reference_harness]
    // r[verify molten.syndicate_dataspace.parity_receipts]
    // r[verify molten.syndicate_dataspace.fixture_parity]
    // r[verify molten.syndicate_dataspace.trace_evidence]
    let ready = Value::string("service.ready").expect("ready value");
    let exact_pattern = Value::new(RuntimePattern::exact(ready.clone()).to_value()).expect("pattern value");
    let steps = vec![
        Step::Assert {
            actor: "owner-a".into(),
            value: ready.clone(),
        },
        Step::Observe {
            actor: "observer-a".into(),
            pattern: exact_pattern,
        },
        Step::Assert {
            actor: "owner-b".into(),
            value: ready.clone(),
        },
        Step::Retract {
            actor: "owner-b".into(),
            value: ready,
        },
    ];

    let run = run_reference_harness(&steps, &CapabilityContext::allow_all(), ResourceBudget::default())
        .expect("syndicate reference run");

    assert_eq!(run.parity.decision, "pass");
    assert_eq!(run.trace.replayability_status, "recorded");
    assert_eq!(run.flow_control.len(), steps.len());
    assert!(run.flow_control.iter().all(|receipt| receipt.decision == "pass"));
}

#[test]
fn syndicate_reference_harness_denies_missing_molten_authority() {
    // r[verify molten.syndicate_dataspace.cap_attenuation]
    // r[verify molten.syndicate_dataspace.fixture_parity]
    let privileged = Value::string("service.privileged").expect("privileged value");
    let steps = vec![Step::Assert {
        actor: "owner-a".into(),
        value: privileged,
    }];

    let run = run_reference_harness(&steps, &CapabilityContext::from_grants(Vec::new()), ResourceBudget::default())
        .expect("syndicate missing authority run");

    assert_eq!(run.parity.decision, "pass");
    assert!(
        matches!(run.molten_events.as_slice(), [Event::TurnRolledBack { reason, .. }] if reason.contains("missing Molten capability"))
    );
    assert_eq!(run.syndicate_events, run.molten_events);
}

#[test]
fn syndicate_flow_control_throttles_fanout_deterministically() {
    // r[verify molten.syndicate_dataspace.flow_control_receipts]
    let ready = Value::string("service.ready").expect("ready value");
    let wildcard = Value::new(RuntimePattern::wildcard("subject").to_value()).expect("wildcard value");
    let steps = vec![
        Step::Observe {
            actor: "observer-a".into(),
            pattern: wildcard.clone(),
        },
        Step::Observe {
            actor: "observer-b".into(),
            pattern: wildcard,
        },
        Step::Assert {
            actor: "owner-a".into(),
            value: ready,
        },
    ];

    let run = run_reference_harness(&steps, &CapabilityContext::allow_all(), ResourceBudget {
        max_fanout: THROTTLE_MAX_FANOUT,
    })
    .expect("syndicate throttled run");

    assert_eq!(run.parity.decision, "pass");
    assert!(run.flow_control.iter().any(|receipt| {
        receipt.decision == "deny"
            && receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "syndicate-account-fanout-budget-exceeded")
    }));
    assert!(
        matches!(run.molten_events.last(), Some(Event::TurnRolledBack { reason, .. }) if reason.contains("fanout"))
    );
}

#[test]
fn syndicate_cleanup_retracts_owned_assertions_and_observers() {
    // r[verify molten.syndicate_dataspace.facet_cleanup]
    let ready = Value::string("service.ready").expect("ready value");
    let mut harness = ReferenceHarness::new();
    harness
        .apply_step(&Step::Assert {
            actor: "owner-a".into(),
            value: ready.clone(),
        })
        .expect("assert ready");
    harness
        .apply_step(&Step::Observe {
            actor: "owner-a".into(),
            pattern: ready,
        })
        .expect("observe ready");

    let cleanup = harness.cleanup_actor_scope("owner-a").expect("cleanup owner");

    assert_eq!(cleanup.actor, "owner-a");
    assert_eq!(cleanup.assertion_refs.len(), THROTTLE_MAX_FANOUT);
    assert_eq!(cleanup.observer_refs.len(), THROTTLE_MAX_FANOUT);
}

#[test]
fn syndicate_empty_trace_remains_diagnostic_only() {
    // r[verify molten.syndicate_dataspace.trace_evidence]
    let run = run_reference_harness(&[], &CapabilityContext::allow_all(), ResourceBudget::default())
        .expect("empty syndicate run");

    assert_eq!(run.parity.decision, "pass");
    assert_eq!(run.trace.replayability_status, "diagnostic-only");
    assert!(
        run.trace
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "syndicate-trace-has-no-committed-action-refs")
    );
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
