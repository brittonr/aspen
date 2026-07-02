
fn step_predicate_receipts(
    step: &super::core::CoreStep,
    before: &crate::runtime::RuntimeSnapshot,
    after: &crate::runtime::RuntimeSnapshot,
) -> Result<Vec<preserves::IOValue>> {
    match step {
        super::core::CoreStep::Observe { actor, pattern } => {
            let observer = RuntimeObserver {
                actor: actor.clone(),
                pattern: pattern.clone(),
            };
            let receipt = crate::runtime::evaluate_observe_initial_delivery(before, &observer)?.receipt;
            Ok(vec![receipt.value])
        }
        super::core::CoreStep::Assert { value, .. } | super::core::CoreStep::Retract { value, .. } => {
            let live_owners = after.assertions.iter().map(|assertion| assertion.actor.clone()).collect();
            let receipt = crate::runtime::evaluate_assertion_visibility(after, value, &live_owners)?.receipt;
            Ok(vec![receipt.value])
        }
        super::core::CoreStep::Send { .. }
        | super::core::CoreStep::Clock { .. }
        | super::core::CoreStep::Random { .. } => Ok(Vec::new()),
    }
}

fn replay_effect_events(
    state: &mut super::core::RuntimeState,
    step: &super::core::CoreStep,
    step_index: u64,
    replay_effect_log: &[super::schema::EffectLogEntry],
    replay_effect_index: &mut usize,
) -> Result<Vec<preserves::IOValue>> {
    let Some(request) = state.begin_effect_for_step(step) else {
        if !is_dataspace_turn(step) {
            return Ok(state.apply_step(step).iter().map(super::schema::event_value).collect());
        }
        let before = state.snapshot();
        let turn = state.begin_turn(step);
        let (runtime_events, receipt) = state.commit_turn_with_predicate_receipt(turn)?;
        let after = state.snapshot();
        let mut events = vec![receipt.value];
        events.extend(runtime_events.iter().map(super::schema::event_value));
        events.extend(step_predicate_receipts(step, &before, &after)?);
        return Ok(events);
    };

    let Some(entry) = replay_effect_log.get(*replay_effect_index) else {
        return Err(divergence(
            "effect-log",
            Some(step_index),
            format!("entry {}", *replay_effect_index),
            "missing",
            "recorded effect log ended before effect request",
        ));
    };
    let request_value = super::schema::event_value(&request);
    let request_hash = crate::preserves_rail::canonical_hash(&request_value)?;
    let recorded_request_hash = crate::preserves_rail::canonical_hash(&entry.request)?;
    if request_hash != recorded_request_hash {
        return Err(divergence(
            "effect-request",
            Some(step_index),
            recorded_request_hash,
            request_hash,
            "effect request does not match recorded log",
        ));
    }

    let (response_sequence, response_value) = super::schema::effect_response_sequence_and_value(&entry.response)?;
    let request_sequence = super::schema::effect_request_sequence(&entry.request)?;
    if response_sequence != request_sequence {
        return Err(divergence(
            "effect-log",
            Some(step_index),
            request_sequence.to_string(),
            response_sequence.to_string(),
            "recorded effect request/response sequence mismatch",
        ));
    }

    let response = state.apply_recorded_effect_response(&request, response_value)?;
    let response_value = super::schema::event_value(&response);
    let response_hash = crate::preserves_rail::canonical_hash(&response_value)?;
    let recorded_response_hash = crate::preserves_rail::canonical_hash(&entry.response)?;
    if response_hash != recorded_response_hash {
        return Err(divergence(
            "effect-response",
            Some(step_index),
            recorded_response_hash,
            response_hash,
            "effect response does not match recorded log",
        ));
    }

    *replay_effect_index += 1;
    with_time_random_handler_receipt(step, step_index, vec![request_value, response_value])
}

fn with_time_random_handler_receipt(
    step: &super::core::CoreStep,
    step_index: u64,
    events: Vec<preserves::IOValue>,
) -> Result<Vec<preserves::IOValue>> {
    let (effect, actor) = match step {
        super::core::CoreStep::Clock { actor } => ("clock", actor.as_str()),
        super::core::CoreStep::Random { actor, .. } => ("random", actor.as_str()),
        super::core::CoreStep::Send { .. }
        | super::core::CoreStep::Observe { .. }
        | super::core::CoreStep::Assert { .. }
        | super::core::CoreStep::Retract { .. } => return Ok(events),
    };
    if events.len() != 2 {
        return Err(MoltenError::invalid_harness(format!(
            "deterministic {effect} handler expected request and response events at step {step_index}"
        )));
    }
    let request_ref = crate::preserves_rail::canonical_hash(&events[0])?;
    let response_ref = crate::preserves_rail::canonical_hash(&events[1])?;
    let handler_binding = crate::preserves_rail::record("time-random-handler-binding-v1", vec![
        crate::preserves_rail::string("local-deterministic"),
        crate::preserves_rail::string(effect),
        crate::preserves_rail::string(actor),
        crate::preserves_rail::u64_value(step_index),
    ]);
    let handler_binding_ref = crate::preserves_rail::canonical_hash(&handler_binding)?;
    let receipt = crate::preserves_rail::record("time-random-handler-receipt-v1", vec![
        crate::preserves_rail::string("molten.effects.time-random-handler.v1"),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string("local-deterministic")]),
        crate::preserves_rail::record("effect", vec![crate::preserves_rail::string(effect)]),
        crate::preserves_rail::record("actor", vec![crate::preserves_rail::string(actor)]),
        crate::preserves_rail::record("request-ref", vec![crate::preserves_rail::string(&request_ref)]),
        crate::preserves_rail::record("handler-binding-ref", vec![crate::preserves_rail::string(&handler_binding_ref)]),
        crate::preserves_rail::record("response-ref", vec![crate::preserves_rail::string(&response_ref)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("deny-by-default-bypassed-only-by-local-test-handler"),
            crate::preserves_rail::string("pass"),
        ])]),
    ]);
    Ok(vec![events[0].clone(), receipt, events[1].clone()])
}

fn divergence(
    kind: impl Into<String>,
    step: Option<u64>,
    expected: impl Into<String>,
    actual: impl Into<String>,
    detail: impl Into<String>,
) -> MoltenError {
    MoltenError::harness_divergence(HarnessDivergence::new(kind, step, expected, actual, detail))
}
