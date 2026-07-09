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
