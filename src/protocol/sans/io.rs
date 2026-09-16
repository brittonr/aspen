type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;
type MoltenError = crate::error::MoltenError;

const SANS_IO_TRANSITION_SCHEMA: &str = "molten.runtime-patterns.sans-io-transition.v1";
const SANS_IO_SHELL_DRAIN_SCHEMA: &str = "molten.runtime-patterns.sans-io-shell-drain.v1";
const SANS_IO_REPLAY_SCHEMA: &str = "molten.runtime-patterns.sans-io-replay-fixture.v1";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const PHASE_INIT: &str = "init";
const PHASE_ACTIVE: &str = "active";
const PHASE_CLOSED: &str = "closed";
const EVENT_OPEN: &str = "open";
const EVENT_MESSAGE: &str = "message";
const EVENT_CLOSE: &str = "close";
const MAX_OUTPUTS: usize = 128;
const MAX_DIAGNOSTICS: usize = 512;
const MAX_SEQUENCE_ADVANCE: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolCoreState {
    pub protocol_id: String,
    pub phase: String,
    pub state_ref: String,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolCoreEvent {
    pub event_kind: String,
    pub message_ref: String,
    pub freshness_ref: String,
    pub sequence: u64,
    pub requires_authority: bool,
    pub requires_policy: bool,
    pub requires_replay: bool,
    pub malformed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolCoreFacts {
    pub limit_profile_ref: String,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub replay_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_response_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolCoreInput {
    pub state: ProtocolCoreState,
    pub event: ProtocolCoreEvent,
    pub facts: ProtocolCoreFacts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolCoreTransition {
    pub decision: String,
    pub before_state_ref: String,
    pub after_state_ref: String,
    pub state_delta_ref: Option<String>,
    pub outbound_envelope_refs: Vec<String>,
    pub effect_intent_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub receipt_facts_value: IoValue,
    pub transition_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellDrainInput {
    pub transition: ProtocolCoreTransition,
    pub transport_admission_refs: Vec<String>,
    pub authority_admission_refs: Vec<String>,
    pub policy_admission_refs: Vec<String>,
    pub resource_admission_refs: Vec<String>,
    pub replay_admission_refs: Vec<String>,
    pub speculative_mutation_observed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellDrainDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub decision_ref: String,
}

// r[impl molten.runtime_patterns.sans_io_protocol_core]
// r[impl molten.runtime_patterns.sans_io_explicit_inputs]
// r[impl molten.runtime_patterns.sans_io_transition_outputs]
pub fn evaluate_protocol_transition(input: &ProtocolCoreInput) -> Result<ProtocolCoreTransition> {
    validate_input(input)?;
    let mut diagnostics = transition_diagnostics(input);
    diagnostics.sort();
    diagnostics.dedup();
    ensure_diagnostic_bound(diagnostics.len())?;
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let state_delta_ref = if decision == DECISION_PASS {
        Some(state_delta_ref(input)?)
    } else {
        None
    };
    let after_state_ref = state_delta_ref.clone().unwrap_or_else(|| input.state.state_ref.clone());
    let outbound_envelope_refs = if decision == DECISION_PASS && input.event.event_kind == EVENT_MESSAGE {
        vec![output_ref("outbound-envelope", &input.event.message_ref)?]
    } else {
        Vec::new()
    };
    let effect_intent_refs = if decision == DECISION_PASS && input.event.event_kind != EVENT_CLOSE {
        vec![output_ref("effect-intent", &input.event.message_ref)?]
    } else {
        Vec::new()
    };
    crate::bounded::ensure_count_at_most(outbound_envelope_refs.len(), MAX_OUTPUTS, "sans-io outbound envelopes")?;
    crate::bounded::ensure_count_at_most(effect_intent_refs.len(), MAX_OUTPUTS, "sans-io effect intents")?;
    let receipt_facts_value = transition_receipt_facts_value(TransitionFactsValueInput {
        decision,
        input,
        after_state_ref: &after_state_ref,
        state_delta_ref: state_delta_ref.as_deref(),
        outbound_envelope_refs: &outbound_envelope_refs,
        effect_intent_refs: &effect_intent_refs,
        diagnostics: &diagnostics,
    })?;
    let transition_ref = crate::preserves_rail::canonical_hash(&receipt_facts_value)?;
    Ok(ProtocolCoreTransition {
        decision: decision.to_string(),
        before_state_ref: input.state.state_ref.clone(),
        after_state_ref,
        state_delta_ref,
        outbound_envelope_refs,
        effect_intent_refs,
        diagnostics,
        receipt_facts_value,
        transition_ref,
    })
}

// r[impl molten.runtime_patterns.sans_io_shell_adapter]
pub fn drain_shell_outputs_after_gates(input: &ShellDrainInput) -> Result<ShellDrainDecision> {
    validate_refs(&input.transport_admission_refs, "sans-io transport admission ref")?;
    validate_refs(&input.authority_admission_refs, "sans-io authority admission ref")?;
    validate_refs(&input.policy_admission_refs, "sans-io policy admission ref")?;
    validate_refs(&input.resource_admission_refs, "sans-io resource admission ref")?;
    validate_refs(&input.replay_admission_refs, "sans-io replay admission ref")?;
    let mut diagnostics = Vec::new();
    if input.speculative_mutation_observed {
        diagnostics.push("pre-admission-shell-mutation".to_string());
    }
    if input.transition.decision != DECISION_PASS {
        diagnostics.push("core-transition-denied-no-shell-effects".to_string());
    }
    if !input.transition.outbound_envelope_refs.is_empty() && input.transport_admission_refs.is_empty() {
        diagnostics.push("missing-transport-admission-for-envelope".to_string());
    }
    if !input.transition.effect_intent_refs.is_empty() && input.resource_admission_refs.is_empty() {
        diagnostics.push("missing-resource-admission-for-effect".to_string());
    }
    if !input.transition.effect_intent_refs.is_empty() && input.policy_admission_refs.is_empty() {
        diagnostics.push("missing-policy-admission-for-effect".to_string());
    }
    if !input.transition.effect_intent_refs.is_empty() && input.replay_admission_refs.is_empty() {
        diagnostics.push("missing-replay-admission-for-effect".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let value = record("sans-io-shell-drain-v1", vec![
        string(SANS_IO_SHELL_DRAIN_SCHEMA),
        field_string("decision", decision),
        field_string("transition", &input.transition.transition_ref),
        field_sequence("transport", ref_values(&input.transport_admission_refs)?),
        field_sequence("authority", ref_values(&input.authority_admission_refs)?),
        field_sequence("policy", ref_values(&input.policy_admission_refs)?),
        field_sequence("resource", ref_values(&input.resource_admission_refs)?),
        field_sequence("replay", ref_values(&input.replay_admission_refs)?),
        field_sequence("diagnostics", string_values(&diagnostics)?),
    ]);
    let decision_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ShellDrainDecision {
        decision: decision.to_string(),
        diagnostics,
        value,
        decision_ref,
    })
}

// r[impl molten.runtime_patterns.sans_io_replay_binding]
pub fn sans_io_replay_fixture_value(
    input: &ProtocolCoreInput,
    transition: &ProtocolCoreTransition,
    shell_decision: &ShellDrainDecision,
) -> Result<IoValue> {
    Ok(record("sans-io-replay-fixture-v1", vec![
        string(SANS_IO_REPLAY_SCHEMA),
        field_string("protocol", &input.state.protocol_id),
        field_string("message", &input.event.message_ref),
        field_string("before-state", &transition.before_state_ref),
        field_string("after-state", &transition.after_state_ref),
        field_string("transition", &transition.transition_ref),
        field_string("shell-drain", &shell_decision.decision_ref),
        field_sequence("outbound", ref_values(&transition.outbound_envelope_refs)?),
        field_sequence("effects", ref_values(&transition.effect_intent_refs)?),
        field_sequence("effect-responses", ref_values(&input.facts.effect_response_refs)?),
    ]))
}

fn validate_input(input: &ProtocolCoreInput) -> Result<()> {
    validate_text("protocol id", &input.state.protocol_id)?;
    validate_phase(&input.state.phase)?;
    validate_ref(&input.state.state_ref, "protocol state ref")?;
    validate_event_kind(&input.event.event_kind)?;
    validate_ref(&input.event.message_ref, "protocol message ref")?;
    validate_ref(&input.event.freshness_ref, "protocol freshness ref")?;
    validate_ref(&input.facts.limit_profile_ref, "protocol limit profile ref")?;
    validate_refs(&input.facts.authority_refs, "protocol authority ref")?;
    validate_refs(&input.facts.policy_refs, "protocol policy ref")?;
    validate_refs(&input.facts.replay_refs, "protocol replay ref")?;
    validate_refs(&input.facts.resource_refs, "protocol resource ref")?;
    validate_refs(&input.facts.effect_response_refs, "protocol effect response ref")
}

fn transition_diagnostics(input: &ProtocolCoreInput) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if input.event.malformed {
        diagnostics.push("malformed-protocol-message".to_string());
    }
    if input.event.sequence > input.state.sequence.saturating_add(MAX_SEQUENCE_ADVANCE) {
        diagnostics.push("protocol-sequence-gap".to_string());
    }
    if input.event.sequence <= input.state.sequence && input.event.event_kind != EVENT_CLOSE {
        diagnostics.push("protocol-replay-or-stale-sequence".to_string());
    }
    if input.event.requires_authority && input.facts.authority_refs.is_empty() {
        diagnostics.push("missing-explicit-authority-fact".to_string());
    }
    if input.event.requires_policy && input.facts.policy_refs.is_empty() {
        diagnostics.push("missing-explicit-policy-fact".to_string());
    }
    if input.event.requires_replay && input.facts.replay_refs.is_empty() {
        diagnostics.push("missing-explicit-replay-fact".to_string());
    }
    if input.state.phase == PHASE_CLOSED && input.event.event_kind != EVENT_OPEN {
        diagnostics.push("closed-protocol-transition-denied".to_string());
    }
    diagnostics
}

fn state_delta_ref(input: &ProtocolCoreInput) -> Result<String> {
    let next_phase = match input.event.event_kind.as_str() {
        EVENT_OPEN | EVENT_MESSAGE => PHASE_ACTIVE,
        EVENT_CLOSE => PHASE_CLOSED,
        _ => input.state.phase.as_str(),
    };
    let value = record("sans-io-state-delta-v1", vec![
        field_string("protocol", &input.state.protocol_id),
        field_string("before", &input.state.state_ref),
        field_string("phase", next_phase),
        field_string("sequence", &input.event.sequence.to_string()),
        field_string("message", &input.event.message_ref),
    ]);
    crate::preserves_rail::canonical_hash(&value)
}

fn output_ref(kind: &str, message_ref: &str) -> Result<String> {
    let value = record("sans-io-output-descriptor-v1", vec![
        field_string("kind", kind),
        field_string("message", message_ref),
    ]);
    crate::preserves_rail::canonical_hash(&value)
}

struct TransitionFactsValueInput<'a> {
    decision: &'a str,
    input: &'a ProtocolCoreInput,
    after_state_ref: &'a str,
    state_delta_ref: Option<&'a str>,
    outbound_envelope_refs: &'a [String],
    effect_intent_refs: &'a [String],
    diagnostics: &'a [String],
}

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
