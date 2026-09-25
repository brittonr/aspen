
fn extend_transcript_parse_refs(input: &TranscriptParseInput, refs: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
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

fn extend_cloned_refs(refs: &mut impl crate::bounded::VecSink<String>, values: &[String], field: &str) -> Result<()> {
    for value in values {
        push_ref(refs, value.clone(), field)?;
    }
    Ok(())
}

fn push_ref(refs: &mut impl crate::bounded::VecSink<String>, value_ref: String, field: &str) -> Result<()> {
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
