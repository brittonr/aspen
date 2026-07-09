
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
        expect_value_ref(state, &canonical_hash(&expected)?)?;
        return Ok(Some(expectation));
    }
    if let Some(fields) = expectation.collect_simple_record("expect-value-ref", Some(1)) {
        expect_value_ref(state, &required_ref(&fields[0], "expected value ref")?)?;
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
    if let Some(fields) = expectation.collect_simple_record("expect-receipt", Some(2)) {
        let expected_kind = required_string(&fields[0], "expected receipt kind")?;
        let expected_decision = required_string(&fields[1], "expected receipt decision")?;
        expect_receipt(state, &expected_kind, Some(&expected_decision))?;
        return Ok(Some(expectation));
    }
    if let Some(fields) = expectation.collect_simple_record("expect-receipt-kind", Some(1)) {
        let expected_kind = required_string(&fields[0], "expected receipt kind")?;
        expect_receipt(state, &expected_kind, None)?;
        return Ok(Some(expectation));
    }
    if let Some(fields) = expectation.collect_simple_record("expect-failure-class", Some(1)) {
        let expected = required_string(&fields[0], "expected failure class")?;
        expect_failure_class(state, &expected)?;
        return Ok(Some(expectation));
    }
    if let Some(fields) = expectation.collect_simple_record("expect-trace-marker", Some(1)) {
        expect_trace_marker(state, &required_ref(&fields[0], "expected trace marker")?)?;
        return Ok(Some(expectation));
    }
    if expectation.collect_simple_record("expect-output-absent", Some(0)).is_some() {
        if state.last_output.is_some() {
            return Err(MoltenError::invalid_harness("expect-output-absent mismatch: previous output was present"));
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
        let haystack = state.last_diagnostics.join("\n");
        if !haystack.contains(&needle) {
            return Err(MoltenError::invalid_harness(format!("expected previous diagnostics to contain {needle:?}")));
        }
        return Ok(Some(expectation));
    }
    if expectation.collect_simple_record("expect-stdout", Some(1)).is_some()
        || expectation.collect_simple_record("expect-raw-output", Some(1)).is_some()
    {
        return Err(MoltenError::invalid_harness(
            "raw transcript output is diagnostic-only; use a canonical Preserves value or receipt oracle",
        ));
    }
    Err(MoltenError::invalid_harness("unsupported transcript expectation"))
}

fn expect_value_ref(state: &RunnerState, expected_ref: &str) -> Result<()> {
    validate_ref(expected_ref, "expected value ref")?;
    let actual = state
        .last_output
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("canonical value expectation requires previous output"))?;
    let actual_ref = canonical_hash(actual)?;
    if expected_ref != actual_ref {
        return Err(MoltenError::invalid_harness(format!(
            "expect-value-ref mismatch: expected {expected_ref}, got {actual_ref}"
        )));
    }
    Ok(())
}

fn expect_receipt(state: &RunnerState, expected_kind: &str, expected_decision: Option<&str>) -> Result<()> {
    let actual = state
        .last_output
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("receipt expectation requires previous output"))?;
    if actual.collect_simple_record(expected_kind, None).is_none() {
        return Err(MoltenError::invalid_harness(format!(
            "expect-receipt mismatch: previous output is not {expected_kind}"
        )));
    }
    if let Some(expected_decision) = expected_decision {
        validate_decision_or_receipt_decision(expected_decision)?;
        let actual_decision = receipt_decision(actual, expected_kind)?;
        if actual_decision != expected_decision {
            return Err(MoltenError::invalid_harness(format!(
                "expect-receipt decision mismatch: expected {expected_decision}, got {actual_decision}"
            )));
        }
    }
    Ok(())
}

fn expect_failure_class(state: &RunnerState, expected: &str) -> Result<()> {
    let decision = state
        .last_decision
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("failure-class expectation requires previous outcome"))?;
    if decision != DECISION_DENY && decision != DECISION_ERROR && decision != DECISION_KNOWN_BUG {
        return Err(MoltenError::invalid_harness(format!(
            "expect-failure-class mismatch: previous decision was {decision}"
        )));
    }
    if !state.last_diagnostics.iter().any(|diagnostic| diagnostic.contains(expected)) {
        return Err(MoltenError::invalid_harness(format!(
            "expect-failure-class mismatch: diagnostics did not contain {expected:?}"
        )));
    }
    Ok(())
}

fn expect_trace_marker(state: &RunnerState, expected_ref: &str) -> Result<()> {
    validate_ref(expected_ref, "expected trace marker")?;
    let actual = state
        .last_output
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("trace marker expectation requires previous output"))?;
    let actual_text = to_text(actual)?;
    if !actual_text.contains(expected_ref) && canonical_hash(actual)? != expected_ref {
        return Err(MoltenError::invalid_harness(format!(
            "expect-trace-marker mismatch: previous output did not bind {expected_ref}"
        )));
    }
    Ok(())
}

fn receipt_decision(value: &IoValue, expected_kind: &str) -> Result<String> {
    match expected_kind {
        "artifact-receipt-v1" => crate::artifacts::parse_artifact_receipt(value).map(|receipt| receipt.decision),
        "artifact-identity-receipt-v1" => {
            crate::artifacts::parse_artifact_identity_receipt(value).map(|receipt| receipt.decision)
        }
        "eval-cache-receipt-v1" => crate::eval_cache::parse_receipt(value).map(|receipt| receipt.decision),
        "transcript-run-receipt-v1" => parse_transcript_run_receipt(value).map(|receipt| receipt.decision),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported transcript receipt oracle kind {other}"
        ))),
    }
}

fn validate_decision_or_receipt_decision(decision: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny" | "error" | "skip" | "known-bug" | "trace-only") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported receipt decision {decision}")))
    }
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
