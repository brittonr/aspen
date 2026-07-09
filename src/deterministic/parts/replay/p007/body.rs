fn profile_or_identity_diagnostic(input: EffectLogValidationInput<'_>) -> Option<String> {
    for entry in input.entries {
        if entry.run_identity_ref != input.expected_run_identity_ref {
            return Some(format!("effect entry {} has wrong run identity", entry.sequence));
        }
        if entry.handler_profile_ref != input.expected_handler_profile_ref {
            return Some(format!("effect entry {} has wrong handler profile", entry.sequence));
        }
    }
    None
}

fn sequence_diagnostic(entries: &[EffectLogEntry]) -> Result<Option<String>> {
    let mut expected = EFFECT_LOG_FIRST_SEQUENCE;
    let mut seen = OrderedSet::new();
    for entry in entries {
        if !seen.insert(entry.sequence) {
            return Ok(Some(format!("duplicate effect sequence {}", entry.sequence)));
        }
        if entry.sequence != expected {
            return Ok(Some(format!(
                "effect sequence expected {expected} but found {}",
                entry.sequence
            )));
        }
        expected = expected
            .checked_add(EFFECT_LOG_SEQUENCE_STEP)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("effect sequence overflow"))?;
    }
    Ok(None)
}

fn duplicate_request_diagnostic(entries: &[EffectLogEntry]) -> Result<Option<String>> {
    let mut seen = OrderedSet::new();
    for entry in entries {
        if !seen.insert(entry.request_ref.clone()) {
            return Ok(Some(format!("duplicate effect request ref at sequence {}", entry.sequence)));
        }
    }
    Ok(None)
}

fn binding_mismatch_diagnostic(entries: &[EffectLogEntry], consumed: &[ConsumedEffect]) -> Result<Option<String>> {
    for consumed_effect in consumed {
        if let Some(entry) = entries.iter().find(|entry| entry.sequence == consumed_effect.sequence) {
            if entry.effect_kind != consumed_effect.effect_kind {
                return Ok(Some(format!("effect kind mismatch at sequence {}", consumed_effect.sequence)));
            }
            if entry.request_ref != consumed_effect.request_ref {
                return Ok(Some(format!("request ref mismatch at sequence {}", consumed_effect.sequence)));
            }
            if entry.response_ref != consumed_effect.response_ref {
                return Ok(Some(format!("response ref mismatch at sequence {}", consumed_effect.sequence)));
            }
            if entry.boundary_ref != consumed_effect.boundary_ref {
                return Ok(Some(format!("boundary ref mismatch at sequence {}", consumed_effect.sequence)));
            }
        }
    }
    Ok(None)
}

fn unconsumed_extra_diagnostic(entries: &[EffectLogEntry], consumed: &[ConsumedEffect]) -> Result<Option<String>> {
    let consumed_sequences = consumed.iter().map(|effect| effect.sequence).collect::<OrderedSet<_>>();
    for entry in entries {
        if !consumed_sequences.contains(&entry.sequence) {
            return Ok(Some(format!("unconsumed effect entry at sequence {}", entry.sequence)));
        }
    }
    Ok(None)
}

fn missing_consumed_diagnostic(entries: &[EffectLogEntry], consumed: &[ConsumedEffect]) -> Result<Option<String>> {
    let entry_sequences = entries.iter().map(|entry| entry.sequence).collect::<OrderedSet<_>>();
    for consumed_effect in consumed {
        if !entry_sequences.contains(&consumed_effect.sequence) {
            return Ok(Some(format!("missing recorded effect at sequence {}", consumed_effect.sequence)));
        }
    }
    Ok(None)
}

fn effect_entry_value(entry: EffectLogEntry) -> IoValue {
    record("effect-entry", vec![
        record("sequence", vec![u64_value(entry.sequence)]),
        record("kind", vec![string(&entry.effect_kind)]),
        record("run-identity", vec![string(&entry.run_identity_ref)]),
        record("handler-profile", vec![string(&entry.handler_profile_ref)]),
        record("turn", vec![string(&entry.turn_ref)]),
        record("boundary", vec![string(&entry.boundary_ref)]),
        record("request", vec![string(&entry.request_ref)]),
        record("response", vec![string(&entry.response_ref)]),
    ])
}

fn consumed_effect_value(consumed: ConsumedEffect) -> IoValue {
    record("consumed-effect", vec![
        record("sequence", vec![u64_value(consumed.sequence)]),
        record("kind", vec![string(&consumed.effect_kind)]),
        record("request", vec![string(&consumed.request_ref)]),
        record("response", vec![string(&consumed.response_ref)]),
        record("boundary", vec![string(&consumed.boundary_ref)]),
        record("live-fallback", vec![string(if consumed.used_live_fallback { "true" } else { "false" })]),
    ])
}

fn validate_effect_log_count(count: usize, label: &str) -> Result<()> {
    if count > EFFECT_LOG_ENTRY_LIMIT {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "{label} count {count} exceeds bound {EFFECT_LOG_ENTRY_LIMIT}"
        )))
    } else {
        Ok(())
    }
}

fn validate_effect_kind(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("effect kind cannot be empty"));
    }
    if value.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness("effect kind must be lowercase ascii token"))
    }
}
