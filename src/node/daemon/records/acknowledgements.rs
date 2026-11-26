//! Bundle acknowledgement parsing and member validation.

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
    apply_receipt_value: preserves::IOValue,
    send_receipt_value: Option<preserves::IOValue>,
    ingress_receipt_value: Option<preserves::IOValue>,
    queue_receipt_value: Option<preserves::IOValue>,
    control_receipt_value: Option<preserves::IOValue>,
    reconcile_receipt_value: preserves::IOValue,
}

impl AckParts {
    fn into_ack(
        self,
        value: &preserves::IOValue,
    ) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleAck> {
        Ok(super::artifacts::ControlLiveWorkflowBundleAck {
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

fn validate_ack_members(parts: &AckParts) -> crate::error::Result<()> {
    let apply = super::receipts::parse_control_live_workflow_bundle_apply_receipt(&parts.apply_receipt_value)?;
    let reconcile =
        super::receipts::parse_control_live_workflow_bundle_reconcile_receipt(&parts.reconcile_receipt_value)?;
    if let Some(value) = parts.send_receipt_value.as_ref() {
        super::receipts::parse_control_live_send_receipt(value)?;
    }
    if let Some(value) = parts.ingress_receipt_value.as_ref() {
        super::receipts::parse_control_ingress_receipt(value)?;
    }
    if let Some(value) = parts.queue_receipt_value.as_ref() {
        super::receipts::parse_control_queue_receipt(value)?;
    }
    if let Some(value) = parts.control_receipt_value.as_ref() {
        crate::node_runtime::parse_control_receipt(value)?;
    }
    super::guards::validate_member_ref(&apply.receipt_ref, &parts.apply_receipt_ref, "ack apply receipt")?;
    super::guards::validate_member_ref(&reconcile.receipt_ref, &parts.reconcile_receipt_ref, "ack reconcile receipt")?;
    super::guards::validate_optional_member_ref(
        parts.send_receipt_value.as_ref(),
        parts.send_receipt_ref.as_deref(),
        "ack send receipt",
    )?;
    super::guards::validate_optional_member_ref(
        parts.ingress_receipt_value.as_ref(),
        parts.ingress_receipt_ref.as_deref(),
        "ack ingress receipt",
    )?;
    super::guards::validate_optional_member_ref(
        parts.queue_receipt_value.as_ref(),
        parts.queue_receipt_ref.as_deref(),
        "ack queue receipt",
    )?;
    super::guards::validate_optional_member_ref(
        parts.control_receipt_value.as_ref(),
        parts.control_receipt_ref.as_deref(),
        "ack control receipt",
    )?;
    validate_ack_reconcile(parts, &reconcile)
}

fn validate_ack_reconcile(
    parts: &AckParts,
    reconcile: &super::artifacts::ControlLiveWorkflowBundleReconcileReceipt,
) -> crate::error::Result<()> {
    if reconcile.apply_receipt_ref != parts.apply_receipt_ref {
        return Err(crate::error::Failure::invalid_harness("node control live workflow bundle ack apply ref mismatch"));
    }
    if reconcile.bundle_ref != parts.bundle_ref {
        return Err(crate::error::Failure::invalid_harness(
            "node control live workflow bundle ack bundle ref mismatch",
        ));
    }
    if reconcile.send_receipt_ref != parts.send_receipt_ref {
        return Err(crate::error::Failure::invalid_harness("node control live workflow bundle ack send ref mismatch"));
    }
    if reconcile.ingress_receipt_ref != parts.ingress_receipt_ref {
        return Err(crate::error::Failure::invalid_harness(
            "node control live workflow bundle ack ingress ref mismatch",
        ));
    }
    if reconcile.queue_receipt_ref != parts.queue_receipt_ref {
        return Err(crate::error::Failure::invalid_harness("node control live workflow bundle ack queue ref mismatch"));
    }
    if reconcile.control_receipt_ref != parts.control_receipt_ref {
        return Err(crate::error::Failure::invalid_harness(
            "node control live workflow bundle ack control ref mismatch",
        ));
    }
    if reconcile.envelope_ref != parts.envelope_ref {
        return Err(crate::error::Failure::invalid_harness(
            "node control live workflow bundle ack envelope ref mismatch",
        ));
    }
    if reconcile.operation_ref != parts.operation_ref {
        return Err(crate::error::Failure::invalid_harness(
            "node control live workflow bundle ack operation ref mismatch",
        ));
    }
    if reconcile.request_ref != parts.request_ref {
        return Err(crate::error::Failure::invalid_harness(
            "node control live workflow bundle ack request ref mismatch",
        ));
    }
    if reconcile.decision != parts.receiver_decision {
        return Err(crate::error::Failure::invalid_harness(
            "node control live workflow bundle ack receiver decision mismatch",
        ));
    }
    if reconcile.diagnostics != parts.receiver_diagnostics {
        return Err(crate::error::Failure::invalid_harness(
            "node control live workflow bundle ack receiver diagnostics mismatch",
        ));
    }
    Ok(())
}

pub fn parse_control_live_workflow_bundle_ack(
    value: &preserves::IOValue,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleAck> {
    let fields =
        value.collect_simple_record("node-control-live-workflow-bundle-ack-v1", Some(22)).ok_or_else(|| {
            crate::error::Failure::invalid_harness("expected <node-control-live-workflow-bundle-ack-v1 ...>")
        })?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_ACK_SCHEMA,
        "node control live workflow bundle ack",
    )?;
    let receiver_decision = super::fields::record_string(&fields[17], "receiver-decision")?;
    super::guards::validate_decision(&receiver_decision)?;
    let _checks = super::fields::record_sequence_len(&fields[20], "checks")?;
    let _member_refs = super::fields::record_sequence_len(&fields[21], "member-refs")?;
    let parts = AckParts {
        apply_receipt_value: super::fields::record_value(&fields[1], "apply-receipt")?,
        send_receipt_value: super::fields::record_optional_value(&fields[2], "send-receipt")?,
        ingress_receipt_value: super::fields::record_optional_value(&fields[3], "ingress-receipt")?,
        queue_receipt_value: super::fields::record_optional_value(&fields[4], "queue-receipt")?,
        control_receipt_value: super::fields::record_optional_value(&fields[5], "control-receipt")?,
        reconcile_receipt_value: super::fields::record_value(&fields[6], "reconcile-receipt")?,
        apply_receipt_ref: super::fields::record_ref_string(&fields[7], "apply-ref")?,
        send_receipt_ref: super::fields::record_optional_ref_string(&fields[8], "send-ref")?,
        ingress_receipt_ref: super::fields::record_optional_ref_string(&fields[9], "ingress-ref")?,
        queue_receipt_ref: super::fields::record_optional_ref_string(&fields[10], "queue-ref")?,
        control_receipt_ref: super::fields::record_optional_ref_string(&fields[11], "control-ref")?,
        reconcile_receipt_ref: super::fields::record_ref_string(&fields[12], "reconcile-ref")?,
        bundle_ref: super::fields::record_ref_string(&fields[13], "bundle")?,
        envelope_ref: super::fields::record_optional_ref_string(&fields[14], "envelope")?,
        operation_ref: super::fields::record_optional_ref_string(&fields[15], "operation")?,
        request_ref: super::fields::record_optional_ref_string(&fields[16], "request")?,
        receiver_decision,
        receiver_diagnostics: super::fields::record_strings(&fields[18], "receiver-diagnostics")?,
        diagnostics: super::fields::record_strings(&fields[19], "diagnostics")?,
    };
    validate_ack_members(&parts)?;
    parts.into_ack(value)
}
