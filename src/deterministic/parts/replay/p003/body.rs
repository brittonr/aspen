
fn parse_replay_rollup_receipt(value: &IoValue, rollup_ref: &str) -> Result<ParsedReplayRollup> {
    let fields = value
        .collect_simple_record("deterministic-replay-rollup-v1", Some(10))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected <deterministic-replay-rollup-v1 ...>"))?;
    require_schema_value(&fields[0], DETERMINISTIC_REPLAY_ROLLUP_SCHEMA, "deterministic replay rollup")?;
    let decision = record_string_value(&fields[1], "decision")?;
    validate_replay_decision(&decision)?;
    let total_count = record_u64_value(&fields[2], "total-count")?;
    let pass_count = record_u64_value(&fields[3], "pass-count")?;
    let deny_count = record_u64_value(&fields[4], "deny-count")?;
    let receipt_refs = record_ref_list_value(&fields[5], "receipt-refs")?;
    let divergence_counts = record_divergence_counts_value(&fields[6])?;
    let first_divergence_refs = record_ref_list_value(&fields[7], "first-divergence-refs")?;
    Ok(ParsedReplayRollup {
        rollup_ref: rollup_ref.to_string(),
        decision,
        total_count,
        pass_count,
        deny_count,
        receipt_refs,
        divergence_counts,
        first_divergence_refs,
    })
}

struct ParsedTurnJournalRefs {
    scheduler_ref: String,
    input_ref: String,
    effect_request_ref: String,
    effect_response_ref: String,
    policy_decision_ref: String,
    action_ref: String,
    receipt_ref: String,
    output_ref: String,
    after_state_ref: String,
}

fn parse_fixture_record_run_parts(value: &IoValue) -> Result<ReplayRunParts> {
    let fields = value
        .collect_simple_record("deterministic-fixture-record-v1", Some(FIXTURE_RECORD_FIELD_COUNT))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected <deterministic-fixture-record-v1 ...>"))?;
    require_schema_value(
        &fields[FIXTURE_SCHEMA_INDEX],
        DETERMINISTIC_FIXTURE_RECORD_SCHEMA,
        "deterministic fixture record",
    )?;
    let identity = value_to_iovalue(&fields[FIXTURE_IDENTITY_VALUE_INDEX]);
    let identity_ref = record_string_value(&fields[FIXTURE_IDENTITY_REF_INDEX], "identity-ref")?;
    validate_record_ref_match("identity", &identity, &identity_ref)?;
    let effect_log = value_to_iovalue(&fields[FIXTURE_EFFECT_LOG_VALUE_INDEX]);
    let effect_log_ref = record_string_value(&fields[FIXTURE_EFFECT_LOG_REF_INDEX], "effect-log-ref")?;
    validate_record_ref_match("effect log", &effect_log, &effect_log_ref)?;
    let turn_journal = first_turn_journal(&fields[FIXTURE_TURN_JOURNALS_INDEX])?;
    let turn_journal_ref = canonical_hash(&turn_journal)?;
    let parsed_turn = parse_turn_journal_refs(&turn_journal)?;
    let output_ref = record_string_value(&fields[FIXTURE_OUTPUT_REF_INDEX], "output-ref")?;
    let after_state_ref = record_string_value(&fields[FIXTURE_FINAL_STATE_REF_INDEX], "final-state-ref")?;
    validate_bound_ref("output", &output_ref, &parsed_turn.output_ref)?;
    validate_bound_ref("final state", &after_state_ref, &parsed_turn.after_state_ref)?;
    Ok(ReplayRunParts {
        identity,
        identity_ref,
        scheduler_ref: parsed_turn.scheduler_ref,
        input_ref: parsed_turn.input_ref,
        effect_request_ref: parsed_turn.effect_request_ref,
        effect_response_ref: parsed_turn.effect_response_ref,
        policy_decision_ref: parsed_turn.policy_decision_ref,
        action_ref: parsed_turn.action_ref,
        receipt_ref: parsed_turn.receipt_ref,
        output_ref,
        after_state_ref,
        turn_journal,
        turn_journal_ref,
        effect_log,
        effect_log_ref,
    })
}

fn first_turn_journal(value: &PreservesValue<IoValue>) -> Result<IoValue> {
    let journals = value
        .collect_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("fixture turn journals must be a sequence"))?;
    let first = journals
        .first()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("fixture turn journals must not be empty"))?;
    Ok(value_to_iovalue(first))
}

fn parse_turn_journal_refs(value: &IoValue) -> Result<ParsedTurnJournalRefs> {
    let fields = value
        .collect_simple_record("deterministic-turn-journal-v1", Some(TURN_JOURNAL_FIELD_COUNT))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected <deterministic-turn-journal-v1 ...>"))?;
    require_schema_value(
        &fields[TURN_JOURNAL_SCHEMA_INDEX],
        DETERMINISTIC_TURN_JOURNAL_SCHEMA,
        "deterministic turn journal",
    )?;
    Ok(ParsedTurnJournalRefs {
        scheduler_ref: required_content_ref_record(&fields[TURN_JOURNAL_SCHEDULER_REF_INDEX], "scheduler-key-ref")?,
        input_ref: required_content_ref_record(&fields[TURN_JOURNAL_INPUT_REF_INDEX], "input-ref")?,
        effect_request_ref: required_content_ref_record(
            &fields[TURN_JOURNAL_EFFECT_REQUEST_REF_INDEX],
            "effect-request-ref",
        )?,
        effect_response_ref: required_content_ref_record(
            &fields[TURN_JOURNAL_EFFECT_RESPONSE_REF_INDEX],
            "effect-response-ref",
        )?,
        policy_decision_ref: required_content_ref_record(
            &fields[TURN_JOURNAL_POLICY_DECISION_REF_INDEX],
            "policy-decision-ref",
        )?,
        action_ref: required_content_ref_record(&fields[TURN_JOURNAL_ACTION_REF_INDEX], "action-ref")?,
        receipt_ref: required_content_ref_record(&fields[TURN_JOURNAL_RECEIPT_REF_INDEX], "receipt-ref")?,
        output_ref: required_content_ref_record(&fields[TURN_JOURNAL_OUTPUT_REF_INDEX], "output-ref")?,
        after_state_ref: required_content_ref_record(&fields[TURN_JOURNAL_AFTER_STATE_REF_INDEX], "after-state-ref")?,
    })
}

fn required_content_ref_record(value: &PreservesValue<IoValue>, label: &'static str) -> Result<String> {
    let reference = record_string_value(value, label)?;
    validate_content_ref(&reference)?;
    Ok(reference)
}

fn validate_record_ref_match(label: &str, value: &IoValue, declared_ref: &str) -> Result<()> {
    validate_content_ref(declared_ref)?;
    let actual_ref = canonical_hash(value)?;
    if actual_ref == declared_ref {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "{label} ref mismatch: declared {declared_ref} actual {actual_ref}"
        )))
    }
}

fn validate_bound_ref(label: &str, declared_ref: &str, journal_ref: &str) -> Result<()> {
    validate_content_ref(declared_ref)?;
    if declared_ref == journal_ref {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "fixture {label} ref mismatch: declared {declared_ref} journal {journal_ref}"
        )))
    }
}

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

fn first_divergence(
    expected: &ReplayRunParts,
    actual: &ReplayRunParts,
    variant: ReplayFixtureVariant,
) -> ReplayDivergenceKind {
    if variant == ReplayFixtureVariant::MissingRecordedEffect {
        return ReplayDivergenceKind::LiveEffect;
    }
    first_divergence_between_parts(expected, actual)
}

fn first_divergence_between_parts(expected: &ReplayRunParts, actual: &ReplayRunParts) -> ReplayDivergenceKind {
    if expected.identity_ref != actual.identity_ref {
        return ReplayDivergenceKind::Identity;
    }
    if expected.scheduler_ref != actual.scheduler_ref {
        return ReplayDivergenceKind::Scheduler;
    }
    if expected.input_ref != actual.input_ref {
        return ReplayDivergenceKind::Input;
    }
    if expected.effect_request_ref != actual.effect_request_ref {
        return ReplayDivergenceKind::EffectRequest;
    }
    if expected.effect_response_ref != actual.effect_response_ref {
        return ReplayDivergenceKind::EffectResponse;
    }
    if expected.policy_decision_ref != actual.policy_decision_ref {
        return ReplayDivergenceKind::PolicyDecision;
    }
    if expected.action_ref != actual.action_ref {
        return ReplayDivergenceKind::Action;
    }
    if expected.receipt_ref != actual.receipt_ref {
        return ReplayDivergenceKind::Receipt;
    }
    if expected.output_ref != actual.output_ref {
        return ReplayDivergenceKind::Output;
    }
    if expected.after_state_ref != actual.after_state_ref {
        return ReplayDivergenceKind::StateHash;
    }
    ReplayDivergenceKind::None
}
