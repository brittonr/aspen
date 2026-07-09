pub const EFFECT_LOG_VALIDATION_SCHEMA: &str = "molten.determinism.effect-log-validation.v1";

const EFFECT_LOG_DIAGNOSTIC_LIMIT: usize = 16;
const EFFECT_LOG_ENTRY_LIMIT: usize = 1024;
const EFFECT_LOG_FIRST_SEQUENCE: u64 = 0;
const EFFECT_LOG_SEQUENCE_STEP: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectLogEntry {
    pub sequence: u64,
    pub effect_kind: String,
    pub run_identity_ref: String,
    pub handler_profile_ref: String,
    pub turn_ref: String,
    pub boundary_ref: String,
    pub request_ref: String,
    pub response_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsumedEffect {
    pub sequence: u64,
    pub effect_kind: String,
    pub request_ref: String,
    pub response_ref: String,
    pub boundary_ref: String,
    pub used_live_fallback: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct EffectLogValidationInput<'a> {
    pub expected_run_identity_ref: &'a str,
    pub expected_handler_profile_ref: &'a str,
    pub entries: &'a [EffectLogEntry],
    pub consumed: &'a [ConsumedEffect],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectLogValidation {
    pub decision: String,
    pub validation_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

pub fn validate_effect_log(input: EffectLogValidationInput<'_>) -> Result<EffectLogValidation> {
    validate_effect_log_input(input)?;
    let mut diagnostics = Vec::new();
    if let Some(diagnostic) = first_effect_log_diagnostic(input)? {
        crate::bounded::push_bounded(
            &mut diagnostics,
            diagnostic,
            EFFECT_LOG_DIAGNOSTIC_LIMIT,
            "effect log diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let value = effect_log_validation_value(input, &decision, &diagnostics)?;
    let validation_ref = canonical_hash(&value)?;
    Ok(EffectLogValidation {
        decision,
        validation_ref,
        diagnostics,
        value,
    })
}

fn validate_replay_run_effects(parts: ReplayRunParts) -> Result<EffectLogValidation> {
    let entry = EffectLogEntry {
        sequence: EFFECT_LOG_FIRST_SEQUENCE,
        effect_kind: "clock".to_string(),
        run_identity_ref: parts.identity_ref.clone(),
        handler_profile_ref: DEFAULT_HANDLER_PROFILE_REF.to_string(),
        turn_ref: parts.turn_journal_ref.clone(),
        boundary_ref: parts.scheduler_ref.clone(),
        request_ref: parts.effect_request_ref.clone(),
        response_ref: parts.effect_response_ref.clone(),
    };
    let consumed = ConsumedEffect {
        sequence: EFFECT_LOG_FIRST_SEQUENCE,
        effect_kind: "clock".to_string(),
        request_ref: parts.effect_request_ref.clone(),
        response_ref: parts.effect_response_ref.clone(),
        boundary_ref: parts.scheduler_ref.clone(),
        used_live_fallback: false,
    };
    validate_effect_log(EffectLogValidationInput {
        expected_run_identity_ref: &parts.identity_ref,
        expected_handler_profile_ref: DEFAULT_HANDLER_PROFILE_REF,
        entries: &[entry],
        consumed: &[consumed],
    })
}

fn first_effect_log_diagnostic(input: EffectLogValidationInput<'_>) -> Result<Option<String>> {
    if let Some(diagnostic) = profile_or_identity_diagnostic(input) {
        return Ok(Some(diagnostic));
    }
    if let Some(diagnostic) = sequence_diagnostic(input.entries)? {
        return Ok(Some(diagnostic));
    }
    if let Some(diagnostic) = duplicate_request_diagnostic(input.entries)? {
        return Ok(Some(diagnostic));
    }
    if let Some(diagnostic) = binding_mismatch_diagnostic(input.entries, input.consumed)? {
        return Ok(Some(diagnostic));
    }
    if let Some(diagnostic) = unconsumed_extra_diagnostic(input.entries, input.consumed)? {
        return Ok(Some(diagnostic));
    }
    if let Some(diagnostic) = missing_consumed_diagnostic(input.entries, input.consumed)? {
        return Ok(Some(diagnostic));
    }
    if input.consumed.iter().any(|consumed| consumed.used_live_fallback) {
        return Ok(Some("replay attempted live effect fallback".to_string()));
    }
    Ok(None)
}

fn validate_effect_log_input(input: EffectLogValidationInput<'_>) -> Result<()> {
    validate_content_ref(input.expected_run_identity_ref)?;
    validate_content_ref(input.expected_handler_profile_ref)?;
    validate_effect_log_count(input.entries.len(), "effect log entries")?;
    validate_effect_log_count(input.consumed.len(), "consumed effects")?;
    for entry in input.entries {
        validate_effect_entry(entry.clone())?;
    }
    for consumed in input.consumed {
        validate_consumed_effect(consumed.clone())?;
    }
    Ok(())
}

fn validate_effect_entry(entry: EffectLogEntry) -> Result<()> {
    validate_effect_kind(&entry.effect_kind)?;
    validate_content_ref(&entry.run_identity_ref)?;
    validate_content_ref(&entry.handler_profile_ref)?;
    validate_content_ref(&entry.turn_ref)?;
    validate_content_ref(&entry.boundary_ref)?;
    validate_content_ref(&entry.request_ref)?;
    validate_content_ref(&entry.response_ref)
}

fn validate_consumed_effect(consumed: ConsumedEffect) -> Result<()> {
    validate_effect_kind(&consumed.effect_kind)?;
    validate_content_ref(&consumed.boundary_ref)?;
    validate_content_ref(&consumed.request_ref)?;
    validate_content_ref(&consumed.response_ref)
}

fn effect_log_validation_value(
    input: EffectLogValidationInput<'_>,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("effect-log-validation-v1", vec![
        string(EFFECT_LOG_VALIDATION_SCHEMA),
        record("decision", vec![string(decision)]),
        record("run-identity", vec![string(input.expected_run_identity_ref)]),
        record("handler-profile", vec![string(input.expected_handler_profile_ref)]),
        record("entries", vec![sequence(input.entries.iter().cloned().map(effect_entry_value).collect())]),
        record("consumed", vec![sequence(input.consumed.iter().cloned().map(consumed_effect_value).collect())]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("ordered-effect-log"), string(decision)]),
            record("check", vec![string("request-response-bound"), string(decision)]),
            record("check", vec![string("no-live-effect-fallback"), string(decision)]),
            record("check", vec![string("evidence-only-no-authority"), string("pass")]),
        ])]),
    ]))
}
