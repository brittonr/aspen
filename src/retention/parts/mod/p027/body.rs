
fn validate_remote_gc_clearance_live_workflow_value_input(
    input: &RemoteGcClearanceLiveWorkflowValueInput<'_>,
) -> Result<()> {
    require_ref(input.request_control_ref, "retention live request control ref")?;
    require_ref(input.request_publish_ref, "retention live request publish ref")?;
    require_ref(input.request_receive_ref, "retention live request receive ref")?;
    require_ref(input.request_ingress_ref, "retention live request ingress ref")?;
    require_ref(input.response_control_ref, "retention live response control ref")?;
    require_ref(input.response_publish_ref, "retention live response publish ref")?;
    require_ref(input.response_receive_ref, "retention live response receive ref")?;
    require_ref(input.response_ingress_ref, "retention live response ingress ref")?;
    ensure_count_at_most(
        input.transport_diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live transport diagnostics",
    )
}

fn remote_clearance_response_diagnostics<Root: ?Sized>(
    input: &RemoteGcClearanceResponseInput<'_, Root>,
) -> Result<Vec<String>> {
    validate_refs(input.evidence_refs, "retention remote clearance response evidence ref")?;
    validate_refs(input.retained_refs, "retention remote clearance response retained ref")?;
    validate_refs(input.revoked_refs, "retention remote clearance response revoked ref")?;
    ensure_count_at_most(
        input.diagnostics.len(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention remote clearance response diagnostics",
    )?;
    let mut diagnostics = input.diagnostics.to_vec();
    if !input.is_current {
        push_bounded(
            &mut diagnostics,
            "remote-clearance-stale".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention remote clearance response diagnostics",
        )?;
    }
    if !input.revoked_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "remote-clearance-revoked".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention remote clearance response diagnostics",
        )?;
    }
    if !input.retained_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "remote-clearance-retained".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention remote clearance response diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn validate_remote_gc_clearance_workflow_scope(
    request: &RemoteGcClearanceRequest,
    clearance: &RemoteGcClearance,
) -> Result<()> {
    if clearance.requester_ref != request.requester_ref
        || clearance.peer_ref != request.peer_ref
        || clearance.object_ref != request.object_ref
        || clearance.object_kind != request.object_kind
        || clearance.retention_class != request.retention_class
        || clearance.action != request.action
        || clearance.remote_ref != request.remote_ref
        || clearance.policy_ref != request.policy_ref
        || clearance.authority_ref != request.authority_ref
    {
        return Err(MoltenError::invalid_harness("remote clearance workflow scope mismatch"));
    }
    Ok(())
}

fn parse_embedded_remote_clearance_request(value: &Value<IoValue>) -> Result<RemoteGcClearanceRequest> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record("request", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected embedded remote clearance request"))?;
    let request_ref = required_string(&fields[0], "remote clearance request ref")?;
    require_ref(&request_ref, "remote clearance request ref")?;
    let request_value = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let request = parse_remote_gc_clearance_request(&request_value)?;
    if request.request_ref != request_ref {
        return Err(MoltenError::invalid_harness("embedded remote clearance request ref mismatch"));
    }
    Ok(request)
}

fn parse_embedded_remote_clearance(value: &Value<IoValue>) -> Result<RemoteGcClearance> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record("clearance", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected embedded remote clearance"))?;
    let clearance_ref = required_string(&fields[0], "remote clearance ref")?;
    require_ref(&clearance_ref, "remote clearance ref")?;
    let clearance_value = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let clearance = parse_remote_gc_clearance(&clearance_value)?;
    if clearance.clearance_ref != clearance_ref {
        return Err(MoltenError::invalid_harness("embedded remote clearance ref mismatch"));
    }
    Ok(clearance)
}

fn parse_embedded_remote_clearance_import(value: &Value<IoValue>) -> Result<RemoteGcClearanceImport> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record("import", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected embedded remote clearance import"))?;
    let import_ref = required_string(&fields[0], "remote clearance import ref")?;
    require_ref(&import_ref, "remote clearance import ref")?;
    let import_value = crate::preserves_rail::value_to_iovalue(&fields[1]);
    let import = parse_remote_gc_clearance_import(&import_value)?;
    if import.import_ref != import_ref {
        return Err(MoltenError::invalid_harness("embedded remote clearance import ref mismatch"));
    }
    Ok(import)
}

fn parse_embedded_value(value: &Value<IoValue>, label: &str) -> Result<(String, IoValue)> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(2))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected embedded {label}")))?;
    let value_ref = required_string(&fields[0], label)?;
    require_ref(&value_ref, label)?;
    let embedded = crate::preserves_rail::value_to_iovalue(&fields[1]);
    if crate::preserves_rail::canonical_hash(&embedded)? != value_ref {
        return Err(MoltenError::invalid_harness(format!("embedded {label} ref mismatch")));
    }
    Ok((value_ref, embedded))
}

fn push_import_diagnostic<S>(diagnostics: &mut S, diagnostic: &str) -> Result<()>
where S: VecSink<String> {
    push_bounded(
        diagnostics,
        diagnostic.to_string(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention remote clearance import diagnostics",
    )
}

fn push_remote_clearance_import_diagnostics<Root: ?Sized, S>(
    diagnostics: &mut S,
    request: &RemoteGcClearanceRequest,
    response: &RemoteGcClearanceResponse,
    input: &RemoteGcClearanceImportInput<'_, Root>,
) -> Result<()>
where
    S: VecSink<String>,
{
    if response.request_ref != request.request_ref {
        push_import_diagnostic(diagnostics, "remote-clearance-wrong-request")?;
    }
    if response.decision != "pass" {
        push_import_diagnostic(diagnostics, "remote-clearance-response-not-pass")?;
    }
    let clearance = &response.clearance;
    if clearance.decision != "pass" {
        push_import_diagnostic(diagnostics, "remote-clearance-not-pass")?;
    }
    if !clearance.is_current {
        push_import_diagnostic(diagnostics, "remote-clearance-stale")?;
    }
    if !clearance.revoked_refs.is_empty() {
        push_import_diagnostic(diagnostics, "remote-clearance-revoked")?;
    }
    if !clearance.retained_refs.is_empty() {
        push_import_diagnostic(diagnostics, "remote-clearance-retained")?;
    }
    if clearance.peer_ref != request.peer_ref {
        push_import_diagnostic(diagnostics, "remote-clearance-wrong-peer")?;
    }
    if clearance.remote_ref != request.remote_ref {
        push_import_diagnostic(diagnostics, "remote-clearance-wrong-remote")?;
    }
    if let Some(expected_peer_ref) = input.expected_peer_ref
        && expected_peer_ref != request.peer_ref
    {
        push_import_diagnostic(diagnostics, "remote-clearance-expected-peer-mismatch")?;
    }
    if let Some(expected_remote_ref) = input.expected_remote_ref
        && expected_remote_ref != request.remote_ref
    {
        push_import_diagnostic(diagnostics, "remote-clearance-expected-remote-mismatch")?;
    }
    for diagnostic in &response.diagnostics {
        push_import_diagnostic(diagnostics, diagnostic)?;
    }
    Ok(())
}

fn ensure_store_with_root(root: &CapabilityRetentionRoot) -> Result<()> {
    for directory in [
        PIN_DIR,
        ADMISSION_DIR,
        REMOTE_CLEARANCE_DIR,
        REMOTE_CLEARANCE_REQUEST_DIR,
        REMOTE_CLEARANCE_RESPONSE_DIR,
        REMOTE_CLEARANCE_IMPORT_DIR,
        REMOTE_CLEARANCE_LIVE_WORKFLOW_DIR,
        GC_PLAN_DIR,
        GC_APPLY_DIR,
        GC_EXECUTE_DIR,
        GC_AUDIT_DIR,
        RECEIPT_DIR,
        TOMBSTONE_DIR,
    ] {
        root.root().create_dir_all(&capability_store_path(directory)?)?;
    }
    Ok(())
}

fn pins_for_object_with_root(root: &CapabilityRetentionRoot, object_ref: &str) -> Result<Vec<Pin>> {
    let mut pins = Vec::new();
    let directory = capability_store_path(PIN_DIR)?;
    if !root.root().try_exists(&directory)? {
        return Ok(pins);
    }
    for entry in root.root().list_entries(&directory)? {
        if entry.kind != crate::local_store::LocalStoreEntryKind::File {
            continue;
        }
        let value = read_store_value_with_root(root, &entry.path)?;
        let pin = parse_pin(&value)?;
        if pin.object_ref == object_ref {
            push_bounded(&mut pins, pin, MAX_RETENTION_REFS, "retention pins")?;
        }
    }
    pins.sort_by(|left, right| left.pin_ref.cmp(&right.pin_ref));
    Ok(pins)
}

fn tombstone_refs_for_object_with_root(root: &CapabilityRetentionRoot, object_ref: &str) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    let directory = capability_store_path(TOMBSTONE_DIR)?;
    if !root.root().try_exists(&directory)? {
        return Ok(refs);
    }
    for entry in root.root().list_entries(&directory)? {
        if entry.kind != crate::local_store::LocalStoreEntryKind::File {
            continue;
        }
        let value = read_store_value_with_root(root, &entry.path)?;
        let tombstone = parse_tombstone(&value)?;
        if tombstone.object_ref == object_ref {
            push_bounded(&mut refs, tombstone.tombstone_ref, MAX_RETENTION_REFS, "retention tombstone refs")?;
        }
    }
    refs.sort();
    Ok(refs)
}
