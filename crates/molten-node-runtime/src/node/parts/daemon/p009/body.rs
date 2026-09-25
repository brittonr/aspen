
#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveWorkflowProtocolEvidence {
    session_id: String,
    authority_ref: String,
    resource_ref: String,
    gate_receipt_ref: String,
    apply_receipt_ref: String,
    reconcile_receipt_ref: String,
    ack_ref: String,
}

fn validate_live_workflow_protocol_gate_input(input: &ControlLiveWorkflowProtocolGateInput<'_>) -> Result<()> {
    if let Some(reference) = input.expected_envelope_ref {
        validate_ingress_ref(reference, "node control live workflow protocol expected envelope ref")?;
    }
    if let Some(reference) = input.expected_operation_ref {
        validate_ingress_ref(reference, "node control live workflow protocol expected operation ref")?;
    }
    if let Some(reference) = input.expected_request_ref {
        validate_ingress_ref(reference, "node control live workflow protocol expected request ref")?;
    }
    Ok(())
}

fn parsed_or_note<T>(
    diagnostics: &mut impl VecSink<String>,
    label: &str,
    parse: impl FnOnce() -> Result<T>,
) -> Option<T> {
    match parse() {
        Ok(parsed) => Some(parsed),
        Err(error) => {
            diagnostics.push_item(format!("{label} parse failed: {error}"));
            None
        }
    }
}

fn note_receipt_decision(
    diagnostics: &mut impl VecSink<String>,
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

fn note_ref_mismatch(diagnostics: &mut impl VecSink<String>, label: &str, observed: &str, expected: &str) {
    if observed != expected {
        diagnostics.push_item(format!("{label} {observed} does not match {expected}"));
    }
}

fn note_optional_ref_mismatch(
    diagnostics: &mut impl VecSink<String>,
    label: &str,
    observed: Option<&str>,
    expected: &str,
) {
    if observed != Some(expected) {
        diagnostics.push_item(format!("{label} {} does not match {expected}", observed.unwrap_or("none")));
    }
}

fn note_expected_ref(
    diagnostics: &mut impl VecSink<String>,
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

struct ReceiptRefs<'a> {
    bundle: &'a str,
    gate: &'a str,
    apply: &'a str,
    reconcile: &'a str,
}

struct ExpectedRefs<'a> {
    envelope: Option<&'a str>,
    operation: Option<&'a str>,
    request: Option<&'a str>,
}

fn note_gate_part(
    diagnostics: &mut impl VecSink<String>,
    gate: &ControlLiveWorkflowBundleGateReceipt,
    refs: &ReceiptRefs<'_>,
) {
    note_receipt_decision(
        diagnostics,
        "node control live workflow protocol gate",
        &gate.receipt_ref,
        &gate.decision,
        &gate.diagnostics,
    );
    note_ref_mismatch(diagnostics, "node control live workflow protocol gate bundle", &gate.bundle_ref, refs.bundle);
}

fn note_apply_part(
    diagnostics: &mut impl VecSink<String>,
    apply: &ControlLiveWorkflowBundleApplyReceipt,
    refs: &ReceiptRefs<'_>,
) {
    note_receipt_decision(
        diagnostics,
        "node control live workflow protocol apply",
        &apply.receipt_ref,
        &apply.decision,
        &apply.diagnostics,
    );
    note_ref_mismatch(diagnostics, "node control live workflow protocol apply bundle", &apply.bundle_ref, refs.bundle);
    note_optional_ref_mismatch(
        diagnostics,
        "node control live workflow protocol apply gate",
        apply.gate_receipt_ref.as_deref(),
        refs.gate,
    );
}

fn note_reconcile_part(
    diagnostics: &mut impl VecSink<String>,
    reconcile: &ControlLiveWorkflowBundleReconcileReceipt,
    refs: &ReceiptRefs<'_>,
) {
    note_receipt_decision(
        diagnostics,
        "node control live workflow protocol reconcile",
        &reconcile.receipt_ref,
        &reconcile.decision,
        &reconcile.diagnostics,
    );
    note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol reconcile apply",
        &reconcile.apply_receipt_ref,
        refs.apply,
    );
    note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol reconcile bundle",
        &reconcile.bundle_ref,
        refs.bundle,
    );
}

fn note_ack_part(
    diagnostics: &mut impl VecSink<String>,
    ack: &ControlLiveWorkflowBundleAck,
    refs: &ReceiptRefs<'_>,
    expected: &ExpectedRefs<'_>,
) {
    if ack.receiver_decision != "pass" {
        diagnostics
            .push_item(format!("node control live workflow protocol ack receiver decision {}", ack.receiver_decision));
        diagnostics.extend_cloned_items(&ack.receiver_diagnostics);
    }
    if !ack.diagnostics.is_empty() {
        diagnostics.extend_cloned_items(&ack.diagnostics);
    }
    note_ref_mismatch(diagnostics, "node control live workflow protocol ack apply", &ack.apply_receipt_ref, refs.apply);
    note_ref_mismatch(
        diagnostics,
        "node control live workflow protocol ack reconcile",
        &ack.reconcile_receipt_ref,
        refs.reconcile,
    );
    note_ref_mismatch(diagnostics, "node control live workflow protocol ack bundle", &ack.bundle_ref, refs.bundle);
    note_expected_ref(
        diagnostics,
        "node control live workflow protocol ack envelope",
        ack.envelope_ref.as_deref(),
        expected.envelope,
    );
    note_expected_ref(
        diagnostics,
        "node control live workflow protocol ack operation",
        ack.operation_ref.as_deref(),
        expected.operation,
    );
    note_expected_ref(
        diagnostics,
        "node control live workflow protocol ack request",
        ack.request_ref.as_deref(),
        expected.request,
    );
}

fn live_workflow_protocol_evidence(
    input: &ControlLiveWorkflowProtocolGateInput<'_>,
) -> Result<(LiveWorkflowProtocolEvidence, Vec<String>)> {
    let mut diagnostics = Vec::with_capacity(16);
    let bundle_ref = crate::preserves_rail::canonical_hash(input.bundle_value)?;
    let gate_receipt_ref = crate::preserves_rail::canonical_hash(input.receipt_value)?;
    let apply_receipt_ref = crate::preserves_rail::canonical_hash(input.apply_receipt_value)?;
    let reconcile_receipt_ref = crate::preserves_rail::canonical_hash(input.reconcile_receipt_value)?;
    let ack_ref = crate::preserves_rail::canonical_hash(input.ack_value)?;
    let bundle = parsed_or_note(&mut diagnostics, "node control live workflow protocol bundle", || {
        parse_control_live_workflow_bundle(input.bundle_value)
    });
    let gate = parsed_or_note(&mut diagnostics, "node control live workflow protocol gate receipt", || {
        parse_control_live_workflow_bundle_gate_receipt(input.receipt_value)
    });
    let apply = parsed_or_note(&mut diagnostics, "node control live workflow protocol apply receipt", || {
        parse_control_live_workflow_bundle_apply_receipt(input.apply_receipt_value)
    });
    let reconcile = parsed_or_note(&mut diagnostics, "node control live workflow protocol reconcile receipt", || {
        parse_control_live_workflow_bundle_reconcile_receipt(input.reconcile_receipt_value)
    });
    let ack = parsed_or_note(&mut diagnostics, "node control live workflow protocol ack", || {
        parse_control_live_workflow_bundle_ack(input.ack_value)
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
        note_gate_part(&mut diagnostics, gate, &refs);
    }
    if let Some(apply) = apply.as_ref() {
        note_apply_part(&mut diagnostics, apply, &refs);
    }
    if let Some(reconcile) = reconcile.as_ref() {
        note_reconcile_part(&mut diagnostics, reconcile, &refs);
    }
    if let Some(ack) = ack.as_ref() {
        note_ack_part(&mut diagnostics, ack, &refs, &expected);
    }
    let authority_ref = if let Some(bundle) = bundle.as_ref() {
        bundle.authority_grant_ref.clone()
    } else {
        local_ref("node-control-live-workflow-protocol-authority", &bundle_ref)?
    };
    let resource_ref = bundle.as_ref().map(|bundle| bundle.bundle_ref.clone()).unwrap_or(bundle_ref.clone());
    let session_id = format!("{LIVE_WORKFLOW_PROTOCOL_SESSION_PREFIX}{}", ref_file_stem(&bundle_ref));
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
