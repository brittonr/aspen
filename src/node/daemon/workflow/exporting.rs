//! Live workflow bundle export and bundle receipt binding checks.

pub fn export_control_live_workflow_bundle(
    input: &super::bundling::ControlLiveWorkflowBundleExportInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleExport> {
    let ticket = super::issuance::parse_control_live_ticket(input.receiver_ticket_value)?;
    let admission = super::peering::parse_control_live_peer_admission(input.peer_admission_value)?;
    let authority = super::issuance::parse_control_authority_grant(input.authority_grant_value)?;
    let receipt_refs = live_workflow_bundle_receipt_refs(input.receipt_values)?;
    let mut diagnostics = live_workflow_bundle_binding_diagnostics(&ticket, &admission, &authority);
    diagnostics.extend(live_workflow_bundle_receipt_diagnostics(input.receipt_values));
    let bundle_value = super::exports::live_workflow_bundle_value(&super::exports::LiveWorkflowBundleValueInput {
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
    let bundle = super::artifacts::ControlLiveWorkflowBundle {
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
    let receipt_value = super::exports::live_workflow_bundle_export_receipt_value(
        &super::exports::LiveWorkflowBundleExportReceiptValueInput {
            decision,
            bundle: &bundle,
            diagnostics: &diagnostics,
        },
    )?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    Ok(super::artifacts::ControlLiveWorkflowBundleExport {
        bundle,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

pub(super) fn live_workflow_bundle_binding_diagnostics(
    ticket: &super::entitlements::ControlLiveTicket,
    admission: &super::entitlements::ControlLivePeerAdmission,
    authority: &super::entitlements::ControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if admission.decision != "pass" {
        diagnostics.push(format!(
            "node control live workflow bundle peer admission {} decision {}",
            admission.admission_ref, admission.decision
        ));
    }
    if admission.ticket_ref != ticket.ticket_ref {
        diagnostics.push("node control live workflow bundle admission does not bind ticket".to_string());
    }
    if admission.node_id != ticket.node_id {
        diagnostics.push("node control live workflow bundle admission node does not match ticket".to_string());
    }
    if admission.topic != ticket.topic {
        diagnostics.push("node control live workflow bundle admission topic does not match ticket".to_string());
    }
    if authority.peer_id != admission.peer_id {
        diagnostics.push("node control live workflow bundle authority peer does not match admission".to_string());
    }
    if authority.node_id != ticket.node_id {
        diagnostics.push("node control live workflow bundle authority node does not match ticket".to_string());
    }
    if !authority.revocation_refs.is_empty() {
        diagnostics.push("node control live workflow bundle authority grant has revocation refs".to_string());
    }
    diagnostics
}

pub(super) fn live_workflow_bundle_receipt_refs(values: &[&preserves::IOValue]) -> crate::error::Result<Vec<String>> {
    let owned_values = values.iter().map(|value| (**value).clone()).collect::<Vec<_>>();
    live_workflow_bundle_receipt_refs_from_values(&owned_values)
}

pub(super) fn live_workflow_bundle_receipt_refs_from_values(
    values: &[preserves::IOValue],
) -> crate::error::Result<Vec<String>> {
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        refs.push(crate::preserves_rail::canonical_hash(value)?);
    }
    Ok(refs)
}

pub(super) fn live_workflow_bundle_receipt_diagnostics(values: &[&preserves::IOValue]) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(values.len());
    for value in values {
        let kind = crate::ledger::artifact_kind(value);
        if !is_live_workflow_bundle_receipt_kind(kind) {
            diagnostics.push(format!("node control live workflow bundle unsupported receipt kind {kind}"));
        }
    }
    diagnostics
}

fn is_live_workflow_bundle_receipt_kind(kind: &str) -> bool {
    matches!(
        kind,
        "node-control-live-ticket-import-receipt"
            | "node-control-authority-grant-import-receipt"
            | "node-control-live-send-receipt"
            | "node-control-live-send-retry-receipt"
            | "node-control-live-send-duplicate-receipt"
            | "node-control-live-workflow-receipt"
            | "node-control-live-workflow-bundle-verify-receipt"
            | "node-control-live-workflow-bundle-gate-receipt"
            | "node-control-live-workflow-bundle-apply-receipt"
            | "node-control-live-workflow-bundle-reconcile-receipt"
            | "node-control-live-transport-receipt"
            | "node-control-live-listener-receipt"
            | "node-control-service-run-receipt"
    )
}
