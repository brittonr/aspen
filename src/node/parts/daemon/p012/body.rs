
pub fn parse_control_live_workflow_bundle_verify_receipt(
    value: &IoValue,
) -> Result<ControlLiveWorkflowBundleVerifyReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-verify-receipt-v1", Some(10))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-verify-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA,
        "node control live workflow bundle verify receipt",
    )?;
    let ticket_ref = record_optional_string(&fields[3], "ticket")?;
    let peer_admission_ref = record_optional_string(&fields[4], "peer-admission")?;
    let authority_grant_ref = record_optional_string(&fields[5], "authority-grant")?;
    for (reference, label) in [
        (ticket_ref.as_deref(), "node control live workflow bundle verify ticket ref"),
        (peer_admission_ref.as_deref(), "node control live workflow bundle verify peer admission ref"),
        (authority_grant_ref.as_deref(), "node control live workflow bundle verify authority grant ref"),
    ] {
        if let Some(reference) = reference {
            validate_ingress_ref(reference, label)?;
        }
    }
    let _expected = record_value(&fields[7], "expected")?;
    let _checks = record_sequence_len(&fields[9], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlLiveWorkflowBundleVerifyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        bundle_ref: record_ref_string(&fields[2], "bundle")?,
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs: record_ref_strings(&fields[6], "receipts")?,
        diagnostics: record_strings(&fields[8], "diagnostics")?,
    })
}

pub fn parse_control_live_workflow_bundle_gate_receipt(
    value: &IoValue,
) -> Result<ControlLiveWorkflowBundleGateReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-gate-receipt-v1", Some(12))
        .ok_or_else(|| {
            MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-gate-receipt-v1 ...>")
        })?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA,
        "node control live workflow bundle gate receipt",
    )?;
    let verify_receipt_ref = record_optional_string(&fields[3], "verify-receipt")?;
    let ticket_ref = record_optional_string(&fields[5], "ticket")?;
    let peer_admission_ref = record_optional_string(&fields[6], "peer-admission")?;
    let authority_grant_ref = record_optional_string(&fields[7], "authority-grant")?;
    for (reference, label) in [
        (verify_receipt_ref.as_deref(), "node control live workflow bundle gate verify receipt ref"),
        (ticket_ref.as_deref(), "node control live workflow bundle gate ticket ref"),
        (peer_admission_ref.as_deref(), "node control live workflow bundle gate peer admission ref"),
        (authority_grant_ref.as_deref(), "node control live workflow bundle gate authority grant ref"),
    ] {
        if let Some(reference) = reference {
            validate_ingress_ref(reference, label)?;
        }
    }
    let _expected = record_value(&fields[9], "expected")?;
    let _checks = record_sequence_len(&fields[11], "checks")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    Ok(ControlLiveWorkflowBundleGateReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        bundle_ref: record_ref_string(&fields[2], "bundle")?,
        verify_receipt_ref,
        recomputed_verify_receipt_ref: record_ref_string(&fields[4], "recomputed-verify")?,
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs: record_ref_strings(&fields[8], "receipts")?,
        diagnostics: record_strings(&fields[10], "diagnostics")?,
    })
}

pub fn parse_control_live_workflow_bundle(value: &IoValue) -> Result<ControlLiveWorkflowBundle> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA,
        "node control live workflow bundle",
    )?;
    let ticket_value = record_value(&fields[1], "ticket")?;
    let peer_admission_value = record_value(&fields[2], "peer-admission")?;
    let authority_grant_value = record_value(&fields[3], "authority-grant")?;
    let receipt_values = record_values(&fields[4], "receipts")?;
    let ticket_ref = record_ref_string(&fields[5], "ticket-ref")?;
    let peer_admission_ref = record_ref_string(&fields[6], "peer-admission-ref")?;
    let authority_grant_ref = record_ref_string(&fields[7], "authority-grant-ref")?;
    let receipt_refs = record_ref_strings(&fields[8], "receipt-refs")?;
    let parsed_ticket = parse_control_live_ticket(&ticket_value)?;
    let parsed_admission = parse_control_live_peer_admission(&peer_admission_value)?;
    let parsed_authority = parse_control_authority_grant(&authority_grant_value)?;
    if parsed_ticket.ticket_ref != ticket_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ticket ref mismatch"));
    }
    if parsed_admission.admission_ref != peer_admission_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle peer admission ref mismatch"));
    }
    if parsed_authority.grant_ref != authority_grant_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle authority grant ref mismatch"));
    }
    let parsed_receipt_refs = live_workflow_bundle_receipt_refs_from_values(&receipt_values)?;
    if parsed_receipt_refs != receipt_refs {
        return Err(MoltenError::invalid_harness("node control live workflow bundle receipt refs mismatch"));
    }
    Ok(ControlLiveWorkflowBundle {
        bundle_ref: crate::preserves_rail::canonical_hash(value)?,
        bundle_value: value.clone(),
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs,
        ticket_value,
        peer_admission_value,
        authority_grant_value,
        receipt_values,
    })
}

struct AckParts {
    apply_receipt_ref: String,
    send_receipt_ref: Option<String>,
    ingress_receipt_ref: Option<String>,
    queue_receipt_ref: Option<String>,
    control_receipt_ref: Option<String>,
    reconcile_receipt_ref: String,
    bundle_ref: String,
    envelope_ref: Option<String>,
    operation_ref: Option<String>,
    request_ref: Option<String>,
    receiver_decision: String,
    receiver_diagnostics: Vec<String>,
    diagnostics: Vec<String>,
    apply_receipt_value: IoValue,
    send_receipt_value: Option<IoValue>,
    ingress_receipt_value: Option<IoValue>,
    queue_receipt_value: Option<IoValue>,
    control_receipt_value: Option<IoValue>,
    reconcile_receipt_value: IoValue,
}

impl AckParts {
    fn into_ack(self, value: &IoValue) -> Result<ControlLiveWorkflowBundleAck> {
        Ok(ControlLiveWorkflowBundleAck {
            ack_ref: crate::preserves_rail::canonical_hash(value)?,
            ack_value: value.clone(),
            apply_receipt_ref: self.apply_receipt_ref,
            send_receipt_ref: self.send_receipt_ref,
            ingress_receipt_ref: self.ingress_receipt_ref,
            queue_receipt_ref: self.queue_receipt_ref,
            control_receipt_ref: self.control_receipt_ref,
            reconcile_receipt_ref: self.reconcile_receipt_ref,
            bundle_ref: self.bundle_ref,
            envelope_ref: self.envelope_ref,
            operation_ref: self.operation_ref,
            request_ref: self.request_ref,
            receiver_decision: self.receiver_decision,
            receiver_diagnostics: self.receiver_diagnostics,
            diagnostics: self.diagnostics,
            apply_receipt_value: self.apply_receipt_value,
            send_receipt_value: self.send_receipt_value,
            ingress_receipt_value: self.ingress_receipt_value,
            queue_receipt_value: self.queue_receipt_value,
            control_receipt_value: self.control_receipt_value,
            reconcile_receipt_value: self.reconcile_receipt_value,
        })
    }
}

fn validate_ack_members(parts: &AckParts) -> Result<()> {
    let apply = parse_control_live_workflow_bundle_apply_receipt(&parts.apply_receipt_value)?;
    let reconcile = parse_control_live_workflow_bundle_reconcile_receipt(&parts.reconcile_receipt_value)?;
    if let Some(value) = parts.send_receipt_value.as_ref() {
        parse_control_live_send_receipt(value)?;
    }
    if let Some(value) = parts.ingress_receipt_value.as_ref() {
        parse_control_ingress_receipt(value)?;
    }
    if let Some(value) = parts.queue_receipt_value.as_ref() {
        parse_control_queue_receipt(value)?;
    }
    if let Some(value) = parts.control_receipt_value.as_ref() {
        crate::node_runtime::parse_control_receipt(value)?;
    }
    validate_member_ref(&apply.receipt_ref, &parts.apply_receipt_ref, "ack apply receipt")?;
    validate_member_ref(&reconcile.receipt_ref, &parts.reconcile_receipt_ref, "ack reconcile receipt")?;
    validate_optional_member_ref(
        parts.send_receipt_value.as_ref(),
        parts.send_receipt_ref.as_deref(),
        "ack send receipt",
    )?;
    validate_optional_member_ref(
        parts.ingress_receipt_value.as_ref(),
        parts.ingress_receipt_ref.as_deref(),
        "ack ingress receipt",
    )?;
    validate_optional_member_ref(
        parts.queue_receipt_value.as_ref(),
        parts.queue_receipt_ref.as_deref(),
        "ack queue receipt",
    )?;
    validate_optional_member_ref(
        parts.control_receipt_value.as_ref(),
        parts.control_receipt_ref.as_deref(),
        "ack control receipt",
    )?;
    validate_ack_reconcile(parts, &reconcile)
}

fn validate_ack_reconcile(parts: &AckParts, reconcile: &ControlLiveWorkflowBundleReconcileReceipt) -> Result<()> {
    if reconcile.apply_receipt_ref != parts.apply_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack apply ref mismatch"));
    }
    if reconcile.bundle_ref != parts.bundle_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack bundle ref mismatch"));
    }
    if reconcile.send_receipt_ref != parts.send_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack send ref mismatch"));
    }
    if reconcile.ingress_receipt_ref != parts.ingress_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack ingress ref mismatch"));
    }
    if reconcile.queue_receipt_ref != parts.queue_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack queue ref mismatch"));
    }
    if reconcile.control_receipt_ref != parts.control_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack control ref mismatch"));
    }
    if reconcile.envelope_ref != parts.envelope_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack envelope ref mismatch"));
    }
    if reconcile.operation_ref != parts.operation_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack operation ref mismatch"));
    }
    if reconcile.request_ref != parts.request_ref {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack request ref mismatch"));
    }
    if reconcile.decision != parts.receiver_decision {
        return Err(MoltenError::invalid_harness("node control live workflow bundle ack receiver decision mismatch"));
    }
    if reconcile.diagnostics != parts.receiver_diagnostics {
        return Err(MoltenError::invalid_harness(
            "node control live workflow bundle ack receiver diagnostics mismatch",
        ));
    }
    Ok(())
}

pub fn parse_control_live_workflow_bundle_ack(value: &IoValue) -> Result<ControlLiveWorkflowBundleAck> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-ack-v1", Some(22))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-workflow-bundle-ack-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA,
        "node control live workflow bundle ack",
    )?;
    let receiver_decision = record_string(&fields[17], "receiver-decision")?;
    validate_decision(&receiver_decision)?;
    let _checks = record_sequence_len(&fields[20], "checks")?;
    let _member_refs = record_sequence_len(&fields[21], "member-refs")?;
    let parts = AckParts {
        apply_receipt_value: record_value(&fields[1], "apply-receipt")?,
        send_receipt_value: record_optional_value(&fields[2], "send-receipt")?,
        ingress_receipt_value: record_optional_value(&fields[3], "ingress-receipt")?,
        queue_receipt_value: record_optional_value(&fields[4], "queue-receipt")?,
        control_receipt_value: record_optional_value(&fields[5], "control-receipt")?,
        reconcile_receipt_value: record_value(&fields[6], "reconcile-receipt")?,
        apply_receipt_ref: record_ref_string(&fields[7], "apply-ref")?,
        send_receipt_ref: record_optional_ref_string(&fields[8], "send-ref")?,
        ingress_receipt_ref: record_optional_ref_string(&fields[9], "ingress-ref")?,
        queue_receipt_ref: record_optional_ref_string(&fields[10], "queue-ref")?,
        control_receipt_ref: record_optional_ref_string(&fields[11], "control-ref")?,
        reconcile_receipt_ref: record_ref_string(&fields[12], "reconcile-ref")?,
        bundle_ref: record_ref_string(&fields[13], "bundle")?,
        envelope_ref: record_optional_ref_string(&fields[14], "envelope")?,
        operation_ref: record_optional_ref_string(&fields[15], "operation")?,
        request_ref: record_optional_ref_string(&fields[16], "request")?,
        receiver_decision,
        receiver_diagnostics: record_strings(&fields[18], "receiver-diagnostics")?,
        diagnostics: record_strings(&fields[19], "diagnostics")?,
    };
    validate_ack_members(&parts)?;
    parts.into_ack(value)
}
