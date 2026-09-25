
pub fn parse_markdown(source: &str, input: &TranscriptParseInput) -> Result<TranscriptArtifact> {
    validate_parse_input(input)?;
    let source_ref = canonical_hash(&string(source))?;
    let stanzas = parse_markdown_stanzas(source)?;
    let stanza_values = stanzas.iter().map(|stanza| stanza.value.clone()).collect::<Vec<_>>();
    let dependency_refs = sorted_unique(&input.dependency_refs);
    let artifact_refs = sorted_unique(&input.artifact_refs);
    let schema_refs = sorted_unique(&input.schema_refs);
    let resource_refs = sorted_unique(&input.resource_refs);
    let effect_manifest_refs = sorted_unique(&input.effect_manifest_refs);
    let resolution_refs = sorted_unique(&input.resolution_refs);
    let dependency_closure_hash = match input.dependency_closure_hash.as_ref() {
        Some(hash) => hash.clone(),
        None => canonical_hash(&record(
            "transcript-dependency-closure-v1",
            vec![refs_sequence(&transcript_dependency_binding_refs(input, &stanzas)?)]
        ))?,
    };
    let value = record("transcript-artifact-v1", vec![
        string(TRANSCRIPT_ARTIFACT_SCHEMA),
        record("source", vec![string(&source_ref)]),
        record("stanzas", vec![sequence(stanza_values)]),
        record("dependencies", vec![string(&dependency_closure_hash), refs_sequence(&dependency_refs)]),
        record("artifacts", vec![refs_sequence(&artifact_refs)]),
        record("schemas", vec![refs_sequence(&schema_refs)]),
        record("handler-profile", vec![optional_ref_value(input.handler_profile_ref.as_deref())]),
        record("policy", vec![refs_sequence(&sorted_unique(&input.policy_refs))]),
        record("capability", vec![refs_sequence(&sorted_unique(&input.capability_refs))]),
        record("resources", vec![refs_sequence(&resource_refs)]),
        record("effects", vec![refs_sequence(&effect_manifest_refs)]),
        record("revocation", vec![refs_sequence(&sorted_unique(&input.revocation_refs))]),
        record("seed", vec![optional_ref_value(input.seed_ref.as_deref())]),
        record("logical-time", vec![optional_u64_value(input.logical_time)]),
        record("expected", vec![refs_sequence(&sorted_unique(&input.expected_refs))]),
        record("resolutions", vec![refs_sequence(&resolution_refs)]),
        checks_value(&[
            "bounded-stanzas",
            "canonical-source-identity",
            "exact-ref-bindings",
            "profile-seed-effect-resource-bound",
            "no-ambient-identity",
            "no-ucm-compat",
        ]),
    ]);
    parse_transcript_artifact(&value)
}

pub fn parse_transcript_artifact(value: &IoValue) -> Result<TranscriptArtifact> {
    let fields = value
        .collect_simple_record("transcript-artifact-v1", None)
        .ok_or_else(|| MoltenError::invalid_harness("expected <transcript-artifact-v1 ...>"))?;
    let field_count = fields.fields_iter().count();
    if field_count != TRANSCRIPT_ARTIFACT_FIELD_COUNT && field_count != TRANSCRIPT_ARTIFACT_LEGACY_FIELD_COUNT {
        return Err(MoltenError::invalid_harness(format!(
            "transcript artifact field count {field_count} is unsupported"
        )));
    }
    require_schema(&fields[0], TRANSCRIPT_ARTIFACT_SCHEMA, "transcript artifact")?;
    let deps = value_to_iovalue(&fields[3]);
    let dep_fields = simple_record(&deps, "dependencies", 2)?;
    let stanzas = record_sequence(&fields[2], "stanzas")?
        .iter()
        .map(|stanza| parse_transcript_stanza(&value_to_iovalue(stanza)))
        .collect::<Result<Vec<_>>>()?;
    let checks_index = field_count - 1;
    let checks = parse_checks(&fields[checks_index])?;
    require_check(&checks, "no-ambient-identity", "transcript artifact")?;
    let is_legacy = field_count == TRANSCRIPT_ARTIFACT_LEGACY_FIELD_COUNT;
    Ok(TranscriptArtifact {
        transcript_ref: canonical_hash(value)?,
        source_ref: record_ref(&fields[1], "source")?,
        stanzas,
        dependency_closure_hash: required_ref(&dep_fields[0], "dependency closure hash")?,
        dependency_refs: parse_ref_sequence_value(&dep_fields[1], "dependency refs")?,
        artifact_refs: if is_legacy { Vec::new() } else { record_ref_sequence(&fields[4], "artifacts")? },
        schema_refs: if is_legacy { Vec::new() } else { record_ref_sequence(&fields[5], "schemas")? },
        handler_profile_ref: record_optional_ref(&fields[if is_legacy { 4 } else { 6 }], "handler-profile")?,
        policy_refs: record_ref_sequence(&fields[if is_legacy { 5 } else { 7 }], "policy")?,
        capability_refs: record_ref_sequence(&fields[if is_legacy { 6 } else { 8 }], "capability")?,
        resource_refs: if is_legacy { Vec::new() } else { record_ref_sequence(&fields[9], "resources")? },
        effect_manifest_refs: if is_legacy { Vec::new() } else { record_ref_sequence(&fields[10], "effects")? },
        revocation_refs: record_ref_sequence(&fields[if is_legacy { 7 } else { 11 }], "revocation")?,
        seed_ref: record_optional_ref(&fields[if is_legacy { 8 } else { 12 }], "seed")?,
        logical_time: if is_legacy { None } else { record_optional_u64(&fields[13], "logical-time")? },
        expected_refs: record_ref_sequence(&fields[if is_legacy { 9 } else { 14 }], "expected")?,
        resolution_refs: if is_legacy { Vec::new() } else { record_ref_sequence(&fields[15], "resolutions")? },
        value: value.clone(),
    })
}
