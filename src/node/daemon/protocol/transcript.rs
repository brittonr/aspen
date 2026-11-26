//! Protocol evidence collection and the protocol manifest.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LiveWorkflowProtocolEvidence {
    pub(super) session_id: String,
    pub(super) authority_ref: String,
    pub(super) resource_ref: String,
    pub(super) gate_receipt_ref: String,
    pub(super) apply_receipt_ref: String,
    pub(super) reconcile_receipt_ref: String,
    pub(super) ack_ref: String,
}

pub(super) fn validate_live_workflow_protocol_gate_input(
    input: &super::bundling::ControlLiveWorkflowProtocolGateInput<'_>,
) -> crate::error::Result<()> {
    if let Some(reference) = input.expected_envelope_ref {
        super::guards::validate_ingress_ref(reference, "node control live workflow protocol expected envelope ref")?;
    }
    if let Some(reference) = input.expected_operation_ref {
        super::guards::validate_ingress_ref(reference, "node control live workflow protocol expected operation ref")?;
    }
    if let Some(reference) = input.expected_request_ref {
        super::guards::validate_ingress_ref(reference, "node control live workflow protocol expected request ref")?;
    }
    Ok(())
}

fn parsed_or_note<T>(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    label: &str,
    parse: impl FnOnce() -> crate::error::Result<T>,
) -> Option<T> {
    match parse() {
        Ok(parsed) => Some(parsed),
        Err(error) => {
            diagnostics.push_item(format!("{label} parse failed: {error}"));
            None
        }
    }
}

pub(super) fn note_receipt_decision(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    label: &str,
    receipt_ref: &str,
    decision: &str,
    notes: &[String],
) {
    if decision != "pass" {
        diagnostics.push_item(format!("{label} receipt {receipt_ref} decision {decision}"));
        diagnostics.extend_cloned_items(notes);
    }
}

pub(super) fn note_ref_mismatch(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    label: &str,
    observed: &str,
    expected: &str,
) {
    if observed != expected {
        diagnostics.push_item(format!("{label} {observed} does not match {expected}"));
    }
}

pub(super) fn note_optional_ref_mismatch(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    label: &str,
    observed: Option<&str>,
    expected: &str,
) {
    if observed != Some(expected) {
        diagnostics.push_item(format!("{label} {} does not match {expected}", observed.unwrap_or("none")));
    }
}

pub(super) fn note_expected_ref(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    label: &str,
    observed: Option<&str>,
    expected: Option<&str>,
) {
    if let Some(expected) = expected
        && observed != Some(expected)
    {
        diagnostics.push_item(format!("{label} {} does not match expected {expected}", observed.unwrap_or("none")));
    }
}

pub(super) struct ReceiptRefs<'a> {
    pub(super) bundle: &'a str,
    pub(super) gate: &'a str,
    pub(super) apply: &'a str,
    pub(super) reconcile: &'a str,
}

pub(super) struct ExpectedRefs<'a> {
    pub(super) envelope: Option<&'a str>,
    pub(super) operation: Option<&'a str>,
    pub(super) request: Option<&'a str>,
}

pub(super) fn live_workflow_protocol_evidence(
    input: &super::bundling::ControlLiveWorkflowProtocolGateInput<'_>,
) -> crate::error::Result<(LiveWorkflowProtocolEvidence, Vec<String>)> {
    let mut diagnostics = Vec::with_capacity(16);
    let bundle_ref = crate::preserves_rail::canonical_hash(input.bundle_value)?;
    let gate_receipt_ref = crate::preserves_rail::canonical_hash(input.receipt_value)?;
    let apply_receipt_ref = crate::preserves_rail::canonical_hash(input.apply_receipt_value)?;
    let reconcile_receipt_ref = crate::preserves_rail::canonical_hash(input.reconcile_receipt_value)?;
    let ack_ref = crate::preserves_rail::canonical_hash(input.ack_value)?;
    let bundle = parsed_or_note(&mut diagnostics, "node control live workflow protocol bundle", || {
        super::manifests::parse_control_live_workflow_bundle(input.bundle_value)
    });
    let gate = parsed_or_note(&mut diagnostics, "node control live workflow protocol gate receipt", || {
        super::manifests::parse_control_live_workflow_bundle_gate_receipt(input.receipt_value)
    });
    let apply = parsed_or_note(&mut diagnostics, "node control live workflow protocol apply receipt", || {
        super::receipts::parse_control_live_workflow_bundle_apply_receipt(input.apply_receipt_value)
    });
    let reconcile = parsed_or_note(&mut diagnostics, "node control live workflow protocol reconcile receipt", || {
        super::receipts::parse_control_live_workflow_bundle_reconcile_receipt(input.reconcile_receipt_value)
    });
    let ack = parsed_or_note(&mut diagnostics, "node control live workflow protocol ack", || {
        super::acknowledgements::parse_control_live_workflow_bundle_ack(input.ack_value)
    });
    let refs = ReceiptRefs {
        bundle: &bundle_ref,
        gate: &gate_receipt_ref,
        apply: &apply_receipt_ref,
        reconcile: &reconcile_receipt_ref,
    };
    let expected = ExpectedRefs {
        envelope: input.expected_envelope_ref,
        operation: input.expected_operation_ref,
        request: input.expected_request_ref,
    };
    if let Some(gate) = gate.as_ref() {
        super::notes::note_gate_part(&mut diagnostics, gate, &refs);
    }
    if let Some(apply) = apply.as_ref() {
        super::notes::note_apply_part(&mut diagnostics, apply, &refs);
    }
    if let Some(reconcile) = reconcile.as_ref() {
        super::notes::note_reconcile_part(&mut diagnostics, reconcile, &refs);
    }
    if let Some(ack) = ack.as_ref() {
        super::notes::note_ack_part(&mut diagnostics, ack, &refs, &expected);
    }
    let authority_ref = if let Some(bundle) = bundle.as_ref() {
        bundle.authority_grant_ref.clone()
    } else {
        super::storage::local_ref("node-control-live-workflow-protocol-authority", &bundle_ref)?
    };
    let resource_ref = bundle.as_ref().map(|bundle| bundle.bundle_ref.clone()).unwrap_or(bundle_ref.clone());
    let session_id = format!(
        "{LIVE_WORKFLOW_PROTOCOL_SESSION_PREFIX}{}",
        super::paths::ref_file_stem(&bundle_ref),
        LIVE_WORKFLOW_PROTOCOL_SESSION_PREFIX = super::settings::LIVE_WORKFLOW_PROTOCOL_SESSION_PREFIX
    );
    Ok((
        LiveWorkflowProtocolEvidence {
            session_id,
            authority_ref,
            resource_ref,
            gate_receipt_ref,
            apply_receipt_ref,
            reconcile_receipt_ref,
            ack_ref,
        },
        diagnostics,
    ))
}

pub(super) fn live_workflow_protocol_manifest_value() -> crate::error::Result<preserves::IOValue> {
    let global = crate::protocol_session::protocol_global_script_value(&[
        crate::protocol_session::ProtocolCommInput {
            from_role: "sender".to_string(),
            to_role: "receiver".to_string(),
            label: "bundle-handoff".to_string(),
            payload_tag: "workflow-bundle".to_string(),
        },
        crate::protocol_session::ProtocolCommInput {
            from_role: "sender".to_string(),
            to_role: "receiver".to_string(),
            label: "apply-evidence".to_string(),
            payload_tag: "apply-receipt".to_string(),
        },
        crate::protocol_session::ProtocolCommInput {
            from_role: "receiver".to_string(),
            to_role: "sender".to_string(),
            label: "ack-evidence".to_string(),
            payload_tag: "workflow-ack".to_string(),
        },
    ])?;
    crate::protocol_session::protocol_manifest_value(&crate::protocol_session::ProtocolManifestInput {
        protocol_id: super::settings::LIVE_WORKFLOW_PROTOCOL_ID.to_string(),
        roles: vec!["sender".to_string(), "receiver".to_string()],
        labels: vec![
            "bundle-handoff".to_string(),
            "apply-evidence".to_string(),
            "ack-evidence".to_string(),
        ],
        payloads: vec![
            crate::protocol_session::ProtocolPayloadInput {
                tag: "workflow-bundle".to_string(),
                schema_ref: super::storage::local_ref("node-control-live-workflow-protocol-schema", "workflow-bundle")?,
            },
            crate::protocol_session::ProtocolPayloadInput {
                tag: "apply-receipt".to_string(),
                schema_ref: super::storage::local_ref("node-control-live-workflow-protocol-schema", "apply-receipt")?,
            },
            crate::protocol_session::ProtocolPayloadInput {
                tag: "workflow-ack".to_string(),
                schema_ref: super::storage::local_ref("node-control-live-workflow-protocol-schema", "workflow-ack")?,
            },
        ],
        global,
        policy_refs: vec![super::storage::local_ref(
            "node-control-live-workflow-protocol-policy",
            "v1",
        )?],
        capability_refs: vec![super::storage::local_ref(
            "node-control-live-workflow-protocol-capability",
            "v1",
        )?],
        resource_refs: vec![super::storage::local_ref(
            "node-control-live-workflow-protocol-resource",
            "v1",
        )?],
    })
}
