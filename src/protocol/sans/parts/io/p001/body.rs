
fn transition_receipt_facts_value(input: TransitionFactsValueInput<'_>) -> Result<IoValue> {
    Ok(record("sans-io-protocol-transition-v1", vec![
        string(SANS_IO_TRANSITION_SCHEMA),
        field_string("decision", input.decision),
        field_string("protocol", &input.input.state.protocol_id),
        field_string("event", &input.input.event.event_kind),
        field_string("message", &input.input.event.message_ref),
        field_string("freshness", &input.input.event.freshness_ref),
        field_string("before-state", &input.input.state.state_ref),
        field_string("after-state", input.after_state_ref),
        field_string("state-delta", input.state_delta_ref.unwrap_or("none")),
        field_string("limit-profile", &input.input.facts.limit_profile_ref),
        field_sequence("authority", ref_values(&input.input.facts.authority_refs)?),
        field_sequence("policy", ref_values(&input.input.facts.policy_refs)?),
        field_sequence("replay", ref_values(&input.input.facts.replay_refs)?),
        field_sequence("resource", ref_values(&input.input.facts.resource_refs)?),
        field_sequence("outbound", ref_values(input.outbound_envelope_refs)?),
        field_sequence("effects", ref_values(input.effect_intent_refs)?),
        field_sequence("effect-responses", ref_values(&input.input.facts.effect_response_refs)?),
        field_sequence("diagnostics", string_values(input.diagnostics)?),
    ]))
}

fn validate_phase(phase: &str) -> Result<()> {
    match phase {
        PHASE_INIT | PHASE_ACTIVE | PHASE_CLOSED => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported protocol phase {other}"))),
    }
}

fn validate_event_kind(kind: &str) -> Result<()> {
    match kind {
        EVENT_OPEN | EVENT_MESSAGE | EVENT_CLOSE => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported protocol event kind {other}"))),
    }
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(refs.len(), MAX_OUTPUTS, label)?;
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} {reference}: {error}")))
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![crate::preserves_rail::sequence(values)])
}

fn string(value: &str) -> IoValue {
    crate::preserves_rail::string(value)
}

fn ref_values(refs: &[String]) -> Result<Vec<IoValue>> {
    validate_refs(refs, "sans-io ref")?;
    Ok(refs.iter().map(|reference| string(reference)).collect())
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_diagnostic_bound(values.len())?;
    Ok(values.iter().map(|value| string(value)).collect())
}

fn ensure_diagnostic_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_DIAGNOSTICS, "sans-io protocol diagnostics")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn refs(label: &str) -> Vec<String> {
        vec![local_ref(label)]
    }

    fn input() -> ProtocolCoreInput {
        ProtocolCoreInput {
            state: ProtocolCoreState {
                protocol_id: "node-control-sans-io-fixture".to_string(),
                phase: PHASE_ACTIVE.to_string(),
                state_ref: local_ref("before-state"),
                sequence: 1,
            },
            event: ProtocolCoreEvent {
                event_kind: EVENT_MESSAGE.to_string(),
                message_ref: local_ref("message"),
                freshness_ref: local_ref("freshness"),
                sequence: 2,
                requires_authority: true,
                requires_policy: true,
                requires_replay: true,
                malformed: false,
            },
            facts: ProtocolCoreFacts {
                limit_profile_ref: local_ref("limits"),
                authority_refs: refs("authority"),
                policy_refs: refs("policy"),
                replay_refs: refs("replay"),
                resource_refs: refs("resource"),
                effect_response_refs: refs("effect-response"),
            },
        }
    }

    // r[verify molten.runtime_patterns.sans_io_protocol_core]
    // r[verify molten.runtime_patterns.sans_io_explicit_inputs]
    // r[verify molten.runtime_patterns.sans_io_transition_outputs]
    // r[verify molten.testing.sans_io_positive_negative_fixtures]
    #[test]
    fn same_inputs_produce_same_transition_outputs() {
        let first = evaluate_protocol_transition(&input()).expect("first transition");
        let second = evaluate_protocol_transition(&input()).expect("second transition");
        assert_eq!(first, second);
        assert_eq!(first.decision, DECISION_PASS);
        assert_eq!(first.outbound_envelope_refs.len(), 1);
        assert_eq!(first.effect_intent_refs.len(), 1);
    }

    #[test]
    fn missing_explicit_evidence_and_malformed_messages_deny_without_outputs() {
        let mut denied = input();
        denied.event.malformed = true;
        denied.facts.authority_refs.clear();
        denied.facts.policy_refs.clear();
        denied.facts.replay_refs.clear();
        let transition = evaluate_protocol_transition(&denied).expect("denied transition");
        assert_eq!(transition.decision, DECISION_DENY);
        assert!(transition.state_delta_ref.is_none());
        assert!(transition.outbound_envelope_refs.is_empty());
        assert!(transition.effect_intent_refs.is_empty());
        assert!(transition.diagnostics.iter().any(|diagnostic| diagnostic == "missing-explicit-authority-fact"));
        assert!(transition.diagnostics.iter().any(|diagnostic| diagnostic == "malformed-protocol-message"));
    }

    // r[verify molten.runtime_patterns.sans_io_shell_adapter]
    #[test]
    fn shell_drain_denies_pre_admission_mutation_and_missing_gates() {
        let transition = evaluate_protocol_transition(&input()).expect("transition");
        let drained = drain_shell_outputs_after_gates(&ShellDrainInput {
            transition,
            transport_admission_refs: Vec::new(),
            authority_admission_refs: Vec::new(),
            policy_admission_refs: Vec::new(),
            resource_admission_refs: Vec::new(),
            replay_admission_refs: Vec::new(),
            speculative_mutation_observed: true,
        })
        .expect("drain");
        assert_eq!(drained.decision, DECISION_DENY);
        assert!(drained.diagnostics.iter().any(|diagnostic| diagnostic == "pre-admission-shell-mutation"));
        assert!(
            drained
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "missing-transport-admission-for-envelope")
        );
    }

    // r[verify molten.runtime_patterns.sans_io_replay_binding]
    #[test]
    fn replay_fixture_binds_transition_state_and_effect_refs() {
        let input = input();
        let transition = evaluate_protocol_transition(&input).expect("transition");
        let drained = drain_shell_outputs_after_gates(&ShellDrainInput {
            transition: transition.clone(),
            transport_admission_refs: refs("transport-admission"),
            authority_admission_refs: refs("authority-admission"),
            policy_admission_refs: refs("policy-admission"),
            resource_admission_refs: refs("resource-admission"),
            replay_admission_refs: refs("replay-admission"),
            speculative_mutation_observed: false,
        })
        .expect("drain");
        assert_eq!(drained.decision, DECISION_PASS);
        let replay = sans_io_replay_fixture_value(&input, &transition, &drained).expect("replay fixture");
        let text = crate::preserves_rail::to_text(&replay).expect("replay text");
        assert!(text.contains("sans-io-replay-fixture-v1"));
        assert!(text.contains(&transition.transition_ref));
    }
}
