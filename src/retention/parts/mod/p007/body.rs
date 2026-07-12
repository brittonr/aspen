
pub fn remote_gc_clearance_live_workflow_value(input: &RemoteGcClearanceLiveWorkflowValueInput<'_>) -> Result<IoValue> {
    validate_remote_gc_clearance_live_workflow_value_input(input)?;
    let parts = flow_parts(input)?;
    let refs = flow_refs(input);
    let decision = if parts.diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(crate::preserves_rail::record("retention-remote-gc-clearance-live-workflow-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_LIVE_WORKFLOW_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("request", vec![
            crate::preserves_rail::string(&parts.request.request_ref),
            input.request_value.clone(),
        ]),
        crate::preserves_rail::record("response", vec![
            crate::preserves_rail::string(&parts.response_ref),
            input.response_value.clone(),
        ]),
        crate::preserves_rail::record("import", vec![
            crate::preserves_rail::string(&parts.import.import_ref),
            input.import_value.clone(),
        ]),
        crate::preserves_rail::record("request-live", vec![strings_sequence(&refs.request)]),
        crate::preserves_rail::record("response-live", vec![strings_sequence(&refs.response)]),
        crate::preserves_rail::record("scope", vec![
            crate::preserves_rail::record("requester", vec![crate::preserves_rail::string(
                &parts.request.requester_ref,
            )]),
            crate::preserves_rail::record("peer", vec![crate::preserves_rail::string(&parts.request.peer_ref)]),
            crate::preserves_rail::record("remote", vec![crate::preserves_rail::string(&parts.request.remote_ref)]),
            object_value(&parts.request.object_ref, &parts.request.object_kind),
            crate::preserves_rail::record("class", vec![crate::preserves_rail::string(&parts.request.retention_class)]),
            crate::preserves_rail::record("action", vec![crate::preserves_rail::string(&parts.request.action)]),
        ]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            parts.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        checks_value(&[
            (
                "request-response-bound",
                pass_or_deny(
                    parts.response.as_ref().is_some_and(|value| value.request_ref == parts.request.request_ref),
                ),
            ),
            ("live-transport-bound", pass_or_deny(input.transport_diagnostics.is_empty())),
            ("import-gate", pass_or_deny(parts.import.decision == "pass")),
            ("transport-is-not-authority", "pass"),
            ("live-receipt-is-not-clearance", "pass"),
            ("authority-policy-still-required", "pass"),
            ("remote-gc-still-required", "pass"),
        ]),
    ]))
}

struct FlowParts {
    request: RemoteGcClearanceRequest,
    response_ref: String,
    response: Option<RemoteGcClearanceResponse>,
    import: RemoteGcClearanceImport,
    diagnostics: Vec<String>,
}

struct FlowRefs {
    request: Vec<String>,
    response: Vec<String>,
}

struct FlowDiagnosticsInput<'a> {
    request: &'a RemoteGcClearanceRequest,
    response: Option<&'a RemoteGcClearanceResponse>,
    response_ref: &'a str,
    import: &'a RemoteGcClearanceImport,
    parse_diagnostic: Option<String>,
    transport_diagnostics: &'a [String],
}

fn flow_parts(input: &RemoteGcClearanceLiveWorkflowValueInput<'_>) -> Result<FlowParts> {
    let request = parse_remote_gc_clearance_request(input.request_value)?;
    let response_ref = crate::preserves_rail::canonical_hash(input.response_value)?;
    let (response, parse_diagnostic) = match parse_remote_gc_clearance_response(input.response_value) {
        Ok(response) => (Some(response), None),
        Err(error) => (None, Some(format!("remote-clearance-live-tampered-response:{error}"))),
    };
    let import = parse_remote_gc_clearance_import(input.import_value)?;
    let diagnostics = flow_diagnostics(FlowDiagnosticsInput {
        request: &request,
        response: response.as_ref(),
        response_ref: &response_ref,
        import: &import,
        parse_diagnostic,
        transport_diagnostics: input.transport_diagnostics,
    })?;
    Ok(FlowParts {
        request,
        response_ref,
        response,
        import,
        diagnostics,
    })
}

fn flow_diagnostics(input: FlowDiagnosticsInput<'_>) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    extend_bounded(
        &mut diagnostics,
        input.transport_diagnostics.to_vec(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    if let Some(diagnostic) = input.parse_diagnostic {
        push_bounded(&mut diagnostics, diagnostic, MAX_RETENTION_DIAGNOSTICS, "retention live workflow diagnostics")?;
    }
    extend_bounded(
        &mut diagnostics,
        response_notes(input.request, input.response)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        import_notes(input.request, input.response_ref, input.import)?,
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    extend_bounded(
        &mut diagnostics,
        input.import.diagnostics.clone(),
        MAX_RETENTION_DIAGNOSTICS,
        "retention live workflow diagnostics",
    )?;
    Ok(diagnostics)
}

fn response_notes(
    request: &RemoteGcClearanceRequest,
    response: Option<&RemoteGcClearanceResponse>,
) -> Result<Vec<String>> {
    let mut notes = Vec::new();
    if let Some(response) = response {
        if response.request_ref != request.request_ref {
            push_bounded(
                &mut notes,
                "remote-clearance-live-wrong-request".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention live workflow diagnostics",
            )?;
        }
        if response.decision != "pass" {
            push_bounded(
                &mut notes,
                "remote-clearance-live-response-not-pass".to_string(),
                MAX_RETENTION_DIAGNOSTICS,
                "retention live workflow diagnostics",
            )?;
        }
    }
    Ok(notes)
}

fn import_notes(
    request: &RemoteGcClearanceRequest,
    response_ref: &str,
    import: &RemoteGcClearanceImport,
) -> Result<Vec<String>> {
    let mut notes = Vec::new();
    if import.request_ref != request.request_ref {
        push_bounded(
            &mut notes,
            "remote-clearance-live-import-wrong-request".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live workflow diagnostics",
        )?;
    }
    if import.response_ref != response_ref {
        push_bounded(
            &mut notes,
            "remote-clearance-live-import-wrong-response".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live workflow diagnostics",
        )?;
    }
    if import.peer_ref != request.peer_ref {
        push_bounded(
            &mut notes,
            "remote-clearance-live-import-wrong-peer".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live workflow diagnostics",
        )?;
    }
    if import.remote_ref != request.remote_ref {
        push_bounded(
            &mut notes,
            "remote-clearance-live-import-wrong-remote".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live workflow diagnostics",
        )?;
    }
    if import.decision != "pass" {
        push_bounded(
            &mut notes,
            "remote-clearance-live-import-deny".to_string(),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live workflow diagnostics",
        )?;
    }
    Ok(notes)
}

fn flow_refs(input: &RemoteGcClearanceLiveWorkflowValueInput<'_>) -> FlowRefs {
    FlowRefs {
        request: vec![
            input.request_control_ref.to_string(),
            input.request_publish_ref.to_string(),
            input.request_receive_ref.to_string(),
            input.request_ingress_ref.to_string(),
        ],
        response: vec![
            input.response_control_ref.to_string(),
            input.response_publish_ref.to_string(),
            input.response_receive_ref.to_string(),
            input.response_ingress_ref.to_string(),
        ],
    }
}

pub fn parse_remote_gc_clearance_live_workflow(value: &IoValue) -> Result<RemoteGcClearanceLiveWorkflow> {
    let fields = value
        .collect_simple_record("retention-remote-gc-clearance-live-workflow-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <retention-remote-gc-clearance-live-workflow-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::RETENTION_REMOTE_GC_CLEARANCE_LIVE_WORKFLOW_SCHEMA,
        "retention remote clearance live workflow schema",
    )?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "transport-is-not-authority", "retention remote clearance live workflow")?;
    require_check(&checks, "live-receipt-is-not-clearance", "retention remote clearance live workflow")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let request = parse_embedded_remote_clearance_request(&fields[2])?;
    let (response_ref, response_value) = parse_embedded_value(&fields[3], "response")?;
    let import = parse_embedded_remote_clearance_import(&fields[4])?;
    let request_live_refs = record_ref_sequence(&fields[5], "request-live")?;
    let response_live_refs = record_ref_sequence(&fields[6], "response-live")?;
    let diagnostics = record_string_sequence(&fields[8], "diagnostics")?;
    if let Ok(response) = parse_remote_gc_clearance_response(&response_value)
        && decision == "pass"
        && response.request_ref != request.request_ref
    {
        return Err(MoltenError::invalid_harness(
            "retention remote clearance live workflow pass response request mismatch",
        ));
    }
    if decision == "pass" && (import.request_ref != request.request_ref || import.response_ref != response_ref) {
        return Err(MoltenError::invalid_harness(
            "retention remote clearance live workflow pass import binding mismatch",
        ));
    }
    Ok(RemoteGcClearanceLiveWorkflow {
        workflow_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        request_ref: request.request_ref,
        response_ref,
        import_ref: import.import_ref,
        clearance_ref: import.clearance_ref,
        peer_ref: request.peer_ref,
        remote_ref: request.remote_ref,
        request_live_refs,
        response_live_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn store_remote_gc_clearance_live_workflow(root: &Path, value: &IoValue) -> Result<RemoteGcClearanceLiveWorkflow> {
    let root = open_capability_retention_root(root)?;
    store_remote_gc_clearance_live_workflow_with_root(&root, value)
}

pub fn store_remote_gc_clearance_live_workflow_with_root(
    root: &CapabilityRetentionRoot,
    value: &IoValue,
) -> Result<RemoteGcClearanceLiveWorkflow> {
    ensure_store_with_root(root)?;
    let workflow = parse_remote_gc_clearance_live_workflow(value)?;
    write_store_value_with_root(
        root,
        &capability_ref_path(REMOTE_CLEARANCE_LIVE_WORKFLOW_DIR, &workflow.workflow_ref)?,
        &workflow.value,
    )?;
    Ok(workflow)
}

struct AdmissionScope<'a> {
    requester_ref: Option<&'a str>,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    action: &'a str,
}

struct AdmissionRefsInput<'a, Root: ?Sized = Path> {
    root: &'a Root,
    refs: &'a [String],
    expected_kind: &'a str,
    scope: &'a AdmissionScope<'a>,
    required_remote_refs: &'a [String],
}
