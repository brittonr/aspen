
pub fn artifact_closure_key_input(input: &ArtifactClosureKeyInput<'_>) -> Result<KeyInput> {
    validate_refs(input.root_refs, "artifact closure root ref")?;
    validate_ref(input.closure_hash, "artifact closure hash")?;
    validate_refs(input.dependency_refs, "artifact closure dependency ref")?;
    Ok(KeyInput {
        operation: "artifact-closure".to_string(),
        version: "v1".to_string(),
        input_ref: canonical_hash(&record("eval-cache-artifact-closure-input", vec![refs_sequence(&sorted_unique(
            input.root_refs,
        ))]))?,
        dependency_closure_hash: input.closure_hash.to_string(),
        dependency_refs: input.dependency_refs.to_vec(),
        artifact_refs: input.root_refs.to_vec(),
        handler_profile_ref: None,
        policy_refs: input.policy_refs.to_vec(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: input.tool_ref.to_string(),
        tool_version: input.tool_version.to_string(),
        assumption_refs: Vec::new(),
        ..KeyInput::default()
    })
}
