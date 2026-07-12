
pub fn import_control_live_ticket(input: &ControlLiveTicketImportInput<'_>) -> Result<ControlLiveTicketImport> {
    validate_state_root(input.state_root)?;
    let state_root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    import_control_live_ticket_with_root(&state_root, input)
}

fn import_control_live_ticket_with_root(
    state_root: &crate::node_state::NodeStateRoot,
    input: &ControlLiveTicketImportInput<'_>,
) -> Result<ControlLiveTicketImport> {
    ensure_state_layout(state_root)?;
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
    let ticket = parse_control_live_ticket(input.ticket_value)?;
    let admission = input.peer_admission_value.map(parse_control_live_peer_admission).transpose()?;
    let mut diagnostics = live_ticket_import_diagnostics(input, &ticket, admission.as_ref());
    if input.peer_admission_value.is_some() && admission.is_none() {
        diagnostics.push("node control live ticket import admission was not parsed".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut imported_refs = Vec::with_capacity(2);
    if diagnostics.is_empty() {
        imported_refs.push(import_artifact(state_root, input.ticket_value)?);
        if let Some(value) = input.peer_admission_value {
            imported_refs.push(import_artifact(state_root, value)?);
        }
    }
    let peer_id = admission.as_ref().map(|value| value.peer_id.as_str()).or(input.expected_peer);
    let receipt_value = live_ticket_import_receipt_value(&LiveTicketImportReceiptValueInput {
        decision,
        ticket: &ticket,
        peer_admission_ref: admission.as_ref().map(|value| value.admission_ref.as_str()),
        peer_id,
        as_of_sequence: input.as_of_sequence,
        imported_refs: &imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    import_artifact(state_root, &receipt_value)?;
    Ok(ControlLiveTicketImport {
        decision: decision.to_string(),
        ticket_ref: ticket.ticket_ref,
        peer_admission_ref: admission.map(|value| value.admission_ref),
        imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
    })
}

pub fn import_control_authority_grant_checked(
    input: &ControlAuthorityGrantImportInput<'_>,
) -> Result<ControlAuthorityGrantImport> {
    validate_state_root(input.state_root)?;
    let state_root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    import_control_authority_grant_checked_with_root(&state_root, input)
}

fn import_control_authority_grant_checked_with_root(
    state_root: &crate::node_state::NodeStateRoot,
    input: &ControlAuthorityGrantImportInput<'_>,
) -> Result<ControlAuthorityGrantImport> {
    ensure_state_layout(state_root)?;
    if let Some(peer) = input.expected_peer {
        validate_node_id(peer)?;
    }
    if let Some(node) = input.expected_node {
        validate_node_id(node)?;
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
    let grant = parse_control_authority_grant(input.grant_value)?;
    let diagnostics = authority_grant_import_diagnostics(input, &grant);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let mut imported_refs = Vec::with_capacity(1);
    if diagnostics.is_empty() {
        imported_refs.push(import_artifact(state_root, input.grant_value)?);
    }
    let receipt_value = authority_grant_import_receipt_value(&AuthorityGrantImportReceiptValueInput {
        decision,
        grant: &grant,
        as_of_epoch: input.as_of_epoch,
        imported_refs: &imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    import_artifact(state_root, &receipt_value)?;
    Ok(ControlAuthorityGrantImport {
        decision: decision.to_string(),
        grant_ref: grant.grant_ref,
        imported_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
    })
}

pub fn export_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleExportInput<'_>,
) -> Result<ControlLiveWorkflowBundleExport> {
    let ticket = parse_control_live_ticket(input.receiver_ticket_value)?;
    let admission = parse_control_live_peer_admission(input.peer_admission_value)?;
    let authority = parse_control_authority_grant(input.authority_grant_value)?;
    let receipt_refs = live_workflow_bundle_receipt_refs(input.receipt_values)?;
    let mut diagnostics = live_workflow_bundle_binding_diagnostics(&ticket, &admission, &authority);
    diagnostics.extend(live_workflow_bundle_receipt_diagnostics(input.receipt_values));
    let bundle_value = live_workflow_bundle_value(&LiveWorkflowBundleValueInput {
        ticket: &ticket,
        admission: &admission,
        authority: &authority,
        ticket_value: input.receiver_ticket_value,
        admission_value: input.peer_admission_value,
        authority_value: input.authority_grant_value,
        receipt_values: input.receipt_values,
        diagnostics: &diagnostics,
    })?;
    let bundle_ref = crate::preserves_rail::canonical_hash(&bundle_value)?;
    let bundle = ControlLiveWorkflowBundle {
        bundle_ref,
        bundle_value,
        ticket_ref: ticket.ticket_ref,
        peer_admission_ref: admission.admission_ref,
        authority_grant_ref: authority.grant_ref,
        receipt_refs,
        ticket_value: input.receiver_ticket_value.clone(),
        peer_admission_value: input.peer_admission_value.clone(),
        authority_grant_value: input.authority_grant_value.clone(),
        receipt_values: input.receipt_values.iter().map(|value| (**value).clone()).collect(),
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_export_receipt_value(&LiveWorkflowBundleExportReceiptValueInput {
        decision,
        bundle: &bundle,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(ControlLiveWorkflowBundleExport {
        bundle,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

pub fn verify_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleVerifyInput<'_>,
) -> Result<ControlLiveWorkflowBundleVerify> {
    validate_live_workflow_bundle_verify_input(input)?;
    let bundle_ref = crate::preserves_rail::canonical_hash(input.bundle_value)?;
    let expected = live_workflow_bundle_expected_input_from_verify(input);
    let parsed = parse_control_live_workflow_bundle(input.bundle_value);
    let (ticket_ref, peer_admission_ref, authority_grant_ref, receipt_refs, diagnostics) = match parsed {
        Ok(bundle) => {
            let ticket = parse_control_live_ticket(&bundle.ticket_value)?;
            let admission = parse_control_live_peer_admission(&bundle.peer_admission_value)?;
            let authority = parse_control_authority_grant(&bundle.authority_grant_value)?;
            let receipt_value_refs = bundle.receipt_values.iter().collect::<Vec<_>>();
            let mut diagnostics = live_workflow_bundle_expected_diagnostics(&expected, &ticket, &admission, &authority);
            diagnostics.extend(live_workflow_bundle_receipt_diagnostics(&receipt_value_refs));
            (
                Some(bundle.ticket_ref),
                Some(bundle.peer_admission_ref),
                Some(bundle.authority_grant_ref),
                bundle.receipt_refs,
                diagnostics,
            )
        }
        Err(error) => (None, None, None, Vec::new(), vec![format!(
            "node control live workflow bundle parse failed: {error}"
        )]),
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_verify_receipt_value(&LiveWorkflowBundleVerifyReceiptValueInput {
        decision,
        bundle_ref: &bundle_ref,
        ticket_ref: ticket_ref.as_deref(),
        peer_admission_ref: peer_admission_ref.as_deref(),
        authority_grant_ref: authority_grant_ref.as_deref(),
        receipt_refs: &receipt_refs,
        expected: &expected,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(ControlLiveWorkflowBundleVerify {
        bundle_ref,
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

pub fn gate_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleGateInput<'_>,
) -> Result<ControlLiveWorkflowBundleGate> {
    let verify_input = live_workflow_bundle_verify_input_from_gate(input);
    let verified = verify_control_live_workflow_bundle(&verify_input)?;
    let expected = live_workflow_bundle_expected_input_from_verify(&verify_input);
    let mut diagnostics = verified.diagnostics.clone();
    let verify_receipt_ref = match input.verify_receipt_value {
        Some(value) => match parse_control_live_workflow_bundle_verify_receipt(value) {
            Ok(receipt) => {
                if receipt.receipt_ref != verified.receipt_ref {
                    diagnostics.push(format!(
                        "node control live workflow bundle gate verify receipt {} does not match recomputed {}",
                        receipt.receipt_ref, verified.receipt_ref
                    ));
                }
                Some(receipt.receipt_ref)
            }
            Err(error) => {
                let receipt_ref = crate::preserves_rail::canonical_hash(value)?;
                diagnostics
                    .push(format!("node control live workflow bundle gate verify receipt parse failed: {error}"));
                Some(receipt_ref)
            }
        },
        None => {
            if input.require_verify_receipt {
                diagnostics
                    .push("node control live workflow bundle gate requires a current verify receipt".to_string());
            }
            None
        }
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_gate_receipt_value(&LiveWorkflowBundleGateReceiptValueInput {
        decision,
        bundle_ref: &verified.bundle_ref,
        verify_receipt_ref: verify_receipt_ref.as_deref(),
        recomputed_verify_receipt_ref: &verified.receipt_ref,
        ticket_ref: verified.ticket_ref.as_deref(),
        peer_admission_ref: verified.peer_admission_ref.as_deref(),
        authority_grant_ref: verified.authority_grant_ref.as_deref(),
        receipt_refs: &verified.receipt_refs,
        expected: &expected,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(ControlLiveWorkflowBundleGate {
        bundle_ref: verified.bundle_ref,
        verify_receipt_ref,
        recomputed_verify_receipt_ref: verified.receipt_ref,
        ticket_ref: verified.ticket_ref,
        peer_admission_ref: verified.peer_admission_ref,
        authority_grant_ref: verified.authority_grant_ref,
        receipt_refs: verified.receipt_refs,
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

#[derive(Debug, Default)]
struct Check {
    receipt_ref: Option<String>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Default)]
struct ImportStep {
    receipt_ref: Option<String>,
    imported_refs: Vec<String>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Default)]
struct TransferStep {
    envelope_ref: Option<String>,
    operation_ref: Option<String>,
    send_receipt_ref: Option<String>,
    send_receipt_value: Option<IoValue>,
    diagnostics: Vec<String>,
}

struct FinishInput<'a> {
    input: &'a ControlLiveWorkflowBundleApplyInput<'a>,
    verified: ControlLiveWorkflowBundleVerify,
    expected: LiveWorkflowBundleExpectedInput<'a>,
    gate_receipt_ref: Option<String>,
    import_receipt_ref: Option<String>,
    imported_refs: Vec<String>,
    envelope_ref: Option<String>,
    operation_ref: Option<String>,
    send_receipt_ref: Option<String>,
    send_receipt_value: Option<IoValue>,
    diagnostics: Vec<String>,
}
