
fn execute_storage_cli(state: &mut RunnerState, args: &[&str]) -> Result<Option<IoValue>> {
    match args.first().copied() {
        Some("put") => {
            let namespace = option_value(args, "--namespace").unwrap_or("transcript").to_string();
            let key = option_value(args, "--key").unwrap_or("value").to_string();
            let schema_ref = option_value(args, "--schema-ref").map(str::to_string);
            let value = state
                .last_output
                .clone()
                .ok_or_else(|| MoltenError::invalid_harness("storage put requires prior preserves output"))?;
            let admission = crate::typed_storage::Admission::local_fixture(&format!("transcript:{namespace}:{key}"));
            let put = crate::typed_storage::put_value(&state.storage, &crate::typed_storage::PutInput {
                namespace,
                key,
                schema_ref,
                value,
                producer_ref: local_ref("transcript-storage-producer", "put")?,
                policy_refs: vec![admission.policy_ref.clone()],
                evidence_refs: admission.evidence_refs.clone(),
                admission,
            })?;
            Ok(Some(put.typed_ref_value))
        }
        Some("get") => {
            let namespace = option_value(args, "--namespace").unwrap_or("transcript");
            let key = option_value(args, "--key").unwrap_or("value");
            let schema_ref = option_value(args, "--schema-ref");
            let admission = crate::typed_storage::Admission::local_fixture(&format!("transcript:{namespace}:{key}"));
            let get = crate::typed_storage::get_value(&state.storage, namespace, key, schema_ref, &admission)?;
            Ok(Some(get.value))
        }
        Some(other) => Err(MoltenError::invalid_harness(format!("unsupported transcript storage command {other}"))),
        None => Err(MoltenError::invalid_harness("missing transcript storage command")),
    }
}

fn execute_cache_cli(state: &mut RunnerState, args: &[&str]) -> Result<Option<IoValue>> {
    match args.first().copied() {
        Some("status") => {
            let status = crate::eval_cache::status(&state.cache)?;
            Ok(Some(record("eval-cache-status", vec![
                u64_value(status.keys as u64),
                u64_value(status.values as u64),
                u64_value(status.tombstones as u64),
                u64_value(status.receipts as u64),
            ])))
        }
        Some("list") => {
            let entries = crate::eval_cache::list(&state.cache, &crate::eval_cache::ListFilter::default())?;
            Ok(Some(record("eval-cache-list", vec![sequence(
                entries.iter().map(|entry| string(&entry.key_ref)).collect(),
            )])))
        }
        Some(other) => Err(MoltenError::invalid_harness(format!("unsupported transcript cache command {other}"))),
        None => Err(MoltenError::invalid_harness("missing transcript cache command")),
    }
}

fn execute_report_cli(state: &RunnerState) -> Result<Option<IoValue>> {
    let value = state
        .last_output
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("report command requires prior output"))?;
    let validation = crate::harness::validate_report_value(value)?;
    Ok(Some(record("report-validation", vec![string(validation.report_ref)])))
}

fn execute_expectation(state: &RunnerState, content: &str) -> Result<Option<IoValue>> {
    let expectation = parse_text(content)?;
    if let Some(fields) = expectation.collect_simple_record("expect-output", Some(1)) {
        let expected = value_to_iovalue(&fields[0]);
        let actual = state
            .last_output
            .as_ref()
            .ok_or_else(|| MoltenError::invalid_harness("expect-output requires previous output"))?;
        let expected_ref = canonical_hash(&expected)?;
        let actual_ref = canonical_hash(actual)?;
        if expected_ref != actual_ref {
            return Err(MoltenError::invalid_harness(format!(
                "expect-output mismatch: expected {expected_ref}, got {actual_ref}"
            )));
        }
        return Ok(Some(expectation));
    }
    if let Some(fields) = expectation.collect_simple_record("expect-decision", Some(1)) {
        let expected = required_string(&fields[0], "expected decision")?;
        let actual = state
            .last_decision
            .as_ref()
            .ok_or_else(|| MoltenError::invalid_harness("expect-decision requires previous outcome"))?;
        if &expected != actual {
            return Err(MoltenError::invalid_harness(format!(
                "expect-decision mismatch: expected {expected}, got {actual}"
            )));
        }
        return Ok(Some(expectation));
    }
    if let Some(fields) = expectation.collect_simple_record("expect-kind", Some(1)) {
        let expected = required_string(&fields[0], "expected kind")?;
        let actual = state
            .last_kind
            .as_ref()
            .ok_or_else(|| MoltenError::invalid_harness("expect-kind requires previous outcome"))?;
        if &expected != actual {
            return Err(MoltenError::invalid_harness(format!(
                "expect-kind mismatch: expected {expected}, got {actual}"
            )));
        }
        return Ok(Some(expectation));
    }
    if let Some(fields) = expectation.collect_simple_record("expect-error-contains", Some(1)) {
        let needle = required_string(&fields[0], "expected error substring")?;
        let haystack = state.last_output.as_ref().map(to_text).transpose()?.unwrap_or_default();
        if !haystack.contains(&needle) {
            return Err(MoltenError::invalid_harness(format!("expected previous output/error to contain {needle:?}")));
        }
        return Ok(Some(expectation));
    }
    Err(MoltenError::invalid_harness("unsupported transcript expectation"))
}

fn stanza_outcome(
    stanza: &TranscriptStanza,
    decision: &str,
    output: Option<IoValue>,
    diagnostics: Vec<String>,
) -> Result<StanzaOutcome> {
    validate_decision(decision)?;
    let output_ref = output.as_ref().map(canonical_hash).transpose()?;
    let value = record("transcript-stanza-outcome-v1", vec![
        string(TRANSCRIPT_STANZA_OUTCOME_SCHEMA),
        record("index", vec![u64_value(stanza.index)]),
        record("kind", vec![string(&stanza.kind)]),
        record("stanza", vec![string(&stanza.stanza_ref)]),
        record("decision", vec![string(decision)]),
        record("output", vec![optional_ref_value(output_ref.as_deref())]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&["stanza-outcome-bound", "hidden-evidence-preserved"]),
    ]);
    Ok(StanzaOutcome {
        outcome_ref: canonical_hash(&value)?,
        index: stanza.index,
        kind: stanza.kind.clone(),
        decision: decision.to_string(),
        output,
        diagnostics,
        value,
    })
}

fn denial_outcome(index: u64, kind: &str, diagnostic: String) -> Result<StanzaOutcome> {
    let stanza = TranscriptStanza {
        stanza_ref: local_ref("transcript-denial-stanza", &format!("{index}:{kind}"))?,
        index,
        kind: kind.to_string(),
        modifiers: Vec::new(),
        content: diagnostic.clone(),
        content_ref: local_ref("transcript-denial-content", &diagnostic)?,
        declared_refs: Vec::new(),
        value: record("transcript-denial-placeholder", vec![string(&diagnostic)]),
    };
    stanza_outcome(&stanza, DECISION_DENY, None, vec![diagnostic])
}

fn run_receipt_value(input: &RunReceiptValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.transcript_ref, "transcript ref")?;
    validate_decision(input.decision)?;
    let output_ref = input.output.map(canonical_hash).transpose()?;
    Ok(record("transcript-run-receipt-v1", vec![
        string(TRANSCRIPT_RUN_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("transcript", vec![string(input.transcript_ref)]),
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
        checks_value_from_pairs(input.checks),
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
    } else if kind == "shell" || kind == "sh" || kind == "bash" {
        Err(MoltenError::invalid_harness("ambient shell transcript stanzas are denied by default"))
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported transcript stanza kind {kind}")))
    }
}

fn validate_modifier(modifier: &str) -> Result<()> {
    if matches!(modifier, "error" | "bug" | "hide" | "skip" | "requires" | "seed" | "profile") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported transcript modifier {modifier}")))
    }
}

fn validate_parse_input(input: &TranscriptParseInput) -> Result<()> {
    validate_refs(&input.dependency_refs, "transcript dependency ref")?;
    if let Some(hash) = input.dependency_closure_hash.as_ref() {
        validate_ref(hash, "transcript dependency closure hash")?;
    }
    if let Some(handler) = input.handler_profile_ref.as_ref() {
        validate_ref(handler, "transcript handler profile ref")?;
    }
    validate_refs(&input.policy_refs, "transcript policy ref")?;
    validate_refs(&input.capability_refs, "transcript capability ref")?;
    validate_refs(&input.revocation_refs, "transcript revocation ref")?;
    if let Some(seed) = input.seed_ref.as_ref() {
        validate_ref(seed, "transcript seed ref")?;
    }
    validate_refs(&input.expected_refs, "transcript expected ref")
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
    refs.extend(transcript.dependency_refs.iter().cloned());
    refs.extend(transcript.policy_refs.iter().cloned());
    refs.extend(transcript.capability_refs.iter().cloned());
    refs.extend(transcript.revocation_refs.iter().cloned());
    refs.extend(transcript.expected_refs.iter().cloned());
    refs.extend(transcript.stanzas.iter().map(|stanza| stanza.stanza_ref.clone()));
    refs.extend(outcomes.iter().map(|outcome| outcome.outcome_ref.clone()));
    if let Some(handler) = transcript.handler_profile_ref.as_ref() {
        refs.push(handler.clone());
    }
    if let Some(seed) = transcript.seed_ref.as_ref() {
        refs.push(seed.clone());
    }
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
