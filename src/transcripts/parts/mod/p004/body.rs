
fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

fn parse_optional_ref_value(value: &PreservesValue<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn parse_optional_string_value(value: &PreservesValue<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_string(&fields[0], "optional string").map(Some);
    }
    required_string(value, "optional string").map(Some)
}

fn parse_optional_u64_value(value: &PreservesValue<IoValue>) -> Result<Option<u64>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_u64(&fields[0], "optional u64").map(Some);
    }
    required_u64(value, "optional u64").map(Some)
}

fn record_string(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_u64(value: &PreservesValue<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_u64(&record[0], label)
}

fn record_ref(value: &PreservesValue<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_optional_ref(value: &PreservesValue<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_optional_u64(value: &PreservesValue<IoValue>, label: &str) -> Result<Option<u64>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_u64_value(&record[0])
}

fn record_ref_sequence(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn record_sequence(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<PreservesValue<IoValue>>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    Ok(required_sequence(&record[0], label)?.iter().cloned().collect())
}

fn record_modifier_sequence(value: &PreservesValue<IoValue>) -> Result<Vec<TranscriptModifier>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, "modifiers", 1)?;
    let items = required_sequence(&record[0], "modifiers")?;
    ensure_count_at_most(items.len(), MAX_TRANSCRIPT_SEQUENCE_ITEMS, "transcript modifiers")?;
    let mut modifiers = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let fields = simple_record(&item, "modifier", 2)?;
        let name = required_string(&fields[0], "modifier name")?;
        validate_modifier(&name)?;
        push_bounded(
            &mut modifiers,
            TranscriptModifier {
                name,
                value: parse_optional_string_value(&fields[1])?,
            },
            MAX_TRANSCRIPT_SEQUENCE_ITEMS,
            "transcript modifiers",
        )?;
    }
    Ok(modifiers)
}

fn parse_ref_sequence_value(value: &PreservesValue<IoValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        refs.push(required_ref(item, label)?);
    }
    Ok(refs)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<Set<_>>().into_iter().collect()
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &PreservesValue<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "checks")?;
    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("transcript check {name} has status {status}")));
        }
        parsed.push(name);
    }
    Ok(parsed)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &PreservesValue<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, PreservesRecord<PreservesValue<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(
    value: &'a PreservesValue<IoValue>,
    field: &str,
) -> Result<std::borrow::Cow<'a, Vec<PreservesValue<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn required_string(value: &PreservesValue<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &PreservesValue<IoValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn required_u64(value: &PreservesValue<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    values.push_item(value);
    Ok(())
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref, got {value_ref}: {error}"))
    })
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn transcript_dependency_binding_refs(input: &TranscriptParseInput, stanzas: &[TranscriptStanza]) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    extend_transcript_parse_refs(input, &mut refs)?;
    for stanza in stanzas {
        extend_cloned_refs(&mut refs, &stanza.declared_refs, "transcript stanza declared ref")?;
    }
    Ok(sorted_unique(&refs))
}

fn transcript_all_binding_refs(transcript: &TranscriptArtifact) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    extend_cloned_refs(&mut refs, &transcript.dependency_refs, "transcript dependency ref")?;
    extend_cloned_refs(&mut refs, &transcript.artifact_refs, "transcript artifact ref")?;
    extend_cloned_refs(&mut refs, &transcript.schema_refs, "transcript schema ref")?;
    extend_cloned_refs(&mut refs, &transcript.policy_refs, "transcript policy ref")?;
    extend_cloned_refs(&mut refs, &transcript.capability_refs, "transcript capability ref")?;
    extend_cloned_refs(&mut refs, &transcript.resource_refs, "transcript resource ref")?;
    extend_cloned_refs(&mut refs, &transcript.effect_manifest_refs, "transcript effect manifest ref")?;
    extend_cloned_refs(&mut refs, &transcript.revocation_refs, "transcript revocation ref")?;
    extend_cloned_refs(&mut refs, &transcript.expected_refs, "transcript expected ref")?;
    extend_cloned_refs(&mut refs, &transcript.resolution_refs, "transcript resolution ref")?;
    for stanza in &transcript.stanzas {
        extend_cloned_refs(&mut refs, &stanza.declared_refs, "transcript stanza declared ref")?;
    }
    push_ref(
        &mut refs,
        effective_handler_profile_ref(transcript)?,
        "transcript handler profile ref",
    )?;
    if let Some(seed) = transcript.seed_ref.as_ref() {
        push_ref(&mut refs, seed.clone(), "transcript seed ref")?;
    }
    if let Some(logical_time_ref) = transcript_logical_time_ref(transcript.logical_time)? {
        push_ref(&mut refs, logical_time_ref, "transcript logical time ref")?;
    }
    Ok(sorted_unique(&refs))
}

fn transcript_cache_dependency_refs(transcript: &TranscriptArtifact) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    extend_cloned_refs(&mut refs, &transcript.dependency_refs, "transcript dependency ref")?;
    extend_cloned_refs(&mut refs, &transcript.artifact_refs, "transcript artifact ref")?;
    extend_cloned_refs(&mut refs, &transcript.schema_refs, "transcript schema ref")?;
    extend_cloned_refs(&mut refs, &transcript.resource_refs, "transcript resource ref")?;
    extend_cloned_refs(&mut refs, &transcript.effect_manifest_refs, "transcript effect manifest ref")?;
    extend_cloned_refs(&mut refs, &transcript.resolution_refs, "transcript resolution ref")?;
    for stanza in &transcript.stanzas {
        extend_cloned_refs(&mut refs, &stanza.declared_refs, "transcript stanza declared ref")?;
    }
    Ok(sorted_unique(&refs))
}

fn extend_transcript_parse_refs(input: &TranscriptParseInput, refs: &mut Vec<String>) -> Result<()> {
    extend_cloned_refs(refs, &input.dependency_refs, "transcript dependency ref")?;
    extend_cloned_refs(refs, &input.artifact_refs, "transcript artifact ref")?;
    extend_cloned_refs(refs, &input.schema_refs, "transcript schema ref")?;
    extend_cloned_refs(refs, &input.policy_refs, "transcript policy ref")?;
    extend_cloned_refs(refs, &input.capability_refs, "transcript capability ref")?;
    extend_cloned_refs(refs, &input.resource_refs, "transcript resource ref")?;
    extend_cloned_refs(refs, &input.effect_manifest_refs, "transcript effect manifest ref")?;
    extend_cloned_refs(refs, &input.revocation_refs, "transcript revocation ref")?;
    extend_cloned_refs(refs, &input.expected_refs, "transcript expected ref")?;
    extend_cloned_refs(refs, &input.resolution_refs, "transcript resolution ref")?;
    if let Some(handler) = input.handler_profile_ref.as_ref() {
        push_ref(refs, handler.clone(), "transcript handler profile ref")?;
    }
    if let Some(seed) = input.seed_ref.as_ref() {
        push_ref(refs, seed.clone(), "transcript seed ref")?;
    }
    if let Some(logical_time_ref) = transcript_logical_time_ref(input.logical_time)? {
        push_ref(refs, logical_time_ref, "transcript logical time ref")?;
    }
    Ok(())
}

fn extend_cloned_refs(refs: &mut Vec<String>, values: &[String], field: &str) -> Result<()> {
    for value in values {
        push_ref(refs, value.clone(), field)?;
    }
    Ok(())
}

fn push_ref(refs: &mut Vec<String>, value_ref: String, field: &str) -> Result<()> {
    validate_ref(&value_ref, field)?;
    push_bounded(refs, value_ref, MAX_TRANSCRIPT_SEQUENCE_ITEMS, field)
}

fn transcript_logical_time_ref(logical_time: Option<u64>) -> Result<Option<String>> {
    logical_time
        .map(|value| canonical_hash(&record("transcript-logical-time-v1", vec![u64_value(value)])))
        .transpose()
}

fn effective_handler_profile_ref(transcript: &TranscriptArtifact) -> Result<String> {
    match transcript.handler_profile_ref.as_ref() {
        Some(handler_profile_ref) => Ok(handler_profile_ref.clone()),
        None => default_handler_profile_ref(),
    }
}

fn default_handler_profile_ref() -> Result<String> {
    canonical_hash(&record("transcript-default-handler-profile", vec![string("deterministic-local")]))
}

fn validate_decision(decision: &str) -> Result<()> {
    if matches!(decision, DECISION_PASS | DECISION_DENY | DECISION_ERROR | DECISION_SKIP | DECISION_KNOWN_BUG) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported transcript decision {decision}")))
    }
}
