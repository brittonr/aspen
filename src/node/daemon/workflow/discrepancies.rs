//! Reconcile diagnostics for send, ingress, queue, and control receipts.

pub(super) fn live_workflow_bundle_reconcile_send_diagnostics(
    artifacts: &super::reconciling::ReconcileArtifacts<'_>,
) -> Vec<String> {
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

pub(super) fn live_workflow_bundle_reconcile_ingress_diagnostics(
    input: &super::bundling::ControlLiveWorkflowBundleReconcileInput<'_>,
    artifacts: &super::reconciling::ReconcileArtifacts<'_>,
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

pub(super) fn live_workflow_bundle_reconcile_queue_diagnostics(
    artifacts: &super::reconciling::ReconcileArtifacts<'_>,
) -> Vec<String> {
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

pub(super) fn live_workflow_bundle_reconcile_control_diagnostics(
    artifacts: &super::reconciling::ReconcileArtifacts<'_>,
) -> Vec<String> {
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
