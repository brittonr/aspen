
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
    execute_state_expectation(state, expectation)
}

/// Trace-marker, absent-output, kind, error, and raw-output expectations.
fn execute_state_expectation(state: &RunnerState, expectation: IoValue) -> Result<Option<IoValue>> {
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
