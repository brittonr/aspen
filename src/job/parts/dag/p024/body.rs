
pub fn remote_execution_closure_descriptor_value(input: &RemoteExecutionClosureDescriptorInput) -> Result<IoValue> {
    validate_ref(&input.root_artifact_ref, "remote execution root artifact ref")?;
    validate_refs(&input.dependency_refs, "remote execution dependency ref")?;
    if !input.dependency_refs.iter().any(|reference| reference == &input.root_artifact_ref) {
        return Err(MoltenError::invalid_harness(
            "remote execution closure descriptor must include the root artifact ref",
        ));
    }
    if let Some(closure_digest_ref) = input.closure_digest_ref.as_deref() {
        validate_ref(closure_digest_ref, "remote execution closure digest ref")?;
    }
    validate_non_empty(&input.artifact_kind, "remote execution artifact kind")?;
    validate_ref(&input.size_bound_ref, "remote execution size bound ref")?;
    validate_ref(&input.effect_manifest_ref, "remote execution effect manifest ref")?;
    validate_non_empty(&input.handler_profile, "remote execution handler profile")?;
    validate_refs(&input.policy_refs, "remote execution policy ref")?;
    validate_refs(&input.evidence_refs, "remote execution evidence ref")?;
    validate_ref(&input.replay_nonce_ref, "remote execution replay nonce ref")?;
    Ok(crate::preserves_rail::record("remote-execution-closure-descriptor-v1", vec![
        crate::preserves_rail::string(REMOTE_EXECUTION_CLOSURE_DESCRIPTOR_SCHEMA),
        crate::preserves_rail::record("root", vec![crate::preserves_rail::string(&input.root_artifact_ref)]),
        crate::preserves_rail::record("dependencies", vec![refs_sequence(&sorted_unique(&input.dependency_refs))]),
        crate::preserves_rail::record("closure-digest", vec![optional_ref_value(input.closure_digest_ref.as_deref())]),
        crate::preserves_rail::record("artifact-kind", vec![crate::preserves_rail::string(&input.artifact_kind)]),
        crate::preserves_rail::record("size-bound", vec![crate::preserves_rail::string(&input.size_bound_ref)]),
        crate::preserves_rail::record("effect-manifest", vec![crate::preserves_rail::string(&input.effect_manifest_ref)]),
        crate::preserves_rail::record("handler-profile", vec![crate::preserves_rail::string(&input.handler_profile)]),
        crate::preserves_rail::record("policy", vec![refs_sequence(&sorted_unique(&input.policy_refs))]),
        crate::preserves_rail::record("evidence", vec![
            refs_sequence(&sorted_unique(&refs_with_required(&input.evidence_refs, &input.replay_nonce_ref)?)),
        ]),
        checks_value(&[
            "exact-root-ref",
            "receiver-computes-missing-set",
            "effect-handler-profile-bound",
            "no-mobile-closure-authority",
        ]),
    ]))
}

pub fn parse_remote_execution_closure_descriptor(value: &IoValue) -> Result<RemoteExecutionClosureDescriptor> {
    let fields = simple_record(
        value,
        "remote-execution-closure-descriptor-v1",
        REMOTE_EXECUTION_CLOSURE_DESCRIPTOR_FIELDS + 1,
    )?;
    require_schema(
        &fields[0],
        REMOTE_EXECUTION_CLOSURE_DESCRIPTOR_SCHEMA,
        "remote execution closure descriptor schema",
    )?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "exact-root-ref", "remote execution closure descriptor")?;
    let root_artifact_ref = record_ref(&fields[1], "root")?;
    let dependency_refs = record_ref_sequence(&fields[2], "dependencies")?;
    Ok(RemoteExecutionClosureDescriptor {
        descriptor_ref: crate::preserves_rail::canonical_hash(value)?,
        root_artifact_ref,
        dependency_refs,
        closure_digest_ref: record_optional_ref(&fields[3], "closure-digest")?,
        artifact_kind: record_string(&fields[4], "artifact-kind")?,
        size_bound_ref: record_ref(&fields[5], "size-bound")?,
        effect_manifest_ref: record_ref(&fields[6], "effect-manifest")?,
        handler_profile: record_string(&fields[7], "handler-profile")?,
        policy_refs: record_ref_sequence(&fields[8], "policy")?,
        evidence_refs: record_ref_sequence(&fields[9], "evidence")?,
        replay_nonce_ref: replay_nonce_from_evidence(&fields[9])?,
        value: value.clone(),
    })
}

fn refs_with_required(refs: &[String], required_ref: &str) -> Result<Vec<String>> {
    validate_refs(refs, "remote execution evidence ref")?;
    validate_ref(required_ref, "remote execution required evidence ref")?;
    let mut combined = refs.to_vec();
    push_bounded(
        &mut combined,
        required_ref.to_string(),
        MAX_JOB_REFS,
        "remote execution evidence refs",
    )?;
    Ok(combined)
}

fn replay_nonce_from_evidence(value: &Value<IoValue>) -> Result<String> {
    record_ref_sequence(value, "evidence")?
        .last()
        .cloned()
        .ok_or_else(|| MoltenError::invalid_harness("remote execution closure descriptor missing replay nonce"))
}
