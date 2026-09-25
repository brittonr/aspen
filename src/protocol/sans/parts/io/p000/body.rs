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
