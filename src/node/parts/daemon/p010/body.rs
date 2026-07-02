
fn live_workflow_protocol_manifest_value() -> Result<IoValue> {
    let global = crate::protocol_session::protocol_global_script_value(&[
        crate::protocol_session::ProtocolCommInput {
            from_role: "sender".to_string(),
            to_role: "receiver".to_string(),
            label: "bundle-handoff".to_string(),
            payload_tag: "workflow-bundle".to_string(),
        },
        crate::protocol_session::ProtocolCommInput {
            from_role: "sender".to_string(),
            to_role: "receiver".to_string(),
            label: "apply-evidence".to_string(),
            payload_tag: "apply-receipt".to_string(),
        },
        crate::protocol_session::ProtocolCommInput {
            from_role: "receiver".to_string(),
            to_role: "sender".to_string(),
            label: "ack-evidence".to_string(),
            payload_tag: "workflow-ack".to_string(),
        },
    ])?;
    crate::protocol_session::protocol_manifest_value(&crate::protocol_session::ProtocolManifestInput {
        protocol_id: LIVE_WORKFLOW_PROTOCOL_ID.to_string(),
        roles: vec!["sender".to_string(), "receiver".to_string()],
        labels: vec![
            "bundle-handoff".to_string(),
            "apply-evidence".to_string(),
            "ack-evidence".to_string(),
        ],
        payloads: vec![
            crate::protocol_session::ProtocolPayloadInput {
                tag: "workflow-bundle".to_string(),
                schema_ref: local_ref("node-control-live-workflow-protocol-schema", "workflow-bundle")?,
            },
            crate::protocol_session::ProtocolPayloadInput {
                tag: "apply-receipt".to_string(),
                schema_ref: local_ref("node-control-live-workflow-protocol-schema", "apply-receipt")?,
            },
            crate::protocol_session::ProtocolPayloadInput {
                tag: "workflow-ack".to_string(),
                schema_ref: local_ref("node-control-live-workflow-protocol-schema", "workflow-ack")?,
            },
        ],
        global,
        policy_refs: vec![local_ref("node-control-live-workflow-protocol-policy", "v1")?],
        capability_refs: vec![local_ref("node-control-live-workflow-protocol-capability", "v1")?],
        resource_refs: vec![local_ref("node-control-live-workflow-protocol-resource", "v1")?],
    })
}

fn protocol_message(
    run: &crate::protocol_session::ProtocolOperationRun,
    label: &str,
) -> Result<crate::protocol_session::ProtocolMessage> {
    run.message.clone().ok_or_else(|| {
        MoltenError::invalid_harness(format!("node control live workflow protocol missing {label} message"))
    })
}

fn protocol_next_state(
    run: &crate::protocol_session::ProtocolOperationRun,
    label: &str,
) -> Result<crate::protocol_session::ProtocolSessionState> {
    run.next_state.clone().ok_or_else(|| {
        MoltenError::invalid_harness(format!("node control live workflow protocol missing {label} next state"))
    })
}

fn validate_live_workflow_bundle_reconcile_input(input: &ControlLiveWorkflowBundleReconcileInput<'_>) -> Result<()> {
    if let Some(reference) = input.expected_envelope_ref {
        validate_ingress_ref(reference, "node control live workflow bundle reconcile expected envelope ref")?;
    }
    if let Some(reference) = input.expected_operation_ref {
        validate_ingress_ref(reference, "node control live workflow bundle reconcile expected operation ref")?;
    }
    if let Some(reference) = input.expected_request_ref {
        validate_ingress_ref(reference, "node control live workflow bundle reconcile expected request ref")?;
    }
    Ok(())
}

fn live_workflow_bundle_reconcile_diagnostics(
    input: &ControlLiveWorkflowBundleReconcileInput<'_>,
    artifacts: &ReconcileArtifacts<'_>,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(16);
    if artifacts.apply.decision != "pass" {
        diagnostics.push(format!(
            "node control live workflow bundle reconcile apply receipt {} decision {}",
            artifacts.apply.receipt_ref, artifacts.apply.decision
        ));
        diagnostics.extend(artifacts.apply.diagnostics.clone());
    }
    if artifacts.apply.envelope_ref.is_none() {
        diagnostics.push(
            "node control live workflow bundle reconcile apply receipt has no live envelope; rerun apply with --request"
                .to_string(),
        );
    }
    diagnostics.extend(live_workflow_bundle_reconcile_send_diagnostics(artifacts));
    diagnostics.extend(live_workflow_bundle_reconcile_ingress_diagnostics(input, artifacts));
    diagnostics.extend(live_workflow_bundle_reconcile_queue_diagnostics(artifacts));
    diagnostics.extend(live_workflow_bundle_reconcile_control_diagnostics(artifacts));
    Ok(diagnostics)
}

fn live_workflow_bundle_reconcile_send_diagnostics(artifacts: &ReconcileArtifacts<'_>) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    match (&artifacts.apply.send_receipt_ref, artifacts.send) {
        (Some(expected), Some(send)) => {
            if send.receipt_ref != *expected {
                diagnostics.push(format!(
                    "node control live workflow bundle reconcile send receipt {} does not match apply {}",
                    send.receipt_ref, expected
                ));
            }
        }
        (Some(expected), None) => diagnostics.push(format!(
            "node control live workflow bundle reconcile requires send receipt {expected} from apply receipt"
        )),
        (None, Some(_)) => {}
        (None, None) => {}
    }
    if let Some(send) = artifacts.send {
        if send.decision != "pass" {
            diagnostics.push(format!(
                "node control live workflow bundle reconcile send receipt {} decision {}",
                send.receipt_ref, send.decision
            ));
            diagnostics.extend(send.diagnostics.clone());
        }
        if let Some(envelope_ref) = artifacts.apply.envelope_ref.as_ref()
            && send.envelope_ref != *envelope_ref
        {
            diagnostics.push(format!(
                "node control live workflow bundle reconcile send envelope {} does not match apply {}",
                send.envelope_ref, envelope_ref
            ));
        }
    }
    diagnostics
}

fn receiver_ref_note(kind: &str, actual: &str, expected: Option<&str>, source: &str) -> Option<String> {
    let expected = expected?;
    if actual == expected {
        None
    } else {
        Some(format!(
            "node control live workflow bundle reconcile receiver {kind} {actual} does not match {source} {expected}"
        ))
    }
}

fn live_workflow_bundle_reconcile_ingress_diagnostics(
    input: &ControlLiveWorkflowBundleReconcileInput<'_>,
    artifacts: &ReconcileArtifacts<'_>,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    let Some(ingress) = artifacts.ingress else {
        diagnostics.push(
            "node control live workflow bundle reconcile requires receiver ingress receipt for the live envelope"
                .to_string(),
        );
        return diagnostics;
    };
    if ingress.decision != "pass" {
        diagnostics.push(format!(
            "node control live workflow bundle reconcile receiver ingress receipt {} decision {}",
            ingress.receipt_ref, ingress.decision
        ));
        diagnostics.extend(ingress.diagnostics.clone());
    }
    if let Some(note) = receiver_ref_note("envelope", &ingress.envelope_ref, input.expected_envelope_ref, "expected") {
        diagnostics.push(note);
    }
    if let Some(note) =
        receiver_ref_note("envelope", &ingress.envelope_ref, artifacts.apply.envelope_ref.as_deref(), "apply")
    {
        diagnostics.push(note);
    }
    if let Some(send) = artifacts.send
        && let Some(note) =
            receiver_ref_note("envelope", &ingress.envelope_ref, Some(send.envelope_ref.as_str()), "send")
    {
        diagnostics.push(note);
    }
    if let Some(note) = receiver_ref_note("operation", &ingress.operation_ref, input.expected_operation_ref, "expected")
    {
        diagnostics.push(note);
    }
    if let Some(note) =
        receiver_ref_note("operation", &ingress.operation_ref, artifacts.apply.operation_ref.as_deref(), "apply")
    {
        diagnostics.push(note);
    }
    if let Some(note) = receiver_ref_note("request", &ingress.request_ref, input.expected_request_ref, "expected") {
        diagnostics.push(note);
    }
    if ingress.decision == "pass" && ingress.queue_receipt_ref.is_none() {
        diagnostics.push(
            "node control live workflow bundle reconcile receiver ingress passed without durable queue receipt"
                .to_string(),
        );
    }
    diagnostics
}

fn live_workflow_bundle_reconcile_queue_diagnostics(artifacts: &ReconcileArtifacts<'_>) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    if let Some(queue) = artifacts.queue {
        if queue.decision != "pass" {
            diagnostics.push(format!(
                "node control live workflow bundle reconcile queue receipt {} decision {}",
                queue.receipt_ref, queue.decision
            ));
            diagnostics.extend(queue.diagnostics.clone());
        }
        if let Some(ingress) = artifacts.ingress {
            if let Some(expected) = ingress.queue_receipt_ref.as_ref()
                && queue.receipt_ref != *expected
            {
                diagnostics.push(format!(
                    "node control live workflow bundle reconcile queue receipt {} does not match ingress {}",
                    queue.receipt_ref, expected
                ));
            }
            if queue.request_ref != ingress.request_ref {
                diagnostics.push(format!(
                    "node control live workflow bundle reconcile queue request {} does not match ingress {}",
                    queue.request_ref, ingress.request_ref
                ));
            }
        }
    }
    diagnostics
}

fn live_workflow_bundle_reconcile_control_diagnostics(artifacts: &ReconcileArtifacts<'_>) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(4);
    if let Some(control) = artifacts.control {
        if control.decision != "pass" {
            diagnostics.push(format!(
                "node control live workflow bundle reconcile receiver control receipt {} decision {}",
                control.receipt_ref, control.decision
            ));
            diagnostics.extend(control.diagnostics.clone());
        }
        if let Some(ingress) = artifacts.ingress
            && control.request_ref != ingress.request_ref
        {
            diagnostics.push(format!(
                "node control live workflow bundle reconcile control request {} does not match ingress {}",
                control.request_ref, ingress.request_ref
            ));
        }
    }
    diagnostics
}

fn live_workflow_bundle_reconcile_bindings<'a>(
    artifacts: &'a ReconcileArtifacts<'a>,
) -> LiveWorkflowBundleReconcileBindings<'a> {
    let send_receipt_ref =
        artifacts.send.map(|send| send.receipt_ref.as_str()).or(artifacts.apply.send_receipt_ref.as_deref());
    let ingress_receipt_ref = artifacts.ingress.map(|ingress| ingress.receipt_ref.as_str());
    let queue_receipt_ref = artifacts
        .queue
        .map(|queue| queue.receipt_ref.as_str())
        .or_else(|| artifacts.ingress.and_then(|ingress| ingress.queue_receipt_ref.as_deref()));
    let control_receipt_ref = artifacts.control.map(|control| control.receipt_ref.as_str());
    let envelope_ref = artifacts
        .ingress
        .map(|ingress| ingress.envelope_ref.as_str())
        .or_else(|| artifacts.send.map(|send| send.envelope_ref.as_str()))
        .or(artifacts.apply.envelope_ref.as_deref());
    let operation_ref = artifacts
        .ingress
        .map(|ingress| ingress.operation_ref.as_str())
        .or(artifacts.apply.operation_ref.as_deref());
    let request_ref = artifacts
        .control
        .map(|control| control.request_ref.as_str())
        .or_else(|| artifacts.queue.map(|queue| queue.request_ref.as_str()))
        .or_else(|| artifacts.ingress.map(|ingress| ingress.request_ref.as_str()));
    LiveWorkflowBundleReconcileBindings {
        send_receipt_ref,
        ingress_receipt_ref,
        queue_receipt_ref,
        control_receipt_ref,
        envelope_ref,
        operation_ref,
        request_ref,
    }
}
