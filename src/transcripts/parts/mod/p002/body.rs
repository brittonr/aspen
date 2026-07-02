
fn parse_markdown_stanzas(source: &str) -> Result<Vec<TranscriptStanza>> {
    let mut stanzas = Vec::new();
    let mut prose = String::new();
    let mut fence_info = None::<String>;
    let mut fence_content = String::new();
    for line in source.lines() {
        if let Some(info) = fence_info.as_deref() {
            if line.trim() == "```" {
                push_stanza_from_fence(StanzaFenceInput {
                    info,
                    content: fence_content.trim_end_matches('\n').to_string(),
                    stanzas: &mut stanzas,
                })?;
                fence_info = None;
                fence_content.clear();
            } else {
                fence_content.push_str(line);
                fence_content.push('\n');
            }
            continue;
        }
        if let Some(info) = line.strip_prefix("```") {
            flush_prose(FlushProseInput {
                prose: &mut prose,
                stanzas: &mut stanzas,
            })?;
            fence_info = Some(info.trim().to_string());
        } else {
            prose.push_str(line);
            prose.push('\n');
        }
    }
    if fence_info.is_some() {
        return Err(MoltenError::invalid_harness("unterminated transcript fenced block"));
    }
    flush_prose(FlushProseInput {
        prose: &mut prose,
        stanzas: &mut stanzas,
    })?;
    Ok(stanzas)
}

struct FlushProseInput<'a> {
    prose: &'a mut String,
    stanzas: &'a mut Vec<TranscriptStanza>,
}

fn flush_prose(input: FlushProseInput<'_>) -> Result<()> {
    let trimmed = input.prose.trim();
    if !trimmed.is_empty() {
        push_stanza(PushStanzaInput {
            kind: KIND_COMMENT,
            modifiers: Vec::new(),
            content: trimmed.to_string(),
            declared_refs: Vec::new(),
            stanzas: input.stanzas,
        })?;
    }
    input.prose.clear();
    Ok(())
}

struct StanzaFenceInput<'a> {
    info: &'a str,
    content: String,
    stanzas: &'a mut Vec<TranscriptStanza>,
}

fn push_stanza_from_fence(input: StanzaFenceInput<'_>) -> Result<()> {
    if input.info.is_empty() {
        return push_stanza(PushStanzaInput {
            kind: KIND_COMMENT,
            modifiers: Vec::new(),
            content: input.content,
            declared_refs: Vec::new(),
            stanzas: input.stanzas,
        });
    }
    let mut tokens = input.info.split_whitespace();
    let first = tokens.next().unwrap_or_default();
    let mut first_parts = first.split(':');
    let kind = first_parts.next().unwrap_or_default();
    validate_kind(kind)?;
    let mut modifiers = first_parts.map(parse_modifier_token).collect::<Result<Vec<_>>>()?;
    for token in tokens {
        modifiers.push(parse_modifier_token(token)?);
    }
    push_stanza(PushStanzaInput {
        kind,
        modifiers,
        content: input.content,
        declared_refs: Vec::new(),
        stanzas: input.stanzas,
    })
}

struct PushStanzaInput<'a> {
    kind: &'a str,
    modifiers: Vec<TranscriptModifier>,
    content: String,
    declared_refs: Vec<String>,
    stanzas: &'a mut Vec<TranscriptStanza>,
}

fn push_stanza(input: PushStanzaInput<'_>) -> Result<()> {
    let index = input.stanzas.len() as u64;
    let content_ref = canonical_hash(&string(&input.content))?;
    let modifier_values = input.modifiers.iter().map(modifier_value).collect::<Vec<_>>();
    let value = record("transcript-stanza-v1", vec![
        string(TRANSCRIPT_STANZA_SCHEMA),
        record("index", vec![u64_value(index)]),
        record("kind", vec![string(input.kind)]),
        record("modifiers", vec![sequence(modifier_values)]),
        record("input", vec![record("inline", vec![string(&content_ref), string(&input.content)])]),
        record("refs", vec![refs_sequence(&input.declared_refs)]),
        checks_value(&["bounded-stanza", "no-ambient-shell"]),
    ]);
    input.stanzas.push(parse_transcript_stanza(&value)?);
    Ok(())
}

fn run_stanza(
    state: &mut RunnerState,
    transcript: &TranscriptArtifact,
    stanza: &TranscriptStanza,
) -> Result<StanzaOutcome> {
    if stanza.has_modifier("skip") {
        return stanza_outcome(stanza, DECISION_SKIP, None, vec!["stanza skipped by modifier".to_string()]);
    }
    match execute_stanza(state, transcript, stanza) {
        Ok(output) => {
            if stanza.has_modifier("error") {
                stanza_outcome(stanza, DECISION_DENY, output, vec![
                    "stanza succeeded but :error expected failure".to_string(),
                ])
            } else if stanza.has_modifier("bug") {
                stanza_outcome(stanza, DECISION_KNOWN_BUG, output, vec!["known bug stanza recorded".to_string()])
            } else {
                stanza_outcome(stanza, DECISION_PASS, output, Vec::new())
            }
        }
        Err(error) => {
            let diagnostic = error.to_string();
            if stanza.has_modifier("error") {
                stanza_outcome(stanza, DECISION_PASS, None, vec![diagnostic])
            } else if stanza.has_modifier("bug") {
                stanza_outcome(stanza, DECISION_KNOWN_BUG, None, vec![diagnostic])
            } else {
                stanza_outcome(stanza, DECISION_ERROR, None, vec![diagnostic])
            }
        }
    }
}

fn execute_stanza(
    state: &mut RunnerState,
    _transcript: &TranscriptArtifact,
    stanza: &TranscriptStanza,
) -> Result<Option<IoValue>> {
    match stanza.kind.as_str() {
        KIND_COMMENT => Ok(None),
        KIND_POLICY => {
            let value = parse_text(&stanza.content)?;
            Ok(Some(value))
        }
        KIND_ARTIFACT | KIND_PRESERVES => {
            let value = parse_text(&stanza.content)?;
            if let Some(record) = value.collect_simple_record("artifact-v1", None) {
                let artifact_ref = canonical_hash(&value)?;
                state.last_artifact_ref = Some(artifact_ref);
                drop(record);
            }
            Ok(Some(value))
        }
        KIND_MOLTEN_CLI => execute_molten_cli(state, &stanza.content),
        KIND_EXPECT => execute_expectation(state, &stanza.content),
        other => Err(MoltenError::invalid_harness(format!("unsupported transcript stanza kind {other}"))),
    }
}

fn execute_molten_cli(state: &mut RunnerState, content: &str) -> Result<Option<IoValue>> {
    let args = content.split_whitespace().collect::<Vec<_>>();
    if args.is_empty() {
        return Err(MoltenError::invalid_harness("empty molten-cli stanza"));
    }
    if args.first() != Some(&"test") {
        return Err(MoltenError::invalid_harness("molten-cli stanzas must start with `test`"));
    }
    match args.get(1).copied() {
        Some("artifact") => execute_artifact_cli(state, &args[2..]),
        Some("schema") => execute_schema_cli(state, &args[2..]),
        Some("storage") => execute_storage_cli(state, &args[2..]),
        Some("cache") => execute_cache_cli(state, &args[2..]),
        Some("report") => execute_report_cli(state),
        Some(other) => {
            Err(MoltenError::invalid_harness(format!("unsupported transcript molten-cli test command {other}")))
        }
        None => Err(MoltenError::invalid_harness("missing molten-cli test subcommand")),
    }
}

fn execute_artifact_cli(state: &mut RunnerState, args: &[&str]) -> Result<Option<IoValue>> {
    match args.first().copied() {
        Some("install") => {
            let kind = option_value(args, "--kind").unwrap_or("artifact");
            let payload = state.last_output.clone().ok_or_else(|| {
                MoltenError::invalid_harness("artifact install requires prior preserves/artifact stanza output")
            })?;
            let install =
                crate::artifacts::install_artifact(&state.registry, &crate::artifacts::ArtifactInstallInput {
                    kind: kind.to_string(),
                    payload,
                    schema_refs: vec![local_ref("transcript-artifact-schema", kind)?],
                    dependency_refs: Vec::new(),
                    effect_manifest_ref: None,
                    policy_refs: vec![local_ref("transcript-artifact-policy", kind)?],
                    evidence_refs: vec![local_ref("transcript-artifact-evidence", kind)?],
                    installer_ref: local_ref("transcript-runner", kind)?,
                    capability_refs: vec![local_ref("transcript-artifact-capability", kind)?],
                })?;
            state.last_artifact_ref = Some(install.artifact_ref.clone());
            Ok(Some(install.artifact.value))
        }
        Some("list") => {
            let refs = crate::artifacts::list_artifacts(&state.registry, None)?
                .iter()
                .map(|artifact| string(&artifact.artifact_ref))
                .collect();
            Ok(Some(record("artifact-list", vec![sequence(refs)])))
        }
        Some("closure") => {
            let artifact_ref = args
                .get(1)
                .map(|value| (*value).to_string())
                .or_else(|| state.last_artifact_ref.clone())
                .ok_or_else(|| MoltenError::invalid_harness("artifact closure requires an artifact ref"))?;
            let closure = crate::artifacts::dependency_closure(&state.registry, &[artifact_ref])?;
            Ok(Some(closure.receipt_value))
        }
        Some(other) => Err(MoltenError::invalid_harness(format!("unsupported transcript artifact command {other}"))),
        None => Err(MoltenError::invalid_harness("missing transcript artifact command")),
    }
}

fn execute_schema_cli(state: &mut RunnerState, args: &[&str]) -> Result<Option<IoValue>> {
    match args.first().copied() {
        Some("identity") => {
            let schema_ref = option_value(args, "--schema-ref")
                .map(str::to_string)
                .unwrap_or(local_ref("transcript-schema", "identity")?);
            let mode = option_value(args, "--mode").unwrap_or(crate::schema_identity::MODE_STRUCTURAL).to_string();
            let shape = state
                .last_output
                .clone()
                .ok_or_else(|| MoltenError::invalid_harness("schema identity requires prior preserves shape output"))?;
            let value = crate::schema_identity::identity_value(&crate::schema_identity::IdentityInput {
                mode,
                schema_ref,
                shape,
                brand_ref: None,
                metadata_refs: vec![local_ref("transcript-schema-metadata", "identity")?],
                policy_refs: vec![local_ref("transcript-schema-policy", "identity")?],
                evidence_refs: vec![local_ref("transcript-schema-evidence", "identity")?],
            })?;
            Ok(Some(value))
        }
        Some(other) => Err(MoltenError::invalid_harness(format!("unsupported transcript schema command {other}"))),
        None => Err(MoltenError::invalid_harness("missing transcript schema command")),
    }
}
