
pub fn remote_execution_request_value(input: &RemoteExecutionRequestInput) -> Result<IoValue> {
    validate_non_empty(&input.execution_id, "remote execution id")?;
    validate_ref(&input.root_artifact_ref, "remote execution request root artifact ref")?;
    let closure = parse_remote_execution_closure_descriptor(&input.closure_descriptor)?;
    if closure.root_artifact_ref != input.root_artifact_ref {
        return Err(MoltenError::invalid_harness("remote execution request root does not match closure descriptor"));
    }
    validate_non_empty(&input.entrypoint_id, "remote execution entrypoint")?;
    reject_mobile_closure_config(&input.argument)?;
    validate_ref(&input.effect_manifest_ref, "remote execution effect manifest ref")?;
    if input.effect_manifest_ref != closure.effect_manifest_ref {
        return Err(MoltenError::invalid_harness(
            "remote execution request effect manifest does not match closure descriptor",
        ));
    }
    validate_non_empty(&input.handler_profile, "remote execution handler profile")?;
    if input.handler_profile != closure.handler_profile {
        return Err(MoltenError::invalid_harness(
            "remote execution request handler profile does not match closure descriptor",
        ));
    }
    validate_refs(&input.capability_refs, "remote execution capability ref")?;
    validate_refs(&input.policy_refs, "remote execution policy ref")?;
    validate_refs(&input.provenance_refs, "remote execution provenance ref")?;
    validate_refs(&input.source_gate_refs, "remote execution source gate ref")?;
    validate_refs(&input.resource_refs, "remote execution resource ref")?;
    validate_ref(&input.reply_route_ref, "remote execution reply route ref")?;
    validate_refs(&input.evidence_refs, "remote execution evidence ref")?;
    Ok(crate::preserves_rail::record("remote-execution-request-v1", vec![
        crate::preserves_rail::string(REMOTE_EXECUTION_REQUEST_SCHEMA),
        crate::preserves_rail::record("execution", vec![crate::preserves_rail::string(&input.execution_id)]),
        crate::preserves_rail::record("root", vec![crate::preserves_rail::string(&input.root_artifact_ref)]),
        crate::preserves_rail::record("closure", vec![input.closure_descriptor.clone()]),
        crate::preserves_rail::record("entrypoint", vec![crate::preserves_rail::string(&input.entrypoint_id)]),
        crate::preserves_rail::record("argument", vec![input.argument.clone()]),
        crate::preserves_rail::record("effect-manifest", vec![crate::preserves_rail::string(&input.effect_manifest_ref)]),
        crate::preserves_rail::record("handler-profile", vec![crate::preserves_rail::string(&input.handler_profile)]),
        crate::preserves_rail::record("capabilities", vec![refs_sequence(&sorted_unique(&input.capability_refs))]),
        crate::preserves_rail::record("policy", vec![refs_sequence(&sorted_unique(&input.policy_refs))]),
        crate::preserves_rail::record("provenance", vec![refs_sequence(&sorted_unique(&input.provenance_refs))]),
        crate::preserves_rail::record("source-gate", vec![refs_sequence(&sorted_unique(&input.source_gate_refs))]),
        crate::preserves_rail::record("resource", vec![refs_sequence(&sorted_unique(&input.resource_refs))]),
        crate::preserves_rail::record("reply-route", vec![crate::preserves_rail::string(&input.reply_route_ref)]),
        crate::preserves_rail::record("evidence", vec![refs_sequence(&sorted_unique(&input.evidence_refs))]),
        checks_value(&[
            "exact-artifact-ref",
            "closure-descriptor-bound",
            "canonical-argument-or-ref",
            "handler-profile-bound",
            "receiver-policy-required",
            "no-mobile-closure-authority",
        ]),
    ]))
}

pub fn parse_remote_execution_request(value: &IoValue) -> Result<RemoteExecutionRequest> {
    let fields = simple_record(value, "remote-execution-request-v1", REMOTE_EXECUTION_REQUEST_FIELDS + 1)?;
    require_schema(&fields[0], REMOTE_EXECUTION_REQUEST_SCHEMA, "remote execution request schema")?;
    let checks = parse_checks(&fields[15])?;
    require_check(&checks, "exact-artifact-ref", "remote execution request")?;
    let closure_value = record_iovalue(&fields[3], "closure")?;
    let closure_descriptor = parse_remote_execution_closure_descriptor(&closure_value)?;
    Ok(RemoteExecutionRequest {
        request_ref: crate::preserves_rail::canonical_hash(value)?,
        execution_id: record_string(&fields[1], "execution")?,
        root_artifact_ref: record_ref(&fields[2], "root")?,
        closure_descriptor,
        entrypoint_id: record_string(&fields[4], "entrypoint")?,
        argument: record_iovalue(&fields[5], "argument")?,
        effect_manifest_ref: record_ref(&fields[6], "effect-manifest")?,
        handler_profile: record_string(&fields[7], "handler-profile")?,
        capability_refs: record_ref_sequence(&fields[8], "capabilities")?,
        policy_refs: record_ref_sequence(&fields[9], "policy")?,
        provenance_refs: record_ref_sequence(&fields[10], "provenance")?,
        source_gate_refs: record_ref_sequence(&fields[11], "source-gate")?,
        resource_refs: record_ref_sequence(&fields[12], "resource")?,
        reply_route_ref: record_ref(&fields[13], "reply-route")?,
        evidence_refs: record_ref_sequence(&fields[14], "evidence")?,
        value: value.clone(),
    })
}
