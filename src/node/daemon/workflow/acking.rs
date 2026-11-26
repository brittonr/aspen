//! Live workflow bundle acknowledgement export and import.

pub fn export_control_live_workflow_bundle_ack(
    input: &super::bundling::ControlLiveWorkflowBundleAckExportInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleAckExport> {
    let reconciled = super::reconciling::reconcile_control_live_workflow_bundle(
        &super::bundling::ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: input.apply_receipt_value,
            send_receipt_value: input.send_receipt_value,
            ingress_receipt_value: input.ingress_receipt_value,
            queue_receipt_value: input.queue_receipt_value,
            control_receipt_value: input.control_receipt_value,
            expected_envelope_ref: None,
            expected_operation_ref: None,
            expected_request_ref: None,
        },
    )?;
    let reconcile =
        super::receipts::parse_control_live_workflow_bundle_reconcile_receipt(input.reconcile_receipt_value)?;
    let mut diagnostics = live_workflow_bundle_ack_export_diagnostics(input, &reconciled, &reconcile)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let ack_value =
        super::reconciles::live_workflow_bundle_ack_value(&super::reconciles::LiveWorkflowBundleAckValueInput {
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
    let ack = super::acknowledgements::parse_control_live_workflow_bundle_ack(&ack_value)?;
    let receipt_value = super::acks::live_workflow_bundle_ack_export_receipt_value(
        &super::acks::LiveWorkflowBundleAckExportReceiptValueInput {
            decision,
            ack: &ack,
            diagnostics: &diagnostics,
        },
    )?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(super::artifacts::ControlLiveWorkflowBundleAckExport {
        receiver_decision: ack.receiver_decision.clone(),
        ack,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}

pub fn import_control_live_workflow_bundle_ack(
    input: &super::bundling::ControlLiveWorkflowBundleAckImportInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleAckImport> {
    validate_live_workflow_bundle_ack_import_input(input)?;
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    super::storage::ensure_state_layout(&state_root)?;
    let ack = super::acknowledgements::parse_control_live_workflow_bundle_ack(input.ack_value)?;
    let mut diagnostics = live_workflow_bundle_ack_import_diagnostics(input, &ack)?;
    let mut imported_refs = Vec::with_capacity(8);
    if diagnostics.is_empty() {
        imported_refs.extend(import_live_workflow_bundle_ack_members(&state_root, &ack)?);
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = super::acks::live_workflow_bundle_ack_import_receipt_value(
        &super::acks::LiveWorkflowBundleAckImportReceiptValueInput {
            decision,
            ack: &ack,
            imported_refs: &imported_refs,
            diagnostics: &diagnostics,
        },
    )?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::import_artifact(&state_root, &receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(super::artifacts::ControlLiveWorkflowBundleAckImport {
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

fn live_workflow_bundle_ack_export_diagnostics(
    input: &super::bundling::ControlLiveWorkflowBundleAckExportInput<'_>,
    reconciled: &super::artifacts::ControlLiveWorkflowBundleReconcile,
    reconcile: &super::artifacts::ControlLiveWorkflowBundleReconcileReceipt,
) -> crate::error::Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    if reconcile.receipt_ref != reconciled.receipt_ref {
        diagnostics.push(format!(
            "node control live workflow bundle ack reconcile receipt {} does not match recomputed {}",
            reconcile.receipt_ref, reconciled.receipt_ref
        ));
    }
    if input.ingress_receipt_value.is_none() {
        diagnostics.push("node control live workflow bundle ack requires receiver ingress receipt".to_string());
    }
    let ingress = input.ingress_receipt_value.map(super::receipts::parse_control_ingress_receipt).transpose()?;
    if let Some(ingress) = ingress.as_ref() {
        if ingress.decision == "pass" && input.queue_receipt_value.is_none() {
            diagnostics.push(format!(
                "node control live workflow bundle ack requires queue receipt {} from receiver ingress",
                ingress.queue_receipt_ref.as_deref().unwrap_or("none")
            ));
        }
        if let Some(queue_receipt_ref) = ingress.queue_receipt_ref.as_ref()
            && input.queue_receipt_value.is_none()
        {
            diagnostics.push(format!(
                "node control live workflow bundle ack missing durable queue receipt {queue_receipt_ref}"
            ));
        }
    }
    Ok(diagnostics)
}

fn validate_live_workflow_bundle_ack_import_input(
    input: &super::bundling::ControlLiveWorkflowBundleAckImportInput<'_>,
) -> crate::error::Result<()> {
    super::storage::validate_state_root(input.state_root)?;
    if let Some(reference) = input.expected_bundle_ref {
        super::guards::validate_ingress_ref(
            reference,
            "node control live workflow bundle ack import expected bundle ref",
        )?;
    }
    if let Some(reference) = input.expected_envelope_ref {
        super::guards::validate_ingress_ref(
            reference,
            "node control live workflow bundle ack import expected envelope ref",
        )?;
    }
    if let Some(reference) = input.expected_operation_ref {
        super::guards::validate_ingress_ref(
            reference,
            "node control live workflow bundle ack import expected operation ref",
        )?;
    }
    if let Some(reference) = input.expected_request_ref {
        super::guards::validate_ingress_ref(
            reference,
            "node control live workflow bundle ack import expected request ref",
        )?;
    }
    Ok(())
}

fn live_workflow_bundle_ack_import_diagnostics(
    input: &super::bundling::ControlLiveWorkflowBundleAckImportInput<'_>,
    ack: &super::artifacts::ControlLiveWorkflowBundleAck,
) -> crate::error::Result<Vec<String>> {
    let recomputed = super::reconciling::reconcile_control_live_workflow_bundle(
        &super::bundling::ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &ack.apply_receipt_value,
            send_receipt_value: ack.send_receipt_value.as_ref(),
            ingress_receipt_value: ack.ingress_receipt_value.as_ref(),
            queue_receipt_value: ack.queue_receipt_value.as_ref(),
            control_receipt_value: ack.control_receipt_value.as_ref(),
            expected_envelope_ref: None,
            expected_operation_ref: None,
            expected_request_ref: None,
        },
    )?;
    let mut diagnostics = ack.diagnostics.clone();
    if ack.reconcile_receipt_ref != recomputed.receipt_ref {
        diagnostics.push(format!(
            "node control live workflow bundle ack import reconcile receipt {} does not match recomputed {}",
            ack.reconcile_receipt_ref, recomputed.receipt_ref
        ));
    }
    if ack.ingress_receipt_value.is_none() {
        diagnostics.push("node control live workflow bundle ack import requires receiver ingress receipt".to_string());
    }
    if let Some(ingress_value) = ack.ingress_receipt_value.as_ref() {
        let ingress = super::receipts::parse_control_ingress_receipt(ingress_value)?;
        if ingress.decision == "pass" && ack.queue_receipt_value.is_none() {
            diagnostics.push(format!(
                "node control live workflow bundle ack import requires queue receipt {} from receiver ingress",
                ingress.queue_receipt_ref.as_deref().unwrap_or("none")
            ));
        }
    }
    if let Some(expected) = input.expected_bundle_ref
        && ack.bundle_ref != expected
    {
        diagnostics.push(format!(
            "node control live workflow bundle ack import bundle {} does not match expected {}",
            ack.bundle_ref, expected
        ));
    }
    if let Some(expected) = input.expected_envelope_ref
        && ack.envelope_ref.as_deref() != Some(expected)
    {
        diagnostics.push(format!(
            "node control live workflow bundle ack import envelope {} does not match expected {}",
            ack.envelope_ref.as_deref().unwrap_or("none"),
            expected
        ));
    }
    if let Some(expected) = input.expected_operation_ref
        && ack.operation_ref.as_deref() != Some(expected)
    {
        diagnostics.push(format!(
            "node control live workflow bundle ack import operation {} does not match expected {}",
            ack.operation_ref.as_deref().unwrap_or("none"),
            expected
        ));
    }
    if let Some(expected) = input.expected_request_ref
        && ack.request_ref.as_deref() != Some(expected)
    {
        diagnostics.push(format!(
            "node control live workflow bundle ack import request {} does not match expected {}",
            ack.request_ref.as_deref().unwrap_or("none"),
            expected
        ));
    }
    Ok(diagnostics)
}

fn import_live_workflow_bundle_ack_members(
    state_root: &crate::node_state::DirectoryRoot,
    ack: &super::artifacts::ControlLiveWorkflowBundleAck,
) -> crate::error::Result<Vec<String>> {
    let mut imported_refs = Vec::with_capacity(8);
    imported_refs.push(super::storage::import_artifact(state_root, &ack.apply_receipt_value)?);
    if let Some(value) = ack.send_receipt_value.as_ref() {
        imported_refs.push(super::storage::import_artifact(state_root, value)?);
    }
    if let Some(value) = ack.ingress_receipt_value.as_ref() {
        imported_refs.push(super::storage::import_artifact(state_root, value)?);
    }
    if let Some(value) = ack.queue_receipt_value.as_ref() {
        imported_refs.push(super::storage::import_artifact(state_root, value)?);
    }
    if let Some(value) = ack.control_receipt_value.as_ref() {
        imported_refs.push(super::storage::import_artifact(state_root, value)?);
    }
    imported_refs.push(super::storage::import_artifact(state_root, &ack.reconcile_receipt_value)?);
    imported_refs.push(super::storage::import_artifact(state_root, &ack.ack_value)?);
    Ok(imported_refs)
}
