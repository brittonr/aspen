
pub fn remote_gc_clearance_request_value(input: &RemoteGcClearanceRequestInput<'_>) -> Result<IoValue> {
    validate_remote_gc_clearance_request_input(input)?;
    Ok(crate::preserves_rail::record("retention-remote-gc-clearance-request-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_REQUEST_SCHEMA),
        crate::preserves_rail::record("requester", vec![crate::preserves_rail::string(input.requester_ref)]),
        crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(input.peer_ref)]),
        object_value(input.object_ref, input.object_kind),
        crate::preserves_rail::record("class", vec![crate::preserves_rail::string(input.retention_class)]),
        crate::preserves_rail::record("action", vec![crate::preserves_rail::string(input.action)]),
        crate::preserves_rail::record("remote", vec![crate::preserves_rail::string(input.remote_ref)]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::string(input.policy_ref)]),
        crate::preserves_rail::record("authority", vec![crate::preserves_rail::string(input.authority_ref)]),
        crate::preserves_rail::record("evidence", vec![strings_sequence(input.evidence_refs)]),
        checks_value(&[("request-scope-bound", "pass"), ("peer-bound", "pass")]),
    ]))
}

pub fn parse_remote_gc_clearance_request(value: &IoValue) -> Result<RemoteGcClearanceRequest> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-request-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-request-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_REQUEST_SCHEMA,
        "retention remote clearance request schema",
    )?;
    require_check(&parse_checks(&fields[10])?, "request-scope-bound", "retention remote clearance request")?;
    let (object_ref, object_kind) = parse_object_value(&fields[3])?;
    let request = RemoteGcClearanceRequest {
        request_ref: crate::preserves_rail::canonical_hash(value)?,
        requester_ref: record_ref(&fields[1], "requester")?,
        peer_ref: record_ref(&fields[2], "peer")?,
        object_ref,
        object_kind,
        retention_class: record_string(&fields[4], "class")?,
        action: record_string(&fields[5], "action")?,
        remote_ref: record_ref(&fields[6], "remote")?,
        policy_ref: record_ref(&fields[7], "policy")?,
        authority_ref: record_ref(&fields[8], "authority")?,
        evidence_refs: record_ref_sequence(&fields[9], "evidence")?,
        value: value.clone(),
    };
    validate_remote_gc_clearance_request(&request)?;
    Ok(request)
}

pub fn store_remote_gc_clearance_request(
    root: &Path,
    input: &RemoteGcClearanceRequestInput<'_>,
) -> Result<RemoteGcClearanceRequest> {
    let root = open_capability_retention_root(root)?;
    store_remote_gc_clearance_request_with_root(&root, input)
}

pub fn store_remote_gc_clearance_request_with_root(
    root: &CapabilityRetentionRoot,
    input: &RemoteGcClearanceRequestInput<'_>,
) -> Result<RemoteGcClearanceRequest> {
    ensure_store_with_root(root)?;
    let value = remote_gc_clearance_request_value(input)?;
    let request = parse_remote_gc_clearance_request(&value)?;
    write_store_value_with_root(
        root,
        &capability_ref_path(REMOTE_CLEARANCE_REQUEST_DIR, &request.request_ref)?,
        &request.value,
    )?;
    Ok(request)
}
