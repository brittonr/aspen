//! Live workflow bundle reconcile and its receiver bindings.

#[derive(Debug, Clone, Copy)]
pub(super) struct ReconcileArtifacts<'a> {
    pub(super) apply: &'a super::artifacts::ControlLiveWorkflowBundleApplyReceipt,
    pub(super) send: Option<&'a super::transits::ControlLiveSendReceipt>,
    pub(super) ingress: Option<&'a super::transits::ControlIngressReceipt>,
    pub(super) queue: Option<&'a super::transits::ControlQueueReceipt>,
    pub(super) control: Option<&'a crate::node_runtime::ControlReceipt>,
}

#[derive(Debug, Clone, Copy)]
struct LiveWorkflowBundleReconcileBindings<'a> {
    send_receipt_ref: Option<&'a str>,
    ingress_receipt_ref: Option<&'a str>,
    queue_receipt_ref: Option<&'a str>,
    control_receipt_ref: Option<&'a str>,
    envelope_ref: Option<&'a str>,
    operation_ref: Option<&'a str>,
    request_ref: Option<&'a str>,
}

pub fn reconcile_control_live_workflow_bundle(
    input: &super::bundling::ControlLiveWorkflowBundleReconcileInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleReconcile> {
    validate_live_workflow_bundle_reconcile_input(input)?;
    let apply = super::receipts::parse_control_live_workflow_bundle_apply_receipt(input.apply_receipt_value)?;
    let send = input.send_receipt_value.map(super::receipts::parse_control_live_send_receipt).transpose()?;
    let ingress = input.ingress_receipt_value.map(super::receipts::parse_control_ingress_receipt).transpose()?;
    let queue = input.queue_receipt_value.map(super::receipts::parse_control_queue_receipt).transpose()?;
    let control = input.control_receipt_value.map(crate::node_runtime::parse_control_receipt).transpose()?;
    let artifacts = ReconcileArtifacts {
        apply: &apply,
        send: send.as_ref(),
        ingress: ingress.as_ref(),
        queue: queue.as_ref(),
        control: control.as_ref(),
    };
    let mut diagnostics = live_workflow_bundle_reconcile_diagnostics(input, &artifacts)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let bindings = live_workflow_bundle_reconcile_bindings(&artifacts);
    let receipt_value = super::reconciles::live_workflow_bundle_reconcile_receipt_value(
        &super::reconciles::LiveWorkflowBundleReconcileReceiptValueInput {
            decision,
            apply_receipt_ref: &apply.receipt_ref,
            bundle_ref: &apply.bundle_ref,
            send_receipt_ref: bindings.send_receipt_ref,
            ingress_receipt_ref: bindings.ingress_receipt_ref,
            queue_receipt_ref: bindings.queue_receipt_ref,
            control_receipt_ref: bindings.control_receipt_ref,
            envelope_ref: bindings.envelope_ref,
            operation_ref: bindings.operation_ref,
            request_ref: bindings.request_ref,
            diagnostics: &diagnostics,
        },
    )?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(super::artifacts::ControlLiveWorkflowBundleReconcile {
        bundle_ref: apply.bundle_ref.clone(),
        apply_receipt_ref: apply.receipt_ref.clone(),
        send_receipt_ref: bindings.send_receipt_ref.map(ToString::to_string),
        ingress_receipt_ref: bindings.ingress_receipt_ref.map(ToString::to_string),
        queue_receipt_ref: bindings.queue_receipt_ref.map(ToString::to_string),
        control_receipt_ref: bindings.control_receipt_ref.map(ToString::to_string),
        envelope_ref: bindings.envelope_ref.map(ToString::to_string),
        operation_ref: bindings.operation_ref.map(ToString::to_string),
        request_ref: bindings.request_ref.map(ToString::to_string),
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

fn validate_live_workflow_bundle_reconcile_input(
    input: &super::bundling::ControlLiveWorkflowBundleReconcileInput<'_>,
) -> crate::error::Result<()> {
    if let Some(reference) = input.expected_envelope_ref {
        super::guards::validate_ingress_ref(
            reference,
            "node control live workflow bundle reconcile expected envelope ref",
        )?;
    }
    if let Some(reference) = input.expected_operation_ref {
        super::guards::validate_ingress_ref(
            reference,
            "node control live workflow bundle reconcile expected operation ref",
        )?;
    }
    if let Some(reference) = input.expected_request_ref {
        super::guards::validate_ingress_ref(
            reference,
            "node control live workflow bundle reconcile expected request ref",
        )?;
    }
    Ok(())
}

fn live_workflow_bundle_reconcile_diagnostics(
    input: &super::bundling::ControlLiveWorkflowBundleReconcileInput<'_>,
    artifacts: &ReconcileArtifacts<'_>,
) -> crate::error::Result<Vec<String>> {
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
    diagnostics.extend(super::discrepancies::live_workflow_bundle_reconcile_send_diagnostics(artifacts));
    diagnostics.extend(super::discrepancies::live_workflow_bundle_reconcile_ingress_diagnostics(input, artifacts));
    diagnostics.extend(super::discrepancies::live_workflow_bundle_reconcile_queue_diagnostics(artifacts));
    diagnostics.extend(super::discrepancies::live_workflow_bundle_reconcile_control_diagnostics(artifacts));
    Ok(diagnostics)
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
