//! Pure live workflow lifecycle ordering evaluation.

pub fn evaluate_live_workflow_lifecycle(
    input: super::entitlements::LiveWorkflowLifecycleInput<'_>,
) -> super::entitlements::LiveWorkflowLifecycleDecision {
    let mut diagnostics = Vec::with_capacity(super::settings::LIVE_WORKFLOW_LIFECYCLE_DIAGNOSTIC_CAPACITY);
    live_workflow_required_steps(&input, &mut diagnostics);
    live_workflow_step_decisions(&input, &mut diagnostics);
    live_workflow_ordered_links(&input, &mut diagnostics);
    live_workflow_expected_refs(&input, &mut diagnostics);
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    super::entitlements::LiveWorkflowLifecycleDecision {
        decision: decision.to_string(),
        diagnostics,
    }
}

fn live_workflow_required_steps(
    input: &super::entitlements::LiveWorkflowLifecycleInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    if input.bundle.is_none() {
        diagnostics.push_item("node-control-live-workflow-bundle-missing".to_string());
    }
    if input.gate.is_none() {
        diagnostics.push_item("node-control-live-workflow-gate-missing".to_string());
    }
    if input.apply.is_none() {
        diagnostics.push_item("node-control-live-workflow-apply-missing".to_string());
    }
    if input.reconcile.is_none() {
        diagnostics.push_item("node-control-live-workflow-reconcile-missing".to_string());
    }
    if input.ack.is_none() {
        diagnostics.push_item("node-control-live-workflow-ack-missing".to_string());
    }
    if input.ack_import.is_none() && input.protocol_gate.is_none() {
        diagnostics.push_item("node-control-live-workflow-terminal-evidence-missing".to_string());
    }
}

fn live_workflow_step_decisions(
    input: &super::entitlements::LiveWorkflowLifecycleInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    if let Some(gate) = input.gate
        && gate.decision != "pass"
    {
        diagnostics.push_item(format!("node-control-live-workflow-gate-decision-{}", gate.decision));
        diagnostics.extend_items(gate.diagnostics.iter().cloned());
    }
    if let Some(apply) = input.apply
        && apply.decision != "pass"
    {
        diagnostics.push_item(format!("node-control-live-workflow-apply-decision-{}", apply.decision));
        diagnostics.extend_items(apply.diagnostics.iter().cloned());
    }
    if let Some(reconcile) = input.reconcile
        && reconcile.decision != "pass"
    {
        diagnostics.push_item(format!("node-control-live-workflow-reconcile-decision-{}", reconcile.decision));
        diagnostics.extend_items(reconcile.diagnostics.iter().cloned());
    }
    if let Some(ack) = input.ack {
        if ack.receiver_decision != "pass" {
            diagnostics
                .push_item(format!("node-control-live-workflow-ack-receiver-decision-{}", ack.receiver_decision));
            diagnostics.extend_items(ack.receiver_diagnostics.iter().cloned());
        }
        diagnostics.extend_items(ack.diagnostics.iter().cloned());
    }
    if let Some(ack_import) = input.ack_import
        && ack_import.decision != "pass"
    {
        diagnostics.push_item(format!("node-control-live-workflow-ack-import-decision-{}", ack_import.decision));
        diagnostics.extend_items(ack_import.diagnostics.iter().cloned());
    }
    if let Some(protocol_gate) = input.protocol_gate
        && protocol_gate.decision != "pass"
    {
        diagnostics.push_item(format!("node-control-live-workflow-protocol-gate-decision-{}", protocol_gate.decision));
        diagnostics.extend_items(protocol_gate.diagnostics.iter().cloned());
    }
}

fn live_workflow_ordered_links(
    input: &super::entitlements::LiveWorkflowLifecycleInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    if let (Some(bundle), Some(gate)) = (input.bundle, input.gate)
        && gate.bundle_ref != bundle.bundle_ref
    {
        diagnostics.push_item("node-control-live-workflow-gate-bundle-mismatch".to_string());
    }
    if let (Some(bundle), Some(apply)) = (input.bundle, input.apply)
        && apply.bundle_ref != bundle.bundle_ref
    {
        diagnostics.push_item("node-control-live-workflow-apply-bundle-mismatch".to_string());
    }
    if let (Some(gate), Some(apply)) = (input.gate, input.apply) {
        if apply.gate_receipt_ref.as_deref() != Some(gate.receipt_ref.as_str()) {
            diagnostics.push_item("node-control-live-workflow-apply-gate-mismatch".to_string());
        }
        if apply.recomputed_verify_receipt_ref != gate.recomputed_verify_receipt_ref {
            diagnostics.push_item("node-control-live-workflow-apply-verify-mismatch".to_string());
        }
    }
    if let (Some(apply), Some(reconcile)) = (input.apply, input.reconcile) {
        if reconcile.apply_receipt_ref != apply.receipt_ref {
            diagnostics.push_item("node-control-live-workflow-reconcile-apply-mismatch".to_string());
        }
        if reconcile.bundle_ref != apply.bundle_ref {
            diagnostics.push_item("node-control-live-workflow-reconcile-bundle-mismatch".to_string());
        }
        if reconcile.send_receipt_ref != apply.send_receipt_ref {
            diagnostics.push_item("node-control-live-workflow-reconcile-send-mismatch".to_string());
        }
        if reconcile.envelope_ref != apply.envelope_ref {
            diagnostics.push_item("node-control-live-workflow-reconcile-envelope-mismatch".to_string());
        }
        if reconcile.operation_ref != apply.operation_ref {
            diagnostics.push_item("node-control-live-workflow-reconcile-operation-mismatch".to_string());
        }
    }
    if let (Some(reconcile), Some(ack)) = (input.reconcile, input.ack) {
        if ack.reconcile_receipt_ref != reconcile.receipt_ref {
            diagnostics.push_item("node-control-live-workflow-ack-reconcile-mismatch".to_string());
        }
        if ack.bundle_ref != reconcile.bundle_ref {
            diagnostics.push_item("node-control-live-workflow-ack-bundle-mismatch".to_string());
        }
        if ack.envelope_ref != reconcile.envelope_ref {
            diagnostics.push_item("node-control-live-workflow-ack-envelope-mismatch".to_string());
        }
        if ack.operation_ref != reconcile.operation_ref {
            diagnostics.push_item("node-control-live-workflow-ack-operation-mismatch".to_string());
        }
        if ack.request_ref != reconcile.request_ref {
            diagnostics.push_item("node-control-live-workflow-ack-request-mismatch".to_string());
        }
    }
    if let (Some(apply), Some(ack)) = (input.apply, input.ack)
        && ack.apply_receipt_ref != apply.receipt_ref
    {
        diagnostics.push_item("node-control-live-workflow-ack-apply-mismatch".to_string());
    }
    if let (Some(ack), Some(ack_import)) = (input.ack, input.ack_import) {
        if ack_import.ack_ref != ack.ack_ref {
            diagnostics.push_item("node-control-live-workflow-ack-import-ack-mismatch".to_string());
        }
        if ack_import.bundle_ref != ack.bundle_ref {
            diagnostics.push_item("node-control-live-workflow-ack-import-bundle-mismatch".to_string());
        }
    }
}

fn live_workflow_expected_refs(
    input: &super::entitlements::LiveWorkflowLifecycleInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    live_workflow_note_expected(
        diagnostics,
        "node-control-live-workflow-expected-bundle-mismatch",
        input.bundle.map(|bundle| bundle.bundle_ref.as_str()),
        input.expected_bundle_ref,
    );
    let envelope_ref = input.ack.and_then(|ack| ack.envelope_ref.as_deref()).or_else(|| {
        input
            .reconcile
            .and_then(|reconcile| reconcile.envelope_ref.as_deref())
            .or_else(|| input.apply.and_then(|apply| apply.envelope_ref.as_deref()))
    });
    live_workflow_note_expected(
        diagnostics,
        "node-control-live-workflow-expected-envelope-mismatch",
        envelope_ref,
        input.expected_envelope_ref,
    );
    let operation_ref = input.ack.and_then(|ack| ack.operation_ref.as_deref()).or_else(|| {
        input
            .reconcile
            .and_then(|reconcile| reconcile.operation_ref.as_deref())
            .or_else(|| input.apply.and_then(|apply| apply.operation_ref.as_deref()))
    });
    live_workflow_note_expected(
        diagnostics,
        "node-control-live-workflow-expected-operation-mismatch",
        operation_ref,
        input.expected_operation_ref,
    );
    let request_ref = input
        .ack
        .and_then(|ack| ack.request_ref.as_deref())
        .or_else(|| input.reconcile.and_then(|reconcile| reconcile.request_ref.as_deref()));
    live_workflow_note_expected(
        diagnostics,
        "node-control-live-workflow-expected-request-mismatch",
        request_ref,
        input.expected_request_ref,
    );
}

fn live_workflow_note_expected(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    diagnostic: &str,
    observed: Option<&str>,
    expected: Option<&str>,
) {
    if let Some(expected) = expected
        && observed != Some(expected)
    {
        diagnostics.push_item(format!("{diagnostic}:{} != {expected}", observed.unwrap_or("none")));
    }
}
