

struct RunChoices {
    scenario_label: &'static str,
    policy_ref: &'static str,
    scheduler_key: &'static str,
    input_message: &'static str,
    request_payload: &'static str,
    response_payload: &'static str,
    decision: &'static str,
    action: &'static str,
    receipt: &'static str,
    output: &'static str,
    after_state: &'static str,
}

struct EffectRefs {
    scheduler_ref: String,
    input_ref: String,
    effect_request_ref: String,
    effect_response_ref: String,
    policy_decision_ref: String,
}

struct OutputRefs {
    action_ref: String,
    receipt_ref: String,
    output_ref: String,
}

struct StateRefs {
    before_state_ref: String,
    after_state_ref: String,
}

fn run_parts(variant: ReplayFixtureVariant) -> Result<ReplayRunParts> {
    let choices = run_choices(variant);
    let identity = run_identity_value(choices.scenario_label, choices.policy_ref);
    let identity_ref = canonical_hash(&identity)?;
    let effects = effect_refs(&choices, &identity_ref)?;
    let outputs = output_refs(&choices, &effects)?;
    let states = state_refs(&choices, &identity_ref, &outputs)?;
    let effect_log = effect_log_value(&effects);
    let effect_log_ref = canonical_hash(&effect_log)?;
    let turn_journal = turn_journal_value(&effects, &outputs, &states);
    let turn_journal_ref = canonical_hash(&turn_journal)?;
    Ok(ReplayRunParts {
        identity,
        identity_ref,
        scheduler_ref: effects.scheduler_ref,
        input_ref: effects.input_ref,
        effect_request_ref: effects.effect_request_ref,
        effect_response_ref: effects.effect_response_ref,
        policy_decision_ref: effects.policy_decision_ref,
        action_ref: outputs.action_ref,
        receipt_ref: outputs.receipt_ref,
        output_ref: outputs.output_ref,
        after_state_ref: states.after_state_ref,
        turn_journal,
        turn_journal_ref,
        effect_log,
        effect_log_ref,
    })
}

fn run_choices(variant: ReplayFixtureVariant) -> RunChoices {
    RunChoices {
        scenario_label: match variant {
            ReplayFixtureVariant::ChangedIdentity => "fixture:changed-identity",
            _ => "fixture:baseline",
        },
        policy_ref: match variant {
            ReplayFixtureVariant::ChangedIdentity => DEFAULT_REVOCATION_REF,
            _ => DEFAULT_POLICY_REF,
        },
        scheduler_key: match variant {
            ReplayFixtureVariant::ChangedScheduler => "logical:0:priority:1:queue:0:actor:helper",
            _ => "logical:0:priority:0:queue:0:actor:helper",
        },
        input_message: match variant {
            ReplayFixtureVariant::ChangedInput => "message:changed",
            _ => "message:root-to-helper",
        },
        request_payload: match variant {
            ReplayFixtureVariant::ChangedEffectRequest => "logical-now:changed-sequence",
            ReplayFixtureVariant::MissingRecordedEffect => "network:live-fetch",
            _ => "logical-now:turn-0001",
        },
        response_payload: match variant {
            ReplayFixtureVariant::ChangedEffectResponse => "logical-time:43",
            ReplayFixtureVariant::MissingRecordedEffect => "denied:missing-recorded-response",
            _ => "logical-time:42",
        },
        decision: match variant {
            ReplayFixtureVariant::ChangedPolicyDecision => "deny",
            _ => "pass",
        },
        action: match variant {
            ReplayFixtureVariant::ChangedAction => "assert:alternate-output",
            _ => "assert:helper-output",
        },
        receipt: match variant {
            ReplayFixtureVariant::ChangedReceipt => "receipt:alternate",
            _ => "receipt:turn-0001",
        },
        output: match variant {
            ReplayFixtureVariant::ChangedOutput => "output:alternate",
            _ => "output:helper-ack",
        },
        after_state: match variant {
            ReplayFixtureVariant::ChangedStateHash => "after:changed",
            _ => "after:committed",
        },
    }
}

fn effect_refs(choices: &RunChoices, identity_ref: &str) -> Result<EffectRefs> {
    let scheduler_ref = canonical_hash(&record("deterministic-scheduler-key-v1", vec![string(choices.scheduler_key)]))?;
    let input_ref = canonical_hash(&record("deterministic-fixture-input-v1", vec![
        string(choices.input_message),
        record("identity-ref", vec![string(identity_ref)]),
    ]))?;
    let effect_request_ref = canonical_hash(&record("deterministic-effect-request-v1", vec![
        string("clock"),
        string(choices.request_payload),
        record("input-ref", vec![string(&input_ref)]),
        record("profile", vec![string("replay")]),
    ]))?;
    let effect_response_ref = canonical_hash(&record("deterministic-effect-response-v1", vec![
        string(choices.response_payload),
        record("request-ref", vec![string(&effect_request_ref)]),
        record("source", vec![string("recorded-effect-log")]),
    ]))?;
    let policy_decision_ref = canonical_hash(&record("deterministic-policy-decision-v1", vec![
        string(choices.decision),
        record("policy-ref", vec![string(choices.policy_ref)]),
        record("input-ref", vec![string(&input_ref)]),
        record("effect-response-ref", vec![string(&effect_response_ref)]),
    ]))?;
    Ok(EffectRefs {
        scheduler_ref,
        input_ref,
        effect_request_ref,
        effect_response_ref,
        policy_decision_ref,
    })
}

fn output_refs(choices: &RunChoices, effects: &EffectRefs) -> Result<OutputRefs> {
    let action_ref = canonical_hash(&record("deterministic-action-v1", vec![
        string(choices.action),
        record("policy-decision-ref", vec![string(&effects.policy_decision_ref)]),
    ]))?;
    let receipt_ref = canonical_hash(&record("deterministic-turn-receipt-v1", vec![
        string(choices.receipt),
        record("action-ref", vec![string(&action_ref)]),
    ]))?;
    let output_ref = canonical_hash(&record("deterministic-output-v1", vec![
        string(choices.output),
        record("receipt-ref", vec![string(&receipt_ref)]),
    ]))?;
    Ok(OutputRefs {
        action_ref,
        receipt_ref,
        output_ref,
    })
}

fn state_refs(choices: &RunChoices, identity_ref: &str, outputs: &OutputRefs) -> Result<StateRefs> {
    let before_state_ref = canonical_hash(&record("deterministic-state-v1", vec![
        string("before"),
        record("identity-ref", vec![string(identity_ref)]),
    ]))?;
    let after_state_ref = canonical_hash(&record("deterministic-state-v1", vec![
        string(choices.after_state),
        record("before-state-ref", vec![string(&before_state_ref)]),
        record("output-ref", vec![string(&outputs.output_ref)]),
    ]))?;
    Ok(StateRefs {
        before_state_ref,
        after_state_ref,
    })
}

fn effect_log_value(effects: &EffectRefs) -> IoValue {
    record("deterministic-effect-log-v1", vec![
        string(DETERMINISTIC_EFFECT_LOG_SCHEMA),
        record("handler-profile-ref", vec![string(DEFAULT_HANDLER_PROFILE_REF)]),
        sequence(vec![record("effect-entry-v1", vec![
            record("sequence", vec![string("0")]),
            record("effect-kind", vec![string("clock")]),
            record("request-ref", vec![string(&effects.effect_request_ref)]),
            record("response-ref", vec![string(&effects.effect_response_ref)]),
        ])]),
    ])
}

fn turn_journal_value(effects: &EffectRefs, outputs: &OutputRefs, states: &StateRefs) -> IoValue {
    record("deterministic-turn-journal-v1", vec![
        string(DETERMINISTIC_TURN_JOURNAL_SCHEMA),
        record("turn-id", vec![string("turn:0001")]),
        record("actor-id", vec![string("actor:helper")]),
        record("scheduler-key-ref", vec![string(&effects.scheduler_ref)]),
        record("input-ref", vec![string(&effects.input_ref)]),
        record("before-state-ref", vec![string(&states.before_state_ref)]),
        record("effect-request-ref", vec![string(&effects.effect_request_ref)]),
        record("effect-response-ref", vec![string(&effects.effect_response_ref)]),
        record("policy-decision-ref", vec![string(&effects.policy_decision_ref)]),
        record("action-ref", vec![string(&outputs.action_ref)]),
        record("receipt-ref", vec![string(&outputs.receipt_ref)]),
        record("output-ref", vec![string(&outputs.output_ref)]),
        record("after-state-ref", vec![string(&states.after_state_ref)]),
    ])
}

fn run_identity_value(scenario_label: &'static str, policy_ref: &'static str) -> IoValue {
    record("deterministic-run-identity-v1", vec![
        string(DETERMINISTIC_RUN_IDENTITY_SCHEMA),
        record("scenario", vec![string(scenario_label)]),
        record("artifact-ref", vec![string(DEFAULT_ARTIFACT_REF)]),
        record("dependency-closure-ref", vec![string(DEFAULT_CLOSURE_REF)]),
        record("initial-state-ref", vec![string(DEFAULT_INITIAL_STATE_REF)]),
        sequence(vec![string(DEFAULT_SCHEMA_REF)]),
        sequence(vec![string(policy_ref)]),
        sequence(vec![string(DEFAULT_CAPABILITY_REF)]),
        sequence(vec![string(DEFAULT_REVOCATION_REF)]),
        record("handler-profile-ref", vec![string(DEFAULT_HANDLER_PROFILE_REF)]),
        record("seed-ref", vec![string(DEFAULT_SEED_REF)]),
        sequence(vec![string(DEFAULT_RUNTIME_REF), string(DEFAULT_TOOL_REF)]),
    ])
}
