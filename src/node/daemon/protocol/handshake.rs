//! Live workflow protocol gate: role pairs and the session message run.

pub fn gate_control_live_workflow_protocol(
    input: &super::bundling::ControlLiveWorkflowProtocolGateInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowProtocolGate> {
    super::transcript::validate_live_workflow_protocol_gate_input(input)?;
    let (evidence, diagnostics) = super::transcript::live_workflow_protocol_evidence(input)?;
    let manifest_value = super::transcript::live_workflow_protocol_manifest_value()?;
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
    Ok(super::artifacts::ControlLiveWorkflowProtocolGate {
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
    origin_state: &'a preserves::IOValue,
    target_state: &'a preserves::IOValue,
    target_role: &'a str,
    label: &'a str,
    payload_tag: &'a str,
    body_or_ref: &'a preserves::IOValue,
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
    initial_state_values: Vec<preserves::IOValue>,
    operation_receipt_values: Vec<preserves::IOValue>,
    message_values: Vec<preserves::IOValue>,
    next_state_values: Vec<preserves::IOValue>,
}

fn start_pair(
    install: &crate::protocol_session::ProtocolInstallReceipt,
    session_id: &str,
    authority_refs: &[String],
    resource_refs: &[String],
) -> crate::error::Result<RolePair> {
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

fn step_leg(input: LegInput<'_>) -> crate::error::Result<LegOutput> {
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
    input: &super::bundling::ControlLiveWorkflowProtocolGateInput<'_>,
    install: &crate::protocol_session::ProtocolInstallReceipt,
    evidence: &super::transcript::LiveWorkflowProtocolEvidence,
    authority_refs: &[String],
    resource_refs: &[String],
) -> crate::error::Result<RunValues> {
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

fn protocol_message(
    run: &crate::protocol_session::ProtocolOperationRun,
    label: &str,
) -> crate::error::Result<crate::protocol_session::ProtocolMessage> {
    run.message.clone().ok_or_else(|| {
        crate::error::Failure::invalid_harness(format!("node control live workflow protocol missing {label} message"))
    })
}

fn protocol_next_state(
    run: &crate::protocol_session::ProtocolOperationRun,
    label: &str,
) -> crate::error::Result<crate::protocol_session::ProtocolSessionState> {
    run.next_state.clone().ok_or_else(|| {
        crate::error::Failure::invalid_harness(format!(
            "node control live workflow protocol missing {label} next state"
        ))
    })
}
