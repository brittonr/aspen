//! Parsers for apply, reconcile, ingress, queue, and send receipts.

pub fn parse_control_live_workflow_bundle_apply_receipt(
    value: &preserves::IOValue,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleApplyReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-apply-receipt-v1", Some(17))
        .or_else(|| value.collect_simple_record("node-control-live-workflow-bundle-apply-receipt-v1", Some(15)))
        .ok_or_else(|| {
            crate::error::Failure::invalid_harness("expected <node-control-live-workflow-bundle-apply-receipt-v1 ...>")
        })?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA,
        "node control live workflow bundle apply receipt",
    )?;
    let gate_receipt_ref = super::fields::record_optional_ref_string(&fields[4], "gate-receipt")?;
    let import_receipt_ref = super::fields::record_optional_ref_string(&fields[6], "import-receipt")?;
    let envelope_ref = super::fields::record_optional_ref_string(&fields[9], "envelope")?;
    let operation_ref = super::fields::record_optional_ref_string(&fields[10], "operation")?;
    let send_receipt_ref = super::fields::record_optional_ref_string(&fields[11], "send-receipt")?;
    let _expected = super::fields::record_value(&fields[12], "expected")?;
    let _checks = super::fields::record_sequence_len(&fields[14], "checks")?;
    let decision = super::fields::record_string(&fields[1], "decision")?;
    super::guards::validate_decision(&decision)?;
    Ok(super::artifacts::ControlLiveWorkflowBundleApplyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        bundle_ref: super::fields::record_ref_string(&fields[3], "bundle")?,
        gate_receipt_ref,
        recomputed_verify_receipt_ref: super::fields::record_ref_string(&fields[5], "recomputed-verify")?,
        import_receipt_ref,
        imported_refs: super::fields::record_ref_strings(&fields[7], "imported")?,
        mode: super::fields::record_string(&fields[8], "mode")?,
        envelope_ref,
        operation_ref,
        send_receipt_ref,
        diagnostics: super::fields::record_strings(&fields[13], "diagnostics")?,
    })
}

pub fn parse_control_live_workflow_bundle_reconcile_receipt(
    value: &preserves::IOValue,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleReconcileReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-reconcile-receipt-v1", Some(13))
        .ok_or_else(|| {
            crate::error::Failure::invalid_harness(
                "expected <node-control-live-workflow-bundle-reconcile-receipt-v1 ...>",
            )
        })?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
        "node control live workflow bundle reconcile receipt",
    )?;
    let send_receipt_ref = super::fields::record_optional_ref_string(&fields[4], "send-receipt")?;
    let ingress_receipt_ref = super::fields::record_optional_ref_string(&fields[5], "ingress-receipt")?;
    let queue_receipt_ref = super::fields::record_optional_ref_string(&fields[6], "queue-receipt")?;
    let control_receipt_ref = super::fields::record_optional_ref_string(&fields[7], "control-receipt")?;
    let envelope_ref = super::fields::record_optional_ref_string(&fields[8], "envelope")?;
    let operation_ref = super::fields::record_optional_ref_string(&fields[9], "operation")?;
    let request_ref = super::fields::record_optional_ref_string(&fields[10], "request")?;
    let _checks = super::fields::record_sequence_len(&fields[12], "checks")?;
    let decision = super::fields::record_string(&fields[1], "decision")?;
    super::guards::validate_decision(&decision)?;
    Ok(super::artifacts::ControlLiveWorkflowBundleReconcileReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        apply_receipt_ref: super::fields::record_ref_string(&fields[2], "apply-receipt")?,
        bundle_ref: super::fields::record_ref_string(&fields[3], "bundle")?,
        send_receipt_ref,
        ingress_receipt_ref,
        queue_receipt_ref,
        control_receipt_ref,
        envelope_ref,
        operation_ref,
        request_ref,
        diagnostics: super::fields::record_strings(&fields[11], "diagnostics")?,
    })
}

pub fn parse_control_ingress_receipt(
    value: &preserves::IOValue,
) -> crate::error::Result<super::transits::ControlIngressReceipt> {
    let fields = value
        .collect_simple_record("node-control-ingress-receipt-v1", Some(15))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-control-ingress-receipt-v1 ...>"))?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA,
        "node control ingress receipt",
    )?;
    let idempotency_receipt_ref = super::fields::record_optional_ref_string(&fields[11], "idempotency")?;
    let queue_receipt_ref = super::fields::record_optional_ref_string(&fields[12], "queue")?;
    let _checks = super::fields::record_sequence_len(&fields[14], "checks")?;
    let decision = super::fields::record_string(&fields[1], "decision")?;
    super::guards::validate_decision(&decision)?;
    Ok(super::transits::ControlIngressReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        phase: super::fields::record_string(&fields[2], "phase")?,
        transport: super::fields::record_string(&fields[3], "transport")?,
        topic: super::fields::record_string(&fields[4], "topic")?,
        from_peer: super::fields::record_string(&fields[5], "from-peer")?,
        to_node: super::fields::record_string(&fields[6], "to-node")?,
        sequence: super::fields::record_u64_string(&fields[7], "sequence")?,
        envelope_ref: super::fields::record_ref_string(&fields[8], "envelope")?,
        operation_ref: super::fields::record_ref_string(&fields[9], "operation")?,
        request_ref: super::fields::record_ref_string(&fields[10], "request")?,
        idempotency_receipt_ref,
        queue_receipt_ref,
        diagnostics: super::fields::record_strings(&fields[13], "diagnostics")?,
    })
}

pub fn parse_control_queue_receipt(
    value: &preserves::IOValue,
) -> crate::error::Result<super::transits::ControlQueueReceipt> {
    let fields = value
        .collect_simple_record("node-control-queue-receipt-v1", Some(9))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-control-queue-receipt-v1 ...>"))?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_QUEUE_RECEIPT_SCHEMA,
        "node control queue receipt",
    )?;
    let _checks = super::fields::record_sequence_len(&fields[8], "checks")?;
    let decision = super::fields::record_string(&fields[1], "decision")?;
    super::guards::validate_decision(&decision)?;
    Ok(super::transits::ControlQueueReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        phase: super::fields::record_string(&fields[2], "phase")?,
        operation: super::fields::record_string(&fields[3], "operation")?,
        request_ref: super::fields::record_ref_string(&fields[4], "request")?,
        location_ref: super::fields::record_ref_string(&fields[6], "location")?,
        diagnostics: super::fields::record_strings(&fields[7], "diagnostics")?,
    })
}

pub fn parse_control_live_send_receipt(
    value: &preserves::IOValue,
) -> crate::error::Result<super::transits::ControlLiveSendReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-send-receipt-v1", Some(15))
        .or_else(|| value.collect_simple_record("node-control-live-send-receipt-v1", Some(13)))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-control-live-send-receipt-v1 ...>"))?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA,
        "node control live send receipt",
    )?;
    let transport_receipt_ref = super::fields::record_optional_string(&fields[10], "transport-receipt")?;
    if let Some(reference) = transport_receipt_ref.as_ref() {
        super::guards::validate_ingress_ref(reference, "node control live send transport receipt ref")?;
    }
    Ok(super::transits::ControlLiveSendReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: super::fields::record_string(&fields[1], "decision")?,
        topic: super::fields::record_string(&fields[3], "topic")?,
        from_peer: super::fields::record_string(&fields[4], "from-peer")?,
        to_node: super::fields::record_string(&fields[5], "to-node")?,
        receiver_ticket_ref: super::fields::record_ref_string(&fields[6], "receiver-ticket")?,
        receiver_endpoint_id: super::fields::record_string(&fields[7], "receiver-endpoint")?,
        receiver_address_refs: super::fields::record_strings(&fields[8], "receiver-addresses")?,
        envelope_ref: super::fields::record_ref_string(&fields[9], "envelope")?,
        transport_receipt_ref,
        diagnostics: super::fields::record_strings(&fields[11], "diagnostics")?,
        value: value.clone(),
    })
}
