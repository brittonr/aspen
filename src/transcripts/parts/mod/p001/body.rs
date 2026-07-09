
pub fn parse_transcript_stanza(value: &IoValue) -> Result<TranscriptStanza> {
    let fields = value
        .collect_simple_record("transcript-stanza-v1", Some(TRANSCRIPT_STANZA_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <transcript-stanza-v1 ...>"))?;
    require_schema(&fields[0], TRANSCRIPT_STANZA_SCHEMA, "transcript stanza")?;
    let input = value_to_iovalue(&fields[4]);
    let input_fields = simple_record(&input, "input", 1)?;
    let inline = value_to_iovalue(&input_fields[0]);
    let inline_fields = simple_record(&inline, "inline", 2)?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "bounded-stanza", "transcript stanza")?;
    Ok(TranscriptStanza {
        stanza_ref: canonical_hash(value)?,
        index: record_u64(&fields[1], "index")?,
        kind: record_string(&fields[2], "kind")?,
        modifiers: record_modifier_sequence(&fields[3])?,
        content_ref: required_ref(&inline_fields[0], "stanza content ref")?,
        content: required_string(&inline_fields[1], "stanza content")?,
        declared_refs: record_ref_sequence(&fields[5], "refs")?,
        value: value.clone(),
    })
}

pub fn run_transcript(transcript: &TranscriptArtifact, input: &TranscriptRunInput) -> Result<TranscriptRun> {
    if matches!(input.mode, TranscriptRunMode::ForkDenied | TranscriptRunMode::InPlaceDenied) {
        return denied_run(transcript, input);
    }
    if let Some(run) = cached_run(transcript, input)? {
        return Ok(run);
    }

    let state_root = match input.mode {
        TranscriptRunMode::Fresh => temp_state_root("fresh")?,
        TranscriptRunMode::Save => match input.save_root.clone() {
            Some(save_root) => save_root,
            None => temp_state_root("save")?,
        },
        TranscriptRunMode::ForkDenied | TranscriptRunMode::InPlaceDenied => {
            return Err(MoltenError::invalid_harness("denied transcript modes cannot allocate runner state"));
        }
    };
    let mut state = RunnerState::new(state_root.clone())?;
    let mut outcomes = Vec::with_capacity(transcript.stanzas.len());
    for stanza in &transcript.stanzas {
        let outcome = run_stanza(&mut state, transcript, stanza)?;
        state.last_decision = Some(outcome.decision.clone());
        state.last_kind = Some(outcome.kind.clone());
        state.last_diagnostics = outcome.diagnostics.clone();
        state.last_output = outcome.output.clone();
        outcomes.push(outcome);
    }
    let decision = final_decision(&outcomes);
    let diagnostics = outcomes.iter().flat_map(|outcome| outcome.diagnostics.iter().cloned()).collect::<Vec<_>>();
    let refs = refs_for_transcript(transcript, &outcomes);
    let receipt = run_receipt_value(&RunReceiptValueInput {
        operation: "run",
        decision: &decision,
        transcript,
        mode: input.mode.as_str(),
        outcomes: &outcomes,
        output: state.last_output.as_ref(),
        refs,
        diagnostics: &diagnostics,
        checks: &[
            ("fresh-state", "pass"),
            ("canonical-expectations", "pass"),
            ("effect-admission", "pass"),
        ],
    })?;
    let cache_receipt_value = store_run(input, transcript, &decision, &receipt)?;
    let receipt_ref = canonical_hash(&receipt)?;
    Ok(TranscriptRun {
        transcript_ref: transcript.transcript_ref.clone(),
        decision,
        stanza_outcomes: outcomes,
        receipt_value: receipt,
        receipt_ref,
        cache_receipt_value,
        state_root: if matches!(input.mode, TranscriptRunMode::Save) {
            Some(state.root)
        } else {
            None
        },
    })
}

fn denied_run(transcript: &TranscriptArtifact, input: &TranscriptRunInput) -> Result<TranscriptRun> {
    let outcome = denial_outcome(0, "mode", format!("{} mode denied by default", input.mode.as_str()))?;
    let receipt = run_receipt_value(&RunReceiptValueInput {
        operation: "deny",
        decision: DECISION_DENY,
        transcript,
        mode: input.mode.as_str(),
        outcomes: std::slice::from_ref(&outcome),
        output: None,
        refs: refs_for_transcript(transcript, &[]),
        diagnostics: &[format!("{} mode denied by default", input.mode.as_str())],
        checks: &[("in-place-denied", "pass"), ("no-ambient-identity", "pass")],
    })?;
    Ok(TranscriptRun {
        transcript_ref: transcript.transcript_ref.clone(),
        decision: DECISION_DENY.to_string(),
        stanza_outcomes: vec![outcome],
        receipt_ref: canonical_hash(&receipt)?,
        receipt_value: receipt,
        cache_receipt_value: None,
        state_root: None,
    })
}

fn cached_run(transcript: &TranscriptArtifact, input: &TranscriptRunInput) -> Result<Option<TranscriptRun>> {
    let Some(cache_root) = input.cache_root.as_ref() else {
        return Ok(None);
    };
    let cache_key = transcript_cache_key(transcript)?;
    if let Ok(cache_get) = crate::eval_cache::get(
        cache_root,
        &canonical_hash(&crate::eval_cache::key_value(&cache_key)?)?,
        &crate::eval_cache::GetInput {
            current_policy_refs: transcript.policy_refs.clone(),
            current_capability_refs: transcript.capability_refs.clone(),
            current_revocation_refs: transcript.revocation_refs.clone(),
            current_resource_refs: transcript.resource_refs.clone(),
            current_handler_profile_ref: Some(effective_handler_profile_ref(transcript)?),
            semantic: true,
            ..crate::eval_cache::GetInput::default()
        },
    ) && let Some(output) = cache_get.output.as_ref()
        && let Ok(receipt) = parse_transcript_run_receipt(output)
    {
        return Ok(Some(TranscriptRun {
            transcript_ref: transcript.transcript_ref.clone(),
            decision: receipt.decision.clone(),
            stanza_outcomes: Vec::new(),
            receipt_ref: receipt.receipt_ref,
            receipt_value: output.clone(),
            cache_receipt_value: Some(cache_get.receipt_value),
            state_root: None,
        }));
    }
    Ok(None)
}

fn store_run(
    input: &TranscriptRunInput,
    transcript: &TranscriptArtifact,
    decision: &str,
    receipt: &IoValue,
) -> Result<Option<IoValue>> {
    if decision == DECISION_PASS
        && let Some(cache_root) = input.cache_root.as_ref()
    {
        let cache_key = transcript_cache_key(transcript)?;
        let put = crate::eval_cache::put(cache_root, &cache_key, &crate::eval_cache::ValueInput {
            tier: crate::eval_cache::TIER_SIMULATED.to_string(),
            status: crate::eval_cache::STATUS_PASS.to_string(),
            output: Some(receipt.clone()),
            dependency_refs: cache_key.dependency_refs.clone(),
            policy_refs: cache_key.policy_refs.clone(),
            evidence_refs: Vec::new(),
            diagnostics: Vec::new(),
        })?;
        Ok(Some(put.receipt_value))
    } else {
        Ok(None)
    }
}

pub fn parse_transcript_run_receipt(value: &IoValue) -> Result<TranscriptRunReceipt> {
    let fields = value
        .collect_simple_record("transcript-run-receipt-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <transcript-run-receipt-v1 ...>"))?;
    let field_count = fields.fields_iter().count();
    if field_count != TRANSCRIPT_RUN_RECEIPT_FIELD_COUNT && field_count != TRANSCRIPT_RUN_RECEIPT_LEGACY_FIELD_COUNT {
        return Err(MoltenError::invalid_harness(format!(
            "transcript run receipt field count {field_count} is unsupported"
        )));
    }
    require_schema(&fields[0], TRANSCRIPT_RUN_RECEIPT_SCHEMA, "transcript run receipt")?;
    let outcomes = record_ref_sequence(&fields[5], "outcomes")?;
    let checks = parse_checks(&fields[field_count - 1])?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("transcript run receipt missing checks"));
    }
    Ok(TranscriptRunReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        transcript_ref: record_ref(&fields[3], "transcript")?,
        mode: record_string(&fields[4], "mode")?,
        outcome_refs: outcomes,
        value: value.clone(),
    })
}

pub fn render_transcript(transcript: &TranscriptArtifact, run: Option<&TranscriptRun>) -> Result<String> {
    let mut rendered = String::new();
    rendered.push_str(&format!("# Transcript {}\n\n", transcript.transcript_ref));
    let outcomes = run.map(|run| &run.stanza_outcomes[..]).unwrap_or(&[]);
    for stanza in &transcript.stanzas {
        rendered.push_str(&format!("```{}\n{}\n```\n", stanza.kind, stanza.content.trim_end()));
        if stanza.has_modifier("hide") {
            rendered.push_str("<!-- transcript output hidden; evidence retained -->\n\n");
            continue;
        }
        if let Some(outcome) = outcomes.iter().find(|outcome| outcome.index == stanza.index) {
            rendered.push_str(&format!("decision: {}\n", outcome.decision));
            if let Some(output) = outcome.output.as_ref() {
                rendered.push_str("```preserves-output\n");
                rendered.push_str(&crate::secrets::redacted_text(output, None)?);
                rendered.push_str("\n```\n");
            }
            if !outcome.diagnostics.is_empty() {
                rendered.push_str(&format!("diagnostics: {}\n", outcome.diagnostics.join("; ")));
            }
            rendered.push('\n');
        }
    }
    if let Some(run) = run {
        rendered.push_str(&format!("\nFinal decision: {}\nReceipt: {}\n", run.decision, run.receipt_ref));
    }
    Ok(rendered)
}

pub fn transcript_cache_key(transcript: &TranscriptArtifact) -> Result<crate::eval_cache::KeyInput> {
    let handler_profile_ref = effective_handler_profile_ref(transcript)?;
    let tool_ref = canonical_hash(&record("transcript-runner-tool", vec![string("molten-local-transcript-runner")]))?;
    let cache_dependency_refs = transcript_cache_dependency_refs(transcript)?;
    let mut key = crate::eval_cache::transcript_run_key_placeholder(&crate::eval_cache::TranscriptRunKeyInput {
        transcript_ref: &transcript.transcript_ref,
        closure_hash: &transcript.dependency_closure_hash,
        dependency_refs: &cache_dependency_refs,
        handler_profile_ref: &handler_profile_ref,
        harness_ref: &tool_ref,
        harness_version: RUNNER_TOOL_VERSION,
    })?;
    key.artifact_refs.extend(transcript.artifact_refs.iter().cloned());
    key.artifact_refs.sort();
    key.artifact_refs.dedup();
    key.schema_refs = transcript.schema_refs.clone();
    key.policy_refs = transcript.policy_refs.clone();
    key.capability_refs = transcript.capability_refs.clone();
    key.revocation_refs = transcript.revocation_refs.clone();
    key.resource_refs = transcript.resource_refs.clone();
    key.effect_manifest_refs = transcript.effect_manifest_refs.clone();
    if let Some(seed_ref) = transcript.seed_ref.as_ref() {
        key.assumption_refs.push(seed_ref.clone());
    }
    if let Some(logical_time_ref) = transcript_logical_time_ref(transcript.logical_time)? {
        key.assumption_refs.push(logical_time_ref);
    }
    key.assumption_refs.extend(transcript.expected_refs.iter().cloned());
    Ok(key)
}

impl TranscriptStanza {
    fn has_modifier(&self, name: &str) -> bool {
        self.modifiers.iter().any(|modifier| modifier.name == name)
    }
}

impl RunnerState {
    fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root).map_err(MoltenError::from)?;
        let registry = root.join("registry");
        let ledger = root.join("ledger");
        let storage = root.join("typed-storage");
        let cache = root.join("eval-cache");
        fs::create_dir_all(&registry).map_err(MoltenError::from)?;
        fs::create_dir_all(&ledger).map_err(MoltenError::from)?;
        fs::create_dir_all(&storage).map_err(MoltenError::from)?;
        fs::create_dir_all(&cache).map_err(MoltenError::from)?;
        Ok(Self {
            root,
            registry,
            storage,
            cache,
            last_output: None,
            last_decision: None,
            last_kind: None,
            last_diagnostics: Vec::new(),
            last_artifact_ref: None,
        })
    }
}
