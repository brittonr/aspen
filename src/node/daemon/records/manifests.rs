//! Parsers for bundle verify and gate receipts and bundle manifests.

pub fn parse_control_live_workflow_bundle_verify_receipt(
    value: &preserves::IOValue,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleVerifyReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-verify-receipt-v1", Some(10))
        .ok_or_else(|| {
            crate::error::Failure::invalid_harness("expected <node-control-live-workflow-bundle-verify-receipt-v1 ...>")
        })?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA,
        "node control live workflow bundle verify receipt",
    )?;
    let ticket_ref = super::fields::record_optional_string(&fields[3], "ticket")?;
    let peer_admission_ref = super::fields::record_optional_string(&fields[4], "peer-admission")?;
    let authority_grant_ref = super::fields::record_optional_string(&fields[5], "authority-grant")?;
    for (reference, label) in [
        (ticket_ref.as_deref(), "node control live workflow bundle verify ticket ref"),
        (peer_admission_ref.as_deref(), "node control live workflow bundle verify peer admission ref"),
        (authority_grant_ref.as_deref(), "node control live workflow bundle verify authority grant ref"),
    ] {
        if let Some(reference) = reference {
            super::guards::validate_ingress_ref(reference, label)?;
        }
    }
    let _expected = super::fields::record_value(&fields[7], "expected")?;
    let _checks = super::fields::record_sequence_len(&fields[9], "checks")?;
    let decision = super::fields::record_string(&fields[1], "decision")?;
    super::guards::validate_decision(&decision)?;
    Ok(super::artifacts::ControlLiveWorkflowBundleVerifyReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        bundle_ref: super::fields::record_ref_string(&fields[2], "bundle")?,
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs: super::fields::record_ref_strings(&fields[6], "receipts")?,
        diagnostics: super::fields::record_strings(&fields[8], "diagnostics")?,
    })
}

pub fn parse_control_live_workflow_bundle_gate_receipt(
    value: &preserves::IOValue,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleGateReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-gate-receipt-v1", Some(12))
        .ok_or_else(|| {
            crate::error::Failure::invalid_harness("expected <node-control-live-workflow-bundle-gate-receipt-v1 ...>")
        })?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA,
        "node control live workflow bundle gate receipt",
    )?;
    let verify_receipt_ref = super::fields::record_optional_string(&fields[3], "verify-receipt")?;
    let ticket_ref = super::fields::record_optional_string(&fields[5], "ticket")?;
    let peer_admission_ref = super::fields::record_optional_string(&fields[6], "peer-admission")?;
    let authority_grant_ref = super::fields::record_optional_string(&fields[7], "authority-grant")?;
    for (reference, label) in [
        (verify_receipt_ref.as_deref(), "node control live workflow bundle gate verify receipt ref"),
        (ticket_ref.as_deref(), "node control live workflow bundle gate ticket ref"),
        (peer_admission_ref.as_deref(), "node control live workflow bundle gate peer admission ref"),
        (authority_grant_ref.as_deref(), "node control live workflow bundle gate authority grant ref"),
    ] {
        if let Some(reference) = reference {
            super::guards::validate_ingress_ref(reference, label)?;
        }
    }
    let _expected = super::fields::record_value(&fields[9], "expected")?;
    let _checks = super::fields::record_sequence_len(&fields[11], "checks")?;
    let decision = super::fields::record_string(&fields[1], "decision")?;
    super::guards::validate_decision(&decision)?;
    Ok(super::artifacts::ControlLiveWorkflowBundleGateReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        bundle_ref: super::fields::record_ref_string(&fields[2], "bundle")?,
        verify_receipt_ref,
        recomputed_verify_receipt_ref: super::fields::record_ref_string(&fields[4], "recomputed-verify")?,
        ticket_ref,
        peer_admission_ref,
        authority_grant_ref,
        receipt_refs: super::fields::record_ref_strings(&fields[8], "receipts")?,
        diagnostics: super::fields::record_strings(&fields[10], "diagnostics")?,
    })
}

pub fn parse_control_live_workflow_bundle(
    value: &preserves::IOValue,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundle> {
    let fields = value
        .collect_simple_record("node-control-live-workflow-bundle-v1", Some(10))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-control-live-workflow-bundle-v1 ...>"))?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA,
        "node control live workflow bundle",
    )?;
    let ticket_value = super::fields::record_value(&fields[1], "ticket")?;
    let peer_admission_value = super::fields::record_value(&fields[2], "peer-admission")?;
    let authority_grant_value = super::fields::record_value(&fields[3], "authority-grant")?;
    let receipt_values = super::fields::record_values(&fields[4], "receipts")?;
    let ticket_ref = super::fields::record_ref_string(&fields[5], "ticket-ref")?;
    let peer_admission_ref = super::fields::record_ref_string(&fields[6], "peer-admission-ref")?;
    let authority_grant_ref = super::fields::record_ref_string(&fields[7], "authority-grant-ref")?;
    let receipt_refs = super::fields::record_ref_strings(&fields[8], "receipt-refs")?;
    let parsed_ticket = super::issuance::parse_control_live_ticket(&ticket_value)?;
    let parsed_admission = super::peering::parse_control_live_peer_admission(&peer_admission_value)?;
    let parsed_authority = super::issuance::parse_control_authority_grant(&authority_grant_value)?;
    if parsed_ticket.ticket_ref != ticket_ref {
        return Err(crate::error::Failure::invalid_harness("node control live workflow bundle ticket ref mismatch"));
    }
    if parsed_admission.admission_ref != peer_admission_ref {
        return Err(crate::error::Failure::invalid_harness(
            "node control live workflow bundle peer admission ref mismatch",
        ));
    }
    if parsed_authority.grant_ref != authority_grant_ref {
        return Err(crate::error::Failure::invalid_harness(
            "node control live workflow bundle authority grant ref mismatch",
        ));
    }
    let parsed_receipt_refs = super::exporting::live_workflow_bundle_receipt_refs_from_values(&receipt_values)?;
    if parsed_receipt_refs != receipt_refs {
        return Err(crate::error::Failure::invalid_harness("node control live workflow bundle receipt refs mismatch"));
    }
    Ok(super::artifacts::ControlLiveWorkflowBundle {
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
