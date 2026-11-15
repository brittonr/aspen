use super::*;

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn initial() -> State {
    State {
        actor_key_ref: reference("actor"),
        value: 0,
        completed: Vec::new(),
    }
}

fn add(label: &str, increment: u64) -> Add {
    Add {
        actor_key_ref: reference("actor"),
        request_id: reference(label),
        increment,
    }
}

// r[verify molten.addressable_actor.public_result]
#[test]
fn logical_replay_returns_original_result_without_second_increment() {
    let mut state = initial();
    assert_eq!(state.apply(&add("one", 3)).expect("first request"), 3);
    assert_eq!(state.apply(&add("two", 7)).expect("second request"), 10);
    let before = state.clone();
    assert_eq!(state.apply(&add("one", 3)).expect("replay"), 3);
    assert_eq!(state, before);
    assert!(state.apply(&add("one", 4)).is_err());
    assert_eq!(state, before);
}

#[test]
fn canonical_state_reopens_with_exact_results() {
    let mut state = initial();
    state.apply(&add("one", 8)).expect("request");
    let bytes = state.materialize().expect("canonical state");
    let reopened = State::decode(&bytes.bytes).expect("reopened state");
    assert_eq!(reopened, state);
    assert_eq!(reopened.materialize().expect("canonical reopen"), bytes);
    let input = add("two", 1);
    assert_eq!(Add::decode(&input.materialize().expect("request bytes").bytes).expect("decoded request"), input);
}

#[test]
fn wrong_key_overflow_and_retention_deny_without_semantic_mutation() {
    let mut state = initial();
    let before = state.clone();
    let mut wrong = add("wrong", 1);
    wrong.actor_key_ref = reference("other actor");
    assert!(state.apply(&wrong).is_err());
    assert_eq!(state, before);
    state.apply(&add("maximum", u64::MAX)).expect("maximum counter");
    let before = state.clone();
    assert!(state.apply(&add("overflow", 1)).is_err());
    assert_eq!(state, before);
    let mut state = initial();
    for index in 0..MAX_COMPLETED_REQUESTS {
        state.apply(&add(&format!("request-{index}"), 1)).expect("bounded request");
    }
    let before = state.clone();
    assert!(state.apply(&add("outside-retention", 1)).is_err());
    assert_eq!(state, before);
    assert_eq!(state.apply(&add("request-0", 1)).expect("retained replay"), 1);
}

#[test]
fn malformed_state_total_is_not_admitted() {
    let state = crate::preserves_rail::record("addressable-counter-state-v1", vec![
        crate::preserves_rail::string(reference("actor")),
        crate::preserves_rail::u64_value(9),
        crate::preserves_rail::sequence(Vec::new()),
    ]);
    assert!(State::decode(&crate::preserves_rail::canonical_bytes(&state).expect("bytes")).is_err());
}

fn envelope(callback: crate::system_extension::CallbackKind) -> crate::system_extension::DecodedCallbackEnvelope {
    crate::system_extension::DecodedCallbackEnvelope {
        invocation: crate::system_extension::CallbackInvocation {
            callback,
            generation: 1,
            incarnation: 2,
            sequence: 3,
            event_ref: reference("event"),
            payload_ref: None,
            logical_tick: 4,
            deadline_tick: 20,
        },
        context: crate::system_extension::CallbackContext {
            manifest_ref: reference("manifest"),
            executable_ref: reference("guest"),
            instance_id: reference("native-instance"),
            extension_id: "counter".to_string(),
            service_id: reference("actor"),
            state_ref: None,
            policy_refs: vec![reference("policy")],
            resource_ref: reference("resource"),
            port_binding_refs: Vec::new(),
        },
        inputs: crate::system_extension::CallbackInputs {
            payload: None,
            state: None,
        },
    }
}

// r[verify molten.addressable_actor.public_native_consumer]
#[test]
fn native_guest_checkpoint_and_recovery_keep_exact_canonical_semantic_bytes() {
    let initialized =
        invoke(&envelope(crate::system_extension::CallbackKind::Initialize)).expect("initialize callback");
    let mut request = envelope(crate::system_extension::CallbackKind::Request);
    request.inputs.state = initialized.state;
    request.inputs.payload = Some(add("one", 7).materialize().expect("counter payload"));
    let applied = invoke(&request).expect("request callback");
    assert!(applied.effects.is_empty());
    let mut checkpoint = envelope(crate::system_extension::CallbackKind::Checkpoint);
    checkpoint.inputs.state = applied.state.clone();
    let selected = invoke(&checkpoint).expect("checkpoint callback");
    assert_eq!(selected.checkpoint, applied.state);
    let mut recover = envelope(crate::system_extension::CallbackKind::Recover);
    recover.inputs.state = applied.state.clone();
    recover.inputs.payload = selected.checkpoint;
    let restored = invoke(&recover).expect("recovery callback");
    assert_eq!(restored.state, applied.state);
    recover.inputs.payload = Some(initial().materialize().expect("wrong checkpoint"));
    assert!(invoke(&recover).is_err());
    recover.context.service_id = reference("other actor");
    assert!(invoke(&recover).is_err());
}
