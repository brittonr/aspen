
pub fn export_control_live_workflow_bundle_ack(
    input: &ControlLiveWorkflowBundleAckExportInput<'_>,
) -> Result<ControlLiveWorkflowBundleAckExport> {
    let reconciled = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
        apply_receipt_value: input.apply_receipt_value,
        send_receipt_value: input.send_receipt_value,
        ingress_receipt_value: input.ingress_receipt_value,
        queue_receipt_value: input.queue_receipt_value,
        control_receipt_value: input.control_receipt_value,
        expected_envelope_ref: None,
        expected_operation_ref: None,
        expected_request_ref: None,
    })?;
    let reconcile = parse_control_live_workflow_bundle_reconcile_receipt(input.reconcile_receipt_value)?;
    let mut diagnostics = live_workflow_bundle_ack_export_diagnostics(input, &reconciled, &reconcile)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let ack_value = live_workflow_bundle_ack_value(&LiveWorkflowBundleAckValueInput {
        apply_receipt_value: input.apply_receipt_value,
        send_receipt_value: input.send_receipt_value,
        ingress_receipt_value: input.ingress_receipt_value,
        queue_receipt_value: input.queue_receipt_value,
        control_receipt_value: input.control_receipt_value,
        reconcile_receipt_value: input.reconcile_receipt_value,
        apply_receipt_ref: &reconcile.apply_receipt_ref,
        send_receipt_ref: reconcile.send_receipt_ref.as_deref(),
        ingress_receipt_ref: reconcile.ingress_receipt_ref.as_deref(),
        queue_receipt_ref: reconcile.queue_receipt_ref.as_deref(),
        control_receipt_ref: reconcile.control_receipt_ref.as_deref(),
        reconcile_receipt_ref: &reconcile.receipt_ref,
        bundle_ref: &reconcile.bundle_ref,
        envelope_ref: reconcile.envelope_ref.as_deref(),
        operation_ref: reconcile.operation_ref.as_deref(),
        request_ref: reconcile.request_ref.as_deref(),
        receiver_decision: &reconcile.decision,
        receiver_diagnostics: &reconcile.diagnostics,
        diagnostics: &diagnostics,
    })?;
    let ack = parse_control_live_workflow_bundle_ack(&ack_value)?;
    let receipt_value = live_workflow_bundle_ack_export_receipt_value(&LiveWorkflowBundleAckExportReceiptValueInput {
        decision,
        ack: &ack,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(ControlLiveWorkflowBundleAckExport {
        receiver_decision: ack.receiver_decision.clone(),
        ack,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

pub fn import_control_live_workflow_bundle_ack(
    input: &ControlLiveWorkflowBundleAckImportInput<'_>,
) -> Result<ControlLiveWorkflowBundleAckImport> {
    validate_live_workflow_bundle_ack_import_input(input)?;
    let state_root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    ensure_state_layout(&state_root)?;
    let ack = parse_control_live_workflow_bundle_ack(input.ack_value)?;
    let mut diagnostics = live_workflow_bundle_ack_import_diagnostics(input, &ack)?;
    let mut imported_refs = Vec::with_capacity(8);
    if diagnostics.is_empty() {
        imported_refs.extend(import_live_workflow_bundle_ack_members(&state_root, &ack)?);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_bundle_ack_import_receipt_value(&LiveWorkflowBundleAckImportReceiptValueInput {
        decision,
        ack: &ack,
        imported_refs: &imported_refs,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    import_artifact(&state_root, &receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(ControlLiveWorkflowBundleAckImport {
        ack_ref: ack.ack_ref.clone(),
        bundle_ref: ack.bundle_ref.clone(),
        imported_refs,
        receiver_decision: ack.receiver_decision.clone(),
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

pub fn gate_control_live_workflow_protocol(
    input: &ControlLiveWorkflowProtocolGateInput<'_>,
) -> Result<ControlLiveWorkflowProtocolGate> {
    validate_live_workflow_protocol_gate_input(input)?;
    let (evidence, diagnostics) = live_workflow_protocol_evidence(input)?;
    let manifest_value = live_workflow_protocol_manifest_value()?;
    let install = crate::protocol_session::install_protocol_manifest_value(&manifest_value)?;
    let authority_refs = vec![evidence.authority_ref.clone()];
    let resource_refs = vec![evidence.resource_ref.clone()];
    let values = run_values(input, &install, &evidence, &authority_refs, &resource_refs)?;
    let gate = crate::protocol_session::gate_protocol_session_lifecycle_with_diagnostics(
        crate::protocol_session::ProtocolSessionGateInput {
            install_receipt: install.value.clone(),
            initial_states: values.initial_state_values.clone(),
            operation_receipts: values.operation_receipt_values.clone(),
            messages: values.message_values.clone(),
            next_states: values.next_state_values.clone(),
        },
        diagnostics,
    )?;
    Ok(ControlLiveWorkflowProtocolGate {
        session_id: evidence.session_id,
        install_receipt_ref: gate.install_ref.clone(),
        protocol_ref: gate.protocol_ref.clone(),
        receipt_ref: gate.receipt_ref.clone(),
        receipt_value: gate.value.clone(),
        decision: gate.decision,
        operation_count: gate.operation_count,
        message_count: gate.message_count,
        diagnostics: gate.diagnostics,
        manifest_value,
        install_receipt_value: install.value,
        initial_state_values: values.initial_state_values,
        operation_receipt_values: values.operation_receipt_values,
        message_values: values.message_values,
        next_state_values: values.next_state_values,
    })
}

struct RolePair {
    sender: crate::protocol_session::ProtocolSessionState,
    receiver: crate::protocol_session::ProtocolSessionState,
}

struct LegInput<'a> {
    origin_state: &'a IoValue,
    target_state: &'a IoValue,
    target_role: &'a str,
    label: &'a str,
    payload_tag: &'a str,
    body_or_ref: &'a IoValue,
    authority_refs: &'a [String],
    resource_refs: &'a [String],
    evidence_refs: Vec<String>,
    carrier_refs: Vec<String>,
    message_label: &'a str,
    origin_label: &'a str,
    target_label: &'a str,
}

struct LegOutput {
    send: crate::protocol_session::ProtocolOperationRun,
    receive: crate::protocol_session::ProtocolOperationRun,
    message: crate::protocol_session::ProtocolMessage,
    origin_next: crate::protocol_session::ProtocolSessionState,
    target_next: crate::protocol_session::ProtocolSessionState,
}

struct RunValues {
    initial_state_values: Vec<IoValue>,
    operation_receipt_values: Vec<IoValue>,
    message_values: Vec<IoValue>,
    next_state_values: Vec<IoValue>,
}

fn start_pair(
    install: &crate::protocol_session::ProtocolInstallReceipt,
    session_id: &str,
    authority_refs: &[String],
    resource_refs: &[String],
) -> Result<RolePair> {
    Ok(RolePair {
        sender: crate::protocol_session::start_protocol_session(
            install,
            "sender",
            session_id,
            authority_refs.to_vec(),
            resource_refs.to_vec(),
        )?,
        receiver: crate::protocol_session::start_protocol_session(
            install,
            "receiver",
            session_id,
            authority_refs.to_vec(),
            resource_refs.to_vec(),
        )?,
    })
}

fn step_leg(input: LegInput<'_>) -> Result<LegOutput> {
    let authority_refs = input.authority_refs.to_vec();
    let resource_refs = input.resource_refs.to_vec();
    let send = crate::protocol_session::send_protocol_message(crate::protocol_session::ProtocolSendInput {
        state: input.origin_state.clone(),
        to_role: input.target_role.to_string(),
        label: input.label.to_string(),
        payload_tag: input.payload_tag.to_string(),
        body_or_ref: input.body_or_ref.clone(),
        authority_refs: authority_refs.clone(),
        resource_refs: resource_refs.clone(),
        evidence_refs: input.evidence_refs,
    })?;
    let message = protocol_message(&send, input.message_label)?;
    let receive = crate::protocol_session::receive_protocol_message(crate::protocol_session::ProtocolReceiveInput {
        state: input.target_state.clone(),
        message: message.value.clone(),
        authority_refs,
        resource_refs,
        carrier_refs: input.carrier_refs,
    })?;
    Ok(LegOutput {
        origin_next: protocol_next_state(&send, input.origin_label)?,
        target_next: protocol_next_state(&receive, input.target_label)?,
        send,
        receive,
        message,
    })
}

fn run_values(
    input: &ControlLiveWorkflowProtocolGateInput<'_>,
    install: &crate::protocol_session::ProtocolInstallReceipt,
    evidence: &LiveWorkflowProtocolEvidence,
    authority_refs: &[String],
    resource_refs: &[String],
) -> Result<RunValues> {
    let initial = start_pair(install, &evidence.session_id, authority_refs, resource_refs)?;
    let handoff = step_leg(LegInput {
        origin_state: &initial.sender.value,
        target_state: &initial.receiver.value,
        target_role: "receiver",
        label: "bundle-handoff",
        payload_tag: "workflow-bundle",
        body_or_ref: input.bundle_value,
        authority_refs,
        resource_refs,
        evidence_refs: vec![evidence.gate_receipt_ref.clone()],
        carrier_refs: vec![evidence.gate_receipt_ref.clone()],
        message_label: "bundle handoff",
        origin_label: "bundle handoff sender",
        target_label: "bundle handoff receiver",
    })?;
    let apply = step_leg(LegInput {
        origin_state: &handoff.origin_next.value,
        target_state: &handoff.target_next.value,
        target_role: "receiver",
        label: "apply-evidence",
        payload_tag: "apply-receipt",
        body_or_ref: input.apply_receipt_value,
        authority_refs,
        resource_refs,
        evidence_refs: vec![evidence.apply_receipt_ref.clone(), evidence.gate_receipt_ref.clone()],
        carrier_refs: vec![evidence.apply_receipt_ref.clone()],
        message_label: "apply evidence",
        origin_label: "apply evidence sender",
        target_label: "apply evidence receiver",
    })?;
    let ack = step_leg(LegInput {
        origin_state: &apply.target_next.value,
        target_state: &apply.origin_next.value,
        target_role: "sender",
        label: "ack-evidence",
        payload_tag: "workflow-ack",
        body_or_ref: input.ack_value,
        authority_refs,
        resource_refs,
        evidence_refs: vec![evidence.reconcile_receipt_ref.clone(), evidence.ack_ref.clone()],
        carrier_refs: vec![evidence.ack_ref.clone()],
        message_label: "workflow ack",
        origin_label: "workflow ack receiver",
        target_label: "workflow ack sender",
    })?;
    Ok(RunValues {
        initial_state_values: vec![initial.sender.value.clone(), initial.receiver.value.clone()],
        operation_receipt_values: vec![
            handoff.send.receipt.value.clone(),
            handoff.receive.receipt.value.clone(),
            apply.send.receipt.value.clone(),
            apply.receive.receipt.value.clone(),
            ack.send.receipt.value.clone(),
            ack.receive.receipt.value.clone(),
        ],
        message_values: vec![handoff.message.value, apply.message.value, ack.message.value],
        next_state_values: vec![
            handoff.origin_next.value,
            handoff.target_next.value,
            apply.origin_next.value,
            apply.target_next.value,
            ack.origin_next.value,
            ack.target_next.value,
        ],
    })
}
