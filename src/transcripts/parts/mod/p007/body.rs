
fn run_receipt_value(input: &RunReceiptValueInput<'_>) -> Result<IoValue> {
    validate_ref(&input.transcript.transcript_ref, "transcript ref")?;
    validate_decision(input.decision)?;
    let output_ref = input.output.map(canonical_hash).transpose()?;
    let bindings = transcript_run_bindings_value(input.transcript)?;
    Ok(record("transcript-run-receipt-v1", vec![
        string(TRANSCRIPT_RUN_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("transcript", vec![string(&input.transcript.transcript_ref)]),
        record("mode", vec![string(input.mode)]),
        record("outcomes", vec![refs_sequence(
            &input.outcomes.iter().map(|outcome| outcome.outcome_ref.clone()).collect::<Vec<_>>(),
        )]),
        record("output", vec![optional_ref_value(output_ref.as_deref())]),
        record("refs", vec![refs_sequence(&sorted_unique(&input.refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("outcome-values", vec![sequence(
            input.outcomes.iter().map(|outcome| outcome.value.clone()).collect(),
        )]),
        bindings,
        checks_value_from_pairs(input.checks),
    ]))
}

fn transcript_run_bindings_value(transcript: &TranscriptArtifact) -> Result<IoValue> {
    let handler_profile_ref = effective_handler_profile_ref(transcript)?;
    let logical_time_ref = transcript_logical_time_ref(transcript.logical_time)?;
    Ok(record("bindings", vec![
        record("handler-profile", vec![optional_ref_value(Some(&handler_profile_ref))]),
        record("seed", vec![optional_ref_value(transcript.seed_ref.as_deref())]),
        record("logical-time", vec![optional_ref_value(logical_time_ref.as_deref())]),
        record("artifacts", vec![refs_sequence(&sorted_unique(&transcript.artifact_refs))]),
        record("schemas", vec![refs_sequence(&sorted_unique(&transcript.schema_refs))]),
        record("effects", vec![refs_sequence(&sorted_unique(&transcript.effect_manifest_refs))]),
        record("resources", vec![refs_sequence(&sorted_unique(&transcript.resource_refs))]),
        record("policies", vec![refs_sequence(&sorted_unique(&transcript.policy_refs))]),
        record("capabilities", vec![refs_sequence(&sorted_unique(&transcript.capability_refs))]),
        record("revocations", vec![refs_sequence(&sorted_unique(&transcript.revocation_refs))]),
        record("resolutions", vec![refs_sequence(&sorted_unique(&transcript.resolution_refs))]),
        checks_value(&["profile-seed-effect-resource-bound", "exact-ref-bindings"]),
    ]))
}

fn parse_modifier_token(token: &str) -> Result<TranscriptModifier> {
    let token = token.strip_prefix(':').unwrap_or(token);
    if token.is_empty() {
        return Err(MoltenError::invalid_harness("empty transcript modifier"));
    }
    let (name, value) = token.split_once('=').map_or((token, None), |(name, value)| (name, Some(value)));
    validate_modifier(name)?;
    Ok(TranscriptModifier {
        name: name.to_string(),
        value: value.map(str::to_string),
    })
}

fn modifier_value(modifier: &TranscriptModifier) -> IoValue {
    record("modifier", vec![string(&modifier.name), optional_string_value(modifier.value.as_deref())])
}

fn validate_kind(kind: &str) -> Result<()> {
    if matches!(kind, KIND_MOLTEN_CLI | KIND_PRESERVES | KIND_ARTIFACT | KIND_POLICY | KIND_EXPECT | KIND_COMMENT) {
        Ok(())
    } else if matches!(kind, "shell" | "sh" | "bash") {
        Err(MoltenError::invalid_harness("ambient shell transcript stanzas are denied by default"))
    } else if matches!(kind, "ucm" | "unison" | "unison-transcript") {
        Err(MoltenError::invalid_harness(
            "UCM compatibility is denied; Unison transcripts are prior art only",
        ))
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported transcript stanza kind {kind}")))
    }
}

fn validate_modifier(modifier: &str) -> Result<()> {
    if matches!(
        modifier,
        "error"
            | "bug"
            | "hide"
            | "skip"
            | "requires"
            | "seed"
            | "profile"
            | "artifact-ref"
            | "schema-ref"
            | "policy-ref"
            | "effect-ref"
            | "capability-ref"
            | "resource-ref"
            | "resolution-ref"
    ) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported transcript modifier {modifier}")))
    }
}

fn declared_refs_from_modifiers(modifiers: &[TranscriptModifier]) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    for modifier in modifiers {
        if !is_ref_modifier(&modifier.name) {
            continue;
        }
        let value = modifier.value.as_ref().ok_or_else(|| {
            MoltenError::invalid_harness(format!("transcript modifier {} requires a ref value", modifier.name))
        })?;
        push_ref(&mut refs, value.clone(), "transcript stanza modifier ref")?;
    }
    Ok(sorted_unique(&refs))
}

fn is_ref_modifier(modifier: &str) -> bool {
    matches!(
        modifier,
        "artifact-ref" | "schema-ref" | "policy-ref" | "effect-ref" | "capability-ref" | "resource-ref" | "resolution-ref"
    )
}

fn stanza_binding_denial(transcript: &TranscriptArtifact, stanza: &TranscriptStanza) -> Result<Option<String>> {
    if !stanza_requires_effect_admission(stanza) {
        return Ok(None);
    }
    if !has_bound_ref(stanza, "policy-ref", &transcript.policy_refs) {
        return Ok(Some("transcript side-effect stanza missing policy ref".to_string()));
    }
    if !has_bound_ref(stanza, "capability-ref", &transcript.capability_refs) {
        return Ok(Some("transcript side-effect stanza missing capability ref".to_string()));
    }
    if !has_bound_ref(stanza, "resource-ref", &transcript.resource_refs) {
        return Ok(Some("transcript side-effect stanza missing resource ref".to_string()));
    }
    if !has_bound_ref(stanza, "effect-ref", &transcript.effect_manifest_refs) {
        return Ok(Some("transcript side-effect stanza missing effect manifest ref".to_string()));
    }
    Ok(None)
}

fn stanza_requires_effect_admission(stanza: &TranscriptStanza) -> bool {
    if stanza.kind != KIND_MOLTEN_CLI {
        return false;
    }
    let args = stanza.content.split_whitespace().collect::<Vec<_>>();
    matches!(
        (args.first().copied(), args.get(1).copied(), args.get(2).copied()),
        (Some("test"), Some("artifact"), Some("install")) | (Some("test"), Some("storage"), Some("put"))
    )
}

fn has_bound_ref(stanza: &TranscriptStanza, modifier: &str, transcript_refs: &[String]) -> bool {
    !transcript_refs.is_empty()
        || stanza
            .modifiers
            .iter()
            .any(|candidate| candidate.name == modifier && candidate.value.as_ref().is_some_and(|value| validate_ref(value, modifier).is_ok()))
}

fn stanza_admission_refs(transcript: &TranscriptArtifact, stanza: &TranscriptStanza) -> Result<StanzaAdmissionRefs> {
    Ok(StanzaAdmissionRefs {
        schema_refs: effective_ref_bindings(&transcript.schema_refs, stanza, "schema-ref")?,
        policy_refs: effective_ref_bindings(&transcript.policy_refs, stanza, "policy-ref")?,
        capability_refs: effective_ref_bindings(&transcript.capability_refs, stanza, "capability-ref")?,
        effect_manifest_refs: effective_ref_bindings(&transcript.effect_manifest_refs, stanza, "effect-ref")?,
        resource_refs: effective_ref_bindings(&transcript.resource_refs, stanza, "resource-ref")?,
    })
}

fn effective_ref_bindings(
    transcript_refs: &[String],
    stanza: &TranscriptStanza,
    modifier: &str,
) -> Result<Vec<String>> {
    let mut refs = transcript_refs.to_vec();
    for candidate in &stanza.modifiers {
        if candidate.name != modifier {
            continue;
        }
        let value = candidate.value.as_ref().ok_or_else(|| {
            MoltenError::invalid_harness(format!("transcript modifier {modifier} requires a ref value"))
        })?;
        push_ref(&mut refs, value.clone(), modifier)?;
    }
    Ok(sorted_unique(&refs))
}

fn ref_binding_or_default(refs: &[String], kind: &str, label: &str) -> Result<Vec<String>> {
    if refs.is_empty() {
        Ok(vec![local_ref(kind, label)?])
    } else {
        Ok(refs.to_vec())
    }
}

fn install_evidence_refs(admission: &StanzaAdmissionRefs, kind: &str) -> Result<Vec<String>> {
    let mut refs = vec![local_ref("transcript-artifact-evidence", kind)?];
    extend_cloned_refs(&mut refs, &admission.resource_refs, "transcript resource ref")?;
    Ok(sorted_unique(&refs))
}

fn optional_first_ref(refs: &[String]) -> Option<String> {
    refs.first().cloned()
}

fn validate_parse_input(input: &TranscriptParseInput) -> Result<()> {
    validate_refs(&input.dependency_refs, "transcript dependency ref")?;
    validate_refs(&input.artifact_refs, "transcript artifact ref")?;
    validate_refs(&input.schema_refs, "transcript schema ref")?;
    if let Some(hash) = input.dependency_closure_hash.as_ref() {
        validate_ref(hash, "transcript dependency closure hash")?;
    }
    if let Some(handler) = input.handler_profile_ref.as_ref() {
        validate_ref(handler, "transcript handler profile ref")?;
    }
    validate_refs(&input.policy_refs, "transcript policy ref")?;
    validate_refs(&input.capability_refs, "transcript capability ref")?;
    validate_refs(&input.resource_refs, "transcript resource ref")?;
    validate_refs(&input.effect_manifest_refs, "transcript effect manifest ref")?;
    validate_refs(&input.revocation_refs, "transcript revocation ref")?;
    if let Some(seed) = input.seed_ref.as_ref() {
        validate_ref(seed, "transcript seed ref")?;
    }
    validate_refs(&input.expected_refs, "transcript expected ref")?;
    validate_refs(&input.resolution_refs, "transcript resolution ref")
}

fn final_decision(outcomes: &[StanzaOutcome]) -> String {
    if outcomes
        .iter()
        .any(|outcome| outcome.decision == DECISION_DENY || outcome.decision == DECISION_ERROR)
    {
        DECISION_DENY.to_string()
    } else if outcomes.iter().any(|outcome| outcome.decision == DECISION_KNOWN_BUG) {
        DECISION_KNOWN_BUG.to_string()
    } else {
        DECISION_PASS.to_string()
    }
}

fn refs_for_transcript(transcript: &TranscriptArtifact, outcomes: &[StanzaOutcome]) -> Vec<String> {
    let mut refs = vec![
        transcript.transcript_ref.clone(),
        transcript.source_ref.clone(),
        transcript.dependency_closure_hash.clone(),
    ];
    if let Ok(binding_refs) = transcript_all_binding_refs(transcript) {
        refs.extend(binding_refs);
    }
    refs.extend(transcript.stanzas.iter().map(|stanza| stanza.stanza_ref.clone()));
    refs.extend(outcomes.iter().map(|outcome| outcome.outcome_ref.clone()));
    sorted_unique(&refs)
}

fn option_value<'a>(args: &'a [&str], name: &str) -> Option<&'a str> {
    args.windows(2).find_map(|window| (window[0] == name).then_some(window[1]))
}

fn temp_state_root(label: &str) -> Result<PathBuf> {
    for _ in 0..MAX_TEMP_STATE_ROOT_ATTEMPTS {
        let nonce = TEMP_STATE_ROOT_COUNTER.fetch_add(1, RELAXED);
        let path = std::env::temp_dir().join(format!("molten-transcript-{label}-{}-{nonce}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(MoltenError::from(error)),
        }
    }
    Err(MoltenError::invalid_harness("exhausted bounded transcript temp root attempts"))
}

fn local_ref(kind: &str, label: &str) -> Result<String> {
    canonical_hash(&record("transcript-local-ref", vec![string(kind), string(label)]))
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}
