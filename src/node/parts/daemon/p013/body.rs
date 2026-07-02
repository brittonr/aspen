
pub fn import_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleImportInput<'_>,
) -> Result<ControlLiveWorkflowBundleImport> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    if let Some(node) = input.expected_node {
        validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        validate_node_id(endpoint)?;
    }
    if let Some(peer) = input.expected_peer {
        validate_node_id(peer)?;
    }
    for operation in input.expected_operations {
        validate_node_id(operation)?;
    }
    if let Some(scope) = input.expected_target_scope {
        validate_node_id(scope)?;
    }
    if let Some(scope) = input.expected_resource_scope {
        validate_node_id(scope)?;
    }
    let bundle = parse_control_live_workflow_bundle(input.bundle_value)?;
    let ticket = parse_control_live_ticket(&bundle.ticket_value)?;
    let admission = parse_control_live_peer_admission(&bundle.peer_admission_value)?;
    let authority = parse_control_authority_grant(&bundle.authority_grant_value)?;
    let mut diagnostics = live_workflow_bundle_import_diagnostics(input, &ticket, &admission, &authority);
    let mut parts = ImportParts {
        imported_refs: Vec::with_capacity(bundle.receipt_values.len().saturating_add(5)),
        ticket_import_ref: None,
        authority_import_ref: None,
    };
    if diagnostics.is_empty() {
        let (imported, import_diagnostics) = import_parts(input, &bundle)?;
        parts = imported;
        diagnostics.extend(import_diagnostics);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_import_receipt_value(&LiveWorkflowBundleImportReceiptValueInput {
        decision,
        state_root: input.state_root,
        bundle: &bundle,
        ticket_import_ref: parts.ticket_import_ref.as_deref(),
        authority_import_ref: parts.authority_import_ref.as_deref(),
        imported_refs: &parts.imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlLiveWorkflowBundleImport {
        bundle_ref: bundle.bundle_ref,
        ticket_import_ref: parts.ticket_import_ref,
        authority_import_ref: parts.authority_import_ref,
        imported_refs: parts.imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

fn import_parts(
    input: &ControlLiveWorkflowBundleImportInput<'_>,
    bundle: &ControlLiveWorkflowBundle,
) -> Result<(ImportParts, Vec<String>)> {
    let mut diagnostics = Vec::new();
    let mut parts = ImportParts {
        imported_refs: Vec::with_capacity(bundle.receipt_values.len().saturating_add(5)),
        ticket_import_ref: None,
        authority_import_ref: None,
    };
    let ticket_import = import_control_live_ticket(&ControlLiveTicketImportInput {
        state_root: input.state_root,
        ticket_value: &bundle.ticket_value,
        peer_admission_value: Some(&bundle.peer_admission_value),
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        as_of_sequence: input.as_of_sequence,
    })?;
    let authority_import = import_control_authority_grant_checked(&ControlAuthorityGrantImportInput {
        state_root: input.state_root,
        grant_value: &bundle.authority_grant_value,
        expected_peer: input.expected_peer,
        expected_node: input.expected_node,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_epoch: input.as_of_epoch,
    })?;
    parts.ticket_import_ref = Some(ticket_import.receipt_ref.clone());
    parts.authority_import_ref = Some(authority_import.receipt_ref.clone());
    if ticket_import.decision != "pass" {
        diagnostics.extend(ticket_import.diagnostics.iter().cloned());
    }
    if authority_import.decision != "pass" {
        diagnostics.extend(authority_import.diagnostics.iter().cloned());
    }
    if diagnostics.is_empty() {
        parts.imported_refs.extend(ticket_import.imported_refs);
        parts.imported_refs.extend(authority_import.imported_refs);
        parts.imported_refs.push(import_artifact(input.state_root, input.bundle_value)?);
        for receipt_value in &bundle.receipt_values {
            parts.imported_refs.push(import_artifact(input.state_root, receipt_value)?);
        }
    }
    Ok((parts, diagnostics))
}

fn validate_live_workflow_bundle_verify_input(input: &ControlLiveWorkflowBundleVerifyInput<'_>) -> Result<()> {
    if let Some(node) = input.expected_node {
        validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        validate_node_id(endpoint)?;
    }
    if let Some(peer) = input.expected_peer {
        validate_node_id(peer)?;
    }
    for operation in input.expected_operations {
        validate_node_id(operation)?;
    }
    if let Some(scope) = input.expected_target_scope {
        validate_node_id(scope)?;
    }
    if let Some(scope) = input.expected_resource_scope {
        validate_node_id(scope)?;
    }
    Ok(())
}

fn validate_live_workflow_bundle_apply_input(input: &ControlLiveWorkflowBundleApplyInput<'_>) -> Result<()> {
    validate_state_root(input.state_root)?;
    validate_live_workflow_bundle_verify_input(&live_workflow_bundle_verify_input_from_apply(input))?;
    if let Some(from_peer) = input.from_peer {
        validate_node_id(from_peer)?;
    }
    if let Some(operation_ref) = input.expected_operation_ref {
        validate_ingress_ref(operation_ref, "node control live workflow bundle apply operation id")?;
    }
    validate_ingress_refs(input.peer_bootstrap_refs, "node control live workflow bundle apply peer bootstrap ref")?;
    validate_ingress_refs(input.authority_refs, "node control live workflow bundle apply authority ref")?;
    validate_ingress_refs(input.policy_refs, "node control live workflow bundle apply policy ref")?;
    validate_ingress_refs(input.resource_refs, "node control live workflow bundle apply resource ref")?;
    validate_ingress_refs(input.evidence_refs, "node control live workflow bundle apply evidence ref")?;
    if input.request_value.is_some() || input.should_send {
        validate_live_send_timeout(input.join_timeout_ms)?;
        validate_live_send_attempts(input.max_attempts)?;
    }
    Ok(())
}

fn live_workflow_bundle_expected_input_from_verify<'a>(
    input: &'a ControlLiveWorkflowBundleVerifyInput<'a>,
) -> LiveWorkflowBundleExpectedInput<'a> {
    LiveWorkflowBundleExpectedInput {
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_verify_input_from_gate<'a>(
    input: &'a ControlLiveWorkflowBundleGateInput<'a>,
) -> ControlLiveWorkflowBundleVerifyInput<'a> {
    ControlLiveWorkflowBundleVerifyInput {
        bundle_value: input.bundle_value,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_verify_input_from_apply<'a>(
    input: &'a ControlLiveWorkflowBundleApplyInput<'a>,
) -> ControlLiveWorkflowBundleVerifyInput<'a> {
    ControlLiveWorkflowBundleVerifyInput {
        bundle_value: input.bundle_value,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_import_input_from_apply<'a>(
    input: &'a ControlLiveWorkflowBundleApplyInput<'a>,
) -> ControlLiveWorkflowBundleImportInput<'a> {
    ControlLiveWorkflowBundleImportInput {
        state_root: input.state_root,
        bundle_value: input.bundle_value,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_expected_input_from_import<'a>(
    input: &'a ControlLiveWorkflowBundleImportInput<'a>,
) -> LiveWorkflowBundleExpectedInput<'a> {
    LiveWorkflowBundleExpectedInput {
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        expected_operations: input.expected_operations,
        expected_target_scope: input.expected_target_scope,
        expected_resource_scope: input.expected_resource_scope,
        as_of_sequence: input.as_of_sequence,
        as_of_epoch: input.as_of_epoch,
    }
}

fn live_workflow_bundle_import_diagnostics(
    input: &ControlLiveWorkflowBundleImportInput<'_>,
    ticket: &ControlLiveTicket,
    admission: &ControlLivePeerAdmission,
    authority: &ControlAuthorityGrant,
) -> Vec<String> {
    live_workflow_bundle_expected_diagnostics(
        &live_workflow_bundle_expected_input_from_import(input),
        ticket,
        admission,
        authority,
    )
}

fn live_workflow_bundle_expected_diagnostics(
    input: &LiveWorkflowBundleExpectedInput<'_>,
    ticket: &ControlLiveTicket,
    admission: &ControlLivePeerAdmission,
    authority: &ControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = live_workflow_bundle_binding_diagnostics(ticket, admission, authority);
    diagnostics.extend(live_ticket_expected_diagnostics(input, ticket, admission));
    diagnostics.extend(authority_grant_expected_diagnostics(input, authority));
    diagnostics
}

fn live_ticket_expected_diagnostics(
    input: &LiveWorkflowBundleExpectedInput<'_>,
    ticket: &ControlLiveTicket,
    admission: &ControlLivePeerAdmission,
) -> Vec<String> {
    live_ticket_import_diagnostics(
        &ControlLiveTicketImportInput {
            state_root: Path::new("."),
            ticket_value: &ticket.value,
            peer_admission_value: Some(&admission.value),
            expected_node: input.expected_node,
            expected_topic: input.expected_topic,
            expected_endpoint: input.expected_endpoint,
            expected_peer: input.expected_peer,
            as_of_sequence: input.as_of_sequence,
        },
        ticket,
        Some(admission),
    )
}
