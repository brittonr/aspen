
fn live_workflow_bundle_ack_export_diagnostics(
    input: &ControlLiveWorkflowBundleAckExportInput<'_>,
    reconciled: &ControlLiveWorkflowBundleReconcile,
    reconcile: &ControlLiveWorkflowBundleReconcileReceipt,
) -> Result<Vec<String>> {
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
    let ingress = input.ingress_receipt_value.map(parse_control_ingress_receipt).transpose()?;
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

fn validate_live_workflow_bundle_ack_import_input(input: &ControlLiveWorkflowBundleAckImportInput<'_>) -> Result<()> {
    validate_state_root(input.state_root)?;
    if let Some(reference) = input.expected_bundle_ref {
        validate_ingress_ref(reference, "node control live workflow bundle ack import expected bundle ref")?;
    }
    if let Some(reference) = input.expected_envelope_ref {
        validate_ingress_ref(reference, "node control live workflow bundle ack import expected envelope ref")?;
    }
    if let Some(reference) = input.expected_operation_ref {
        validate_ingress_ref(reference, "node control live workflow bundle ack import expected operation ref")?;
    }
    if let Some(reference) = input.expected_request_ref {
        validate_ingress_ref(reference, "node control live workflow bundle ack import expected request ref")?;
    }
    Ok(())
}

fn live_workflow_bundle_ack_import_diagnostics(
    input: &ControlLiveWorkflowBundleAckImportInput<'_>,
    ack: &ControlLiveWorkflowBundleAck,
) -> Result<Vec<String>> {
    let recomputed = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
        apply_receipt_value: &ack.apply_receipt_value,
        send_receipt_value: ack.send_receipt_value.as_ref(),
        ingress_receipt_value: ack.ingress_receipt_value.as_ref(),
        queue_receipt_value: ack.queue_receipt_value.as_ref(),
        control_receipt_value: ack.control_receipt_value.as_ref(),
        expected_envelope_ref: None,
        expected_operation_ref: None,
        expected_request_ref: None,
    })?;
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
        let ingress = parse_control_ingress_receipt(ingress_value)?;
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
    state_root: &Path,
    ack: &ControlLiveWorkflowBundleAck,
) -> Result<Vec<String>> {
    let mut imported_refs = Vec::with_capacity(8);
    imported_refs.push(import_artifact(state_root, &ack.apply_receipt_value)?);
    if let Some(value) = ack.send_receipt_value.as_ref() {
        imported_refs.push(import_artifact(state_root, value)?);
    }
    if let Some(value) = ack.ingress_receipt_value.as_ref() {
        imported_refs.push(import_artifact(state_root, value)?);
    }
    if let Some(value) = ack.queue_receipt_value.as_ref() {
        imported_refs.push(import_artifact(state_root, value)?);
    }
    if let Some(value) = ack.control_receipt_value.as_ref() {
        imported_refs.push(import_artifact(state_root, value)?);
    }
    imported_refs.push(import_artifact(state_root, &ack.reconcile_receipt_value)?);
    imported_refs.push(import_artifact(state_root, &ack.ack_value)?);
    Ok(imported_refs)
}

pub fn parse_control_live_workflow_bundle_apply_receipt(
    value: &IoValue,
) -> Result<ControlLiveWorkflowBundleApplyReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-apply-receipt-v1", Some(15))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-apply-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA,
        "node control live workflow bundle apply receipt",
    )?;
    let gate_receipt_ref = record_optional_ref_string(&fields[4], "gate-receipt")?;
    let import_receipt_ref = record_optional_ref_string(&fields[6], "import-receipt")?;
    let envelope_ref = record_optional_ref_string(&fields[9], "envelope")?;
    let operation_ref = record_optional_ref_string(&fields[10], "operation")?;
    let send_receipt_ref = record_optional_ref_string(&fields[11], "send-receipt")?;
    let _expected = record_value(&fields[12], "expected")?;
    let _checks = record_sequence_len(&fields[14], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlLiveWorkflowBundleApplyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        bundle_ref: record_ref_string(&fields[3], "bundle")?,
        gate_receipt_ref,
        recomputed_verify_receipt_ref: record_ref_string(&fields[5], "recomputed-verify")?,
        import_receipt_ref,
        imported_refs: record_ref_strings(&fields[7], "imported")?,
        mode: record_string(&fields[8], "mode")?,
        envelope_ref,
        operation_ref,
        send_receipt_ref,
        diagnostics: record_strings(&fields[13], "diagnostics")?,
    })
}

pub fn parse_control_live_workflow_bundle_reconcile_receipt(
    value: &IoValue,
) -> Result<ControlLiveWorkflowBundleReconcileReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-reconcile-receipt-v1", Some(13))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-reconcile-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
        "node control live workflow bundle reconcile receipt",
    )?;
    let send_receipt_ref = record_optional_ref_string(&fields[4], "send-receipt")?;
    let ingress_receipt_ref = record_optional_ref_string(&fields[5], "ingress-receipt")?;
    let queue_receipt_ref = record_optional_ref_string(&fields[6], "queue-receipt")?;
    let control_receipt_ref = record_optional_ref_string(&fields[7], "control-receipt")?;
    let envelope_ref = record_optional_ref_string(&fields[8], "envelope")?;
    let operation_ref = record_optional_ref_string(&fields[9], "operation")?;
    let request_ref = record_optional_ref_string(&fields[10], "request")?;
    let _checks = record_sequence_len(&fields[12], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlLiveWorkflowBundleReconcileReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        apply_receipt_ref: record_ref_string(&fields[2], "apply-receipt")?,
        bundle_ref: record_ref_string(&fields[3], "bundle")?,
        send_receipt_ref,
        ingress_receipt_ref,
        queue_receipt_ref,
        control_receipt_ref,
        envelope_ref,
        operation_ref,
        request_ref,
        diagnostics: record_strings(&fields[11], "diagnostics")?,
    })
}

pub fn parse_control_ingress_receipt(value: &IoValue) -> Result<ControlIngressReceipt> {
    let fields = value
        .collect_simple_record("node-control-ingress-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-ingress-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA,
        "node control ingress receipt",
    )?;
    let idempotency_receipt_ref = record_optional_ref_string(&fields[11], "idempotency")?;
    let queue_receipt_ref = record_optional_ref_string(&fields[12], "queue")?;
    let _checks = record_sequence_len(&fields[14], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlIngressReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        phase: record_string(&fields[2], "phase")?,
        transport: record_string(&fields[3], "transport")?,
        topic: record_string(&fields[4], "topic")?,
        from_peer: record_string(&fields[5], "from-peer")?,
        to_node: record_string(&fields[6], "to-node")?,
        sequence: record_u64_string(&fields[7], "sequence")?,
        envelope_ref: record_ref_string(&fields[8], "envelope")?,
        operation_ref: record_ref_string(&fields[9], "operation")?,
        request_ref: record_ref_string(&fields[10], "request")?,
        idempotency_receipt_ref,
        queue_receipt_ref,
        diagnostics: record_strings(&fields[13], "diagnostics")?,
    })
}

pub fn parse_control_queue_receipt(value: &IoValue) -> Result<ControlQueueReceipt> {
    let fields = value
        .collect_simple_record("node-control-queue-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-queue-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_CONTROL_QUEUE_RECEIPT_SCHEMA, "node control queue receipt")?;
    let _checks = record_sequence_len(&fields[8], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlQueueReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        phase: record_string(&fields[2], "phase")?,
        operation: record_string(&fields[3], "operation")?,
        request_ref: record_ref_string(&fields[4], "request")?,
        location_ref: record_ref_string(&fields[6], "location")?,
        diagnostics: record_strings(&fields[7], "diagnostics")?,
    })
}
